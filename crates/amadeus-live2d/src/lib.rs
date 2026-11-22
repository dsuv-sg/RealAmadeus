//! # amadeus-live2d
//!
//! Rust port of the Live2D Cubism Web Framework (5-r.5) subset used by RealAmadeus, a WebGL
//! renderer, and the custom "ReamaCubism" player that V1 shipped as
//! `reama-cubism-runtime.js`. The proprietary Cubism Core (`live2dcubismcore.min.js`,
//! redistributable under the Live2D Proprietary Software License) is loaded at runtime and
//! accessed through wasm-bindgen.
//!
//! Layout:
//! * [`core`]      — bindings to the global `Live2DCubismCore` object + model wrapper.
//! * [`math`]      — CubismMatrix44 / ModelMatrix / ViewMatrix / Vector2 / CubismMath.
//! * [`framework`] — model setting, physics, breath, eye blink, pose, look, motion,
//!   expression, update scheduler, default parameter ids.
//! * [`render`]    — WebGL renderer, shaders (embedded), clipping masks, offscreen.
//! * [`app`]       — sample-app layer equivalents (LAppModel, LAppLive2DManager, view,
//!   subdelegate, texture loading) and the public [`Live2DPlayer`].
//!
//! Console output: the V1 `console.*` lines (`[ReamaCubism] …`, pipeline spec §11) are printed
//! directly ([`app::console`]). V1's `CubismLog*` calls, the Core's own log lines and V2-only
//! render notes for cases where V1 printed nothing go to `log` with `target: "cubism"`; V1 ran
//! the framework with `LogLevel_Off`, so the app's logger keeps that target silent by default.
//! Other `log` records (module-path target) are V1 `console.warn` sites or V2 diagnostics that
//! are documented V1 differences, and stay visible (the list is pinned by `log_channel_tests`).
//!
//! Public API used by `amadeus-ui`:
//! ```ignore
//! amadeus_live2d::ensure_core_loaded("resources/live2d/live2dcubismcore.min.js").await?;
//! let player = amadeus_live2d::Live2DPlayer::create(host_element, amadeus_live2d::PlayerOptions::default());
//! player.set_emotion("SMILE");
//! player.set_lip_sync(0.7);
//! player.set_spoken_char("あ");
//! player.set_eye_tracking(0.2, -0.1);
//! player.destroy();
//! ```

pub mod app;
pub mod core;
pub mod framework;
pub mod math;
pub mod render;

#[cfg(test)]
mod log_channel_tests;

pub use app::core_loader::ensure_core_loaded;
pub use app::player::{Live2DPlayer, PlayerOptions};
pub use app::preload::preload_default_assets;
