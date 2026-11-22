//! # amadeus-core
//!
//! Platform-agnostic logic of RealAmadeus V2. Compiles for `wasm32-unknown-unknown`
//! (frontend) and native targets (server / desktop). This crate performs **no I/O**:
//! persistence goes through [`storage::KvStore`] and networking lives in
//! `amadeus-backend` (native) or `amadeus-ui` (browser).

pub mod achievements;
pub mod changelog;
pub mod chat_utils;
pub mod i18n;
pub mod jscompat;
pub mod memory;
pub mod minigame;
pub mod persona;
pub mod protocol;
pub mod providers;
pub mod random_events;
pub mod release;
pub mod settings;
pub mod storage;
pub mod types;
pub mod vector;
pub mod version;
