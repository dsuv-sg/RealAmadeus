//! The model half of V1's per-frame pipeline (pipeline spec §7.6–§7.9, D8), generic over the model
//! so the exact same code runs on the Core-backed [`CubismModel`] in the browser and on the pure
//! [`ModelState`] in native golden tests.
//!
//! ```text
//! LAppModel.update()   (only in CompleteSetup):
//!     load_parameters → motions (idle start / update) → save_parameters
//!     → scheduler (Look 400 → Breath 500 → Physics 600, Clock-1 dt, uncapped) → Core update #1
//! driver (player, Clock 2)  → Core update #2       (the caller; see `app::manager`)
//! ```

use std::cell::Cell;

use crate::core::model::CubismModel;
use crate::framework::model_state::ModelState;
use crate::framework::motion::CubismMotionManager;
use crate::framework::parameter_store::ParameterStore;
use crate::framework::update::CubismUpdateScheduler;

/// What the frame pipeline needs from a model besides [`ParameterStore`].
pub trait PipelineModel: ParameterStore {
    /// `CubismModel.loadParameters()`.
    fn load_parameters(&mut self);
    /// `CubismModel.saveParameters()`.
    fn save_parameters(&mut self);
    /// `CubismModel.update()` (the Core update; `resetDynamicFlags` included).
    fn core_update(&mut self);
}

impl PipelineModel for CubismModel {
    fn load_parameters(&mut self) {
        CubismModel::load_parameters(self);
    }
    fn save_parameters(&mut self) {
        CubismModel::save_parameters(self);
    }
    fn core_update(&mut self) {
        self.update();
    }
}

impl PipelineModel for ModelState {
    fn load_parameters(&mut self) {
        ModelState::load_parameters(self);
    }
    fn save_parameters(&mut self) {
        ModelState::save_parameters(self);
    }
    fn core_update(&mut self) {
        self.emulate_core_update();
    }
}

/// `LAppModel.update()` for a model in `CompleteSetup` (spec §7.6):
///
/// ```js
/// this._model.loadParameters();
/// this._motionUpdated = false;
/// if (this._motionManager.isFinished()) this.startRandomMotion(MotionGroupIdle, PriorityIdle);
/// else this._motionUpdated = this._motionManager.updateMotion(this._model, dt);
/// this._model.saveParameters();
/// this._updateScheduler.onLateUpdate(this._model, dt);
/// this._model.update();
/// ```
///
/// `motion_updated` is the flag the eye-blink updater reads; `start_idle_motion` is
/// `startRandomMotion("Idle", PriorityIdle)` (returns immediately when the group is empty).
pub fn sdk_update(
    model: &mut dyn PipelineModel,
    motion_manager: &mut CubismMotionManager,
    scheduler: &mut CubismUpdateScheduler,
    motion_updated: &Cell<bool>,
    delta_time_seconds: f64,
    start_idle_motion: &mut dyn FnMut(&mut CubismMotionManager),
) {
    model.load_parameters();
    motion_updated.set(false);
    if motion_manager.is_finished() {
        start_idle_motion(motion_manager);
    } else {
        let updated = motion_manager.update_motion(&mut *model, delta_time_seconds);
        motion_updated.set(updated);
    }
    model.save_parameters();
    scheduler.on_late_update(&mut *model, delta_time_seconds);
    model.core_update();
}
