//! `CubismMotion` — motion3.json motions, port of `Framework/src/motion/cubismmotion.ts` and
//! `cubismmotionjson.ts` (spec §14.2, §14.3, §14.6).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! QUIRKs kept: `Meta.Loop` is ignored (motions never loop unless `set_loop(true)`); curve groups
//! must be ordered Model → Parameter → PartOpacity (an out-of-order curve stops processing);
//! PartOpacity curves write the *parameter* named like the part; the finished callback fires only on
//! natural end (and on every loop in `MotionBehavior::V2`). Declared safe deviations: segment/point
//! arrays are sized from the data (the TS trusts `Meta` totals and throws when they are too small)
//! and an unknown segment type is an error (the TS loops forever).

use super::acubism_motion::{ACubismMotion, MotionBase, MotionCallback};
use super::internal::*;
use super::queue_entry::CubismMotionQueueEntry;
use crate::core::model::Live2DError;
use crate::framework::json::{self, get, index};
use crate::framework::parameter_store::ParameterStore;
use crate::math::cubism_math;

/// `EffectNameEyeBlink`.
pub const EFFECT_NAME_EYE_BLINK: &str = "EyeBlink";
/// `EffectNameLipSync`.
pub const EFFECT_NAME_LIP_SYNC: &str = "LipSync";
/// `TargetNameModel`.
pub const TARGET_NAME_MODEL: &str = "Model";
/// `TargetNameParameter`.
pub const TARGET_NAME_PARAMETER: &str = "Parameter";
/// `TargetNamePartOpacity`.
pub const TARGET_NAME_PART_OPACITY: &str = "PartOpacity";
/// `IdNameOpacity`.
pub const ID_NAME_OPACITY: &str = "Opacity";
/// `UseOldBeziersCurveMotion`.
pub const USE_OLD_BEZIERS_CURVE_MOTION: bool = false;
/// Maximum number of eye-blink / lip-sync targets tracked by the flag bitmasks.
const MAX_TARGET_SIZE: usize = 64;

/// `MotionBehavior`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionBehavior {
    V1 = 0,
    #[default]
    V2 = 1,
}

/// A motion3.json motion (`CubismMotion`).
#[derive(Debug, Clone)]
pub struct CubismMotion {
    base: MotionBase,
    motion_behavior: MotionBehavior,
    source_frame_rate: f64,
    loop_duration_seconds: f64,
    last_weight: f64,
    motion_data: Option<CubismMotionData>,
    eye_blink_parameter_ids: Vec<String>,
    lip_sync_parameter_ids: Vec<String>,
    model_opacity: f64,
    debug_mode: bool,
}

impl Default for CubismMotion {
    fn default() -> Self {
        Self {
            base: MotionBase::default(),
            motion_behavior: MotionBehavior::V2,
            source_frame_rate: 30.0,
            loop_duration_seconds: -1.0,
            last_weight: 0.0,
            motion_data: None,
            eye_blink_parameter_ids: Vec::new(),
            lip_sync_parameter_ids: Vec::new(),
            model_opacity: 1.0,
            debug_mode: false,
        }
    }
}

/// JS `1 << i` (32-bit, shift count mod 32).
#[inline]
fn js_bit(i: usize) -> u32 {
    1u32.wrapping_shl(i as u32)
}

/// JS `(flags >> i) & 0x01` (32-bit, shift count mod 32).
#[inline]
fn js_flag_is_set(flags: u32, i: usize) -> bool {
    flags.wrapping_shr(i as u32) & 1 != 0
}

/// Reads segment float `k` of curve `c` (`getMotionCurveSegment(c, k)`: `toFloat()`, 0 if missing).
#[inline]
fn segment_value(segments: Option<&serde_json::Value>, k: usize) -> f64 {
    json::to_float(index(segments, k), 0.0)
}

impl CubismMotion {
    /// `CubismMotion.create(buffer, size, onFinished, onBegan, shouldCheckMotionConsistency)`.
    /// A consistency failure (only when checking) is an error, like the TS `null`.
    pub fn create(
        bytes: &[u8],
        on_finished_motion: Option<MotionCallback>,
        on_began_motion: Option<MotionCallback>,
        should_check_motion_consistency: bool,
    ) -> Result<CubismMotion, Live2DError> {
        let mut ret = CubismMotion::default();
        ret.parse(bytes, should_check_motion_consistency)?;
        let Some(data) = ret.motion_data.as_ref() else {
            return Err(Live2DError::Load("Inconsistent motion3.json.".to_owned()));
        };
        ret.source_frame_rate = data.fps;
        ret.loop_duration_seconds = data.duration;
        ret.base.on_finished_motion = on_finished_motion;
        ret.base.on_began_motion = on_began_motion;
        Ok(ret)
    }

    /// `parse(motionJson, size, shouldCheckMotionConsistency)`.
    fn parse(
        &mut self,
        bytes: &[u8],
        should_check_motion_consistency: bool,
    ) -> Result<(), Live2DError> {
        let root_value = json::parse_json(bytes, "motion3.json")?;
        let root = Some(&root_value);
        if should_check_motion_consistency && !has_consistency(root) {
            log::error!(target: "cubism", "Inconsistent motion3.json.");
            return Ok(());
        }
        let meta = get(root, "Meta");
        let mut data = CubismMotionData {
            duration: json::to_float(get(meta, "Duration"), 0.0),
            r#loop: json::to_bool(get(meta, "Loop"), false),
            curve_count: json::to_count(get(meta, "CurveCount")),
            fps: json::to_float(get(meta, "Fps"), 0.0),
            event_count: json::to_count(get(meta, "UserDataCount")),
            ..CubismMotionData::default()
        };
        let are_beziers_restricted = json::to_bool(get(meta, "AreBeziersRestricted"), false);

        let fade_in = get(meta, "FadeInTime");
        self.base.fade_in_seconds = if !json::is_null(fade_in) {
            let v = json::to_float(fade_in, 0.0);
            if v < 0.0 {
                1.0
            } else {
                v
            }
        } else {
            1.0
        };
        let fade_out = get(meta, "FadeOutTime");
        self.base.fade_out_seconds = if !json::is_null(fade_out) {
            let v = json::to_float(fade_out, 0.0);
            if v < 0.0 {
                1.0
            } else {
                v
            }
        } else {
            1.0
        };

        let total_segment_count_meta = json::to_count(get(meta, "TotalSegmentCount"));
        let total_point_count_meta = json::to_count(get(meta, "TotalPointCount"));
        data.segments.reserve(total_segment_count_meta);
        data.points.reserve(total_point_count_meta);

        let curves_json = get(root, "Curves");
        for curve_index in 0..data.curve_count {
            let curve_json = index(curves_json, curve_index);
            let target = match json::raw_string(get(curve_json, "Target")).as_deref() {
                Some(TARGET_NAME_MODEL) => CubismMotionCurveTarget::Model,
                Some(TARGET_NAME_PARAMETER) => CubismMotionCurveTarget::Parameter,
                Some(TARGET_NAME_PART_OPACITY) => CubismMotionCurveTarget::PartOpacity,
                _ => {
                    log::warn!(
                        target: "cubism",
                        "Warning : Unable to get segment type from Curve! The number of \"CurveCount\" may be incorrect!"
                    );
                    CubismMotionCurveTarget::Model
                }
            };
            let fade_in_time = get(curve_json, "FadeInTime");
            let fade_out_time = get(curve_json, "FadeOutTime");
            let mut curve = CubismMotionCurve {
                target,
                id: json::raw_string_or_empty(get(curve_json, "Id")),
                segment_count: 0,
                base_segment_index: data.segments.len(),
                fade_in_time: if !json::is_null(fade_in_time) {
                    json::to_float(fade_in_time, 0.0)
                } else {
                    -1.0
                },
                fade_out_time: if !json::is_null(fade_out_time) {
                    json::to_float(fade_out_time, 0.0)
                } else {
                    -1.0
                },
            };

            let segments = get(curve_json, "Segments");
            let segment_float_count = json::vector_len(segments);
            let mut segment_position = 0usize;
            while segment_position < segment_float_count {
                let base_point_index = if segment_position == 0 {
                    data.points.push(CubismMotionPoint {
                        time: segment_value(segments, segment_position),
                        value: segment_value(segments, segment_position + 1),
                    });
                    segment_position += 2;
                    data.points.len() - 1
                } else {
                    data.points.len().saturating_sub(1)
                };
                let segment_type = segment_value(segments, segment_position);
                let (segment_type, evaluator, point_count) = if segment_type == 0.0 {
                    (CubismMotionSegmentType::Linear, SegmentEvaluator::Linear, 1)
                } else if segment_type == 1.0 {
                    let evaluator = if are_beziers_restricted || USE_OLD_BEZIERS_CURVE_MOTION {
                        SegmentEvaluator::Bezier
                    } else {
                        SegmentEvaluator::BezierCardano
                    };
                    (CubismMotionSegmentType::Bezier, evaluator, 3)
                } else if segment_type == 2.0 {
                    (
                        CubismMotionSegmentType::Stepped,
                        SegmentEvaluator::Stepped,
                        1,
                    )
                } else if segment_type == 3.0 {
                    (
                        CubismMotionSegmentType::InverseStepped,
                        SegmentEvaluator::InverseStepped,
                        1,
                    )
                } else {
                    return Err(Live2DError::Load(format!(
                        "motion3.json: unknown segment type {segment_type} in curve {curve_index}"
                    )));
                };
                for k in 0..point_count {
                    data.points.push(CubismMotionPoint {
                        time: segment_value(segments, segment_position + 1 + 2 * k),
                        value: segment_value(segments, segment_position + 2 + 2 * k),
                    });
                }
                segment_position += 1 + 2 * point_count;
                data.segments.push(CubismMotionSegment {
                    evaluator,
                    base_point_index,
                    segment_type,
                });
                curve.segment_count += 1;
            }
            data.curves.push(curve);
        }
        if data.segments.len() != total_segment_count_meta
            || data.points.len() != total_point_count_meta
        {
            log::warn!(
                target: "cubism",
                "motion3.json: Meta totals (segments {total_segment_count_meta}, points {total_point_count_meta}) differ \
                 from the data ({}, {}); using the data",
                data.segments.len(),
                data.points.len()
            );
        }

        let user_data = get(root, "UserData");
        for u in 0..data.event_count {
            let event = index(user_data, u);
            data.events.push(CubismMotionEvent {
                fire_time: json::to_float(get(event, "Time"), 0.0),
                value: json::raw_string_or_empty(get(event, "Value")),
            });
        }
        self.motion_data = Some(data);
        Ok(())
    }

    /// The parsed motion data (`None` after `release()`).
    pub fn motion_data(&self) -> Option<&CubismMotionData> {
        self.motion_data.as_ref()
    }

    /// `setMotionBehavior(behavior)`.
    pub fn set_motion_behavior(&mut self, motion_behavior: MotionBehavior) {
        self.motion_behavior = motion_behavior;
    }

    /// `getMotionBehavior()`.
    pub fn motion_behavior(&self) -> MotionBehavior {
        self.motion_behavior
    }

    /// `_sourceFrameRate` (= `Meta.Fps`).
    pub fn source_frame_rate(&self) -> f64 {
        self.source_frame_rate
    }

    /// `_lastWeight`.
    pub fn last_weight(&self) -> f64 {
        self.last_weight
    }

    /// `setParameterFadeInTime(parameterId, value)` (first curve with that ID).
    pub fn set_parameter_fade_in_time(&mut self, parameter_id: &str, value: f64) {
        if let Some(c) = self.curve_mut(parameter_id) {
            c.fade_in_time = value;
        }
    }

    /// `setParameterFadeOutTime(parameterId, value)`.
    pub fn set_parameter_fade_out_time(&mut self, parameter_id: &str, value: f64) {
        if let Some(c) = self.curve_mut(parameter_id) {
            c.fade_out_time = value;
        }
    }

    /// `getParameterFadeInTime(parameterId)` (−1 when absent).
    pub fn parameter_fade_in_time(&self, parameter_id: &str) -> f64 {
        self.curves()
            .iter()
            .find(|c| c.id == parameter_id)
            .map_or(-1.0, |c| c.fade_in_time)
    }

    /// `getParameterFadeOutTime(parameterId)` (−1 when absent).
    pub fn parameter_fade_out_time(&self, parameter_id: &str) -> f64 {
        self.curves()
            .iter()
            .find(|c| c.id == parameter_id)
            .map_or(-1.0, |c| c.fade_out_time)
    }

    /// `setEffectIds(eyeBlinkParameterIds, lipSyncParameterIds)`.
    pub fn set_effect_ids(
        &mut self,
        eye_blink_parameter_ids: Vec<String>,
        lip_sync_parameter_ids: Vec<String>,
    ) {
        self.eye_blink_parameter_ids = eye_blink_parameter_ids;
        self.lip_sync_parameter_ids = lip_sync_parameter_ids;
    }

    /// `setDebugMode(debugMode)`.
    pub fn set_debug_mode(&mut self, debug_mode: bool) {
        self.debug_mode = debug_mode;
    }

    fn curves(&self) -> &[CubismMotionCurve] {
        self.motion_data
            .as_ref()
            .map_or(&[], |d| &d.curves[..d.curve_count.min(d.curves.len())])
    }

    fn curve_mut(&mut self, parameter_id: &str) -> Option<&mut CubismMotionCurve> {
        let data = self.motion_data.as_mut()?;
        let n = data.curve_count.min(data.curves.len());
        data.curves[..n].iter_mut().find(|c| c.id == parameter_id)
    }

    /// `updateForNextLoop(motionQueueEntry, userTimeSeconds, time)`.
    fn update_for_next_loop(
        &self,
        entry: &mut CubismMotionQueueEntry,
        user_time_seconds: f64,
        time: f64,
    ) {
        match self.motion_behavior {
            MotionBehavior::V2 => {
                entry.set_start_time(user_time_seconds - time);
                if self.base.is_loop_fade_in {
                    entry.set_fade_in_start_time(user_time_seconds - time);
                }
                if let Some(callback) = self.base.on_finished_motion.clone() {
                    callback(self);
                }
            }
            MotionBehavior::V1 => {
                entry.set_start_time(user_time_seconds);
                if self.base.is_loop_fade_in {
                    entry.set_fade_in_start_time(user_time_seconds);
                }
            }
        }
    }
}

impl ACubismMotion for CubismMotion {
    fn base(&self) -> &MotionBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut MotionBase {
        &mut self.base
    }

    fn as_dyn(&self) -> &dyn ACubismMotion {
        self
    }

    fn as_cubism_motion(&self) -> Option<&CubismMotion> {
        Some(self)
    }

    fn as_cubism_motion_mut(&mut self) -> Option<&mut CubismMotion> {
        Some(self)
    }

    /// `getDuration()`: −1 while looping, otherwise the motion length.
    fn duration(&self) -> f64 {
        if self.base.is_loop {
            -1.0
        } else {
            self.loop_duration_seconds
        }
    }

    fn loop_duration(&self) -> f64 {
        self.loop_duration_seconds
    }

    /// `getFiredEvent(before, now)`: values of the events with `before < fireTime <= now`.
    fn fired_event(
        &mut self,
        before_check_time_seconds: f64,
        motion_time_seconds: f64,
    ) -> Vec<String> {
        self.base.fired_event_values.clear();
        if let Some(data) = self.motion_data.as_ref() {
            for event in data.events.iter().take(data.event_count) {
                if event.fire_time > before_check_time_seconds
                    && event.fire_time <= motion_time_seconds
                {
                    self.base.fired_event_values.push(event.value.clone());
                }
            }
        }
        self.base.fired_event_values.clone()
    }

    fn is_exist_model_opacity(&self) -> bool {
        self.model_opacity_index() >= 0
    }

    fn model_opacity_index(&self) -> i64 {
        self.curves()
            .iter()
            .position(|c| c.target == CubismMotionCurveTarget::Model && c.id == ID_NAME_OPACITY)
            .map_or(-1, |i| i as i64)
    }

    fn model_opacity_id(&self, index: i64) -> Option<String> {
        let curve = self.curves().get(usize::try_from(index).ok()?)?;
        (curve.target == CubismMotionCurveTarget::Model && curve.id == ID_NAME_OPACITY)
            .then(|| curve.id.clone())
    }

    fn model_opacity_value(&self) -> f64 {
        self.model_opacity
    }

    /// `release()`: drops the motion data (the weight is kept, like the TS override).
    fn release(&mut self) {
        self.motion_data = None;
    }

    /// `doUpdateParameters(model, userTimeSeconds, fadeWeight, motionQueueEntry)` (spec §14.6).
    fn do_update_parameters(
        &mut self,
        model: &mut dyn ParameterStore,
        user_time_seconds: f64,
        fade_weight: f64,
        entry: &mut CubismMotionQueueEntry,
    ) {
        if self.motion_data.is_none() {
            return;
        }
        if self.motion_behavior == MotionBehavior::V2
            && self.base.previous_loop_state != self.base.is_loop
        {
            self.adjust_end_time(entry);
            self.base.previous_loop_state = self.base.is_loop;
        }
        let Some(md) = self.motion_data.as_ref() else {
            return;
        };

        let mut time_offset_seconds = user_time_seconds - entry.start_time();
        if time_offset_seconds < 0.0 {
            time_offset_seconds = 0.0;
        }
        let mut lip_sync_value = f64::MAX;
        let mut eye_blink_value = f64::MAX;
        // JS bit operations are 32-bit (`1 << i` and `flags >> i` take the shift count mod 32), so
        // targets `i` and `i + 32` share a flag bit, exactly like the TS.
        let mut lip_sync_flags: u32 = 0;
        let mut eye_blink_flags: u32 = 0;
        if self.eye_blink_parameter_ids.len() > MAX_TARGET_SIZE {
            log::debug!(target: "cubism", "too many eye blink targets : {}", self.eye_blink_parameter_ids.len());
        }
        if self.lip_sync_parameter_ids.len() > MAX_TARGET_SIZE {
            log::debug!(target: "cubism", "too many lip sync targets : {}", self.lip_sync_parameter_ids.len());
        }

        let fade_in_seconds = self.base.fade_in_seconds;
        let fade_out_seconds = self.base.fade_out_seconds;
        let tmp_fade_in = if fade_in_seconds <= 0.0 {
            1.0
        } else {
            cubism_math::get_easing_sine(
                (user_time_seconds - entry.fade_in_start_time()) / fade_in_seconds,
            )
        };
        let tmp_fade_out = if fade_out_seconds <= 0.0 || entry.end_time() < 0.0 {
            1.0
        } else {
            cubism_math::get_easing_sine((entry.end_time() - user_time_seconds) / fade_out_seconds)
        };

        let mut time = time_offset_seconds;
        let mut duration = md.duration;
        let is_correction = self.motion_behavior == MotionBehavior::V2 && self.base.is_loop;
        if self.base.is_loop {
            if self.motion_behavior == MotionBehavior::V2 {
                duration += 1.0 / md.fps;
            }
            while time > duration {
                time -= duration;
            }
        }

        let curves = &md.curves;
        let curve_count = md.curve_count.min(curves.len());
        let mut c = 0;

        // (1) Model curves (only while consecutive curves from index 0 are Model curves).
        while c < curve_count && curves[c].target == CubismMotionCurveTarget::Model {
            let value = evaluate_curve(md, c, time, is_correction, duration);
            if curves[c].id == EFFECT_NAME_EYE_BLINK {
                eye_blink_value = value;
            } else if curves[c].id == EFFECT_NAME_LIP_SYNC {
                lip_sync_value = value;
            } else if curves[c].id == ID_NAME_OPACITY {
                self.model_opacity = value;
                model.set_model_opacity(value);
            }
            c += 1;
        }

        // (2) Parameter curves.
        while c < curve_count && curves[c].target == CubismMotionCurveTarget::Parameter {
            let curve = &curves[c];
            let parameter_index = model.parameter_index(&curve.id);
            let source_value = model.parameter_value_f64(parameter_index);
            let mut value = evaluate_curve(md, c, time, is_correction, duration);

            if eye_blink_value != f64::MAX {
                for (i, id) in self
                    .eye_blink_parameter_ids
                    .iter()
                    .take(MAX_TARGET_SIZE)
                    .enumerate()
                {
                    if *id == curve.id {
                        value *= eye_blink_value;
                        eye_blink_flags |= js_bit(i);
                        break;
                    }
                }
            }
            if lip_sync_value != f64::MAX {
                for (i, id) in self
                    .lip_sync_parameter_ids
                    .iter()
                    .take(MAX_TARGET_SIZE)
                    .enumerate()
                {
                    if *id == curve.id {
                        value += lip_sync_value;
                        lip_sync_flags |= js_bit(i);
                        break;
                    }
                }
            }
            if model.is_repeat(parameter_index) {
                value = model.parameter_repeat_value(parameter_index, value);
            }

            let v = if curve.fade_in_time < 0.0 && curve.fade_out_time < 0.0 {
                source_value + (value - source_value) * fade_weight
            } else {
                let fin = if curve.fade_in_time < 0.0 {
                    tmp_fade_in
                } else if curve.fade_in_time == 0.0 {
                    1.0
                } else {
                    cubism_math::get_easing_sine(
                        (user_time_seconds - entry.fade_in_start_time()) / curve.fade_in_time,
                    )
                };
                let fout = if curve.fade_out_time < 0.0 {
                    tmp_fade_out
                } else if curve.fade_out_time == 0.0 || entry.end_time() < 0.0 {
                    1.0
                } else {
                    cubism_math::get_easing_sine(
                        (entry.end_time() - user_time_seconds) / curve.fade_out_time,
                    )
                };
                let param_weight = self.base.weight * fin * fout;
                source_value + (value - source_value) * param_weight
            };
            model.set_parameter_value_f64(parameter_index, v, 1.0);
            c += 1;
        }

        // (3) Automatic eye blink / lip sync for targets without their own curve.
        if eye_blink_value != f64::MAX {
            for (i, id) in self
                .eye_blink_parameter_ids
                .iter()
                .take(MAX_TARGET_SIZE)
                .enumerate()
            {
                let source_value = model.parameter_value_by_id_f64(id);
                if js_flag_is_set(eye_blink_flags, i) {
                    continue;
                }
                let v = source_value + (eye_blink_value - source_value) * fade_weight;
                model.set_parameter_value_by_id_f64(id, v, 1.0);
            }
        }
        if lip_sync_value != f64::MAX {
            for (i, id) in self
                .lip_sync_parameter_ids
                .iter()
                .take(MAX_TARGET_SIZE)
                .enumerate()
            {
                let source_value = model.parameter_value_by_id_f64(id);
                if js_flag_is_set(lip_sync_flags, i) {
                    continue;
                }
                let v = source_value + (lip_sync_value - source_value) * fade_weight;
                model.set_parameter_value_by_id_f64(id, v, 1.0);
            }
        }

        // (4) PartOpacity curves → the parameter named like the part (unblended).
        while c < curve_count && curves[c].target == CubismMotionCurveTarget::PartOpacity {
            let parameter_index = model.parameter_index(&curves[c].id);
            let value = evaluate_curve(md, c, time, is_correction, duration);
            model.set_parameter_value_f64(parameter_index, value, 1.0);
            c += 1;
        }

        if time_offset_seconds >= duration {
            if self.base.is_loop {
                self.update_for_next_loop(entry, user_time_seconds, time);
            } else {
                if let Some(callback) = self.base.on_finished_motion.clone() {
                    callback(self);
                }
                entry.set_is_finished(true);
            }
        }
        self.last_weight = fade_weight;
    }
}

/// `CubismMotionJson.hasConsistency()` (only used when consistency checking is enabled; V1 never
/// enables it). An unknown segment type makes the file inconsistent instead of looping forever.
fn has_consistency(root: Option<&serde_json::Value>) -> bool {
    let Some(serde_json::Value::Object(_)) = root else {
        return false;
    };
    let meta = get(root, "Meta");
    let curves = get(root, "Curves");
    let actual_curve_list_size = json::vector_len(curves);
    let mut actual_total_segment_count = 0usize;
    let mut actual_total_point_count = 0usize;
    for curve_position in 0..actual_curve_list_size {
        let segments = get(index(curves, curve_position), "Segments");
        let n = json::vector_len(segments);
        let mut segment_position = 0usize;
        while segment_position < n {
            if segment_position == 0 {
                actual_total_point_count += 1;
                segment_position += 2;
            }
            let segment = segment_value(segments, segment_position);
            if segment == 0.0 || segment == 2.0 || segment == 3.0 {
                actual_total_point_count += 1;
                segment_position += 3;
            } else if segment == 1.0 {
                actual_total_point_count += 3;
                segment_position += 7;
            } else {
                return false;
            }
            actual_total_segment_count += 1;
        }
    }
    let mut result = true;
    if actual_curve_list_size as f64 != json::to_int(get(meta, "CurveCount"), 0.0) {
        log::warn!(target: "cubism", "The number of curves does not match the metadata.");
        result = false;
    }
    if actual_total_segment_count as f64 != json::to_int(get(meta, "TotalSegmentCount"), 0.0) {
        log::warn!(target: "cubism", "The number of segment does not match the metadata.");
        result = false;
    }
    if actual_total_point_count as f64 != json::to_int(get(meta, "TotalPointCount"), 0.0) {
        log::warn!(target: "cubism", "The number of point does not match the metadata.");
        result = false;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_segment_types() {
        let src = br#"{"Version":3,"Meta":{"Duration":2,"Fps":30,"Loop":true,"AreBeziersRestricted":false,
            "CurveCount":2,"TotalSegmentCount":4,"TotalPointCount":8,"UserDataCount":1,"TotalUserDataSize":3},
            "Curves":[{"Target":"Parameter","Id":"A","Segments":[0,0, 0,0.5,1, 1,0.7,1,0.9,1,1.0,0, 2,1.5,2]},
                      {"Target":"Parameter","Id":"B","FadeInTime":0.2,"Segments":[0,1, 3,2,5]}],
            "UserData":[{"Time":1.0,"Value":"ev"}]}"#;
        let m = CubismMotion::create(src, None, None, true).unwrap_or_default();
        let md = m.motion_data().cloned().unwrap_or_default();
        assert_eq!(md.curves.len(), 2);
        assert_eq!(md.segments.len(), 4);
        assert_eq!(md.points.len(), 8);
        assert_eq!(md.segments[1].evaluator, SegmentEvaluator::BezierCardano);
        assert_eq!(md.curves[1].fade_in_time, 0.2);
        assert_eq!(md.curves[1].fade_out_time, -1.0);
        assert_eq!(evaluate_curve(&md, 0, 0.25, false, 2.0), 0.5);
        assert_eq!(evaluate_curve(&md, 0, 1.25, false, 2.0), 0.0); // stepped: first point value
        assert_eq!(evaluate_curve(&md, 0, 1.75, false, 2.0), 2.0); // after the last point
        assert_eq!(evaluate_curve(&md, 0, 1.9, true, 2.0), 2.0); // loop wrap after a stepped segment
        assert_eq!(evaluate_curve(&md, 1, 2.5, true, 3.0), 1.0); // wrap after inverse stepped → first value
        assert_eq!(evaluate_curve(&md, 1, 1.0, false, 2.0), 5.0);
        assert!(!m.is_loop()); // Meta.Loop ignored
        assert_eq!((m.fade_in_time(), m.fade_out_time()), (1.0, 1.0));
        let bad = br#"{"Meta":{"CurveCount":1},"Curves":[{"Target":"Parameter","Id":"A","Segments":[0,0,9,1,1]}]}"#;
        assert!(CubismMotion::create(bad, None, None, false).is_err());
    }

    #[test]
    fn eye_blink_flags_alias_like_js_32_bit_shifts() {
        use crate::framework::model_state::ModelState;
        use crate::framework::motion::motion_ref;
        // EyeBlink model curve 0.5; a Parameter curve only for target #32. JS `1 << 32 == 1`, so
        // target #0 counts as "driven by a curve" and is skipped by the automatic blink.
        let src =
            br#"{"Meta":{"Duration":1,"Fps":30,"CurveCount":2,"FadeInTime":0,"FadeOutTime":0},
            "Curves":[{"Target":"Model","Id":"EyeBlink","Segments":[0,0.5,0,1,0.5]},
                      {"Target":"Parameter","Id":"E32","Segments":[0,1,0,1,1]}]}"#;
        let mut motion = CubismMotion::create(src, None, None, false).unwrap_or_default();
        let ids: Vec<String> = (0..33).map(|i| format!("E{i}")).collect();
        motion.set_effect_ids(ids.clone(), Vec::new());
        let n = ids.len();
        let mut m = ModelState::new(
            ids,
            vec![1.0; n],
            vec![0.0; n],
            vec![1.0; n],
            vec![1.0; n],
            vec![false; n],
            Vec::new(),
            Vec::new(),
        );
        let mut entry = CubismMotionQueueEntry::new(motion_ref(motion.clone()), false);
        motion.update_parameters(&mut m, &mut entry, 0.0);
        assert_eq!(m.parameter_value_f64(32), 0.5); // curve 1.0 × blink 0.5
        assert_eq!(m.parameter_value_f64(0), 1.0); // aliased flag bit → untouched
        assert_eq!(m.parameter_value_f64(1), 0.5); // automatic blink
        assert_eq!(m.parameter_value_f64(31), 0.5);
    }
}
