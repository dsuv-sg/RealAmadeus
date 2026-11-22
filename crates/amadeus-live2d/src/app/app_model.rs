//! `LAppModel` (pipeline spec §4.9, §6.3–§6.5, §7.6): the asynchronous load state machine and the
//! model's per-frame work (SDK update, driver hook, draw).
//!
//! Loading mirrors V1's closure chain step by step (the [`LoadStep`] values are V1's), generic for
//! every model3.json feature the Framework supports (expressions, physics, pose, eye blink, user
//! data, lip-sync ids, layout, motions) — the shipped model only uses moc + physics + one texture.
//! Every continuation re-checks that the model is still alive and not released (destroy safety,
//! spec §10.4), and nothing is borrowed across an `await`.
//!
//! V2 differences (all invisible for the shipped model): requests of a step are issued in parallel
//! but completed in model3.json order (deterministic expression order); a moc that fails to load
//! stops the chain (logged) instead of running on to `SetupLayout`; `WaitLoadTexture` is set before
//! the texture loads are issued (spec §6.3 guard); a failed pose/user-data/motion fetch continues
//! with an empty buffer instead of stalling the chain; an empty-but-present motion group no longer
//! hangs the load.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::rc::{Rc, Weak};

use wasm_bindgen::JsValue;
use web_sys::{HtmlCanvasElement, WebGlFramebuffer};

use super::assets;
use super::console::{self, LogOnce};
use super::define::{
    CLIPPING_MASK_BUFFER_SIZE, MASK_BUFFER_COUNT, MOC_CONSISTENCY_VALIDATION_ENABLE,
    MOTION_CONSISTENCY_VALIDATION_ENABLE, MOTION_GROUP_IDLE, TEXTURE_ANISOTROPY, USE_PREMULTIPLY,
};
use super::driver::EmotionDriver;
use super::load_step::LoadStep;
use super::pipeline::sdk_update;
use super::projection::model_view_projection;
use super::texture_manager::{create_texture_from_png_file, TextureManager};
use super::url::{resolve, ModelHome};
use super::web::{math_random, performance_now_ms};
use crate::core::model::CubismModel;
use crate::framework::effect::{CubismBreath, CubismEyeBlink, CubismLook};
use crate::framework::model_setting::ModelSetting;
use crate::framework::motion::{
    motion_ref, CubismMotionManager, CubismMotionQueueEntryHandle, MotionRef, PRIORITY_FORCE,
    PRIORITY_IDLE, PRIORITY_NONE,
};
use crate::framework::update::{
    CubismBreathUpdater, CubismExpressionUpdater, CubismEyeBlinkUpdater, CubismLipSyncUpdater,
    CubismLookUpdater, CubismPhysicsUpdater, CubismPoseUpdater, CubismUpdateScheduler,
};
use crate::framework::user_model::CubismUserModel;
use crate::math::matrix44::CubismMatrix44;
use crate::math::model_matrix::CubismModelMatrix;
use crate::render::gl::Gl;
use crate::render::{CubismRendererWebGl, RenderError};

/// What a loading model needs from its subdelegate (`_subdelegate` in V1).
#[derive(Clone)]
pub struct LoadContext {
    pub gl: Gl,
    pub canvas: HtmlCanvasElement,
    pub textures: Rc<RefCell<TextureManager>>,
}

/// A model texture slot (`setupTextures` loop entry).
#[derive(Debug, Clone, PartialEq)]
pub struct TextureSlot {
    /// Model texture number (`modelTextureNumber`).
    pub index: usize,
    /// Resolved request URL.
    pub url: String,
    /// `_modelHomeDir + texturePath` as printed by V1.
    pub display: String,
}

/// `LAppModel`.
pub struct AppModel {
    state: LoadStep,
    user: CubismUserModel,
    setting: Option<ModelSetting>,
    home: ModelHome,
    model_matrix: Option<CubismModelMatrix>,
    scheduler: CubismUpdateScheduler,
    renderer: Option<CubismRendererWebGl>,
    /// `_motionUpdated` (read by the eye-blink updater).
    motion_updated: Rc<Cell<bool>>,
    user_time_seconds: f64,
    /// `_expressions` (insertion order; a failed load keeps its name with no motion, like V1).
    expressions: Vec<(String, Option<MotionRef>)>,
    motions: HashMap<String, MotionRef>,
    all_motion_count: usize,
    motion_count: usize,
    eye_blink_ids: Vec<String>,
    lip_sync_ids: Vec<String>,
    look: Option<Rc<RefCell<CubismLook>>>,
    textures: Vec<TextureSlot>,
    texture_count: usize,
    texture_generation: u64,
    released: bool,
    errors: LogOnce,
}

impl std::fmt::Debug for AppModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppModel")
            .field("state", &self.state)
            .field("home", &self.home)
            .field("has_core", &self.user.model.is_some())
            .field("textures", &self.textures)
            .field("released", &self.released)
            .finish_non_exhaustive()
    }
}

impl AppModel {
    /// `new LAppModel()` + `_modelHomeDir`.
    pub fn new(home: ModelHome) -> Self {
        Self {
            state: LoadStep::LoadAssets,
            user: CubismUserModel::new(),
            setting: None,
            home,
            model_matrix: None,
            scheduler: CubismUpdateScheduler::new(),
            renderer: None,
            motion_updated: Rc::new(Cell::new(false)),
            user_time_seconds: 0.0,
            expressions: Vec::new(),
            motions: HashMap::new(),
            all_motion_count: 0,
            motion_count: 0,
            eye_blink_ids: Vec::new(),
            lip_sync_ids: Vec::new(),
            look: None,
            textures: Vec::new(),
            texture_count: 0,
            texture_generation: 0,
            released: false,
            errors: LogOnce::new(),
        }
    }

    // ------------------------------------------------------------------------------ accessors

    /// `_state`.
    pub fn state(&self) -> LoadStep {
        self.state
    }

    /// `getModel()` (the Core-backed model once the moc is loaded).
    pub fn core(&self) -> Option<&CubismModel> {
        self.user.model.as_ref()
    }

    /// Mutable `getModel()`.
    pub fn core_mut(&mut self) -> Option<&mut CubismModel> {
        self.user.model.as_mut()
    }

    /// `getModelMatrix()`.
    pub fn model_matrix(&self) -> Option<&CubismModelMatrix> {
        self.model_matrix.as_ref()
    }

    /// The renderer (created at the `LoadTexture` step).
    pub fn renderer(&self) -> Option<&CubismRendererWebGl> {
        self.renderer.as_ref()
    }

    /// The Framework part (`CubismUserModel`: motion/expression managers, drag manager, …).
    pub fn user_model(&self) -> &CubismUserModel {
        &self.user
    }

    /// Mutable Framework part.
    pub fn user_model_mut(&mut self) -> &mut CubismUserModel {
        &mut self.user
    }

    /// `_modelSetting`.
    pub fn setting(&self) -> Option<&ModelSetting> {
        self.setting.as_ref()
    }

    /// Update-scheduler execution orders (Look 400, Breath 500, Physics 600 for the shipped model).
    pub fn updater_orders(&self) -> Vec<i64> {
        self.scheduler.execution_orders()
    }

    /// `_expressions` keys in insertion order.
    pub fn expression_names(&self) -> Vec<String> {
        self.expressions.iter().map(|(n, _)| n.clone()).collect()
    }

    /// Whether [`AppModel::release`] ran.
    pub fn is_released(&self) -> bool {
        self.released
    }

    /// Whether the Core heap views were detached (heap growth): the model must be reloaded.
    pub fn is_core_lost(&self) -> bool {
        self.user
            .model
            .as_ref()
            .is_some_and(CubismModel::is_core_lost)
    }

    /// `_modelHomeDir`.
    pub fn home(&self) -> &ModelHome {
        &self.home
    }

    // ------------------------------------------------------------------------------ frame

    /// `LAppModel.update()`: only in `CompleteSetup` — load → (idle motion) → save → scheduler
    /// (Clock-1 `dt`) → Core update #1.
    pub fn update(&mut self, delta_time_seconds: f64) {
        if self.state != LoadStep::CompleteSetup {
            return;
        }
        self.user_time_seconds += delta_time_seconds;
        let Self {
            user,
            scheduler,
            motion_updated,
            setting,
            motions,
            ..
        } = self;
        let CubismUserModel {
            model,
            motion_manager,
            ..
        } = user;
        let Some(model) = model.as_mut() else {
            return;
        };
        sdk_update(
            model,
            motion_manager,
            scheduler,
            motion_updated,
            delta_time_seconds,
            &mut |manager| {
                start_random_motion(
                    manager,
                    setting.as_ref(),
                    motions,
                    MOTION_GROUP_IDLE,
                    PRIORITY_IDLE,
                    &mut math_random,
                );
            },
        );
    }

    /// The parameter driver (`applyParameters`) followed by Core update #2 — both only while the
    /// Core model exists (also during the loading window before `CompleteSetup`, spec §7.8 c).
    pub fn drive(&mut self, driver: &RefCell<EmotionDriver>) {
        if self.user.model.is_none() {
            return;
        }
        let names = self.expression_names();
        let frame = driver
            .try_borrow_mut()
            .ok()
            .map(|mut d| d.step(performance_now_ms() / 1000.0, &mut math_random, &names));
        if let Some(name) = frame.as_ref().and_then(|f| f.expression.clone()) {
            self.set_expression(&name);
        }
        if let Some(core) = self.user.model.as_mut() {
            if let Some(frame) = &frame {
                frame.params.write_to(core);
            }
            core.update();
        }
    }

    /// `LAppModel.draw(projection)` + `doDraw()`: in `CompleteSetup` only —
    /// `projection · modelMatrix` → `setMvpMatrix`, `setRenderState(fbo, [0, 0, w, h])`, `drawModel`.
    pub fn draw(
        &mut self,
        projection: CubismMatrix44,
        canvas_width: u32,
        canvas_height: u32,
        frame_buffer: Option<&WebGlFramebuffer>,
    ) -> Result<(), RenderError> {
        if self.state != LoadStep::CompleteSetup {
            return Ok(());
        }
        let (Some(core), Some(model_matrix), Some(renderer)) = (
            self.user.model.as_ref(),
            self.model_matrix.as_ref(),
            self.renderer.as_mut(),
        ) else {
            return Ok(());
        };
        let mvp = model_view_projection(projection, model_matrix);
        renderer.set_mvp_matrix(&mvp);
        let viewport = [
            0,
            0,
            i32::try_from(canvas_width).unwrap_or(i32::MAX),
            i32::try_from(canvas_height).unwrap_or(i32::MAX),
        ];
        renderer.set_render_state(frame_buffer, viewport);
        renderer.draw_model(core)
    }

    /// Mutable model matrix together with the model canvas width (the projection's portrait
    /// branch may mutate the matrix, a V1 QUIRK).
    pub fn projection_inputs(&mut self) -> Option<(f64, &mut CubismModelMatrix)> {
        let width = self.user.model.as_ref()?.canvas_width_f64();
        Some((width, self.model_matrix.as_mut()?))
    }

    /// `setRenderTargetSize(w, h)` (only used by Cubism 5.3 offscreens).
    pub fn set_render_target_size(&mut self, width: u32, height: u32) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.set_render_target_size(
                i32::try_from(width).unwrap_or(i32::MAX),
                i32::try_from(height).unwrap_or(i32::MAX),
            );
        }
    }

    /// `setExpression(name)`: starts the named expression (no-op for unknown/failed ones).
    pub fn set_expression(&mut self, name: &str) {
        if let Some((_, Some(motion))) = self.expressions.iter().find(|(n, _)| n == name) {
            self.user
                .expression_manager
                .borrow_mut()
                .start_motion(motion.clone(), false);
        }
    }

    /// `startMotion(group, no, priority)` (`None` = V1's `-1`).
    pub fn start_motion(
        &mut self,
        group: &str,
        no: usize,
        priority: i32,
    ) -> Option<CubismMotionQueueEntryHandle> {
        self.setting.as_ref()?;
        start_motion(
            &mut self.user.motion_manager,
            &self.motions,
            group,
            no,
            priority,
        )
    }

    /// `startRandomMotion(group, priority)` (`None` = V1's `-1`).
    pub fn start_random_motion(
        &mut self,
        group: &str,
        priority: i32,
    ) -> Option<CubismMotionQueueEntryHandle> {
        start_random_motion(
            &mut self.user.motion_manager,
            self.setting.as_ref(),
            &self.motions,
            group,
            priority,
            &mut math_random,
        )
    }

    // ------------------------------------------------------------------------------ GL lifecycle

    /// `webglcontextlost` (D6): the renderer forgets its GL objects; drawing pauses.
    pub fn on_context_lost(&mut self) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.on_context_lost();
        }
    }

    /// `webglcontextrestored` (D6), synchronous half: rebuild the renderer's GL side and pause
    /// drawing until the textures are re-uploaded ([`reload_textures`]). Returns whether the
    /// textures must be reloaded.
    pub fn on_context_restored(&mut self) -> Result<bool, RenderError> {
        let Some(renderer) = self.renderer.as_mut() else {
            return Ok(false);
        };
        renderer.on_context_restored()?;
        if matches!(
            self.state,
            LoadStep::WaitLoadTexture | LoadStep::CompleteSetup
        ) {
            self.state = LoadStep::WaitLoadTexture;
            self.texture_count = 0;
            self.texture_generation = self.texture_generation.wrapping_add(1);
            return Ok(true);
        }
        Ok(false)
    }

    /// `release()`: renderer (programs, buffers, mask targets), updaters, motions/expressions and the
    /// Core model + moc. Pending loads become no-ops. Idempotent; also run on drop. The GL
    /// textures belong to the [`TextureManager`].
    pub fn release(&mut self) {
        if self.released {
            return;
        }
        self.released = true;
        if let Some(mut renderer) = self.renderer.take() {
            renderer.release();
        }
        self.look = None;
        self.scheduler.release();
        self.expressions.clear();
        self.motions.clear();
        self.user.release();
    }

    // ------------------------------------------------------------------------------ load steps

    fn load_model(&mut self, bytes: &[u8]) -> bool {
        match self
            .user
            .load_model(bytes, MOC_CONSISTENCY_VALIDATION_ENABLE)
        {
            Ok(()) => {
                if let Some(model) = self.user.model.as_ref() {
                    self.model_matrix = Some(CubismModelMatrix::new(
                        model.canvas_width_f64(),
                        model.canvas_height_f64(),
                    ));
                }
                self.user.model.is_some()
            }
            Err(e) => {
                console::error_with(
                    &format!(
                        "[ReamaCubism] Failed to create the model from {}",
                        self.setting
                            .as_ref()
                            .map(|s| self.home.display_path(&s.model_file_name()))
                            .unwrap_or_default()
                    ),
                    &JsValue::from_str(&e.to_string()),
                );
                false
            }
        }
    }

    fn insert_expression(&mut self, name: String, motion: Option<MotionRef>) {
        if let Some(entry) = self.expressions.iter_mut().find(|(n, _)| *n == name) {
            entry.1 = motion;
        } else {
            self.expressions.push((name, motion));
        }
    }

    fn setup_eye_blink_and_breath(&mut self, setting: &ModelSetting) {
        self.state = LoadStep::SetupEyeBlink;
        if setting.eye_blink_parameter_count() > 0 {
            let eye_blink = Rc::new(RefCell::new(CubismEyeBlink::create(Some(setting))));
            self.user.eye_blink = Some(eye_blink.clone());
            let motion_updated = self.motion_updated.clone();
            self.scheduler
                .add_updatable_list(Box::new(CubismEyeBlinkUpdater::new(
                    Box::new(move || motion_updated.get()),
                    eye_blink,
                    None,
                )));
        }
        self.state = LoadStep::SetupBreath;
        let breath = Rc::new(RefCell::new(CubismBreath::create()));
        breath
            .borrow_mut()
            .set_parameters(CubismBreath::v1_default_parameters());
        self.user.breath = Some(breath.clone());
        self.scheduler
            .add_updatable_list(Box::new(CubismBreathUpdater::new(breath, None)));
        self.state = LoadStep::LoadUserData;
    }

    /// Eye-blink ids → lip-sync ids → look → sort → layout. False when the chain stops at
    /// `SetupLayout` (no model matrix — V1's suppressed "Failed to setupLayout()").
    fn setup_ids_look_layout(&mut self, setting: &ModelSetting) -> bool {
        self.state = LoadStep::SetupEyeBlinkIds;
        self.eye_blink_ids = setting.eye_blink_parameter_ids();
        self.state = LoadStep::SetupLipSyncIds;
        self.lip_sync_ids = setting.lip_sync_parameter_ids();
        if !self.lip_sync_ids.is_empty() {
            // V1's provider was the never-started LAppWavFileHandler (always "no new value").
            self.scheduler
                .add_updatable_list(Box::new(CubismLipSyncUpdater::new(
                    self.lip_sync_ids.clone(),
                    None,
                    None,
                )));
        }
        self.state = LoadStep::SetupLook;
        let look = Rc::new(RefCell::new(CubismLook::create()));
        look.borrow_mut()
            .set_parameters(CubismLook::v1_default_parameters());
        self.scheduler
            .add_updatable_list(Box::new(CubismLookUpdater::new(
                look.clone(),
                self.user.drag_manager.clone(),
                None,
            )));
        self.look = Some(look);
        self.scheduler.sort_updatable_list();
        self.state = LoadStep::SetupLayout;
        let Some(model_matrix) = self.model_matrix.as_mut() else {
            self.errors.error("Failed to setupLayout().");
            return false;
        };
        model_matrix.setup_from_layout(&setting.layout_map());
        self.state = LoadStep::LoadMotion;
        true
    }

    /// Tail of `loadCubismMotion` / `preLoadMotionGroup`: `LoadTexture`, stop motions, renderer,
    /// texture slots (V2: `WaitLoadTexture` first), `startUp(gl)`, `loadShaders()`.
    fn finish_setup(&mut self, ctx: &LoadContext, setting: &ModelSetting) {
        self.state = LoadStep::LoadTexture;
        self.user.motion_manager.stop_all_motions();
        self.user.updating = false;
        self.user.initialized = true;
        let Some(core) = self.user.model.as_ref() else {
            return;
        };
        let mut renderer = CubismRendererWebGl::new(
            i32::try_from(ctx.canvas.width()).unwrap_or(i32::MAX),
            i32::try_from(ctx.canvas.height()).unwrap_or(i32::MAX),
        );
        renderer.initialize(core, MASK_BUFFER_COUNT);
        renderer.set_clipping_mask_buffer_size(core, CLIPPING_MASK_BUFFER_SIZE);
        renderer.set_anisotropy(TEXTURE_ANISOTROPY);
        // setupTextures
        self.state = LoadStep::WaitLoadTexture;
        let texture_count = setting.texture_count();
        console::log(&format!(
            "[ReamaCubism] Loading {texture_count} texture(s)..."
        ));
        self.textures.clear();
        for index in 0..texture_count {
            let name = setting.texture_file_name(index);
            if name.is_empty() {
                console::log("getTextureFileName null");
                continue;
            }
            let display = self.home.display_path(&name);
            console::log(&format!("[ReamaCubism] Loading texture {index}: {display}"));
            self.textures.push(TextureSlot {
                index,
                url: resolve(&self.home.file_url(&name)),
                display,
            });
            renderer.set_is_premultiplied_alpha(USE_PREMULTIPLY);
        }
        if let Err(e) = renderer.start_up(ctx.gl.clone()) {
            self.errors
                .error(&format!("Cubism renderer startUp failed: {e}"));
        }
        renderer.load_shaders();
        self.renderer = Some(renderer);
    }
}

impl Drop for AppModel {
    fn drop(&mut self) {
        self.release();
    }
}

/// `startMotion(group, no, priority)` without a model borrow (used from the frame update).
fn start_motion(
    manager: &mut CubismMotionManager,
    motions: &HashMap<String, MotionRef>,
    group: &str,
    no: usize,
    priority: i32,
) -> Option<CubismMotionQueueEntryHandle> {
    if priority == PRIORITY_FORCE {
        manager.set_reserve_priority(priority);
    } else if !manager.reserve_motion(priority) {
        return None;
    }
    let Some(motion) = motions.get(&format!("{group}_{no}")) else {
        // V1 starts a fetch here but tests the (still unset) result synchronously: a motion that
        // was not preloaded never starts (stock SDK quirk).
        manager.set_reserve_priority(PRIORITY_NONE);
        return None;
    };
    if let Ok(mut m) = motion.try_borrow_mut() {
        m.set_began_motion_handler(None);
        m.set_finished_motion_handler(None);
    }
    // V1 would start its (unused) WAV handler for a motion `Sound` here.
    Some(manager.start_motion_priority(motion.clone(), false, priority))
}

/// `startRandomMotion(group, priority)`: `-1` (`None`) at once when the group is empty — the
/// shipped model calls this every frame with `"Idle"` and no motions, drawing no random number.
fn start_random_motion(
    manager: &mut CubismMotionManager,
    setting: Option<&ModelSetting>,
    motions: &HashMap<String, MotionRef>,
    group: &str,
    priority: i32,
    rng: &mut dyn FnMut() -> f64,
) -> Option<CubismMotionQueueEntryHandle> {
    let setting = setting?;
    let count = setting.motion_count(group);
    if count == 0 {
        return None;
    }
    let no = (rng() * count as f64).floor() as usize;
    start_motion(manager, motions, group, no, priority)
}

// ------------------------------------------------------------------------------------------------
// The asynchronous load chain
// ------------------------------------------------------------------------------------------------

fn alive(weak: &Weak<RefCell<AppModel>>) -> Option<Rc<RefCell<AppModel>>> {
    let rc = weak.upgrade()?;
    let ok = rc.try_borrow().map(|m| !m.released).unwrap_or(false);
    ok.then_some(rc)
}

/// Runs `f` on the live model (between awaits); `None` when it is gone/released.
fn with_model<R>(weak: &Weak<RefCell<AppModel>>, f: impl FnOnce(&mut AppModel) -> R) -> Option<R> {
    let rc = alive(weak)?;
    let mut model = rc.try_borrow_mut().ok()?;
    Some(f(&mut model))
}

/// `LAppModel.loadAssets(dir, fileName)`: prints `[ReamaCubism] Loading model: …`, issues the
/// model3.json request synchronously (like V1) and runs the setup chain asynchronously.
pub fn load_assets(model: &Rc<RefCell<AppModel>>, ctx: LoadContext, file_name: &str) {
    let Ok(home) = model.try_borrow().map(|m| m.home.clone()) else {
        return;
    };
    let display = home.display_path(file_name);
    console::log(&format!("[ReamaCubism] Loading model: {display}"));
    let request = assets::fetch_bytes(&home.file_url(file_name));
    let weak = Rc::downgrade(model);
    wasm_bindgen_futures::spawn_local(async move {
        let _ = setup(weak, ctx, display, request).await;
    });
}

async fn setup(
    weak: Weak<RefCell<AppModel>>,
    ctx: LoadContext,
    display: String,
    request: impl Future<Output = Result<Vec<u8>, JsValue>>,
) -> Option<()> {
    let result = request.await;
    alive(&weak)?;
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(e) => {
            console::error_with(
                &format!("[ReamaCubism] Failed to load model: {display}"),
                &e,
            );
            return None;
        }
    };
    console::log(&format!(
        "[ReamaCubism] model3.json loaded ({} bytes)",
        bytes.len()
    ));
    let setting = match ModelSetting::from_bytes(&bytes) {
        Ok(setting) => setting,
        Err(e) => {
            console::error_with(
                &format!("[ReamaCubism] Failed to load model: {display}"),
                &JsValue::from_str(&e.to_string()),
            );
            return None;
        }
    };
    with_model(&weak, |m| m.state = LoadStep::LoadModel)?;
    setup_model(&weak, &ctx, setting).await
}

/// `setupModel(setting)` — the closure chain of spec §6.3.
async fn setup_model(
    weak: &Weak<RefCell<AppModel>>,
    ctx: &LoadContext,
    setting: ModelSetting,
) -> Option<()> {
    let home = with_model(weak, |m| {
        m.user.updating = true;
        m.user.initialized = false;
        m.setting = Some(setting.clone());
        m.home.clone()
    })?;

    // Parallel prefetch of everything model3.json references (customization A6).
    let moc_file = setting.model_file_name();
    let physics_file = setting.physics_file_name();
    if !moc_file.is_empty() {
        assets::prefetch_bytes(&home.file_url(&moc_file));
    }
    if !physics_file.is_empty() {
        assets::prefetch_bytes(&home.file_url(&physics_file));
    }
    for i in 0..setting.texture_count() {
        let file = setting.texture_file_name(i);
        if !file.is_empty() {
            assets::prefetch_image(&home.file_url(&file));
        }
    }

    // moc3 → loadModel (failures yield an empty buffer like V1's `.catch`).
    if moc_file.is_empty() {
        console::log("Model data does not exist.");
        return None;
    }
    let moc_request = assets::fetch_bytes(&home.file_url(&moc_file));
    with_model(weak, |m| m.state = LoadStep::WaitLoadModel)?;
    let moc = moc_request.await.unwrap_or_default();
    with_model(weak, |m| {
        let loaded = m.load_model(&moc);
        m.state = LoadStep::LoadExpression;
        loaded
    })?
    .then_some(())?;

    // Expressions (none in the shipped model).
    let expression_count = setting.expression_count();
    if expression_count > 0 {
        let requests: Vec<_> = (0..expression_count)
            .map(|i| {
                (
                    setting.expression_name(i),
                    assets::fetch_bytes(&home.file_url(&setting.expression_file_name(i))),
                )
            })
            .collect();
        with_model(weak, |m| m.state = LoadStep::WaitLoadExpression)?;
        for (name, request) in requests {
            let bytes = request.await.unwrap_or_default();
            with_model(weak, |m| {
                let motion = m.user.load_expression(&bytes).ok().map(motion_ref);
                m.insert_expression(name, motion);
            })?;
        }
        with_model(weak, |m| {
            let manager = m.user.expression_manager.clone();
            m.scheduler
                .add_updatable_list(Box::new(CubismExpressionUpdater::new(manager, None)));
        })?;
    }
    with_model(weak, |m| m.state = LoadStep::LoadPhysics)?;

    // Physics.
    if !physics_file.is_empty() {
        let request = assets::fetch_bytes(&home.file_url(&physics_file));
        with_model(weak, |m| m.state = LoadStep::WaitLoadPhysics)?;
        let bytes = request.await.unwrap_or_default();
        with_model(weak, |m| {
            if m.user.load_physics(&bytes).is_ok() {
                if let Some(physics) = m.user.physics.clone() {
                    m.scheduler
                        .add_updatable_list(Box::new(CubismPhysicsUpdater::new(physics, None)));
                }
            }
        })?;
    }
    with_model(weak, |m| m.state = LoadStep::LoadPose)?;

    // Pose (plain fetch in V1).
    let pose_file = setting.pose_file_name();
    if !pose_file.is_empty() {
        with_model(weak, |m| m.state = LoadStep::WaitLoadPose)?;
        let (bytes, error) = assets::fetch_plain(&home.file_url(&pose_file)).await;
        if let Some(e) = error {
            console::error_with(
                &format!("Failed to load file {}", home.display_path(&pose_file)),
                &e,
            );
        }
        with_model(weak, |m| {
            if m.user.load_pose(&bytes).is_ok() {
                if let Some(pose) = m.user.pose.clone() {
                    m.scheduler
                        .add_updatable_list(Box::new(CubismPoseUpdater::new(pose, None)));
                }
            }
        })?;
    }

    // Eye blink → breath (synchronous in V1).
    with_model(weak, |m| m.setup_eye_blink_and_breath(&setting))?;

    // User data (plain fetch in V1).
    let user_data_file = setting.user_data_file();
    if !user_data_file.is_empty() {
        with_model(weak, |m| m.state = LoadStep::WaitLoadUserData)?;
        let (bytes, error) = assets::fetch_plain(&home.file_url(&user_data_file)).await;
        if let Some(e) = error {
            console::error_with(
                &format!("Failed to load file {}", home.display_path(&user_data_file)),
                &e,
            );
        }
        with_model(weak, |m| {
            let _ = m.user.load_user_data(&bytes);
        })?;
    }

    // Ids → look → sort → layout; then motions.
    with_model(weak, |m| m.setup_ids_look_layout(&setting))?.then_some(())?;
    let groups = setting.motion_group_names();
    with_model(weak, |m| {
        m.state = LoadStep::WaitLoadMotion;
        if let Some(core) = m.user.model.as_mut() {
            core.save_parameters();
        }
        m.all_motion_count = groups.iter().map(|g| setting.motion_count(g)).sum();
        m.motion_count = 0;
    })?;
    for group in &groups {
        for i in 0..setting.motion_count(group) {
            let file = setting.motion_file_name(group, i);
            let (bytes, error) = assets::fetch_plain(&home.file_url(&file)).await;
            if let Some(e) = error {
                console::error_with(
                    &format!("Failed to load file {}", home.display_path(&file)),
                    &e,
                );
            }
            with_model(weak, |m| {
                match m.user.load_motion(
                    &bytes,
                    None,
                    None,
                    Some(&setting),
                    group,
                    i,
                    MOTION_CONSISTENCY_VALIDATION_ENABLE,
                ) {
                    Ok(mut motion) => {
                        motion.set_effect_ids(m.eye_blink_ids.clone(), m.lip_sync_ids.clone());
                        m.motions.insert(format!("{group}_{i}"), motion_ref(motion));
                        m.motion_count += 1;
                    }
                    Err(_) => m.all_motion_count = m.all_motion_count.saturating_sub(1),
                }
            })?;
        }
    }
    with_model(weak, |m| m.finish_setup(ctx, &setting))?;
    load_textures(weak, ctx).await
}

/// Texture loads of `setupTextures` (parallel requests via the image cache; each texture is bound
/// when ready; `CompleteSetup` once `getTextureCount()` textures are bound). Also used to re-upload
/// after a context restore (a newer run supersedes an older one).
pub async fn load_textures(weak: &Weak<RefCell<AppModel>>, ctx: &LoadContext) -> Option<()> {
    let (slots, total, generation) = with_model(weak, |m| {
        (
            m.textures.clone(),
            m.setting.as_ref().map_or(0, ModelSetting::texture_count),
            m.texture_generation,
        )
    })?;
    for slot in slots {
        let info = create_texture_from_png_file(
            Rc::downgrade(&ctx.textures),
            ctx.gl.clone(),
            slot.url.clone(),
            slot.display.clone(),
            USE_PREMULTIPLY,
        )
        .await;
        let Some(info) = info else {
            continue;
        };
        with_model(weak, |m| {
            if m.texture_generation != generation {
                return;
            }
            console::log(&format!(
                "[ReamaCubism] Texture {} loaded: {}x{}",
                slot.index, info.width, info.height
            ));
            if let Some(renderer) = m.renderer.as_mut() {
                renderer.bind_texture(slot.index, info.id.clone());
            }
            m.texture_count += 1;
            if m.texture_count >= total {
                console::log("[ReamaCubism] All textures loaded. Model setup complete!");
                m.state = LoadStep::CompleteSetup;
            }
        })?;
    }
    Some(())
}

/// Re-uploads and re-binds every texture after `webglcontextrestored` (D6).
pub fn reload_textures(model: &Rc<RefCell<AppModel>>, ctx: LoadContext) {
    let weak = Rc::downgrade(model);
    wasm_bindgen_futures::spawn_local(async move {
        let _ = load_textures(&weak, &ctx).await;
    });
}
