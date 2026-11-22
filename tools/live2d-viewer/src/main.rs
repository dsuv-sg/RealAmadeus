//! live2d-viewer — developer tool rendering the default Amadeus model with `amadeus-live2d`.
//!
//! Query parameters: `resources` (resources prefix, default `resources/`), `modelDir`, `modelJson`
//! (override [`amadeus_live2d::PlayerOptions`]), `emotion`, `lip`, `char`, `eyeX`, `eyeY`,
//! `preload=1` (warm the asset caches first, like the app's boot screen), `destroyAfter=<ms>` /
//! `remountAfter=<ms>` (lifecycle checks). `window.l2dViewerStart(resources)` re-runs the start-up
//! (Core load retry checks).
//!
//! For automated checks the page exposes `window.l2dViewer.{state, setEmotion, setLipSync,
//! setSpokenChar, setEyeTracking, destroy, remount}` (`state()` → `"<load state>|<emotion>/<tag>|
//! <destroyed>"`, load state 23 = drawing). With `status=1` the state is also mirrored every 100 ms
//! into invisible `<html data-l2d-*>` attributes (off by default: a periodic timer makes headless
//! Chrome's `--virtual-time-budget` race ahead of the asynchronous texture decode).

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn param(params: &web_sys::UrlSearchParams, key: &str) -> Option<String> {
    params.get(key).filter(|v| !v.is_empty())
}

fn param_f64(params: &web_sys::UrlSearchParams, key: &str) -> Option<f64> {
    param(params, key).and_then(|v| v.parse::<f64>().ok())
}

struct Viewer {
    host: web_sys::HtmlElement,
    options: amadeus_live2d::PlayerOptions,
    params: web_sys::UrlSearchParams,
    player: Option<amadeus_live2d::Live2DPlayer>,
}

impl Viewer {
    fn mount(&mut self) {
        if let Some(old) = self.player.take() {
            old.destroy();
        }
        let player = amadeus_live2d::Live2DPlayer::create(self.host.clone(), self.options.clone());
        let p = &self.params;
        player.set_emotion(&param(p, "emotion").unwrap_or_else(|| "NORMAL".to_owned()));
        if let Some(lip) = param_f64(p, "lip") {
            player.set_lip_sync(lip);
        }
        if let Some(ch) = param(p, "char") {
            player.set_spoken_char(&ch);
        }
        player.set_eye_tracking(
            param_f64(p, "eyeX").unwrap_or(0.0),
            param_f64(p, "eyeY").unwrap_or(0.0),
        );
        self.player = Some(player);
    }
}

fn set_root_attr(name: &str, value: &str) {
    if let Some(root) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    {
        let _ = root.set_attribute(name, value);
    }
}

fn after_ms(window: &web_sys::Window, ms: i32, f: impl FnOnce() + 'static) {
    let callback = Closure::once_into_js(f);
    let _ =
        window.set_timeout_with_callback_and_timeout_and_arguments_0(callback.unchecked_ref(), ms);
}

/// `window.l2dViewer.<name>(…)` bound to the viewer (leaked: lives as long as the page).
fn export_fn<F>(target: &js_sys::Object, name: &str, f: F)
where
    F: FnMut(JsValue, JsValue) + 'static,
{
    let closure = Closure::<dyn FnMut(JsValue, JsValue)>::new(f);
    let _ = js_sys::Reflect::set(target, &JsValue::from_str(name), closure.as_ref());
    closure.forget();
}

fn export_api(window: &web_sys::Window, viewer: &Rc<RefCell<Viewer>>) {
    let api = js_sys::Object::new();
    let with_player = |viewer: &Rc<RefCell<Viewer>>, f: &dyn Fn(&amadeus_live2d::Live2DPlayer)| {
        if let Some(player) = viewer.borrow().player.as_ref() {
            f(player);
        }
    };
    let v = viewer.clone();
    export_fn(&api, "setEmotion", move |a, _| {
        let tag = a.as_string().unwrap_or_default();
        with_player(&v, &|p| p.set_emotion(&tag));
    });
    let v = viewer.clone();
    export_fn(&api, "setLipSync", move |a, _| {
        let value = a.as_f64().unwrap_or(0.0);
        with_player(&v, &|p| p.set_lip_sync(value));
    });
    let v = viewer.clone();
    export_fn(&api, "setSpokenChar", move |a, _| {
        let ch = a.as_string().unwrap_or_default();
        with_player(&v, &|p| p.set_spoken_char(&ch));
    });
    let v = viewer.clone();
    export_fn(&api, "setEyeTracking", move |x, y| {
        let (x, y) = (x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0));
        with_player(&v, &|p| p.set_eye_tracking(x, y));
    });
    let v = viewer.clone();
    export_fn(&api, "destroy", move |_, _| {
        with_player(&v, &|p| p.destroy());
    });
    let v = viewer.clone();
    let state = Closure::<dyn FnMut() -> JsValue>::new(move || {
        JsValue::from_str(&state_string(&v).unwrap_or_default())
    });
    let _ = js_sys::Reflect::set(&api, &JsValue::from_str("state"), state.as_ref());
    state.forget();
    let v = viewer.clone();
    export_fn(&api, "remount", move |_, _| {
        if let Ok(mut viewer) = v.try_borrow_mut() {
            viewer.mount();
        }
    });
    let _ = js_sys::Reflect::set(window, &JsValue::from_str("l2dViewer"), &api);
}

/// `"<load state>|<emotion>/<current tag>|<destroyed>"` of the current player.
fn state_string(viewer: &Rc<RefCell<Viewer>>) -> Option<String> {
    let viewer = viewer.try_borrow().ok()?;
    let player = viewer.player.as_ref()?;
    let (state, emotion, current) = player.debug_state()?;
    let state = state.map_or_else(|| "none".to_owned(), |s| s.to_string());
    Some(format!(
        "{state}|{emotion}/{current}|{}",
        player.is_destroyed()
    ))
}

fn start_status_mirror(window: &web_sys::Window, viewer: &Rc<RefCell<Viewer>>) {
    let v = viewer.clone();
    let closure = Closure::<dyn FnMut()>::new(move || {
        let Some(state) = state_string(&v) else {
            return;
        };
        let mut parts = state.split('|');
        set_root_attr("data-l2d-state", parts.next().unwrap_or_default());
        set_root_attr("data-l2d-emotion", parts.next().unwrap_or_default());
        set_root_attr("data-l2d-destroyed", parts.next().unwrap_or_default());
    });
    let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
        closure.as_ref().unchecked_ref(),
        100,
    );
    closure.forget();
}

/// `window.l2dViewerStart(resources)`: runs the viewer again with another resources prefix (e.g.
/// after a failed Core load — exercises the loader's retry).
fn export_restart(window: &web_sys::Window) {
    let closure = Closure::<dyn FnMut(JsValue)>::new(move |resources: JsValue| {
        let resources = resources.as_string();
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = run(resources).await {
                web_sys::console::error_2(&"live2d-viewer failed".into(), &e);
            }
        });
    });
    let _ = js_sys::Reflect::set(
        window,
        &JsValue::from_str("l2dViewerStart"),
        closure.as_ref(),
    );
    closure.forget();
}

async fn run(resources_override: Option<String>) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let search = window.location().search().unwrap_or_default();
    let params = web_sys::UrlSearchParams::new_with_str(&search)?;
    let resources = resources_override
        .or_else(|| param(&params, "resources"))
        .unwrap_or_else(|| "resources/".to_owned());
    if param(&params, "preload").as_deref() == Some("1") {
        amadeus_live2d::preload_default_assets(&resources).await?;
        set_root_attr("data-l2d-preloaded", "true");
    }
    let core_url = format!("{resources}live2d/live2dcubismcore.min.js");
    if let Err(e) = amadeus_live2d::ensure_core_loaded(&core_url).await {
        web_sys::console::error_2(&"Cubism init failed".into(), &e);
        set_root_attr("data-l2d-state", "core-failed");
        return Ok(());
    }
    set_root_attr("data-l2d-state", "core-ready");
    let host = window
        .document()
        .and_then(|d| d.get_element_by_id("host"))
        .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
        .ok_or("#host missing")?;
    let defaults = amadeus_live2d::PlayerOptions::default();
    let options = amadeus_live2d::PlayerOptions {
        resources_base: resources,
        model_dir: param(&params, "modelDir").unwrap_or(defaults.model_dir),
        model_json: param(&params, "modelJson").unwrap_or(defaults.model_json),
    };
    let viewer = Rc::new(RefCell::new(Viewer {
        host,
        options,
        params: params.clone(),
        player: None,
    }));
    viewer.borrow_mut().mount();
    export_api(&window, &viewer);
    if param(&params, "status").as_deref() == Some("1") {
        start_status_mirror(&window, &viewer);
    }
    if let Some(ms) = param_f64(&params, "destroyAfter") {
        let v = viewer.clone();
        after_ms(&window, ms as i32, move || {
            if let Some(player) = v.borrow().player.as_ref() {
                player.destroy();
            }
        });
    }
    if let Some(ms) = param_f64(&params, "remountAfter") {
        let v = viewer.clone();
        after_ms(&window, ms as i32, move || {
            if let Ok(mut viewer) = v.try_borrow_mut() {
                viewer.mount();
            }
        });
    }
    Ok(())
}

fn main() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    if let Some(window) = web_sys::window() {
        export_restart(&window);
    }
    wasm_bindgen_futures::spawn_local(async {
        if let Err(e) = run(None).await {
            web_sys::console::error_2(&"live2d-viewer failed".into(), &e);
        }
    });
}
