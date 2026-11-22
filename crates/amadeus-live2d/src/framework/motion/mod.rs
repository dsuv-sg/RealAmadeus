//! Motion and expression system (`Framework/src/motion/*`, spec §14–§15).
//!
//! * [`internal`] — motion data + curve evaluation (`evaluate_curve`, linear / bezier (restricted and
//!   Cardano) / stepped / inverse stepped, loop end-point correction).
//! * [`acubism_motion`] — [`ACubismMotion`] trait (fades, weight, loop, callbacks) and [`MotionRef`].
//! * [`cubism_motion`] — [`CubismMotion`] (motion3.json).
//! * [`queue_entry`], [`queue_manager`], [`motion_manager`] — playback queue with priorities.
//! * [`expression_motion`], [`expression_manager`] — exp3.json expressions and their blending.

pub mod acubism_motion;
pub mod cubism_motion;
pub mod expression_manager;
pub mod expression_motion;
pub mod internal;
pub mod motion_manager;
pub mod queue_entry;
pub mod queue_manager;

pub use acubism_motion::{motion_ref, ACubismMotion, MotionBase, MotionCallback, MotionRef};
pub use cubism_motion::{CubismMotion, MotionBehavior};
pub use expression_manager::CubismExpressionMotionManager;
pub use expression_motion::{
    CubismExpressionMotion, ExpressionBlendType, ExpressionParameter, ExpressionParameterValue,
};
pub use internal::{
    evaluate_curve, CubismMotionCurveTarget, CubismMotionData, CubismMotionSegmentType,
};
pub use motion_manager::CubismMotionManager;
pub use queue_entry::{CubismMotionQueueEntry, CubismMotionQueueEntryHandle};
pub use queue_manager::{CubismMotionEventFunction, CubismMotionQueueManager};

/// `LAppDefine.PriorityNone`.
pub const PRIORITY_NONE: i32 = 0;
/// `LAppDefine.PriorityIdle`.
pub const PRIORITY_IDLE: i32 = 1;
/// `LAppDefine.PriorityNormal`.
pub const PRIORITY_NORMAL: i32 = 2;
/// `LAppDefine.PriorityForce`.
pub const PRIORITY_FORCE: i32 = 3;
