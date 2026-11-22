//! `CubismMotionQueueManager` — port of `Framework/src/motion/cubismmotionqueuemanager.ts`
//! (spec §14.8).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use std::fmt;

use super::acubism_motion::MotionRef;
use super::queue_entry::{CubismMotionQueueEntry, CubismMotionQueueEntryHandle};
use crate::framework::parameter_store::ParameterStore;

/// User-data event callback (`(caller, eventValue, customData) => void`; custom data is captured).
pub type CubismMotionEventFunction = Box<dyn FnMut(&str)>;

/// Plays queued motions (`CubismMotionQueueManager`).
#[derive(Default)]
pub struct CubismMotionQueueManager {
    pub(crate) user_time_seconds: f64,
    event_callback: Option<CubismMotionEventFunction>,
    pub(crate) motions: Vec<CubismMotionQueueEntry>,
}

impl fmt::Debug for CubismMotionQueueManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CubismMotionQueueManager")
            .field("user_time_seconds", &self.user_time_seconds)
            .field("motions", &self.motions)
            .finish_non_exhaustive()
    }
}

impl CubismMotionQueueManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Accumulated user time (`_userTimeSeconds`).
    pub fn user_time_seconds(&self) -> f64 {
        self.user_time_seconds
    }

    /// `release()`: releases every entry.
    pub fn release(&mut self) {
        for entry in &mut self.motions {
            entry.release();
        }
        self.motions.clear();
    }

    /// `startMotion(motion, autoDelete)`: every playing entry is armed to fade out with its motion's
    /// fade-out time; the new entry is appended (newest last).
    pub fn start_motion(
        &mut self,
        motion: MotionRef,
        auto_delete: bool,
    ) -> CubismMotionQueueEntryHandle {
        for entry in &mut self.motions {
            let fade_out = entry.motion.try_borrow().map_or(0.0, |m| m.fade_out_time());
            entry.set_fade_out(fade_out);
        }
        let entry = CubismMotionQueueEntry::new(motion, auto_delete);
        let handle = entry.handle();
        self.motions.push(entry);
        handle
    }

    /// `isFinished()`: true when no entry is still playing (also when empty).
    pub fn is_finished(&self) -> bool {
        self.motions.iter().all(CubismMotionQueueEntry::is_finished)
    }

    /// `isFinishedByHandle(handle)`.
    pub fn is_finished_by_handle(&self, handle: CubismMotionQueueEntryHandle) -> bool {
        !self
            .motions
            .iter()
            .any(|e| e.handle() == handle && !e.is_finished())
    }

    /// `stopAllMotions()` — QUIRK kept: splicing while incrementing removes only every other entry
    /// (`[A, B, C]` → `[B]`).
    pub fn stop_all_motions(&mut self) {
        let mut i = 0;
        while i < self.motions.len() {
            self.motions[i].release();
            self.motions.remove(i);
            i += 1;
        }
    }

    /// `getCubismMotionQueueEntries()`.
    pub fn entries(&self) -> &[CubismMotionQueueEntry] {
        &self.motions
    }

    /// Mutable entries.
    pub fn entries_mut(&mut self) -> &mut Vec<CubismMotionQueueEntry> {
        &mut self.motions
    }

    /// `getCubismMotionQueueEntry(handle)`.
    pub fn entry(&self, handle: CubismMotionQueueEntryHandle) -> Option<&CubismMotionQueueEntry> {
        self.motions.iter().find(|e| e.handle() == handle)
    }

    /// `getCubismMotionQueueEntry(handle)` (mutable).
    pub fn entry_mut(
        &mut self,
        handle: CubismMotionQueueEntryHandle,
    ) -> Option<&mut CubismMotionQueueEntry> {
        self.motions.iter_mut().find(|e| e.handle() == handle)
    }

    /// `setEventCallback(callback, customData)`.
    pub fn set_event_callback(&mut self, callback: Option<CubismMotionEventFunction>) {
        self.event_callback = callback;
    }

    /// `doUpdateMotion(model, userTimeSeconds)`: updates every entry oldest first, fires user-data
    /// events, removes finished entries and applies armed fade-outs. Returns whether any entry ran.
    pub fn do_update_motion(
        &mut self,
        model: &mut dyn ParameterStore,
        user_time_seconds: f64,
    ) -> bool {
        let mut updated = false;
        let mut i = 0;
        while i < self.motions.len() {
            let motion = self.motions[i].motion.clone();
            let fired = {
                let Ok(mut m) = motion.try_borrow_mut() else {
                    i += 1;
                    continue;
                };
                let entry = &mut self.motions[i];
                m.update_parameters(model, entry, user_time_seconds);
                updated = true;
                m.fired_event(
                    entry.last_check_event_seconds() - entry.start_time(),
                    user_time_seconds - entry.start_time(),
                )
            };
            if let Some(callback) = self.event_callback.as_mut() {
                for value in &fired {
                    callback(value);
                }
            }
            let entry = &mut self.motions[i];
            entry.set_last_check_event_seconds(user_time_seconds);
            if entry.is_finished() {
                entry.release();
                self.motions.remove(i);
            } else {
                if entry.is_triggered_fade_out() {
                    let fade_out_seconds = entry.fade_out_seconds();
                    entry.start_fade_out(fade_out_seconds, user_time_seconds);
                }
                i += 1;
            }
        }
        updated
    }
}
