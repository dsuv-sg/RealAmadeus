//! Platform commands for the UI's desktop `PlatformShell`: fullscreen, quit, external links and
//! native file dialogs. Names and shapes are documented in `lib.rs` and `docs/DESKTOP_IPC.md`.
//!
//! Unlike the backend IPC commands, these reject the invoke promise with a message string when
//! something fails (a cancelled dialog is not a failure).

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder, FilePath};

use crate::shell;

/// Largest file `open_text_file` reads (the result travels over IPC as one JSON string).
const MAX_OPEN_TEXT_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Used when the suggested name is empty after sanitizing.
const FALLBACK_FILE_STEM: &str = "amadeus_export";

/// Result of `save_text_file`: `{ "saved": false }` when the dialog was cancelled,
/// `{ "saved": true, "path": "…" }` after the file was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTextFileResult {
    pub saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// `window_set_fullscreen { fullscreen }` — native window fullscreen (not the DOM API). The
/// change is reported back through the `amadeus://fullscreen-changed` event.
#[tauri::command]
pub async fn window_set_fullscreen(window: WebviewWindow, fullscreen: bool) -> Result<(), String> {
    window
        .set_fullscreen(fullscreen)
        .map_err(|err| format!("cannot change the fullscreen mode: {err}"))
}

/// `window_is_fullscreen` — current native fullscreen state of the calling window (on Linux the
/// state GTK reports, which includes changes made by the window manager).
#[tauri::command]
pub async fn window_is_fullscreen(window: WebviewWindow) -> Result<bool, String> {
    shell::is_fullscreen(&window).map_err(|err| format!("cannot read the fullscreen mode: {err}"))
}

/// `app_quit` — leaves fullscreen (so the windowed size/position are what gets saved) and exits.
#[tauri::command]
pub async fn app_quit(app: AppHandle) {
    shell::quit(app).await;
}

/// `open_external { url }` — opens an http(s) URL in the default browser.
#[tauri::command]
pub async fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let url = shell::external_http_url(&url)?;
    shell::open_in_browser(&app, url).await
}

/// `save_text_file { suggestedName, contents, mime? }` — native save dialog, then writes
/// `contents` as UTF-8. `mime` (e.g. `application/json`) picks the file-type filter and the
/// extension when the suggested name has none.
#[tauri::command]
pub async fn save_text_file(
    window: WebviewWindow,
    suggested_name: String,
    contents: String,
    mime: Option<String>,
) -> Result<SaveTextFileResult, String> {
    let (file_name, extension) = suggested_file_name(&suggested_name, mime.as_deref());
    let mut dialog = window
        .dialog()
        .file()
        .set_parent(&window)
        .set_file_name(&file_name)
        .set_can_create_directories(true);
    if let Some(dir) = default_directory(&window) {
        dialog = dialog.set_directory(dir);
    }
    if let Some(ext) = &extension {
        dialog = dialog.add_filter(filter_label(ext), &[ext.as_str()]);
    }

    let Some(path) = pick(dialog, Picker::Save).await? else {
        return Ok(SaveTextFileResult {
            saved: false,
            path: None,
        });
    };
    let target = path.clone();
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&target, contents.as_bytes()))
        .await
        .map_err(|err| format!("cannot write {}: {err}", path.display()))?
        .map_err(|err| format!("cannot write {}: {err}", path.display()))?;
    log::info!("saved {}", path.display());
    Ok(SaveTextFileResult {
        saved: true,
        path: Some(path.to_string_lossy().into_owned()),
    })
}

/// `open_text_file { extensions? }` — native open dialog (optionally filtered, e.g.
/// `["json"]`), then reads the file as UTF-8 text like the browser's `FileReader.readAsText`
/// (BOM stripped, invalid sequences replaced). `null` when the dialog was cancelled.
#[tauri::command]
pub async fn open_text_file(
    window: WebviewWindow,
    extensions: Option<Vec<String>>,
) -> Result<Option<String>, String> {
    let extensions = normalize_extensions(extensions.unwrap_or_default());
    let mut dialog = window.dialog().file().set_parent(&window);
    if let Some(dir) = default_directory(&window) {
        dialog = dialog.set_directory(dir);
    }
    if !extensions.is_empty() {
        let label = extensions
            .iter()
            .map(|e| filter_label(e))
            .collect::<Vec<_>>()
            .join(" / ");
        let refs: Vec<&str> = extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(label, &refs);
    }

    let Some(path) = pick(dialog, Picker::Open).await? else {
        return Ok(None);
    };
    let source = path.clone();
    let text = tauri::async_runtime::spawn_blocking(move || read_text_file(&source))
        .await
        .map_err(|err| format!("cannot read {}: {err}", path.display()))??;
    log::info!("opened {}", path.display());
    Ok(Some(text))
}

enum Picker {
    Save,
    Open,
}

/// Shows the dialog without blocking a runtime thread; `None` when cancelled.
async fn pick(
    dialog: FileDialogBuilder<tauri::Wry>,
    picker: Picker,
) -> Result<Option<PathBuf>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<FilePath>>();
    let reply = move |path: Option<FilePath>| {
        let _ = tx.send(path);
    };
    match picker {
        Picker::Save => dialog.save_file(reply),
        Picker::Open => dialog.pick_file(reply),
    }
    // A dropped sender means the dialog could not be shown; treat it like a cancel.
    match rx.await.ok().flatten() {
        Some(path) => path
            .into_path()
            .map(Some)
            .map_err(|err| format!("unsupported file location: {err}")),
        None => Ok(None),
    }
}

/// Downloads (where the web version's exports end up), else Documents.
fn default_directory(window: &WebviewWindow) -> Option<PathBuf> {
    let paths = window.path();
    [paths.download_dir(), paths.document_dir()]
        .into_iter()
        .flatten()
        .find(|dir| dir.is_dir())
}

/// Sanitized file name for the save dialog plus its (lower-case) extension. When the name has
/// no extension, one derived from `mime` is appended.
fn suggested_file_name(suggested: &str, mime: Option<&str>) -> (String, Option<String>) {
    let mut name = sanitize_file_name(suggested);
    if let Some(ext) = extension_of(&name) {
        return (name, Some(ext));
    }
    let ext = mime.and_then(extension_for_mime).map(str::to_owned);
    if let Some(ext) = &ext {
        name.push('.');
        name.push_str(ext);
    }
    (name, ext)
}

/// Removes path separators, characters Windows forbids and control characters.
fn sanitize_file_name(raw: &str) -> String {
    let replaced: String = raw
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = replaced.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        FALLBACK_FILE_STEM.to_owned()
    } else {
        trimmed.chars().take(200).collect()
    }
}

fn extension_of(name: &str) -> Option<String> {
    let ext = Path::new(name).extension()?.to_str()?;
    is_plain_extension(ext).then(|| ext.to_ascii_lowercase())
}

fn is_plain_extension(ext: &str) -> bool {
    (1..=16).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_alphanumeric())
}

fn extension_for_mime(mime: &str) -> Option<&'static str> {
    let essence = mime
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    Some(match essence.as_str() {
        "application/json" | "text/json" => "json",
        "text/plain" => "txt",
        "text/markdown" => "md",
        "text/csv" => "csv",
        "text/html" => "html",
        "application/xml" | "text/xml" => "xml",
        _ => return None,
    })
}

fn filter_label(ext: &str) -> String {
    match ext {
        "json" => "JSON".to_owned(),
        "txt" => "Text".to_owned(),
        "md" => "Markdown".to_owned(),
        other => other.to_ascii_uppercase(),
    }
}

/// `"*.json"`, `".json"`, `"JSON"` → `"json"`; anything else is dropped. Order kept, no duplicates.
fn normalize_extensions(raw: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for ext in raw {
        let ext = ext
            .trim()
            .trim_start_matches('*')
            .trim_start_matches('.')
            .to_ascii_lowercase();
        if is_plain_extension(&ext) && !out.contains(&ext) {
            out.push(ext);
        }
    }
    out
}

fn read_text_file(path: &Path) -> Result<String, String> {
    let meta =
        std::fs::metadata(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a file", path.display()));
    }
    if meta.len() > MAX_OPEN_TEXT_FILE_BYTES {
        return Err(format!(
            "{} is too large ({} bytes, the limit is {} MiB)",
            path.display(),
            meta.len(),
            MAX_OPEN_TEXT_FILE_BYTES / (1024 * 1024)
        ));
    }
    let bytes =
        std::fs::read(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    Ok(decode_text(bytes))
}

/// UTF-8 decode like `FileReader.readAsText`: a leading BOM is removed and invalid sequences
/// become U+FFFD.
fn decode_text(bytes: Vec<u8>) -> String {
    let mut text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(err) => String::from_utf8_lossy(err.as_bytes()).into_owned(),
    };
    if text.starts_with('\u{feff}') {
        text.drain(..'\u{feff}'.len_utf8());
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_result_wire_format() {
        let cancelled = SaveTextFileResult {
            saved: false,
            path: None,
        };
        assert_eq!(
            serde_json::to_string(&cancelled).unwrap(),
            r#"{"saved":false}"#
        );
        let saved = SaveTextFileResult {
            saved: true,
            path: Some("/tmp/a.json".into()),
        };
        assert_eq!(
            serde_json::to_string(&saved).unwrap(),
            r#"{"saved":true,"path":"/tmp/a.json"}"#
        );
    }

    #[test]
    fn file_names_are_sanitized() {
        assert_eq!(
            sanitize_file_name("chat_2026-11-16T12:00:00.json"),
            "chat_2026-11-16T12_00_00.json"
        );
        assert_eq!(sanitize_file_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(sanitize_file_name(" .. "), FALLBACK_FILE_STEM);
        assert_eq!(sanitize_file_name("a\u{0}b\nc"), "a_b_c");
        assert_eq!(sanitize_file_name("会話ログ.txt"), "会話ログ.txt");
        assert_eq!(sanitize_file_name(&"x".repeat(500)).chars().count(), 200);
    }

    #[test]
    fn extension_from_name_or_mime() {
        assert_eq!(
            suggested_file_name("log.JSON", Some("text/plain")),
            ("log.JSON".into(), Some("json".into()))
        );
        assert_eq!(
            suggested_file_name("settings", Some("application/json; charset=utf-8")),
            ("settings.json".into(), Some("json".into()))
        );
        assert_eq!(suggested_file_name("notes", None), ("notes".into(), None));
        assert_eq!(
            suggested_file_name("notes", Some("application/x-unknown")),
            ("notes".into(), None)
        );
    }

    #[test]
    fn extensions_are_normalized() {
        let raw = vec![
            "*.json".into(),
            ".TXT".into(),
            "json".into(),
            "".into(),
            "a/b".into(),
            "md".into(),
        ];
        assert_eq!(normalize_extensions(raw), vec!["json", "txt", "md"]);
    }

    #[test]
    fn text_is_decoded_like_file_reader() {
        assert_eq!(
            decode_text("\u{feff}{\"a\":1}".as_bytes().to_vec()),
            "{\"a\":1}"
        );
        assert_eq!(decode_text(b"ok".to_vec()), "ok");
        assert_eq!(decode_text(vec![b'a', 0xff, b'b']), "a\u{fffd}b");
    }
}
