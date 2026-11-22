//! Sample-app layer equivalents and the public player API (pipeline spec
//! `docs/specs/spec-live2d-pipeline.md`; the deployed V1 bundle is the source of truth).
//!
//! | Module | V1 counterpart | Kind |
//! |---|---|---|
//! | [`player`] | `ReamaCubismPlayer` lifecycle (canvas, init retry, rAF loop, destroy) | web |
//! | [`driver`] | `ReamaCubismPlayer` state, setters, `applyParameters` | pure |
//! | [`emotion`], [`noise`] | `EMOTION_TARGETS`, `MOTION_BURSTS`, vowel table, drift/noise/idle | pure |
//! | [`clock`] | `LAppPal` (Clock 1, `Date.now()`) and the driver clock (Clock 2) | pure |
//! | [`pipeline`] | `LAppModel.update()` frame order, generic over the model | pure |
//! | [`projection`] | `LAppView.initialize`, `onUpdate` projection / upper-body zoom | pure |
//! | [`url`], [`load_step`], [`define`], [`jsnum`] | asset paths, `LoadStep`, `lappdefine`, JS number semantics | pure |
//! | [`subdelegate`] | `LAppSubdelegate` + `LAppGlManager` (canvas, DPR, resize, context loss) | web |
//! | [`manager`] | `LAppLive2DManager.onUpdate` | web |
//! | [`app_model`] | `LAppModel` load state machine, update/draw | web |
//! | [`texture_manager`] | `LAppTextureManager` | web |
//! | [`assets`], [`preload`] | `cachedFetch*`, `cachedLoadImage`, `preloadDefaultAssets` | web |
//! | [`core_loader`] | Core `<script>` + readiness polling (D7) | web |
//!
//! "Pure" modules have no JS calls and are covered by native unit tests (`cargo test --lib`,
//! including a bit-exact replay of V1's 600-frame pipeline trace); "web" modules compile everywhere
//! but only run in a browser.

pub mod app_model;
pub mod assets;
pub mod clock;
pub mod console;
pub mod core_loader;
pub mod define;
pub mod driver;
pub mod emotion;
pub mod jsnum;
pub mod load_step;
pub mod manager;
pub mod noise;
pub mod pipeline;
pub mod player;
pub mod preload;
pub mod projection;
pub mod subdelegate;
pub mod texture_manager;
pub mod url;
pub mod web;

#[cfg(test)]
mod golden_tests;
