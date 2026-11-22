//! `ACubismMotion` — the abstract motion (fades, weight, loop flags, callbacks) of
//! `Framework/src/motion/acubismmotion.ts` (spec §14.5).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! Rust shape: the shared fields live in [`MotionBase`]; [`ACubismMotion`] is an object-safe trait
//! whose provided methods are the TS template methods (`updateParameters`, `setupMotionQueueEntry`,
//! `updateFadeWeight`, `adjustEndTime`, fade/weight/loop accessors). Motions are shared between
//! queue entries through [`MotionRef`] (`Rc<RefCell<dyn ACubismMotion>>`), like JS references.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use super::cubism_motion::CubismMotion;
use super::expression_motion::CubismExpressionMotion;
use super::queue_entry::CubismMotionQueueEntry;
use crate::framework::parameter_store::ParameterStore;
use crate::math::cubism_math;

/// Began/finished handler (`(self: ACubismMotion) => void`). Called while the motion is mutably
/// borrowed, so it must not borrow the motion's `RefCell` again.
pub type MotionCallback = Rc<dyn Fn(&dyn ACubismMotion)>;

/// A motion shared between the owner and the queue entries.
pub type MotionRef = Rc<RefCell<dyn ACubismMotion>>;

/// Wraps a concrete motion into a [`MotionRef`].
pub fn motion_ref<M: ACubismMotion + 'static>(motion: M) -> MotionRef {
    Rc::new(RefCell::new(motion))
}

/// Fields of `ACubismMotion`.
#[derive(Clone)]
pub struct MotionBase {
    /// `_fadeInSeconds` (−1 = unset).
    pub fade_in_seconds: f64,
    /// `_fadeOutSeconds` (−1 = unset).
    pub fade_out_seconds: f64,
    /// `_weight` (1).
    pub weight: f64,
    /// `_offsetSeconds` (0).
    pub offset_seconds: f64,
    /// `_isLoop` (false).
    pub is_loop: bool,
    /// `_isLoopFadeIn` (true).
    pub is_loop_fade_in: bool,
    /// `_previousLoopState` (false).
    pub previous_loop_state: bool,
    /// `_firedEventValues`.
    pub fired_event_values: Vec<String>,
    /// `_onBeganMotion`.
    pub on_began_motion: Option<MotionCallback>,
    /// `_onFinishedMotion`.
    pub on_finished_motion: Option<MotionCallback>,
}

impl Default for MotionBase {
    fn default() -> Self {
        Self {
            fade_in_seconds: -1.0,
            fade_out_seconds: -1.0,
            weight: 1.0,
            offset_seconds: 0.0,
            is_loop: false,
            is_loop_fade_in: true,
            previous_loop_state: false,
            fired_event_values: Vec::new(),
            on_began_motion: None,
            on_finished_motion: None,
        }
    }
}

impl fmt::Debug for MotionBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MotionBase")
            .field("fade_in_seconds", &self.fade_in_seconds)
            .field("fade_out_seconds", &self.fade_out_seconds)
            .field("weight", &self.weight)
            .field("offset_seconds", &self.offset_seconds)
            .field("is_loop", &self.is_loop)
            .field("is_loop_fade_in", &self.is_loop_fade_in)
            .field("has_on_began_motion", &self.on_began_motion.is_some())
            .field("has_on_finished_motion", &self.on_finished_motion.is_some())
            .finish_non_exhaustive()
    }
}

/// The abstract motion (`ACubismMotion`).
pub trait ACubismMotion {
    /// Shared fields.
    fn base(&self) -> &MotionBase;
    /// Shared fields (mutable).
    fn base_mut(&mut self) -> &mut MotionBase;
    /// `self` as a trait object (for the began/finished callbacks).
    fn as_dyn(&self) -> &dyn ACubismMotion;

    /// `doUpdateParameters(model, userTimeSeconds, fadeWeight, motionQueueEntry)`.
    fn do_update_parameters(
        &mut self,
        model: &mut dyn ParameterStore,
        user_time_seconds: f64,
        fade_weight: f64,
        entry: &mut CubismMotionQueueEntry,
    );

    /// `getDuration()` (−1 = endless).
    fn duration(&self) -> f64 {
        -1.0
    }

    /// `getLoopDuration()`.
    fn loop_duration(&self) -> f64 {
        -1.0
    }

    /// `getFiredEvent(beforeCheckTimeSeconds, motionTimeSeconds)` (a copy of the values).
    fn fired_event(
        &mut self,
        _before_check_time_seconds: f64,
        _motion_time_seconds: f64,
    ) -> Vec<String> {
        self.base().fired_event_values.clone()
    }

    /// `isExistModelOpacity()`.
    fn is_exist_model_opacity(&self) -> bool {
        false
    }

    /// `getModelOpacityIndex()` (−1 = none).
    fn model_opacity_index(&self) -> i64 {
        -1
    }

    /// `getModelOpacityId(index)`.
    fn model_opacity_id(&self, _index: i64) -> Option<String> {
        None
    }

    /// `getModelOpacityValue()`.
    fn model_opacity_value(&self) -> f64 {
        1.0
    }

    /// `release()`: `_weight = 0` (CubismMotion drops its data instead).
    fn release(&mut self) {
        self.base_mut().weight = 0.0;
    }

    /// Downcast to an expression motion.
    fn as_expression(&self) -> Option<&CubismExpressionMotion> {
        None
    }

    /// Downcast to an expression motion (mutable).
    fn as_expression_mut(&mut self) -> Option<&mut CubismExpressionMotion> {
        None
    }

    /// Downcast to a `CubismMotion`.
    fn as_cubism_motion(&self) -> Option<&CubismMotion> {
        None
    }

    /// Downcast to a `CubismMotion` (mutable).
    fn as_cubism_motion_mut(&mut self) -> Option<&mut CubismMotion> {
        None
    }

    // ------------------------------------------------------------------ template methods

    /// `updateParameters(model, motionQueueEntry, userTimeSeconds)`.
    fn update_parameters(
        &mut self,
        model: &mut dyn ParameterStore,
        entry: &mut CubismMotionQueueEntry,
        user_time_seconds: f64,
    ) {
        if !entry.is_available() || entry.is_finished() {
            return;
        }
        self.setup_motion_queue_entry(entry, user_time_seconds);
        let fade_weight = self.update_fade_weight(entry, user_time_seconds);
        self.do_update_parameters(model, user_time_seconds, fade_weight, entry);
        if entry.end_time() > 0.0 && entry.end_time() < user_time_seconds {
            entry.set_is_finished(true);
        }
    }

    /// `setupMotionQueueEntry(motionQueueEntry, userTimeSeconds)`: first-update initialisation and
    /// the began callback.
    fn setup_motion_queue_entry(
        &mut self,
        entry: &mut CubismMotionQueueEntry,
        user_time_seconds: f64,
    ) {
        if entry.is_started() || !entry.is_available() {
            return;
        }
        entry.set_is_started(true);
        entry.set_start_time(user_time_seconds - self.base().offset_seconds);
        entry.set_fade_in_start_time(user_time_seconds);
        if entry.end_time() < 0.0 {
            self.adjust_end_time(entry);
        }
        if let Some(callback) = self.base().on_began_motion.clone() {
            callback(self.as_dyn());
        }
    }

    /// `updateFadeWeight(motionQueueEntry, userTimeSeconds)`.
    fn update_fade_weight(
        &mut self,
        entry: &mut CubismMotionQueueEntry,
        user_time_seconds: f64,
    ) -> f64 {
        let base = self.base();
        let fade_in = if base.fade_in_seconds == 0.0 {
            1.0
        } else {
            cubism_math::get_easing_sine(
                (user_time_seconds - entry.fade_in_start_time()) / base.fade_in_seconds,
            )
        };
        let fade_out = if base.fade_out_seconds == 0.0 || entry.end_time() < 0.0 {
            1.0
        } else {
            cubism_math::get_easing_sine(
                (entry.end_time() - user_time_seconds) / base.fade_out_seconds,
            )
        };
        let fade_weight = base.weight * fade_in * fade_out;
        entry.set_state(user_time_seconds, fade_weight);
        fade_weight
    }

    /// `adjustEndTime(motionQueueEntry)`: `start + duration`, or −1 for endless motions.
    fn adjust_end_time(&self, entry: &mut CubismMotionQueueEntry) {
        let duration = self.duration();
        let end_time = if duration <= 0.0 {
            -1.0
        } else {
            entry.start_time() + duration
        };
        entry.set_end_time(end_time);
    }

    // ------------------------------------------------------------------ field accessors

    /// `setFadeInTime(seconds)`.
    fn set_fade_in_time(&mut self, fade_in_seconds: f64) {
        self.base_mut().fade_in_seconds = fade_in_seconds;
    }
    /// `getFadeInTime()`.
    fn fade_in_time(&self) -> f64 {
        self.base().fade_in_seconds
    }
    /// `setFadeOutTime(seconds)`.
    fn set_fade_out_time(&mut self, fade_out_seconds: f64) {
        self.base_mut().fade_out_seconds = fade_out_seconds;
    }
    /// `getFadeOutTime()`.
    fn fade_out_time(&self) -> f64 {
        self.base().fade_out_seconds
    }
    /// `setWeight(weight)`.
    fn set_weight(&mut self, weight: f64) {
        self.base_mut().weight = weight;
    }
    /// `getWeight()`.
    fn weight(&self) -> f64 {
        self.base().weight
    }
    /// `setOffsetTime(seconds)`.
    fn set_offset_time(&mut self, offset_seconds: f64) {
        self.base_mut().offset_seconds = offset_seconds;
    }
    /// `setLoop(loop)`.
    fn set_loop(&mut self, is_loop: bool) {
        self.base_mut().is_loop = is_loop;
    }
    /// `getLoop()`.
    fn is_loop(&self) -> bool {
        self.base().is_loop
    }
    /// `setLoopFadeIn(value)`.
    fn set_loop_fade_in(&mut self, loop_fade_in: bool) {
        self.base_mut().is_loop_fade_in = loop_fade_in;
    }
    /// `getLoopFadeIn()`.
    fn is_loop_fade_in(&self) -> bool {
        self.base().is_loop_fade_in
    }
    /// `setBeganMotionHandler(handler)`.
    fn set_began_motion_handler(&mut self, handler: Option<MotionCallback>) {
        self.base_mut().on_began_motion = handler;
    }
    /// `getBeganMotionHandler()`.
    fn began_motion_handler(&self) -> Option<MotionCallback> {
        self.base().on_began_motion.clone()
    }
    /// `setFinishedMotionHandler(handler)`.
    fn set_finished_motion_handler(&mut self, handler: Option<MotionCallback>) {
        self.base_mut().on_finished_motion = handler;
    }
    /// `getFinishedMotionHandler()`.
    fn finished_motion_handler(&self) -> Option<MotionCallback> {
        self.base().on_finished_motion.clone()
    }
}
