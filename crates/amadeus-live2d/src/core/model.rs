//! [`CubismMoc`] / [`CubismModel`] — the Framework's `cubismmoc.ts` / `cubismmodel.ts` (+ the colour
//! overrides of `cubismmodelmultiplyandscreencolor.ts`) on top of [`super::ffi`]. Contract: see
//! `core/mod.rs` (signatures fixed, additions allowed).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! Data flow (spec §3.6): parameter values and part opacities are mirrored in a pure
//! [`ModelState`] (`f32` vectors with the exact Framework semantics: clamp before weight, virtual
//! "not exist" slots, repeat overrides, save/load). [`CubismModel::update`] pushes the mirrors into
//! the Core's typed arrays, runs `Model.update()` + `drawables.resetDynamicFlags()`, pulls the
//! (Core-clamped) values back and refreshes the cached dynamic drawable data (dynamic flags,
//! opacities, render orders, multiply/screen colours). Static drawable data (IDs, texture indices,
//! constant flags → blend mode / culling / inverted mask, masks, counts) is cached at creation;
//! geometry is exposed as the Core's own typed-array views (zero-copy WebGL uploads).
//!
//! Every framework algorithm uses the model through [`ParameterStore`] (`f64` JS semantics); the
//! `f32` methods below are the contract's convenience API.

use std::cell::Cell;
use std::rc::Rc;

use js_sys::{Array, ArrayBuffer, Float32Array, Int32Array, Uint16Array, Uint8Array};
use wasm_bindgen::JsCast;

use super::ffi;
use crate::framework::model_state::ModelState;
use crate::framework::multiply_screen_color::{
    CubismModelMultiplyAndScreenColor, CubismTextureColor, PartsContext,
};
use crate::framework::parameter_store::ParameterStore;
use crate::framework::parts_hierarchy::{self, CubismModelPartInfo};

/// Errors of the Live2D layer.
#[derive(Debug, thiserror::Error)]
pub enum Live2DError {
    #[error("Live2DCubismCore is not loaded")]
    CoreNotLoaded,
    #[error("invalid moc3: {0}")]
    InvalidMoc(String),
    #[error("javascript error: {0}")]
    Js(String),
    #[error("load error: {0}")]
    Load(String),
}

fn js_error(context: &str, e: &wasm_bindgen::JsValue) -> Live2DError {
    Live2DError::Js(format!(
        "{context}: {}",
        e.as_string().unwrap_or_else(|| format!("{e:?}"))
    ))
}

/// Drawable blend mode (pre-5.3 models use only these three; `CubismBlendMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Additive,
    Multiplicative,
}

/// `CubismColorBlend` (Cubism 5.3 colour blend of drawables/offscreens; `None` = not cached).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorBlend {
    None = -1,
    Normal = 0,
    AddCompatible = 1,
    MultiplyCompatible = 2,
    Add = 3,
    AddGlow = 4,
    Darken = 5,
    Multiply = 6,
    ColorBurn = 7,
    LinearBurn = 8,
    Lighten = 9,
    Screen = 10,
    ColorDodge = 11,
    Overlay = 12,
    SoftLight = 13,
    HardLight = 14,
    LinearLight = 15,
    Hue = 16,
    Color = 17,
}

impl ColorBlend {
    /// From the Core value (`blendModes[i] & 0xff`); unknown values map to `None`.
    pub fn from_i32(v: i32) -> Self {
        use ColorBlend::*;
        match v {
            0 => Normal,
            1 => AddCompatible,
            2 => MultiplyCompatible,
            3 => Add,
            4 => AddGlow,
            5 => Darken,
            6 => Multiply,
            7 => ColorBurn,
            8 => LinearBurn,
            9 => Lighten,
            10 => Screen,
            11 => ColorDodge,
            12 => Overlay,
            13 => SoftLight,
            14 => HardLight,
            15 => LinearLight,
            16 => Hue,
            17 => Color,
            _ => None,
        }
    }
}

/// `CubismAlphaBlend`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaBlend {
    None = -1,
    Over = 0,
    Atop = 1,
    Out = 2,
    ConjointOver = 3,
    DisjointOver = 4,
}

impl AlphaBlend {
    /// From the Core value (`(blendModes[i] >> 8) & 0xff`); unknown values map to `None`.
    pub fn from_i32(v: i32) -> Self {
        match v {
            0 => AlphaBlend::Over,
            1 => AlphaBlend::Atop,
            2 => AlphaBlend::Out,
            3 => AlphaBlend::ConjointOver,
            4 => AlphaBlend::DisjointOver,
            _ => AlphaBlend::None,
        }
    }
}

/// `CullingData { isOverridden, isCulling }` — ONE instance shared by all drawables (QUIRK: the
/// per-index setters of the TS affect every drawable) and one for all offscreens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CullingData {
    pub is_overridden: bool,
    pub is_culling: bool,
}

// ------------------------------------------------------------------------------------------------
// CubismMoc
// ------------------------------------------------------------------------------------------------

/// Shared moc handle: released (`Moc._release()`) when the `CubismMoc` and every model created from
/// it are gone, so a model never outlives its moc memory.
struct MocInner {
    moc: ffi::Moc,
    model_count: Cell<u32>,
}

impl Drop for MocInner {
    fn drop(&mut self) {
        #[cfg(target_arch = "wasm32")]
        self.moc.release();
    }
}

/// `Live2DCubismCore.Moc` wrapper (`CubismMoc`).
pub struct CubismMoc {
    inner: Option<Rc<MocInner>>,
    moc_version: u32,
}

impl std::fmt::Debug for CubismMoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CubismMoc")
            .field("moc_version", &self.moc_version)
            .field("released", &self.inner.is_none())
            .field("model_count", &self.model_count())
            .finish()
    }
}

impl CubismMoc {
    /// `CubismMoc.create(mocBytes, shouldCheckMocConsistency)`. Registers the Core log handler first
    /// (once per page), so Core errors are logged instead of throwing.
    pub fn from_bytes(bytes: &[u8], check_consistency: bool) -> Result<CubismMoc, Live2DError> {
        if !ffi::core_available() {
            return Err(Live2DError::CoreNotLoaded);
        }
        ffi::register_core_log_handler();
        let buffer = Uint8Array::from(bytes).buffer();
        if check_consistency && !ffi::has_moc_consistency(&buffer) {
            log::error!(target: "cubism", "Inconsistent MOC3.");
            return Err(Live2DError::InvalidMoc("Inconsistent MOC3.".to_owned()));
        }
        match ffi::Moc::from_array_buffer(&buffer) {
            Ok(Some(moc)) => Ok(CubismMoc {
                inner: Some(Rc::new(MocInner {
                    moc,
                    model_count: Cell::new(0),
                })),
                moc_version: ffi::moc_version_of(&buffer),
            }),
            Ok(None) => Err(Live2DError::InvalidMoc(format!(
                "Failed to CubismMoc.create(). (moc3 version {}, latest supported {})",
                ffi::moc_version_of(&buffer),
                ffi::latest_moc_version()
            ))),
            Err(e) => Err(js_error("Live2DCubismCore.Moc.fromArrayBuffer", &e)),
        }
    }

    /// `getMocVersion()` (5 for the Kurisu model).
    pub fn moc_version(&self) -> u32 {
        self.moc_version
    }

    /// `getLatestMocVersion()` (6 for Core 6.0.1).
    pub fn latest_moc_version(&self) -> u32 {
        ffi::latest_moc_version()
    }

    /// `CubismMoc.getMocVersionFromBuffer(bytes)`.
    pub fn moc_version_from_bytes(bytes: &[u8]) -> u32 {
        if !ffi::core_available() {
            return 0;
        }
        ffi::moc_version_of(&Uint8Array::from(bytes).buffer())
    }

    /// `CubismMoc.hasMocConsistency(bytes)`.
    pub fn has_moc_consistency(bytes: &[u8]) -> bool {
        ffi::core_available() && ffi::has_moc_consistency(&Uint8Array::from(bytes).buffer())
    }

    /// `createModel()`: `Model.fromMoc` + `CubismModel.initialize()`.
    pub fn create_model(&self) -> Result<CubismModel, Live2DError> {
        let inner = self
            .inner
            .as_ref()
            .ok_or_else(|| Live2DError::Load("CubismMoc was released".to_owned()))?;
        let core_model = ffi::Model::from_moc(&inner.moc)
            .map_err(|e| js_error("Live2DCubismCore.Model.fromMoc", &e))?
            .ok_or_else(|| Live2DError::Load("Failed to CreateModel().".to_owned()))?;
        let model = CubismModel::from_core(core_model, inner.clone())?;
        inner.model_count.set(inner.model_count.get() + 1);
        Ok(model)
    }

    /// Number of live models created from this moc.
    pub fn model_count(&self) -> u32 {
        self.inner.as_ref().map_or(0, |i| i.model_count.get())
    }

    /// `release()`: drops this handle (the Core moc is freed once its models are released too).
    pub fn release(&mut self) {
        self.inner = None;
    }
}

// ------------------------------------------------------------------------------------------------
// CubismModel
// ------------------------------------------------------------------------------------------------

/// The Core-side objects of a live model.
struct CoreHandle {
    model: ffi::Model,
    moc: Rc<MocInner>,
    parameter_values: Float32Array,
    part_opacities: Float32Array,
    drawables: ffi::Drawables,
    dynamic_flags: Uint8Array,
    drawable_opacities: Float32Array,
    render_orders: Int32Array,
    multiply_colors: Float32Array,
    screen_colors: Float32Array,
    draw_orders: Option<Int32Array>,
    offscreen_opacities: Option<Float32Array>,
    offscreen_multiply_colors: Option<Float32Array>,
    offscreen_screen_colors: Option<Float32Array>,
}

/// Rust equivalent of the Framework `CubismModel`.
pub struct CubismModel {
    state: ModelState,
    core: Option<CoreHandle>,
    core_lost: bool,
    logged_update_error: bool,
    // canvas
    canvas_width_px: f64,
    canvas_height_px: f64,
    canvas_origin_x: f64,
    canvas_origin_y: f64,
    pixels_per_unit: f64,
    // parameters (static extras)
    parameter_types: Vec<i32>,
    parameter_key_values: Vec<Vec<f32>>,
    // parts
    part_parent_indices: Vec<i32>,
    part_offscreen_indices: Vec<i32>,
    parts_hierarchy: Vec<CubismModelPartInfo>,
    // drawables (static)
    drawable_ids: Vec<String>,
    texture_indices: Vec<i32>,
    constant_flags: Vec<u8>,
    color_blends: Vec<i32>,
    alpha_blends: Vec<i32>,
    mask_counts: Vec<i32>,
    masks: Vec<Vec<i32>>,
    vertex_counts: Vec<i32>,
    index_counts: Vec<i32>,
    drawable_parent_part_indices: Vec<i32>,
    vertex_positions: Vec<Float32Array>,
    vertex_uvs: Vec<Float32Array>,
    indices: Vec<Uint16Array>,
    // drawables (dynamic, refreshed by update)
    dynamic_flags: Vec<u8>,
    drawable_opacities: Vec<f32>,
    render_orders: Vec<i32>,
    multiply_colors: Vec<f32>,
    screen_colors: Vec<f32>,
    draw_orders: Vec<i32>,
    // offscreens (Cubism 5.3; 0 for the Kurisu model)
    offscreen_count: usize,
    offscreen_color_blends: Vec<i32>,
    offscreen_alpha_blends: Vec<i32>,
    offscreen_owner_indices: Vec<i32>,
    offscreen_mask_counts: Vec<i32>,
    offscreen_masks: Vec<Vec<i32>>,
    offscreen_constant_flags: Vec<u8>,
    offscreen_opacities: Vec<f32>,
    offscreen_multiply_colors: Vec<f32>,
    offscreen_screen_colors: Vec<f32>,
    // overrides
    is_overridden_cullings: bool,
    drawable_culling: CullingData,
    offscreen_culling: CullingData,
    color_overrides: CubismModelMultiplyAndScreenColor,
    is_blend_mode_enabled: bool,
}

impl std::fmt::Debug for CubismModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CubismModel")
            .field("parameters", &self.state.parameter_count())
            .field("parts", &self.state.part_count())
            .field("drawables", &self.drawable_ids.len())
            .field("released", &self.core.is_none())
            .field("core_lost", &self.core_lost)
            .finish_non_exhaustive()
    }
}

fn strings(array: &Array) -> Vec<String> {
    array
        .iter()
        .map(|v| v.as_string().unwrap_or_default())
        .collect()
}

fn i32_arrays(array: &Array) -> Vec<Vec<i32>> {
    array
        .iter()
        .map(|v| {
            v.dyn_into::<Int32Array>()
                .map(|a| a.to_vec())
                .unwrap_or_default()
        })
        .collect()
}

fn typed_views<T: JsCast>(array: &Array) -> Vec<T> {
    array.iter().map(JsCast::unchecked_into::<T>).collect()
}

/// Copies a Core view into a Rust cache; `false` if the view was detached (heap growth).
fn pull_f32(view: &Float32Array, dst: &mut Vec<f32>) -> bool {
    let n = view.length() as usize;
    if n != dst.len() {
        if n == 0 && !dst.is_empty() {
            return false;
        }
        dst.resize(n, 0.0);
    }
    view.copy_to(dst);
    true
}

fn pull_i32(view: &Int32Array, dst: &mut Vec<i32>) -> bool {
    let n = view.length() as usize;
    if n != dst.len() {
        if n == 0 && !dst.is_empty() {
            return false;
        }
        dst.resize(n, 0);
    }
    view.copy_to(dst);
    true
}

fn pull_u8(view: &Uint8Array, dst: &mut Vec<u8>) -> bool {
    let n = view.length() as usize;
    if n != dst.len() {
        if n == 0 && !dst.is_empty() {
            return false;
        }
        dst.resize(n, 0);
    }
    view.copy_to(dst);
    true
}

fn color4(colors: &[f32], index: usize) -> [f32; 4] {
    let i = index * 4;
    match colors.get(i..i + 4) {
        Some(c) => [c[0], c[1], c[2], c[3]],
        None => [1.0, 1.0, 1.0, 1.0],
    }
}

impl CubismModel {
    /// `new CubismModel(coreModel)` + `initialize()` (spec §7.1–§7.2).
    fn from_core(model: ffi::Model, moc: Rc<MocInner>) -> Result<CubismModel, Live2DError> {
        let parameters = model.parameters();
        let parts = model.parts();
        let drawables = model.drawables();
        let offscreens = model.offscreens();
        let canvas = model.canvasinfo();

        let parameter_ids = strings(&parameters.ids());
        let parameter_values = parameters.values();
        let repeats = parameters.repeats().map(|a| a.to_vec()).unwrap_or_default();
        let part_opacities = parts.opacities();
        let state = ModelState::new(
            parameter_ids,
            parameter_values.to_vec(),
            parameters.minimum_values().to_vec(),
            parameters.maximum_values().to_vec(),
            parameters.default_values().to_vec(),
            repeats.iter().map(|r| *r != 0).collect(),
            strings(&parts.ids()),
            part_opacities.to_vec(),
        );

        let drawable_count = drawables.count() as usize;
        let blend_modes = drawables
            .blend_modes()
            .map(|a| a.to_vec())
            .unwrap_or_default();
        let color_blends: Vec<i32> = (0..drawable_count)
            .map(|i| blend_modes.get(i).map_or(0, |b| b & 0xff))
            .collect();
        let alpha_blends: Vec<i32> = (0..drawable_count)
            .map(|i| blend_modes.get(i).map_or(0, |b| (b >> 8) & 0xff))
            .collect();

        let render_orders_view = model
            .get_render_orders()
            .map_err(|e| js_error("Model.getRenderOrders", &e))?;
        let part_parent_indices = parts.parent_indices().to_vec();
        let part_count = part_parent_indices.len();
        let part_offscreen_indices = parts
            .offscreen_indices()
            .map(|a| a.to_vec())
            .unwrap_or_else(|| vec![-1; part_count]);
        let drawable_parent_part_indices = drawables.parent_part_indices().to_vec();

        let offscreen_count = offscreens.as_ref().map_or(0, |o| o.count() as usize);
        let offscreen_blend_modes = offscreens
            .as_ref()
            .and_then(ffi::Offscreens::blend_modes)
            .map(|a| a.to_vec())
            .unwrap_or_default();

        let core = CoreHandle {
            parameter_values,
            part_opacities,
            dynamic_flags: drawables.dynamic_flags(),
            drawable_opacities: drawables.opacities(),
            render_orders: render_orders_view,
            multiply_colors: drawables.multiply_colors(),
            screen_colors: drawables.screen_colors(),
            draw_orders: drawables.draw_orders(),
            offscreen_opacities: offscreens.as_ref().and_then(ffi::Offscreens::opacities),
            offscreen_multiply_colors: offscreens
                .as_ref()
                .and_then(ffi::Offscreens::multiply_colors),
            offscreen_screen_colors: offscreens.as_ref().and_then(ffi::Offscreens::screen_colors),
            drawables: drawables.clone(),
            model: model.clone(),
            moc,
        };

        let mut ret = CubismModel {
            state,
            core_lost: false,
            logged_update_error: false,
            canvas_width_px: canvas.canvas_width(),
            canvas_height_px: canvas.canvas_height(),
            canvas_origin_x: canvas.canvas_origin_x(),
            canvas_origin_y: canvas.canvas_origin_y(),
            pixels_per_unit: canvas.pixels_per_unit(),
            parameter_types: parameters.types().map(|a| a.to_vec()).unwrap_or_default(),
            parameter_key_values: parameters
                .key_values()
                .map(|a| {
                    a.iter()
                        .map(|v| {
                            v.dyn_into::<Float32Array>()
                                .map(|f| f.to_vec())
                                .unwrap_or_default()
                        })
                        .collect()
                })
                .unwrap_or_default(),
            parts_hierarchy: parts_hierarchy::setup_parts_hierarchy(
                &part_parent_indices,
                &drawable_parent_part_indices,
                &part_offscreen_indices,
            ),
            part_parent_indices,
            part_offscreen_indices,
            drawable_ids: strings(&drawables.ids()),
            texture_indices: drawables.texture_indices().to_vec(),
            constant_flags: drawables.constant_flags().to_vec(),
            color_blends,
            alpha_blends,
            mask_counts: drawables.mask_counts().to_vec(),
            masks: i32_arrays(&drawables.masks()),
            vertex_counts: drawables.vertex_counts().to_vec(),
            index_counts: drawables.index_counts().to_vec(),
            drawable_parent_part_indices,
            vertex_positions: typed_views(&drawables.vertex_positions()),
            vertex_uvs: typed_views(&drawables.vertex_uvs()),
            indices: typed_views(&drawables.indices()),
            dynamic_flags: Vec::new(),
            drawable_opacities: Vec::new(),
            render_orders: Vec::new(),
            multiply_colors: Vec::new(),
            screen_colors: Vec::new(),
            draw_orders: Vec::new(),
            offscreen_count,
            offscreen_color_blends: (0..offscreen_count)
                .map(|i| offscreen_blend_modes.get(i).map_or(0, |b| b & 0xff))
                .collect(),
            offscreen_alpha_blends: (0..offscreen_count)
                .map(|i| offscreen_blend_modes.get(i).map_or(0, |b| (b >> 8) & 0xff))
                .collect(),
            offscreen_owner_indices: offscreens
                .as_ref()
                .and_then(ffi::Offscreens::owner_indices)
                .map(|a| a.to_vec())
                .unwrap_or_default(),
            offscreen_mask_counts: offscreens
                .as_ref()
                .and_then(ffi::Offscreens::mask_counts)
                .map(|a| a.to_vec())
                .unwrap_or_default(),
            offscreen_masks: offscreens
                .as_ref()
                .and_then(ffi::Offscreens::masks)
                .map(|a| i32_arrays(&a))
                .unwrap_or_default(),
            offscreen_constant_flags: offscreens
                .as_ref()
                .and_then(ffi::Offscreens::constant_flags)
                .map(|a| a.to_vec())
                .unwrap_or_default(),
            offscreen_opacities: Vec::new(),
            offscreen_multiply_colors: Vec::new(),
            offscreen_screen_colors: Vec::new(),
            is_overridden_cullings: false,
            drawable_culling: CullingData::default(),
            offscreen_culling: CullingData::default(),
            color_overrides: CubismModelMultiplyAndScreenColor::new(),
            is_blend_mode_enabled: false,
            core: Some(core),
        };

        // Blend-mode detection (spec §7.2 step 6).
        ret.is_blend_mode_enabled = ret.offscreen_count > 0
            || (0..drawable_count).any(|i| {
                let c = ret.color_blends[i];
                let a = ret.alpha_blends[i];
                !(c == ffi::color_blend_type::NORMAL && a == ffi::alpha_blend_type::OVER)
                    && c != ffi::color_blend_type::ADD_COMPATIBLE
                    && c != ffi::color_blend_type::MULTIPLY_COMPATIBLE
            });
        ret.color_overrides
            .initialize(part_count, drawable_count, offscreen_count);
        ret.refresh_dynamic();
        Ok(ret)
    }

    /// Re-reads the dynamic drawable data from the Core views.
    fn refresh_dynamic(&mut self) {
        let Some(core) = self.core.as_ref() else {
            return;
        };
        let mut ok = pull_u8(&core.dynamic_flags, &mut self.dynamic_flags)
            && pull_f32(&core.drawable_opacities, &mut self.drawable_opacities)
            && pull_i32(&core.render_orders, &mut self.render_orders)
            && pull_f32(&core.multiply_colors, &mut self.multiply_colors)
            && pull_f32(&core.screen_colors, &mut self.screen_colors);
        if let Some(v) = core.draw_orders.as_ref() {
            ok &= pull_i32(v, &mut self.draw_orders);
        }
        if let Some(v) = core.offscreen_opacities.as_ref() {
            ok &= pull_f32(v, &mut self.offscreen_opacities);
        }
        if let Some(v) = core.offscreen_multiply_colors.as_ref() {
            ok &= pull_f32(v, &mut self.offscreen_multiply_colors);
        }
        if let Some(v) = core.offscreen_screen_colors.as_ref() {
            ok &= pull_f32(v, &mut self.offscreen_screen_colors);
        }
        if !ok {
            self.mark_core_lost();
        }
    }

    fn mark_core_lost(&mut self) {
        if !self.core_lost {
            log::error!(
                target: "cubism",
                "Live2DCubismCore heap views were detached (heap growth); the model must be recreated"
            );
        }
        self.core_lost = true;
    }

    // --------------------------------------------------------------------------- lifecycle

    /// `CubismModel.update()`: mirrors → Core, `Model.update()`, `drawables.resetDynamicFlags()`,
    /// Core (clamped values) → mirrors, refresh of the dynamic drawable caches.
    pub fn update(&mut self) {
        if self.core_lost {
            return;
        }
        let Some(core) = self.core.as_ref() else {
            return;
        };
        if core.parameter_values.length() as usize != self.state.parameter_count()
            || core.part_opacities.length() as usize != self.state.part_count()
        {
            self.mark_core_lost();
            return;
        }
        core.parameter_values
            .copy_from(self.state.parameter_values());
        core.part_opacities.copy_from(self.state.part_opacities());
        let result = core.model.update();
        let reset = core.drawables.reset_dynamic_flags();
        core.parameter_values
            .copy_to(self.state.parameter_values_mut());
        core.part_opacities.copy_to(self.state.part_opacities_mut());
        if let Err(e) = result.and(reset) {
            if !self.logged_update_error {
                self.logged_update_error = true;
                log::error!(target: "cubism", "Live2DCubismCore Model.update() failed: {e:?}");
            }
        }
        self.refresh_dynamic();
    }

    /// `release()`: frees the Core model (idempotent; also done on drop).
    pub fn release(&mut self) {
        if let Some(core) = self.core.take() {
            #[cfg(target_arch = "wasm32")]
            {
                if let Err(e) = core.model.release() {
                    log::warn!(target: "cubism", "Live2DCubismCore Model.release() failed: {e:?}");
                }
            }
            let count = core.moc.model_count.get();
            core.moc.model_count.set(count.saturating_sub(1));
        }
    }

    /// Whether `release()` has been called.
    pub fn is_released(&self) -> bool {
        self.core.is_none()
    }

    /// Whether the Core heap views were detached by a heap growth (the model must be recreated).
    pub fn is_core_lost(&self) -> bool {
        self.core_lost
    }

    /// `loadParameters()`: restores the saved real parameter values.
    pub fn load_parameters(&mut self) {
        self.state.load_parameters();
    }

    /// `saveParameters()`: snapshots the real parameter values.
    pub fn save_parameters(&mut self) {
        self.state.save_parameters();
    }

    /// The pure parameter/part state.
    pub fn state(&self) -> &ModelState {
        &self.state
    }

    /// The pure parameter/part state (mutable; changes reach the Core on the next `update()`).
    pub fn state_mut(&mut self) -> &mut ModelState {
        &mut self.state
    }

    // --------------------------------------------------------------------------- canvas

    /// `getCanvasWidth()` in model units (`CanvasWidth / PixelsPerUnit`; 0 after release).
    pub fn canvas_width(&self) -> f32 {
        self.canvas_width_f64() as f32
    }
    /// `getCanvasHeight()` in model units.
    pub fn canvas_height(&self) -> f32 {
        self.canvas_height_f64() as f32
    }
    /// `getPixelsPerUnit()`.
    pub fn pixels_per_unit(&self) -> f32 {
        self.pixels_per_unit_f64() as f32
    }
    /// `CanvasInfo.CanvasWidth` (pixels).
    pub fn canvas_width_pixel(&self) -> f32 {
        self.canvas_width_px as f32
    }
    /// `CanvasInfo.CanvasHeight` (pixels).
    pub fn canvas_height_pixel(&self) -> f32 {
        self.canvas_height_px as f32
    }
    /// `getCanvasWidth()` as a JS number (1.0 for the Kurisu model).
    pub fn canvas_width_f64(&self) -> f64 {
        if self.core.is_none() {
            return 0.0;
        }
        self.canvas_width_px / self.pixels_per_unit
    }
    /// `getCanvasHeight()` as a JS number (1.4285714285714286 for the Kurisu model).
    pub fn canvas_height_f64(&self) -> f64 {
        if self.core.is_none() {
            return 0.0;
        }
        self.canvas_height_px / self.pixels_per_unit
    }
    /// `getPixelsPerUnit()` as a JS number.
    pub fn pixels_per_unit_f64(&self) -> f64 {
        if self.core.is_none() {
            return 0.0;
        }
        self.pixels_per_unit
    }
    /// `CanvasInfo.CanvasOriginX/Y` (pixels).
    pub fn canvas_origin_pixel(&self) -> (f64, f64) {
        (self.canvas_origin_x, self.canvas_origin_y)
    }

    // --------------------------------------------------------------------------- parameters

    pub fn parameter_count(&self) -> usize {
        self.state.parameter_count()
    }
    /// `getParameterIndex(id)` — unknown IDs get virtual slots (`>= parameter_count()`).
    pub fn parameter_index(&mut self, id: &str) -> usize {
        self.state.parameter_index(id)
    }
    /// `getParameterId(i)` (`""` for virtual/out-of-range).
    pub fn parameter_id(&self, index: usize) -> &str {
        self.state.parameter_id(index)
    }
    /// `getParameterValueByIndex(i)` (virtual `null` → 0; out of range → NaN).
    pub fn parameter_value(&self, index: usize) -> f32 {
        self.state.parameter_value_f64(index) as f32
    }
    pub fn parameter_minimum(&self, index: usize) -> f32 {
        self.state.parameter_minimum_f64(index) as f32
    }
    pub fn parameter_maximum(&self, index: usize) -> f32 {
        self.state.parameter_maximum_f64(index) as f32
    }
    pub fn parameter_default(&self, index: usize) -> f32 {
        self.state.parameter_default_f64(index) as f32
    }
    /// `isRepeat(i)` (the editor flag is overridden by default → false).
    pub fn is_repeat(&self, index: usize) -> bool {
        ParameterStore::is_repeat(&self.state, index)
    }
    /// `setParameterValueByIndex(i, value, weight)` (clamped before weight blending).
    pub fn set_parameter_value(&mut self, index: usize, value: f32, weight: f32) {
        self.state
            .set_parameter_value_f64(index, f64::from(value), f64::from(weight));
    }
    pub fn add_parameter_value(&mut self, index: usize, value: f32, weight: f32) {
        self.state
            .add_parameter_value_f64(index, f64::from(value), f64::from(weight));
    }
    pub fn multiply_parameter_value(&mut self, index: usize, value: f32, weight: f32) {
        self.state
            .multiply_parameter_value_f64(index, f64::from(value), f64::from(weight));
    }
    pub fn parameter_value_by_id(&mut self, id: &str) -> f32 {
        self.state.parameter_value_by_id_f64(id) as f32
    }
    pub fn set_parameter_value_by_id(&mut self, id: &str, value: f32, weight: f32) {
        self.state
            .set_parameter_value_by_id_f64(id, f64::from(value), f64::from(weight));
    }
    pub fn add_parameter_value_by_id(&mut self, id: &str, value: f32, weight: f32) {
        self.state
            .add_parameter_value_by_id_f64(id, f64::from(value), f64::from(weight));
    }
    pub fn multiply_parameter_value_by_id(&mut self, id: &str, value: f32, weight: f32) {
        self.state
            .multiply_parameter_value_by_id_f64(id, f64::from(value), f64::from(weight));
    }
    /// `getParameterType(i)` (0 normal, 1 blend shape).
    pub fn parameter_type(&self, index: usize) -> i32 {
        self.parameter_types.get(index).copied().unwrap_or(0)
    }
    /// `parameters.keyValues[i]`.
    pub fn parameter_key_values(&self, index: usize) -> &[f32] {
        self.parameter_key_values
            .get(index)
            .map_or(&[], Vec::as_slice)
    }
    /// `getParameterRepeats(i)` (the editor's flag).
    pub fn parameter_repeats(&self, index: usize) -> bool {
        self.state.parameter_repeats(index)
    }
    /// `getOverrideFlagForModelParameterRepeat()`.
    pub fn override_flag_for_model_parameter_repeat(&self) -> bool {
        self.state.override_flag_for_model_parameter_repeat()
    }
    /// `setOverrideFlagForModelParameterRepeat(v)`.
    pub fn set_override_flag_for_model_parameter_repeat(&mut self, value: bool) {
        self.state
            .set_override_flag_for_model_parameter_repeat(value);
    }
    /// `getOverrideFlagForParameterRepeat(i)`.
    pub fn override_flag_for_parameter_repeat(&self, index: usize) -> bool {
        self.state.override_flag_for_parameter_repeat(index)
    }
    /// `setOverrideFlagForParameterRepeat(i, v)`.
    pub fn set_override_flag_for_parameter_repeat(&mut self, index: usize, value: bool) {
        self.state
            .set_override_flag_for_parameter_repeat(index, value);
    }
    /// `getRepeatFlagForParameterRepeat(i)`.
    pub fn repeat_flag_for_parameter_repeat(&self, index: usize) -> bool {
        self.state.repeat_flag_for_parameter_repeat(index)
    }
    /// `setRepeatFlagForParameterRepeat(i, v)`.
    pub fn set_repeat_flag_for_parameter_repeat(&mut self, index: usize, value: bool) {
        self.state
            .set_repeat_flag_for_parameter_repeat(index, value);
    }

    // --------------------------------------------------------------------------- parts

    pub fn part_count(&self) -> usize {
        self.state.part_count()
    }
    /// `getPartIndex(id)` — unknown IDs get virtual slots.
    pub fn part_index(&mut self, id: &str) -> usize {
        self.state.part_index(id)
    }
    pub fn part_id(&self, index: usize) -> &str {
        self.state.part_id(index)
    }
    pub fn part_opacity(&self, index: usize) -> f32 {
        self.state.part_opacity_f64(index) as f32
    }
    /// `setPartOpacityByIndex(i, opacity)` (no clamp; the Core clamps on update).
    pub fn set_part_opacity(&mut self, index: usize, opacity: f32) {
        self.state.set_part_opacity_f64(index, f64::from(opacity));
    }
    /// `parts.parentIndices[i]` (−1 = none).
    pub fn part_parent_index(&self, index: usize) -> i32 {
        self.part_parent_indices.get(index).copied().unwrap_or(-1)
    }
    /// `getPartParentPartIndices()`.
    pub fn part_parent_part_indices(&self) -> &[i32] {
        &self.part_parent_indices
    }
    /// `getPartOffscreenIndices()` (−1 = none).
    pub fn part_offscreen_indices(&self) -> &[i32] {
        &self.part_offscreen_indices
    }
    /// `getPartsHierarchy()`.
    pub fn parts_hierarchy(&self) -> &[CubismModelPartInfo] {
        &self.parts_hierarchy
    }

    // --------------------------------------------------------------------------- drawables

    pub fn drawable_count(&self) -> usize {
        self.drawable_ids.len()
    }
    pub fn drawable_id(&self, index: usize) -> &str {
        self.drawable_ids.get(index).map_or("", String::as_str)
    }
    /// `getDrawableIndex(id)` (`None` = −1).
    pub fn drawable_index(&self, id: &str) -> Option<usize> {
        self.drawable_ids.iter().position(|d| d == id)
    }
    /// `getRenderOrders()` — drawing position per object (drawables, then offscreens).
    pub fn render_orders(&self) -> &[i32] {
        &self.render_orders
    }
    pub fn drawable_texture_index(&self, index: usize) -> usize {
        self.texture_indices
            .get(index)
            .map_or(0, |t| (*t).max(0) as usize)
    }
    pub fn drawable_opacity(&self, index: usize) -> f32 {
        self.drawable_opacities.get(index).copied().unwrap_or(0.0)
    }
    /// `getDrawableBlendMode(i)` from the constant flags.
    pub fn drawable_blend_mode(&self, index: usize) -> BlendMode {
        let flags = self.constant_flags.get(index).copied().unwrap_or(0);
        if ffi::utils::has_blend_additive_bit(flags) {
            BlendMode::Additive
        } else if ffi::utils::has_blend_multiplicative_bit(flags) {
            BlendMode::Multiplicative
        } else {
            BlendMode::Normal
        }
    }
    /// `getDrawableColorBlend(i)` (Cubism 5.3).
    pub fn drawable_color_blend(&self, index: usize) -> ColorBlend {
        ColorBlend::from_i32(self.color_blends.get(index).copied().unwrap_or(-1))
    }
    /// `getDrawableAlphaBlend(i)` (Cubism 5.3).
    pub fn drawable_alpha_blend(&self, index: usize) -> AlphaBlend {
        AlphaBlend::from_i32(self.alpha_blends.get(index).copied().unwrap_or(-1))
    }
    /// `isBlendModeEnabled()` (false for the Kurisu model).
    pub fn is_blend_mode_enabled(&self) -> bool {
        self.is_blend_mode_enabled
    }
    /// `getDrawableCulling(i)`: true = cull back faces (honours the shared override).
    pub fn drawable_culling(&self, index: usize) -> bool {
        if self.is_overridden_cullings || self.drawable_culling.is_overridden {
            return self.drawable_culling.is_culling;
        }
        !ffi::utils::has_is_double_sided_bit(self.constant_flags.get(index).copied().unwrap_or(0))
    }
    /// `getDrawableInvertedMaskBit(i)`.
    pub fn drawable_is_inverted_mask(&self, index: usize) -> bool {
        ffi::utils::has_is_inverted_mask_bit(self.constant_flags.get(index).copied().unwrap_or(0))
    }
    /// `drawables.constantFlags[i]`.
    pub fn drawable_constant_flags(&self, index: usize) -> u8 {
        self.constant_flags.get(index).copied().unwrap_or(0)
    }
    /// `drawables.dynamicFlags[i]` (as of the last `update()`).
    pub fn drawable_dynamic_flags(&self, index: usize) -> u8 {
        self.dynamic_flags.get(index).copied().unwrap_or(0)
    }
    pub fn drawable_is_visible(&self, index: usize) -> bool {
        ffi::utils::has_is_visible_bit(self.drawable_dynamic_flags(index))
    }
    pub fn drawable_visibility_did_change(&self, index: usize) -> bool {
        ffi::utils::has_visibility_did_change_bit(self.drawable_dynamic_flags(index))
    }
    pub fn drawable_vertex_positions_did_change(&self, index: usize) -> bool {
        ffi::utils::has_vertex_positions_did_change_bit(self.drawable_dynamic_flags(index))
    }
    pub fn drawable_opacity_did_change(&self, index: usize) -> bool {
        ffi::utils::has_opacity_did_change_bit(self.drawable_dynamic_flags(index))
    }
    pub fn drawable_render_order_did_change(&self, index: usize) -> bool {
        ffi::utils::has_render_order_did_change_bit(self.drawable_dynamic_flags(index))
    }
    pub fn drawable_draw_order_did_change(&self, index: usize) -> bool {
        ffi::utils::has_draw_order_did_change_bit(self.drawable_dynamic_flags(index))
    }
    pub fn drawable_blend_color_did_change(&self, index: usize) -> bool {
        ffi::utils::has_blend_color_did_change_bit(self.drawable_dynamic_flags(index))
    }
    /// `drawables.drawOrders[i]`.
    pub fn drawable_draw_order(&self, index: usize) -> i32 {
        self.draw_orders.get(index).copied().unwrap_or(0)
    }
    pub fn drawable_vertex_count(&self, index: usize) -> usize {
        self.vertex_counts
            .get(index)
            .map_or(0, |c| (*c).max(0) as usize)
    }
    /// `drawables.vertexPositions[i]` — the Core's view (xy pairs; empty after release).
    pub fn drawable_vertex_positions(&self, index: usize) -> Float32Array {
        match (self.core.as_ref(), self.vertex_positions.get(index)) {
            (Some(_), Some(v)) => v.clone(),
            _ => Float32Array::new_with_length(0),
        }
    }
    /// Copy of `vertexPositions[i]` (for bounds / hit tests).
    pub fn drawable_vertex_positions_vec(&self, index: usize) -> Vec<f32> {
        match (self.core.as_ref(), self.vertex_positions.get(index)) {
            (Some(_), Some(v)) => v.to_vec(),
            _ => Vec::new(),
        }
    }
    /// `drawables.vertexUvs[i]` — the Core's view.
    pub fn drawable_vertex_uvs(&self, index: usize) -> Float32Array {
        match (self.core.as_ref(), self.vertex_uvs.get(index)) {
            (Some(_), Some(v)) => v.clone(),
            _ => Float32Array::new_with_length(0),
        }
    }
    pub fn drawable_index_count(&self, index: usize) -> usize {
        self.index_counts
            .get(index)
            .map_or(0, |c| (*c).max(0) as usize)
    }
    /// `drawables.indices[i]` — the Core's view.
    pub fn drawable_indices(&self, index: usize) -> Uint16Array {
        match (self.core.as_ref(), self.indices.get(index)) {
            (Some(_), Some(v)) => v.clone(),
            _ => Uint16Array::new_with_length(0),
        }
    }
    /// `getDrawableMaskCounts()`.
    pub fn drawable_mask_counts(&self) -> &[i32] {
        &self.mask_counts
    }
    /// `getDrawableMasks()[i]`.
    pub fn drawable_masks(&self, index: usize) -> &[i32] {
        self.masks.get(index).map_or(&[], Vec::as_slice)
    }
    /// Multiply colour honouring the overrides (`getOverrideMultiplyAndScreenColor()
    /// .getDrawableMultiplyColor(i)`).
    pub fn drawable_multiply_color(&self, index: usize) -> [f32; 4] {
        match self.color_overrides.drawable_multiply_color(index) {
            Err(default) => default.to_f32_array(),
            Ok(Some(user)) => user.to_f32_array(),
            Ok(None) => color4(&self.multiply_colors, index),
        }
    }
    /// Screen colour honouring the overrides.
    pub fn drawable_screen_color(&self, index: usize) -> [f32; 4] {
        match self.color_overrides.drawable_screen_color(index) {
            Err(default) => default.to_f32_array(),
            Ok(Some(user)) => user.to_f32_array(),
            Ok(None) => {
                let i = index * 4;
                match self.screen_colors.get(i..i + 4) {
                    Some(c) => [c[0], c[1], c[2], c[3]],
                    None => [0.0, 0.0, 0.0, 1.0],
                }
            }
        }
    }
    /// The Core's multiply colour (`CubismModel.getDrawableMultiplyColor(i)`, no overrides).
    pub fn core_drawable_multiply_color(&self, index: usize) -> [f32; 4] {
        color4(&self.multiply_colors, index)
    }
    /// The Core's screen colour (no overrides).
    pub fn core_drawable_screen_color(&self, index: usize) -> [f32; 4] {
        color4(&self.screen_colors, index)
    }
    pub fn drawable_parent_part_index(&self, index: usize) -> i32 {
        self.drawable_parent_part_indices
            .get(index)
            .copied()
            .unwrap_or(-1)
    }
    /// `isUsingMasking()`: some drawable has masks.
    pub fn is_using_masking(&self) -> bool {
        self.mask_counts.iter().any(|c| *c > 0)
    }
    /// `drawables.resetDynamicFlags()` (Core 6.0.1: no visible effect, see render spec §4.2).
    pub fn reset_dynamic_flags(&mut self) {
        if let Some(core) = self.core.as_ref() {
            let _ = core.drawables.reset_dynamic_flags();
            if !pull_u8(&core.dynamic_flags, &mut self.dynamic_flags) {
                self.mark_core_lost();
            }
        }
    }

    // --------------------------------------------------------------------------- culling overrides

    /// `setDrawableCulling(i, v)` — QUIRK: one shared value for all drawables.
    pub fn set_drawable_culling(&mut self, _index: usize, is_culling: bool) {
        self.drawable_culling.is_culling = is_culling;
    }
    /// `setOverrideFlagForDrawableCullings(i, v)` — shared, like the TS.
    pub fn set_override_flag_for_drawable_cullings(&mut self, _index: usize, is_overridden: bool) {
        self.drawable_culling.is_overridden = is_overridden;
    }
    /// `getOverrideFlagForDrawableCullings(i)`.
    pub fn override_flag_for_drawable_cullings(&self, _index: usize) -> bool {
        self.drawable_culling.is_overridden
    }
    /// `getOverrideFlagForModelCullings()`.
    pub fn override_flag_for_model_cullings(&self) -> bool {
        self.is_overridden_cullings
    }
    /// `setOverrideFlagForModelCullings(v)`.
    pub fn set_override_flag_for_model_cullings(&mut self, value: bool) {
        self.is_overridden_cullings = value;
    }

    // --------------------------------------------------------------------------- colour overrides

    /// `getOverrideMultiplyAndScreenColor()`.
    pub fn color_overrides(&self) -> &CubismModelMultiplyAndScreenColor {
        &self.color_overrides
    }
    /// Drawable/offscreen/model-wide override setters live on the returned object.
    pub fn color_overrides_mut(&mut self) -> &mut CubismModelMultiplyAndScreenColor {
        &mut self.color_overrides
    }
    /// `setPartMultiplyColorEnabled(part, v)` (propagates to the part's children).
    pub fn set_part_multiply_color_enabled(&mut self, part: usize, value: bool) {
        let ctx = PartsContext {
            parts_hierarchy: &self.parts_hierarchy,
            part_offscreen_indices: &self.part_offscreen_indices,
        };
        self.color_overrides
            .set_part_multiply_color_enabled(ctx, part, value);
    }
    /// `setPartScreenColorEnabled(part, v)`.
    pub fn set_part_screen_color_enabled(&mut self, part: usize, value: bool) {
        let ctx = PartsContext {
            parts_hierarchy: &self.parts_hierarchy,
            part_offscreen_indices: &self.part_offscreen_indices,
        };
        self.color_overrides
            .set_part_screen_color_enabled(ctx, part, value);
    }
    /// `setPartMultiplyColorByRGBA(part, r, g, b, a)`.
    pub fn set_part_multiply_color(&mut self, part: usize, color: CubismTextureColor) {
        let ctx = PartsContext {
            parts_hierarchy: &self.parts_hierarchy,
            part_offscreen_indices: &self.part_offscreen_indices,
        };
        self.color_overrides
            .set_part_multiply_color(ctx, part, color);
    }
    /// `setPartScreenColorByRGBA(part, r, g, b, a)`.
    pub fn set_part_screen_color(&mut self, part: usize, color: CubismTextureColor) {
        let ctx = PartsContext {
            parts_hierarchy: &self.parts_hierarchy,
            part_offscreen_indices: &self.part_offscreen_indices,
        };
        self.color_overrides.set_part_screen_color(ctx, part, color);
    }

    // --------------------------------------------------------------------------- offscreens (5.3)

    pub fn offscreen_count(&self) -> usize {
        self.offscreen_count
    }
    /// `getOffscreenOpacity(o)` (1.0 out of range).
    pub fn offscreen_opacity(&self, index: usize) -> f32 {
        if index >= self.offscreen_count {
            return 1.0;
        }
        self.offscreen_opacities.get(index).copied().unwrap_or(1.0)
    }
    pub fn offscreen_color_blend(&self, index: usize) -> ColorBlend {
        ColorBlend::from_i32(
            self.offscreen_color_blends
                .get(index)
                .copied()
                .unwrap_or(-1),
        )
    }
    pub fn offscreen_alpha_blend(&self, index: usize) -> AlphaBlend {
        AlphaBlend::from_i32(
            self.offscreen_alpha_blends
                .get(index)
                .copied()
                .unwrap_or(-1),
        )
    }
    /// `getOffscreenOwnerIndices()` (owner part per offscreen).
    pub fn offscreen_owner_indices(&self) -> &[i32] {
        &self.offscreen_owner_indices
    }
    pub fn offscreen_mask_counts(&self) -> &[i32] {
        &self.offscreen_mask_counts
    }
    pub fn offscreen_masks(&self, index: usize) -> &[i32] {
        self.offscreen_masks.get(index).map_or(&[], Vec::as_slice)
    }
    pub fn offscreen_inverted_mask(&self, index: usize) -> bool {
        ffi::utils::has_is_inverted_mask_bit(
            self.offscreen_constant_flags
                .get(index)
                .copied()
                .unwrap_or(0),
        )
    }
    /// `getOffscreenCulling(o)` (shared override like drawables).
    pub fn offscreen_culling(&self, index: usize) -> bool {
        if self.is_overridden_cullings || self.offscreen_culling.is_overridden {
            return self.offscreen_culling.is_culling;
        }
        !ffi::utils::has_is_double_sided_bit(
            self.offscreen_constant_flags
                .get(index)
                .copied()
                .unwrap_or(0),
        )
    }
    /// `setOffscreenCulling(o, v)` — shared.
    pub fn set_offscreen_culling(&mut self, _index: usize, is_culling: bool) {
        self.offscreen_culling.is_culling = is_culling;
    }
    /// `isUsingMaskingForOffscreen()`.
    pub fn is_using_masking_for_offscreen(&self) -> bool {
        self.offscreen_mask_counts
            .iter()
            .take(self.offscreen_count)
            .any(|c| *c > 0)
    }
    /// Offscreen multiply colour honouring the overrides.
    pub fn offscreen_multiply_color(&self, index: usize) -> [f32; 4] {
        match self.color_overrides.offscreen_multiply_color(index) {
            Err(default) => default.to_f32_array(),
            Ok(Some(user)) => user.to_f32_array(),
            Ok(None) => color4(&self.offscreen_multiply_colors, index),
        }
    }
    /// Offscreen screen colour honouring the overrides.
    pub fn offscreen_screen_color(&self, index: usize) -> [f32; 4] {
        match self.color_overrides.offscreen_screen_color(index) {
            Err(default) => default.to_f32_array(),
            Ok(Some(user)) => user.to_f32_array(),
            Ok(None) => {
                let i = index * 4;
                match self.offscreen_screen_colors.get(i..i + 4) {
                    Some(c) => [c[0], c[1], c[2], c[3]],
                    None => [0.0, 0.0, 0.0, 1.0],
                }
            }
        }
    }

    /// The Core moc3 buffer helper for callers holding raw bytes (`ArrayBuffer` copy).
    pub fn bytes_to_array_buffer(bytes: &[u8]) -> ArrayBuffer {
        Uint8Array::from(bytes).buffer()
    }
}

impl Drop for CubismModel {
    fn drop(&mut self) {
        self.release();
    }
}

impl ParameterStore for CubismModel {
    fn model_id(&self) -> u64 {
        self.state.model_id()
    }
    fn parameter_count(&self) -> usize {
        self.state.parameter_count()
    }
    fn parameter_index(&mut self, id: &str) -> usize {
        self.state.parameter_index(id)
    }
    fn parameter_value_f64(&self, index: usize) -> f64 {
        self.state.parameter_value_f64(index)
    }
    fn parameter_minimum_f64(&self, index: usize) -> f64 {
        self.state.parameter_minimum_f64(index)
    }
    fn parameter_maximum_f64(&self, index: usize) -> f64 {
        self.state.parameter_maximum_f64(index)
    }
    fn parameter_default_f64(&self, index: usize) -> f64 {
        self.state.parameter_default_f64(index)
    }
    fn set_parameter_value_f64(&mut self, index: usize, value: f64, weight: f64) {
        self.state.set_parameter_value_f64(index, value, weight);
    }
    fn is_repeat(&self, index: usize) -> bool {
        ParameterStore::is_repeat(&self.state, index)
    }
    fn parameter_repeat_value(&self, index: usize, value: f64) -> f64 {
        self.state.parameter_repeat_value(index, value)
    }
    fn parameter_values(&self) -> &[f32] {
        self.state.parameter_values()
    }
    fn parameter_values_mut(&mut self) -> &mut [f32] {
        self.state.parameter_values_mut()
    }
    fn parameter_minimums(&self) -> &[f32] {
        self.state.parameter_minimums()
    }
    fn parameter_maximums(&self) -> &[f32] {
        self.state.parameter_maximums()
    }
    fn parameter_defaults(&self) -> &[f32] {
        self.state.parameter_defaults()
    }
    fn part_count(&self) -> usize {
        self.state.part_count()
    }
    fn part_index(&mut self, id: &str) -> usize {
        self.state.part_index(id)
    }
    fn part_opacity_f64(&self, index: usize) -> f64 {
        self.state.part_opacity_f64(index)
    }
    fn set_part_opacity_f64(&mut self, index: usize, opacity: f64) {
        self.state.set_part_opacity_f64(index, opacity);
    }
    fn model_opacity(&self) -> f64 {
        self.state.model_opacity()
    }
    fn set_model_opacity(&mut self, value: f64) {
        self.state.set_model_opacity(value);
    }
}
