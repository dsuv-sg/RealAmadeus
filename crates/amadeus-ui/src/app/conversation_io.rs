//! Conversation export / import and the back-log store of V1 `RealAmadeusApp`
//! (`addBackLog` L4018–4043, `exportConversation` L4045–4085, `importConversation`
//! L4087–4162). The pure halves are host-testable; the `*_conversation` functions add the
//! platform save and the achievement notification. OWNED BY: app-root engineer.

use amadeus_core::achievements::AchievementEvent;
use amadeus_core::i18n::t;
use amadeus_core::protocol::{ChatMessage, ChatRole};
use amadeus_core::types::{
    backlog_entries_for_import, conversation_export_file_name, conversation_export_json,
    parse_conversation_import, BackLogEntry as CoreBackLogEntry, ConversationImportError,
    ImportedMessage,
};
use futures::future::{FutureExt, LocalBoxFuture};

use super::managers;
use super::state::{BackLogEntry, ConfigIoResult, ConversationBridge};
use crate::platform::{platform, SaveOutcome, JSON_MIME};

/// V1 `addBackLog(role, rawMessage)` cleaning and colouring (`None` = nothing to add).
pub fn backlog_entry(role: &str, raw_message: &str) -> Option<BackLogEntry> {
    CoreBackLogEntry::from_raw(role, raw_message).map(Into::into)
}

/// `t("export_no_data", "会話データがありません", lang)`.
pub fn export_no_data_message(lang: i64) -> String {
    t("export_no_data", "会話データがありません", lang).to_owned()
}

/// `t("export_success", "エクスポート完了: ", lang) + fileName`.
pub fn export_success_message(lang: i64, file_name: &str) -> String {
    format!(
        "{}{file_name}",
        t("export_success", "エクスポート完了: ", lang)
    )
}

/// `t("export_failed", "エクスポート失敗: ", lang) + String(error)`.
pub fn export_failed_message(lang: i64, error: &str) -> String {
    format!("{}{error}", t("export_failed", "エクスポート失敗: ", lang))
}

/// `t("import_invalid", "無効なファイル形式です", lang)`.
pub fn import_invalid_message(lang: i64) -> String {
    t("import_invalid", "無効なファイル形式です", lang).to_owned()
}

/// `t("import_failed", "インポート失敗: ", lang) + String(error)`.
pub fn import_failed_message(lang: i64, error: &str) -> String {
    format!("{}{error}", t("import_failed", "インポート失敗: ", lang))
}

/// `t("import_success", …) + String(imported.length) + t("import_turns", …)` — counts every
/// imported message (system ones included), not turns (V1 QUIRK-5).
pub fn import_success_message(lang: i64, count: usize) -> String {
    format!(
        "{}{count}{}",
        t("import_success", "インポート完了: ", lang),
        t("import_turns", " ターン", lang)
    )
}

/// An export ready to be saved.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportFile {
    /// `chat_export_<UTC stamp>.json`.
    pub file_name: String,
    /// `JSON.stringify({version, exportedAt, turnCount, messages}, null, 2)`.
    pub json: String,
}

/// Pure part of V1 `exportConversation` once the bridge exists: `None` when the history has no
/// non-system message (`export_no_data`). Calls happen in V1 order: the file-name timestamp
/// (`now_iso`, 1st `new Date()`), `exportedAt` (2nd `new Date()`), then `getTurnCount()`.
pub fn prepare_export(
    history: &[ChatMessage],
    turn_count: impl FnOnce() -> f64,
    mut now_iso: impl FnMut() -> String,
) -> Option<ExportFile> {
    if !history
        .iter()
        .any(|message| message.role != ChatRole::System)
    {
        return None;
    }
    let file_name = conversation_export_file_name(&now_iso());
    let exported_at = now_iso();
    let turn_count = turn_count();
    let json = conversation_export_json(history, turn_count, &exported_at)?;
    Some(ExportFile { file_name, json })
}

/// A validated import, ready to apply.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedImport {
    /// History for `bridge.applyImported` (`toChatRole` applied).
    pub messages: Vec<ChatMessage>,
    /// `typeof turnCount === "number" ? turnCount : 0`.
    pub turn_count: f64,
    /// The rebuilt back log (system skipped, assistant-like roles split into pages).
    pub backlog: Vec<BackLogEntry>,
    /// The success message.
    pub message: String,
}

/// Pure part of V1 `importConversation(raw)`: validation in V1 order, `Err` carries the
/// failure result (`import_invalid`, or `import_failed` + the parse error).
pub fn prepare_import(raw: &str, lang: i64) -> Result<PreparedImport, ConfigIoResult> {
    let conversation = parse_conversation_import(raw).map_err(|error| ConfigIoResult {
        success: false,
        message: match error {
            ConversationImportError::Invalid => import_invalid_message(lang),
            ConversationImportError::Parse(message) => import_failed_message(lang, &message),
        },
    })?;
    Ok(PreparedImport {
        messages: conversation
            .messages
            .iter()
            .map(ImportedMessage::to_chat_message)
            .collect(),
        turn_count: conversation.turn_count,
        backlog: backlog_entries_for_import(&conversation.messages)
            .into_iter()
            .map(Into::into)
            .collect(),
        message: import_success_message(lang, conversation.messages.len()),
    })
}

/// The status line once a save finished: V1's success text after the file was written, V1's
/// `export_failed` text (+ the error) when the save failed, and nothing new (`None`) when the
/// desktop save dialog was cancelled.
pub fn export_save_status(
    outcome: &Result<SaveOutcome, String>,
    file_name: &str,
    lang: i64,
) -> Option<ConfigIoResult> {
    match outcome {
        Ok(SaveOutcome::Saved { .. }) => Some(ConfigIoResult {
            success: true,
            message: export_success_message(lang, file_name),
        }),
        Ok(SaveOutcome::Cancelled) => None,
        Err(error) => Some(ConfigIoResult {
            success: false,
            message: export_failed_message(lang, error),
        }),
    }
}

/// A pending [`PlatformShell::save_text_file`](crate::platform::PlatformShell::save_text_file).
type PendingSave = LocalBoxFuture<'static, Result<SaveOutcome, String>>;

/// Calls `on_saved` with the outcome once `save` has finished: right away when it already has
/// (the web download happens inside `save_text_file`, as in V1), otherwise from the task handed
/// to `spawn` once the desktop save dialog has closed and the file is written.
fn when_saved(
    mut save: PendingSave,
    on_saved: impl FnOnce(Result<SaveOutcome, String>) + 'static,
    spawn: impl FnOnce(LocalBoxFuture<'static, ()>),
) {
    match (&mut save).now_or_never() {
        Some(outcome) => on_saved(outcome),
        None => spawn(async move { on_saved(save.await) }.boxed_local()),
    }
}

/// V1 `exportConversation()` (ConfigPanel "書き出す / Export"). `report` receives what the
/// status line shows, if anything (V1 `setIoStatus(onExportConversation())`).
///
/// Web: exactly V1. The Blob + `<a download>` save happens inside this call, then
/// `notifyEvent("export")` and the success text are reported at once (a browser download cannot
/// be observed). Desktop: the native save dialog opens; once it closes, a written file fires the
/// achievement and reports the success text, a failed write reports V1's `export_failed` text, and
/// a cancel reports nothing (the status line keeps what it showed). No data: `export_no_data` at
/// once, on both.
pub fn export_conversation(
    bridge: Option<ConversationBridge>,
    lang: i64,
    report: impl FnOnce(ConfigIoResult) + 'static,
) {
    let history = bridge
        .as_ref()
        .map(|bridge| (bridge.get_history)())
        .unwrap_or_default();
    let prepared = bridge.as_ref().and_then(|bridge| {
        prepare_export(&history, || (bridge.get_turn_count)(), crate::env::now_iso)
    });
    let Some(ExportFile { file_name, json }) = prepared else {
        report(ConfigIoResult {
            success: false,
            message: export_no_data_message(lang),
        });
        return;
    };
    let save = platform().save_text_file(&file_name, JSON_MIME, &json);
    when_saved(
        save,
        move |outcome| {
            match &outcome {
                Ok(SaveOutcome::Saved { .. }) => {
                    managers::notify_achievement(&AchievementEvent::Export);
                }
                Ok(SaveOutcome::Cancelled) => log::info!("conversation export cancelled"),
                Err(error) => log::warn!("conversation export failed: {error}"),
            }
            if let Some(status) = export_save_status(&outcome, &file_name, lang) {
                report(status);
            }
        },
        wasm_bindgen_futures::spawn_local,
    );
}

/// V1 `importConversation(raw)` (ConfigPanel "読み込む / Import"). Without a mounted ChatPanel
/// (login/boot) the back log is still replaced, the achievement fires and success is reported
/// (V1 QUIRK-4). The history replacement drops the system prompt (QUIRK-3, ChatPanel's bridge).
pub fn import_conversation(
    raw: &str,
    bridge: Option<ConversationBridge>,
    lang: i64,
    set_backlog: impl FnOnce(Vec<BackLogEntry>),
) -> ConfigIoResult {
    let prepared = match prepare_import(raw, lang) {
        Ok(prepared) => prepared,
        Err(failure) => return failure,
    };
    if let Some(bridge) = bridge {
        (bridge.apply_imported)(prepared.messages, prepared.turn_count);
    }
    set_backlog(prepared.backlog);
    managers::notify_achievement(&AchievementEvent::Import);
    ConfigIoResult {
        success: true,
        message: prepared.message,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use futures::channel::oneshot;
    use futures::future;

    use super::*;

    const FILE: &str = "chat_export_2026-11-21T11-53-16.json";

    #[test]
    fn export_status_after_the_save() {
        let saved = Ok(SaveOutcome::Saved {
            path: Some(format!("/home/u/Downloads/{FILE}")),
        });
        assert_eq!(
            export_save_status(&saved, FILE, 0),
            Some(ConfigIoResult {
                success: true,
                message: format!("エクスポート完了: {FILE}"),
            })
        );
        assert_eq!(
            export_save_status(&Ok(SaveOutcome::Saved { path: None }), FILE, 1),
            Some(ConfigIoResult {
                success: true,
                message: format!("Export complete: {FILE}"),
            })
        );
        // A cancelled dialog shows nothing new.
        assert_eq!(
            export_save_status(&Ok(SaveOutcome::Cancelled), FILE, 0),
            None
        );
        let failed = Err("cannot write /x/a.json: Permission denied (os error 13)".to_owned());
        assert_eq!(
            export_save_status(&failed, FILE, 0),
            Some(ConfigIoResult {
                success: false,
                message:
                    "エクスポート失敗: cannot write /x/a.json: Permission denied (os error 13)"
                        .to_owned(),
            })
        );
        assert_eq!(
            export_save_status(&failed, FILE, 1).map(|status| status.message),
            Some(
                "Export failed: cannot write /x/a.json: Permission denied (os error 13)".to_owned()
            )
        );
    }

    type Seen = Rc<RefCell<Vec<Result<SaveOutcome, String>>>>;

    fn recorder() -> (Seen, impl FnOnce(Result<SaveOutcome, String>) + 'static) {
        let seen = Seen::default();
        let sink = Rc::clone(&seen);
        (seen, move |outcome| sink.borrow_mut().push(outcome))
    }

    #[test]
    fn a_finished_save_reports_at_once() {
        // The web download is done inside `save_text_file`: V1 reported in the same click.
        for outcome in [
            Ok(SaveOutcome::Saved { path: None }),
            Err("document.body is not available".to_owned()),
        ] {
            let (seen, on_saved) = recorder();
            let mut spawned = false;
            when_saved(
                future::ready(outcome.clone()).boxed_local(),
                on_saved,
                |_| spawned = true,
            );
            assert_eq!(*seen.borrow(), vec![outcome]);
            assert!(!spawned);
        }
    }

    #[test]
    fn a_pending_save_reports_only_once_the_dialog_closed() {
        for outcome in [
            Ok(SaveOutcome::Saved {
                path: Some(format!("/home/u/Downloads/{FILE}")),
            }),
            Ok(SaveOutcome::Cancelled),
            Err("cannot write /x/a.json: No space left on device (os error 28)".to_owned()),
        ] {
            let (dialog, pending) = oneshot::channel::<Result<SaveOutcome, String>>();
            let (seen, on_saved) = recorder();
            let mut task = None;
            when_saved(
                async move { pending.await.expect("dialog result") }.boxed_local(),
                on_saved,
                |spawned| task = Some(spawned),
            );
            let mut task = task.expect("a task waits for the dialog");
            assert!((&mut task).now_or_never().is_none(), "dialog still open");
            assert!(seen.borrow().is_empty(), "nothing before the dialog closed");
            dialog.send(outcome.clone()).expect("task alive");
            futures::executor::block_on(task);
            assert_eq!(*seen.borrow(), vec![outcome]);
        }
    }

    #[test]
    fn backlog_entries_follow_v1_rules() {
        let user = backlog_entry("user", "  [SMILE] hi ").expect("entry");
        assert_eq!(
            (
                user.role.as_str(),
                user.name_color.as_str(),
                user.message.as_str()
            ),
            ("user", "#66CCFF", "hi")
        );
        let kurisu = backlog_entry("Kurisu", "[BLUSH] べ、別に！").expect("entry");
        assert_eq!(
            (kurisu.role.as_str(), kurisu.name_color.as_str()),
            ("kurisu", "#FF6666")
        );
        let other = backlog_entry("System", "[note] text [WINK]").expect("entry");
        assert_eq!(
            (
                other.role.as_str(),
                other.name_color.as_str(),
                other.message.as_str()
            ),
            ("system", "#808080", "text")
        );
        assert_eq!(backlog_entry("user", "  [ANGRY]  "), None);
    }

    #[test]
    fn export_needs_a_non_system_message() {
        let system_only = vec![ChatMessage::new(ChatRole::System, "persona")];
        assert_eq!(
            prepare_export(&system_only, || 0.0, || "2026-11-16T12:34:56.789Z".into()),
            None
        );
        let mut stamps = vec![
            "2026-11-16T12:34:57.001Z".to_owned(),
            "2026-11-16T12:34:56.789Z".to_owned(),
        ];
        let history = vec![
            ChatMessage::new(ChatRole::System, "persona"),
            ChatMessage::new(ChatRole::User, "こんにちは"),
            ChatMessage::new(ChatRole::Assistant, "[SMILE] ……"),
        ];
        let file =
            prepare_export(&history, || 3.0, || stamps.pop().unwrap_or_default()).expect("export");
        assert_eq!(file.file_name, "chat_export_2026-11-16T12-34-56.json");
        assert_eq!(
            file.json,
            "{\n  \"version\": 1,\n  \"exportedAt\": \"2026-11-16T12:34:57.001Z\",\n  \"turnCount\": 3,\n  \"messages\": [\n    {\n      \"role\": \"user\",\n      \"content\": \"こんにちは\"\n    },\n    {\n      \"role\": \"assistant\",\n      \"content\": \"[SMILE] ……\"\n    }\n  ]\n}"
        );
    }

    #[test]
    fn messages_per_language() {
        assert_eq!(export_no_data_message(0), "会話データがありません");
        assert_eq!(export_no_data_message(1), "No conversation data available");
        assert_eq!(
            export_success_message(1, "a.json"),
            "Export complete: a.json"
        );
        assert_eq!(
            export_failed_message(0, "Error: x"),
            "エクスポート失敗: Error: x"
        );
        assert_eq!(import_invalid_message(1), "Invalid file format");
        assert_eq!(import_success_message(0, 12), "インポート完了: 12 ターン");
        assert_eq!(import_success_message(1, 2), "Import complete: 2 turns");
        // Out-of-range language index → the Japanese dictionary (V1 `translations[i] || ja`).
        assert_eq!(import_success_message(99, 1), "インポート完了: 1 ターン");
    }

    #[test]
    fn import_validation_and_backlog() {
        let invalid = prepare_import("[1,2]", 1).expect_err("array root");
        assert_eq!(
            (invalid.success, invalid.message.as_str()),
            (false, "Invalid file format")
        );
        let invalid = prepare_import(r#"{"messages":[{"role":"user","content":1}]}"#, 0)
            .expect_err("bad entry");
        assert_eq!(invalid.message, "無効なファイル形式です");
        let failed = prepare_import("xyz", 0).expect_err("not json");
        assert!(
            failed.message.starts_with("インポート失敗: SyntaxError: "),
            "{}",
            failed.message
        );

        let raw = r#"{"version":1,"turnCount":2.5,"messages":[
            {"role":"system","content":"p"},
            {"role":"USER","content":"[x] hello"},
            {"role":"kurisu","content":"[SMILE] 一つ目。二つ目！"},
            {"role":"bob","content":"hey"}]}"#;
        let prepared = prepare_import(raw, 0).expect("valid");
        assert_eq!(prepared.turn_count, 2.5);
        assert_eq!(prepared.message, "インポート完了: 4 ターン");
        let roles: Vec<_> = prepared.messages.iter().map(|m| m.role).collect();
        assert_eq!(
            roles,
            vec![
                ChatRole::System,
                ChatRole::Assistant,
                ChatRole::Assistant,
                ChatRole::Assistant
            ]
        );
        let backlog: Vec<_> = prepared
            .backlog
            .iter()
            .map(|e| (e.role.as_str(), e.name_color.as_str(), e.message.as_str()))
            .collect();
        assert_eq!(
            backlog,
            vec![
                ("user", "#66CCFF", "hello"),
                ("kurisu", "#FF6666", "一つ目。"),
                ("kurisu", "#FF6666", "二つ目！"),
                ("bob", "#808080", "hey"),
            ]
        );
        let no_turns = prepare_import(r#"{"messages":[],"turnCount":"3"}"#, 1).expect("valid");
        assert_eq!(
            (no_turns.turn_count, no_turns.message.as_str()),
            (0.0, "Import complete: 0 turns")
        );
    }
}
