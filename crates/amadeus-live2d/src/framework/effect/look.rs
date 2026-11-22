//! `CubismLook` — port of `Framework/src/effect/cubismlook.ts` (spec §12.4).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use crate::framework::default_parameter_id as ids;
use crate::framework::parameter_store::ParameterStore;

/// `LookParameterData(parameterId, factorX, factorY, factorXY)`.
#[derive(Debug, Clone, PartialEq)]
pub struct LookParameterData {
    pub parameter_id: String,
    pub factor_x: f64,
    pub factor_y: f64,
    pub factor_xy: f64,
}

impl LookParameterData {
    pub fn new(
        parameter_id: impl Into<String>,
        factor_x: f64,
        factor_y: f64,
        factor_xy: f64,
    ) -> Self {
        Self {
            parameter_id: parameter_id.into(),
            factor_x,
            factor_y,
            factor_xy,
        }
    }
}

/// Look-at (`CubismLook`).
#[derive(Debug, Clone, Default)]
pub struct CubismLook {
    look_parameters: Vec<LookParameterData>,
}

impl CubismLook {
    /// `CubismLook.create()`.
    pub fn create() -> Self {
        Self::default()
    }

    /// `setParameters(list)`.
    pub fn set_parameters(&mut self, look_parameters: Vec<LookParameterData>) {
        self.look_parameters = look_parameters;
    }

    /// `getParameters()`.
    pub fn parameters(&self) -> &[LookParameterData] {
        &self.look_parameters
    }

    /// `updateParameters(model, dragX, dragY)`: `add(id, fX·x + fY·y + fXY·x·y)` (weight 1).
    pub fn update_parameters(&self, model: &mut dyn ParameterStore, drag_x: f64, drag_y: f64) {
        for data in &self.look_parameters {
            model.add_parameter_value_by_id_f64(
                &data.parameter_id,
                data.factor_x * drag_x + data.factor_y * drag_y + data.factor_xy * drag_x * drag_y,
                1.0,
            );
        }
    }

    /// The six parameters V1 `LAppModel.setupLook` registers (bundle 15911–15932, in order).
    pub fn v1_default_parameters() -> Vec<LookParameterData> {
        vec![
            LookParameterData::new(ids::PARAM_ANGLE_X, 30.0, 0.0, 0.0),
            LookParameterData::new(ids::PARAM_ANGLE_Y, 0.0, 30.0, 0.0),
            LookParameterData::new(ids::PARAM_ANGLE_Z, 0.0, 0.0, -30.0),
            LookParameterData::new(ids::PARAM_BODY_ANGLE_X, 10.0, 0.0, 0.0),
            LookParameterData::new(ids::PARAM_EYE_BALL_X, 1.0, 0.0, 0.0),
            LookParameterData::new(ids::PARAM_EYE_BALL_Y, 0.0, 1.0, 0.0),
        ]
    }
}
