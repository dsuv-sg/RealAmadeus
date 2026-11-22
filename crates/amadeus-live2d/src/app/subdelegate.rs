//! `LAppSubdelegate` + `LAppGlManager` (pipeline spec §4.5–§4.6, §5, §7.3, §10): the canvas, its
//! WebGL context (WebGL1 first, WebGL2 fallback, V1 attributes), `devicePixelRatio` sizing,
//! `ResizeObserver`, the per-frame host GL state, and WebGL context loss/restore (D6).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlCanvasElement, ResizeObserver, WebGlFramebuffer};

use super::app_model::LoadContext;
use super::driver::EmotionDriver;
use super::manager::Live2DManager;
use super::projection::AppView;
use super::texture_manager::TextureManager;
use super::url::ModelHome;
use super::web::device_pixel_ratio;
use crate::render::gl::{clear_frame_like_v1, Gl, GlConst as GL};

/// Minimum backing-store pixel scale (2× matches Qt's 3840×2160 supersampled Live2D FBO on 1080p DPR=1).
const MIN_RENDER_SCALE: f64 = 2.0;
/// Ceiling per axis for the canvas backing buffer (keeps VRAM reasonable and within GPU limits).
const MAX_BACKING_STORE_DIM: i32 = 4096;

/// Computes the WebGL canvas backing-store dimensions (`(width, height)` in device/supersampled pixels)
/// preserving exact aspect ratio when clamped by GPU limits (`max_dim_x`, `max_dim_y`).
pub(crate) fn compute_backing_store_size(
    client_w: i32,
    client_h: i32,
    dpr: f64,
    max_dim_x: i32,
    max_dim_y: i32,
) -> (u32, u32) {
    if client_w <= 0 || client_h <= 0 {
        return (0, 0);
    }
    let effective_dpr = if dpr.is_finite() && dpr > 0.0 {
        dpr
    } else {
        1.0
    };
    let scale = effective_dpr.max(MIN_RENDER_SCALE);
    let mut w = f64::from(client_w) * scale;
    let mut h = f64::from(client_h) * scale;
    let cap_x = f64::from(if max_dim_x > 0 {
        max_dim_x.min(MAX_BACKING_STORE_DIM)
    } else {
        MAX_BACKING_STORE_DIM
    });
    let cap_y = f64::from(if max_dim_y > 0 {
        max_dim_y.min(MAX_BACKING_STORE_DIM)
    } else {
        MAX_BACKING_STORE_DIM
    });
    if w > cap_x || h > cap_y {
        let down = (cap_x / w).min(cap_y / h);
        w *= down;
        h *= down;
    }
    (w.round().max(1.0) as u32, h.round().max(1.0) as u32)
}

/// Context events recorded by the canvas listeners and applied at the start of the next frame.
#[derive(Default)]
struct ContextEvents {
    lost: Cell<bool>,
    restored: Cell<bool>,
}

type EventClosure = Closure<dyn FnMut(web_sys::Event)>;

/// `LAppSubdelegate`.
pub struct Subdelegate {
    canvas: HtmlCanvasElement,
    gl: Gl,
    texture_manager: Rc<RefCell<TextureManager>>,
    view: AppView,
    manager: Live2DManager,
    /// `_frameBuffer` (`FRAMEBUFFER_BINDING` at init: the default framebuffer).
    frame_buffer: Option<WebGlFramebuffer>,
    need_resize: Rc<Cell<bool>>,
    last_dpr: f64,
    resize_observer: Option<ResizeObserver>,
    _resize_callback: Option<Closure<dyn FnMut(JsValue, JsValue)>>,
    context_events: Rc<ContextEvents>,
    context_lost_listener: Option<EventClosure>,
    context_restored_listener: Option<EventClosure>,
    released: bool,
}

impl Subdelegate {
    /// `initialize(canvas)`: context (logs V1's `Cannot initialize WebGL context.` on failure),
    /// `resizeCanvas()`, blend state, view, `changeScene(0)` (starts loading `home + model_json`),
    /// `ResizeObserver`, context-loss listeners. `None` when WebGL is unavailable.
    pub fn initialize(
        canvas: HtmlCanvasElement,
        home: ModelHome,
        model_json: &str,
    ) -> Option<Self> {
        let gl = Gl::from_canvas(&canvas)?;
        let mut this = Self {
            canvas,
            gl,
            texture_manager: Rc::new(RefCell::new(TextureManager::new())),
            view: AppView::new(),
            manager: Live2DManager::new(),
            frame_buffer: None,
            need_resize: Rc::new(Cell::new(false)),
            last_dpr: device_pixel_ratio(),
            resize_observer: None,
            _resize_callback: None,
            context_events: Rc::new(ContextEvents::default()),
            context_lost_listener: None,
            context_restored_listener: None,
            released: false,
        };
        this.resize_canvas();
        this.frame_buffer = this.gl.framebuffer_binding();
        this.gl.enable(GL::BLEND);
        this.gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
        this.view
            .initialize(this.canvas.width(), this.canvas.height());
        this.manager
            .set_offscreen_size(this.canvas.width(), this.canvas.height());
        // initializeSprite(): V1 compiled an unused sprite program (no back/gear image) — skipped.
        let ctx = this.load_context();
        this.manager.change_scene(ctx, home, model_json);
        this.observe_resize();
        this.listen_context_loss();
        Some(this)
    }

    fn load_context(&self) -> LoadContext {
        LoadContext {
            gl: self.gl.clone(),
            canvas: self.canvas.clone(),
            textures: self.texture_manager.clone(),
        }
    }

    fn observe_resize(&mut self) {
        let need_resize = self.need_resize.clone();
        // `resizeObserverCallback`: `_needResize = true` (CanvasSize === "auto").
        let callback = Closure::<dyn FnMut(JsValue, JsValue)>::new(
            move |_entries: JsValue, _observer: JsValue| need_resize.set(true),
        );
        if let Ok(observer) = ResizeObserver::new(callback.as_ref().unchecked_ref()) {
            observer.observe(&self.canvas);
            self.resize_observer = Some(observer);
        }
        self._resize_callback = Some(callback);
    }

    fn listen_context_loss(&mut self) {
        let events = self.context_events.clone();
        let lost = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            // Asks the browser to restore the context later (D6; V1 never did).
            event.prevent_default();
            events.lost.set(true);
        });
        let events = self.context_events.clone();
        let restored = Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
            events.restored.set(true);
        });
        let _ = self
            .canvas
            .add_event_listener_with_callback("webglcontextlost", lost.as_ref().unchecked_ref());
        let _ = self.canvas.add_event_listener_with_callback(
            "webglcontextrestored",
            restored.as_ref().unchecked_ref(),
        );
        self.context_lost_listener = Some(lost);
        self.context_restored_listener = Some(restored);
    }

    /// The WebGL context.
    pub fn gl(&self) -> &Gl {
        &self.gl
    }

    /// The canvas.
    pub fn canvas(&self) -> &HtmlCanvasElement {
        &self.canvas
    }

    /// `getLive2DManager()`.
    pub fn manager(&self) -> &Live2DManager {
        &self.manager
    }

    /// `resizeCanvas()`: backing store = client size × `max(devicePixelRatio, 2.0)` clamped
    /// uniformly to GPU limits, viewport = drawing buffer size.
    pub fn resize_canvas(&mut self) {
        let dpr = device_pixel_ratio();
        self.last_dpr = dpr;
        let max_tex = self.gl.max_texture_size();
        let (vp_w, vp_h) = self.gl.max_viewport_dims();
        let lim = |a: i32, b: i32| match (a > 0, b > 0) {
            (true, true) => a.min(b),
            (true, false) => a,
            (false, true) => b,
            (false, false) => 0,
        };
        let (w, h) = compute_backing_store_size(
            self.canvas.client_width(),
            self.canvas.client_height(),
            dpr,
            lim(max_tex, vp_w),
            lim(max_tex, vp_h),
        );
        self.canvas.set_width(w);
        self.canvas.set_height(h);
        self.gl.viewport(
            0,
            0,
            self.gl.drawing_buffer_width(),
            self.gl.drawing_buffer_height(),
        );
    }

    /// `onResize()`.
    fn on_resize(&mut self) {
        self.resize_canvas();
        self.view
            .initialize(self.canvas.width(), self.canvas.height());
        self.manager
            .set_offscreen_size(self.canvas.width(), self.canvas.height());
    }

    fn forget_textures(&self) {
        if let Ok(mut textures) = self.texture_manager.try_borrow_mut() {
            textures.forget();
        }
    }

    fn apply_context_events(&mut self) {
        if self.context_events.lost.replace(false) {
            self.manager.on_context_lost();
            self.forget_textures();
        }
        if self.context_events.restored.replace(false) && !self.gl.is_context_lost() {
            self.forget_textures();
            self.manager.on_context_restored();
            self.gl.enable(GL::BLEND);
            self.gl.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);
            self.need_resize.set(true);
        }
    }

    /// `update()` — one frame: skip while the context is lost; apply a pending resize; V1's host
    /// GL state (transparent clear, depth test, blend); `LAppView.render()` → manager `onUpdate`.
    pub fn update(&mut self, dt1: f64, driver: &RefCell<EmotionDriver>) {
        if self.released {
            return;
        }
        self.apply_context_events();
        if self.gl.is_context_lost() {
            return;
        }
        let dpr = device_pixel_ratio();
        if dpr.is_finite() && (dpr - self.last_dpr).abs() > f64::EPSILON {
            self.need_resize.set(true);
        }
        if self.need_resize.get() {
            self.on_resize();
            self.need_resize.set(false);
        }
        clear_frame_like_v1(&self.gl);
        // LAppView.render(): (sprite program skipped) flush, view matrix → manager, onUpdate.
        self.gl.flush();
        self.manager.set_view_matrix(self.view.view_matrix.matrix());
        self.manager
            .on_update(&self.canvas, dt1, Some(driver), self.frame_buffer.as_ref());
    }

    /// `release()` (V1 threw here): observer, listeners, model (renderer + Core), textures.
    /// Idempotent; also run on drop. The context itself is freed by the host's `loseContext()`.
    pub fn release(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        if let Some(observer) = self.resize_observer.take() {
            observer.unobserve(&self.canvas);
            observer.disconnect();
        }
        if let Some(listener) = self.context_lost_listener.take() {
            let _ = self.canvas.remove_event_listener_with_callback(
                "webglcontextlost",
                listener.as_ref().unchecked_ref(),
            );
        }
        if let Some(listener) = self.context_restored_listener.take() {
            let _ = self.canvas.remove_event_listener_with_callback(
                "webglcontextrestored",
                listener.as_ref().unchecked_ref(),
            );
        }
        self.manager.release();
        if let Ok(mut textures) = self.texture_manager.try_borrow_mut() {
            textures.release(Some(&self.gl));
        }
    }
}

impl Drop for Subdelegate {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backing_store_size_supersamples_and_preserves_aspect_ratio() {
        assert_eq!(
            compute_backing_store_size(1920, 1045, 1.0, 16384, 16384),
            (3840, 2090)
        );
        assert_eq!(
            compute_backing_store_size(1920, 1045, 1.5, 16384, 16384),
            (3840, 2090)
        );
        assert_eq!(
            compute_backing_store_size(1920, 1045, 2.0, 16384, 16384),
            (3840, 2090)
        );
        // 3.0 DPR hits MAX_BACKING_STORE_DIM (4096) and downscales both axes uniformly.
        let (w3, h3) = compute_backing_store_size(1920, 1045, 3.0, 16384, 16384);
        assert_eq!((w3, h3), (4096, 2229));
        assert!(((f64::from(w3) / f64::from(h3)) - (1920.0 / 1045.0)).abs() < 0.001);

        assert_eq!(
            compute_backing_store_size(390, 844, 3.0, 16384, 16384),
            (1170, 2532)
        );
        // GPU cap 2048 on 1920×1045 downscales both axes uniformly.
        let (w_cap, h_cap) = compute_backing_store_size(1920, 1045, 1.0, 2048, 2048);
        assert_eq!((w_cap, h_cap), (2048, 1115));
        assert!(((f64::from(w_cap) / f64::from(h_cap)) - (1920.0 / 1045.0)).abs() < 0.001);

        // Zero or negative client sizes.
        assert_eq!(compute_backing_store_size(0, 1045, 1.0, 4096, 4096), (0, 0));
        assert_eq!(
            compute_backing_store_size(1920, -1, 1.0, 4096, 4096),
            (0, 0)
        );
    }
}
