//! The concrete updaters (`cubism{EyeBlink,Expression,Look,Breath,Physics,Pose,LipSync}Updater.ts`).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).

use std::cell::RefCell;
use std::rc::Rc;

use super::{order, CubismUpdater};
use crate::framework::effect::{CubismBreath, CubismEyeBlink, CubismLook, CubismPose};
use crate::framework::motion::CubismExpressionMotionManager;
use crate::framework::parameter_store::ParameterStore;
use crate::framework::physics::CubismPhysics;
use crate::math::target_point::CubismTargetPoint;

macro_rules! impl_order {
    () => {
        fn execution_order(&self) -> i64 {
            self.execution_order
        }
        fn set_execution_order(&mut self, execution_order: i64) {
            self.execution_order = execution_order;
        }
    };
}

/// `CubismBreathUpdater` (order 500).
pub struct CubismBreathUpdater {
    breath: Rc<RefCell<CubismBreath>>,
    execution_order: i64,
}

impl CubismBreathUpdater {
    /// `new CubismBreathUpdater(breath, executionOrder?)`.
    pub fn new(breath: Rc<RefCell<CubismBreath>>, execution_order: Option<i64>) -> Self {
        Self {
            breath,
            execution_order: execution_order.unwrap_or(order::BREATH),
        }
    }
}

impl CubismUpdater for CubismBreathUpdater {
    impl_order!();
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        self.breath
            .borrow_mut()
            .update_parameters(model, delta_time_seconds);
    }
}

/// `CubismLookUpdater` (order 400): advances the drag target point, then applies the look.
pub struct CubismLookUpdater {
    look: Rc<RefCell<CubismLook>>,
    drag_manager: Rc<RefCell<CubismTargetPoint>>,
    execution_order: i64,
}

impl CubismLookUpdater {
    /// `new CubismLookUpdater(look, dragManager, executionOrder?)`.
    pub fn new(
        look: Rc<RefCell<CubismLook>>,
        drag_manager: Rc<RefCell<CubismTargetPoint>>,
        execution_order: Option<i64>,
    ) -> Self {
        Self {
            look,
            drag_manager,
            execution_order: execution_order.unwrap_or(order::DRAG),
        }
    }
}

impl CubismUpdater for CubismLookUpdater {
    impl_order!();
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        let (drag_x, drag_y) = {
            let mut drag = self.drag_manager.borrow_mut();
            drag.update(delta_time_seconds);
            (drag.get_x(), drag.get_y())
        };
        self.look.borrow().update_parameters(model, drag_x, drag_y);
    }
}

/// `CubismEyeBlinkUpdater` (order 200): blinks only while no main motion updated this frame.
pub struct CubismEyeBlinkUpdater {
    motion_updated: Box<dyn Fn() -> bool>,
    eye_blink: Rc<RefCell<CubismEyeBlink>>,
    execution_order: i64,
}

impl CubismEyeBlinkUpdater {
    /// `new CubismEyeBlinkUpdater(motionUpdated, eyeBlink, executionOrder?)`.
    pub fn new(
        motion_updated: Box<dyn Fn() -> bool>,
        eye_blink: Rc<RefCell<CubismEyeBlink>>,
        execution_order: Option<i64>,
    ) -> Self {
        Self {
            motion_updated,
            eye_blink,
            execution_order: execution_order.unwrap_or(order::EYE_BLINK),
        }
    }
}

impl CubismUpdater for CubismEyeBlinkUpdater {
    impl_order!();
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        if !(self.motion_updated)() {
            self.eye_blink
                .borrow_mut()
                .update_parameters(model, delta_time_seconds);
        }
    }
}

/// `CubismExpressionUpdater` (order 300).
pub struct CubismExpressionUpdater {
    expression_manager: Rc<RefCell<CubismExpressionMotionManager>>,
    execution_order: i64,
}

impl CubismExpressionUpdater {
    /// `new CubismExpressionUpdater(expressionManager, executionOrder?)`.
    pub fn new(
        expression_manager: Rc<RefCell<CubismExpressionMotionManager>>,
        execution_order: Option<i64>,
    ) -> Self {
        Self {
            expression_manager,
            execution_order: execution_order.unwrap_or(order::EXPRESSION),
        }
    }
}

impl CubismUpdater for CubismExpressionUpdater {
    impl_order!();
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        self.expression_manager
            .borrow_mut()
            .update_motion(model, delta_time_seconds);
    }
}

/// `CubismPhysicsUpdater` (order 600).
pub struct CubismPhysicsUpdater {
    physics: Rc<RefCell<CubismPhysics>>,
    execution_order: i64,
}

impl CubismPhysicsUpdater {
    /// `new CubismPhysicsUpdater(physics, executionOrder?)`.
    pub fn new(physics: Rc<RefCell<CubismPhysics>>, execution_order: Option<i64>) -> Self {
        Self {
            physics,
            execution_order: execution_order.unwrap_or(order::PHYSICS),
        }
    }
}

impl CubismUpdater for CubismPhysicsUpdater {
    impl_order!();
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        self.physics
            .borrow_mut()
            .evaluate(model, delta_time_seconds);
    }
}

/// `CubismPoseUpdater` (order 800).
pub struct CubismPoseUpdater {
    pose: Rc<RefCell<CubismPose>>,
    execution_order: i64,
}

impl CubismPoseUpdater {
    /// `new CubismPoseUpdater(pose, executionOrder?)`.
    pub fn new(pose: Rc<RefCell<CubismPose>>, execution_order: Option<i64>) -> Self {
        Self {
            pose,
            execution_order: execution_order.unwrap_or(order::POSE),
        }
    }
}

impl CubismUpdater for CubismPoseUpdater {
    impl_order!();
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        self.pose
            .borrow_mut()
            .update_parameters(model, delta_time_seconds);
    }
}

/// `IParameterProvider` — a lip-sync value source (V1's implementation was the unused
/// `LAppWavFileHandler`).
pub trait IParameterProvider {
    /// `update(deltaTimeSeconds)`: returns whether a new value is available.
    fn update(&mut self, delta_time_seconds: f64) -> bool;
    /// `getParameter()`.
    fn parameter(&self) -> f64;
}

/// `CubismLipSyncUpdater` (order 700): adds the provider's value to every lip-sync parameter.
pub struct CubismLipSyncUpdater {
    lip_sync_ids: Vec<String>,
    audio_provider: Option<Rc<RefCell<dyn IParameterProvider>>>,
    execution_order: i64,
}

impl CubismLipSyncUpdater {
    /// `new CubismLipSyncUpdater(lipSyncIds, audioProvider, executionOrder?)`.
    pub fn new(
        lip_sync_ids: Vec<String>,
        audio_provider: Option<Rc<RefCell<dyn IParameterProvider>>>,
        execution_order: Option<i64>,
    ) -> Self {
        Self {
            lip_sync_ids,
            audio_provider,
            execution_order: execution_order.unwrap_or(order::LIP_SYNC),
        }
    }

    /// `setAudioProvider(provider)`.
    pub fn set_audio_provider(
        &mut self,
        audio_provider: Option<Rc<RefCell<dyn IParameterProvider>>>,
    ) {
        self.audio_provider = audio_provider;
    }

    /// `getAudioProvider()`.
    pub fn audio_provider(&self) -> Option<Rc<RefCell<dyn IParameterProvider>>> {
        self.audio_provider.clone()
    }
}

impl CubismUpdater for CubismLipSyncUpdater {
    impl_order!();
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        let Some(provider) = self.audio_provider.as_ref() else {
            return;
        };
        let value = {
            let mut provider = provider.borrow_mut();
            if !provider.update(delta_time_seconds) {
                return;
            }
            provider.parameter()
        };
        for id in &self.lip_sync_ids {
            model.add_parameter_value_by_id_f64(id, value, 1.0);
        }
    }
}

/// Callback of a [`CubismFnUpdater`].
pub type UpdaterCallback = Box<dyn FnMut(&mut dyn ParameterStore, f64)>;

/// An updater from a closure (for app-specific steps that must run inside the scheduler order).
pub struct CubismFnUpdater {
    callback: UpdaterCallback,
    execution_order: i64,
}

impl CubismFnUpdater {
    pub fn new(execution_order: i64, callback: UpdaterCallback) -> Self {
        Self {
            callback,
            execution_order,
        }
    }
}

impl CubismUpdater for CubismFnUpdater {
    impl_order!();
    fn on_late_update(&mut self, model: &mut dyn ParameterStore, delta_time_seconds: f64) {
        (self.callback)(model, delta_time_seconds);
    }
}
