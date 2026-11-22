//! [`ModelState`] — the pure (JS-free) parameter/part state of the Framework's `CubismModel`
//! (`cubismmodel.ts` §7.4/§7.5): `f32` mirrors of the Core's `parameters.values/minimumValues/
//! maximumValues/defaultValues` and `parts.opacities`, the "not exist" (virtual) parameters and
//! parts, the repeat overrides, the saved-parameter snapshot and the model opacity.
//!
//! [`crate::core::model::CubismModel`] owns one `ModelState` and syncs the mirrors with the Core in
//! `update()`. Built directly from arrays (e.g. `golden/model_info.json`), a `ModelState` is also the
//! native test double for every framework algorithm.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::parameter_store::ParameterStore;
use crate::math::cubism_math;

static NEXT_MODEL_ID: AtomicU64 = AtomicU64::new(1);

/// Process-unique model identity (see [`ParameterStore::model_id`]).
pub fn next_model_id() -> u64 {
    NEXT_MODEL_ID.fetch_add(1, Ordering::Relaxed)
}

/// `ParameterRepeatData { isOverridden, isParameterRepeated }`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParameterRepeatData {
    pub is_overridden: bool,
    pub is_parameter_repeated: bool,
}

/// Pure parameter/part state with the exact Framework semantics (see module docs).
#[derive(Debug, Clone)]
pub struct ModelState {
    id: u64,
    parameter_ids: Vec<String>,
    values: Vec<f32>,
    minimums: Vec<f32>,
    maximums: Vec<f32>,
    defaults: Vec<f32>,
    core_repeats: Vec<bool>,
    saved_parameters: Vec<f32>,
    not_exist_parameter_ids: HashMap<String, usize>,
    not_exist_parameter_values: Vec<Option<f64>>,
    is_overridden_parameter_repeat: bool,
    user_parameter_repeat: Vec<ParameterRepeatData>,
    part_ids: Vec<String>,
    part_opacities: Vec<f32>,
    not_exist_part_ids: HashMap<String, usize>,
    not_exist_part_opacities: Vec<Option<f64>>,
    model_opacity: f64,
}

impl ModelState {
    /// Builds the state. `values` start equal to the Core's current values (`defaults` for a fresh
    /// model). Shorter optional arrays are padded (min/max/default 0, repeat false, opacity 1).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parameter_ids: Vec<String>,
        values: Vec<f32>,
        minimums: Vec<f32>,
        maximums: Vec<f32>,
        defaults: Vec<f32>,
        core_repeats: Vec<bool>,
        part_ids: Vec<String>,
        part_opacities: Vec<f32>,
    ) -> Self {
        let n = parameter_ids.len();
        let fit = |mut v: Vec<f32>, fill: f32| {
            v.resize(n, fill);
            v
        };
        let mut core_repeats = core_repeats;
        core_repeats.resize(n, false);
        let mut part_opacities = part_opacities;
        part_opacities.resize(part_ids.len(), 1.0);
        Self {
            id: next_model_id(),
            values: fit(values, 0.0),
            minimums: fit(minimums, 0.0),
            maximums: fit(maximums, 0.0),
            defaults: fit(defaults, 0.0),
            core_repeats,
            saved_parameters: Vec::new(),
            not_exist_parameter_ids: HashMap::new(),
            not_exist_parameter_values: Vec::new(),
            is_overridden_parameter_repeat: true,
            user_parameter_repeat: vec![ParameterRepeatData::default(); n],
            parameter_ids,
            part_ids,
            part_opacities,
            not_exist_part_ids: HashMap::new(),
            not_exist_part_opacities: Vec::new(),
            model_opacity: 1.0,
        }
    }

    // ------------------------------------------------------------------ raw access

    /// Real parameter IDs in Core order.
    pub fn parameter_ids(&self) -> &[String] {
        &self.parameter_ids
    }

    /// `getParameterId(i)`; `""` for virtual/out-of-range indices.
    pub fn parameter_id(&self, index: usize) -> &str {
        self.parameter_ids.get(index).map_or("", String::as_str)
    }

    /// Real part IDs in Core order.
    pub fn part_ids(&self) -> &[String] {
        &self.part_ids
    }

    /// `getPartId(i)`; `""` for virtual/out-of-range indices.
    pub fn part_id(&self, index: usize) -> &str {
        self.part_ids.get(index).map_or("", String::as_str)
    }

    /// Part opacity mirror (`parts.opacities`).
    pub fn part_opacities(&self) -> &[f32] {
        &self.part_opacities
    }

    /// Mutable part opacity mirror (Core sync).
    pub fn part_opacities_mut(&mut self) -> &mut [f32] {
        &mut self.part_opacities
    }

    /// Value of a virtual parameter slot (`None` = JS `null`, also for non-virtual indices).
    pub fn virtual_parameter_value(&self, index: usize) -> Option<f64> {
        self.virtual_param_slot(index)
            .and_then(|i| self.not_exist_parameter_values[i])
    }

    /// `getParameterValueByIndex(i)` with the JS `null` of virtual slots preserved.
    pub fn parameter_value_raw(&self, index: usize) -> Option<f64> {
        if let Some(slot) = self.virtual_param_slot(index) {
            return self.not_exist_parameter_values[slot];
        }
        self.values.get(index).map(|v| f64::from(*v))
    }

    /// Value of a virtual part slot (`None` = JS `null`).
    pub fn virtual_part_opacity(&self, index: usize) -> Option<f64> {
        self.virtual_part_slot(index)
            .and_then(|i| self.not_exist_part_opacities[i])
    }

    /// Number of registered virtual parameters.
    pub fn virtual_parameter_count(&self) -> usize {
        self.not_exist_parameter_values.len()
    }

    // ------------------------------------------------------------------ repeat overrides

    /// `getParameterRepeats(i)`: the editor's repeat flag from the Core.
    pub fn parameter_repeats(&self, index: usize) -> bool {
        self.core_repeats.get(index).copied().unwrap_or(false)
    }

    /// `getOverrideFlagForModelParameterRepeat()` (default **true** → editor repeat ignored).
    pub fn override_flag_for_model_parameter_repeat(&self) -> bool {
        self.is_overridden_parameter_repeat
    }

    /// `setOverrideFlagForModelParameterRepeat(v)`.
    pub fn set_override_flag_for_model_parameter_repeat(&mut self, value: bool) {
        self.is_overridden_parameter_repeat = value;
    }

    /// `getOverrideFlagForParameterRepeat(i)`.
    pub fn override_flag_for_parameter_repeat(&self, index: usize) -> bool {
        self.user_parameter_repeat
            .get(index)
            .is_some_and(|d| d.is_overridden)
    }

    /// `setOverrideFlagForParameterRepeat(i, v)`.
    pub fn set_override_flag_for_parameter_repeat(&mut self, index: usize, value: bool) {
        if let Some(d) = self.user_parameter_repeat.get_mut(index) {
            d.is_overridden = value;
        }
    }

    /// `getRepeatFlagForParameterRepeat(i)`.
    pub fn repeat_flag_for_parameter_repeat(&self, index: usize) -> bool {
        self.user_parameter_repeat
            .get(index)
            .is_some_and(|d| d.is_parameter_repeated)
    }

    /// `setRepeatFlagForParameterRepeat(i, v)`.
    pub fn set_repeat_flag_for_parameter_repeat(&mut self, index: usize, value: bool) {
        if let Some(d) = self.user_parameter_repeat.get_mut(index) {
            d.is_parameter_repeated = value;
        }
    }

    /// `getParameterClampValue(i, value)`.
    pub fn parameter_clamp_value(&self, index: usize, value: f64) -> f64 {
        if self.virtual_param_slot(index).is_some() || index >= self.values.len() {
            return value;
        }
        cubism_math::clamp(
            value,
            f64::from(self.minimums[index]),
            f64::from(self.maximums[index]),
        )
    }

    // ------------------------------------------------------------------ save / load

    /// `saveParameters()`: snapshot of the real parameter values (virtual slots are not saved).
    pub fn save_parameters(&mut self) {
        let count = self.values.len();
        let saved = self.saved_parameters.len();
        for i in 0..count {
            if i < saved {
                self.saved_parameters[i] = self.values[i];
            } else {
                self.saved_parameters.push(self.values[i]);
            }
        }
    }

    /// `loadParameters()`: restore `min(count, saved)` real values (virtual slots are not restored).
    pub fn load_parameters(&mut self) {
        let n = self.values.len().min(self.saved_parameters.len());
        self.values[..n].copy_from_slice(&self.saved_parameters[..n]);
    }

    /// The saved snapshot.
    pub fn saved_parameters(&self) -> &[f32] {
        &self.saved_parameters
    }

    /// What `Model.update()` does to the inputs (measured on Core 6.0.1): parameter values clamped
    /// in place to `[min, max]` (NaN stays NaN) and part opacities clamped to `[0, 1]`. The real
    /// `CubismModel` pulls these back from the Core; tests call this to emulate a Core update.
    pub fn emulate_core_update(&mut self) {
        for i in 0..self.values.len() {
            let v = self.values[i];
            if v < self.minimums[i] {
                self.values[i] = self.minimums[i];
            } else if v > self.maximums[i] {
                self.values[i] = self.maximums[i];
            }
        }
        for o in &mut self.part_opacities {
            *o = o.clamp(0.0, 1.0); // NaN stays NaN
        }
    }

    // ------------------------------------------------------------------ helpers

    fn virtual_param_slot(&self, index: usize) -> Option<usize> {
        let count = self.values.len();
        if index >= count && index - count < self.not_exist_parameter_values.len() {
            Some(index - count)
        } else {
            None
        }
    }

    fn virtual_part_slot(&self, index: usize) -> Option<usize> {
        let count = self.part_opacities.len();
        if index >= count && index - count < self.not_exist_part_opacities.len() {
            Some(index - count)
        } else {
            None
        }
    }

    fn real_f64(slice: &[f32], index: usize) -> f64 {
        slice.get(index).map_or(f64::NAN, |v| f64::from(*v))
    }
}

impl ParameterStore for ModelState {
    fn model_id(&self) -> u64 {
        self.id
    }

    fn parameter_count(&self) -> usize {
        self.values.len()
    }

    fn parameter_index(&mut self, id: &str) -> usize {
        if let Some(i) = self.parameter_ids.iter().position(|p| p == id) {
            return i;
        }
        if let Some(&i) = self.not_exist_parameter_ids.get(id) {
            return i;
        }
        let index = self.values.len() + self.not_exist_parameter_ids.len();
        self.not_exist_parameter_ids.insert(id.to_owned(), index);
        self.not_exist_parameter_values.push(None);
        index
    }

    fn parameter_value_f64(&self, index: usize) -> f64 {
        if let Some(slot) = self.virtual_param_slot(index) {
            return self.not_exist_parameter_values[slot].unwrap_or(0.0);
        }
        Self::real_f64(&self.values, index)
    }

    fn parameter_minimum_f64(&self, index: usize) -> f64 {
        Self::real_f64(&self.minimums, index)
    }

    fn parameter_maximum_f64(&self, index: usize) -> f64 {
        Self::real_f64(&self.maximums, index)
    }

    fn parameter_default_f64(&self, index: usize) -> f64 {
        Self::real_f64(&self.defaults, index)
    }

    fn set_parameter_value_f64(&mut self, index: usize, value: f64, weight: f64) {
        if let Some(slot) = self.virtual_param_slot(index) {
            let stored = &mut self.not_exist_parameter_values[slot];
            *stored = Some(if weight == 1.0 {
                value
            } else {
                stored.unwrap_or(0.0) * (1.0 - weight) + value * weight
            });
            return;
        }
        if index >= self.values.len() {
            return; // JS: out-of-range Float32Array write is ignored
        }
        let value = if self.is_repeat(index) {
            self.parameter_repeat_value(index, value)
        } else {
            self.parameter_clamp_value(index, value)
        };
        self.values[index] = if weight == 1.0 {
            value as f32
        } else {
            (f64::from(self.values[index]) * (1.0 - weight) + value * weight) as f32
        };
    }

    fn is_repeat(&self, index: usize) -> bool {
        if self.virtual_param_slot(index).is_some() || index >= self.values.len() {
            return false;
        }
        let user = self.user_parameter_repeat[index];
        if self.is_overridden_parameter_repeat || user.is_overridden {
            user.is_parameter_repeated
        } else {
            self.core_repeats[index]
        }
    }

    fn parameter_repeat_value(&self, index: usize, value: f64) -> f64 {
        if self.virtual_param_slot(index).is_some() || index >= self.values.len() {
            return value;
        }
        let max_value = f64::from(self.maximums[index]);
        let min_value = f64::from(self.minimums[index]);
        let value_size = max_value - min_value;
        let mut value = value;
        if max_value < value {
            let over_value = cubism_math::modulo(value - max_value, value_size);
            value = if !over_value.is_nan() {
                min_value + over_value
            } else {
                max_value
            };
        }
        if value < min_value {
            let over_value = cubism_math::modulo(min_value - value, value_size);
            value = if !over_value.is_nan() {
                max_value - over_value
            } else {
                min_value
            };
        }
        value
    }

    fn parameter_values(&self) -> &[f32] {
        &self.values
    }

    fn parameter_values_mut(&mut self) -> &mut [f32] {
        &mut self.values
    }

    fn parameter_minimums(&self) -> &[f32] {
        &self.minimums
    }

    fn parameter_maximums(&self) -> &[f32] {
        &self.maximums
    }

    fn parameter_defaults(&self) -> &[f32] {
        &self.defaults
    }

    fn part_count(&self) -> usize {
        self.part_opacities.len()
    }

    fn part_index(&mut self, id: &str) -> usize {
        if let Some(i) = self.part_ids.iter().position(|p| p == id) {
            return i;
        }
        if let Some(&i) = self.not_exist_part_ids.get(id) {
            return i;
        }
        let index = self.part_opacities.len() + self.not_exist_part_ids.len();
        self.not_exist_part_ids.insert(id.to_owned(), index);
        self.not_exist_part_opacities.push(None);
        index
    }

    fn part_opacity_f64(&self, index: usize) -> f64 {
        if let Some(slot) = self.virtual_part_slot(index) {
            return self.not_exist_part_opacities[slot].unwrap_or(0.0);
        }
        Self::real_f64(&self.part_opacities, index)
    }

    fn set_part_opacity_f64(&mut self, index: usize, opacity: f64) {
        if let Some(slot) = self.virtual_part_slot(index) {
            self.not_exist_part_opacities[slot] = Some(opacity);
            return;
        }
        if let Some(o) = self.part_opacities.get_mut(index) {
            *o = opacity as f32;
        }
    }

    fn model_opacity(&self) -> f64 {
        self.model_opacity
    }

    fn set_model_opacity(&mut self, value: f64) {
        self.model_opacity = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> ModelState {
        ModelState::new(
            vec!["A".into(), "B".into(), "R".into()],
            vec![0.0, 1.0, 0.0],
            vec![-30.0, 0.0, -1.0],
            vec![30.0, 1.0, 1.0],
            vec![0.0, 1.0, 0.0],
            vec![false, false, true],
            vec!["P0".into()],
            vec![1.0],
        )
    }

    #[test]
    fn clamp_before_weight() {
        let mut s = state();
        s.set_parameter_value_f64(0, 100.0, 0.5);
        // clamp(100) = 30, then 0 * 0.5 + 30 * 0.5
        assert_eq!(s.parameter_value_f64(0), 15.0);
        s.add_parameter_value_f64(0, 0.1, 1.0);
        assert_eq!(s.parameter_value_f64(0), f64::from(15.1f32));
        s.multiply_parameter_value_f64(1, 0.5, 0.5); // 1 * (1 + (0.5-1)*0.5) = 0.75
        assert_eq!(s.parameter_value_f64(1), 0.75);
        s.set_parameter_value_f64(1, f64::NAN, 1.0);
        assert!(s.parameter_value_f64(1).is_nan());
    }

    #[test]
    fn repeat_is_overridden_by_default() {
        let mut s = state();
        assert!(!s.is_repeat(2)); // editor flag ignored (QUIRK)
        s.set_parameter_value_f64(2, 1.5, 1.0);
        assert_eq!(s.parameter_value_f64(2), 1.0);
        s.set_override_flag_for_model_parameter_repeat(false);
        assert!(s.is_repeat(2));
        s.set_parameter_value_f64(2, 1.5, 1.0); // wraps: min + mod(0.5, 2) = -0.5
        assert_eq!(s.parameter_value_f64(2), -0.5);
        s.set_parameter_value_f64(2, -1.25, 1.0); // max - mod(0.25, 2) = 0.75
        assert_eq!(s.parameter_value_f64(2), 0.75);
    }

    #[test]
    fn virtual_slots() {
        let mut s = state();
        let v = s.parameter_index("X");
        assert_eq!(v, 3);
        assert_eq!(s.parameter_index("X"), 3);
        assert_eq!(s.parameter_index("Y"), 4);
        assert_eq!(s.parameter_value_raw(v), None);
        assert_eq!(s.parameter_value_f64(v), 0.0);
        s.set_parameter_value_f64(v, 5.0, 0.5); // null * 0.5 + 5 * 0.5, unclamped f64
        assert_eq!(s.virtual_parameter_value(v), Some(2.5));
        s.add_parameter_value_f64(v, 100.0, 1.0);
        assert_eq!(s.parameter_value_f64(v), 102.5);
        assert!(s.parameter_minimum_f64(v).is_nan());
        assert!(!s.is_repeat(v));
        assert!(s.parameter_value_f64(99).is_nan());
        s.set_parameter_value_f64(99, 1.0, 1.0); // ignored
        let p = s.part_index("Missing");
        assert_eq!(p, 1);
        s.set_part_opacity_f64(p, 0.25);
        assert_eq!(s.part_opacity_f64(p), 0.25);
        s.set_part_opacity_f64(0, 3.0); // no clamp until the Core update
        assert_eq!(s.part_opacity_f64(0), 3.0);
        s.emulate_core_update();
        assert_eq!(s.part_opacity_f64(0), 1.0);
    }

    #[test]
    fn save_load_real_only() {
        let mut s = state();
        let v = s.parameter_index("X");
        s.save_parameters();
        s.set_parameter_value_f64(0, 10.0, 1.0);
        s.set_parameter_value_f64(v, 10.0, 1.0);
        s.load_parameters();
        assert_eq!(s.parameter_value_f64(0), 0.0);
        assert_eq!(s.parameter_value_f64(v), 10.0);
    }
}
