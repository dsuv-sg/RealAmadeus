//! `LAppLive2DManager` (pipeline spec §4.8, §7.5): owns the model and runs V1's frame order —
//! projection → `model.update()` (SDK, Core update #1) → parameter driver → Core update #2 →
//! upper-body zoom → draw (D8).

use std::cell::RefCell;
use std::rc::Rc;

use web_sys::{HtmlCanvasElement, WebGlFramebuffer};

use super::app_model::{load_assets, reload_textures, AppModel, LoadContext};
use super::console::{self, LogOnce};
use super::driver::EmotionDriver;
use super::projection::{apply_upper_body_zoom, base_projection};
use super::url::ModelHome;
use crate::math::matrix44::CubismMatrix44;

/// `LAppLive2DManager` for one model (V1's `_models[0]`).
pub struct Live2DManager {
    /// `_viewMatrix` (overwritten every frame from `LAppView`; identity in practice).
    view_matrix: CubismMatrix44,
    model: Option<Rc<RefCell<AppModel>>>,
    /// What `changeScene` needs to (re)load the model.
    scene: Option<(LoadContext, ModelHome, String)>,
    errors: LogOnce,
}

impl Default for Live2DManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Live2DManager {
    pub fn new() -> Self {
        Self {
            view_matrix: CubismMatrix44::new(),
            model: None,
            scene: None,
            errors: LogOnce::new(),
        }
    }

    /// `changeScene(0)`: releases the previous model (V1 only truncated the array — a leak) and
    /// starts loading `home + model_json`.
    pub fn change_scene(&mut self, ctx: LoadContext, home: ModelHome, model_json: &str) {
        self.release_all_models();
        let model = Rc::new(RefCell::new(AppModel::new(home.clone())));
        load_assets(&model, ctx.clone(), model_json);
        self.model = Some(model);
        self.scene = Some((ctx, home, model_json.to_owned()));
    }

    /// `getMainModel()`.
    pub fn main_model(&self) -> Option<Rc<RefCell<AppModel>>> {
        self.model.clone()
    }

    /// `setViewMatrix(m)` (copies the 16 elements).
    pub fn set_view_matrix(&mut self, matrix: &CubismMatrix44) {
        self.view_matrix.set_matrix(matrix.array());
    }

    /// `setOffscreenSize(w, h)` → `model.setRenderTargetSize(w, h)`.
    pub fn set_offscreen_size(&mut self, width: u32, height: u32) {
        if let Some(mut model) = self.model.as_ref().and_then(|m| m.try_borrow_mut().ok()) {
            model.set_render_target_size(width, height);
        }
    }

    /// `onUpdate()` (spec §7.5/§7.9): `canvas` is the backing store (device pixels), `dt1` the
    /// Clock-1 delta, `driver` the player (always set while the player runs).
    pub fn on_update(
        &mut self,
        canvas: &HtmlCanvasElement,
        dt1: f64,
        driver: Option<&RefCell<EmotionDriver>>,
        frame_buffer: Option<&WebGlFramebuffer>,
    ) {
        self.reload_if_core_lost();
        let (width, height) = (canvas.width(), canvas.height());
        let Some(model_rc) = self.model.clone() else {
            return;
        };
        let Ok(mut model) = model_rc.try_borrow_mut() else {
            return;
        };
        let mut projection =
            base_projection(width, height, model.projection_inputs(), &self.view_matrix);
        model.update(dt1);
        if let Some(driver) = driver {
            model.drive(driver);
        }
        apply_upper_body_zoom(&mut projection, width, height);
        if let Err(e) = model.draw(projection, width, height, frame_buffer) {
            self.errors
                .error(&format!("[ReamaCubism] Draw failed: {e}"));
        }
    }

    /// Heap growth detached the Core views (spec §3.4): reload the model from the asset cache.
    fn reload_if_core_lost(&mut self) {
        let lost = self
            .model
            .as_ref()
            .and_then(|m| m.try_borrow().ok().map(|m| m.is_core_lost()))
            .unwrap_or(false);
        if !lost {
            return;
        }
        console::warn("[ReamaCubism] Cubism Core memory was reallocated; reloading the model.");
        if let Some((ctx, home, json)) = self.scene.clone() {
            self.change_scene(ctx, home, &json);
        }
    }

    /// `webglcontextlost` (D6).
    pub fn on_context_lost(&mut self) {
        if let Some(mut model) = self.model.as_ref().and_then(|m| m.try_borrow_mut().ok()) {
            model.on_context_lost();
        }
    }

    /// `webglcontextrestored` (D6): rebuild the renderer and re-upload/re-bind the textures.
    pub fn on_context_restored(&mut self) {
        let (Some(model_rc), Some((ctx, _, _))) = (self.model.clone(), self.scene.clone()) else {
            return;
        };
        let restored = model_rc
            .try_borrow_mut()
            .map(|mut model| model.on_context_restored());
        match restored {
            Ok(Ok(true)) => reload_textures(&model_rc, ctx),
            Ok(Ok(false)) | Err(_) => {}
            Ok(Err(e)) => self
                .errors
                .error(&format!("[ReamaCubism] WebGL context restore failed: {e}")),
        }
    }

    fn release_all_models(&mut self) {
        if let Some(model) = self.model.take() {
            if let Ok(mut model) = model.try_borrow_mut() {
                model.release();
            }
        }
    }

    /// `release()` (empty in V1 — the model leaked): releases the model.
    pub fn release(&mut self) {
        self.release_all_models();
        self.scene = None;
    }
}
