//! `CubismBreath` — port of `Framework/src/effect/cubismbreath.ts` (spec §12.1).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use std::f64::consts::PI;

use crate::framework::default_parameter_id as ids;
use crate::framework::parameter_store::ParameterStore;
use crate::math::cubism_math::js_sin;

/// `BreathParameterData(parameterId, offset, peak, cycle, weight)`.
#[derive(Debug, Clone, PartialEq)]
pub struct BreathParameterData {
    pub parameter_id: String,
    pub offset: f64,
    pub peak: f64,
    pub cycle: f64,
    pub weight: f64,
}

impl BreathParameterData {
    pub fn new(
        parameter_id: impl Into<String>,
        offset: f64,
        peak: f64,
        cycle: f64,
        weight: f64,
    ) -> Self {
        Self {
            parameter_id: parameter_id.into(),
            offset,
            peak,
            cycle,
            weight,
        }
    }
}

/// Breathing (`CubismBreath`).
#[derive(Debug, Clone, Default)]
pub struct CubismBreath {
    breath_parameters: Vec<BreathParameterData>,
    current_time: f64,
}

impl CubismBreath {
    /// `CubismBreath.create()`.
    pub fn create() -> Self {
        Self::default()
    }

    /// `setParameters(list)`.
    pub fn set_parameters(&mut self, breath_parameters: Vec<BreathParameterData>) {
        self.breath_parameters = breath_parameters;
    }

    /// `getParameters()`.
    pub fn parameters(&self) -> &[BreathParameterData] {
        &self.breath_parameters
    }

    /// Accumulated time (`_currentTime`, a left fold of `+=` like JS).
    pub fn current_time(&self) -> f64 {
        self.current_time
    }

    /// `updateParameters(model, deltaTimeSeconds)`:
    /// `add(id, offset + peak * sin(t / cycle), weight)` with `t = currentTime * 2 * PI`.
    pub fn update_parameters(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        self.current_time += delta_time_seconds;
        let t = self.current_time * 2.0 * PI;
        for data in &self.breath_parameters {
            model.add_parameter_value_by_id_f64(
                &data.parameter_id,
                data.offset + data.peak * js_sin(t / data.cycle),
                data.weight,
            );
        }
    }

    /// The five parameters V1 `LAppModel.setupBreath` registers (bundle 15834–15854, in order).
    pub fn v1_default_parameters() -> Vec<BreathParameterData> {
        vec![
            BreathParameterData::new(ids::PARAM_ANGLE_X, 0.0, 15.0, 6.5345, 0.5),
            BreathParameterData::new(ids::PARAM_ANGLE_Y, 0.0, 8.0, 3.5345, 0.5),
            BreathParameterData::new(ids::PARAM_ANGLE_Z, 0.0, 10.0, 5.5345, 0.5),
            BreathParameterData::new(ids::PARAM_BODY_ANGLE_X, 0.0, 4.0, 15.5345, 0.5),
            BreathParameterData::new(ids::PARAM_BREATH, 0.5, 0.5, 3.2345, 1.0),
        ]
    }
}
