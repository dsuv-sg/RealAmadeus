//! Cubism Core access layer.
//!
//! * [`ffi`]   — wasm-bindgen bindings to the global `Live2DCubismCore` object (Moc, Model,
//!   Parameters, Parts, Drawables, Offscreens, CanvasInfo, Utils, Version, Memory, Logging).
//! * [`model`] — [`CubismModel`](model::CubismModel): the Rust equivalent of the Framework's `CubismModel`
//!   (`Framework/src/model/cubismmodel.ts`) — parameters, parts, drawables, colors, culling,
//!   blend modes, save/load parameters — plus [`CubismMoc`](model::CubismMoc).
//!
//! ## Contract (other modules rely on exactly these items)
//!
//! Parameter and part values are mirrored in Rust `Vec<f32>`s. All framework code
//! (motion, physics, effects, the player) reads/writes the mirrors; [`CubismModel::update`](model::CubismModel::update)
//! copies the mirrors into the Core's typed arrays, calls `Model.update()`, and refreshes
//! cached dynamic drawable data. Drawable geometry is exposed as the Core's own JS typed-array
//! views (stable views into the Core heap) so the renderer can upload them to WebGL without
//! copying through WASM memory.
//!
//! ```ignore
//! pub struct CubismMoc { /* Live2DCubismCore.Moc */ }
//! impl CubismMoc {
//!     pub fn from_bytes(bytes: &[u8], check_consistency: bool) -> Result<CubismMoc, Live2DError>;
//!     pub fn moc_version(&self) -> u32;
//!     pub fn create_model(&self) -> Result<CubismModel, Live2DError>;
//! }
//!
//! pub struct CubismModel { /* … */ }
//! impl CubismModel {
//!     // --- lifecycle
//!     pub fn update(&mut self);                 // mirrors -> core, core.update(), refresh dynamic caches
//!     pub fn release(&mut self);
//!     pub fn load_parameters(&mut self);        // restore saved parameter values (Framework semantics)
//!     pub fn save_parameters(&mut self);
//!     // --- canvas
//!     pub fn canvas_width(&self) -> f32;        // in units (CanvasInfo.CanvasWidth / PixelsPerUnit)
//!     pub fn canvas_height(&self) -> f32;
//!     pub fn pixels_per_unit(&self) -> f32;
//!     pub fn canvas_width_pixel(&self) -> f32;
//!     pub fn canvas_height_pixel(&self) -> f32;
//!     // --- parameters (index based; ids not present in the model are tracked like the
//!     //     Framework's "not exist" parameters so set/get by id never panics)
//!     pub fn parameter_count(&self) -> usize;
//!     pub fn parameter_index(&mut self, id: &str) -> usize;           // Framework getParameterIndex
//!     pub fn parameter_id(&self, index: usize) -> &str;
//!     pub fn parameter_value(&self, index: usize) -> f32;
//!     pub fn parameter_minimum(&self, index: usize) -> f32;
//!     pub fn parameter_maximum(&self, index: usize) -> f32;
//!     pub fn parameter_default(&self, index: usize) -> f32;
//!     pub fn is_repeat(&self, index: usize) -> bool;
//!     pub fn set_parameter_value(&mut self, index: usize, value: f32, weight: f32);   // clamps like TS
//!     pub fn add_parameter_value(&mut self, index: usize, value: f32, weight: f32);
//!     pub fn multiply_parameter_value(&mut self, index: usize, value: f32, weight: f32);
//!     pub fn parameter_value_by_id(&mut self, id: &str) -> f32;
//!     pub fn set_parameter_value_by_id(&mut self, id: &str, value: f32, weight: f32);
//!     pub fn add_parameter_value_by_id(&mut self, id: &str, value: f32, weight: f32);
//!     pub fn multiply_parameter_value_by_id(&mut self, id: &str, value: f32, weight: f32);
//!     // --- parts
//!     pub fn part_count(&self) -> usize;
//!     pub fn part_index(&mut self, id: &str) -> usize;
//!     pub fn part_id(&self, index: usize) -> &str;
//!     pub fn part_opacity(&self, index: usize) -> f32;
//!     pub fn set_part_opacity(&mut self, index: usize, opacity: f32);
//!     pub fn part_parent_index(&self, index: usize) -> i32;
//!     // --- drawables (renderer)
//!     pub fn drawable_count(&self) -> usize;
//!     pub fn drawable_id(&self, index: usize) -> &str;
//!     pub fn drawable_index(&self, id: &str) -> Option<usize>;
//!     pub fn render_orders(&self) -> &[i32];                          // refreshed by update()
//!     pub fn drawable_texture_index(&self, index: usize) -> usize;
//!     pub fn drawable_opacity(&self, index: usize) -> f32;
//!     pub fn drawable_blend_mode(&self, index: usize) -> BlendMode;
//!     pub fn drawable_culling(&self, index: usize) -> bool;            // true = cull back faces (not double sided), honoring overrides
//!     pub fn drawable_is_inverted_mask(&self, index: usize) -> bool;
//!     pub fn drawable_is_visible(&self, index: usize) -> bool;         // dynamic flag
//!     pub fn drawable_vertex_positions_did_change(&self, index: usize) -> bool;
//!     pub fn drawable_opacity_did_change(&self, index: usize) -> bool;
//!     pub fn drawable_render_order_did_change(&self, index: usize) -> bool;
//!     pub fn drawable_blend_color_did_change(&self, index: usize) -> bool;
//!     pub fn drawable_vertex_count(&self, index: usize) -> usize;
//!     pub fn drawable_vertex_positions(&self, index: usize) -> js_sys::Float32Array; // xy pairs
//!     pub fn drawable_vertex_uvs(&self, index: usize) -> js_sys::Float32Array;
//!     pub fn drawable_index_count(&self, index: usize) -> usize;
//!     pub fn drawable_indices(&self, index: usize) -> js_sys::Uint16Array;
//!     pub fn drawable_mask_counts(&self) -> &[i32];
//!     pub fn drawable_masks(&self, index: usize) -> &[i32];
//!     pub fn drawable_multiply_color(&self, index: usize) -> [f32; 4];  // honoring overrides
//!     pub fn drawable_screen_color(&self, index: usize) -> [f32; 4];
//!     pub fn drawable_parent_part_index(&self, index: usize) -> i32;
//!     pub fn is_using_masking(&self) -> bool;
//!     pub fn reset_dynamic_flags(&mut self);                          // Drawables.resetDynamicFlags
//! }
//!
//! #[derive(Clone, Copy, PartialEq, Eq, Debug)]
//! pub enum BlendMode { Normal, Additive, Multiplicative /* + Cubism 5.3 color/alpha blend types if the model needs them */ }
//!
//! #[derive(Debug, thiserror::Error)]
//! pub enum Live2DError { CoreNotLoaded, InvalidMoc(String), Js(String), Load(String) }
//! ```
//! Implementations may add items but must not remove or rename these.
//!
//! ## Implementation notes and additions (framework engineer)
//!
//! * Semantics are those of `cubismmodel.ts` (spec `spec-live2d-framework-model.md` §7): values
//!   are clamped (or wrapped for repeat parameters, which the default override disables) **before**
//!   weight blending; unknown IDs get virtual slots `>= parameter_count()` / `>= part_count()` whose
//!   values are unclamped `f64`, never saved/restored and never sent to the Core.
//! * `update()` = push mirrors → `Model.update()` → `drawables.resetDynamicFlags()` → pull the
//!   Core-clamped values back → refresh dynamic flags, opacities, render orders and multiply/screen
//!   colours. A detached heap view (heap growth) sets [`model::CubismModel::is_core_lost`]
//!   instead of panicking. `release()` is idempotent and also runs on drop; the moc's Core memory is
//!   freed when the `CubismMoc` and all its models are gone.
//! * `CubismModel` implements [`crate::framework::ParameterStore`] (`f64` JS-number API used by
//!   physics/breath/look/motions/expressions/updaters). The `f32` methods above are conveniences
//!   for the player/renderer (`value as f32` before the clamp is equivalent for weight 1).
//! * Additions: `state()/state_mut()` (the pure [`crate::framework::ModelState`]),
//!   `canvas_width_f64()/canvas_height_f64()/pixels_per_unit_f64()/canvas_origin_pixel()`,
//!   `parameter_type()/parameter_key_values()/parameter_repeats()` and the repeat override flags,
//!   `part_parent_part_indices()/part_offscreen_indices()/parts_hierarchy()`,
//!   `drawable_color_blend()/drawable_alpha_blend()` ([`model::ColorBlend`], [`model::AlphaBlend`]),
//!   `is_blend_mode_enabled()`, `drawable_dynamic_flags()/drawable_constant_flags()`, the remaining
//!   dynamic-flag getters, `drawable_draw_order()`, `drawable_vertex_positions_vec()`,
//!   `core_drawable_multiply_color()/core_drawable_screen_color()` (no overrides), the culling
//!   override setters (one shared `CullingData`, like the TS), the colour-override API
//!   (`color_overrides()/color_overrides_mut()`, `set_part_multiply_color*`, …), the Cubism 5.3
//!   offscreen accessors (`offscreen_count()` is 0 for the shipped model), `is_released()`,
//!   `CubismMoc::{latest_moc_version, moc_version_from_bytes, has_moc_consistency, model_count,
//!   release}`.
//! * [`ffi::core_available`] / [`ffi::core_version`] answer Core readiness (poll them after
//!   injecting `live2dcubismcore.min.js`); [`ffi::register_core_log_handler`] (called by
//!   `CubismMoc::from_bytes` and `framework::start_up`) installs the Core log handler once per page
//!   (a call made before the Core is ready is a no-op, so registration happens on the first call
//!   after readiness). `framework::acquire()` / `framework::release()` are the ref-counted V1
//!   `acquireFramework` / `releaseFramework` (re-acquiring after a full release is safe).
//! * Verified in headless Chrome against the real Core 6.0.1 (scratch harness): counts/versions/
//!   canvas/IDs/hierarchy/render orders, write-back clamping (incl. NaN and part opacities), dynamic
//!   flags (127 → 33), virtual slots, release/drop, invalid/truncated/inconsistent mocs, and all five
//!   V1 pipeline goldens bit-exact through the real `Model.update()`.

pub mod ffi;
pub mod model;
