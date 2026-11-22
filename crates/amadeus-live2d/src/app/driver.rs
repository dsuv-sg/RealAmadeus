//! The `ReamaCubismPlayer` parameter driver — a pure, native-testable port of the player state,
//! its public setters and `applyParameters` (pipeline spec §4.1, §9.5–§9.13; deployed bundle
//! 18076–18603). No web types: time and randomness are passed in (`performance.now() / 1000` and
//! `Math.random`), the 22 parameter writes are returned as a [`ParamFrame`] in V1 write order.
//!
//! All arithmetic is JS `f64` with the original operation order; the writes go through
//! `setParameterValueById(id, value)` (weight 1, clamp, f32 store) via [`ParamFrame::write_to`].

use std::f64::consts::PI;

use super::clock::PlayerClock;
use super::emotion::{
    emotion_target, motion_burst, normal_target, surprised_target, vowel_mouth_opening,
    EmotionTarget,
};
use super::jsnum::{js_max, js_min, js_or_zero, js_trim};
use super::noise::{burst_spring, emotion_idle_motion, lerp, noise01};
use crate::framework::default_parameter_id as ids;
use crate::framework::parameter_store::ParameterStore;
use crate::math::cubism_math::js_sin;

/// The 22 parameters the driver writes (`resolveParameterHandles`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerParam {
    BrowLY,
    BrowRY,
    BrowLForm,
    BrowRForm,
    BrowLAngle,
    BrowRAngle,
    EyeLSmile,
    EyeRSmile,
    MouthForm,
    Cheek,
    EyeLOpen,
    EyeROpen,
    BodyAngleX,
    BodyAngleY,
    BodyAngleZ,
    AngleX,
    AngleY,
    AngleZ,
    EyeBallX,
    EyeBallY,
    Breath,
    MouthOpenY,
}

impl PlayerParam {
    /// V1 write order of `applyParameters` (spec §9.7 steps 10, 13–15).
    pub const WRITE_ORDER: [PlayerParam; 22] = [
        PlayerParam::BrowLY,
        PlayerParam::BrowRY,
        PlayerParam::BrowLForm,
        PlayerParam::BrowRForm,
        PlayerParam::BrowLAngle,
        PlayerParam::BrowRAngle,
        PlayerParam::EyeLSmile,
        PlayerParam::EyeRSmile,
        PlayerParam::MouthForm,
        PlayerParam::Cheek,
        PlayerParam::EyeLOpen,
        PlayerParam::EyeROpen,
        PlayerParam::BodyAngleX,
        PlayerParam::BodyAngleY,
        PlayerParam::BodyAngleZ,
        PlayerParam::AngleX,
        PlayerParam::AngleY,
        PlayerParam::AngleZ,
        PlayerParam::EyeBallX,
        PlayerParam::EyeBallY,
        PlayerParam::Breath,
        PlayerParam::MouthOpenY,
    ];

    /// The Cubism parameter ID.
    pub fn id(self) -> &'static str {
        match self {
            PlayerParam::BrowLY => ids::PARAM_BROW_L_Y,
            PlayerParam::BrowRY => ids::PARAM_BROW_R_Y,
            PlayerParam::BrowLForm => ids::PARAM_BROW_L_FORM,
            PlayerParam::BrowRForm => ids::PARAM_BROW_R_FORM,
            PlayerParam::BrowLAngle => ids::PARAM_BROW_L_ANGLE,
            PlayerParam::BrowRAngle => ids::PARAM_BROW_R_ANGLE,
            PlayerParam::EyeLSmile => ids::PARAM_EYE_L_SMILE,
            PlayerParam::EyeRSmile => ids::PARAM_EYE_R_SMILE,
            PlayerParam::MouthForm => ids::PARAM_MOUTH_FORM,
            PlayerParam::Cheek => ids::PARAM_CHEEK,
            PlayerParam::EyeLOpen => ids::PARAM_EYE_L_OPEN,
            PlayerParam::EyeROpen => ids::PARAM_EYE_R_OPEN,
            PlayerParam::BodyAngleX => ids::PARAM_BODY_ANGLE_X,
            PlayerParam::BodyAngleY => ids::PARAM_BODY_ANGLE_Y,
            PlayerParam::BodyAngleZ => ids::PARAM_BODY_ANGLE_Z,
            PlayerParam::AngleX => ids::PARAM_ANGLE_X,
            PlayerParam::AngleY => ids::PARAM_ANGLE_Y,
            PlayerParam::AngleZ => ids::PARAM_ANGLE_Z,
            PlayerParam::EyeBallX => ids::PARAM_EYE_BALL_X,
            PlayerParam::EyeBallY => ids::PARAM_EYE_BALL_Y,
            PlayerParam::Breath => ids::PARAM_BREATH,
            PlayerParam::MouthOpenY => ids::PARAM_MOUTH_OPEN_Y,
        }
    }
}

/// The 22 writes of one `applyParameters` call, in V1 order (unclamped `f64`; the model clamps).
#[derive(Debug, Clone, PartialEq)]
pub struct ParamFrame {
    pub values: [(PlayerParam, f64); 22],
}

impl ParamFrame {
    /// The value written for `param`.
    pub fn get(&self, param: PlayerParam) -> f64 {
        self.values
            .iter()
            .find(|(p, _)| *p == param)
            .map_or(f64::NAN, |(_, v)| *v)
    }

    /// `setParam(model, id, value)` for every entry in order (`setParameterValueById(id, value)`,
    /// weight 1: clamp, then f32 store).
    pub fn write_to(&self, model: &mut dyn ParameterStore) {
        for (param, value) in &self.values {
            model.set_parameter_value_by_id_f64(param.id(), *value, 1.0);
        }
    }
}

/// Result of [`EmotionDriver::step`].
#[derive(Debug, Clone, PartialEq)]
pub struct DriverFrame {
    /// The capped Clock-2 delta used this frame.
    pub delta_seconds: f64,
    /// `model.setExpression(name)` requested by the (latent) expression hook (spec §9.5).
    pub expression: Option<String>,
    /// The 22 parameter writes.
    pub params: ParamFrame,
}

/// Blink FSM state (`blinkState`: 0 open, 1 closing, 2 opening).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlinkState {
    #[default]
    Open = 0,
    Closing = 1,
    Opening = 2,
}

/// The player's state (every non-DOM field of `ReamaCubismPlayer`, spec §4.1).
#[derive(Debug, Clone)]
pub struct EmotionDriver {
    current: EmotionTarget,
    user_time: f64,
    mouth_open: f64,
    emotion: String,
    lip_sync_value: f64,
    wake_up_timer: f64,
    wake_up_head_x: f64,
    wake_up_gaze_x: f64,
    sneeze_easter_egg_timer: f64,
    burst_progress: f64,
    burst_timer: f64,
    active_burst_tag: String,
    sleeping_nod_timer: f64,
    next_sleeping_nod_interval: f64,
    spoken_char: String,
    current_emotion_tag: String,
    idle_phase: f64,
    smoothed_body_x: f64,
    smoothed_body_y: f64,
    smoothed_body_z: f64,
    smoothed_head_x: f64,
    smoothed_head_y: f64,
    smoothed_head_z: f64,
    last_applied_emotion_tag: String,
    eye_tracking_x: f64,
    eye_tracking_y: f64,
    smoothed_gaze_x: f64,
    smoothed_gaze_y: f64,
    blink_state: BlinkState,
    blink_timer: f64,
    blink_interval: f64,
    blink_duration: f64,
    clock: PlayerClock,
}

impl EmotionDriver {
    /// `new ReamaCubismPlayer(host)` field initialization; draws the first blink interval
    /// `3.0 + Math.random() * 3.5` (the source comment's "2.5-6.0s" is wrong).
    pub fn new(rng: &mut dyn FnMut() -> f64) -> Self {
        let mut current = *normal_target();
        current.is_wink = false;
        Self {
            current,
            user_time: 0.0,
            mouth_open: 0.0,
            emotion: "NORMAL".to_owned(),
            lip_sync_value: 0.0,
            wake_up_timer: 0.0,
            wake_up_head_x: 0.0,
            wake_up_gaze_x: 0.0,
            sneeze_easter_egg_timer: 0.0,
            burst_progress: 1.0,
            burst_timer: 0.0,
            active_burst_tag: String::new(),
            sleeping_nod_timer: 0.0,
            next_sleeping_nod_interval: 30.0,
            spoken_char: String::new(),
            current_emotion_tag: "NORMAL".to_owned(),
            idle_phase: 0.0,
            smoothed_body_x: 0.0,
            smoothed_body_y: 0.0,
            smoothed_body_z: 0.0,
            smoothed_head_x: 0.0,
            smoothed_head_y: 0.0,
            smoothed_head_z: 0.0,
            last_applied_emotion_tag: String::new(),
            eye_tracking_x: 0.0,
            eye_tracking_y: 0.0,
            smoothed_gaze_x: 0.0,
            smoothed_gaze_y: 0.0,
            blink_state: BlinkState::Open,
            blink_timer: 0.0,
            blink_interval: 3.0 + rng() * 3.5,
            blink_duration: 0.1,
            clock: PlayerClock::new(),
        }
    }

    // ------------------------------------------------------------------------------ setters

    /// `setEmotion(emotion)`: `trim().toUpperCase()`; same tag → no-op; leaving `SLEEPING` starts the
    /// 2.5-s startled wake-up (SURPRISED burst); otherwise a new tag cancels a wake-up and, if it
    /// differs from `currentEmotionTag`, starts its burst and restarts the idle phase.
    pub fn set_emotion(&mut self, emotion: &str) {
        let normalized_tag = js_trim(emotion).to_uppercase();
        if normalized_tag == self.emotion {
            return;
        }
        let is_waking_up = self.emotion == "SLEEPING" && normalized_tag != "SLEEPING";
        self.emotion = normalized_tag;
        if is_waking_up {
            self.wake_up_timer = 2.5;
            self.active_burst_tag = "SURPRISED".to_owned();
            self.burst_timer = 0.0;
            self.burst_progress = 0.0;
            self.idle_phase = 0.0;
            self.current_emotion_tag = "SURPRISED".to_owned();
        } else {
            self.wake_up_timer = 0.0;
            if self.emotion != self.current_emotion_tag {
                self.active_burst_tag = self.emotion.clone();
                self.burst_timer = 0.0;
                self.burst_progress = 0.0;
                self.idle_phase = 0.0;
                self.current_emotion_tag = self.emotion.clone();
            }
        }
    }

    /// `setSpokenChar(c)`: `spokenChar = c || ""`.
    pub fn set_spoken_char(&mut self, spoken_char: &str) {
        self.spoken_char = spoken_char.to_owned();
    }

    /// `setLipSync(v)`: clamp to `[0, 1]`. Deviation (spec §9.6): NaN becomes 0 ("not speaking");
    /// V1 kept NaN, which additionally blocked the sneeze timer.
    pub fn set_lip_sync(&mut self, value: f64) {
        self.lip_sync_value = if value.is_nan() {
            0.0
        } else {
            js_max(0.0, js_min(1.0, value))
        };
    }

    /// `setEyeTracking(x, y)`: clamp both to `[-1, 1]`; NaN → 0 (V1 read NaN back as `|| 0`).
    pub fn set_eye_tracking(&mut self, x: f64, y: f64) {
        let clamp = |v: f64| {
            if v.is_nan() {
                0.0
            } else {
                js_max(-1.0, js_min(1.0, v))
            }
        };
        self.eye_tracking_x = clamp(x);
        self.eye_tracking_y = clamp(y);
    }

    // ------------------------------------------------------------------------------ accessors

    /// `emotion` (normalized tag last set).
    pub fn emotion(&self) -> &str {
        &self.emotion
    }
    /// `currentEmotionTag` (SURPRISED during a wake-up).
    pub fn current_emotion_tag(&self) -> &str {
        &self.current_emotion_tag
    }
    /// `activeBurstTag` (never cleared after a burst completes).
    pub fn active_burst_tag(&self) -> &str {
        &self.active_burst_tag
    }
    /// `burstProgress` (1 = no burst running).
    pub fn burst_progress(&self) -> f64 {
        self.burst_progress
    }
    /// `blinkState`.
    pub fn blink_state(&self) -> BlinkState {
        self.blink_state
    }
    /// `blinkInterval` (seconds until the next blink starts).
    pub fn blink_interval(&self) -> f64 {
        self.blink_interval
    }
    /// `wakeUpTimer`.
    pub fn wake_up_timer(&self) -> f64 {
        self.wake_up_timer
    }
    /// `userTime` (sum of the capped deltas).
    pub fn user_time(&self) -> f64 {
        self.user_time
    }
    /// `idlePhase`.
    pub fn idle_phase(&self) -> f64 {
        self.idle_phase
    }
    /// `lipSyncValue`.
    pub fn lip_sync_value(&self) -> f64 {
        self.lip_sync_value
    }
    /// `spokenChar`.
    pub fn spoken_char(&self) -> &str {
        &self.spoken_char
    }
    /// `(eyeTrackingX, eyeTrackingY)`.
    pub fn eye_tracking(&self) -> (f64, f64) {
        (self.eye_tracking_x, self.eye_tracking_y)
    }
    /// `current` (the smoothed face/pose values).
    pub fn current(&self) -> &EmotionTarget {
        &self.current
    }
    /// `mouthOpen` (may leave `[0, 1]` at low frame rates — V1 QUIRK).
    pub fn mouth_open(&self) -> f64 {
        self.mouth_open
    }
    /// `sneezeEasterEggTimer`.
    pub fn sneeze_easter_egg_timer(&self) -> f64 {
        self.sneeze_easter_egg_timer
    }
    /// `nextSleepingNodInterval`.
    pub fn next_sleeping_nod_interval(&self) -> f64 {
        self.next_sleeping_nod_interval
    }

    // ------------------------------------------------------------------------------ frame

    /// `applyParameters(model)` after its guards (the caller only calls this while the Core model
    /// exists): Clock 2 → expression hook → sneeze / sleep-nod / wake-up timers → emotion
    /// smoothing → bursts → blink FSM → eyes → idle drift + gaze → the 22 writes.
    ///
    /// `now_seconds` = `performance.now() / 1000.0`; `rng` = `Math.random`; `expression_names` =
    /// the model's `_expressions` keys in insertion order (empty for the shipped model).
    pub fn step(
        &mut self,
        now_seconds: f64,
        rng: &mut dyn FnMut() -> f64,
        expression_names: &[String],
    ) -> DriverFrame {
        let mut target: EmotionTarget = *emotion_target(&self.emotion).unwrap_or(normal_target());
        let delta_seconds = self.clock.tick(now_seconds);
        self.user_time += delta_seconds;
        self.idle_phase += delta_seconds;

        // Set initial/new expression once model is loaded (latent).
        let mut expression = None;
        if !expression_names.is_empty() && self.last_applied_emotion_tag != self.current_emotion_tag
        {
            if let Some(name) = find_expression_name(expression_names, &self.current_emotion_tag) {
                expression = Some(name.to_owned());
                self.last_applied_emotion_tag = self.current_emotion_tag.clone();
            }
        }

        // 1. Sneeze easter egg (idle NORMAL, not speaking, not waking up).
        if self.emotion == "NORMAL"
            && self.burst_progress >= 1.0
            && self.wake_up_timer <= 0.0
            && self.lip_sync_value <= 0.01
        {
            self.sneeze_easter_egg_timer += delta_seconds;
            if self.sneeze_easter_egg_timer >= 45.0 {
                self.sneeze_easter_egg_timer = 0.0;
                if rng() < 0.15 {
                    self.active_burst_tag = "SNEEZE".to_owned();
                    self.burst_timer = 0.0;
                    self.burst_progress = 0.0;
                }
            }
        } else {
            self.sneeze_easter_egg_timer = 0.0;
        }

        // 2. Periodic sleep nodding.
        if self.emotion == "SLEEPING" {
            self.sleeping_nod_timer += delta_seconds;
            if self.sleeping_nod_timer >= self.next_sleeping_nod_interval {
                self.sleeping_nod_timer = 0.0;
                self.next_sleeping_nod_interval = 20.0 + rng() * 20.0;
                self.active_burst_tag = "SLEEPING".to_owned();
                self.burst_progress = 0.0;
                self.burst_timer = 0.0;
            }
        } else {
            self.sleeping_nod_timer = 0.0;
        }

        // 3. Startled wake-up transition blend.
        if self.wake_up_timer > 0.0 {
            self.wake_up_timer -= delta_seconds;
            if self.wake_up_timer <= 0.0 {
                self.wake_up_timer = 0.0;
                self.wake_up_head_x = 0.0;
                self.wake_up_gaze_x = 0.0;
                self.current_emotion_tag = self.emotion.clone();
            } else {
                let mut blend = 1.0;
                if self.wake_up_timer < 1.7 {
                    blend = js_max(0.0, js_min(1.0, self.wake_up_timer / 1.7));
                }
                target = target.lerp_to(surprised_target(), blend);
                let elapsed = 2.5 - self.wake_up_timer;
                if (0.4..1.4).contains(&elapsed) {
                    let wave = js_sin((elapsed - 0.4) * (2.0 * PI / 1.0));
                    self.wake_up_head_x = wave * 15.0;
                    self.wake_up_gaze_x = wave * 0.7;
                } else {
                    self.wake_up_head_x = 0.0;
                    self.wake_up_gaze_x = 0.0;
                }
            }
        } else {
            self.wake_up_head_x = 0.0;
            self.wake_up_gaze_x = 0.0;
        }

        // Smooth emotion blend (`_emotionLerpSpeed = 3.0`).
        let smooth = js_min(delta_seconds * 3.0, 1.0);
        self.current = self.current.lerp_to(&target, smooth);

        // 4. Bursts (nodding / sneeze / transitions).
        let (mut burst_body_x, mut burst_body_y, mut burst_body_z) = (0.0, 0.0, 0.0);
        let (mut burst_head_x, mut burst_head_y, mut burst_head_z) = (0.0, 0.0, 0.0);
        let mut sneeze_mouth_open = 0.0;
        let mut sneeze_eye_open = 1.0;
        let is_sneeze_active = self.burst_progress < 1.0 && self.active_burst_tag == "SNEEZE";
        if self.burst_progress < 1.0 {
            self.burst_timer += delta_seconds;
            let active_burst = motion_burst(&self.active_burst_tag);
            let duration = active_burst.duration;
            self.burst_progress = js_max(0.0, js_min(1.0, self.burst_timer / duration));
            let t01 = self.burst_progress;
            if self.active_burst_tag == "SLEEPING" {
                let nod_curve = if t01 < 0.33 {
                    js_sin((t01 / 0.33) * PI) * -10.0
                } else if t01 < 0.66 {
                    js_sin(((t01 - 0.33) / 0.33) * PI) * -10.0
                } else {
                    js_sin(((t01 - 0.66) / 0.34) * PI) * -18.0
                };
                burst_head_y = nod_curve;
                burst_body_y = t01 * -1.5 + (nod_curve * 0.15);
            } else if self.active_burst_tag == "SNEEZE" {
                if t01 < 0.5 {
                    let local_t = t01 / 0.5;
                    burst_head_y = local_t * 12.0;
                    burst_head_x = local_t * -3.0;
                    burst_body_y = local_t * 1.5;
                    sneeze_eye_open = 1.0 - (local_t * 0.9);
                } else if t01 < 0.65 {
                    let local_t = (t01 - 0.5) / 0.15;
                    burst_head_y = 12.0 - (local_t * 32.0);
                    burst_head_x = -3.0 + (local_t * 6.0);
                    burst_body_y = 1.5 - (local_t * 5.0);
                    sneeze_eye_open = 0.1 * (1.0 - local_t);
                    sneeze_mouth_open = local_t * 0.9;
                } else {
                    let local_t = (t01 - 0.65) / 0.35;
                    burst_head_y = -20.0 + (local_t * 20.0);
                    burst_head_x = 3.0 - (local_t * 3.0);
                    burst_body_y = -3.5 + (local_t * 3.5);
                    sneeze_eye_open = 0.0 + (local_t * 1.0);
                    sneeze_mouth_open = 0.9 * (1.0 - local_t);
                }
            } else {
                // General spring-physics overshoot animation.
                let spring = burst_spring(t01);
                let intensity = active_burst.intensity * spring;
                burst_body_x = active_burst.body_x * intensity;
                burst_body_y = active_burst.body_y * intensity;
                burst_body_z = active_burst.body_z * intensity;
                burst_head_x = active_burst.head_x * intensity;
                burst_head_y = active_burst.head_y * intensity;
                burst_head_z = active_burst.head_z * intensity;
            }
        }

        // Blink FSM (QT C++ UpdateBlink parity).
        self.blink_timer += delta_seconds;
        let mut blink_value = 1.0;
        match self.blink_state {
            BlinkState::Open => {
                if self.blink_timer >= self.blink_interval {
                    self.blink_state = BlinkState::Closing;
                    self.blink_timer = 0.0;
                }
            }
            BlinkState::Closing => {
                let t = js_min(self.blink_timer / self.blink_duration, 1.0);
                blink_value = 1.0 - t;
                if self.blink_timer >= self.blink_duration {
                    self.blink_state = BlinkState::Opening;
                    self.blink_timer = 0.0;
                }
            }
            BlinkState::Opening => {
                let t = js_min(self.blink_timer / self.blink_duration, 1.0);
                blink_value = t;
                if self.blink_timer >= self.blink_duration {
                    self.blink_state = BlinkState::Open;
                    self.blink_timer = 0.0;
                    self.blink_interval = 2.5 + rng() * 3.5;
                }
            }
        }
        let blink = blink_value;
        let sleeping_nod = self.emotion == "SLEEPING" && self.burst_progress < 1.0;
        let mut nod_eye_open = 1.0;
        if sleeping_nod {
            let t01 = self.burst_progress;
            let local_t = if t01 < 0.33 {
                t01 / 0.33
            } else if t01 < 0.66 {
                (t01 - 0.33) / 0.33
            } else {
                (t01 - 0.66) / 0.34
            };
            nod_eye_open = 0.18 * js_sin(local_t * PI);
        }
        let mut base_eye_open = self.current.eye_open;
        if sleeping_nod {
            base_eye_open = nod_eye_open;
        } else if is_sneeze_active {
            base_eye_open = sneeze_eye_open;
        }
        let eye_open = js_max(0.0, base_eye_open * blink);
        let eye_r_open = if target.is_wink { 0.0 } else { eye_open };
        let face = self.current;

        // 5. Idle movement (breathing & drift) with smoothing.
        let idle = emotion_idle_motion(&self.current_emotion_tag, self.idle_phase);
        let motion_smooth_t = js_max(0.0, js_min(delta_seconds * 6.0, 1.0));
        self.smoothed_body_x = lerp(
            self.smoothed_body_x,
            burst_body_x + idle.body_x,
            motion_smooth_t,
        );
        self.smoothed_body_y = lerp(
            self.smoothed_body_y,
            burst_body_y + idle.body_y,
            motion_smooth_t,
        );
        self.smoothed_body_z = lerp(
            self.smoothed_body_z,
            burst_body_z + idle.body_z,
            motion_smooth_t,
        );
        self.smoothed_head_x = lerp(
            self.smoothed_head_x,
            burst_head_x + idle.head_x + self.wake_up_head_x,
            motion_smooth_t,
        );
        self.smoothed_head_y = lerp(
            self.smoothed_head_y,
            burst_head_y + idle.head_y,
            motion_smooth_t,
        );
        self.smoothed_head_z = lerp(
            self.smoothed_head_z,
            burst_head_z + idle.head_z,
            motion_smooth_t,
        );

        // Gaze tracking (smoothed before the body/head writes; QT parity).
        let disable_gaze = self.wake_up_timer > 0.0
            || self.emotion == "SLEEPING"
            || self.current_emotion_tag == "SLEEPING";
        let target_gaze_x = if self.wake_up_timer > 0.0 {
            self.wake_up_gaze_x
        } else if disable_gaze {
            0.0
        } else {
            js_or_zero(self.eye_tracking_x)
        };
        let target_gaze_y = if disable_gaze {
            0.0
        } else {
            js_or_zero(self.eye_tracking_y)
        };
        let gaze_smooth_t = js_max(0.0, js_min(1.0, delta_seconds * 5.0));
        self.smoothed_gaze_x = lerp(self.smoothed_gaze_x, target_gaze_x, gaze_smooth_t);
        self.smoothed_gaze_y = lerp(self.smoothed_gaze_y, target_gaze_y, gaze_smooth_t);

        let body_angle_x =
            self.current.body_angle_x + self.smoothed_body_x + (self.smoothed_gaze_x * 5.0);
        let body_angle_y =
            self.current.body_angle_y + self.smoothed_body_y + (self.smoothed_gaze_y * 5.0);
        let body_angle_z = self.current.body_angle_z + self.smoothed_body_z;
        let angle_x =
            self.current.head_angle_x + self.smoothed_head_x + (self.smoothed_gaze_x * 25.0);
        let angle_y =
            self.current.head_angle_y + self.smoothed_head_y + (self.smoothed_gaze_y * 25.0);
        let angle_z = self.current.head_angle_z + self.smoothed_head_z;

        // Breath cycles (the phase jumps when the speed switches — reproduced).
        let breath_cycle_speed = if self.emotion == "SLEEPING" { 0.8 } else { 1.2 };
        let breath = (js_sin(self.user_time * breath_cycle_speed) + 1.0) * 0.5;

        // 6. Lip sync & vowel mouth opening (lerp factors deliberately unclamped — V1 QUIRK).
        if is_sneeze_active {
            self.mouth_open = lerp(self.mouth_open, sneeze_mouth_open, 16.0 * delta_seconds);
        } else if self.lip_sync_value > 0.01 {
            let target_mouth = if !self.spoken_char.is_empty() {
                vowel_mouth_opening(&self.spoken_char)
            } else {
                let syllable = js_sin(self.user_time * 12.0).abs();
                let sub_beat = js_sin(self.user_time * 7.3).abs();
                let flutter = noise01(self.user_time * 8.0, 5.0);
                syllable * 0.5 + sub_beat * 0.3 + flutter * 0.2
            };
            let final_mouth = js_max(
                0.0,
                js_min(
                    1.0,
                    target_mouth * js_max(0.0, js_min(1.0, self.lip_sync_value)),
                ),
            );
            self.mouth_open = lerp(self.mouth_open, final_mouth, 14.0 * delta_seconds);
        } else {
            self.mouth_open = lerp(self.mouth_open, 0.0, 10.0 * delta_seconds);
        }

        let params = ParamFrame {
            values: [
                (PlayerParam::BrowLY, face.brow_y),
                (PlayerParam::BrowRY, face.brow_y),
                (PlayerParam::BrowLForm, face.brow_form),
                (PlayerParam::BrowRForm, face.brow_form),
                (PlayerParam::BrowLAngle, face.brow_angle),
                (PlayerParam::BrowRAngle, face.brow_angle),
                (PlayerParam::EyeLSmile, face.eye_smile),
                (PlayerParam::EyeRSmile, face.eye_smile),
                (PlayerParam::MouthForm, face.mouth_form),
                (PlayerParam::Cheek, face.cheek),
                (PlayerParam::EyeLOpen, eye_open),
                (PlayerParam::EyeROpen, eye_r_open),
                (PlayerParam::BodyAngleX, body_angle_x),
                (PlayerParam::BodyAngleY, body_angle_y),
                (PlayerParam::BodyAngleZ, body_angle_z),
                (PlayerParam::AngleX, angle_x),
                (PlayerParam::AngleY, angle_y),
                (PlayerParam::AngleZ, angle_z),
                (PlayerParam::EyeBallX, self.smoothed_gaze_x),
                (PlayerParam::EyeBallY, self.smoothed_gaze_y),
                (PlayerParam::Breath, breath),
                (PlayerParam::MouthOpenY, self.mouth_open),
            ],
        };
        DriverFrame {
            delta_seconds,
            expression,
            params,
        }
    }
}

/// `findExpressionName(model, tag)` over the model's expression names (insertion order): preferred
/// key (`panic` → `surprised`), else the first name containing the lower-cased tag, else the first
/// name. `None` only when there are no expressions.
pub fn find_expression_name<'a>(names: &'a [String], tag: &str) -> Option<&'a str> {
    if names.is_empty() {
        return None;
    }
    let needle = tag.to_lowercase();
    let preferred = match needle.as_str() {
        "panic" => "surprised",
        other => other,
    };
    let lower = |name: &String| name.to_lowercase();
    names
        .iter()
        .find(|name| lower(name) == preferred)
        .or_else(|| names.iter().find(|name| lower(name).contains(&needle)))
        .or_else(|| names.first())
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed(values: &'static [f64]) -> impl FnMut() -> f64 {
        let mut i = 0;
        move || {
            let v = values.get(i).copied().unwrap_or(0.5);
            i += 1;
            v
        }
    }

    #[test]
    fn initial_blink_interval_uses_first_draw() {
        let mut rng = fixed(&[0.0, 0.999]);
        assert_eq!(EmotionDriver::new(&mut rng).blink_interval(), 3.0);
        let d = EmotionDriver::new(&mut rng);
        assert!((d.blink_interval() - (3.0 + 0.999 * 3.5)).abs() < 1e-15);
    }

    #[test]
    fn set_emotion_semantics() {
        let mut rng = fixed(&[0.5]);
        let mut d = EmotionDriver::new(&mut rng);
        // Same tag (after trim/upper-casing) is a no-op.
        d.set_emotion(" normal ");
        assert_eq!(d.burst_progress(), 1.0);
        assert_eq!(d.active_burst_tag(), "");
        d.set_emotion("smile");
        assert_eq!(d.emotion(), "SMILE");
        assert_eq!(d.current_emotion_tag(), "SMILE");
        assert_eq!(d.active_burst_tag(), "SMILE");
        assert_eq!(d.burst_progress(), 0.0);
        // Sleeping → anything: startled wake-up as SURPRISED.
        d.set_emotion("SLEEPING");
        d.set_emotion("NORMAL");
        assert_eq!(d.wake_up_timer(), 2.5);
        assert_eq!(d.current_emotion_tag(), "SURPRISED");
        assert_eq!(d.active_burst_tag(), "SURPRISED");
        // SURPRISED during the wake-up: cancelled, no new burst (tag already SURPRISED).
        d.set_emotion("SURPRISED");
        assert_eq!(d.wake_up_timer(), 0.0);
        assert_eq!(d.current_emotion_tag(), "SURPRISED");
        // Re-entering SLEEPING during a wake-up restarts the nod burst.
        d.set_emotion("SLEEPING");
        d.set_emotion("WINK");
        assert_eq!(d.wake_up_timer(), 2.5);
        d.set_emotion("SLEEPING");
        assert_eq!(d.wake_up_timer(), 0.0);
        assert_eq!(d.active_burst_tag(), "SLEEPING");
        assert_eq!(d.current_emotion_tag(), "SLEEPING");
    }

    #[test]
    fn setters_clamp_and_map_nan() {
        let mut rng = fixed(&[0.5]);
        let mut d = EmotionDriver::new(&mut rng);
        d.set_lip_sync(3.0);
        assert_eq!(d.lip_sync_value(), 1.0);
        d.set_lip_sync(-1.0);
        assert_eq!(d.lip_sync_value(), 0.0);
        d.set_lip_sync(f64::NAN);
        assert_eq!(d.lip_sync_value(), 0.0);
        d.set_eye_tracking(2.0, f64::NAN);
        assert_eq!(d.eye_tracking(), (1.0, 0.0));
        d.set_spoken_char("あ");
        assert_eq!(d.spoken_char(), "あ");
    }

    #[test]
    fn first_step_has_zero_delta_and_neutral_face() {
        let mut rng = fixed(&[0.5]);
        let mut d = EmotionDriver::new(&mut rng);
        let f = d.step(1000.0, &mut rng, &[]);
        assert_eq!(f.delta_seconds, 0.0);
        assert_eq!(f.params.get(PlayerParam::Breath), 0.5);
        assert_eq!(f.params.get(PlayerParam::BrowLForm), 0.5);
        assert_eq!(f.params.get(PlayerParam::EyeLOpen), 1.0);
        assert_eq!(f.params.get(PlayerParam::AngleX), 0.0);
        assert!(f.expression.is_none());
        let order: Vec<PlayerParam> = f.params.values.iter().map(|(p, _)| *p).collect();
        assert_eq!(order, PlayerParam::WRITE_ORDER.to_vec());
    }

    #[test]
    fn wink_closes_right_eye_and_wake_up_drops_it() {
        let mut rng = fixed(&[0.5, 0.5, 0.5, 0.5]);
        let mut d = EmotionDriver::new(&mut rng);
        d.set_emotion("WINK");
        d.step(10.0, &mut rng, &[]);
        let f = d.step(10.1, &mut rng, &[]);
        assert_eq!(f.params.get(PlayerParam::EyeROpen), 0.0);
        assert!(f.params.get(PlayerParam::EyeLOpen) > 0.0);
        // Waking up into WINK keeps the right eye open until the wake-up ends (QUIRK).
        d.set_emotion("SLEEPING");
        d.set_emotion("WINK");
        let f = d.step(10.2, &mut rng, &[]);
        assert!(f.params.get(PlayerParam::EyeROpen) > 0.0);
    }

    #[test]
    fn sneeze_after_45s_idle_with_low_draw() {
        let mut rng = fixed(&[0.5, 0.1]);
        let mut d = EmotionDriver::new(&mut rng);
        let mut now = 100.0;
        d.step(now, &mut rng, &[]);
        // ~450 steps of 0.1 s (every draw, blink intervals included, returns 0.1).
        let mut rng_low = || 0.1;
        for _ in 0..500 {
            now += 0.1;
            d.step(now, &mut rng_low, &[]);
            if d.active_burst_tag() == "SNEEZE" {
                break;
            }
        }
        assert_eq!(d.active_burst_tag(), "SNEEZE");
        assert_eq!(d.emotion(), "NORMAL");
    }

    #[test]
    fn expression_names_lookup() {
        let names: Vec<String> = ["f01", "Smile", "Surprised", "sadness"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(find_expression_name(&names, "SMILE"), Some("Smile"));
        assert_eq!(find_expression_name(&names, "PANIC"), Some("Surprised"));
        assert_eq!(find_expression_name(&names, "SAD"), Some("sadness"));
        assert_eq!(find_expression_name(&names, "NORMAL"), Some("f01"));
        assert_eq!(find_expression_name(&[], "NORMAL"), None);
        // The hook fires once per tag change.
        let mut rng = fixed(&[0.5]);
        let mut d = EmotionDriver::new(&mut rng);
        assert_eq!(
            d.step(1.0, &mut rng, &names).expression.as_deref(),
            Some("f01")
        );
        assert_eq!(d.step(1.1, &mut rng, &names).expression, None);
        d.set_emotion("smile");
        assert_eq!(
            d.step(1.2, &mut rng, &names).expression.as_deref(),
            Some("Smile")
        );
    }
}
