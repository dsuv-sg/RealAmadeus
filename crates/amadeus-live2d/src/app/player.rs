//! Public player API — port of V1 `ReamaCubismPlayer` (`scripts/cubism-runtime-entry.ts`, deployed
//! bundle 17689–18643; pipeline spec §3–§4, §7.1, §10).
//!
//! `create` builds the canvas inside the host, waits (one animation frame at a time) until it has a
//! size, starts the framework, creates the WebGL context and the model, and runs the first frame
//! synchronously. Each animation frame: Clock 1 (`Date.now()`) → subdelegate frame (SDK update,
//! driver, double Core update, draw). The pure parameter logic lives in
//! [`super::driver::EmotionDriver`]; the setters only touch that state, so they work before the model
//! has loaded (like V1).
//!
//! V2 fixes (D5–D8): `destroy()` never throws and releases everything (rAF callbacks, observer,
//! listeners, renderer/GL objects, textures, Core model + moc, framework ref-count; pending loads
//! become no-ops); per-frame errors are logged once and the loop keeps running; WebGL context loss
//! is recovered; the canvas gets the final inline style V1's host component forced
//! (`pointer-events:auto`, 100 % × 100 %).

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, HtmlElement};

use super::clock::FrameClock;
use super::console;
use super::define::{MODEL_DIR, MODEL_JSON_FILE_NAME, RESOURCES_BASE};
use super::driver::EmotionDriver;
use super::subdelegate::Subdelegate;
use super::url::ModelHome;
use super::web::{date_now_ms, math_random};
use crate::core::ffi;
use crate::framework;

/// Options for [`Live2DPlayer::create`].
#[derive(Debug, Clone)]
pub struct PlayerOptions {
    /// URL prefix of the static resources directory, resolved against `document.baseURI`.
    /// V1 used the origin-absolute `/resources/`; V2 default is base-relative `resources/`.
    pub resources_base: String,
    /// Model directory under `resources_base` (V1 `ResourcesPath` + `ModelDir[0]`).
    pub model_dir: String,
    /// Model setting file name (V1 `ModelJsonFileName`).
    pub model_json: String,
}

impl Default for PlayerOptions {
    fn default() -> Self {
        Self {
            resources_base: RESOURCES_BASE.to_owned(),
            model_dir: MODEL_DIR.to_owned(),
            model_json: MODEL_JSON_FILE_NAME.to_owned(),
        }
    }
}

type FrameCallback = Closure<dyn FnMut(f64)>;

/// DOM / GL side of a player (borrowed only inside callbacks, never across them).
struct PlayerRuntime {
    host: HtmlElement,
    options: PlayerOptions,
    canvas: Option<HtmlCanvasElement>,
    subdelegate: Option<Subdelegate>,
    framework_acquired: bool,
    clock: FrameClock,
    raf_handle: Option<i32>,
    raf_callback: Option<FrameCallback>,
    init_handle: Option<i32>,
    init_callback: Option<FrameCallback>,
    released: bool,
}

impl PlayerRuntime {
    fn request_frame(&mut self) {
        let (Some(window), Some(callback)) = (web_sys::window(), self.raf_callback.as_ref()) else {
            return;
        };
        self.raf_handle = window
            .request_animation_frame(callback.as_ref().unchecked_ref())
            .ok();
    }

    fn request_init_retry(&mut self) {
        let (Some(window), Some(callback)) = (web_sys::window(), self.init_callback.as_ref())
        else {
            return;
        };
        self.init_handle = window
            .request_animation_frame(callback.as_ref().unchecked_ref())
            .ok();
    }

    /// The V2 `destroy()` body (spec §10.4). Idempotent. The callbacks themselves are kept until
    /// the player is dropped (they hold only weak references and are never scheduled again).
    fn release_all(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        if let Some(window) = web_sys::window() {
            if let Some(handle) = self.init_handle.take() {
                let _ = window.cancel_animation_frame(handle);
            }
            if let Some(handle) = self.raf_handle.take() {
                let _ = window.cancel_animation_frame(handle);
            }
        }
        if let Some(mut subdelegate) = self.subdelegate.take() {
            subdelegate.release();
        }
        self.canvas = None;
        self.host.set_inner_html("");
        if self.framework_acquired {
            framework::release();
            self.framework_acquired = false;
        }
    }
}

struct PlayerShared {
    destroyed: Cell<bool>,
    driver: RefCell<EmotionDriver>,
    runtime: RefCell<PlayerRuntime>,
}

impl Drop for PlayerShared {
    fn drop(&mut self) {
        if let Ok(mut runtime) = self.runtime.try_borrow_mut() {
            runtime.release_all();
        }
    }
}

/// Handle to a running Live2D player bound to a host element. Cheap to clone.
#[derive(Clone)]
pub struct Live2DPlayer {
    shared: Rc<PlayerShared>,
}

impl std::fmt::Debug for Live2DPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Live2DPlayer")
            .field("destroyed", &self.shared.destroyed.get())
            .finish_non_exhaustive()
    }
}

impl Live2DPlayer {
    /// Create a player: appends a `<canvas>` to `host`, waits until the host has a non-zero
    /// size, initializes WebGL + the model, and starts the render loop. Call after
    /// [`super::core_loader::ensure_core_loaded`]; without a ready Core the player stays inert
    /// (`[ReamaCubism] Live2DCubismCore is not loaded.`), like V1.
    pub fn create(host: HtmlElement, options: PlayerOptions) -> Live2DPlayer {
        let driver = EmotionDriver::new(&mut math_random);
        let shared = Rc::new(PlayerShared {
            destroyed: Cell::new(false),
            driver: RefCell::new(driver),
            runtime: RefCell::new(PlayerRuntime {
                host,
                options,
                canvas: None,
                subdelegate: None,
                framework_acquired: false,
                clock: FrameClock::new(date_now_ms()),
                raf_handle: None,
                raf_callback: None,
                init_handle: None,
                init_callback: None,
                released: false,
            }),
        });
        begin_initialize(&shared);
        Live2DPlayer { shared }
    }

    /// Emotion tag (`NORMAL`, `SMILE`, `ANGRY`, `SAD`, `SURPRISED`, `BLUSH`, `WINK`, `DISGUST`,
    /// `SMUG`, `THINKING`, `PANIC`, `SLEEPING`); case-insensitive, trimmed.
    pub fn set_emotion(&self, emotion: &str) {
        if let Ok(mut driver) = self.shared.driver.try_borrow_mut() {
            driver.set_emotion(emotion);
        }
    }

    /// Lip-sync level, clamped to 0..=1.
    pub fn set_lip_sync(&self, value: f64) {
        if let Ok(mut driver) = self.shared.driver.try_borrow_mut() {
            driver.set_lip_sync(value);
        }
    }

    /// Currently spoken character (drives vowel mouth shapes); empty string = none.
    pub fn set_spoken_char(&self, ch: &str) {
        if let Ok(mut driver) = self.shared.driver.try_borrow_mut() {
            driver.set_spoken_char(ch);
        }
    }

    /// Eye/gaze target in -1..=1 (x right, y up).
    pub fn set_eye_tracking(&self, x: f64, y: f64) {
        if let Ok(mut driver) = self.shared.driver.try_borrow_mut() {
            driver.set_eye_tracking(x, y);
        }
    }

    /// Stop the render loop, release GL resources and clear the host element. Never panics;
    /// idempotent.
    pub fn destroy(&self) {
        if self.shared.destroyed.replace(true) {
            return;
        }
        // If a frame is running right now (re-entrant call), it releases on return.
        if let Ok(mut runtime) = self.shared.runtime.try_borrow_mut() {
            runtime.release_all();
        }
    }

    pub fn is_destroyed(&self) -> bool {
        self.shared.destroyed.get()
    }

    /// The canvas the player created (`None` before creation / after destroy).
    pub fn canvas(&self) -> Option<HtmlCanvasElement> {
        self.shared
            .runtime
            .try_borrow()
            .ok()
            .and_then(|r| r.canvas.clone())
    }

    /// Debug snapshot: `(load state value, emotion, current emotion tag)`; `None` while busy.
    pub fn debug_state(&self) -> Option<(Option<u8>, String, String)> {
        let runtime = self.shared.runtime.try_borrow().ok()?;
        let state = runtime
            .subdelegate
            .as_ref()
            .and_then(|s| s.manager().main_model())
            .and_then(|m| m.try_borrow().ok().map(|m| m.state().value()));
        let driver = self.shared.driver.try_borrow().ok()?;
        Some((
            state,
            driver.emotion().to_owned(),
            driver.current_emotion_tag().to_owned(),
        ))
    }
}

/// `beginInitialize()`: Core check → canvas (`display:block; width/height:100%;
/// pointer-events:auto`) replaces the host content → synchronous first `tryInit()`.
fn begin_initialize(shared: &Rc<PlayerShared>) {
    if !ffi::core_available() {
        console::error("[ReamaCubism] Live2DCubismCore is not loaded.");
        return;
    }
    {
        let Ok(mut runtime) = shared.runtime.try_borrow_mut() else {
            return;
        };
        let Some(canvas) = create_canvas() else {
            console::error("[ReamaCubism] Failed to create the canvas element.");
            return;
        };
        runtime.host.set_inner_html("");
        if runtime.host.append_child(&canvas).is_err() {
            console::error("[ReamaCubism] Failed to attach the canvas element.");
            return;
        }
        runtime.canvas = Some(canvas);
        let weak = Rc::downgrade(shared);
        runtime.init_callback = Some(Closure::<dyn FnMut(f64)>::new(move |_time: f64| {
            if let Some(shared) = weak.upgrade() {
                try_init(&shared);
            }
        }));
        let weak = Rc::downgrade(shared);
        runtime.raf_callback = Some(Closure::<dyn FnMut(f64)>::new(move |_time: f64| {
            if let Some(shared) = Weak::upgrade(&weak) {
                tick(&shared);
            }
        }));
    }
    try_init(shared);
}

fn create_canvas() -> Option<HtmlCanvasElement> {
    let canvas = web_sys::window()?
        .document()?
        .create_element("canvas")
        .ok()?
        .dyn_into::<HtmlCanvasElement>()
        .ok()?;
    let style = canvas.style();
    let _ = style.set_property("display", "block");
    let _ = style.set_property("width", "100%");
    let _ = style.set_property("height", "100%");
    // V1's runtime set `none`, its host component forced `auto`: the user saw `auto`.
    let _ = style.set_property("pointer-events", "auto");
    Some(canvas)
}

/// `tryInit()`: initialize once the canvas (or, as a fallback, the host) has a non-zero size;
/// otherwise retry on the next animation frame (no limit, until destroyed).
fn try_init(shared: &Rc<PlayerShared>) {
    if shared.destroyed.get() {
        return;
    }
    let canvas = {
        let Ok(mut runtime) = shared.runtime.try_borrow_mut() else {
            return;
        };
        runtime.init_handle = None;
        let Some(canvas) = runtime.canvas.clone() else {
            return;
        };
        let width = nonzero_or(canvas.client_width(), runtime.host.client_width());
        let height = nonzero_or(canvas.client_height(), runtime.host.client_height());
        if width <= 0 || height <= 0 {
            runtime.request_init_retry();
            return;
        }
        canvas
    };
    init_subdelegate(shared, &canvas);
}

/// JS `a || b` for integers.
fn nonzero_or(a: i32, b: i32) -> i32 {
    if a != 0 {
        a
    } else {
        b
    }
}

/// `initSubdelegate(canvas)`: framework ref-count, context + model, then the first frame
/// synchronously.
fn init_subdelegate(shared: &Rc<PlayerShared>, canvas: &HtmlCanvasElement) {
    {
        let Ok(mut runtime) = shared.runtime.try_borrow_mut() else {
            return;
        };
        if framework::acquire().is_err() {
            console::error("[ReamaCubism] Live2DCubismCore is not loaded.");
            return;
        }
        runtime.framework_acquired = true;
        console::log(&format!(
            "[ReamaCubism] Initializing. Canvas client size: {}x{}",
            canvas.client_width(),
            canvas.client_height()
        ));
        let home = ModelHome::new(&runtime.options.resources_base, &runtime.options.model_dir);
        let model_json = runtime.options.model_json.clone();
        let Some(subdelegate) = Subdelegate::initialize(canvas.clone(), home, &model_json) else {
            console::error("[ReamaCubism] Failed to initialize Cubism subdelegate.");
            runtime.host.set_inner_html("");
            runtime.canvas = None;
            framework::release();
            runtime.framework_acquired = false;
            return;
        };
        let gl = subdelegate.gl();
        console::log(&format!(
            "[ReamaCubism] WebGL ready. Max texture size: {}, Canvas buffer: {}x{}",
            gl.max_texture_size(),
            gl.drawing_buffer_width(),
            gl.drawing_buffer_height()
        ));
        runtime.subdelegate = Some(subdelegate);
    }
    tick(shared);
}

/// `tick`: Clock 1, one subdelegate frame, re-arm. Errors inside a frame are logged (once per
/// message) by the layers below; nothing can stop the loop except `destroy()`.
fn tick(shared: &Rc<PlayerShared>) {
    if shared.destroyed.get() {
        return;
    }
    let Ok(mut runtime) = shared.runtime.try_borrow_mut() else {
        return;
    };
    let runtime = &mut *runtime;
    let Some(subdelegate) = runtime.subdelegate.as_mut() else {
        return;
    };
    runtime.clock.update(date_now_ms());
    subdelegate.update(runtime.clock.delta_seconds(), &shared.driver);
    if shared.destroyed.get() {
        // destroy() was called while this frame held the runtime.
        runtime.release_all();
        return;
    }
    runtime.request_frame();
}
