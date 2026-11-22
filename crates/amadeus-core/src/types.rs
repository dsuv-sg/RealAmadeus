//! Shared V1 types: `lib/amadeus/types.ts`, the local types of `RealAmadeusApp.tsx`
//! (`OverlayPanel`, `BackLogEntry`, `ConfigIoResult`, `ImportedMessage`, `toChatRole`), the
//! emotion tag set, and [`Emitter`], the port of V1's `Signal` observer (`lib/amadeus/signal.ts`).
//! Also the pure parts of the root's conversation export/import and the [`login`] rules.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::io;
use std::rc::Rc;

use serde::{Deserialize, Serialize};
use serde_json::ser::{Formatter, PrettyFormatter};
use serde_json::{Map, Value};

pub use crate::protocol::{ChatMessage, ChatRole};

/// Screen state machine (`AppState = "login" | "boot" | "chat"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppState {
    /// Login screen (initial state).
    #[default]
    Login,
    /// Boot/loading sequence.
    Boot,
    /// Main chat screen.
    Chat,
}

impl AppState {
    /// V1 string id.
    pub fn as_str(self) -> &'static str {
        match self {
            AppState::Login => "login",
            AppState::Boot => "boot",
            AppState::Chat => "chat",
        }
    }
}

/// Strict parse of a V1 role string (`"system" | "user" | "assistant"`, case-sensitive).
pub fn parse_chat_role(role: &str) -> Option<ChatRole> {
    match role {
        "system" => Some(ChatRole::System),
        "user" => Some(ChatRole::User),
        "assistant" => Some(ChatRole::Assistant),
        _ => None,
    }
}

/// V1 `toChatRole` (conversation import): exactly `"system"`, `"user"` or `"assistant"`
/// (case-sensitive) map to themselves, **anything else becomes `Assistant`**
/// (`"USER"` → assistant, `"Kurisu"` → assistant).
pub fn to_chat_role(role: &str) -> ChatRole {
    parse_chat_role(role).unwrap_or(ChatRole::Assistant)
}

/// The 11 emotion tags in canonical V1 order (persona prompts, achievements, tag regexes).
/// `SLEEPING` and `SNEEZE` are Live2D motions but NOT emotion tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Emotion {
    /// `NORMAL` (the fallback).
    #[default]
    Normal,
    /// `SMILE`.
    Smile,
    /// `ANGRY`.
    Angry,
    /// `SAD`.
    Sad,
    /// `SURPRISED`.
    Surprised,
    /// `BLUSH`.
    Blush,
    /// `WINK`.
    Wink,
    /// `DISGUST`.
    Disgust,
    /// `SMUG`.
    Smug,
    /// `THINKING`.
    Thinking,
    /// `PANIC`.
    Panic,
}

impl Emotion {
    /// All tags in canonical order.
    pub const ALL: [Emotion; 11] = [
        Emotion::Normal,
        Emotion::Smile,
        Emotion::Angry,
        Emotion::Sad,
        Emotion::Surprised,
        Emotion::Blush,
        Emotion::Wink,
        Emotion::Disgust,
        Emotion::Smug,
        Emotion::Thinking,
        Emotion::Panic,
    ];

    /// Upper-case tag name (`"SMILE"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Emotion::Normal => "NORMAL",
            Emotion::Smile => "SMILE",
            Emotion::Angry => "ANGRY",
            Emotion::Sad => "SAD",
            Emotion::Surprised => "SURPRISED",
            Emotion::Blush => "BLUSH",
            Emotion::Wink => "WINK",
            Emotion::Disgust => "DISGUST",
            Emotion::Smug => "SMUG",
            Emotion::Thinking => "THINKING",
            Emotion::Panic => "PANIC",
        }
    }

    /// Exact (upper-case) tag name → emotion.
    pub fn from_name(name: &str) -> Option<Emotion> {
        Emotion::ALL.into_iter().find(|e| e.as_str() == name)
    }

    /// Tag name compared ASCII-case-insensitively (JS `/…/i` without the `u` flag folds ASCII
    /// only, so `"ſmile"` does NOT match).
    pub fn from_name_ignore_ascii_case(name: &str) -> Option<Emotion> {
        Emotion::ALL
            .into_iter()
            .find(|e| e.as_str().eq_ignore_ascii_case(name))
    }
}

impl fmt::Display for Emotion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One `[SAVE_EVENT: …]` result (`ChatMemoryExtraction.events[i]`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MemoryEvent {
    /// Episode summary (never empty in an extraction result).
    pub summary: String,
    /// De-duplicated (exact match) tags.
    pub tags: Vec<String>,
}

/// V1 `ChatMemoryExtraction` (result of `extractMemoryCommands`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMemoryExtraction {
    /// Reply text with the commands removed.
    pub cleaned_text: String,
    /// `[SAVE_FACT: …]` payloads (de-duplicated, exact match).
    pub facts: Vec<String>,
    /// `[SAVE_EVENT: …]` payloads (not de-duplicated).
    pub events: Vec<MemoryEvent>,
}

/// Latency class of the status line (`> 3000` bad, `> 1000` warn, else good).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LatencyClass {
    /// ≤ 1000 ms.
    Good,
    /// 1001–3000 ms.
    Warn,
    /// > 3000 ms.
    Bad,
}

impl LatencyClass {
    /// CSS class used by V1 (`good` / `warn` / `bad`).
    pub fn as_str(self) -> &'static str {
        match self {
            LatencyClass::Good => "good",
            LatencyClass::Warn => "warn",
            LatencyClass::Bad => "bad",
        }
    }
}

/// V1 `LlmStatus` (`{ provider, model, latencyMs }`); `latency_ms == -1` means unknown.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmStatus {
    /// Display name (`providers::provider_display_name`).
    pub provider: String,
    /// Model name (configured or the display default).
    pub model: String,
    /// Last measured round trip in ms (`Date.now() - start`), or -1.
    pub latency_ms: i64,
}

impl Default for LlmStatus {
    /// V1 root initial state: `{ provider: "", model: "", latencyMs: -1 }`.
    fn default() -> Self {
        Self {
            provider: String::new(),
            model: String::new(),
            latency_ms: Self::UNKNOWN_LATENCY,
        }
    }
}

impl LlmStatus {
    /// Sentinel for "no measurement yet".
    pub const UNKNOWN_LATENCY: i64 = -1;

    /// `provider && model ? `${provider} / ${model}` : "---"` (status panel and mobile marquee).
    pub fn model_text(&self) -> String {
        if self.provider.is_empty() || self.model.is_empty() {
            "---".to_owned()
        } else {
            format!("{} / {}", self.provider, self.model)
        }
    }

    /// `None` when unknown (`latencyMs < 0`, shown as `--- ms`), else the latency class.
    pub fn latency_class(&self) -> Option<LatencyClass> {
        match self.latency_ms {
            ms if ms < 0 => None,
            ms if ms > 3000 => Some(LatencyClass::Bad),
            ms if ms > 1000 => Some(LatencyClass::Warn),
            _ => Some(LatencyClass::Good),
        }
    }
}

/// V1 `VectorEntry` (client-side RAG store).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VectorEntry {
    /// Entry id.
    pub id: String,
    /// Stored text.
    pub content: String,
    /// Category label.
    pub category: String,
    /// Embedding vector.
    pub embedding: Vec<f64>,
    /// Arbitrary metadata (`Record<string, unknown>`).
    pub metadata: Map<String, Value>,
}

/// V1 `VectorSearchResult`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// Entry id.
    pub id: String,
    /// Stored text.
    pub content: String,
    /// Category label.
    pub category: String,
    /// Similarity score.
    pub similarity: f64,
    /// Entry metadata.
    pub metadata: Map<String, Value>,
}

/// V1 `OverlayPanel` (in-memory UI state, never persisted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum OverlayPanel {
    /// `"none"`.
    #[default]
    #[serde(rename = "none")]
    None,
    /// `"backlog"`.
    #[serde(rename = "backlog")]
    Backlog,
    /// `"config"`.
    #[serde(rename = "config")]
    Config,
    /// `"status"`.
    #[serde(rename = "status")]
    Status,
    /// `"changeLog"`.
    #[serde(rename = "changeLog")]
    ChangeLog,
    /// `"help"`.
    #[serde(rename = "help")]
    Help,
    /// `"confirmLogout"`.
    #[serde(rename = "confirmLogout")]
    ConfirmLogout,
    /// `"confirmShutdown"`.
    #[serde(rename = "confirmShutdown")]
    ConfirmShutdown,
    /// `"minigame"` (declared but never assigned in V1).
    #[serde(rename = "minigame")]
    Minigame,
    /// `"achievements"` (only set by the mobile root).
    #[serde(rename = "achievements")]
    Achievements,
}

impl OverlayPanel {
    /// V1 string id.
    pub fn as_str(self) -> &'static str {
        match self {
            OverlayPanel::None => "none",
            OverlayPanel::Backlog => "backlog",
            OverlayPanel::Config => "config",
            OverlayPanel::Status => "status",
            OverlayPanel::ChangeLog => "changeLog",
            OverlayPanel::Help => "help",
            OverlayPanel::ConfirmLogout => "confirmLogout",
            OverlayPanel::ConfirmShutdown => "confirmShutdown",
            OverlayPanel::Minigame => "minigame",
            OverlayPanel::Achievements => "achievements",
        }
    }
}

/// Backlog name colour of user lines (`user` / `me`).
pub const BACKLOG_COLOR_USER: &str = "#66CCFF";
/// Backlog name colour of assistant lines (`assistant` / `kurisu` / `amadeus`).
pub const BACKLOG_COLOR_ASSISTANT: &str = "#FF6666";
/// Backlog name colour of any other role.
pub const BACKLOG_COLOR_OTHER: &str = "#808080";

/// Name colour for a lower-cased role id (V1 `addBackLog`).
pub fn backlog_name_color(role_id: &str) -> &'static str {
    match role_id {
        "user" | "me" => BACKLOG_COLOR_USER,
        "assistant" | "kurisu" | "amadeus" => BACKLOG_COLOR_ASSISTANT,
        _ => BACKLOG_COLOR_OTHER,
    }
}

/// True when a lower-cased role id is shown as the assistant (`assistant`/`kurisu`/`amadeus`);
/// such imported messages are split into pages with `chat_utils::split_message`.
pub fn is_assistant_role_id(role_id: &str) -> bool {
    matches!(role_id, "assistant" | "kurisu" | "amadeus")
}

/// V1 `BackLogEntry` (`{ role, nameColor, message }`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackLogEntry {
    /// Lower-cased role id.
    pub role: String,
    /// `#66CCFF` / `#FF6666` / `#808080`.
    pub name_color: String,
    /// Cleaned message text.
    pub message: String,
}

impl BackLogEntry {
    /// V1 `addBackLog(role, rawMessage)`: cleans the text with
    /// `chat_utils::clean_backlog_message` (drops ANY leading `[...]` prefix and every emotion
    /// tag) and returns `None` when nothing is left; `role` is lower-cased (Unicode, like JS
    /// `toLowerCase`).
    pub fn from_raw(role: &str, raw_message: &str) -> Option<Self> {
        let message = crate::chat_utils::clean_backlog_message(raw_message)?;
        let role_id = role.to_lowercase();
        let name_color = backlog_name_color(&role_id).to_owned();
        Some(Self {
            role: role_id,
            name_color,
            message,
        })
    }
}

/// V1 `ConfigIoResult` (`{ success, message }`) of conversation export/import.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ConfigIoResult {
    /// Shown in green (`#00CC00`) when true, red (`#FF4444`) otherwise.
    pub success: bool,
    /// User-visible message.
    pub message: String,
}

impl ConfigIoResult {
    /// Successful result.
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
        }
    }
    /// Failed result.
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
        }
    }
}

/// V1 `ImportedMessage` (raw role string as found in an import file).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ImportedMessage {
    /// Raw role (`"kurisu"`, `"System"`, …).
    pub role: String,
    /// Message text.
    pub content: String,
}

impl ImportedMessage {
    /// The history message V1 stores for it (`{ role: toChatRole(role), content }`).
    pub fn to_chat_message(&self) -> ChatMessage {
        ChatMessage::new(to_chat_role(&self.role), self.content.clone())
    }
}

/// Handle returned by [`Emitter::connect`]; pass it to [`Emitter::disconnect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

type Handler<T> = Rc<dyn Fn(&T)>;

/// Port of V1 `Signal<TArgs>` (`lib/amadeus/signal.ts`), named `Emitter` to avoid clashing
/// with Leptos' `Signal`. Single-threaded (wasm UI and tests).
///
/// Semantics kept from V1: handlers run synchronously in connection order; `emit` iterates a
/// **snapshot**, so a handler connected during an emit is not called in that emit, and one
/// disconnected during an emit is still called if it was in the snapshot; re-entrant `emit`,
/// `connect` and `disconnect` from inside a handler are allowed. (V1's dedup of the same
/// function object cannot be expressed with closures; V1 never relies on it.)
pub struct Emitter<T: ?Sized> {
    handlers: RefCell<Vec<(ConnectionId, Handler<T>)>>,
    next_id: Cell<u64>,
}

impl<T: ?Sized> Default for Emitter<T> {
    fn default() -> Self {
        Self {
            handlers: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
        }
    }
}

impl<T: ?Sized> fmt::Debug for Emitter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Emitter")
            .field("handlers", &self.len())
            .finish()
    }
}

impl<T: ?Sized> Emitter<T> {
    /// Emitter without handlers.
    pub fn new() -> Self {
        Self::default()
    }

    /// V1 `connect(handler)`; returns the id used to disconnect.
    pub fn connect(&self, handler: impl Fn(&T) + 'static) -> ConnectionId {
        let id = ConnectionId(self.next_id.get());
        self.next_id.set(self.next_id.get().wrapping_add(1));
        self.handlers.borrow_mut().push((id, Rc::new(handler)));
        id
    }

    /// V1 `disconnect(handler)`; returns whether the handler was connected.
    pub fn disconnect(&self, id: ConnectionId) -> bool {
        let mut handlers = self.handlers.borrow_mut();
        let before = handlers.len();
        handlers.retain(|(hid, _)| *hid != id);
        handlers.len() != before
    }

    /// V1 `emit(...args)`: calls a snapshot of the handlers in connection order.
    pub fn emit(&self, args: &T) {
        let snapshot: Vec<Handler<T>> = self
            .handlers
            .borrow()
            .iter()
            .map(|(_, h)| Rc::clone(h))
            .collect();
        for handler in snapshot {
            handler(args);
        }
    }

    /// V1 `clear()`: removes every handler.
    pub fn clear(&self) {
        self.handlers.borrow_mut().clear();
    }

    /// Number of connected handlers.
    pub fn len(&self) -> usize {
        self.handlers.borrow().len()
    }

    /// True when no handler is connected.
    pub fn is_empty(&self) -> bool {
        self.handlers.borrow().is_empty()
    }
}

// ---------------------------------------------------------------------------------------------
// Conversation export / import (V1 root `exportConversation` / `importConversation`)
// ---------------------------------------------------------------------------------------------

/// `version` written into V1 export files.
pub const CONVERSATION_EXPORT_VERSION: u32 = 1;

/// V1 export file name: `"chat_export_" + iso.replace(/[:.]/g, "-").substring(0, 19) + ".json"`
/// (`now_iso` is `new Date().toISOString()`, UTC, e.g. `chat_export_2026-11-16T12-34-56.json`).
pub fn conversation_export_file_name(now_iso: &str) -> String {
    let stamp: String = now_iso
        .chars()
        .map(|c| if c == ':' || c == '.' { '-' } else { c })
        .take(19)
        .collect();
    format!("chat_export_{stamp}.json")
}

/// A JS number serialised like `JSON.stringify`: `null` when not finite, otherwise ECMAScript
/// `Number::toString`. Exact output for every finite value needs [`JsPrettyFormatter`]. Safe
/// integers are emitted as integers, so any serde_json formatter prints those like JS.
struct JsNumber(f64);

impl Serialize for JsNumber {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        const MAX_SAFE: f64 = 9_007_199_254_740_992.0;
        let v = self.0;
        if !v.is_finite() {
            serializer.serialize_unit() // JSON.stringify(NaN / Infinity) === "null"
        } else if v.fract() == 0.0 && v.abs() < MAX_SAFE {
            serializer.serialize_i64(v as i64)
        } else {
            serializer.serialize_f64(v)
        }
    }
}

/// serde_json formatter for `JSON.stringify(value, null, 2)`. It keeps serde_json's pretty
/// layout, which matches JS (2-space indent, `": "`, `[]` / `{}` for empty containers, same
/// string escapes). Floats are written with ECMAScript `Number::toString` (`ryu-js`) instead
/// of serde_json's style: `2^53` → `9007199254740992` (not `9007199254740992.0`), `1e20` →
/// `100000000000000000000`, `1e21` → `1e+21`, `1e-6` → `0.000001`.
struct JsPrettyFormatter(PrettyFormatter<'static>);

impl Formatter for JsPrettyFormatter {
    fn write_f64<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        writer.write_all(ryu_js::Buffer::new().format(value).as_bytes())
    }

    fn write_f32<W: ?Sized + io::Write>(&mut self, writer: &mut W, value: f32) -> io::Result<()> {
        self.write_f64(writer, f64::from(value))
    }

    fn begin_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_array(writer)
    }

    fn end_array<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_array(writer)
    }

    fn begin_array_value<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.0.begin_array_value(writer, first)
    }

    fn end_array_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_array_value(writer)
    }

    fn begin_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_object(writer)
    }

    fn end_object<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_object(writer)
    }

    fn begin_object_key<W: ?Sized + io::Write>(
        &mut self,
        writer: &mut W,
        first: bool,
    ) -> io::Result<()> {
        self.0.begin_object_key(writer, first)
    }

    fn begin_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.begin_object_value(writer)
    }

    fn end_object_value<W: ?Sized + io::Write>(&mut self, writer: &mut W) -> io::Result<()> {
        self.0.end_object_value(writer)
    }
}

/// `JSON.stringify(value, null, 2)` for a serde value (see [`JsPrettyFormatter`]).
fn js_json_stringify_pretty<T: Serialize + ?Sized>(value: &T) -> Option<String> {
    let mut out = Vec::new();
    let formatter = JsPrettyFormatter(PrettyFormatter::with_indent(b"  "));
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
    value.serialize(&mut serializer).ok()?;
    String::from_utf8(out).ok()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationExportFile<'a> {
    version: u32,
    exported_at: &'a str,
    turn_count: JsNumber,
    messages: Vec<&'a ChatMessage>,
}

/// V1 export body: `JSON.stringify({ version: 1, exportedAt, turnCount, messages }, null, 2)`
/// with system messages removed (2-space indent, key order kept, no trailing newline).
/// `None` when there is no non-system message (V1 shows `export_no_data`).
pub fn conversation_export_json(
    history: &[ChatMessage],
    turn_count: f64,
    exported_at_iso: &str,
) -> Option<String> {
    let messages: Vec<&ChatMessage> = history
        .iter()
        .filter(|m| m.role != ChatRole::System)
        .collect();
    if messages.is_empty() {
        return None;
    }
    let file = ConversationExportFile {
        version: CONVERSATION_EXPORT_VERSION,
        exported_at: exported_at_iso,
        turn_count: JsNumber(turn_count),
        messages,
    };
    js_json_stringify_pretty(&file)
}

/// A validated import file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImportedConversation {
    /// Messages with their raw roles (map with [`to_chat_role`] for the history).
    pub messages: Vec<ImportedMessage>,
    /// `turnCount` when it is a JSON number, else 0.
    pub turn_count: f64,
}

/// Why an import file was rejected. `Display` is for logs. The UI shows
/// `t("import_invalid", "無効なファイル形式です", lang)` for [`Invalid`](Self::Invalid) and
/// `t("import_failed", "インポート失敗: ", lang) + message` for [`Parse`](Self::Parse).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConversationImportError {
    /// Valid JSON of the wrong shape (V1 `import_invalid`, "無効なファイル形式です").
    #[error("invalid conversation file")]
    Invalid,
    /// Not JSON (V1 `import_failed` + `String(error)`); V2 cannot reproduce V8's message text
    /// and uses `"SyntaxError: <serde_json message>"`.
    #[error("{0}")]
    Parse(String),
}

/// V1 import validation: the root must be a non-array object with a `messages` array whose
/// elements are objects with string `role` and `content` (extra fields ignored, `version` not
/// checked); `turnCount` is taken when it is a number.
///
/// Numbers are parsed with correct rounding, like `JSON.parse`. Known deviation: JSON that
/// `JSON.parse` accepts but serde_json rejects is a [`Parse`](ConversationImportError::Parse)
/// error here. That covers numbers beyond the `f64` range (`1e400`, `Infinity` in V1),
/// lone-surrogate escapes (`"\ud800"`) and nesting deeper than 128 levels.
pub fn parse_conversation_import(
    raw: &str,
) -> Result<ImportedConversation, ConversationImportError> {
    let parsed: Value = serde_json::from_str(raw)
        .map_err(|e| ConversationImportError::Parse(format!("SyntaxError: {e}")))?;
    let Value::Object(root) = parsed else {
        return Err(ConversationImportError::Invalid);
    };
    let Some(Value::Array(entries)) = root.get("messages") else {
        return Err(ConversationImportError::Invalid);
    };
    let mut messages = Vec::with_capacity(entries.len());
    for entry in entries {
        let Value::Object(obj) = entry else {
            return Err(ConversationImportError::Invalid);
        };
        match (obj.get("role"), obj.get("content")) {
            (Some(Value::String(role)), Some(Value::String(content))) => {
                messages.push(ImportedMessage {
                    role: role.clone(),
                    content: content.clone(),
                });
            }
            _ => return Err(ConversationImportError::Invalid),
        }
    }
    let turn_count = root.get("turnCount").and_then(Value::as_f64).unwrap_or(0.0);
    Ok(ImportedConversation {
        messages,
        turn_count,
    })
}

/// V1 backlog rebuild after an import: `role === "system"` (case-sensitive) is skipped;
/// assistant-like roles (lower-cased `assistant`/`kurisu`/`amadeus`) are split into pages with
/// `chat_utils::split_message`; every page/message goes through [`BackLogEntry::from_raw`].
pub fn backlog_entries_for_import(messages: &[ImportedMessage]) -> Vec<BackLogEntry> {
    let mut entries = Vec::new();
    for message in messages {
        if message.role == "system" {
            continue;
        }
        if is_assistant_role_id(&message.role.to_lowercase()) {
            for segment in crate::chat_utils::split_message(&message.content) {
                entries.extend(BackLogEntry::from_raw(&message.role, &segment));
            }
        } else {
            entries.extend(BackLogEntry::from_raw(&message.role, &message.content));
        }
    }
    entries
}

// ---------------------------------------------------------------------------------------------
// Login screens (V1 `MobileLoginPanel.submit` / desktop `LoginPanel.attemptLogin`)
// ---------------------------------------------------------------------------------------------

/// Pure rules of the two login screens. The credentials are a fake in-universe login
/// (Salieri / MakiseKurisu) with no security role, kept verbatim from V1. On success both
/// screens call `MemoryManager.setUserName(LOGIN_USER_NAME)`, whatever was typed.
pub mod login {
    use crate::jscompat::js_trim;

    /// Accepted user ids (V1 `validNames`, exact and case-sensitive).
    pub const VALID_USER_IDS: [&str; 1] = ["Salieri"];
    /// Accepted passwords (V1 `validPasswords`, exact and case-sensitive).
    pub const VALID_PASSWORDS: [&str; 1] = ["MakiseKurisu"];
    /// User name stored after every successful login (`MemoryManager.setUserName("Salieri")`).
    pub const LOGIN_USER_NAME: &str = "Salieri";
    /// Error shown by the desktop panel (`.login-error`, literal, never translated).
    pub const DESKTOP_LOGIN_ERROR: &str = "ACCESS DENIED";

    /// Result of the mobile login check.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum MobileLogin {
        /// A field is empty (`!userId || !password`, no trim).
        Empty,
        /// Both fields are filled but neither the id nor the password is valid.
        Denied,
        /// Accepted: clear the error, set the user name, call `onAccepted()`.
        Accepted,
    }

    impl MobileLogin {
        /// `(i18n key, inline default)` of the error to show, `None` when accepted. Pass it
        /// to `i18n::t(key, default, lang)`; the result is computed once at submit time.
        /// `login_empty` is in no locale, so every language except English (1) shows the
        /// Japanese default.
        pub fn error_message(self, config_language: i64) -> Option<(&'static str, &'static str)> {
            let english = config_language == 1;
            match self {
                MobileLogin::Empty => Some((
                    "login_empty",
                    if english {
                        "Please enter both fields."
                    } else {
                        "ユーザーIDとパスワードを入力してください。"
                    },
                )),
                MobileLogin::Denied => Some((
                    "login_access_denied",
                    if english {
                        "ACCESS DENIED"
                    } else {
                        "アクセスが拒否されました"
                    },
                )),
                MobileLogin::Accepted => None,
            }
        }
    }

    /// V1 mobile `submit()`. Uses the raw values without trimming: an empty field is
    /// [`Empty`](MobileLogin::Empty), and the login is [`Denied`](MobileLogin::Denied) only
    /// when both the id and the password are wrong (`"   "` + `MakiseKurisu` is accepted,
    /// `"Salieri "` + a wrong password is denied).
    pub fn validate_mobile_login(user_id: &str, password: &str) -> MobileLogin {
        if user_id.is_empty() || password.is_empty() {
            MobileLogin::Empty
        } else if !VALID_USER_IDS.contains(&user_id) && !VALID_PASSWORDS.contains(&password) {
            MobileLogin::Denied
        } else {
            MobileLogin::Accepted
        }
    }

    /// V1 desktop `attemptLogin()`. Both fields are trimmed (JS `trim()`). The login is
    /// accepted when both are non-empty and the id or the password is valid. Otherwise the
    /// panel shows [`DESKTOP_LOGIN_ERROR`].
    pub fn validate_desktop_login(user_id: &str, password: &str) -> bool {
        let user = js_trim(user_id);
        let pass = js_trim(password);
        !user.is_empty()
            && !pass.is_empty()
            && (VALID_USER_IDS.contains(&user) || VALID_PASSWORDS.contains(&pass))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_role_mapping_matches_v1() {
        assert_eq!(to_chat_role("system"), ChatRole::System);
        assert_eq!(to_chat_role("user"), ChatRole::User);
        assert_eq!(to_chat_role("assistant"), ChatRole::Assistant);
        assert_eq!(to_chat_role("USER"), ChatRole::Assistant);
        assert_eq!(to_chat_role("Kurisu"), ChatRole::Assistant);
        assert_eq!(parse_chat_role("System"), None);
        let m = ImportedMessage {
            role: "me".into(),
            content: "x".into(),
        };
        assert_eq!(m.to_chat_message(), ChatMessage::assistant("x"));
    }

    #[test]
    fn emotion_names() {
        assert_eq!(
            Emotion::ALL.map(Emotion::as_str).join(","),
            "NORMAL,SMILE,ANGRY,SAD,SURPRISED,BLUSH,WINK,DISGUST,SMUG,THINKING,PANIC"
        );
        assert_eq!(Emotion::from_name("SMUG"), Some(Emotion::Smug));
        assert_eq!(Emotion::from_name("smug"), None);
        assert_eq!(
            Emotion::from_name_ignore_ascii_case("sMuG"),
            Some(Emotion::Smug)
        );
        assert_eq!(Emotion::from_name_ignore_ascii_case("\u{17F}mile"), None);
        assert_eq!(
            serde_json::to_string(&Emotion::Thinking).ok().as_deref(),
            Some("\"THINKING\"")
        );
    }

    #[test]
    fn serde_shapes_match_v1() {
        let status = LlmStatus {
            provider: "FreeAPI".into(),
            model: "default".into(),
            latency_ms: -1,
        };
        assert_eq!(
            serde_json::to_string(&status).ok().as_deref(),
            Some(r#"{"provider":"FreeAPI","model":"default","latencyMs":-1}"#)
        );
        let extraction = ChatMemoryExtraction {
            cleaned_text: "x".into(),
            facts: vec!["f".into()],
            events: vec![MemoryEvent {
                summary: "s".into(),
                tags: vec!["t".into()],
            }],
        };
        assert_eq!(
            serde_json::to_string(&extraction).ok().as_deref(),
            Some(r#"{"cleanedText":"x","facts":["f"],"events":[{"summary":"s","tags":["t"]}]}"#)
        );
        let entry = BackLogEntry {
            role: "user".into(),
            name_color: "#66CCFF".into(),
            message: "m".into(),
        };
        assert_eq!(
            serde_json::to_string(&entry).ok().as_deref(),
            Some(r##"{"role":"user","nameColor":"#66CCFF","message":"m"}"##)
        );
        assert_eq!(
            serde_json::to_string(&OverlayPanel::ChangeLog)
                .ok()
                .as_deref(),
            Some("\"changeLog\"")
        );
        assert_eq!(OverlayPanel::ConfirmShutdown.as_str(), "confirmShutdown");
        assert_eq!(
            serde_json::to_string(&AppState::Boot).ok().as_deref(),
            Some("\"boot\"")
        );
    }

    #[test]
    fn llm_status_display_rules() {
        let mut s = LlmStatus::default();
        assert_eq!(s.model_text(), "---");
        assert_eq!(s.latency_class(), None);
        s.provider = "Gemini".into();
        s.model = "gemini-1.5-pro".into();
        assert_eq!(s.model_text(), "Gemini / gemini-1.5-pro");
        for (ms, class) in [
            (0, LatencyClass::Good),
            (1000, LatencyClass::Good),
            (1001, LatencyClass::Warn),
            (3000, LatencyClass::Warn),
            (3001, LatencyClass::Bad),
        ] {
            s.latency_ms = ms;
            assert_eq!(s.latency_class(), Some(class), "{ms}");
        }
    }

    /// Golden: V1 `addBackLog` results.
    #[test]
    fn backlog_entries_match_v1() {
        type Expected = Option<(&'static str, &'static str, &'static str)>;
        let cases: [(&str, &str, Expected); 8] = [
            (
                "user",
                "  [SMILE] hello [wink] world ",
                Some(("user", "#66CCFF", "hello  world")),
            ),
            ("Kurisu", "[注釈] 本文", Some(("kurisu", "#FF6666", "本文"))),
            ("ME", "[SMILE]", None),
            ("System", "plain", Some(("system", "#808080", "plain"))),
            ("assistant", "[x", Some(("assistant", "#FF6666", "[x"))),
            ("AMADEUS", "[]rest", Some(("amadeus", "#FF6666", "rest"))),
            ("user", "a [SAD] b", Some(("user", "#66CCFF", "a  b"))),
            ("\u{130}", "x", Some(("i\u{307}", "#808080", "x"))),
        ];
        for (role, raw, expected) in cases {
            let got = BackLogEntry::from_raw(role, raw);
            let expected = expected.map(|(r, c, m)| BackLogEntry {
                role: r.into(),
                name_color: c.into(),
                message: m.into(),
            });
            assert_eq!(got, expected, "{role:?} {raw:?}");
        }
        assert!(is_assistant_role_id("kurisu"));
        assert!(!is_assistant_role_id("Kurisu"));
    }

    #[test]
    fn emitter_snapshot_semantics() {
        let emitter: Rc<Emitter<str>> = Rc::new(Emitter::new());
        let log = Rc::new(RefCell::new(Vec::<String>::new()));
        let (l1, e1) = (log.clone(), emitter.clone());
        let second = Rc::new(Cell::new(None::<ConnectionId>));
        let second_in_first = second.clone();
        emitter.connect(move |k| {
            l1.borrow_mut().push(format!("a:{k}"));
            // Disconnecting a handler that is in the snapshot does not prevent its call.
            if let Some(id) = second_in_first.get() {
                e1.disconnect(id);
            }
            // Connecting during an emit: not called in this emit.
            let l3 = l1.clone();
            e1.connect(move |k| l3.borrow_mut().push(format!("late:{k}")));
        });
        let l2 = log.clone();
        second.set(Some(
            emitter.connect(move |k| l2.borrow_mut().push(format!("b:{k}"))),
        ));
        emitter.emit("x");
        assert_eq!(*log.borrow(), ["a:x", "b:x"]);
        assert_eq!(emitter.len(), 2);
        log.borrow_mut().clear();
        emitter.emit("y");
        assert_eq!(*log.borrow(), ["a:y", "late:y"]);
        emitter.clear();
        assert!(emitter.is_empty());
        emitter.emit("z");
        assert_eq!(log.borrow().len(), 2);
    }

    #[test]
    fn emitter_reentrant_emit() {
        let emitter: Rc<Emitter<u32>> = Rc::new(Emitter::new());
        let total = Rc::new(Cell::new(0));
        let (t, e) = (total.clone(), emitter.clone());
        emitter.connect(move |n| {
            t.set(t.get() + n);
            if *n > 0 {
                e.emit(&(n - 1));
            }
        });
        emitter.emit(&3);
        assert_eq!(total.get(), 6);
    }

    /// Golden: V1 `JSON.stringify(exportData, null, 2)` (Node 22).
    #[test]
    fn conversation_export_matches_v1() {
        let history = vec![
            ChatMessage::system("SYS"),
            ChatMessage::user("こんにちは\n\"q\"\\ \u{1}\u{1f}\u{2028}😀"),
            ChatMessage::assistant("[SMILE] ……"),
        ];
        assert_eq!(
            conversation_export_json(&history, 3.0, "2026-11-16T12:34:56.789Z").as_deref(),
            Some("{\n  \"version\": 1,\n  \"exportedAt\": \"2026-11-16T12:34:56.789Z\",\n  \"turnCount\": 3,\n  \"messages\": [\n    {\n      \"role\": \"user\",\n      \"content\": \"こんにちは\\n\\\"q\\\"\\\\ \\u0001\\u001f\u{2028}😀\"\n    },\n    {\n      \"role\": \"assistant\",\n      \"content\": \"[SMILE] ……\"\n    }\n  ]\n}")
        );
        assert_eq!(
            conversation_export_json(&[ChatMessage::system("S")], 0.0, "x"),
            None
        );
        let one = [ChatMessage::user("a")];
        let json = conversation_export_json(&one, 2.5, "x").unwrap_or_default();
        assert!(json.contains("\"turnCount\": 2.5,"));
        let json = conversation_export_json(&one, -0.0, "x").unwrap_or_default();
        assert!(json.contains("\"turnCount\": 0,"));
        assert_eq!(
            conversation_export_file_name("2026-11-16T12:34:56.789Z"),
            "chat_export_2026-11-16T12-34-56.json"
        );
    }

    /// Golden: V1 `JSON.stringify({ turnCount }, null, 2)` number output (Node 22).
    #[test]
    fn conversation_export_numbers_match_v1() {
        let cases: [(f64, &str); 21] = [
            (9_007_199_254_740_992.0, "9007199254740992"),
            (12_345_678_901_234_568.0, "12345678901234568"),
            (1e20, "100000000000000000000"),
            (1e21, "1e+21"),
            (1e-7, "1e-7"),
            (1e-6, "0.000001"),
            (2.5, "2.5"),
            (-3.0, "-3"),
            (0.1 + 0.2, "0.30000000000000004"),
            (f64::MAX, "1.7976931348623157e+308"),
            (5e-324, "5e-324"),
            (f64::NAN, "null"),
            (f64::INFINITY, "null"),
            (f64::NEG_INFINITY, "null"),
            (1_152_921_504_606_846_976.0, "1152921504606847000"),
            (-9_007_199_254_740_992.0, "-9007199254740992"),
            (123_456_789_012_345_680_000.0, "123456789012345680000"),
            (100_499.999_999_999_99, "100499.99999999999"),
            (9_007_199_254_740_991.0, "9007199254740991"),
            (1.5e300, "1.5e+300"),
            (0.000_001_234, "0.000001234"),
        ];
        let one = [ChatMessage::user("a")];
        for (turn_count, expected) in cases {
            let json = conversation_export_json(&one, turn_count, "x").unwrap_or_default();
            let needle = format!("\n  \"turnCount\": {expected},\n");
            assert!(json.contains(&needle), "{turn_count:?}: {json}");
        }
    }

    #[test]
    fn conversation_import_validation() {
        use ConversationImportError::{Invalid, Parse};
        assert!(
            matches!(parse_conversation_import("xyz"), Err(Parse(m)) if m.starts_with("SyntaxError: "))
        );
        for raw in [
            "[]",
            "null",
            "5",
            "{}",
            "{\"messages\":5}",
            "{\"messages\":[[1]]}",
            "{\"messages\":[null]}",
            "{\"messages\":[{\"role\":\"user\"}]}",
            "{\"messages\":[{\"role\":1,\"content\":\"x\"}]}",
        ] {
            assert_eq!(parse_conversation_import(raw), Err(Invalid), "{raw}");
        }
        let ok = parse_conversation_import(
            " {\"version\":9,\"turnCount\":\"3\",\"messages\":[{\"role\":\"kurisu\",\"content\":\"[SMILE] はい。そうね。\",\"x\":1},{\"role\":\"System\",\"content\":\"s\"},{\"role\":\"system\",\"content\":\"hidden\"},{\"role\":\"USER\",\"content\":\"[note] hi\"}]} ",
        );
        let Ok(conv) = ok else {
            panic!("expected a valid import: {ok:?}");
        };
        assert_eq!(conv.turn_count, 0.0, "non-number turnCount → 0");
        assert_eq!(conv.messages.len(), 4);
        let history: Vec<ChatMessage> = conv
            .messages
            .iter()
            .map(ImportedMessage::to_chat_message)
            .collect();
        assert_eq!(
            history[1],
            ChatMessage::assistant("s"),
            "\"System\" → assistant"
        );
        assert_eq!(history[2], ChatMessage::system("hidden"));
        let backlog = backlog_entries_for_import(&conv.messages);
        let rows: Vec<(&str, &str)> = backlog
            .iter()
            .map(|e| (e.role.as_str(), e.message.as_str()))
            .collect();
        assert_eq!(
            rows,
            [
                ("kurisu", "はい。"),
                ("kurisu", "そうね。"),
                ("system", "s"),
                ("user", "hi")
            ]
        );
        let conv = parse_conversation_import("{\"messages\":[],\"turnCount\":7}");
        assert_eq!(conv.map(|c| c.turn_count), Ok(7.0));
        // Correctly rounded like `JSON.parse` (the default serde_json parser gives 100500).
        let conv = parse_conversation_import("{\"messages\":[],\"turnCount\":100499.99999999999}");
        assert_eq!(
            conv.map(|c| c.turn_count.to_bits()),
            Ok(100_499.999_999_999_99_f64.to_bits())
        );
        let conv = parse_conversation_import("{\"messages\":[],\"turnCount\":0.9095702657941729}");
        assert_eq!(
            conv.map(|c| c.turn_count.to_bits()),
            Ok(0.909_570_265_794_172_9_f64.to_bits())
        );
        assert_eq!(
            ConversationImportError::Parse("SyntaxError: x".into()).to_string(),
            "SyntaxError: x"
        );
    }

    /// Golden: spec-app-part1 §13.2 truth table and §13.4 (V1 lines 495–509 and 2006–2020).
    #[test]
    fn login_truth_table_matches_v1() {
        use login::*;
        let mobile: [(&str, &str, MobileLogin); 12] = [
            ("Salieri", "anything", MobileLogin::Accepted),
            ("anything", "MakiseKurisu", MobileLogin::Accepted),
            ("Salieri", "MakiseKurisu", MobileLogin::Accepted),
            ("Salieri ", "wrong", MobileLogin::Denied),
            ("salieri", "wrong", MobileLogin::Denied),
            ("x", "makisekurisu", MobileLogin::Denied),
            ("   ", "MakiseKurisu", MobileLogin::Accepted),
            ("   ", "   ", MobileLogin::Denied),
            ("", "MakiseKurisu", MobileLogin::Empty),
            ("Salieri", "", MobileLogin::Empty),
            ("", "", MobileLogin::Empty),
            ("Salieri", " ", MobileLogin::Accepted),
        ];
        for (user, pass, expected) in mobile {
            assert_eq!(
                validate_mobile_login(user, pass),
                expected,
                "{user:?} {pass:?}"
            );
        }
        let desktop: [(&str, &str, bool); 11] = [
            ("Salieri", "anything", true),
            ("anything", "MakiseKurisu", true),
            (" Salieri\u{3000}", "wrong", true),
            ("x", "\u{FEFF}MakiseKurisu\n", true),
            ("salieri", "wrong", false),
            ("Salieri", "   ", false),
            ("   ", "MakiseKurisu", false),
            ("", "MakiseKurisu", false),
            ("Salieri", "", false),
            ("\u{0085}Salieri", "wrong", false),
            ("", "", false),
        ];
        for (user, pass, expected) in desktop {
            assert_eq!(
                validate_desktop_login(user, pass),
                expected,
                "{user:?} {pass:?}"
            );
        }
        assert_eq!(
            MobileLogin::Empty.error_message(1),
            Some(("login_empty", "Please enter both fields."))
        );
        assert_eq!(
            MobileLogin::Empty.error_message(2),
            Some(("login_empty", "ユーザーIDとパスワードを入力してください。"))
        );
        assert_eq!(
            MobileLogin::Denied.error_message(0),
            Some(("login_access_denied", "アクセスが拒否されました"))
        );
        assert_eq!(
            MobileLogin::Denied.error_message(1),
            Some(("login_access_denied", "ACCESS DENIED"))
        );
        assert_eq!(MobileLogin::Accepted.error_message(0), None);
        assert_eq!(LOGIN_USER_NAME, "Salieri");
        assert_eq!(DESKTOP_LOGIN_ERROR, "ACCESS DENIED");
    }
}
