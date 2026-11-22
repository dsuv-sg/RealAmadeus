//! # amadeus-backend
//!
//! Native (tokio + reqwest) implementation of everything the V1 Next.js API routes did.
//! Used by `amadeus-server` (HTTP, web version) and `amadeus-desktop` (Tauri IPC).
//!
//! Both front-ends call the SAME functions so status codes and JSON bodies are identical.
//! Request bodies are passed as raw bytes and parsed leniently (V1 ignored `Content-Type`
//! and relied on JS coercion), see the porting specs.
//!
//! Module ownership:
//! * [`chat`]  — `/chat`, `/chat/stream`, `/chat/quota`, `/chat/vertex-proxy`: providers,
//!   SSE parsing, Vertex auth/failover, FreeAPI, rate limiting, chat log.
//! * [`aux`]   — `/tts/*`, `/stt/*`, `/rag/*` proxies.
//! * [`local_proxy`] — library part of the `amadeus-local-proxy` binary (V1 `server.mjs`).

#[path = "aux_services/mod.rs"]
pub mod aux;
pub mod chat;
pub mod config;
pub mod http;
pub mod local_proxy;

use std::sync::Arc;

use amadeus_core::protocol::{ApiReply, StreamEvent, SttTranscribeFields};
use futures::stream::BoxStream;

pub use config::{BackendConfig, BackendMode};

/// Information about the caller (web server only; desktop uses [`ClientMeta::local`]).
#[derive(Debug, Clone)]
pub struct ClientMeta {
    /// Client IP as V1 `getClientIp` derived it (first `X-Forwarded-For` entry, then
    /// `X-Real-IP`, else `"unknown"`).
    pub ip: String,
}

impl ClientMeta {
    pub fn local() -> Self {
        Self {
            ip: "127.0.0.1".to_owned(),
        }
    }
}

/// Outcome of starting a streaming request.
pub enum ChatStreamStart {
    /// Reply sent INSTEAD of a stream (e.g. 400 invalid payload, 429 rate limited).
    Reply(ApiReply),
    /// HTTP 200 + SSE headers; every item is written as one `data: …\n\n` frame
    /// ([`StreamEvent::to_sse_frame`]) and flushed immediately.
    Stream(BoxStream<'static, StreamEvent>),
}

/// Cheaply clonable handle to the backend (all state is behind an `Arc`).
#[derive(Clone)]
pub struct Backend {
    inner: Arc<Inner>,
}

pub(crate) struct Inner {
    pub(crate) config: BackendConfig,
    pub(crate) http: reqwest::Client,
    pub(crate) chat: chat::ChatState,
}

impl Backend {
    pub fn new(config: BackendConfig) -> anyhow::Result<Self> {
        let http = http::build_client()?;
        let chat = chat::ChatState::new(&config)?;
        Ok(Self {
            inner: Arc::new(Inner { config, http, chat }),
        })
    }

    pub fn config(&self) -> &BackendConfig {
        &self.inner.config
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.inner.http
    }

    pub(crate) fn inner(&self) -> &Inner {
        &self.inner
    }

    // ----- chat (owned by `chat` module) ----------------------------------

    /// `POST {base}/app/api/chat`.
    pub async fn chat(&self, body: &[u8], client: &ClientMeta) -> ApiReply {
        chat::complete(self, body, client).await
    }

    /// `POST {base}/app/api/chat/stream`.
    pub async fn chat_stream(&self, body: &[u8], client: &ClientMeta) -> ChatStreamStart {
        chat::stream(self, body, client).await
    }

    /// `GET {base}/app/api/chat/quota`.
    pub async fn chat_quota(&self) -> ApiReply {
        chat::quota(self).await
    }

    /// `POST {base}/app/api/chat/vertex-proxy` (`stream` = header `x-vertex-stream: true`).
    pub async fn vertex_proxy(&self, body: &[u8], stream: bool) -> ChatStreamStart {
        chat::vertex_proxy(self, body, stream).await
    }

    // ----- aux (owned by `aux` module) ------------------------------------

    /// `POST {base}/app/api/tts/synthesize`.
    pub async fn tts_synthesize(&self, body: &[u8]) -> ApiReply {
        aux::tts::synthesize(self, body).await
    }

    /// `POST {base}/app/api/tts/health`.
    pub async fn tts_health(&self, body: &[u8]) -> ApiReply {
        aux::tts::health(self, body).await
    }

    /// `POST {base}/app/api/stt/transcribe` (multipart body as received by the server).
    pub async fn stt_transcribe(&self, content_type: Option<&str>, body: bytes::Bytes) -> ApiReply {
        aux::stt::transcribe(self, content_type, body).await
    }

    /// Same as [`Backend::stt_transcribe`] for callers that already have the parts (desktop IPC).
    pub async fn stt_transcribe_parts(
        &self,
        fields: &SttTranscribeFields,
        audio: Vec<u8>,
        mime: Option<String>,
    ) -> ApiReply {
        aux::stt::transcribe_parts(self, fields, audio, mime).await
    }

    /// `POST {base}/app/api/stt/health`.
    pub async fn stt_health(&self, body: &[u8]) -> ApiReply {
        aux::stt::health(self, body).await
    }

    /// `POST {base}/app/api/rag/embed`.
    pub async fn rag_embed(&self, body: &[u8]) -> ApiReply {
        aux::rag::embed(self, body).await
    }

    /// `POST {base}/app/api/rag/health`.
    pub async fn rag_health(&self, body: &[u8]) -> ApiReply {
        aux::rag::health(self, body).await
    }
}
