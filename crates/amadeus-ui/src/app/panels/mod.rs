//! Overlay panels of the app screen (V1 `RealAmadeusApp.tsx` 1997–3060). OWNED BY: panels
//! engineer.
//!
//! Desktop: [`language_select::LanguageSelectPanel`], [`login::LoginPanel`],
//! [`loading::LoadingPanel`], [`status::StatusPanel`], [`changelog::ChangeLogPanel`],
//! [`help::HelpPanel`], [`confirm::ConfirmationDialog`].
//! Both layouts: [`backlog::BackLogPanel`], [`achievements::AchievementPanel`],
//! [`minigame::MiniGamePanel`] (inside ChatPanel). Shared overlay helpers: [`common`].

pub mod achievements;
pub mod backlog;
pub mod changelog;
pub mod common;
pub mod confirm;
pub mod help;
pub mod language_select;
pub mod loading;
pub mod login;
pub mod minigame;
pub mod status;
