//! # RealAmadeus V2 desktop shell (Tauri 2)
//!
//! Hosts the shared WASM frontend (Trunk `apps/web/dist-desktop`, window URL `app`) and answers
//! its backend calls over IPC by calling `amadeus-backend` in-process in
//! `BackendMode::Desktop`: no HTTP server, no CORS, TTS/STT/RAG server URLs such as
//! `http://localhost:5100` resolve on the user's machine, and the FreeAPI provider is forwarded to
//! the hosted server (`config::DEFAULT_REMOTE_API_BASE`, override with the environment variable
//! `AMADEUS_REMOTE_API_BASE`). Backend data lives in `{app_data_dir}/data`.
//!
//! The full frontend reference is `docs/DESKTOP_IPC.md`. JS calls go through
//! `window.__TAURI__.core.invoke(command, args)` (`app.withGlobalTauri`); argument keys are
//! camelCase. Custom commands need no capability entries (no app ACL manifest is defined) and
//! Tauri accepts them only from the app's own origin; Channels need no permission either.
//!
//! ## Backend IPC commands (contract: `amadeus_core::protocol::ipc`)
//!
//! These never reject: failures are [`ApiReply`](amadeus_core::protocol::ApiReply) values, e.g.
//! `{"kind":"json","status":500,"body":{"error":"Internal error (500): …"}}`.
//!
//! | command              | JS args                                                   | resolves to |
//! |----------------------|-----------------------------------------------------------|-------------|
//! | `chat`               | `{ payload }` (ChatRequestPayload JSON)                   | `ApiReply` |
//! | `chat_stream`        | `{ payload, streamId: u32, onEvent: Channel<StreamEvent> }` | `null` once the stream ended or was cancelled; `ApiReply` for a pre-stream error (400, 429, …) |
//! | `chat_stream_cancel` | `{ streamId }`                                            | `null` |
//! | `chat_quota`         | `{}`                                                      | `ApiReply` |
//! | `tts_synthesize`     | `{ body: TtsSynthesizeBody }`                             | `ApiReply` (`binary` audio on success) |
//! | `tts_health`         | `{ body: ServerUrlBody }`                                 | `ApiReply` |
//! | `stt_transcribe`     | `{ fields: SttTranscribeFields, audioBase64, mime }`      | `ApiReply` (400 `Invalid form data` for bad base64) |
//! | `stt_health`         | `{ body: ServerUrlBody }`                                 | `ApiReply` |
//! | `rag_embed`          | `{ body: RagEmbedBody }`                                  | `ApiReply` |
//! | `rag_health`         | `{ body: ServerUrlBody }`                                 | `ApiReply` |
//! | `app_info`           | `{}`                                                      | `AppInfo` `{ version, displayVersion, platform, arch, remoteApiBase }` |
//!
//! `onEvent` receives the V1 SSE objects (`{"type":"token","token":…}`,
//! `{"type":"complete","fullResponse":…}`, `{"type":"error","error":…}`) in order. Give every
//! stream a new `streamId` (an incrementing counter per page load). `chat_stream_cancel` may be
//! sent right after `chat_stream`: if it overtakes it, the stream is cancelled as soon as it
//! starts.
//!
//! A page reload or navigation aborts every backend call the previous page left running, streams
//! included, and Tauri's re-send of an invoke the unloading page abandoned does not run it again
//! (see the `calls` module).
//!
//! ## Platform commands (the UI's desktop `PlatformShell`)
//!
//! These reject the invoke promise with a message string on failure.
//!
//! | command                 | JS args                                                   | resolves to |
//! |-------------------------|-----------------------------------------------------------|-------------|
//! | `window_set_fullscreen` | `{ fullscreen: boolean }`                                 | `null` |
//! | `window_is_fullscreen`  | `{}`                                                      | `boolean` |
//! | `app_quit`              | `{}`                                                      | `null` if it settles at all (leaves fullscreen, then the app exits) |
//! | `open_external`         | `{ url: string }` (http/https only)                       | `null` |
//! | `save_text_file`        | `{ suggestedName: string, contents: string, mime?: string }` | `{ saved: boolean, path?: string }` (`saved: false` = cancelled) |
//! | `open_text_file`        | `{ extensions?: string[] }` (e.g. `["json"]`)            | `string` (UTF-8 text) or `null` (cancelled) |
//!
//! ## Events
//!
//! * `amadeus://fullscreen-changed` with payload `{ fullscreen: boolean }`: the main window
//!   entered or left fullscreen (via `window_set_fullscreen`, the macOS green button / View menu,
//!   a window-manager shortcut on Linux, …). Keep `Config_ScreenMode` (0 = fullscreen,
//!   1 = windowed) in sync with it, and read `window_is_fullscreen` once at startup. Not emitted
//!   while the app exits (`app_quit`, closing the window, restart after an update), so the saved
//!   setting stays the user's choice. On Linux both follow GTK's window-state events, because tao
//!   only knows the state the app last requested.
//!
//! ## Shell behavior
//!
//! * Plugins: single-instance (a second launch focuses this window), log (app log directory; the
//!   app's own records from `info` up, libraries' from `warn` up, panics included; see
//!   `logging`), window-state (size, position, maximized; not fullscreen, see
//!   `shell::window_state_flags`), opener and dialog (Rust APIs), process, and updater with the
//!   cargo feature `updater`.
//! * Capabilities (`capabilities/default.json`, main window, app origin only): `core:default`
//!   only (Tauri's baseline: event listen/emit for the event above, read-only window/app/path
//!   getters). No plugin API is exposed to JS: dialog, log, opener, process, updater and
//!   window-state stay Rust-only, so the app log holds Rust-side records only.
//! * CSP (`app.security.csp` in `tauri.conf.json`): Tauri hashes the inline scripts of
//!   `index.html` (Trunk's `<script type="module">` bootstrap and the Google Fonts loader) at
//!   build time and adds the hashes to `script-src`, so no `'unsafe-inline'` is needed for
//!   scripts. `dangerousDisableAssetCspModification: ["style-src"]` keeps Tauri from adding
//!   nonces to `style-src` (it would for any inline `<style>` in `index.html`), because a nonce
//!   makes browsers ignore `'unsafe-inline'`, which would block every `style="…"` attribute.
//!   Google Fonts (`fonts.googleapis.com` styles, `fonts.gstatic.com` fonts) is allowed for V1's
//!   Noto Serif JP sheet, which that loader adds on the `/app` route (V1: an `@import` in its app
//!   CSS).
//! * Remote pages never load inside the app: navigations away from the exact app origin
//!   (`tauri://localhost`, or `http://tauri.localhost` without a port on Windows), `window.open`
//!   and `target="_blank"` links open http(s) URLs in the default browser instead.
//! * macOS keeps Tauri's default menu (app menu with Quit, Edit with copy/paste, View, Window).

mod calls;
mod config;
mod ipc;
mod logging;
mod platform;
mod shell;
mod state;
mod updater;

use std::panic::AssertUnwindSafe;

use amadeus_backend::{Backend, BackendConfig};
use amadeus_core::version;
use tauri::{webview::PageLoadEvent, AppHandle, Manager, RunEvent};

use crate::state::DesktopState;

// Only needed for its `log` feature (see `Cargo.toml` and `logging`).
use tracing as _;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logging::install_panic_hook();

    let built = tauri::Builder::default()
        // Must be the first plugin: a second launch hands over to the running instance and exits.
        .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
            log::info!("second launch (args {args:?}, cwd {cwd}); focusing the main window");
            shell::focus_main_window(app);
        }))
        .plugin(logging::plugin())
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(shell::window_state_flags())
                .build(),
        )
        // Link clicks are handled by the navigation policy in `shell`, not by injected JS.
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .on_page_load(|webview, payload| {
            // `Started` = the new document committed, before its scripts run. Nothing can receive
            // the previous page's replies or channel events any more.
            if payload.event() == PageLoadEvent::Started {
                if let Some(state) = webview.try_state::<DesktopState>() {
                    let aborted = state.calls().abort_all();
                    // Also forgets the old page's stream ids (the new page counts from 1 again).
                    state.streams().cancel_all();
                    if aborted > 0 {
                        log::info!(
                            "page load: aborted {aborted} backend call(s) of the previous page"
                        );
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            ipc::chat,
            ipc::chat_stream,
            ipc::chat_stream_cancel,
            ipc::chat_quota,
            ipc::tts_synthesize,
            ipc::tts_health,
            ipc::stt_transcribe,
            ipc::stt_health,
            ipc::rag_embed,
            ipc::rag_health,
            ipc::app_info,
            platform::window_set_fullscreen,
            platform::window_is_fullscreen,
            platform::app_quit,
            platform::open_external,
            platform::save_text_file,
            platform::open_text_file,
        ])
        .setup(|app| setup(app.handle()).map_err(Into::into))
        .build(tauri::generate_context!());

    match built {
        Ok(app) => app.run(|_app, event| {
            if let RunEvent::Exit = event {
                log::info!("{} exited", version::PRODUCT_NAME);
            }
        }),
        Err(err) => {
            log::error!("{} failed to start: {err}", version::PRODUCT_NAME);
            eprintln!("{} failed to start: {err}", version::PRODUCT_NAME);
            std::process::exit(1);
        }
    }
}

fn setup(app: &AppHandle) -> anyhow::Result<()> {
    log::info!(
        "{} {} starting ({}/{})",
        version::PRODUCT_NAME,
        version::display_version(),
        ipc::PLATFORM,
        std::env::consts::ARCH
    );
    let remote_api_base = config::remote_api_base();
    let data_dir = config::data_dir(app);
    log::info!("backend data directory: {}", data_dir.display());
    let backend = create_backend(BackendConfig::desktop(data_dir, remote_api_base.clone()));
    app.manage(DesktopState::new(backend, remote_api_base));

    shell::create_main_window(app)?;

    updater::spawn_check(app);
    Ok(())
}

/// Builds the backend inside the async runtime context (its constructor may spawn tasks). A
/// failure or panic does not stop the app: the commands then answer 500 with the reason.
fn create_backend(config: BackendConfig) -> Result<Backend, String> {
    let runtime = tauri::async_runtime::handle();
    let _context = runtime.inner().enter();
    match std::panic::catch_unwind(AssertUnwindSafe(|| Backend::new(config))) {
        Ok(Ok(backend)) => Ok(backend),
        Ok(Err(err)) => {
            log::error!("backend initialization failed: {err:#}");
            Err(format!("{err:#}"))
        }
        Err(payload) => {
            let message = logging::panic_message(payload.as_ref());
            log::error!("backend initialization panicked: {message}");
            Err(format!("initialization panicked: {message}"))
        }
    }
}
