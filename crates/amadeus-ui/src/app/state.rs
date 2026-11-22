//! Shared UI state types of the app screen (V1 `RealAmadeusApp.tsx` types).
//!
//! OWNED BY: app-root engineer. Other UI engineers import these; additive changes only.

use std::rc::Rc;

use amadeus_core::protocol::ChatMessage;

/// Screen state machine (`login` → `boot` → `chat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Login,
    Boot,
    Chat,
}

/// V1 `OverlayPanel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayPanel {
    #[default]
    None,
    Backlog,
    Config,
    Status,
    ChangeLog,
    Help,
    ConfirmLogout,
    ConfirmShutdown,
    Minigame,
    Achievements,
}

/// V1 `BackLogEntry` (`{ role, nameColor, message }`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackLogEntry {
    pub role: String,
    pub name_color: String,
    pub message: String,
}

/// V1 `ConfigIoResult`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConfigIoResult {
    pub success: bool,
    pub message: String,
}

/// V1 `ConversationBridge`: filled by ChatPanel on mount so the root (config export/import)
/// can read and replace the conversation.
#[derive(Clone)]
pub struct ConversationBridge {
    pub get_history: Rc<dyn Fn() -> Vec<ChatMessage>>,
    pub get_turn_count: Rc<dyn Fn() -> f64>,
    pub apply_imported: Rc<dyn Fn(Vec<ChatMessage>, f64)>,
}

/// Window scaling (`{ width, height, scale }`, base 1920×1080).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dimensions {
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

impl Default for Dimensions {
    fn default() -> Self {
        Self {
            width: 1920.0,
            height: 1080.0,
            scale: 1.0,
        }
    }
}

impl From<amadeus_core::types::BackLogEntry> for BackLogEntry {
    /// Core `BackLogEntry::from_raw` / `backlog_entries_for_import` → UI entry (same fields).
    fn from(entry: amadeus_core::types::BackLogEntry) -> Self {
        Self {
            role: entry.role,
            name_color: entry.name_color,
            message: entry.message,
        }
    }
}

impl From<amadeus_core::types::ConfigIoResult> for ConfigIoResult {
    fn from(result: amadeus_core::types::ConfigIoResult) -> Self {
        Self {
            success: result.success,
            message: result.message,
        }
    }
}
