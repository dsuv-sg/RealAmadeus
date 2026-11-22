//! Platform shell: fullscreen, file save/open, shutdown, external links — DOM APIs on the
//! web, Tauri IPC on desktop. OWNED BY: ui-foundation.
//!
//! ```ignore
//! let shell = platform();                       // Copy + Send + Sync: capture it anywhere
//! shell.set_fullscreen(true);                   // V1 requestFullscreen / window_set_fullscreen
//! let listener = shell.on_fullscreen_change(|full| set_app_setting_int("Config_ScreenMode", if full { 0 } else { 1 }));
//! on_cleanup(move || listener.remove());
//!
//! let save = shell.save_text_file(&file_name, JSON_MIME, &json);   // starts immediately
//! spawn_local(async move { match save.await { /* … */ } });
//!
//! // in the click handler (user activation), then await:
//! let pick = shell.pick_text_file(IMPORT_ACCEPT);
//! spawn_local(async move { if let Ok(Some(raw)) = pick.await { /* … */ } });
//! ```
//!
//! Desktop differences (DESKTOP_IPC §3–§4): native window fullscreen instead of the DOM API
//! (state reported by `amadeus://fullscreen-changed`, initial state read once; Escape leaves it
//! like a browser's Escape leaves DOM fullscreen, see [`escape_leaves_fullscreen`]), a native
//! save dialog (which can be cancelled) and open dialog, `app_quit` instead of V1's
//! close/back/redirect, and `open_external` for links.

pub mod tauri;
pub mod web;

use std::cell::{Cell, RefCell};

use amadeus_core::types::{ConnectionId, Emitter};
use futures::future::{self, FutureExt, LocalBoxFuture};
use js_sys::Reflect;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};

pub use web::{read_file_text, SHUTDOWN_REDIRECT_URL};

/// MIME type of V1 conversation exports (`new Blob([json], { type: "application/json" })`).
pub const JSON_MIME: &str = "application/json";

/// `accept` of V1's conversation import input (`<input type="file" accept=".json">`).
pub const IMPORT_ACCEPT: &str = ".json";

/// Which shell the page runs in. Stateless and `Copy + Send + Sync`; get it with [`platform`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlatformShell {
    /// A browser tab (DOM APIs).
    Web,
    /// The Tauri desktop webview (IPC commands of `docs/DESKTOP_IPC.md`).
    Desktop,
}

/// The current platform: [`PlatformShell::Desktop`] inside Tauri (`window.__TAURI__`),
/// [`PlatformShell::Web`] otherwise.
pub fn platform() -> PlatformShell {
    if crate::env::is_tauri() {
        PlatformShell::Desktop
    } else {
        PlatformShell::Web
    }
}

/// Result of [`PlatformShell::save_text_file`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    /// Written. `path` is the chosen file on desktop; `None` on the web (the browser's download
    /// manager decides).
    Saved { path: Option<String> },
    /// The desktop save dialog was cancelled (never happens on the web).
    Cancelled,
}

/// Handle of an [`PlatformShell::on_fullscreen_change`] subscription.
#[derive(Debug)]
#[must_use = "call .remove() to stop listening"]
pub struct FullscreenListener(ConnectionId);

impl FullscreenListener {
    /// Stops calling the handler.
    pub fn remove(self) {
        FULLSCREEN.with(|hub| {
            hub.changed.disconnect(self.0);
        });
    }
}

/// A JS event listener kept alive by Rust.
type JsListener = Closure<dyn FnMut(JsValue)>;

/// Page-wide fullscreen state: one native listener fanned out to the Rust handlers.
#[derive(Default)]
struct FullscreenHub {
    changed: Emitter<bool>,
    installed: Cell<bool>,
    /// Last native state reported by the desktop shell.
    desktop_fullscreen: Cell<bool>,
    /// A desktop fullscreen event arrived (newer than the initial `window_is_fullscreen`).
    desktop_event_seen: Cell<bool>,
    _listener: RefCell<Option<JsListener>>,
    /// Desktop: the capture-phase `keydown` listener that leaves fullscreen on Escape.
    _escape_listener: RefCell<Option<JsListener>>,
}

/// Desktop: whether a `keydown` leaves native fullscreen, as a browser's Escape leaves DOM
/// fullscreen in V1 (spec-app-part4 §7.4, KEY-011: the app itself does nothing with Escape, the
/// browser exits fullscreen and `fullscreenchange` syncs `Config_ScreenMode`). Any Escape while
/// the window is fullscreen counts, whatever has the focus and whichever modifiers are held, except
/// one that an IME composition uses (it cancels the composition instead).
pub fn escape_leaves_fullscreen(key: &str, is_composing: bool, fullscreen: bool) -> bool {
    fullscreen && !is_composing && key == "Escape"
}

/// Desktop: installs the Escape exit of [`escape_leaves_fullscreen`]. The listener sits on
/// `window` in the capture phase, so it runs before every other `keydown` listener of the page,
/// and it consumes the key press (`preventDefault` + `stopImmediatePropagation`) because a browser
/// never delivers the Escape that leaves fullscreen to the page: overlays and menus (the
/// achievement panel, the mini game, ChatPanel's idle reset) must not react to it as well. The
/// shell then reports the change through `amadeus://fullscreen-changed` like any other.
fn install_escape_exit() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let listener = JsListener::new(|event: JsValue| {
        let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() else {
            return;
        };
        let fullscreen = FULLSCREEN.with(|hub| hub.desktop_fullscreen.get());
        if escape_leaves_fullscreen(&event.key(), event.is_composing(), fullscreen) {
            event.prevent_default();
            event.stop_immediate_propagation();
            PlatformShell::Desktop.exit_fullscreen();
        }
    });
    match window.add_event_listener_with_callback_and_bool(
        "keydown",
        listener.as_ref().unchecked_ref(),
        true,
    ) {
        Ok(()) => FULLSCREEN.with(|hub| *hub._escape_listener.borrow_mut() = Some(listener)),
        Err(error) => log::warn!(
            "cannot listen for Escape: {}",
            crate::env::js_to_string(&error)
        ),
    }
}

thread_local! {
    static FULLSCREEN: FullscreenHub = FullscreenHub::default();
}

fn emit_fullscreen(fullscreen: bool) {
    FULLSCREEN.with(|hub| hub.changed.emit(&fullscreen));
}

fn set_desktop_fullscreen(fullscreen: bool, notify_unchanged: bool) {
    let changed = FULLSCREEN.with(|hub| hub.desktop_fullscreen.replace(fullscreen) != fullscreen);
    if changed || notify_unchanged {
        emit_fullscreen(fullscreen);
    }
}

fn ensure_fullscreen_watcher(shell: PlatformShell) {
    if FULLSCREEN.with(|hub| hub.installed.replace(true)) {
        return;
    }
    match shell {
        PlatformShell::Web => {
            let listener = JsListener::new(|_event: JsValue| emit_fullscreen(web::is_fullscreen()));
            web::add_fullscreen_listener(listener.as_ref().unchecked_ref());
            FULLSCREEN.with(|hub| *hub._listener.borrow_mut() = Some(listener));
        }
        PlatformShell::Desktop => {
            let listener = JsListener::new(|event: JsValue| {
                if let Some(fullscreen) = tauri::fullscreen_from_event(&event) {
                    FULLSCREEN.with(|hub| hub.desktop_event_seen.set(true));
                    // Every event is a real transition: forward it even if the cache agreed.
                    set_desktop_fullscreen(fullscreen, true);
                }
            });
            match tauri::listen(
                tauri::FULLSCREEN_CHANGED_EVENT,
                listener.as_ref().unchecked_ref(),
            ) {
                Ok(promise) => spawn_local(async move {
                    if let Err(error) = JsFuture::from(promise).await {
                        log::warn!(
                            "listen({}) failed: {}",
                            tauri::FULLSCREEN_CHANGED_EVENT,
                            crate::env::js_to_string(&error)
                        );
                    }
                }),
                Err(error) => log::warn!(
                    "listen({}) failed: {}",
                    tauri::FULLSCREEN_CHANGED_EVENT,
                    crate::env::js_to_string(&error)
                ),
            }
            FULLSCREEN.with(|hub| *hub._listener.borrow_mut() = Some(listener));
            install_escape_exit();
            // The event only reports changes: read the initial state once.
            spawn_local(async {
                match tauri::invoke_async(tauri::commands::WINDOW_IS_FULLSCREEN, &tauri::args(&[]))
                    .await
                {
                    Ok(value) => {
                        let stale = FULLSCREEN.with(|hub| hub.desktop_event_seen.get());
                        if let (Some(fullscreen), false) = (value.as_bool(), stale) {
                            set_desktop_fullscreen(fullscreen, false);
                        }
                    }
                    Err(error) => log::warn!(
                        "window_is_fullscreen failed: {}",
                        crate::env::js_to_string(&error)
                    ),
                }
            });
        }
    }
}

/// Fire-and-forget desktop command; failures are logged.
fn invoke_logged(cmd: &'static str, args: JsValue) {
    match tauri::invoke(cmd, &args) {
        Ok(promise) => spawn_local(async move {
            if let Err(error) = JsFuture::from(promise).await {
                log::warn!("{cmd} failed: {}", crate::env::js_to_string(&error));
            }
        }),
        Err(error) => log::warn!("{cmd} failed: {}", crate::env::js_to_string(&error)),
    }
}

impl PlatformShell {
    /// True on the desktop build.
    pub fn is_desktop(self) -> bool {
        self == PlatformShell::Desktop
    }

    /// Web: `!!document.fullscreenElement`. Desktop: the last native state the shell reported
    /// (`false` until the initial `window_is_fullscreen` answer arrives).
    pub fn is_fullscreen(self) -> bool {
        match self {
            PlatformShell::Web => web::is_fullscreen(),
            PlatformShell::Desktop => {
                ensure_fullscreen_watcher(self);
                FULLSCREEN.with(|hub| hub.desktop_fullscreen.get())
            }
        }
    }

    /// Enter fullscreen. Web: V1 `if (!document.fullscreenElement)
    /// document.documentElement.requestFullscreen().catch(() => {})` (no-op where the API is
    /// missing). Desktop: `window_set_fullscreen { fullscreen: true }` (no user gesture needed).
    pub fn request_fullscreen(self) {
        match self {
            PlatformShell::Web => web::request_fullscreen(),
            PlatformShell::Desktop => {
                ensure_fullscreen_watcher(self);
                invoke_logged(
                    tauri::commands::WINDOW_SET_FULLSCREEN,
                    tauri::args(&[("fullscreen", JsValue::TRUE)]),
                );
            }
        }
    }

    /// Leave fullscreen. Web: V1 `if (document.fullscreenElement)
    /// document.exitFullscreen().catch(() => {})`. Desktop: `window_set_fullscreen { false }`.
    pub fn exit_fullscreen(self) {
        match self {
            PlatformShell::Web => web::exit_fullscreen(),
            PlatformShell::Desktop => {
                ensure_fullscreen_watcher(self);
                invoke_logged(
                    tauri::commands::WINDOW_SET_FULLSCREEN,
                    tauri::args(&[("fullscreen", JsValue::FALSE)]),
                );
            }
        }
    }

    /// [`request_fullscreen`](Self::request_fullscreen) or [`exit_fullscreen`](Self::exit_fullscreen).
    pub fn set_fullscreen(self, fullscreen: bool) {
        if fullscreen {
            self.request_fullscreen();
        } else {
            self.exit_fullscreen();
        }
    }

    /// Calls `handler(is_fullscreen)` on every fullscreen change, whatever the cause. Web:
    /// `document` `fullscreenchange` (Escape, browser UI, native F11). Desktop:
    /// `amadeus://fullscreen-changed` (F11 / menu / CONFIG, Escape while fullscreen — see
    /// [`escape_leaves_fullscreen`] — and window-manager changes; plus once at startup if the
    /// window already is fullscreen). V1's root wrote `Config_ScreenMode = full ? 0 : 1` from it.
    /// On desktop the first fullscreen call of the page (normally this one, from the app root)
    /// installs the event listener and the Escape exit.
    pub fn on_fullscreen_change(self, handler: impl Fn(bool) + 'static) -> FullscreenListener {
        ensure_fullscreen_watcher(self);
        FullscreenListener(
            FULLSCREEN.with(|hub| hub.changed.connect(move |full: &bool| handler(*full))),
        )
    }

    /// Saves `contents` as a text file. Web: V1's Blob + `<a download>` (synchronous, inside
    /// this call). Desktop: `save_text_file` native dialog (starts inside this call).
    /// Errors are messages (V1 showed `export_failed` + `String(error)`).
    pub fn save_text_file(
        self,
        file_name: &str,
        mime: &str,
        contents: &str,
    ) -> LocalBoxFuture<'static, Result<SaveOutcome, String>> {
        match self {
            PlatformShell::Web => future::ready(
                web::save_text_file(file_name, mime, contents)
                    .map(|()| SaveOutcome::Saved { path: None }),
            )
            .boxed_local(),
            PlatformShell::Desktop => {
                let args = tauri::args(&[
                    ("suggestedName", JsValue::from_str(file_name)),
                    ("contents", JsValue::from_str(contents)),
                    ("mime", JsValue::from_str(mime)),
                ]);
                let pending = tauri::invoke(tauri::commands::SAVE_TEXT_FILE, &args);
                async move {
                    let reply = JsFuture::from(pending.map_err(|e| crate::env::js_to_string(&e))?)
                        .await
                        .map_err(|e| crate::env::js_to_string(&e))?;
                    let saved = Reflect::get(&reply, &"saved".into())
                        .ok()
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if saved {
                        let path = Reflect::get(&reply, &"path".into())
                            .ok()
                            .and_then(|v| v.as_string());
                        Ok(SaveOutcome::Saved { path })
                    } else {
                        Ok(SaveOutcome::Cancelled)
                    }
                }
                .boxed_local()
            }
        }
    }

    /// Lets the user pick a text file and resolves to its contents (`Ok(None)` = cancelled).
    /// Web: hidden `<input type="file" accept=…>` (see [`web::pick_text_file`]; call this
    /// directly in the click handler). Desktop: `open_text_file` native dialog filtered by the
    /// extensions of `accept`. The text is decoded like V1's `file.text()`.
    pub fn pick_text_file(
        self,
        accept: &str,
    ) -> LocalBoxFuture<'static, Result<Option<String>, String>> {
        match self {
            PlatformShell::Web => web::pick_text_file(accept),
            PlatformShell::Desktop => {
                let extensions = tauri::accept_to_extensions(accept);
                let args = if extensions.is_empty() {
                    tauri::args(&[])
                } else {
                    tauri::args(&[(
                        "extensions",
                        tauri::string_array(extensions.iter().map(String::as_str)),
                    )])
                };
                let pending = tauri::invoke(tauri::commands::OPEN_TEXT_FILE, &args);
                async move {
                    let reply = JsFuture::from(pending.map_err(|e| crate::env::js_to_string(&e))?)
                        .await
                        .map_err(|e| crate::env::js_to_string(&e))?;
                    Ok(reply.as_string())
                }
                .boxed_local()
            }
        }
    }

    /// SHUT DOWN. Web: V1's `window.close()` → 100 ms → `history.back()` (if any) → 300 ms →
    /// `location.href = "https://dsuv.elvelt.com/amadeus"`. Desktop: `app_quit` (leaves
    /// fullscreen, saves the window geometry, exits). The caller closes its overlays first.
    pub fn shutdown(self) {
        match self {
            PlatformShell::Web => web::shutdown(),
            PlatformShell::Desktop => invoke_logged(tauri::commands::APP_QUIT, tauri::args(&[])),
        }
    }

    /// Opens an external http(s) URL. Web: `window.open(url, "_blank", "noopener,noreferrer")`.
    /// Desktop: the default browser through `open_external` (non-http(s) URLs are refused).
    pub fn open_external(self, url: &str) {
        match self {
            PlatformShell::Web => web::open_external(url),
            PlatformShell::Desktop => invoke_logged(
                tauri::commands::OPEN_EXTERNAL,
                tauri::args(&[("url", JsValue::from_str(url))]),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::escape_leaves_fullscreen;

    #[test]
    fn escape_leaves_fullscreen_only_while_fullscreen() {
        assert!(escape_leaves_fullscreen("Escape", false, true));
        // Windowed: Escape keeps its V1 meaning (e.g. it closes the achievement panel).
        assert!(!escape_leaves_fullscreen("Escape", false, false));
        // An IME composition takes the Escape (it cancels the composition).
        assert!(!escape_leaves_fullscreen("Escape", true, true));
        for key in ["F11", "Backspace", "Esc", "escape", "Process", "Enter", " "] {
            assert!(!escape_leaves_fullscreen(key, false, true), "{key}");
        }
    }
}
