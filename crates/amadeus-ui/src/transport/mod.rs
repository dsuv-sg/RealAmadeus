//! Backend transport: HTTP `fetch` (web) or Tauri IPC (desktop). OWNED BY: ui-services.
//!
//! One [`Transport`] with the same Rust API on both platforms. Every call reports what V1's
//! client code saw — `(status, ok, json-or-null)` as a [`JsonReply`], or a [`TransportError`] —
//! so the services keep V1's error texts byte-for-byte:
//!
//! * [`HttpTransport`](http::HttpTransport) — `fetch` to `env::api_url(protocol::paths::*)`
//!   (V1 URLs `{base}/app/api/...`), JSON bodies in V1 key order, multipart STT upload,
//!   `cache: "no-store"` quota GET, streaming over `response.body` + `TextDecoder`, cancel with
//!   `AbortController`.
//! * [`TauriTransport`](tauri::TauriTransport) — `window.__TAURI__.core.invoke` with the
//!   commands of `amadeus_core::protocol::ipc` (`docs/DESKTOP_IPC.md`); streaming through a
//!   `Channel`, cancel with `chat_stream_cancel` plus an aborted flag (no events after cancel).
//!
//! Both feed the stream through the same V1 client parser ([`sse::SseParser`]).
//!
//! The shared instance is [`transport()`] (selected once per page with [`crate::env::is_tauri`]).

pub mod http;
mod js;
pub mod reply;
pub mod sse;
pub mod tauri;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use amadeus_core::protocol::ipc::AppInfo;
use amadeus_core::protocol::{
    ChatRequestPayload, RagEmbedBody, SttTranscribeFields, TtsSynthesizeBody,
};

pub(crate) use js::error_message as js_error_message;
pub use reply::{JsonReply, TransportError};
pub use sse::{SseEvent, SseParser};

/// V1 multipart filename of the STT upload, regardless of the real container (`audio/ogg`,
/// `audio/mp4`, …), kept because the upstream server may rely on it.
pub const RECORDING_FILE_NAME: &str = "recording.webm";

/// Result of `tts_synthesize`.
#[derive(Debug, Clone)]
pub enum TtsReply {
    /// OK response: the audio as a `Blob` (type = response `Content-Type`). May be empty; V1
    /// checks `blob.size === 0` itself.
    Audio(web_sys::Blob),
    /// Non-OK response (V1 then reads `data?.error`).
    Failed(JsonReply),
}

/// How a chat stream ended (errors are [`TransportError`]s).
#[derive(Debug, Clone, PartialEq)]
pub enum StreamOutcome {
    /// Every event was delivered: a terminal `Complete`/`Error`, or the end of the body (then
    /// a final `Complete(<accumulated tokens>)` was delivered, V1's `streamComplete(full)`).
    Finished,
    /// V1 `!response.ok || !response.body`: the pre-stream error reply (400 invalid payload,
    /// 429 rate limit, …). No event was delivered.
    Rejected(JsonReply),
}

/// Receives the parsed stream events, synchronously and in order. Never called after the
/// request was cancelled, and never after a terminal event.
pub type StreamSink = Rc<dyn Fn(SseEvent)>;

/// Cancellation handle of one request (V1 `AbortController`). Cheap to clone; single-threaded.
#[derive(Clone, Default)]
pub struct RequestCancel {
    inner: Rc<CancelInner>,
}

#[derive(Default)]
struct CancelInner {
    cancelled: Cell<bool>,
    controller: RefCell<Option<web_sys::AbortController>>,
    hooks: RefCell<Vec<Box<dyn FnOnce()>>>,
}

impl std::fmt::Debug for RequestCancel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestCancel")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

impl RequestCancel {
    /// A fresh, not yet cancelled handle.
    pub fn new() -> Self {
        Self::default()
    }

    /// V1 `controller.abort()`: aborts the `fetch` (web) or sends `chat_stream_cancel`
    /// (desktop). Idempotent. Sets the flag first, so no event is delivered afterwards.
    pub fn cancel(&self) {
        if self.inner.cancelled.replace(true) {
            return;
        }
        let controller = self.inner.controller.borrow().clone();
        if let Some(controller) = controller {
            controller.abort();
        }
        let hooks = std::mem::take(&mut *self.inner.hooks.borrow_mut());
        for hook in hooks {
            hook();
        }
    }

    /// V1 `signal.aborted`.
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.get()
    }

    /// The `AbortSignal` for `fetch` (created on first use; already aborted after `cancel`).
    pub(crate) fn signal(&self) -> Option<web_sys::AbortSignal> {
        let mut slot = self.inner.controller.borrow_mut();
        if slot.is_none() {
            let controller = web_sys::AbortController::new().ok()?;
            if self.is_cancelled() {
                controller.abort();
            }
            *slot = Some(controller);
        }
        slot.as_ref().map(web_sys::AbortController::signal)
    }

    /// Runs `hook` on `cancel` (immediately when already cancelled).
    pub(crate) fn on_cancel(&self, hook: impl FnOnce() + 'static) {
        if self.is_cancelled() {
            hook();
        } else {
            self.inner.hooks.borrow_mut().push(Box::new(hook));
        }
    }

    /// `Err(Aborted)` once cancelled.
    pub(crate) fn check(&self) -> Result<(), TransportError> {
        if self.is_cancelled() {
            Err(TransportError::Aborted)
        } else {
            Ok(())
        }
    }
}

/// The backend transport of this page.
#[derive(Debug)]
pub enum Transport {
    /// Web build: `fetch` against the V2 server (V1 HTTP contract).
    Http(http::HttpTransport),
    /// Desktop build: Tauri IPC.
    Tauri(tauri::TauriTransport),
}

thread_local! {
    static TRANSPORT: Rc<Transport> = Rc::new(Transport::detect());
}

/// The page-wide transport (Tauri IPC inside the desktop webview, HTTP otherwise).
pub fn transport() -> Rc<Transport> {
    TRANSPORT.with(Rc::clone)
}

/// Selects the transport now (idempotent). Called by [`crate::services::init`].
pub fn init() {
    TRANSPORT.with(|_| {});
}

impl Transport {
    /// Tauri IPC when `window.__TAURI__` exists, HTTP otherwise.
    pub fn detect() -> Self {
        if crate::env::is_tauri() {
            Transport::Tauri(tauri::TauriTransport::new())
        } else {
            Transport::Http(http::HttpTransport)
        }
    }

    /// True for the desktop (Tauri IPC) transport.
    pub fn is_desktop(&self) -> bool {
        matches!(self, Transport::Tauri(_))
    }

    /// `POST app/api/chat` / `chat` (V1 `AIService.sendChat`). `cancel` is optional; V1 never
    /// aborts this request.
    pub async fn chat(
        &self,
        payload: &ChatRequestPayload,
        cancel: Option<&RequestCancel>,
    ) -> Result<JsonReply, TransportError> {
        match self {
            Transport::Http(t) => t.chat(payload, cancel).await,
            Transport::Tauri(t) => t.chat(payload, cancel).await,
        }
    }

    /// `POST app/api/chat/stream` / `chat_stream` (V1 `AIService.sendChatStreaming`). Events go
    /// to `sink` as they arrive; the future resolves when the stream is over.
    /// `Err(Aborted)` after `cancel.cancel()` (silent in V1); `Err(Failed(_))` for network
    /// failures, including mid-stream (V1 `errorOccurred(error.message)`).
    pub async fn chat_stream(
        &self,
        payload: &ChatRequestPayload,
        cancel: &RequestCancel,
        sink: StreamSink,
    ) -> Result<StreamOutcome, TransportError> {
        match self {
            Transport::Http(t) => t.chat_stream(payload, cancel, sink).await,
            Transport::Tauri(t) => t.chat_stream(payload, cancel, sink).await,
        }
    }

    /// `GET app/api/chat/quota` with `cache: "no-store"` / `chat_quota` (FreeAPI quota).
    pub async fn chat_quota(&self) -> Result<JsonReply, TransportError> {
        match self {
            Transport::Http(t) => t.chat_quota().await,
            Transport::Tauri(t) => t.chat_quota().await,
        }
    }

    /// `POST app/api/tts/synthesize` / `tts_synthesize`.
    pub async fn tts_synthesize(
        &self,
        body: &TtsSynthesizeBody,
    ) -> Result<TtsReply, TransportError> {
        match self {
            Transport::Http(t) => t.tts_synthesize(body).await,
            Transport::Tauri(t) => t.tts_synthesize(body).await,
        }
    }

    /// `POST app/api/tts/health` / `tts_health` with `{"serverUrl"}`.
    pub async fn tts_health(&self, server_url: &str) -> Result<JsonReply, TransportError> {
        match self {
            Transport::Http(t) => {
                t.health(amadeus_core::protocol::paths::API_TTS_HEALTH, server_url)
                    .await
            }
            Transport::Tauri(t) => {
                t.health(amadeus_core::protocol::ipc::TTS_HEALTH, server_url)
                    .await
            }
        }
    }

    /// `POST app/api/stt/transcribe` (multipart: `audio` as `recording.webm`, then `provider`,
    /// `language`, `serverUrl`) / `stt_transcribe` (`audioBase64` + `mime`).
    pub async fn stt_transcribe(
        &self,
        audio: &web_sys::Blob,
        fields: &SttTranscribeFields,
    ) -> Result<JsonReply, TransportError> {
        match self {
            Transport::Http(t) => t.stt_transcribe(audio, fields).await,
            Transport::Tauri(t) => t.stt_transcribe(audio, fields).await,
        }
    }

    /// `POST app/api/stt/health` / `stt_health`.
    pub async fn stt_health(&self, server_url: &str) -> Result<JsonReply, TransportError> {
        match self {
            Transport::Http(t) => {
                t.health(amadeus_core::protocol::paths::API_STT_HEALTH, server_url)
                    .await
            }
            Transport::Tauri(t) => {
                t.health(amadeus_core::protocol::ipc::STT_HEALTH, server_url)
                    .await
            }
        }
    }

    /// `POST app/api/rag/embed` / `rag_embed` with `{"serverUrl","text"}`.
    pub async fn rag_embed(&self, body: &RagEmbedBody) -> Result<JsonReply, TransportError> {
        match self {
            Transport::Http(t) => t.rag_embed(body).await,
            Transport::Tauri(t) => t.rag_embed(body).await,
        }
    }

    /// `POST app/api/rag/health` / `rag_health`.
    pub async fn rag_health(&self, server_url: &str) -> Result<JsonReply, TransportError> {
        match self {
            Transport::Http(t) => {
                t.health(amadeus_core::protocol::paths::API_RAG_HEALTH, server_url)
                    .await
            }
            Transport::Tauri(t) => {
                t.health(amadeus_core::protocol::ipc::RAG_HEALTH, server_url)
                    .await
            }
        }
    }

    /// Desktop `app_info` (version, platform, remote API base). `None` on the web or on failure.
    pub async fn app_info(&self) -> Option<AppInfo> {
        match self {
            Transport::Http(_) => None,
            Transport::Tauri(t) => t.app_info().await,
        }
    }
}
