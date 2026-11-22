//! `CubismMotionManager` — priorities on top of the queue manager, port of
//! `Framework/src/motion/cubismmotionmanager.ts` (spec §14.9).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use std::ops::{Deref, DerefMut};

use super::acubism_motion::MotionRef;
use super::queue_entry::CubismMotionQueueEntryHandle;
use super::queue_manager::CubismMotionQueueManager;
use crate::framework::parameter_store::ParameterStore;

/// Motion manager with priorities (`CubismMotionManager extends CubismMotionQueueManager`; the
/// queue API is reachable through `Deref`).
#[derive(Debug, Default)]
pub struct CubismMotionManager {
    queue: CubismMotionQueueManager,
    current_priority: i32,
    reserve_priority: i32,
}

impl Deref for CubismMotionManager {
    type Target = CubismMotionQueueManager;
    fn deref(&self) -> &CubismMotionQueueManager {
        &self.queue
    }
}

impl DerefMut for CubismMotionManager {
    fn deref_mut(&mut self) -> &mut CubismMotionQueueManager {
        &mut self.queue
    }
}

impl CubismMotionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// `getCurrentPriority()`.
    pub fn current_priority(&self) -> i32 {
        self.current_priority
    }

    /// `getReservePriority()`.
    pub fn reserve_priority(&self) -> i32 {
        self.reserve_priority
    }

    /// `setReservePriority(val)`.
    pub fn set_reserve_priority(&mut self, val: i32) {
        self.reserve_priority = val;
    }

    /// `startMotionPriority(motion, autoDelete, priority)`.
    pub fn start_motion_priority(
        &mut self,
        motion: MotionRef,
        auto_delete: bool,
        priority: i32,
    ) -> CubismMotionQueueEntryHandle {
        if priority == self.reserve_priority {
            self.reserve_priority = 0;
        }
        self.current_priority = priority;
        self.queue.start_motion(motion, auto_delete)
    }

    /// `updateMotion(model, deltaTimeSeconds)`: returns whether a motion updated the model.
    pub fn update_motion(
        &mut self,
        model: &mut dyn ParameterStore,
        delta_time_seconds: f64,
    ) -> bool {
        self.queue.user_time_seconds += delta_time_seconds;
        let user_time = self.queue.user_time_seconds;
        let updated = self.queue.do_update_motion(model, user_time);
        if self.queue.is_finished() {
            self.current_priority = 0;
        }
        updated
    }

    /// `reserveMotion(priority)`: false when an equal or higher priority is playing/reserved.
    pub fn reserve_motion(&mut self, priority: i32) -> bool {
        if priority <= self.reserve_priority || priority <= self.current_priority {
            return false;
        }
        self.reserve_priority = priority;
        true
    }
}
