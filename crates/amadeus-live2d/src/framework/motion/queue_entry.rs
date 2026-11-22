//! `CubismMotionQueueEntry` — port of `Framework/src/motion/cubismmotionqueueentry.ts` (spec §14.7).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use super::acubism_motion::MotionRef;

static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

/// Identity of a queue entry (the TS handle is the entry object itself; `-1`
/// (`InvalidMotionQueueEntryHandleValue`) is `Option::None` in Rust).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CubismMotionQueueEntryHandle(pub u64);

/// One playing motion (`CubismMotionQueueEntry`).
pub struct CubismMotionQueueEntry {
    pub(crate) auto_delete: bool,
    pub(crate) motion: MotionRef,
    available: bool,
    finished: bool,
    started: bool,
    start_time_seconds: f64,
    fade_in_start_time_seconds: f64,
    end_time_seconds: f64,
    state_time_seconds: f64,
    state_weight: f64,
    last_event_check_seconds: f64,
    handle: CubismMotionQueueEntryHandle,
    fade_out_seconds: f64,
    is_triggered_fade_out: bool,
}

impl fmt::Debug for CubismMotionQueueEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CubismMotionQueueEntry")
            .field("handle", &self.handle)
            .field("available", &self.available)
            .field("finished", &self.finished)
            .field("started", &self.started)
            .field("start_time_seconds", &self.start_time_seconds)
            .field(
                "fade_in_start_time_seconds",
                &self.fade_in_start_time_seconds,
            )
            .field("end_time_seconds", &self.end_time_seconds)
            .field("state_weight", &self.state_weight)
            .field("is_triggered_fade_out", &self.is_triggered_fade_out)
            .finish_non_exhaustive()
    }
}

impl CubismMotionQueueEntry {
    /// `new CubismMotionQueueEntry()` with `_autoDelete` / `_motion` set (as `startMotion` does).
    pub fn new(motion: MotionRef, auto_delete: bool) -> Self {
        Self {
            auto_delete,
            motion,
            available: true,
            finished: false,
            started: false,
            start_time_seconds: -1.0,
            fade_in_start_time_seconds: 0.0,
            end_time_seconds: -1.0,
            state_time_seconds: 0.0,
            state_weight: 0.0,
            last_event_check_seconds: 0.0,
            handle: CubismMotionQueueEntryHandle(NEXT_HANDLE.fetch_add(1, Ordering::Relaxed)),
            fade_out_seconds: 0.0,
            is_triggered_fade_out: false,
        }
    }

    /// `release()`: deletes (releases) the motion when `autoDelete` was requested.
    pub fn release(&mut self) {
        if self.auto_delete {
            if let Ok(mut motion) = self.motion.try_borrow_mut() {
                motion.release();
            }
        }
    }

    /// `setFadeOut(fadeOutSeconds)`: arms a fade-out applied by the next `startFadeOut`.
    pub fn set_fade_out(&mut self, fade_out_seconds: f64) {
        self.fade_out_seconds = fade_out_seconds;
        self.is_triggered_fade_out = true;
    }

    /// `startFadeOut(fadeOutSeconds, userTimeSeconds)`: the end time can only move earlier.
    pub fn start_fade_out(&mut self, fade_out_seconds: f64, user_time_seconds: f64) {
        let new_end_time_seconds = user_time_seconds + fade_out_seconds;
        self.is_triggered_fade_out = true;
        if self.end_time_seconds < 0.0 || new_end_time_seconds < self.end_time_seconds {
            self.end_time_seconds = new_end_time_seconds;
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }
    pub fn is_started(&self) -> bool {
        self.started
    }
    pub fn start_time(&self) -> f64 {
        self.start_time_seconds
    }
    pub fn fade_in_start_time(&self) -> f64 {
        self.fade_in_start_time_seconds
    }
    pub fn end_time(&self) -> f64 {
        self.end_time_seconds
    }
    pub fn set_start_time(&mut self, start_time: f64) {
        self.start_time_seconds = start_time;
    }
    pub fn set_fade_in_start_time(&mut self, start_time: f64) {
        self.fade_in_start_time_seconds = start_time;
    }
    pub fn set_end_time(&mut self, end_time: f64) {
        self.end_time_seconds = end_time;
    }
    pub fn set_is_finished(&mut self, f: bool) {
        self.finished = f;
    }
    pub fn set_is_started(&mut self, f: bool) {
        self.started = f;
    }
    pub fn is_available(&self) -> bool {
        self.available
    }
    pub fn set_is_available(&mut self, v: bool) {
        self.available = v;
    }
    /// `setState(timeSeconds, weight)`.
    pub fn set_state(&mut self, time_seconds: f64, weight: f64) {
        self.state_time_seconds = time_seconds;
        self.state_weight = weight;
    }
    pub fn state_time(&self) -> f64 {
        self.state_time_seconds
    }
    pub fn state_weight(&self) -> f64 {
        self.state_weight
    }
    pub fn last_check_event_seconds(&self) -> f64 {
        self.last_event_check_seconds
    }
    pub fn set_last_check_event_seconds(&mut self, check_seconds: f64) {
        self.last_event_check_seconds = check_seconds;
    }
    pub fn is_triggered_fade_out(&self) -> bool {
        self.is_triggered_fade_out
    }
    pub fn fade_out_seconds(&self) -> f64 {
        self.fade_out_seconds
    }
    /// `getCubismMotion()`.
    pub fn cubism_motion(&self) -> &MotionRef {
        &self.motion
    }
    /// The entry's handle.
    pub fn handle(&self) -> CubismMotionQueueEntryHandle {
        self.handle
    }
}
