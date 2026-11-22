//! `CubismExpressionMotion` — exp3.json expressions, port of
//! `Framework/src/motion/cubismexpressionmotion.ts` (spec §15.1–15.3).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use super::acubism_motion::{ACubismMotion, MotionBase};
use super::queue_entry::CubismMotionQueueEntry;
use crate::core::model::Live2DError;
use crate::framework::json::{self, get, index};
use crate::framework::parameter_store::ParameterStore;

/// `DefaultFadeTime` of exp3.json.
pub const DEFAULT_FADE_TIME: f64 = 1.0;
/// `CubismExpressionMotion.DefaultAdditiveValue`.
pub const DEFAULT_ADDITIVE_VALUE: f64 = 0.0;
/// `CubismExpressionMotion.DefaultMultiplyValue`.
pub const DEFAULT_MULTIPLY_VALUE: f64 = 1.0;

/// `ExpressionBlendType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionBlendType {
    Additive = 0,
    Multiply = 1,
    Overwrite = 2,
}

/// `ExpressionParameter { parameterId, blendType, value }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionParameter {
    pub parameter_id: String,
    pub blend_type: ExpressionBlendType,
    pub value: f64,
}

/// `ExpressionParameterValue` — the manager's per-parameter accumulator.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpressionParameterValue {
    pub parameter_id: String,
    pub additive_value: f64,
    pub multiply_value: f64,
    /// JS `null` of a virtual parameter behaves as 0 in every expression and is stored as 0.0.
    pub overwrite_value: f64,
}

/// An exp3.json expression (`CubismExpressionMotion`).
#[derive(Debug, Clone, Default)]
pub struct CubismExpressionMotion {
    base: MotionBase,
    parameters: Vec<ExpressionParameter>,
}

impl CubismExpressionMotion {
    /// `CubismExpressionMotion.create(buffer, size)`: `FadeInTime`/`FadeOutTime` default to 1 s;
    /// `Blend` missing/`"Add"`/unknown → Additive, `"Multiply"`, `"Overwrite"`.
    pub fn create(bytes: &[u8]) -> Result<Self, Live2DError> {
        let root_value = json::parse_json(bytes, "exp3.json")?;
        let root = Some(&root_value);
        let mut ret = Self::default();
        ret.base.fade_in_seconds = json::to_float(get(root, "FadeInTime"), DEFAULT_FADE_TIME);
        ret.base.fade_out_seconds = json::to_float(get(root, "FadeOutTime"), DEFAULT_FADE_TIME);
        let parameters = get(root, "Parameters");
        for i in 0..json::size(parameters) {
            let param = index(parameters, i);
            let blend = get(param, "Blend");
            let blend_type = if json::is_null(blend) {
                ExpressionBlendType::Additive
            } else {
                match json::raw_string(blend).as_deref() {
                    Some("Multiply") => ExpressionBlendType::Multiply,
                    Some("Overwrite") => ExpressionBlendType::Overwrite,
                    _ => ExpressionBlendType::Additive,
                }
            };
            ret.parameters.push(ExpressionParameter {
                parameter_id: json::raw_string_or_empty(get(param, "Id")),
                blend_type,
                value: json::to_float(get(param, "Value"), 0.0),
            });
        }
        Ok(ret)
    }

    /// `getExpressionParameters()`.
    pub fn expression_parameters(&self) -> &[ExpressionParameter] {
        &self.parameters
    }

    /// `calculateValue(source, destination, fadeWeight)`.
    pub fn calculate_value(source: f64, destination: f64, fade_weight: f64) -> f64 {
        source * (1.0 - fade_weight) + destination * fade_weight
    }

    /// `calculateExpressionParameters(model, userTime, entry, values, expressionIndex, fadeWeight)`:
    /// blends this expression's contribution into the manager's accumulators. QUIRK: the overwrite
    /// value is reset to the current model value first, so an older expression's Overwrite is lost.
    pub fn calculate_expression_parameters(
        &self,
        model: &mut dyn ParameterStore,
        entry: &CubismMotionQueueEntry,
        expression_parameter_values: &mut [ExpressionParameterValue],
        expression_index: usize,
        fade_weight: f64,
    ) {
        if !entry.is_available() {
            return;
        }
        for ev in expression_parameter_values.iter_mut() {
            let current_parameter_value = model.parameter_value_by_id_f64(&ev.parameter_id);
            ev.overwrite_value = current_parameter_value;
            let Some(param) = self
                .parameters
                .iter()
                .find(|p| p.parameter_id == ev.parameter_id)
            else {
                if expression_index == 0 {
                    ev.additive_value = DEFAULT_ADDITIVE_VALUE;
                    ev.multiply_value = DEFAULT_MULTIPLY_VALUE;
                    ev.overwrite_value = current_parameter_value;
                } else {
                    ev.additive_value = Self::calculate_value(
                        ev.additive_value,
                        DEFAULT_ADDITIVE_VALUE,
                        fade_weight,
                    );
                    ev.multiply_value = Self::calculate_value(
                        ev.multiply_value,
                        DEFAULT_MULTIPLY_VALUE,
                        fade_weight,
                    );
                    ev.overwrite_value = Self::calculate_value(
                        ev.overwrite_value,
                        current_parameter_value,
                        fade_weight,
                    );
                }
                continue;
            };
            let value = param.value;
            let (new_additive, new_multiply, new_overwrite) = match param.blend_type {
                ExpressionBlendType::Additive => {
                    (value, DEFAULT_MULTIPLY_VALUE, current_parameter_value)
                }
                ExpressionBlendType::Multiply => {
                    (DEFAULT_ADDITIVE_VALUE, value, current_parameter_value)
                }
                ExpressionBlendType::Overwrite => {
                    (DEFAULT_ADDITIVE_VALUE, DEFAULT_MULTIPLY_VALUE, value)
                }
            };
            if expression_index == 0 {
                ev.additive_value = new_additive;
                ev.multiply_value = new_multiply;
                ev.overwrite_value = new_overwrite;
            } else {
                ev.additive_value =
                    ev.additive_value * (1.0 - fade_weight) + new_additive * fade_weight;
                ev.multiply_value =
                    ev.multiply_value * (1.0 - fade_weight) + new_multiply * fade_weight;
                ev.overwrite_value =
                    ev.overwrite_value * (1.0 - fade_weight) + new_overwrite * fade_weight;
            }
        }
    }
}

impl ACubismMotion for CubismExpressionMotion {
    fn base(&self) -> &MotionBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut MotionBase {
        &mut self.base
    }

    fn as_dyn(&self) -> &dyn ACubismMotion {
        self
    }

    fn as_expression(&self) -> Option<&CubismExpressionMotion> {
        Some(self)
    }

    fn as_expression_mut(&mut self) -> Option<&mut CubismExpressionMotion> {
        Some(self)
    }

    /// Legacy `doUpdateParameters` (only via `updateParameters`; the expression manager uses
    /// `calculate_expression_parameters`).
    fn do_update_parameters(
        &mut self,
        model: &mut dyn ParameterStore,
        _user_time_seconds: f64,
        weight: f64,
        _entry: &mut CubismMotionQueueEntry,
    ) {
        for parameter in &self.parameters {
            match parameter.blend_type {
                ExpressionBlendType::Additive => {
                    model.add_parameter_value_by_id_f64(
                        &parameter.parameter_id,
                        parameter.value,
                        weight,
                    );
                }
                ExpressionBlendType::Multiply => {
                    model.multiply_parameter_value_by_id_f64(
                        &parameter.parameter_id,
                        parameter.value,
                        weight,
                    );
                }
                ExpressionBlendType::Overwrite => {
                    model.set_parameter_value_by_id_f64(
                        &parameter.parameter_id,
                        parameter.value,
                        weight,
                    );
                }
            }
        }
    }
}
