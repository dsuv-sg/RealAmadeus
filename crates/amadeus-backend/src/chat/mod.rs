//! Chat endpoints (`/chat`, `/chat/stream`, `/chat/quota`, `/chat/vertex-proxy`), a faithful
//! port of V1's `app/amadeus/app/api/chat/**` (porting spec `spec-server-chat.md`).
//!
//! * Request bodies are parsed leniently (`payload`) and keep V1's JS coercion semantics.
//! * Every provider of V1 is implemented for `/chat` (`providers`) and `/chat/stream`
//!   (`streaming`); Vertex auth/failover lives in `vertex` / `google`, FreeAPI in `free_api`.
//! * [`crate::BackendMode::Server`]: per-IP and global rate limits, the request counter file and
//!   the JSONL chat log in [`crate::BackendConfig::data_dir`].
//! * [`crate::BackendMode::Desktop`]: no limits/log/counter; FreeAPI and its quota are relayed to
//!   the hosted server (`remote`).
//! * Streams are `'static`; dropping one aborts its producer task and the upstream request.
//! * Every body V1 built with `JSON.stringify` is serialized with JS semantics
//!   ([`crate::aux::js_json::stringify`]: V8 key order, `Number#toString`).

mod bodies;
mod chatlog;
mod extract;
mod free_api;
mod google;
mod net;
mod payload;
mod providers;
mod ratelimit;
mod remote;
mod sink;
mod sse;
mod streaming;
pub mod texts;
mod vertex;

#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use amadeus_core::jscompat::js_trim;
use amadeus_core::protocol::{ApiReply, StreamEvent};
use serde_json::{json, Value};

pub use chatlog::CHAT_LOG_FILE;
pub use free_api::{
    FREE_API_GCP_LOCATION, FREE_API_GCP_MODEL, FREE_API_GCP_PROJECT, FREE_API_STUDIO_MODELS,
};
pub use ratelimit::{
    MAX_GLOBAL_REQUESTS, MAX_REQUESTS_PER_WINDOW, RATE_LIMIT_WINDOW_MS, REQUEST_COUNT_FILE,
};

use self::payload::{value_text, Payload, ProviderVal, Thrown};
use self::providers::Once;
use self::sink::{fixed_stream, spawn_stream, Interceptor, StreamErr};
use self::texts::{
    ERR_INVALID_PAYLOAD, ERR_TOO_MANY_REQUESTS_DEFAULT, FREE_FALLBACK_COMM_ERROR,
    FREE_FALLBACK_RATE_LIMITED,
};
use crate::{Backend, BackendConfig, BackendMode, ChatStreamStart, ClientMeta};

/// `tracing` target of the V1 console lines (`spec-server-chat.md` §15.3).
const LOG_TARGET: &str = "amadeus_backend::chat";

/// Upstream endpoints (V1 constants; tests point them at a local mock server).
#[derive(Debug, Clone)]
pub(crate) struct Endpoints {
    pub(crate) openai: String,
    /// `…/v1beta/models` (AI Studio Gemini).
    pub(crate) gemini_models: String,
    pub(crate) anthropic: String,
    pub(crate) groq: String,
    pub(crate) openrouter: String,
    /// Replaces the Google Vertex hosts (`https://[{region}-]aiplatform.googleapis.com`).
    pub(crate) vertex_origin: Option<String>,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            openai: "https://api.openai.com/v1/chat/completions".to_owned(),
            gemini_models: "https://generativelanguage.googleapis.com/v1beta/models".to_owned(),
            anthropic: "https://api.anthropic.com/v1/messages".to_owned(),
            groq: "https://api.groq.com/openai/v1/chat/completions".to_owned(),
            openrouter: "https://openrouter.ai/api/v1/chat/completions".to_owned(),
            vertex_origin: None,
        }
    }
}

impl Endpoints {
    /// AI Studio `generateContent` / `streamGenerateContent?alt=sse` URL.
    pub(crate) fn gemini(&self, model: &str, stream: bool) -> String {
        if stream {
            format!(
                "{}/{model}:streamGenerateContent?alt=sse",
                self.gemini_models
            )
        } else {
            format!("{}/{model}:generateContent", self.gemini_models)
        }
    }

    /// `buildVertexUrl(project, location, model, method)`.
    pub(crate) fn vertex(
        &self,
        project: &str,
        location: &str,
        model: &str,
        method: vertex::VertexMethod,
    ) -> String {
        match &self.vertex_origin {
            Some(origin) => vertex::build_vertex_url_at(origin, project, location, model, method),
            None => vertex::build_vertex_url(project, location, model, method),
        }
    }
}

/// `setTimeout` jitter between Vertex retries (recorded instead of slept in tests).
#[derive(Default)]
pub(crate) struct Jitter {
    #[cfg(test)]
    pub(crate) recorded: Option<std::sync::Arc<std::sync::Mutex<Vec<Duration>>>>,
}

impl Jitter {
    /// Sleep for `d`.
    pub(crate) async fn sleep(&self, d: Duration) {
        #[cfg(test)]
        if let Some(rec) = &self.recorded {
            rec.lock().unwrap_or_else(|e| e.into_inner()).push(d);
            return;
        }
        tokio::time::sleep(d).await;
    }
}

/// `Math.random()` substitute for retry jitter (no `rand` dependency).
pub(crate) fn js_random() -> f64 {
    use std::hash::{BuildHasher, Hasher};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    (hasher.finish() >> 11) as f64 / (1u64 << 53) as f64
}

/// `base + Math.random() * span` milliseconds (fractional, like V1's stream retry).
pub(crate) fn delay_float_ms(base: f64, span: f64) -> Duration {
    Duration::from_secs_f64((base + js_random() * span) / 1000.0)
}

/// `base + Math.floor(Math.random() * span)` milliseconds.
pub(crate) fn delay_int_ms(base: u64, span: u64) -> Duration {
    Duration::from_millis(base + (js_random() * span as f64) as u64)
}

/// Shared chat state: rate limiter, global counter, chat log writer, Google token caches.
pub struct ChatState {
    limiter: Option<ratelimit::RateLimiter>,
    log: Option<chatlog::ChatLog>,
    pub(crate) google: google::GoogleAuth,
    pub(crate) endpoints: Endpoints,
    pub(crate) jitter: Jitter,
    clock: Instant,
}

impl ChatState {
    /// Build the state. On the web server this loads `.amadeus_request_count` from
    /// `config.data_dir` (created if missing), applies the FreeAPI cap `config.free_api_max`
    /// (an error unless it is positive) and prepares `.amadeus_chat_log.jsonl`.
    pub fn new(config: &BackendConfig) -> anyhow::Result<Self> {
        let (limiter, log) = if config.is_server() {
            anyhow::ensure!(
                config.free_api_max > 0,
                "the FreeAPI cap must be a positive number of requests, got {}",
                config.free_api_max
            );
            if let Err(e) = std::fs::create_dir_all(&config.data_dir) {
                console_error(&format!(
                    "Failed to create data dir {}: {e}",
                    config.data_dir.display()
                ));
            }
            (
                Some(ratelimit::RateLimiter::load(
                    config.data_dir.join(REQUEST_COUNT_FILE),
                    config.free_api_max,
                )),
                Some(chatlog::ChatLog::new(config.data_dir.join(CHAT_LOG_FILE))),
            )
        } else {
            (None, None)
        };
        Ok(Self {
            limiter,
            log,
            google: google::GoogleAuth::new(config.vertex_credentials_path.clone()),
            endpoints: Endpoints::default(),
            jitter: Jitter::default(),
            clock: Instant::now(),
        })
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.clock.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// The chat state of a backend.
pub(crate) fn state(backend: &Backend) -> &ChatState {
    &backend.inner().chat
}

/// `console.log` line of V1.
pub(crate) fn console_log(msg: &str) {
    tracing::info!(target: LOG_TARGET, "{msg}");
}

/// `console.warn` line of V1.
pub(crate) fn console_warn(msg: &str) {
    tracing::warn!(target: LOG_TARGET, "{msg}");
}

/// `console.error` line of V1.
pub(crate) fn console_error(msg: &str) {
    tracing::error!(target: LOG_TARGET, "{msg}");
}

/// V1 `getClientIp(req)` for servers: first `X-Forwarded-For` entry, then `X-Real-IP`, else
/// `"unknown"`. Pass every header occurrence joined with `", "` (undici `Headers.get`).
pub fn client_ip(x_forwarded_for: Option<&str>, x_real_ip: Option<&str>) -> String {
    let normalize = |v: &str| {
        v.trim_matches(|c| matches!(c, '\t' | '\n' | '\r' | ' '))
            .to_owned()
    };
    if let Some(xff) = x_forwarded_for.map(normalize).filter(|v| !v.is_empty()) {
        if let Some(first) = xff.split(',').next().map(js_trim).filter(|f| !f.is_empty()) {
            return first.to_owned();
        }
    }
    if let Some(real) = x_real_ip.map(normalize).filter(|v| !v.is_empty()) {
        return js_trim(&real).to_owned();
    }
    "unknown".to_owned()
}

/// `[Req #n] A: …` plus `logInteraction` (web server only).
pub(crate) fn log_answer(
    backend: &Backend,
    ip: &str,
    provider: i64,
    question: &Value,
    reply: &Value,
    count: i64,
) {
    if let Some(log) = &state(backend).log {
        console_log(&format!("[Req #{count}] A: {}", value_text(reply)));
        log.log_interaction(ip, provider, question, reply, count);
    }
}

/// Result of the rate-limit admission step.
enum Admission {
    Admitted { count: i64 },
    Denied { error: String },
}

/// `checkRateLimit(request, payload.provider)` (web server only).
fn admit(backend: &Backend, client: &ClientMeta, provider: ProviderVal) -> Admission {
    let st = state(backend);
    match &st.limiter {
        Some(limiter) => {
            let decision = limiter.check(&client.ip, provider.counts_global(), st.now_ms());
            if decision.allowed {
                Admission::Admitted {
                    count: decision.count,
                }
            } else {
                Admission::Denied {
                    error: decision
                        .error_msg
                        .unwrap_or_else(|| ERR_TOO_MANY_REQUESTS_DEFAULT.to_owned()),
                }
            }
        }
        None => Admission::Admitted { count: 0 },
    }
}

fn remote_base(backend: &Backend) -> Option<&str> {
    match &backend.config().mode {
        BackendMode::Desktop { remote_api_base } => Some(remote_api_base.as_str()),
        BackendMode::Server => None,
    }
}

/// `POST {base}/app/api/chat`.
pub async fn complete(backend: &Backend, body: &[u8], client: &ClientMeta) -> ApiReply {
    let Some(payload) = Payload::parse(body) else {
        return ApiReply::error(400, ERR_INVALID_PAYLOAD);
    };
    if let Some(base) = remote_base(backend) {
        if payload.provider().is(8) {
            return remote::forward_chat(backend, base, body).await;
        }
    }
    complete_payload(backend, payload, client).await
}

/// `route.ts` `POST` after body parsing (§6.1), as Next.js serves it: an exception thrown
/// outside the handler's `try` block becomes a 500 with an empty body.
pub(crate) async fn complete_payload(
    backend: &Backend,
    payload: Payload,
    client: &ClientMeta,
) -> ApiReply {
    chat_route(backend, payload, client)
        .await
        .unwrap_or(ApiReply::Empty { status: 500 })
}

/// The `route.ts` `POST` handler called as a plain function. `Err` is an exception thrown
/// outside its `try` block (a `null` message, a non-string `model`), after the rate-limit check
/// and, for the model, after the `Q:` console line. The streaming fallback awaits the handler
/// this way, so it receives the rejection instead of a response.
pub(crate) async fn chat_route(
    backend: &Backend,
    mut payload: Payload,
    client: &ClientMeta,
) -> Result<ApiReply, Thrown> {
    let provider = payload.provider();
    let count = match admit(backend, client, provider) {
        Admission::Admitted { count } => count,
        Admission::Denied { error } => {
            if provider.is(8) {
                return Ok(ApiReply::json(
                    200,
                    json!({ "response": FREE_FALLBACK_RATE_LIMITED }),
                ));
            }
            return Ok(ApiReply::error(429, error));
        }
    };
    let question = payload.question()?;
    let logging = state(backend).log.is_some();
    if logging {
        console_log(&format!("\n[Req #{count}] Q: {}", value_text(&question)));
    }
    let model = payload.trim_or_empty("model", "payload.model?.trim")?;
    Ok(
        match providers::chat_once(backend, &mut payload, provider, model).await {
            Ok(Once::Send(text)) => {
                log_answer(
                    backend,
                    &client.ip,
                    provider.log_value(),
                    &question,
                    &text,
                    count,
                );
                ApiReply::json(200, json!({ "response": text }))
            }
            Ok(Once::Fail { status, error }) => net::error_reply(status, error),
            Err(Thrown(msg)) => {
                if provider.is(8) {
                    ApiReply::json(200, json!({ "response": FREE_FALLBACK_COMM_ERROR }))
                } else {
                    ApiReply::error(500, msg)
                }
            }
        },
    )
}

/// `POST {base}/app/api/chat/stream`.
pub async fn stream(backend: &Backend, body: &[u8], client: &ClientMeta) -> ChatStreamStart {
    let Some(payload) = Payload::parse(body) else {
        return ChatStreamStart::Reply(ApiReply::error(400, ERR_INVALID_PAYLOAD));
    };
    let provider = payload.provider();
    if let Some(base) = remote_base(backend) {
        if provider.is(8) {
            return remote::forward_stream(backend, base, body).await;
        }
    }
    let count = match admit(backend, client, provider) {
        Admission::Admitted { count } => count,
        Admission::Denied { error } => {
            if provider.is(8) {
                return ChatStreamStart::Stream(fixed_stream(vec![
                    StreamEvent::token(FREE_FALLBACK_RATE_LIMITED),
                    StreamEvent::complete(FREE_FALLBACK_RATE_LIMITED),
                ]));
            }
            return ChatStreamStart::Reply(ApiReply::error(429, error));
        }
    };
    let Ok(question) = payload.question() else {
        return ChatStreamStart::Reply(ApiReply::Empty { status: 500 });
    };
    let interceptor = if state(backend).log.is_some() {
        console_log(&format!("\n[Req #{count}] Q: {}", value_text(&question)));
        Some(Interceptor {
            backend: backend.clone(),
            ip: client.ip.clone(),
            provider: provider.log_value(),
            question,
            count,
        })
    } else {
        None
    };
    let backend = backend.clone();
    ChatStreamStart::Stream(spawn_stream(interceptor, move |mut sink| async move {
        let mut payload = payload;
        if let Err(StreamErr::Thrown(msg)) =
            streaming::dispatch(&backend, &mut payload, &mut sink).await
        {
            // §7.2 step c (the top-level catch).
            if provider.is(8) {
                if sink
                    .emit(StreamEvent::token(FREE_FALLBACK_COMM_ERROR))
                    .await
                    .is_ok()
                {
                    let _ = sink
                        .emit(StreamEvent::complete(FREE_FALLBACK_COMM_ERROR))
                        .await;
                }
            } else {
                let _ = sink.emit(StreamEvent::error(msg)).await;
            }
        }
    }))
}

/// `GET {base}/app/api/chat/quota`.
pub async fn quota(backend: &Backend) -> ApiReply {
    if let Some(base) = remote_base(backend) {
        return remote::forward_quota(backend, base).await;
    }
    let (count, max) = match &state(backend).limiter {
        Some(limiter) => (limiter.global_count(), limiter.global_max()),
        None => (0, backend.config().free_api_max),
    };
    ApiReply::json(
        200,
        json!({ "count": count, "max": max, "remaining": max.saturating_sub(count).max(0) }),
    )
}

/// `POST {base}/app/api/chat/vertex-proxy` (`stream` = header `x-vertex-stream: true`). Not
/// rate limited, counted or logged (V1 parity).
pub async fn vertex_proxy(backend: &Backend, body: &[u8], stream: bool) -> ChatStreamStart {
    let req = match vertex::proxy_prepare(backend, body).await {
        Ok(req) => req,
        Err(reply) => return ChatStreamStart::Reply(reply),
    };
    if !stream {
        return ChatStreamStart::Reply(vertex::proxy_once(backend, &req).await);
    }
    let backend = backend.clone();
    ChatStreamStart::Stream(spawn_stream(None, move |mut sink| async move {
        let _ = vertex::proxy_stream(&backend, &req, &mut sink).await;
    }))
}
