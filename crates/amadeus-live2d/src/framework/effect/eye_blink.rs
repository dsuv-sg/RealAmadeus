//! `CubismEyeBlink` — port of `Framework/src/effect/cubismeyeblink.ts` (spec §12.2).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! `Math.random()` is injectable ([`CubismEyeBlink::set_random_source`]) so the state machine can be
//! replayed deterministically (golden `eyeblink.jsonl`).

use std::fmt;

use crate::framework::model_setting::ModelSetting;
use crate::framework::parameter_store::ParameterStore;
use crate::framework::random::js_random;

/// `CubismEyeBlink.CloseIfZero` (true: parameter value 0 = closed).
pub const CLOSE_IF_ZERO: bool = true;

/// `EyeState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeState {
    First = 0,
    Interval = 1,
    Closing = 2,
    Closed = 3,
    Opening = 4,
}

/// Automatic eye blink (`CubismEyeBlink`).
pub struct CubismEyeBlink {
    blinking_state: EyeState,
    next_blinking_time: f64,
    state_start_time_seconds: f64,
    blinking_interval_seconds: f64,
    closing_seconds: f64,
    closed_seconds: f64,
    opening_seconds: f64,
    user_time_seconds: f64,
    parameter_ids: Vec<String>,
    random: Box<dyn FnMut() -> f64>,
}

impl fmt::Debug for CubismEyeBlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CubismEyeBlink")
            .field("blinking_state", &self.blinking_state)
            .field("next_blinking_time", &self.next_blinking_time)
            .field("state_start_time_seconds", &self.state_start_time_seconds)
            .field("user_time_seconds", &self.user_time_seconds)
            .field("parameter_ids", &self.parameter_ids)
            .finish_non_exhaustive()
    }
}

impl Default for CubismEyeBlink {
    fn default() -> Self {
        Self::create(None)
    }
}

impl CubismEyeBlink {
    /// `CubismEyeBlink.create(modelSetting)`: the eye-blink IDs come from model3.json `Groups`.
    pub fn create(model_setting: Option<&ModelSetting>) -> Self {
        Self {
            blinking_state: EyeState::First,
            next_blinking_time: 0.0,
            state_start_time_seconds: 0.0,
            blinking_interval_seconds: 4.0,
            closing_seconds: 0.1,
            closed_seconds: 0.05,
            opening_seconds: 0.15,
            user_time_seconds: 0.0,
            parameter_ids: model_setting
                .map(ModelSetting::eye_blink_parameter_ids)
                .unwrap_or_default(),
            random: Box::new(js_random),
        }
    }

    /// Replaces `Math.random()` (tests / deterministic replays).
    pub fn set_random_source(&mut self, random: Box<dyn FnMut() -> f64>) {
        self.random = random;
    }

    /// `setBlinkingInterval(seconds)`.
    pub fn set_blinking_interval(&mut self, blinking_interval: f64) {
        self.blinking_interval_seconds = blinking_interval;
    }

    /// `setBlinkingSetting(closing, closed, opening)`.
    pub fn set_blinking_setting(&mut self, closing: f64, closed: f64, opening: f64) {
        self.closing_seconds = closing;
        self.closed_seconds = closed;
        self.opening_seconds = opening;
    }

    /// `setParameterIds(ids)`.
    pub fn set_parameter_ids(&mut self, parameter_ids: Vec<String>) {
        self.parameter_ids = parameter_ids;
    }

    /// `getParameterIds()`.
    pub fn parameter_ids(&self) -> &[String] {
        &self.parameter_ids
    }

    /// Current state (`_blinkingState`).
    pub fn blinking_state(&self) -> EyeState {
        self.blinking_state
    }

    /// `_nextBlinkingTime`.
    pub fn next_blinking_time(&self) -> f64 {
        self.next_blinking_time
    }

    /// `_stateStartTimeSeconds`.
    pub fn state_start_time_seconds(&self) -> f64 {
        self.state_start_time_seconds
    }

    /// `_userTimeSeconds`.
    pub fn user_time_seconds(&self) -> f64 {
        self.user_time_seconds
    }

    /// `determinNextBlinkingTiming()` (sic): `now + random * (2 * interval - 1)`.
    fn determin_next_blinking_timing(&mut self) -> f64 {
        let r = (self.random)();
        self.user_time_seconds + r * (2.0 * self.blinking_interval_seconds - 1.0)
    }

    /// `updateParameters(model, deltaTimeSeconds)`: advances the state machine and overwrites every
    /// eye-blink parameter (weight 1, clamped).
    pub fn update_parameters(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        self.user_time_seconds += delta_time_seconds;
        let mut parameter_value = match self.blinking_state {
            EyeState::Closing => {
                let mut t =
                    (self.user_time_seconds - self.state_start_time_seconds) / self.closing_seconds;
                if t >= 1.0 {
                    t = 1.0;
                    self.blinking_state = EyeState::Closed;
                    self.state_start_time_seconds = self.user_time_seconds;
                }
                1.0 - t
            }
            EyeState::Closed => {
                let t =
                    (self.user_time_seconds - self.state_start_time_seconds) / self.closed_seconds;
                if t >= 1.0 {
                    self.blinking_state = EyeState::Opening;
                    self.state_start_time_seconds = self.user_time_seconds;
                }
                0.0
            }
            EyeState::Opening => {
                let mut t =
                    (self.user_time_seconds - self.state_start_time_seconds) / self.opening_seconds;
                if t >= 1.0 {
                    t = 1.0;
                    self.blinking_state = EyeState::Interval;
                    self.next_blinking_time = self.determin_next_blinking_timing();
                }
                t
            }
            EyeState::Interval => {
                if self.next_blinking_time < self.user_time_seconds {
                    self.blinking_state = EyeState::Closing;
                    self.state_start_time_seconds = self.user_time_seconds;
                }
                1.0
            }
            EyeState::First => {
                self.blinking_state = EyeState::Interval;
                self.next_blinking_time = self.determin_next_blinking_timing();
                1.0
            }
        };
        if !CLOSE_IF_ZERO {
            parameter_value = -parameter_value;
        }
        for id in &self.parameter_ids {
            model.set_parameter_value_by_id_f64(id, parameter_value, 1.0);
        }
    }
}
