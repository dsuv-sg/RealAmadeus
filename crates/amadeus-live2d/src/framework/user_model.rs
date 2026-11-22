//! `CubismUserModel` — the Framework's base class of `LAppModel` (`cubismusermodel.ts`, spec §10.1),
//! without the renderer and the model matrix (owned by the render/app layers: create the matrix with
//! `CubismModelMatrix::new(model.canvas_width_f64(), model.canvas_height_f64())` after
//! [`CubismUserModel::load_model`]).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! Components that updaters drive are `Rc<RefCell<…>>` so the app layer can register them with a
//! [`super::update::CubismUpdateScheduler`] and still reach them (e.g. `drag_manager` for dragging,
//! `expression_manager` for `set_expression`). The V1 frame order (spec §1.2) is:
//!
//! ```ignore
//! model.load_parameters();
//! let motion_updated = if motion_manager.is_finished() { /* start idle motion */ false }
//!                      else { motion_manager.update_motion(&mut model, dt) };
//! model.save_parameters();
//! scheduler.on_late_update(&mut model, dt);   // EyeBlink 200, Expression 300, Look 400, Breath 500,
//!                                             // Physics 600, LipSync 700, Pose 800
//! model.update();                             // Core update
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use super::effect::{CubismBreath, CubismEyeBlink, CubismPose};
use super::model_setting::ModelSetting;
use super::motion::{
    ACubismMotion, CubismExpressionMotion, CubismExpressionMotionManager, CubismMotion,
    CubismMotionManager, MotionCallback,
};
use super::physics::CubismPhysics;
use super::user_data::CubismModelUserData;
use crate::core::model::{CubismMoc, CubismModel, Live2DError};
use crate::math::target_point::CubismTargetPoint;

/// Framework part of a user model (`CubismUserModel`).
pub struct CubismUserModel {
    pub moc: Option<CubismMoc>,
    pub model: Option<CubismModel>,
    pub motion_manager: CubismMotionManager,
    pub expression_manager: Rc<RefCell<CubismExpressionMotionManager>>,
    pub drag_manager: Rc<RefCell<CubismTargetPoint>>,
    pub eye_blink: Option<Rc<RefCell<CubismEyeBlink>>>,
    pub breath: Option<Rc<RefCell<CubismBreath>>>,
    pub pose: Option<Rc<RefCell<CubismPose>>>,
    pub physics: Option<Rc<RefCell<CubismPhysics>>>,
    pub model_user_data: Option<CubismModelUserData>,
    pub initialized: bool,
    pub updating: bool,
    pub opacity: f64,
    pub moc_consistency: bool,
    pub motion_consistency: bool,
    pub debug_mode: bool,
}

impl Default for CubismUserModel {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CubismUserModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CubismUserModel")
            .field("model", &self.model)
            .field("initialized", &self.initialized)
            .field("updating", &self.updating)
            .finish_non_exhaustive()
    }
}

impl CubismUserModel {
    /// `new CubismUserModel()`: motion events go to [`CubismUserModel::motion_event_fired`]'s log.
    pub fn new() -> Self {
        let mut motion_manager = CubismMotionManager::new();
        motion_manager.set_event_callback(Some(Box::new(|value: &str| {
            Self::motion_event_fired(value)
        })));
        Self {
            moc: None,
            model: None,
            motion_manager,
            expression_manager: Rc::new(RefCell::new(CubismExpressionMotionManager::new())),
            drag_manager: Rc::new(RefCell::new(CubismTargetPoint::new())),
            eye_blink: None,
            breath: None,
            pose: None,
            physics: None,
            model_user_data: None,
            initialized: false,
            updating: false,
            opacity: 1.0,
            moc_consistency: false,
            motion_consistency: false,
            debug_mode: false,
        }
    }

    /// `motionEventFired(eventValue)`: `CubismLogInfo('{0}', value)`.
    pub fn motion_event_fired(event_value: &str) {
        log::info!(target: "cubism", "{event_value}");
    }

    /// `loadModel(buffer, shouldCheckMocConsistency)`: moc + model, then `saveParameters()`.
    pub fn load_model(
        &mut self,
        bytes: &[u8],
        should_check_moc_consistency: bool,
    ) -> Result<(), Live2DError> {
        let moc = CubismMoc::from_bytes(bytes, should_check_moc_consistency).inspect_err(|_| {
            log::error!(target: "cubism", "Failed to CubismMoc.create().");
        })?;
        let mut model = moc.create_model().inspect_err(|_| {
            log::error!(target: "cubism", "Failed to CreateModel().");
        })?;
        model.save_parameters();
        self.moc = Some(moc);
        self.model = Some(model);
        Ok(())
    }

    /// `loadMotion(buffer, size, name, onFinished, onBegan, modelSetting, group, index, check)`:
    /// model3.json `FadeInTime`/`FadeOutTime` override the motion's when `>= 0`.
    #[allow(clippy::too_many_arguments)]
    pub fn load_motion(
        &self,
        bytes: &[u8],
        on_finished_motion: Option<MotionCallback>,
        on_began_motion: Option<MotionCallback>,
        model_setting: Option<&ModelSetting>,
        group: &str,
        index: usize,
        should_check_motion_consistency: bool,
    ) -> Result<CubismMotion, Live2DError> {
        if bytes.is_empty() {
            log::error!(target: "cubism", "Failed to loadMotion().");
            return Err(Live2DError::Load("Failed to loadMotion().".to_owned()));
        }
        let mut motion = CubismMotion::create(
            bytes,
            on_finished_motion,
            on_began_motion,
            should_check_motion_consistency,
        )
        .inspect_err(|_| {
            log::error!(target: "cubism", "Failed to create motion from buffer in LoadMotion()");
        })?;
        if let Some(setting) = model_setting {
            let fade_in_time = setting.motion_fade_in_time_value(group, index);
            if fade_in_time >= 0.0 {
                motion.set_fade_in_time(fade_in_time);
            }
            let fade_out_time = setting.motion_fade_out_time_value(group, index);
            if fade_out_time >= 0.0 {
                motion.set_fade_out_time(fade_out_time);
            }
        }
        Ok(motion)
    }

    /// `loadExpression(buffer, size, name)`.
    pub fn load_expression(&self, bytes: &[u8]) -> Result<CubismExpressionMotion, Live2DError> {
        if bytes.is_empty() {
            log::error!(target: "cubism", "Failed to loadExpression().");
            return Err(Live2DError::Load("Failed to loadExpression().".to_owned()));
        }
        CubismExpressionMotion::create(bytes)
    }

    /// `loadPose(buffer, size)`.
    pub fn load_pose(&mut self, bytes: &[u8]) -> Result<(), Live2DError> {
        if bytes.is_empty() {
            log::error!(target: "cubism", "Failed to loadPose().");
            return Err(Live2DError::Load("Failed to loadPose().".to_owned()));
        }
        self.pose = Some(Rc::new(RefCell::new(CubismPose::create(bytes)?)));
        Ok(())
    }

    /// `loadUserData(buffer, size)`.
    pub fn load_user_data(&mut self, bytes: &[u8]) -> Result<(), Live2DError> {
        if bytes.is_empty() {
            log::error!(target: "cubism", "Failed to loadUserData().");
            return Err(Live2DError::Load("Failed to loadUserData().".to_owned()));
        }
        self.model_user_data = Some(CubismModelUserData::create(bytes)?);
        Ok(())
    }

    /// `loadPhysics(buffer, size)`.
    pub fn load_physics(&mut self, bytes: &[u8]) -> Result<(), Live2DError> {
        if bytes.is_empty() {
            log::error!(target: "cubism", "Failed to loadPhysics().");
            return Err(Live2DError::Load("Failed to loadPhysics().".to_owned()));
        }
        self.physics = Some(Rc::new(RefCell::new(CubismPhysics::create(bytes)?)));
        Ok(())
    }

    /// `setDragging(x, y)`.
    pub fn set_dragging(&self, x: f64, y: f64) {
        self.drag_manager.borrow_mut().set(x, y);
    }

    /// `isHit(drawableId, x, y)` with the point already in model space (the caller applies
    /// `modelMatrix.invertTransformX/Y`): bounding box of the drawable's vertices.
    pub fn is_hit(&self, drawable_id: &str, model_x: f64, model_y: f64) -> bool {
        let Some(model) = self.model.as_ref() else {
            return false;
        };
        let Some(index) = model.drawable_index(drawable_id) else {
            return false;
        };
        let count = model.drawable_vertex_count(index);
        let vertices = model.drawable_vertex_positions_vec(index);
        is_hit_vertices(&vertices, count, model_x, model_y)
    }

    /// `release()`: drops the model (Core release), the moc and every component.
    pub fn release(&mut self) {
        self.motion_manager.release();
        self.expression_manager.borrow_mut().release();
        if let Some(mut model) = self.model.take() {
            model.release();
        }
        if let Some(mut moc) = self.moc.take() {
            moc.release();
        }
        self.pose = None;
        self.eye_blink = None;
        self.breath = None;
        self.physics = None;
        self.model_user_data = None;
    }
}

/// Bounding-box hit test over `count` xy vertices (`CubismUserModel.isHit` inner loop).
pub fn is_hit_vertices(vertices: &[f32], count: usize, tx: f64, ty: f64) -> bool {
    if count == 0 || vertices.len() < 2 {
        return false;
    }
    let mut left = f64::from(vertices[0]);
    let mut right = left;
    let mut top = f64::from(vertices[1]);
    let mut bottom = top;
    for j in 1..count {
        let (Some(x), Some(y)) = (vertices.get(j * 2), vertices.get(j * 2 + 1)) else {
            break;
        };
        let (x, y) = (f64::from(*x), f64::from(*y));
        if x < left {
            left = x;
        }
        if x > right {
            right = x;
        }
        if y < top {
            top = y;
        }
        if y > bottom {
            bottom = y;
        }
    }
    left <= tx && tx <= right && top <= ty && ty <= bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hit_box() {
        let v = [0.0f32, 0.0, 1.0, 0.0, 1.0, 2.0, 0.0, 2.0];
        assert!(is_hit_vertices(&v, 4, 0.5, 1.0));
        assert!(!is_hit_vertices(&v, 4, 1.5, 1.0));
        assert!(!is_hit_vertices(&v, 0, 0.0, 0.0));
    }

    #[test]
    fn load_motion_fade_overrides() {
        let user = CubismUserModel::new();
        let setting = ModelSetting::from_bytes(
            br#"{"FileReferences":{"Motions":{"Idle":[{"File":"a.motion3.json","FadeInTime":0.25}]}}}"#,
        )
        .unwrap_or_else(|_| ModelSetting::from_value(serde_json::Value::Null));
        let src = br#"{"Meta":{"Duration":1,"Fps":30,"CurveCount":0,"FadeOutTime":0.5}}"#;
        let m = user.load_motion(src, None, None, Some(&setting), "Idle", 0, false);
        let m = m.unwrap_or_default();
        assert_eq!(m.fade_in_time(), 0.25);
        assert_eq!(m.fade_out_time(), 0.5);
        assert!(user
            .load_motion(b"", None, None, None, "Idle", 0, false)
            .is_err());
    }
}
