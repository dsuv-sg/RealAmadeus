//! Cubism Web Framework 5-r.5 (model/physics/effects/motion layer) ported to Rust
//! (spec `docs/specs/spec-live2d-framework-model.md`).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! Everything here is pure Rust (no JS types) except where it touches [`crate::core::model`]; all
//! algorithms talk to the model through [`ParameterStore`], implemented by
//! [`crate::core::model::CubismModel`] (Core-backed) and by [`ModelState`] (pure; native tests).
//!
//! | Module | Port of |
//! |---|---|
//! | [`model_setting`] | `CubismModelSettingJson` (model3.json) |
//! | [`default_parameter_id`] | `CubismDefaultParameterId` |
//! | [`physics`] | `CubismPhysics` (+ json, internal) |
//! | [`effect`] | `CubismBreath`, `CubismEyeBlink`, `CubismPose`, `CubismLook` |
//! | [`motion`] | `CubismMotion`, queue entry/manager, `CubismMotionManager`, expressions |
//! | [`update`] | `CubismUpdateScheduler` + the seven updaters (V1 orders 200…800) |
//! | [`user_model`] | `CubismUserModel` (framework subset) |
//! | [`user_data`] | `CubismModelUserData` |
//! | [`multiply_screen_color`], [`parts_hierarchy`] | colour overrides, parts hierarchy |
//! | [`model_state`], [`parameter_store`] | the parameter/part semantics of `CubismModel` |
//! | [`json`] | `CubismJson` value semantics on `serde_json` |
//!
//! V1 frame order (spec §1.2): `load_parameters` → (motion) → `save_parameters` →
//! `scheduler.on_late_update` (Look 400 → Breath 500 → Physics 600 for the Kurisu model) →
//! `CubismModel::update()`; then the player driver writes its parameters and calls `update()` again.

pub mod default_parameter_id;
pub mod effect;
pub mod json;
pub mod model_setting;
pub mod model_state;
pub mod motion;
pub mod multiply_screen_color;
pub mod parameter_store;
pub mod parts_hierarchy;
pub mod physics;
pub mod random;
pub mod update;
pub mod user_data;
pub mod user_model;

#[cfg(test)]
mod golden_tests;

pub use model_setting::{CubismModelSettingJson, ModelSetting};
pub use model_state::ModelState;
pub use parameter_store::ParameterStore;
pub use physics::CubismPhysics;
pub use random::js_random;
pub use user_model::CubismUserModel;

use crate::core::ffi;
use crate::core::model::Live2DError;

/// `CubismFramework.startUp()` + `initialize()` for V2 (spec §4.1): requires a ready Core (see
/// `app::core_loader`), registers the Core log handler once per page and returns the Core version
/// (`Live2D Cubism Core version: 06.00.0001 (100663297)` is logged at debug level). Idempotent;
/// there is no global dispose (the Core lives for the page).
pub fn start_up() -> Result<ffi::CoreVersion, Live2DError> {
    let version = ffi::core_version().ok_or(Live2DError::CoreNotLoaded)?;
    ffi::register_core_log_handler();
    log::debug!(target: "cubism", "Live2D Cubism Core version: {version}");
    Ok(version)
}

thread_local! {
    static FRAMEWORK_REF_COUNT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// V1 `acquireFramework()` (pipeline spec §4.4): ref-counted [`start_up`] shared by every player
/// on the page. Fails (without counting) while the Core is not ready.
pub fn acquire() -> Result<ffi::CoreVersion, Live2DError> {
    let version = start_up()?;
    FRAMEWORK_REF_COUNT.with(|c| c.set(c.get().saturating_add(1)));
    Ok(version)
}

/// V1 `releaseFramework()`: decrements the count (never below 0). Reaching 0 disposes nothing in
/// V2: the Core, its one-shot log handler and the ID strings live for the page, and shader programs
/// belong to (and are freed with) each renderer — so a later [`acquire`] (remount) is safe, unlike
/// V1's second `csmSetLogFunction`.
pub fn release() {
    FRAMEWORK_REF_COUNT.with(|c| c.set(c.get().saturating_sub(1)));
}

/// Current [`acquire`] count (0 = no player holds the framework).
pub fn acquired_count() -> u32 {
    FRAMEWORK_REF_COUNT.with(std::cell::Cell::get)
}
