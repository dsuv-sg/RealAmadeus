//! Backend IPC commands. Names, argument names and return types are fixed by
//! [`amadeus_core::protocol::ipc`]. Each command runs the same `amadeus-backend` function as the
//! web server's HTTP route (with [`ClientMeta::local`]), so status codes and JSON bodies match.
//!
//! These commands never reject the invoke promise: every failure is an [`ApiReply`], 500 for
//! internal errors. Backend calls run on their own task, so a backend panic also becomes a 500
//! reply (or a terminal `error` event mid-stream) instead of a promise that never settles.
//!
//! Every backend call belongs to the page that made it ([`crate::calls`]): a page reload aborts
//! it, and Tauri's re-send of an invoke that the unloading page abandoned returns the original's
//! outcome instead of running the call a second time. The [`Delivery`] argument is filled in by
//! the shell, not by the frontend.

use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use amadeus_backend::{Backend, ChatStreamStart, ClientMeta};
use amadeus_core::{
    protocol::{
        b64,
        ipc::{self, AppInfo},
        ApiReply, RagEmbedBody, ServerUrlBody, StreamEvent, SttTranscribeFields, TtsSynthesizeBody,
    },
    version,
};
use futures::StreamExt;
use serde::Serialize;
use tauri::{ipc::Channel, AppHandle, Manager};
use tokio::task::JoinError;

use crate::{
    calls::{
        spawn_task, Admission, CallKey, Delivery, NewCall, Outcome, SharedOutcome, RESEND_WINDOW,
    },
    logging::panic_message,
    state::{DesktopState, StreamGuard},
};

/// `windows` | `macos` | `linux` (the other desktop unixes use the GTK port and report `linux`).
pub const PLATFORM: &str = if cfg!(target_os = "windows") {
    "windows"
} else if cfg!(target_os = "macos") {
    "macos"
} else {
    "linux"
};

/// 500 reply for failures inside the desktop shell or backend. The text contains "500" so the
/// UI's error classifier (V1 `handleApiError`) shows its server-error message.
fn internal_error(detail: impl std::fmt::Display) -> ApiReply {
    ApiReply::error(500, format!("Internal error (500): {detail}"))
}

fn desktop_state(app: &AppHandle) -> Result<tauri::State<'_, DesktopState>, ApiReply> {
    app.try_state::<DesktopState>()
        .ok_or_else(|| internal_error("the desktop backend is not initialized"))
}

fn backend(state: &DesktopState) -> Result<Backend, ApiReply> {
    state
        .backend()
        .map_err(|err| internal_error(format!("the desktop backend is unavailable: {err}")))
}

fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ApiReply> {
    serde_json::to_vec(value)
        .map_err(|err| internal_error(format!("cannot encode the request body: {err}")))
}

/// Why a call's task ended without a result. Backend tasks are only aborted by a page reload
/// (`chat_stream` handles its own cancellation first).
fn join_error_detail(err: JoinError) -> String {
    if err.is_panic() {
        format!(
            "the backend panicked: {}",
            panic_message(err.into_panic().as_ref())
        )
    } else {
        "the request was cancelled because the page was reloaded".to_owned()
    }
}

/// Awaits the outcome of the call this invoke is Tauri's re-send of.
async fn replay(command: &str, outcome: SharedOutcome) -> Outcome {
    log::info!(
        "{command}: Tauri re-sent a running invoke through postMessage (its IPC fetch failed, \
         usually because the page is unloading); returning the original call's outcome instead \
         of running it again"
    );
    outcome.await
}

/// Runs one backend call as a call of the current page ([`crate::calls`]); a panic or an abort
/// becomes a 500 reply. A re-send gets the original call's reply.
async fn page_call<F, Fut>(
    app: &AppHandle,
    command: &'static str,
    delivery: Delivery,
    key: CallKey,
    call: F,
) -> ApiReply
where
    F: FnOnce(Backend) -> Fut,
    Fut: Future<Output = ApiReply> + Send + 'static,
{
    let state = match desktop_state(app) {
        Ok(state) => state,
        Err(reply) => return reply,
    };
    let outcome = match state.calls().admit(delivery, key) {
        Admission::Resent(original) => replay(command, original).await,
        Admission::New(slot) => match backend(&state) {
            Ok(backend) => run(command, slot, call(backend)).await,
            Err(reply) => return reply,
        },
    };
    outcome.unwrap_or_else(|| internal_error("the backend returned no reply"))
}

/// Starts `call` on its own task and awaits it.
async fn run<Fut>(command: &'static str, slot: NewCall, call: Fut) -> Outcome
where
    Fut: Future<Output = ApiReply> + Send + 'static,
{
    let running = slot.spawn(async move { Some(call.await) }, move |err| {
        let cancelled = err.is_cancelled();
        let detail = join_error_detail(err);
        if cancelled {
            log::info!("{command}: {detail}");
        } else {
            log::error!("{command}: {detail}");
        }
        Some(internal_error(detail))
    });
    running.outcome.await
}

/// [`page_call`] for the commands whose web route takes a JSON body: `body` is serialized to the
/// request bytes, which also identify the call.
async fn forward_json<T, F, Fut>(
    app: &AppHandle,
    command: &'static str,
    delivery: Delivery,
    body: &T,
    call: F,
) -> ApiReply
where
    T: Serialize,
    F: FnOnce(Backend, Vec<u8>) -> Fut,
    Fut: Future<Output = ApiReply> + Send + 'static,
{
    let bytes = match json_bytes(body) {
        Ok(bytes) => bytes,
        Err(reply) => return reply,
    };
    let key = CallKey::of(command).bytes(&bytes).finish();
    page_call(app, command, delivery, key, move |backend| {
        call(backend, bytes)
    })
    .await
}

/// `chat` — `POST {base}/app/api/chat`.
#[tauri::command]
pub async fn chat(app: AppHandle, payload: serde_json::Value, delivery: Delivery) -> ApiReply {
    forward_json(
        &app,
        ipc::CHAT,
        delivery,
        &payload,
        |backend, body| async move { backend.chat(&body, &ClientMeta::local()).await },
    )
    .await
}

/// `chat_stream` — `POST {base}/app/api/chat/stream`. Every [`StreamEvent`] is forwarded through
/// `on_event` in order. Resolves to `None` once the stream has ended (or was cancelled with
/// `chat_stream_cancel` or by a page reload), or to `Some(reply)` when the backend answered
/// without starting a stream (e.g. 400 invalid payload, 429), exactly where the web route sends a
/// non-SSE response.
#[tauri::command]
pub async fn chat_stream(
    app: AppHandle,
    payload: serde_json::Value,
    stream_id: u32,
    on_event: Channel<StreamEvent>,
    delivery: Delivery,
) -> Option<ApiReply> {
    let state = match desktop_state(&app) {
        Ok(state) => state,
        Err(reply) => return Some(reply),
    };
    let body = match json_bytes(&payload) {
        Ok(body) => body,
        Err(reply) => return Some(reply),
    };
    // The channel id is part of the key: it is unique per invoke and equal in its re-send.
    let key = CallKey::of(ipc::CHAT_STREAM)
        .json(&stream_id)
        .json(&on_event.id())
        .bytes(&body)
        .finish();
    let slot = match state.calls().admit(delivery, key) {
        Admission::New(slot) => slot,
        Admission::Resent(original) => {
            // Not registered under `stream_id`: that id belongs to the original stream.
            let outcome = replay(ipc::CHAT_STREAM, original).await;
            // This copy of the original's channel must not end it early. Dropping a channel tells
            // the page "no more messages after index <messages sent through this copy>", here 0;
            // if the page reads that before the original's first message (one of 8 KiB or more
            // arrives through an extra fetch), it stops listening and loses the whole stream. So
            // the copy is dropped only when every message has certainly arrived.
            spawn_task(async move {
                tokio::time::sleep(RESEND_WINDOW).await;
                drop(on_event);
            });
            return outcome;
        }
    };
    let backend = match backend(&state) {
        Ok(backend) => backend,
        Err(reply) => return Some(reply),
    };

    let streaming = Arc::new(AtomicBool::new(false));
    let running = slot.spawn(
        run_chat_stream(backend, body, on_event.clone(), streaming.clone()),
        move |err| chat_stream_ended_abnormally(stream_id, err, &streaming, &on_event),
    );
    // Registered before the first await, so `chat_stream_cancel` can find it right away. The
    // guard unregisters (and aborts) on every exit path.
    let _guard = StreamGuard::new(state.streams().clone(), stream_id, running.abort.clone());
    log::debug!("chat stream {stream_id} started");
    let outcome = running.outcome.await;
    log::debug!("chat stream {stream_id} finished");
    outcome
}

/// Outcome of a stream task that was aborted (cancel, page reload) or panicked.
fn chat_stream_ended_abnormally(
    stream_id: u32,
    err: JoinError,
    streaming: &AtomicBool,
    events: &Channel<StreamEvent>,
) -> Outcome {
    if err.is_cancelled() {
        log::info!("chat stream {stream_id} cancelled");
        return None;
    }
    let detail = join_error_detail(err);
    log::error!("chat stream {stream_id}: {detail}");
    if streaming.load(Ordering::Acquire) {
        // The frontend is already consuming events: finish with a V1-style error event.
        if let Err(send_err) = events.send(StreamEvent::error(format!(
            "Internal error (500): {detail}"
        ))) {
            log::warn!("chat stream {stream_id}: cannot deliver the error event: {send_err}");
        }
        None
    } else {
        Some(internal_error(detail))
    }
}

async fn run_chat_stream(
    backend: Backend,
    body: Vec<u8>,
    events: Channel<StreamEvent>,
    streaming: Arc<AtomicBool>,
) -> Outcome {
    let mut stream = match backend.chat_stream(&body, &ClientMeta::local()).await {
        ChatStreamStart::Reply(reply) => return Some(reply),
        ChatStreamStart::Stream(stream) => stream,
    };
    streaming.store(true, Ordering::Release);
    while let Some(event) = stream.next().await {
        if let Err(err) = events.send(event) {
            // The webview went away: stop pulling from the provider.
            log::warn!(
                "chat stream: cannot deliver events to the webview ({err}); closing the stream"
            );
            break;
        }
    }
    None
}

/// `chat_stream_cancel` — aborts the stream task promptly (the provider connection is dropped
/// with it) and removes it from the registry. Finished ids are ignored; a cancel that overtook
/// its `chat_stream` invoke cancels that stream as soon as it starts.
///
/// Tauri's re-send of a cancel that already ran is ignored: run again after a page reload, it
/// would count as an early cancel of the new page's stream with that id.
#[tauri::command]
pub async fn chat_stream_cancel(app: AppHandle, stream_id: u32, delivery: Delivery) {
    let Some(state) = app.try_state::<DesktopState>() else {
        return;
    };
    let key = CallKey::of(ipc::CHAT_STREAM_CANCEL)
        .json(&stream_id)
        .finish();
    let slot = match state.calls().admit(delivery, key) {
        Admission::New(slot) => slot,
        Admission::Resent(_) => {
            log::info!(
                "chat_stream_cancel {stream_id}: Tauri re-sent an invoke that already ran; ignored"
            );
            return;
        }
    };
    let streams = Arc::clone(state.streams());
    let running = slot.spawn(
        async move {
            if streams.cancel(stream_id) {
                log::debug!("chat stream {stream_id}: cancel requested");
            } else {
                log::debug!(
                    "chat_stream_cancel: stream {stream_id} is not running (finished, or not \
                     started yet)"
                );
            }
            None
        },
        // Only a page reload aborts it, and that cancels every stream anyway.
        |_| None,
    );
    running.outcome.await;
}

/// `chat_quota` — `GET {base}/app/api/chat/quota` (FreeAPI quota of the hosted server).
#[tauri::command]
pub async fn chat_quota(app: AppHandle, delivery: Delivery) -> ApiReply {
    let key = CallKey::of(ipc::CHAT_QUOTA).finish();
    page_call(&app, ipc::CHAT_QUOTA, delivery, key, |backend| async move {
        backend.chat_quota().await
    })
    .await
}

/// `tts_synthesize` — `POST {base}/app/api/tts/synthesize`; `binary` audio reply on success.
#[tauri::command]
pub async fn tts_synthesize(
    app: AppHandle,
    body: TtsSynthesizeBody,
    delivery: Delivery,
) -> ApiReply {
    forward_json(
        &app,
        ipc::TTS_SYNTHESIZE,
        delivery,
        &body,
        |backend, bytes| async move { backend.tts_synthesize(&bytes).await },
    )
    .await
}

/// `tts_health` — `POST {base}/app/api/tts/health`.
#[tauri::command]
pub async fn tts_health(app: AppHandle, body: ServerUrlBody, delivery: Delivery) -> ApiReply {
    forward_json(
        &app,
        ipc::TTS_HEALTH,
        delivery,
        &body,
        |backend, bytes| async move { backend.tts_health(&bytes).await },
    )
    .await
}

/// `stt_transcribe` — `POST {base}/app/api/stt/transcribe` with the multipart parts already split:
/// `audio_base64` is the recording (standard or URL-safe base64), `mime` its content type.
#[tauri::command]
pub async fn stt_transcribe(
    app: AppHandle,
    fields: SttTranscribeFields,
    audio_base64: String,
    mime: String,
    delivery: Delivery,
) -> ApiReply {
    let audio = match b64::decode(&audio_base64) {
        Ok(audio) => audio,
        Err(err) => {
            // V1 answers an unparsable multipart body with exactly this 400.
            log::warn!("stt_transcribe: audioBase64 is not valid base64 ({err})");
            return ApiReply::error(400, "Invalid form data");
        }
    };
    let key = CallKey::of(ipc::STT_TRANSCRIBE)
        .json(&fields)
        .bytes(audio_base64.as_bytes())
        .bytes(mime.as_bytes())
        .finish();
    // An empty type means "unknown" (the backend then uses application/octet-stream, like V1).
    let mime = Some(mime).filter(|m| !m.trim().is_empty());
    page_call(
        &app,
        ipc::STT_TRANSCRIBE,
        delivery,
        key,
        |backend| async move { backend.stt_transcribe_parts(&fields, audio, mime).await },
    )
    .await
}

/// `stt_health` — `POST {base}/app/api/stt/health`.
#[tauri::command]
pub async fn stt_health(app: AppHandle, body: ServerUrlBody, delivery: Delivery) -> ApiReply {
    forward_json(
        &app,
        ipc::STT_HEALTH,
        delivery,
        &body,
        |backend, bytes| async move { backend.stt_health(&bytes).await },
    )
    .await
}

/// `rag_embed` — `POST {base}/app/api/rag/embed`.
#[tauri::command]
pub async fn rag_embed(app: AppHandle, body: RagEmbedBody, delivery: Delivery) -> ApiReply {
    forward_json(
        &app,
        ipc::RAG_EMBED,
        delivery,
        &body,
        |backend, bytes| async move { backend.rag_embed(&bytes).await },
    )
    .await
}

/// `rag_health` — `POST {base}/app/api/rag/health`.
#[tauri::command]
pub async fn rag_health(app: AppHandle, body: ServerUrlBody, delivery: Delivery) -> ApiReply {
    forward_json(
        &app,
        ipc::RAG_HEALTH,
        delivery,
        &body,
        |backend, bytes| async move { backend.rag_health(&bytes).await },
    )
    .await
}

/// `app_info` — version, platform and the remote API base used for FreeAPI.
#[tauri::command]
pub async fn app_info(app: AppHandle) -> AppInfo {
    let remote_api_base = app
        .try_state::<DesktopState>()
        .map(|state| state.remote_api_base().to_owned())
        .unwrap_or_else(|| crate::config::DEFAULT_REMOTE_API_BASE.to_owned());
    AppInfo {
        version: version::VERSION.to_owned(),
        display_version: version::display_version(),
        platform: PLATFORM.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        remote_api_base,
    }
}

#[cfg(test)]
mod tests {
    use amadeus_core::protocol::ipc;

    /// The Tauri command name is the function name; it must equal the protocol constant.
    macro_rules! assert_command_names {
        ($($func:ident => $name:expr),* $(,)?) => {$(
            let _ = super::$func;
            assert_eq!(stringify!($func), $name);
        )*};
    }

    #[test]
    fn command_names_match_the_protocol() {
        assert_command_names!(
            chat => ipc::CHAT,
            chat_stream => ipc::CHAT_STREAM,
            chat_stream_cancel => ipc::CHAT_STREAM_CANCEL,
            chat_quota => ipc::CHAT_QUOTA,
            tts_synthesize => ipc::TTS_SYNTHESIZE,
            tts_health => ipc::TTS_HEALTH,
            stt_transcribe => ipc::STT_TRANSCRIBE,
            stt_health => ipc::STT_HEALTH,
            rag_embed => ipc::RAG_EMBED,
            rag_health => ipc::RAG_HEALTH,
            app_info => ipc::APP_INFO,
        );
    }

    #[test]
    fn internal_errors_are_classified_as_server_errors() {
        let reply = super::internal_error("boom");
        assert_eq!(reply.status(), 500);
        let message = reply
            .json_body()
            .and_then(|b| b["error"].as_str())
            .unwrap_or_default();
        assert!(
            message.contains("500") && message.contains("boom"),
            "{message}"
        );
    }

    #[test]
    fn platform_is_one_of_the_documented_values() {
        assert!(["windows", "macos", "linux"].contains(&super::PLATFORM));
    }
}
