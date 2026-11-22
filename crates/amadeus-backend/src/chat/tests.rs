//! End-to-end tests of [`complete`], [`stream`], [`quota`] and [`vertex_proxy`] against local
//! mock upstream servers (axum on 127.0.0.1:0).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use amadeus_core::protocol::{ApiReply, StreamEvent};
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Method, Uri};
use axum::response::Response;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use futures::StreamExt;
use serde_json::{json, Value};

use super::google::GcloudProgram;
use super::texts::*;
use super::*;
use crate::{Backend, BackendConfig, ClientMeta, Inner};

// ---------------------------------------------------------------------------
// Mock upstream
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Recorded {
    method: String,
    path: String,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Recorded {
    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// Canned upstream answer.
#[derive(Clone)]
struct Reply {
    status: u16,
    content_type: &'static str,
    chunks: Vec<Vec<u8>>,
    /// Reset the connection after the chunks (body read error on the client).
    break_after: bool,
    /// Keep the body open (SSE comments) and set the flag when hyper drops the body.
    hang: Option<Arc<AtomicBool>>,
    /// Non-canonical HTTP/1.1 reason phrase (undici's `statusText`).
    reason: Option<&'static [u8]>,
}

impl Reply {
    fn json(status: u16, v: Value) -> Self {
        Self::raw(
            status,
            "application/json",
            vec![serde_json::to_vec(&v).unwrap_or_default()],
        )
    }

    fn text(status: u16, s: &str) -> Self {
        Self::raw(status, "text/plain", vec![s.as_bytes().to_vec()])
    }

    fn sse(chunks: &[&str]) -> Self {
        Self::raw(
            200,
            "text/event-stream",
            chunks.iter().map(|c| c.as_bytes().to_vec()).collect(),
        )
    }

    fn raw(status: u16, content_type: &'static str, chunks: Vec<Vec<u8>>) -> Self {
        Self {
            status,
            content_type,
            chunks,
            break_after: false,
            hang: None,
            reason: None,
        }
    }

    fn not_found() -> Self {
        Self::json(404, json!({ "error": "no route" }))
    }
}

struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

type Handler = Arc<dyn Fn(&Recorded) -> Reply + Send + Sync>;

#[derive(Clone)]
struct MockState {
    handler: Handler,
    log: Arc<Mutex<Vec<Recorded>>>,
}

struct Mock {
    base: String,
    port: u16,
    log: Arc<Mutex<Vec<Recorded>>>,
    task: tokio::task::JoinHandle<()>,
}

impl Mock {
    async fn start(handler: impl Fn(&Recorded) -> Reply + Send + Sync + 'static) -> Mock {
        let log = Arc::new(Mutex::new(Vec::new()));
        let state = MockState {
            handler: Arc::new(handler),
            log: log.clone(),
        };
        let app = Router::new().fallback(serve).with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock");
        let addr = listener.local_addr().expect("mock addr");
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Mock {
            base: format!("http://{addr}"),
            port: addr.port(),
            log,
            task,
        }
    }

    fn requests(&self) -> Vec<Recorded> {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn requests_to(&self, prefix: &str) -> Vec<Recorded> {
        self.requests()
            .into_iter()
            .filter(|r| r.path.starts_with(prefix))
            .collect()
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}

type ChunkState = (std::vec::IntoIter<Vec<u8>>, bool, Option<DropFlag>, bool);

async fn serve(
    State(st): State<MockState>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let rec = Recorded {
        method: method.to_string(),
        path: uri
            .path_and_query()
            .map(ToString::to_string)
            .unwrap_or_default(),
        headers,
        body: body.to_vec(),
    };
    st.log
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .push(rec.clone());
    let reply = (st.handler)(&rec);
    let reason = reply.reason;
    let init: ChunkState = (
        reply.chunks.into_iter(),
        reply.break_after,
        reply.hang.map(DropFlag),
        false,
    );
    let stream = futures::stream::unfold(init, |(mut it, brk, hang, done)| async move {
        if done {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(3)).await;
        match it.next() {
            Some(chunk) => Some((Ok(Bytes::from(chunk)), (it, brk, hang, false))),
            None if brk => Some((Err(std::io::Error::other("reset")), (it, brk, hang, true))),
            None if hang.is_some() => {
                tokio::time::sleep(Duration::from_millis(20)).await;
                Some((
                    Ok(Bytes::from_static(b": ping\n\n")),
                    (it, brk, hang, false),
                ))
            }
            None => None,
        }
    });
    let mut response = Response::builder()
        .status(reply.status)
        .header("content-type", reply.content_type)
        .body(Body::from_stream(stream))
        .unwrap_or_else(|_| Response::new(Body::empty()));
    if let Some(reason) = reason {
        // hyper's HTTP/1 server writes this phrase into the status line.
        response
            .extensions_mut()
            .insert(hyper::ext::ReasonPhrase::from_static(reason));
    }
    response
}

/// A port with nothing listening on it.
async fn closed_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    listener.local_addr().expect("addr").port()
}

// ---------------------------------------------------------------------------
// Backend helpers
// ---------------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "amadeus-chat-e2e-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        TempDir(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn server_config(dir: &TempDir) -> BackendConfig {
    let mut config = BackendConfig::server_default();
    config.data_dir = dir.path().to_path_buf();
    config
}

fn desktop_config(dir: &TempDir, remote: &str) -> BackendConfig {
    BackendConfig::desktop(dir.path().to_path_buf(), remote)
}

fn backend_with(
    config: BackendConfig,
    mock: Option<&Mock>,
    tweak: impl FnOnce(&mut ChatState),
) -> Backend {
    let mut chat = ChatState::new(&config).expect("chat state");
    if let Some(m) = mock {
        chat.endpoints = Endpoints {
            openai: format!("{}/openai/v1/chat/completions", m.base),
            gemini_models: format!("{}/v1beta/models", m.base),
            anthropic: format!("{}/anthropic/v1/messages", m.base),
            groq: format!("{}/groq/openai/v1/chat/completions", m.base),
            openrouter: format!("{}/openrouter/api/v1/chat/completions", m.base),
            vertex_origin: Some(format!("{}/vertex", m.base)),
        };
    }
    chat.jitter = Jitter {
        recorded: Some(Arc::new(Mutex::new(Vec::new()))),
    };
    chat.google.gcloud_program = GcloudProgram::Fixed(String::new());
    tweak(&mut chat);
    let http = crate::http::build_client().expect("http client");
    Backend {
        inner: Arc::new(Inner { config, http, chat }),
    }
}

fn jitter_of(backend: &Backend) -> Vec<Duration> {
    state(backend)
        .jitter
        .recorded
        .as_ref()
        .map(|r| r.lock().unwrap_or_else(|e| e.into_inner()).clone())
        .unwrap_or_default()
}

fn client() -> ClientMeta {
    ClientMeta {
        ip: "203.0.113.7".to_owned(),
    }
}

fn body(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).unwrap_or_default()
}

fn user_messages() -> Value {
    json!([{ "role": "system", "content": "S" }, { "role": "user", "content": "U" }])
}

async fn events(start: ChatStreamStart) -> Vec<StreamEvent> {
    match start {
        ChatStreamStart::Stream(s) => {
            tokio::time::timeout(Duration::from_secs(30), s.collect::<Vec<_>>())
                .await
                .expect("stream timed out")
        }
        ChatStreamStart::Reply(r) => panic!("expected a stream, got {r:?}"),
    }
}

fn reply(start: ChatStreamStart) -> ApiReply {
    match start {
        ChatStreamStart::Reply(r) => r,
        ChatStreamStart::Stream(_) => panic!("expected a reply, got a stream"),
    }
}

fn tok(s: &str) -> StreamEvent {
    StreamEvent::token(s)
}

fn done(s: &str) -> StreamEvent {
    StreamEvent::complete(s)
}

fn response_json(text: &str) -> ApiReply {
    ApiReply::json(200, json!({ "response": text }))
}

fn openai_sse(content: &str) -> String {
    format!(
        "data: {}\n\n",
        json!({ "choices": [{ "delta": { "content": content } }] })
    )
}

fn gemini_chunk(text: &str) -> Value {
    json!({ "candidates": [{ "content": { "parts": [{ "text": text }] } }] })
}

// ---------------------------------------------------------------------------
// Streaming providers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn openai_compatible_stream_relays_tokens() {
    let split = openai_sse("lo あ").into_bytes();
    let cut = split.iter().position(|&b| b == 0xE3).unwrap_or(0) + 1;
    let (a, b) = (split[..cut].to_vec(), split[cut..].to_vec());
    let mock = Mock::start(move |_| {
        Reply::raw(
            200,
            "text/event-stream",
            vec![
                b"data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n".to_vec(),
                openai_sse("Hel").into_bytes(),
                a.to_vec(),
                b.to_vec(),
                b": keep-alive\n\ndata: {\"error\":{\"message\":\"ignored\"}}\n\n".to_vec(),
                b"data: [DONE]\n\n".to_vec(),
                openai_sse("after-done").into_bytes(),
            ],
        )
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    let payload = json!({
        "messages": [{ "role": "user", "content": "U" }], "provider": 0, "apiKey": "sk-test", "model": "",
        "openaiCompatible": true, "openaiBaseUrl": format!(" {}/v1/ ", mock.base)
    });
    let got = events(stream(&backend, &body(&payload), &client()).await).await;
    assert_eq!(got, vec![tok("Hel"), tok("lo あ"), done("Hello あ")]);
    let reqs = mock.requests();
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].path, "/v1/chat/completions");
    assert_eq!(
        reqs[0].header("authorization").as_deref(),
        Some("Bearer sk-test")
    );
    assert_eq!(
        reqs[0].header("accept").as_deref(),
        Some("text/event-stream")
    );
    assert_eq!(
        reqs[0].text(),
        r#"{"model":"gpt-4o","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"stream":true}"#
    );
}

#[tokio::test]
async fn gemini_stream_processes_unterminated_last_line() {
    let mock = Mock::start(|r| {
        if r.path != "/v1beta/models/gemini-2.0-flash:streamGenerateContent?alt=sse" {
            return Reply::not_found();
        }
        Reply::sse(&[
            &format!("data: {}\r\n\r\n", gemini_chunk("こん")),
            "data: {\"candidates\":[{\"finishReason\":\"SAFETY\"}]}\n\n",
            &format!("data: {}", gemini_chunk("にちは")),
        ])
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    let payload = json!({ "messages": user_messages(), "provider": 1, "apiKey": "g-key", "model": "  ", "webSearch": true });
    let got = events(stream(&backend, &body(&payload), &client()).await).await;
    assert_eq!(got, vec![tok("こん"), tok("にちは"), done("こんにちは")]);
    let req = &mock.requests()[0];
    assert_eq!(req.header("x-goog-api-key").as_deref(), Some("g-key"));
    assert_eq!(
        req.text(),
        r#"{"contents":[{"role":"user","parts":[{"text":"U"}]}],"generationConfig":{"maxOutputTokens":8192},"system_instruction":{"parts":[{"text":"S"}]},"tools":[{"googleSearch":{}}]}"#
    );
}

#[tokio::test]
async fn stream_upstream_error_uses_e_s_format() {
    let mock =
        Mock::start(|_| Reply::json(400, json!({ "error": { "code": 400, "message": "x\ny" } })))
            .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    let payload = json!({ "messages": user_messages(), "provider": 1, "apiKey": "k" });
    let got = events(stream(&backend, &body(&payload), &client()).await).await;
    assert_eq!(
        got,
        vec![StreamEvent::error(
            r#"400 Bad Request: {"error":{"code":400,"message":"x\ny"}}"#
        )]
    );
    // A null-body status is treated like `!response.body`.
    let no_body = Mock::start(|_| Reply::raw(204, "text/event-stream", Vec::new())).await;
    let backend = backend_with(
        desktop_config(&dir, "http://unused"),
        Some(&no_body),
        |_| {},
    );
    let got = events(stream(&backend, &body(&payload), &client()).await).await;
    assert_eq!(got, vec![StreamEvent::error("204 No Content: null")]);
}

#[tokio::test]
async fn groq_stream_validates_model_and_body() {
    let mock = Mock::start(|_| Reply::sse(&[&openai_sse("G"), "data: [DONE]\n\n"])).await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    // `groq/compound` is not allowed when streaming: replaced by qwen3-32b.
    let payload = json!({ "messages": [{ "role": "user", "content": "U" }], "provider": 3, "apiKey": "gk", "model": "groq/compound" });
    let got = events(stream(&backend, &body(&payload), &client()).await).await;
    assert_eq!(got, vec![tok("G"), done("G")]);
    let req = &mock.requests()[0];
    assert_eq!(req.path, "/groq/openai/v1/chat/completions");
    assert_eq!(
        req.text(),
        r#"{"model":"qwen3-32b","messages":[{"role":"user","content":"U"}],"max_tokens":2048,"temperature":0.85,"top_p":0.9,"reasoning_format":"hidden","stream":true}"#
    );
}

#[tokio::test]
async fn ollama_stream_drops_unterminated_last_line() {
    let mock = Mock::start(|_| {
        Reply::raw(
            200,
            "application/x-ndjson",
            vec![
                b"{\"message\":{\"content\":\"Hi\"},\"done\":false}\n\n".to_vec(),
                b"{\"message\":{\"content\":\" there\"},\"done\":false}\nnot json\n".to_vec(),
                b"{\"message\":{\"content\":\"LOST\"},\"done\":true}".to_vec(),
            ],
        )
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), None, |_| {});
    let payload = json!({ "messages": [{ "role": "user", "content": "U" }], "provider": 5, "ollamaHost": mock.base, "model": "GPT-x" });
    let got = events(stream(&backend, &body(&payload), &client()).await).await;
    assert_eq!(got, vec![tok("Hi"), tok(" there"), done("Hi there")]);
    let req = &mock.requests()[0];
    assert_eq!(req.path, "/api/chat");
    assert_eq!(
        req.text(),
        r#"{"model":"mistral","messages":[{"role":"user","content":"U"}],"stream":true}"#
    );
}

#[tokio::test]
async fn transport_errors_map_to_undici_messages() {
    let port = closed_port().await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), None, |_| {});
    let refused = json!({
        "messages": [], "provider": 0, "apiKey": "k", "openaiCompatible": true,
        "openaiBaseUrl": format!("http://127.0.0.1:{port}")
    });
    assert_eq!(
        complete(&backend, &body(&refused), &client()).await,
        ApiReply::error(500, FETCH_FAILED)
    );
    assert_eq!(
        events(stream(&backend, &body(&refused), &client()).await).await,
        vec![StreamEvent::error(FETCH_FAILED)]
    );
    let blank = json!({ "messages": [], "provider": 0, "apiKey": "k", "openaiCompatible": true, "openaiBaseUrl": "   " });
    assert_eq!(
        complete(&backend, &body(&blank), &client()).await,
        ApiReply::error(500, "Failed to parse URL from /chat/completions")
    );
    let bad_type = json!({ "messages": [], "provider": 0, "apiKey": "k", "openaiCompatible": true, "openaiBaseUrl": 7 });
    assert_eq!(
        complete(&backend, &body(&bad_type), &client()).await,
        ApiReply::error(500, "payload.openaiBaseUrl.trim is not a function")
    );
}

#[tokio::test]
async fn body_failure_mid_stream_is_terminated() {
    let mock = Mock::start(|_| Reply {
        break_after: true,
        ..Reply::sse(&[&openai_sse("A")])
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    let payload = json!({ "messages": [], "provider": 6, "apiKey": "or" });
    let got = events(stream(&backend, &body(&payload), &client()).await).await;
    assert_eq!(got, vec![tok("A"), StreamEvent::error(TERMINATED)]);
    let req = &mock.requests()[0];
    assert_eq!(req.path, "/openrouter/api/v1/chat/completions");
    assert_eq!(
        req.header("http-referer").as_deref(),
        Some("https://dsuv.vercel.app")
    );
    assert_eq!(req.header("x-title").as_deref(), Some("RealAmadeus"));
    assert_eq!(req.json()["model"], json!("openai/gpt-4o"));
}

#[tokio::test]
async fn dropping_the_stream_cancels_the_upstream_request() {
    let flag = Arc::new(AtomicBool::new(false));
    let hang = flag.clone();
    let mock = Mock::start(move |_| Reply {
        hang: Some(hang.clone()),
        ..Reply::sse(&[&openai_sse("first")])
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), None, |_| {});
    let payload = json!({
        "messages": [], "provider": 0, "apiKey": "k", "openaiCompatible": true, "openaiBaseUrl": mock.base
    });
    let ChatStreamStart::Stream(mut s) = stream(&backend, &body(&payload), &client()).await else {
        panic!("expected a stream");
    };
    let first = tokio::time::timeout(Duration::from_secs(10), s.next())
        .await
        .expect("first token");
    assert_eq!(first, Some(tok("first")));
    drop(s);
    let cancelled = tokio::time::timeout(Duration::from_secs(10), async {
        while !flag.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(
        cancelled.is_ok(),
        "upstream body was not dropped after the client went away"
    );
}

// ---------------------------------------------------------------------------
// Non-streaming providers and validation
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_stream_providers_hit_v1_endpoints() {
    let mock = Mock::start(|r| match r.path.as_str() {
        "/openai/v1/chat/completions"
        | "/groq/openai/v1/chat/completions"
        | "/openrouter/api/v1/chat/completions" => Reply::json(
            200,
            json!({ "choices": [{ "message": { "content": format!("ok {}", r.path) } }] }),
        ),
        "/v1beta/models/gemini-2.5-flash:generateContent" => Reply::json(200, gemini_chunk("gem")),
        "/anthropic/v1/messages" => Reply::json(200, json!({ "content": [{ "text": "claude" }] })),
        _ => Reply::not_found(),
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    let send = |provider: i64, model: &str| {
        let payload = json!({ "messages": user_messages(), "provider": provider, "apiKey": "key", "model": model });
        let backend = backend.clone();
        async move { complete(&backend, &body(&payload), &client()).await }
    };
    assert_eq!(
        send(0, "").await,
        response_json("ok /openai/v1/chat/completions")
    );
    assert_eq!(send(1, "gpt-4o").await, response_json("gem"));
    assert_eq!(send(2, "gemini-x").await, response_json("claude"));
    assert_eq!(
        send(3, "openai/gpt-oss-120b").await,
        response_json("ok /groq/openai/v1/chat/completions")
    );
    assert_eq!(
        send(6, "GPT-5").await,
        response_json("ok /openrouter/api/v1/chat/completions")
    );
    let reqs = mock.requests();
    let claude = reqs
        .iter()
        .find(|r| r.path == "/anthropic/v1/messages")
        .expect("claude request");
    assert_eq!(claude.header("x-api-key").as_deref(), Some("key"));
    assert_eq!(
        claude.header("anthropic-version").as_deref(),
        Some("2023-06-01")
    );
    assert_eq!(
        claude.text(),
        r#"{"model":"claude-3-7-sonnet-20250219","max_tokens":2048,"system":"S","messages":[{"role":"user","content":"U"}]}"#
    );
    let groq = reqs
        .iter()
        .find(|r| r.path.starts_with("/groq"))
        .expect("groq request");
    assert_eq!(groq.json()["model"], json!("qwen3-32b"));
    assert_eq!(groq.json()["temperature"], json!(0.85));
    let openrouter = reqs
        .iter()
        .find(|r| r.path.starts_with("/openrouter"))
        .expect("openrouter request");
    assert_eq!(openrouter.json()["model"], json!("google/gemini-2.5-flash"));
    assert_eq!(
        openrouter.header("authorization").as_deref(),
        Some("Bearer key")
    );
}

#[tokio::test]
async fn non_stream_upstream_error_uses_e_ns_and_passes_status() {
    let mock =
        Mock::start(|_| Reply::json(401, json!({ "error": { "message": "bad key" } }))).await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    let payload = json!({ "messages": user_messages(), "provider": 0, "apiKey": "k" });
    assert_eq!(
        complete(&backend, &body(&payload), &client()).await,
        ApiReply::error(
            401,
            "401 Unauthorized\n{\"error\":{\"message\":\"bad key\"}}"
        )
    );
    let text_mock = Mock::start(|_| Reply::text(503, "<html>down</html>")).await;
    let backend = backend_with(
        desktop_config(&dir, "http://unused"),
        Some(&text_mock),
        |_| {},
    );
    assert_eq!(
        complete(&backend, &body(&payload), &client()).await,
        ApiReply::error(503, "503 Service Unavailable\nnull")
    );
}

#[tokio::test]
async fn upstream_reason_phrase_is_used_like_status_text() {
    // Anthropic-style 529 with a non-canonical reason phrase (no canonical one exists).
    let mock = Mock::start(|_| Reply {
        reason: Some(b"Overloaded"),
        ..Reply::json(
            529,
            json!({ "type": "error", "error": { "type": "overloaded_error" } }),
        )
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    let claude = json!({ "messages": user_messages(), "provider": 2, "apiKey": "k" });
    assert_eq!(
        complete(&backend, &body(&claude), &client()).await,
        ApiReply::error(
            529,
            "529 Overloaded\n{\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\"}}"
        )
    );
    let groq = json!({ "messages": user_messages(), "provider": 3, "apiKey": "k" });
    assert_eq!(
        events(stream(&backend, &body(&groq), &client()).await).await,
        vec![StreamEvent::error(
            "529 Overloaded: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\"}}"
        )]
    );
}

#[tokio::test]
async fn validation_errors_match_v1() {
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), None, |_| {});
    let invalid = ApiReply::error(400, ERR_INVALID_PAYLOAD);
    assert_eq!(complete(&backend, b"not json", &client()).await, invalid);
    assert_eq!(
        complete(&backend, b"{\"messages\":\"x\"}", &client()).await,
        invalid
    );
    assert_eq!(reply(stream(&backend, b"[]", &client()).await), invalid);
    let no_key = json!({ "messages": [], "provider": 1 });
    assert_eq!(
        complete(&backend, &body(&no_key), &client()).await,
        ApiReply::error(400, ERR_NO_API_KEY)
    );
    assert_eq!(
        events(stream(&backend, &body(&no_key), &client()).await).await,
        vec![StreamEvent::error(ERR_NO_API_KEY)]
    );
    let unknown = json!({ "messages": [], "provider": "8" });
    assert_eq!(
        complete(&backend, &body(&unknown), &client()).await,
        ApiReply::error(400, ERR_UNKNOWN_PROVIDER)
    );
    assert_eq!(
        events(stream(&backend, &body(&unknown), &client()).await).await,
        vec![StreamEvent::error(ERR_UNKNOWN_PROVIDER)]
    );
    let bad_model = json!({ "messages": [], "provider": 0, "model": 5 });
    assert_eq!(
        complete(&backend, &body(&bad_model), &client()).await,
        ApiReply::Empty { status: 500 }
    );
    // Streaming evaluates `payload.model?.trim()` inside the try block: error event.
    let bad_model_stream = json!({ "messages": [], "provider": 1, "apiKey": "k", "model": 5 });
    assert_eq!(
        events(stream(&backend, &body(&bad_model_stream), &client()).await).await,
        vec![StreamEvent::error("payload.model?.trim is not a function")]
    );
    // ... but the fallback awaits the `/chat` handler directly (not through Next.js): the
    // TypeError it throws outside its try block rejects and reaches the stream's catch.
    let claude_bad_model = json!({ "messages": [], "provider": 2, "apiKey": "k", "model": 5 });
    assert_eq!(
        events(stream(&backend, &body(&claude_bad_model), &client()).await).await,
        vec![StreamEvent::error("payload.model?.trim is not a function")]
    );
    let local_bad_model = json!({ "messages": [], "provider": 7, "model": {} });
    assert_eq!(
        events(stream(&backend, &body(&local_bad_model), &client()).await).await,
        vec![StreamEvent::error("payload.model?.trim is not a function")]
    );
    let null_message = json!({ "messages": [null], "provider": 0 });
    assert_eq!(
        complete(&backend, &body(&null_message), &client()).await,
        ApiReply::Empty { status: 500 }
    );
    assert_eq!(
        reply(stream(&backend, &body(&null_message), &client()).await),
        ApiReply::Empty { status: 500 }
    );
}

#[tokio::test]
async fn local_proxy_forwards_the_whole_payload() {
    let mock = Mock::start(|r| {
        if r.json()["model"] == json!("boom") {
            return Reply::json(500, json!({ "error": "vertexProject is required" }));
        }
        Reply::json(200, json!({ "response": "local-ok" }))
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), None, |_| {});
    let raw = format!(
        r#"{{"messages":[{{"role":"user","content":"U"}}],"provider":7,"apiKey":"secret","model":"m","localProxyHost":" 127.0.0.1 ","localProxyPort":"{}","extra":{{"z":1,"a":2}}}}"#,
        mock.port
    );
    assert_eq!(
        complete(&backend, raw.as_bytes(), &client()).await,
        response_json("local-ok")
    );
    let req = &mock.requests()[0];
    assert_eq!(req.path, "/chat");
    assert_eq!(
        req.text(),
        raw,
        "the payload must be forwarded verbatim (key order kept)"
    );
    assert_eq!(
        events(stream(&backend, raw.as_bytes(), &client()).await).await,
        vec![tok("local-ok"), done("local-ok")]
    );
    let failing = raw.replace("\"model\":\"m\"", "\"model\":\"boom\"");
    assert_eq!(
        complete(&backend, failing.as_bytes(), &client()).await,
        ApiReply::error(500, "vertexProject is required")
    );
    assert_eq!(
        events(stream(&backend, failing.as_bytes(), &client()).await).await,
        vec![StreamEvent::error("vertexProject is required")]
    );
}

// ---------------------------------------------------------------------------
// Rate limiting, counter, chat log (web server mode)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rate_limit_counter_and_log_in_server_mode() {
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), None, |_| {});
    let ip = ClientMeta {
        ip: "198.51.100.1".to_owned(),
    };
    let no_key = body(&json!({ "messages": [], "provider": 0 }));
    for _ in 0..20 {
        assert_eq!(
            complete(&backend, &no_key, &ip).await,
            ApiReply::error(400, ERR_NO_API_KEY)
        );
    }
    assert_eq!(
        complete(&backend, &no_key, &ip).await,
        ApiReply::error(429, ERR_RATE_LIMITED)
    );
    let free =
        body(&json!({ "messages": [{ "role": "user", "content": "hello" }], "provider": 8 }));
    assert_eq!(
        complete(&backend, &free, &ip).await,
        response_json(FREE_FALLBACK_RATE_LIMITED)
    );
    assert_eq!(
        events(stream(&backend, &free, &ip).await).await,
        vec![
            tok(FREE_FALLBACK_RATE_LIMITED),
            done(FREE_FALLBACK_RATE_LIMITED)
        ]
    );
    assert_eq!(
        quota(&backend).await,
        ApiReply::json(200, json!({ "count": 0, "max": 65000, "remaining": 65000 }))
    );

    // Another IP: FreeAPI without AI Studio key and without gcloud → logged comm-error fallback.
    let other = ClientMeta {
        ip: "198.51.100.2".to_owned(),
    };
    assert_eq!(
        complete(&backend, &free, &other).await,
        response_json(FREE_FALLBACK_COMM_ERROR)
    );
    assert_eq!(
        quota(&backend).await,
        ApiReply::json(200, json!({ "count": 1, "max": 65000, "remaining": 64999 }))
    );
    let counter = std::fs::read_to_string(dir.path().join(REQUEST_COUNT_FILE)).unwrap_or_default();
    assert_eq!(counter, "1");
    let log = std::fs::read_to_string(dir.path().join(CHAT_LOG_FILE)).unwrap_or_default();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 1);
    let entry: Value = serde_json::from_str(lines[0]).unwrap_or(Value::Null);
    let keys: Vec<&str> = entry
        .as_object()
        .map(|o| o.keys().map(String::as_str).collect())
        .unwrap_or_default();
    assert_eq!(
        keys,
        [
            "timestamp",
            "ip",
            "provider",
            "count",
            "requestText",
            "replyText"
        ]
    );
    assert_eq!(entry["ip"], json!("198.51.100.2"));
    assert_eq!(entry["provider"], json!(8));
    assert_eq!(entry["count"], json!(1));
    assert_eq!(entry["requestText"], json!("hello"));
    assert_eq!(entry["replyText"], json!(FREE_FALLBACK_COMM_ERROR));
    assert_eq!(entry["timestamp"].as_str().map(str::len), Some(24));
}

#[tokio::test]
async fn global_cap_and_counter_file_are_loaded() {
    let dir = TempDir::new();
    std::fs::create_dir_all(dir.path()).expect("data dir");
    std::fs::write(dir.path().join(REQUEST_COUNT_FILE), "65000").expect("counter");
    let backend = backend_with(server_config(&dir), None, |_| {});
    let missing_provider = body(&json!({ "messages": [] }));
    assert_eq!(
        complete(&backend, &missing_provider, &client()).await,
        ApiReply::error(429, texts::global_cap_message(65_000))
    );
    let free = body(&json!({ "messages": [], "provider": 8 }));
    assert_eq!(
        complete(&backend, &free, &client()).await,
        response_json(FREE_FALLBACK_RATE_LIMITED)
    );
    // `null` does not count against the global cap.
    let null_provider = body(&json!({ "messages": [], "provider": null }));
    assert_eq!(
        complete(&backend, &null_provider, &client()).await,
        ApiReply::error(400, ERR_NO_API_KEY)
    );
    assert_eq!(
        quota(&backend).await,
        ApiReply::json(200, json!({ "count": 65000, "max": 65000, "remaining": 0 }))
    );
}

/// API-013: `BackendConfig::free_api_max` (`AMADEUS_FREEAPI_MAX`) replaces V1's hard-coded 65000
/// in admission, the cap message and the quota; the counter file keeps V1's format.
#[tokio::test]
async fn configured_free_api_cap_drives_admission_message_and_quota() {
    let dir = TempDir::new();
    std::fs::create_dir_all(dir.path()).expect("data dir");
    // V1's live counter (at its 65000 cap) under a raised cap: FreeAPI is admitted again.
    std::fs::write(dir.path().join(REQUEST_COUNT_FILE), "65000").expect("counter");
    let mut config = server_config(&dir);
    config.free_api_max = 65_002;
    let backend = backend_with(config, None, |_| {});
    assert_eq!(
        quota(&backend).await,
        ApiReply::json(200, json!({ "count": 65000, "max": 65002, "remaining": 2 }))
    );
    // A provider-less request counts globally, then fails like V1 (OpenAI without a key).
    let missing_provider = body(&json!({ "messages": [] }));
    for _ in 0..2 {
        assert_eq!(
            complete(&backend, &missing_provider, &client()).await,
            ApiReply::error(400, ERR_NO_API_KEY)
        );
    }
    assert_eq!(
        complete(&backend, &missing_provider, &client()).await,
        ApiReply::error(
            429,
            "FreeAPIの全体リクエスト上限（65002回）に達しました。サービスを停止しています。"
        )
    );
    let free = body(&json!({ "messages": [], "provider": 8 }));
    assert_eq!(
        complete(&backend, &free, &client()).await,
        response_json(FREE_FALLBACK_RATE_LIMITED)
    );
    assert_eq!(
        quota(&backend).await,
        ApiReply::json(200, json!({ "count": 65002, "max": 65002, "remaining": 0 }))
    );
    let counter = std::fs::read_to_string(dir.path().join(REQUEST_COUNT_FILE)).unwrap_or_default();
    assert_eq!(counter, "65002");
}

#[test]
fn non_positive_free_api_cap_is_rejected_on_the_server_only() {
    let dir = TempDir::new();
    for bad in [0, -5] {
        let mut config = server_config(&dir);
        config.free_api_max = bad;
        let err = ChatState::new(&config).err().map(|e| e.to_string());
        assert_eq!(
            err,
            Some(format!(
                "the FreeAPI cap must be a positive number of requests, got {bad}"
            ))
        );
    }
    // The desktop relays FreeAPI to the hosted server and has no limiter.
    let mut config = desktop_config(&dir, "http://unused");
    config.free_api_max = 0;
    assert!(ChatState::new(&config).is_ok());
}

#[tokio::test]
async fn streaming_fallback_reenters_chat_like_v1() {
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), None, |_| {});
    // Missing provider → nonStreamingFallback → `/chat` treats it as OpenAI. The global counter
    // is incremented twice (outer check + inner check under IP "unknown"), V1 quirk Q6.
    let payload = body(&json!({ "messages": [{ "role": "user", "content": "q" }] }));
    assert_eq!(
        events(stream(&backend, &payload, &client()).await).await,
        vec![StreamEvent::error(ERR_NO_API_KEY)]
    );
    assert_eq!(
        state(&backend).limiter.as_ref().map(|l| l.global_count()),
        Some(2)
    );
}

#[tokio::test]
async fn streaming_fallback_surfaces_the_uncaught_model_type_error() {
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), None, |_| {});
    // Missing provider + non-string model: the re-entered handler runs its rate-limit check
    // (and `Q:` line) before `payload.model?.trim()` throws, so the global counter is still
    // incremented twice; the rejection becomes an `error` event.
    let payload = body(&json!({ "messages": [{ "role": "user", "content": "q" }], "model": 5 }));
    assert_eq!(
        events(stream(&backend, &payload, &client()).await).await,
        vec![StreamEvent::error("payload.model?.trim is not a function")]
    );
    let global = || state(&backend).limiter.as_ref().map(|l| l.global_count());
    assert_eq!(global(), Some(2));
    // Served by Next.js, `/chat` answers the same exception with a 500 and an empty body.
    assert_eq!(
        complete(&backend, &payload, &client()).await,
        ApiReply::Empty { status: 500 }
    );
    assert_eq!(global(), Some(3));
    assert!(
        !dir.path().join(CHAT_LOG_FILE).exists(),
        "nothing is logged"
    );
}

// ---------------------------------------------------------------------------
// FreeAPI
// ---------------------------------------------------------------------------

#[tokio::test]
async fn free_api_uses_ai_studio_then_logs() {
    let mock = Mock::start(|r| match r.path.as_str() {
        "/v1beta/models/gemini-3.5-flash-lite:generateContent" => {
            Reply::json(503, json!({ "error": "busy" }))
        }
        "/v1beta/models/gemini-3.1-flash-lite:generateContent" => {
            Reply::json(200, gemini_chunk("free-ok"))
        }
        "/v1beta/models/gemini-3.5-flash-lite:streamGenerateContent?alt=sse" => Reply::sse(&[
            &format!("data: {}\n\n", gemini_chunk("A")),
            &format!("data: {}\n\n", gemini_chunk("B")),
        ]),
        _ => Reply::not_found(),
    })
    .await;
    let dir = TempDir::new();
    let mut config = server_config(&dir);
    config.ai_studio_api_key = Some(" studio-key ".to_owned());
    let backend = backend_with(config, Some(&mock), |_| {});
    let free = body(&json!({ "messages": [{ "role": "user", "content": "q" }], "provider": 8 }));
    assert_eq!(
        complete(&backend, &free, &client()).await,
        response_json("free-ok")
    );
    assert_eq!(
        events(stream(&backend, &free, &client()).await).await,
        vec![tok("A"), tok("B"), done("AB")]
    );
    assert!(mock
        .requests()
        .iter()
        .all(|r| r.header("x-goog-api-key").as_deref() == Some("studio-key")));
    let log = std::fs::read_to_string(dir.path().join(CHAT_LOG_FILE)).unwrap_or_default();
    let replies: Vec<Value> = log
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .map(|v| v["replyText"].clone())
        .collect();
    assert_eq!(replies, vec![json!("free-ok"), json!("AB")]);
}

#[tokio::test]
async fn free_api_falls_back_to_vertex_with_gcloud() {
    let mock = Mock::start(|r| {
        let path = "/vertex/v1/projects/realamadeusfreecredit/locations/global/publishers/google/models/gemini-3.1-flash-lite";
        if r.path == format!("{path}:streamGenerateContent?alt=sse") {
            return Reply::sse(&[&format!("data: {}\n\n", gemini_chunk("V"))]);
        }
        if r.path == format!("{path}:generateContent") {
            return Reply::json(200, gemini_chunk("V1"));
        }
        Reply::not_found()
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |c| {
        c.google.gcloud_program = GcloudProgram::Fixed("ya29.free-token".to_owned());
    });
    // Desktop relays provider 8, so call the server-side producers directly.
    let mut payload = Payload::from_value(
        json!({ "messages": [{ "role": "user", "content": "q" }], "provider": 8, "model": 3 }),
    )
    .expect("payload");
    let result = free_api::fetch_free_api_response(&backend, &mut payload.clone()).await;
    assert_eq!(result, Ok(Ok("V1".to_owned())));
    let got = {
        let b = backend.clone();
        let s = sink::spawn_stream(None, move |mut sink| async move {
            let _ = free_api::create_free_api_stream(&b, &mut payload, &mut sink).await;
        });
        tokio::time::timeout(Duration::from_secs(20), s.collect::<Vec<_>>())
            .await
            .expect("stream")
    };
    assert_eq!(got, vec![tok("V"), done("V")]);
    assert!(mock
        .requests()
        .iter()
        .all(|r| r.header("authorization").as_deref() == Some("Bearer ya29.free-token")));
}

#[tokio::test]
async fn free_api_without_any_credential_returns_the_fallback() {
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), None, |_| {});
    let free = body(&json!({ "messages": [], "provider": 8 }));
    assert_eq!(
        events(stream(&backend, &free, &client()).await).await,
        vec![
            tok(FREE_FALLBACK_COMM_ERROR),
            done(FREE_FALLBACK_COMM_ERROR)
        ]
    );
}

// ---------------------------------------------------------------------------
// Vertex
// ---------------------------------------------------------------------------

struct TestKey {
    private_pem: String,
    public_pem: String,
}

fn test_key() -> &'static TestKey {
    use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey, LineEnding};
    static KEY: std::sync::OnceLock<TestKey> = std::sync::OnceLock::new();
    KEY.get_or_init(|| {
        let key = rsa::PrivateKey::new(&mut getrandom::SysRng, 2048).expect("rsa keygen");
        TestKey {
            private_pem: key
                .to_pkcs8_pem(LineEnding::LF)
                .expect("pkcs8 pem")
                .to_string(),
            public_pem: key
                .to_public_key()
                .to_public_key_pem(LineEnding::LF)
                .expect("spki pem"),
        }
    })
}

fn b64_json(part: &str) -> Value {
    URL_SAFE_NO_PAD
        .decode(part)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or(Value::Null)
}

#[tokio::test]
async fn vertex_service_account_stream_with_region_failover() {
    let mock = Mock::start(|r| {
        if r.path == "/token" {
            return Reply::json(
                200,
                json!({ "access_token": "ya29.sa-token", "expires_in": 3599 }),
            );
        }
        let base = "/vertex/v1/projects/proj-x/locations";
        let tail = "/publishers/google/models/gemini-2.5-flash:streamGenerateContent?alt=sse";
        if r.path == format!("{base}/global{tail}") {
            return Reply::json(503, json!({ "error": "overloaded" }));
        }
        if r.path == format!("{base}/us-west1{tail}") {
            return Reply::sse(&[&format!("data: {}\n\n", gemini_chunk("vx"))]);
        }
        Reply::not_found()
    })
    .await;
    let key = test_key();
    let sa = json!({
        "type": "service_account", "project_id": "sa-proj", "client_email": "sa@proj.iam.gserviceaccount.com",
        "private_key": key.private_pem.replace('\n', "\\n"), "token_uri": format!("{}/token", mock.base)
    })
    .to_string();
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), Some(&mock), |_| {});
    let payload = body(&json!({
        "messages": user_messages(), "provider": 4, "apiKey": sa, "vertexProject": "proj-x", "vertexLocation": "",
        "useGCloudCLI": true
    }));
    assert_eq!(
        events(stream(&backend, &payload, &client()).await).await,
        vec![tok("vx"), done("vx")]
    );
    assert_eq!(
        mock.requests_to("/vertex/v1/projects/proj-x/locations/global/")
            .len(),
        3,
        "3 attempts on a 503"
    );
    assert_eq!(
        mock.requests_to("/vertex/v1/projects/proj-x/locations/us-west1/")
            .len(),
        1
    );
    let sleeps = jitter_of(&backend);
    assert_eq!(sleeps.len(), 2);
    assert!(
        sleeps.iter().all(|d| (300..=799).contains(&d.as_millis())),
        "{sleeps:?}"
    );
    assert!(mock
        .requests_to("/vertex")
        .iter()
        .all(|r| r.header("authorization").as_deref() == Some("Bearer ya29.sa-token")));

    // The JWT assertion is byte-compatible with V1 and verifiable with the public key.
    let token_reqs = mock.requests_to("/token");
    assert_eq!(token_reqs.len(), 1);
    assert_eq!(
        token_reqs[0].header("content-type").as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    let form = token_reqs[0].text();
    let assertion = form
        .strip_prefix("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion=")
        .expect("form body");
    let parts: Vec<&str> = assertion.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(
        URL_SAFE_NO_PAD.decode(parts[0]).ok(),
        Some(br#"{"alg":"RS256","typ":"JWT"}"#.to_vec())
    );
    let claims = b64_json(parts[1]);
    let keys: Vec<&str> = claims
        .as_object()
        .map(|o| o.keys().map(String::as_str).collect())
        .unwrap_or_default();
    assert_eq!(keys, ["iss", "sub", "aud", "scope", "iat", "exp"]);
    assert_eq!(claims["iss"], json!("sa@proj.iam.gserviceaccount.com"));
    assert_eq!(claims["aud"], json!(format!("{}/token", mock.base)));
    assert_eq!(
        claims["scope"],
        json!("https://www.googleapis.com/auth/cloud-platform")
    );
    // `iat` is backdated by 30 s like V1; the lifetime is capped at one hour (spec Q23).
    assert_eq!(
        claims["exp"]
            .as_i64()
            .zip(claims["iat"].as_i64())
            .map(|(e, i)| e - i),
        Some(3600)
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
    let iat = claims["iat"].as_i64().unwrap_or(0);
    assert!(
        (now - 40..=now - 29).contains(&iat),
        "iat {iat} is not ~30 s before {now}"
    );
    let decoding =
        jsonwebtoken::DecodingKey::from_rsa_pem(key.public_pem.as_bytes()).expect("public key");
    let message = format!("{}.{}", parts[0], parts[1]);
    let valid = jsonwebtoken::crypto::verify(
        parts[2],
        message.as_bytes(),
        &decoding,
        jsonwebtoken::Algorithm::RS256,
    );
    assert_eq!(valid.ok(), Some(true));

    // The service-account token is cached (V1 exchanged it on every request).
    assert_eq!(
        events(stream(&backend, &payload, &client()).await).await,
        vec![tok("vx"), done("vx")]
    );
    assert_eq!(mock.requests_to("/token").len(), 1);
}

#[tokio::test]
async fn vertex_stream_errors_and_silent_close() {
    let mock = Mock::start(|r| {
        if r.path.contains("/locations/global/") {
            return Reply::json(404, json!({ "error": "model not found" }));
        }
        if r.path.contains("/locations/us-west1/") {
            // 200 that breaks before the first token: V1 quirk Q5 (silent close).
            return Reply {
                break_after: true,
                ..Reply::sse(&[])
            };
        }
        Reply::not_found()
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), Some(&mock), |_| {});
    let payload =
        body(&json!({ "messages": [], "provider": 4, "apiKey": "AIza-raw", "vertexProject": "p" }));
    assert_eq!(
        events(stream(&backend, &payload, &client()).await).await,
        Vec::<StreamEvent>::new()
    );
    let req = &mock.requests()[0];
    assert_eq!(
        req.header("x-goog-api-key").as_deref(),
        Some("AIza-raw"),
        "non-ya29 tokens are API keys"
    );
    assert!(req.header("authorization").is_none());

    // No project: the credential is checked first, then the project.
    let no_project = body(&json!({ "messages": [], "provider": 4, "apiKey": "AIza-raw" }));
    assert_eq!(
        events(stream(&backend, &no_project, &client()).await).await,
        vec![StreamEvent::error(ERR_VERTEX_PROJECT_MISSING)]
    );
    let no_cred = body(&json!({ "messages": [], "provider": 4, "vertexProject": "p" }));
    assert_eq!(
        events(stream(&backend, &no_cred, &client()).await).await,
        vec![StreamEvent::error(ERR_VERTEX_NO_CREDENTIALS)]
    );
    let bad_key_type =
        body(&json!({ "messages": [], "provider": 4, "apiKey": 5, "vertexProject": "p" }));
    assert_eq!(
        events(stream(&backend, &bad_key_type, &client()).await).await,
        vec![StreamEvent::error("payloadApiKey?.trim is not a function")]
    );
}

#[tokio::test]
async fn vertex_non_stream_failover_and_aggregate_error() {
    let ok_mock = Mock::start(|r| {
        if r.path.contains("/locations/global/") {
            Reply::json(429, json!({ "error": "quota" }))
        } else if r.path.contains("/locations/us-west1/") {
            Reply::text(400, "bad")
        } else if r.path.contains("/locations/us-west4/") {
            Reply::json(200, gemini_chunk("a-ok"))
        } else {
            Reply::not_found()
        }
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), Some(&ok_mock), |_| {});
    let payload = body(&json!({
        "messages": [], "provider": 4, "apiKey": "AIza-raw", "vertexProject": "p", "vertexLocation": "us-central1", "useGCloudCLI": true
    }));
    assert_eq!(
        complete(&backend, &payload, &client()).await,
        response_json("a-ok")
    );
    let sleeps = jitter_of(&backend);
    assert_eq!(sleeps.len(), 1);
    assert!((100..=399).contains(&sleeps[0].as_millis()));

    let fail_mock = Mock::start(|r| {
        if r.path.contains("/locations/global/") {
            Reply::json(429, json!({ "error": "quota" }))
        } else {
            Reply::text(400, "bad")
        }
    })
    .await;
    let backend = backend_with(server_config(&dir), Some(&fail_mock), |_| {});
    let ApiReply::Json { status, body: err } = complete(&backend, &payload, &client()).await else {
        panic!("expected JSON");
    };
    assert_eq!(status, 502);
    let msg = err["error"].as_str().unwrap_or_default().to_owned();
    assert!(msg.starts_with(
        "Vertex AI: All regions failed (global, us-west1, us-west4, us-east1, us-east4, us-east5, us-south1, asia-northeast1, northamerica-northeast1). Errors: global: 429; Vertex AI Error (us-west1): 400 Bad Request\nbad; "
    ));
    assert!(msg.ends_with("Vertex AI Error (northamerica-northeast1): 400 Bad Request\nbad"));
}

#[tokio::test]
async fn vertex_api_key_candidates_without_gcloud_flag() {
    let mock = Mock::start(|r| {
        if r.path
            .starts_with("/vertex/v1/projects/p1/locations/asia-northeast1/")
        {
            Reply::json(404, json!({ "error": "nf" }))
        } else if r.path == "/v1beta/models/gemini-2.5-flash:generateContent" {
            Reply::json(200, gemini_chunk("b3-ok"))
        } else {
            Reply::not_found()
        }
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), Some(&mock), |_| {});
    let payload = body(&json!({
        "messages": [], "provider": 4, "apiKey": " AIza key/+ ", "vertexProject": "p1", "vertexLocation": "asia-northeast1",
        "useGCloudCLI": false
    }));
    assert_eq!(
        complete(&backend, &payload, &client()).await,
        response_json("b3-ok")
    );
    let reqs = mock.requests();
    assert_eq!(
        reqs[0].path,
        "/vertex/v1/projects/p1/locations/asia-northeast1/publishers/google/models/gemini-2.5-flash:generateContent?key=AIza%20key%2F%2B"
    );
    assert!(reqs
        .iter()
        .all(|r| r.header("x-goog-api-key").as_deref() == Some("AIza key/+")));
    let broken_json =
        body(&json!({ "messages": [], "provider": 4, "apiKey": "{oops", "useGCloudCLI": false }));
    assert_eq!(
        complete(&backend, &broken_json, &client()).await,
        ApiReply::error(400, ERR_VERTEX_SA_JSON_INVALID)
    );
    let no_cred = body(&json!({ "messages": [], "provider": 4, "useGCloudCLI": false }));
    assert_eq!(
        complete(&backend, &no_cred, &client()).await,
        ApiReply::error(400, ERR_VERTEX_NO_CREDENTIALS_FOUND)
    );
}

/// Mock OAuth token endpoint plus a Vertex credential check, shared with a mock handler.
#[derive(Default)]
struct Issuer {
    /// Token exchanges served so far (tokens are `ya29.t1`, `ya29.t2`, …).
    issued: AtomicUsize,
    /// `expires_in` of the tokens issued next.
    expires_in: Mutex<Value>,
    /// Bearer tokens Vertex rejects with 401.
    revoked: Mutex<Vec<String>>,
    /// Vertex rejects every token with 401.
    deny_all: AtomicBool,
}

impl Issuer {
    fn new(expires_in: Value) -> Arc<Self> {
        let issuer = Arc::new(Issuer::default());
        issuer.set_expires_in(expires_in);
        issuer
    }

    fn set_expires_in(&self, v: Value) {
        *self.expires_in.lock().unwrap_or_else(|e| e.into_inner()) = v;
    }

    fn revoke(&self, token: &str) {
        self.revoked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(token.to_owned());
    }

    fn deny_all(&self, deny: bool) {
        self.deny_all.store(deny, Ordering::SeqCst);
    }

    fn issued(&self) -> usize {
        self.issued.load(Ordering::SeqCst)
    }

    fn token_reply(&self) -> Reply {
        let n = self.issued.fetch_add(1, Ordering::SeqCst) + 1;
        let expires_in = self
            .expires_in
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        Reply::json(
            200,
            json!({ "access_token": format!("ya29.t{n}"), "expires_in": expires_in }),
        )
    }

    /// The 401 Vertex answers when the request's bearer token is not accepted.
    fn reject(&self, r: &Recorded) -> Option<Reply> {
        let auth = r.header("authorization").unwrap_or_default();
        let token = auth.strip_prefix("Bearer ").unwrap_or_default();
        let revoked = self
            .revoked
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|t| t == token);
        (revoked || self.deny_all.load(Ordering::SeqCst))
            .then(|| Reply::json(401, json!({ "error": "revoked" })))
    }
}

fn sa_json(token_uri: &str, project_id: &str) -> String {
    json!({
        "type": "service_account", "project_id": project_id,
        "client_email": "sa@proj.iam.gserviceaccount.com",
        "private_key": test_key().private_pem.replace('\n', "\\n"), "token_uri": token_uri
    })
    .to_string()
}

/// (region, Authorization header) of every Vertex request so far.
fn vertex_calls(mock: &Mock) -> Vec<(String, String)> {
    mock.requests_to("/vertex/")
        .iter()
        .map(|r| {
            let region = r.path.split('/').nth(6).unwrap_or_default().to_owned();
            (region, r.header("authorization").unwrap_or_default())
        })
        .collect()
}

#[tokio::test]
async fn vertex_route_service_account_redoes_a_rejected_cached_token() {
    let issuer = Issuer::new(json!(3599));
    let iss = issuer.clone();
    let mock = Mock::start(move |r| {
        if r.path == "/token" {
            return iss.token_reply();
        }
        if r.path == "/vertex/v1/projects/sa-proj/locations/us-central1/publishers/google/models/gemini-2.5-flash:generateContent" {
            return iss
                .reject(r)
                .unwrap_or_else(|| Reply::json(200, gemini_chunk("b1-ok")));
        }
        Reply::not_found()
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), Some(&mock), |_| {});
    // route.ts B1: `useGCloudCLI` off, the project comes from the credential JSON.
    let payload = body(&json!({
        "messages": user_messages(), "provider": 4,
        "apiKey": sa_json(&format!("{}/token", mock.base), "sa-proj"), "useGCloudCLI": false
    }));
    let ok = response_json("b1-ok");

    // Exchanged once, then reused (V1 exchanged per request; the cache is internal).
    assert_eq!(complete(&backend, &payload, &client()).await, ok);
    assert_eq!(complete(&backend, &payload, &client()).await, ok);
    assert_eq!(issuer.issued(), 1);

    // Vertex rejects the cached token: the call is redone once with a fresh exchange, which is
    // what V1 would have sent in the first place.
    issuer.revoke("ya29.t1");
    assert_eq!(complete(&backend, &payload, &client()).await, ok);
    assert_eq!(issuer.issued(), 2);
    let tokens: Vec<String> = vertex_calls(&mock).into_iter().map(|c| c.1).collect();
    assert_eq!(
        tokens,
        [
            "Bearer ya29.t1",
            "Bearer ya29.t1",
            "Bearer ya29.t1",
            "Bearer ya29.t2"
        ]
    );
    assert_eq!(complete(&backend, &payload, &client()).await, ok);
    assert_eq!(issuer.issued(), 2, "the fresh token is cached");

    // A rejected fresh token gives V1's E_NS error with the upstream status and is not kept.
    issuer.deny_all(true);
    let denied = ApiReply::error(401, "401 Unauthorized\n{\"error\":\"revoked\"}");
    assert_eq!(complete(&backend, &payload, &client()).await, denied);
    assert_eq!(
        issuer.issued(),
        3,
        "cached t2 rejected, fresh t3 rejected too"
    );
    assert_eq!(complete(&backend, &payload, &client()).await, denied);
    assert_eq!(issuer.issued(), 4, "t3 was forgotten");

    // Tokens without a usable lifetime are not cached.
    issuer.deny_all(false);
    issuer.set_expires_in(json!(30));
    assert_eq!(complete(&backend, &payload, &client()).await, ok);
    assert_eq!(complete(&backend, &payload, &client()).await, ok);
    assert_eq!(issuer.issued(), 6);
    // A huge `expires_in` used to panic the request; it is clamped to the 55-minute cap.
    issuer.set_expires_in(json!(1e20));
    assert_eq!(complete(&backend, &payload, &client()).await, ok);
    assert_eq!(complete(&backend, &payload, &client()).await, ok);
    assert_eq!(issuer.issued(), 7);
}

#[tokio::test]
async fn vertex_helpers_restart_once_when_a_cached_token_is_rejected() {
    let issuer = Issuer::new(json!(3599));
    let iss = issuer.clone();
    let mock = Mock::start(move |r| {
        if r.path == "/token" {
            return iss.token_reply();
        }
        if !r.path.starts_with("/vertex/v1/projects/proj-h/locations/") {
            return Reply::not_found();
        }
        if let Some(denied) = iss.reject(r) {
            return denied;
        }
        if r.path.contains(":streamGenerateContent") {
            Reply::sse(&[&format!("data: {}\n\n", gemini_chunk("h-stream"))])
        } else {
            Reply::json(200, gemini_chunk("h-ok"))
        }
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(server_config(&dir), Some(&mock), |_| {});
    let payload = body(&json!({
        "messages": user_messages(), "provider": 4,
        "apiKey": sa_json(&format!("{}/token", mock.base), "sa-proj"), "vertexProject": "proj-h",
        "useGCloudCLI": true
    }));
    let streamed = vec![tok("h-stream"), done("h-stream")];
    assert_eq!(
        events(stream(&backend, &payload, &client()).await).await,
        streamed
    );
    assert_eq!(issuer.issued(), 1);

    // createVertexStream: the cached token is rejected before anything was emitted, so the whole
    // attempt starts over (from the first region) with a fresh token.
    issuer.revoke("ya29.t1");
    assert_eq!(
        events(stream(&backend, &payload, &client()).await).await,
        streamed
    );
    assert_eq!(issuer.issued(), 2);
    // fetchVertexResponse (`/chat` with `useGCloudCLI`): same.
    issuer.revoke("ya29.t2");
    assert_eq!(
        complete(&backend, &payload, &client()).await,
        response_json("h-ok")
    );
    assert_eq!(issuer.issued(), 3);
    let global = |t: &str| ("global".to_owned(), format!("Bearer ya29.{t}"));
    assert_eq!(
        vertex_calls(&mock),
        [
            global("t1"),
            global("t1"),
            global("t2"),
            global("t2"),
            global("t3")
        ]
    );
    assert!(jitter_of(&backend).is_empty());

    // Fresh tokens rejected too: a single restart, then V1's aggregate error, which does not
    // mention the discarded attempt with the cached token.
    let regions = "global, us-west1, us-west4, us-east1, us-east4, us-east5, us-south1, asia-northeast1, northamerica-northeast1";
    issuer.deny_all(true);
    let evs = events(stream(&backend, &payload, &client()).await).await;
    let [StreamEvent::Error { error }] = evs.as_slice() else {
        panic!("expected one error event, got {evs:?}");
    };
    assert!(error.starts_with(&format!(
        "Vertex AI Stream: All regions failed ({regions}). Errors: Vertex AI Error (global): 401\n{{\"error\":\"revoked\"}}; "
    )));
    assert_eq!(error.matches("Vertex AI Error (").count(), 9);
    assert_eq!(
        issuer.issued(),
        4,
        "cached t3 rejected, then one fresh exchange"
    );

    issuer.deny_all(false);
    assert_eq!(
        complete(&backend, &payload, &client()).await,
        response_json("h-ok")
    );
    assert_eq!(issuer.issued(), 5, "t4 was forgotten");
    issuer.deny_all(true);
    let ApiReply::Json { status, body: err } = complete(&backend, &payload, &client()).await else {
        panic!("expected JSON");
    };
    assert_eq!(status, 502);
    let msg = err["error"].as_str().unwrap_or_default();
    assert!(msg.starts_with(&format!(
        "Vertex AI: All regions failed ({regions}). Errors: Vertex AI Error (global): 401 Unauthorized\n{{\"error\":\"revoked\"}}; "
    )));
    assert_eq!(msg.matches("Vertex AI Error (").count(), 9);
    assert_eq!(issuer.issued(), 6);
}

#[tokio::test]
async fn vertex_proxy_gcloud_paths() {
    let mock = Mock::start(|r| {
        if r.path.contains("/locations/us-central1/") {
            return Reply::json(500, json!({ "error": "x" }));
        }
        if r.path.contains("/locations/us-west1/") && r.path.contains(":generateContent") {
            return Reply::json(200, gemini_chunk("proxy-ok"));
        }
        if r.path.contains("/locations/us-west1/") {
            return Reply::sse(&[
                r#"data: {"candidates":[{"content":{"parts":[{"text": "a\nb"}]}}]}"#,
                "\n\n",
            ]);
        }
        Reply::not_found()
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |c| {
        c.google.gcloud_program = GcloudProgram::Fixed("ya29.proxy".to_owned());
    });
    let payload =
        body(&json!({ "messages": [{ "role": "user", "content": "U" }], "vertexProject": "pp" }));
    assert_eq!(
        reply(vertex_proxy(&backend, &payload, false).await),
        response_json("proxy-ok")
    );
    assert_eq!(
        events(vertex_proxy(&backend, &payload, true).await).await,
        vec![tok("a\nb"), done("a\nb")]
    );
    assert!(mock
        .requests()
        .iter()
        .all(|r| r.header("authorization").as_deref() == Some("Bearer ya29.proxy")));
    let no_project = body(&json!({ "messages": [] }));
    assert_eq!(
        reply(vertex_proxy(&backend, &no_project, false).await),
        ApiReply::error(400, ERR_VERTEX_PROJECT_MISSING)
    );
    let bad_model = body(&json!({ "messages": [], "model": [] }));
    assert_eq!(
        reply(vertex_proxy(&backend, &bad_model, false).await),
        ApiReply::Empty { status: 500 }
    );
    // `buildGeminiBody` throws on a `null` message, after the token and project checks.
    let null_message = body(&json!({ "messages": [null], "vertexProject": "pp" }));
    assert_eq!(
        reply(vertex_proxy(&backend, &null_message, true).await),
        ApiReply::Empty { status: 500 }
    );
    let null_no_project = body(&json!({ "messages": [null] }));
    assert_eq!(
        reply(vertex_proxy(&backend, &null_no_project, false).await),
        ApiReply::error(400, ERR_VERTEX_PROJECT_MISSING)
    );
    let no_gcloud = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    assert_eq!(
        reply(vertex_proxy(&no_gcloud, &payload, true).await),
        ApiReply::error(503, ERR_VERTEX_PROXY_NO_TOKEN)
    );
}

#[tokio::test]
async fn desktop_vertex_gcloud_stream_and_non_stream() {
    let mock = Mock::start(|r| {
        if r.path.contains("/locations/global/") || r.path.contains("/locations/us-central1/") {
            return Reply::json(429, json!({ "error": "quota" }));
        }
        if r.path.contains("/locations/us-west1/") && r.path.contains(":generateContent") {
            return Reply::json(200, gemini_chunk("desk-once"));
        }
        if r.path.contains("/locations/us-west1/") && r.path.contains(":streamGenerateContent") {
            return Reply::sse(&[&format!("data: {}\n\n", gemini_chunk("desk-stream"))]);
        }
        Reply::not_found()
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |c| {
        c.google.gcloud_program = GcloudProgram::Fixed("ya29.desktop".to_owned());
    });

    // Empty or gpt* model defaults to gemini-2.0-flash, stale apiKey is ignored,
    // default location "us-central1" IS attempted first before failing over to us-west1,
    // and webSearch adds Google Search grounding tools.
    let once_payload = body(&json!({
        "messages": user_messages(),
        "provider": 4,
        "apiKey": "sk-stale-openai",
        "model": "gpt-4o",
        "webSearch": true,
        "vertexProject": "desk-proj",
        "vertexLocation": ""
    }));
    assert_eq!(
        complete(&backend, &once_payload, &client()).await,
        response_json("desk-once")
    );
    let stream_payload = body(&json!({
        "messages": user_messages(),
        "provider": 4,
        "apiKey": "sk-stale-openai",
        "model": "gemini-2.5-flash",
        "webSearch": false,
        "vertexProject": "desk-proj",
        "vertexLocation": "global"
    }));
    assert_eq!(
        events(stream(&backend, &stream_payload, &client()).await).await,
        vec![tok("desk-stream"), done("desk-stream")]
    );

    let reqs = mock.requests();
    assert_eq!(
        reqs.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
        [
            "/vertex/v1/projects/desk-proj/locations/us-central1/publishers/google/models/gemini-2.0-flash:generateContent",
            "/vertex/v1/projects/desk-proj/locations/us-west1/publishers/google/models/gemini-2.0-flash:generateContent",
            "/vertex/v1/projects/desk-proj/locations/global/publishers/google/models/gemini-2.5-flash:streamGenerateContent?alt=sse",
            "/vertex/v1/projects/desk-proj/locations/us-central1/publishers/google/models/gemini-2.5-flash:streamGenerateContent?alt=sse",
            "/vertex/v1/projects/desk-proj/locations/us-west1/publishers/google/models/gemini-2.5-flash:streamGenerateContent?alt=sse",
        ]
    );
    assert!(reqs
        .iter()
        .all(|r| r.header("authorization").as_deref() == Some("Bearer ya29.desktop")));
    assert!(reqs.iter().all(|r| r.header("x-goog-api-key").is_none()));
    let body0: Value = serde_json::from_slice(&reqs[0].body).expect("json body");
    assert_eq!(body0["tools"], json!([{ "googleSearch": {} }]));

    let sleeps = jitter_of(&backend);
    assert_eq!(sleeps.len(), 3);
    assert!(sleeps.iter().all(|d| (100..=400).contains(&d.as_millis())));

    // Missing project id.
    let no_project = body(&json!({ "messages": user_messages(), "provider": 4 }));
    assert_eq!(
        complete(&backend, &no_project, &client()).await,
        ApiReply::error(400, ERR_VERTEX_PROJECT_MISSING)
    );
    assert_eq!(
        events(stream(&backend, &no_project, &client()).await).await,
        vec![StreamEvent::error(ERR_VERTEX_PROJECT_MISSING)]
    );

    // Missing gcloud token.
    let no_gcloud = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |_| {});
    let valid_payload =
        body(&json!({ "messages": user_messages(), "provider": 4, "vertexProject": "desk-proj" }));
    assert_eq!(
        complete(&no_gcloud, &valid_payload, &client()).await,
        ApiReply::error(503, ERR_VERTEX_DESKTOP_NO_TOKEN)
    );
    assert_eq!(
        events(stream(&no_gcloud, &valid_payload, &client()).await).await,
        vec![StreamEvent::error(ERR_VERTEX_DESKTOP_NO_TOKEN)]
    );
}

#[tokio::test]
async fn desktop_vertex_cached_token_refreshes_on_401_and_client_error_does_not_failover() {
    let fail_400 = Arc::new(AtomicBool::new(false));
    let break_after_token = Arc::new(AtomicBool::new(false));
    let fail_400_ref = Arc::clone(&fail_400);
    let break_ref = Arc::clone(&break_after_token);
    let mock = Mock::start(move |r| {
        if fail_400_ref.load(Ordering::SeqCst) {
            return Reply::text(400, "bad model");
        }
        if break_ref.load(Ordering::SeqCst) {
            return Reply {
                break_after: true,
                ..Reply::sse(&[&format!("data: {}\n\n", gemini_chunk("partial"))])
            };
        }
        let auth = r.header("authorization").unwrap_or_default();
        if auth == "Bearer ya29.t1" && r.path.contains("/locations/us-central1/") {
            // First call gets 200 if query isn't revoked yet? We'll distinguish via message text.
            let body_str = r.text();
            if body_str.contains("REVOKE_T1") {
                return Reply::json(401, json!({ "error": "expired" }));
            }
        }
        if r.path.contains(":streamGenerateContent") {
            Reply::sse(&[&format!("data: {}\n\n", gemini_chunk("ok-stream"))])
        } else {
            Reply::json(200, gemini_chunk("ok-once"))
        }
    })
    .await;

    let tokens = Arc::new(Mutex::new(VecDeque::from([
        "ya29.t1".to_owned(),
        "ya29.t2".to_owned(),
        "ya29.t3".to_owned(),
    ])));
    let dir = TempDir::new();
    let backend = backend_with(desktop_config(&dir, "http://unused"), Some(&mock), |c| {
        c.google.gcloud_program = GcloudProgram::Sequence(tokens);
    });

    let p_ok = body(&json!({
        "messages": [{ "role": "user", "content": "hi" }],
        "provider": 4,
        "vertexProject": "p",
        "vertexLocation": "us-central1"
    }));
    // Call 1 & 2 reuse cached ya29.t1.
    assert_eq!(
        complete(&backend, &p_ok, &client()).await,
        response_json("ok-once")
    );
    assert_eq!(
        complete(&backend, &p_ok, &client()).await,
        response_json("ok-once")
    );

    // Call 3: 401 on cached ya29.t1 clears cache, fetches ya29.t2, and succeeds.
    let p_revoke = body(&json!({
        "messages": [{ "role": "user", "content": "REVOKE_T1" }],
        "provider": 4,
        "vertexProject": "p",
        "vertexLocation": "us-central1"
    }));
    assert_eq!(
        events(stream(&backend, &p_revoke, &client()).await).await,
        vec![tok("ok-stream"), done("ok-stream")]
    );
    let auths: Vec<String> = mock
        .requests()
        .iter()
        .map(|r| r.header("authorization").unwrap_or_default())
        .collect();
    assert_eq!(
        auths,
        [
            "Bearer ya29.t1",
            "Bearer ya29.t1",
            "Bearer ya29.t1",
            "Bearer ya29.t2"
        ]
    );

    // Call 4 & 5: 400 client error fails immediately on the first region without failover.
    fail_400.store(true, Ordering::SeqCst);
    let before = mock.requests().len();
    let ApiReply::Json { status, body: err } = complete(&backend, &p_ok, &client()).await else {
        panic!("expected JSON");
    };
    assert_eq!(status, 400);
    assert!(err["error"]
        .as_str()
        .unwrap_or_default()
        .starts_with("Vertex AI Error (us-central1): 400 Bad Request\nbad model"));
    assert_eq!(mock.requests().len(), before + 1);

    let evs = events(stream(&backend, &p_ok, &client()).await).await;
    assert_eq!(evs.len(), 1);
    let StreamEvent::Error { error } = &evs[0] else {
        panic!("expected error event, got {evs:?}");
    };
    assert!(error.starts_with("Vertex AI Stream Error (us-central1): 400 Bad Request\nbad model"));
    assert_eq!(mock.requests().len(), before + 2);

    // Mid-stream disconnect after tokens emits Stream Interrupted and does not retry.
    fail_400.store(false, Ordering::SeqCst);
    break_after_token.store(true, Ordering::SeqCst);
    let evs = events(stream(&backend, &p_ok, &client()).await).await;
    assert_eq!(evs.len(), 2);
    assert_eq!(evs[0], tok("partial"));
    assert!(
        matches!(&evs[1], StreamEvent::Error { error } if error.starts_with("Vertex AI Stream Interrupted (us-central1):"))
    );
    assert_eq!(mock.requests().len(), before + 3);
}

// ---------------------------------------------------------------------------
// Desktop relay
// ---------------------------------------------------------------------------

#[tokio::test]
async fn desktop_relays_free_api_and_quota() {
    let mock = Mock::start(|r| match r.path.as_str() {
        "/amadeus/app/api/chat" => Reply::json(200, json!({ "response": "remote-ok" })),
        "/amadeus/app/api/chat/stream" => Reply::sse(&[
            "data: {\"type\":\"token\",\"token\":\"R\"}\n\n",
            "data: {\"type\":\"token\",\"token\":\"\"}\n\ndata: not-json\n\n",
            "data: {\"type\":\"complete\",\"fullResponse\":\"R!\"}\n\n",
            "data: {\"type\":\"token\",\"token\":\"late\"}\n\n",
        ]),
        "/amadeus/app/api/chat/quota" => {
            Reply::json(200, json!({ "count": 5, "max": 65000, "remaining": 64995 }))
        }
        _ => Reply::not_found(),
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(
        desktop_config(&dir, &format!("{}/amadeus/", mock.base)),
        None,
        |_| {},
    );
    let raw = br#"{"messages":[{"role":"user","content":"hi"}],"provider":8,"apiKey":""}"#;
    assert_eq!(
        complete(&backend, raw, &client()).await,
        response_json("remote-ok")
    );
    assert_eq!(mock.requests()[0].body, raw.to_vec());
    assert_eq!(
        events(stream(&backend, raw, &client()).await).await,
        vec![tok("R"), tok("not-json"), done("R!")]
    );
    assert_eq!(
        quota(&backend).await,
        ApiReply::json(200, json!({ "count": 5, "max": 65000, "remaining": 64995 }))
    );
    let methods: Vec<(String, String)> = mock
        .requests()
        .into_iter()
        .map(|r| (r.method, r.path))
        .collect();
    assert_eq!(
        methods,
        [
            ("POST".to_owned(), "/amadeus/app/api/chat".to_owned()),
            ("POST".to_owned(), "/amadeus/app/api/chat/stream".to_owned()),
            ("GET".to_owned(), "/amadeus/app/api/chat/quota".to_owned()),
        ]
    );

    let port = closed_port().await;
    let offline = backend_with(
        desktop_config(&dir, &format!("http://127.0.0.1:{port}/amadeus")),
        None,
        |_| {},
    );
    assert_eq!(
        complete(&offline, raw, &client()).await,
        response_json(FREE_FALLBACK_COMM_ERROR)
    );
    assert_eq!(
        events(stream(&offline, raw, &client()).await).await,
        vec![
            tok(FREE_FALLBACK_COMM_ERROR),
            done(FREE_FALLBACK_COMM_ERROR)
        ]
    );
    assert_eq!(quota(&offline).await, ApiReply::error(502, FETCH_FAILED));
    // Other providers never touch the remote server and are not rate limited.
    let no_key = body(&json!({ "messages": [], "provider": 0 }));
    for _ in 0..25 {
        assert_eq!(
            complete(&offline, &no_key, &client()).await,
            ApiReply::error(400, ERR_NO_API_KEY)
        );
    }
}

#[tokio::test]
async fn desktop_relay_treats_non_json_answers_as_unreachable() {
    // A CDN error page while the hosted server is down, and a captive portal answering 200.
    let down = Mock::start(|r| {
        if r.path.ends_with("/stream") {
            return Reply::raw(200, "text/html", vec![b"<html>login</html>".to_vec()]);
        }
        Reply::raw(502, "text/html", vec![b"<html>Bad gateway</html>".to_vec()])
    })
    .await;
    let dir = TempDir::new();
    let backend = backend_with(
        desktop_config(&dir, &format!("{}/amadeus", down.base)),
        None,
        |_| {},
    );
    let raw = br#"{"messages":[{"role":"user","content":"hi"}],"provider":8}"#;
    assert_eq!(
        complete(&backend, raw, &client()).await,
        response_json(FREE_FALLBACK_COMM_ERROR)
    );
    assert_eq!(
        events(stream(&backend, raw, &client()).await).await,
        vec![
            tok(FREE_FALLBACK_COMM_ERROR),
            done(FREE_FALLBACK_COMM_ERROR)
        ]
    );
    assert_eq!(quota(&backend).await, ApiReply::error(502, FETCH_FAILED));

    // JSON answers of the hosted server are passed through whatever their status.
    let json_errors = Mock::start(|_| Reply::json(503, json!({ "error": "maintenance" }))).await;
    let backend = backend_with(
        desktop_config(&dir, &format!("{}/amadeus", json_errors.base)),
        None,
        |_| {},
    );
    assert_eq!(
        complete(&backend, raw, &client()).await,
        ApiReply::error(503, "maintenance")
    );
    assert_eq!(
        reply(stream(&backend, raw, &client()).await),
        ApiReply::error(503, "maintenance")
    );
}

#[test]
fn client_ip_matches_get_client_ip() {
    assert_eq!(
        client_ip(Some(" 1.2.3.4 , 5.6.7.8"), Some("9.9.9.9")),
        "1.2.3.4"
    );
    assert_eq!(client_ip(Some(", 1.2.3.4"), Some(" 9.9.9.9 ")), "9.9.9.9");
    assert_eq!(client_ip(Some("   "), None), "unknown");
    assert_eq!(client_ip(None, None), "unknown");
    assert_eq!(client_ip(None, Some("2001:db8::1")), "2001:db8::1");
}
