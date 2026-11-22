//! [`ParameterStore`] — the parameter/part API of the Framework's `CubismModel` with JS-number
//! (`f64`) semantics. Every framework algorithm (physics, breath, look, eye blink, pose, motions,
//! expressions, updaters) talks to the model only through this trait, so it is implemented by the
//! Core-backed [`crate::core::model::CubismModel`] and by the pure
//! [`crate::framework::model_state::ModelState`] (which doubles as the native test double).
//!
//! Semantics (spec §7.4/§7.5, `cubismmodel.ts`):
//! * Real parameters live in `f32` arrays (the Core's `Float32Array` views); every store rounds to
//!   `f32`, every read widens to `f64`.
//! * Unknown IDs get *virtual* slots `index >= parameter_count()` (never `-1`) whose values are full
//!   `f64`, start as JS `null` (read as `0.0`), are never clamped, saved, restored or seen by the Core.
//! * `set` clamps (or wraps for repeat parameters) **before** weight blending.
//!
//! The `*_f64` names keep these JS-precision methods apart from the `f32` convenience methods of the
//! `CubismModel` contract.

/// JS-semantics parameter/part access (see the module docs).
pub trait ParameterStore {
    /// Unique identity of the model instance (`CubismPose` resets when the model changes).
    fn model_id(&self) -> u64;

    // ---------------------------------------------------------------- parameters
    /// `getParameterCount()` — number of real parameters.
    fn parameter_count(&self) -> usize;
    /// `getParameterIndex(id)` — registers a virtual slot for unknown IDs; never fails.
    fn parameter_index(&mut self, id: &str) -> usize;
    /// `getParameterValueByIndex(i)`: f32 value widened; virtual → stored value (`null` → 0.0);
    /// unregistered out-of-range → NaN (JS `undefined`).
    fn parameter_value_f64(&self, index: usize) -> f64;
    /// `getParameterMinimumValue(i)` (NaN for virtual/out-of-range, i.e. JS `undefined`).
    fn parameter_minimum_f64(&self, index: usize) -> f64;
    /// `getParameterMaximumValue(i)` (NaN for virtual/out-of-range).
    fn parameter_maximum_f64(&self, index: usize) -> f64;
    /// `getParameterDefaultValue(i)` (NaN for virtual/out-of-range).
    fn parameter_default_f64(&self, index: usize) -> f64;
    /// `setParameterValueByIndex(i, value, weight)`.
    fn set_parameter_value_f64(&mut self, index: usize, value: f64, weight: f64);
    /// `isRepeat(i)` (false for virtual slots).
    fn is_repeat(&self, index: usize) -> bool;
    /// `getParameterRepeatValue(i, value)`.
    fn parameter_repeat_value(&self, index: usize, value: f64) -> f64;

    /// `addParameterValueByIndex(i, value, weight)` = `set(i, get(i) + value * weight, 1)`.
    fn add_parameter_value_f64(&mut self, index: usize, value: f64, weight: f64) {
        let current = self.parameter_value_f64(index);
        self.set_parameter_value_f64(index, current + value * weight, 1.0);
    }
    /// `multiplyParameterValueByIndex(i, value, weight)` = `set(i, get(i) * (1 + (value - 1) * weight), 1)`.
    fn multiply_parameter_value_f64(&mut self, index: usize, value: f64, weight: f64) {
        let current = self.parameter_value_f64(index);
        self.set_parameter_value_f64(index, current * (1.0 + (value - 1.0) * weight), 1.0);
    }
    /// `getParameterValueById(id)`.
    fn parameter_value_by_id_f64(&mut self, id: &str) -> f64 {
        let index = self.parameter_index(id);
        self.parameter_value_f64(index)
    }
    /// `setParameterValueById(id, value, weight)`.
    fn set_parameter_value_by_id_f64(&mut self, id: &str, value: f64, weight: f64) {
        let index = self.parameter_index(id);
        self.set_parameter_value_f64(index, value, weight);
    }
    /// `addParameterValueById(id, value, weight)`.
    fn add_parameter_value_by_id_f64(&mut self, id: &str, value: f64, weight: f64) {
        let index = self.parameter_index(id);
        self.add_parameter_value_f64(index, value, weight);
    }
    /// `multiplyParameterValueById(id, value, weight)`.
    fn multiply_parameter_value_by_id_f64(&mut self, id: &str, value: f64, weight: f64) {
        let index = self.parameter_index(id);
        self.multiply_parameter_value_f64(index, value, weight);
    }

    // ---------------------------------------------------------------- raw arrays (physics)
    /// `model.getModel().parameters.values` (real parameters only).
    fn parameter_values(&self) -> &[f32];
    /// Mutable `parameters.values` (physics writes interpolated outputs straight into it).
    fn parameter_values_mut(&mut self) -> &mut [f32];
    /// `parameters.minimumValues`.
    fn parameter_minimums(&self) -> &[f32];
    /// `parameters.maximumValues`.
    fn parameter_maximums(&self) -> &[f32];
    /// `parameters.defaultValues`.
    fn parameter_defaults(&self) -> &[f32];

    // ---------------------------------------------------------------- parts
    /// `getPartCount()`.
    fn part_count(&self) -> usize;
    /// `getPartIndex(id)` — registers a virtual slot for unknown IDs.
    fn part_index(&mut self, id: &str) -> usize;
    /// `getPartOpacityByIndex(i)` (virtual `null` → 0.0; unregistered out-of-range → NaN).
    fn part_opacity_f64(&self, index: usize) -> f64;
    /// `setPartOpacityByIndex(i, opacity)` (f32 store, no clamp — the Core clamps on update).
    fn set_part_opacity_f64(&mut self, index: usize, opacity: f64);
    /// `getPartOpacityById(id)`.
    fn part_opacity_by_id_f64(&mut self, id: &str) -> f64 {
        let index = self.part_index(id);
        self.part_opacity_f64(index)
    }
    /// `setPartOpacityById(id, opacity)`.
    fn set_part_opacity_by_id_f64(&mut self, id: &str, opacity: f64) {
        let index = self.part_index(id);
        self.set_part_opacity_f64(index, opacity);
    }

    // ---------------------------------------------------------------- misc
    /// `getModelOapcity()` (sic) — written by motions with a Model `Opacity` curve.
    fn model_opacity(&self) -> f64;
    /// `setModelOapcity(value)` (sic).
    fn set_model_opacity(&mut self, value: f64);
}
