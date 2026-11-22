//! Shared helpers: a temporary Trunk-like dist, the router under test (driven with `oneshot`,
//! no listening socket) and a recording mock upstream on 127.0.0.1 standing in for the TTS /
//! STT / RAG servers, Ollama and Vertex AI.

use std::future::IntoFuture;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use amadeus_backend::Backend;
use amadeus_server::{build_router, AppState, ServerConfig};
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Router;
use bytes::Bytes;
use futures_util::StreamExt;
use tempfile::TempDir;
use tower::ServiceExt;

pub const INDEX_HTML: &str =
    "<!doctype html><html><head><base href=\"/amadeus/\" /></head><body>INDEX</body></html>";
pub const HASHED_JS: &str = "amadeus-web-0123456789abcdef.js";
pub const HASHED_WASM: &str = "amadeus-web-0123456789abcdef_bg.wasm";
pub const SMALL_CSS: &str = "tailwind-0123456789abcdef.css";
pub const WASM_BYTES: &[u8] = b"\0asm\x01\0\0\0";
pub const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\nPNGDATA";
pub const MANIFEST: &str = r#"{"name":"Amadeus","start_url":"app"}"#;
/// Content of a file next to (outside) the dist directory; must never be served.
pub const SECRET: &str = "SECRET-OUTSIDE-DIST";

/// The no-store header set every non-resource response must carry.
pub const NO_STORE: [(&str, &str); 5] = [
    (
        "cache-control",
        "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0",
    ),
    ("pragma", "no-cache"),
    ("expires", "0"),
    ("surrogate-control", "no-store"),
    ("cdn-cache-control", "no-store"),
];

/// The `/resources` public header set.
pub const PUBLIC: [(&str, &str); 5] = [
    (
        "cache-control",
        "public, max-age=86400, stale-while-revalidate=604800",
    ),
    ("pragma", "public"),
    ("expires", "86400"),
    ("surrogate-control", "public, max-age=86400"),
    ("cdn-cache-control", "public, max-age=86400"),
];

/// `Cache-Control` of not-found pages.
pub const NOT_FOUND_CC: &str = "private, no-cache, no-store, max-age=0, must-revalidate";

fn write(root: &Path, rel: &str, data: &[u8]) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create dir");
    }
    std::fs::write(path, data).expect("write file");
}

/// A temporary directory holding `dist/` (shaped like Trunk's output) and, next to it, a
/// secret file that path traversal must not reach.
pub struct Dist {
    pub root: TempDir,
}

impl Dist {
    pub fn path(&self) -> PathBuf {
        self.root.path().join("dist")
    }
}

pub fn make_dist() -> Dist {
    let root = tempfile::tempdir().expect("tempdir");
    write(root.path(), "secret.txt", SECRET.as_bytes());
    let dist = root.path().join("dist");
    write(&dist, "index.html", INDEX_HTML.as_bytes());
    write(
        &dist,
        HASHED_JS,
        "console.log('amadeus');\n".repeat(200).as_bytes(),
    );
    write(&dist, HASHED_WASM, WASM_BYTES);
    write(&dist, SMALL_CSS, b"body{}");
    write(&dist, "manifest.webmanifest", MANIFEST.as_bytes());
    write(&dist, "resources/images/Amadeus_BG.png", PNG_BYTES);
    write(
        &dist,
        "resources/fonts/Eurostile Condensed Bold.otf",
        b"OTTO-font",
    );
    write(
        &dist,
        "resources/models/\u{7d05}\u{8389}\u{6816}/model.json",
        b"{\"v\":1}",
    );
    write(&dist, "static/amadeus_logo.png", PNG_BYTES);
    Dist { root }
}

/// The router under test plus the temporary directories it uses.
pub struct TestApp {
    pub router: Router,
    pub dist: Dist,
    pub data: TempDir,
}

/// App with the default config (`/amadeus`, no allowlist), tweaked by `f`.
pub fn app_with(f: impl FnOnce(&mut ServerConfig)) -> TestApp {
    app_full(&[], f)
}

/// App whose data directory is seeded with `data_files` before the backend starts.
pub fn app_full(data_files: &[(&str, &[u8])], f: impl FnOnce(&mut ServerConfig)) -> TestApp {
    let dist = make_dist();
    let data = tempfile::tempdir().expect("tempdir");
    for (name, content) in data_files {
        write(data.path(), name, content);
    }
    let mut config = ServerConfig::new(dist.path(), data.path());
    f(&mut config);
    let backend = Backend::new(config.backend_config()).expect("backend");
    let router = build_router(AppState::new(backend, &config));
    TestApp { router, dist, data }
}

/// App with the default config.
pub fn app() -> TestApp {
    app_with(|_| {})
}

/// A buffered response.
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl TestResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn json(&self) -> serde_json::Value {
        serde_json::from_slice(&self.body).unwrap_or(serde_json::Value::Null)
    }

    /// Asserts every `(name, value)` pair (exact, single-valued).
    pub fn assert_headers(&self, set: &[(&str, &str)]) {
        for (name, value) in set {
            assert_eq!(self.header(name), Some(*value), "header {name}");
            assert_eq!(
                self.headers.get_all(*name).iter().count(),
                1,
                "header {name} count"
            );
        }
    }

    /// Asserts a JSON reply: status, `content-type: application/json`, exact body text.
    pub fn assert_json(&self, status: u16, body: &str) {
        assert_eq!(
            self.status.as_u16(),
            status,
            "status (body {})",
            self.text()
        );
        assert_eq!(self.header("content-type"), Some("application/json"));
        assert_eq!(self.text(), body);
    }

    /// Asserts V1's empty replies (status only: no body, no content type).
    pub fn assert_empty(&self, status: u16) {
        assert_eq!(
            self.status.as_u16(),
            status,
            "status (body {})",
            self.text()
        );
        assert!(self.body.is_empty(), "unexpected body {:?}", self.text());
        assert!(self.header("content-type").is_none());
    }
}

impl TestApp {
    /// Sends a request through the full router and buffers the response.
    pub async fn send(&self, req: Request) -> TestResponse {
        let res = self
            .router
            .clone()
            .oneshot(req)
            .await
            .expect("infallible router");
        buffer(res).await
    }

    /// Sends a request and returns the unbuffered response.
    pub async fn send_raw(&self, req: Request) -> Response {
        self.router
            .clone()
            .oneshot(req)
            .await
            .expect("infallible router")
    }

    pub async fn get(&self, uri: &str) -> TestResponse {
        self.send(request(Method::GET, uri, &[], Body::empty()))
            .await
    }

    pub async fn method(&self, method: Method, uri: &str) -> TestResponse {
        self.send(request(method, uri, &[], Body::empty())).await
    }

    /// POST with an `application/json` content type.
    pub async fn post_json(&self, uri: &str, body: &str) -> TestResponse {
        let headers = [("content-type", "application/json")];
        self.send(request(
            Method::POST,
            uri,
            &headers,
            Body::from(body.to_owned()),
        ))
        .await
    }
}

/// Buffers a response.
pub async fn buffer(res: Response) -> TestResponse {
    let status = res.status();
    let headers = res.headers().clone();
    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .expect("body");
    TestResponse {
        status,
        headers,
        body,
    }
}

/// The body's data frames, one entry per frame as produced by the server.
pub async fn frames(res: Response) -> Vec<Bytes> {
    let mut stream = res.into_body().into_data_stream();
    let mut out = Vec::new();
    while let Some(frame) = stream.next().await {
        out.push(frame.expect("body frame"));
    }
    out
}

pub fn request(method: Method, uri: &str, headers: &[(&str, &str)], body: Body) -> Request {
    let mut builder = Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        builder = builder.header(*k, *v);
    }
    builder.body(body).expect("request")
}

/// One part of a hand-built (browser-style) multipart body.
pub enum FormPart<'a> {
    /// A text field.
    Text(&'a str, &'a str),
    /// A file field: name, filename, optional content type, bytes.
    File(&'a str, &'a str, Option<&'a str>, &'a [u8]),
}

/// Boundary used by [`multipart`].
pub const BOUNDARY: &str = "----WebKitFormBoundaryAmadeus7MA4YWxk";

/// `multipart/form-data` body as Chrome serializes a `FormData`.
pub fn multipart(parts: &[FormPart<'_>]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        out.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
        match part {
            FormPart::Text(name, value) => {
                out.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                        .as_bytes(),
                );
            }
            FormPart::File(name, filename, content_type, data) => {
                out.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\n")
                        .as_bytes(),
                );
                if let Some(ct) = content_type {
                    out.extend_from_slice(format!("Content-Type: {ct}\r\n").as_bytes());
                }
                out.extend_from_slice(b"\r\n");
                out.extend_from_slice(data);
                out.extend_from_slice(b"\r\n");
            }
        }
    }
    out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    out
}

/// `Content-Type` header value for [`multipart`] bodies.
pub fn multipart_type() -> String {
    format!("multipart/form-data; boundary={BOUNDARY}")
}

// ---------------------------------------------------------------------------------------------
// Mock upstream
// ---------------------------------------------------------------------------------------------

/// One request received by the mock upstream.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

type Log = Arc<Mutex<Vec<Recorded>>>;

#[derive(Clone)]
struct MockState {
    log: Log,
    addr: SocketAddr,
}

/// A running mock upstream (`base` = `http://127.0.0.1:<port>`).
pub struct MockUpstream {
    pub base: String,
    log: Log,
}

impl MockUpstream {
    /// `{base}/{kind}` server URL, e.g. `good`, `bad`, `slow`.
    pub fn url(&self, kind: &str) -> String {
        format!("{}/{kind}", self.base)
    }

    /// The mock's port (for allowlist entries).
    pub fn port(&self) -> &str {
        self.base.rsplit(':').next().unwrap_or_default()
    }

    /// Requests received after the first `skip` ones.
    pub fn requests_since(&self, skip: usize) -> Vec<Recorded> {
        self.requests().into_iter().skip(skip).collect()
    }

    /// Requests received so far.
    pub fn requests(&self) -> Vec<Recorded> {
        self.log.lock().map(|l| l.clone()).unwrap_or_default()
    }

    /// The last request whose path ends with `suffix`.
    pub fn last(&self, suffix: &str) -> Recorded {
        self.requests()
            .into_iter()
            .rev()
            .find(|r| r.path.ends_with(suffix))
            .unwrap_or_else(|| panic!("no upstream request ending with {suffix}"))
    }
}

pub const GOOD_HEALTH: &str = r#"{"status":"ok","available":"yes-from-upstream","runtime":{"using_gpu":true,"device":"cuda","gpu_name":"MockGPU","attn_implementation":"sdpa"},"torch":{"cuda_available":true,"detected_gpu_name":"MockGPU"}}"#;
pub const WAV_BYTES: &[u8] = b"RIFF$\0\0\0WAVEfmt ";
pub const OLLAMA_LINES: [&str; 3] = [
    "{\"message\":{\"content\":\"Hel\"}}\n",
    "{\"message\":{\"content\":\"lo\"}}\n",
    "{\"done\":true}\n",
];
/// Pause before the second line of `/ollamaslow/api/chat`.
pub const SLOW_TAIL: Duration = Duration::from_millis(800);
pub const VERTEX_OK: &str = r#"{"candidates":[{"content":{"parts":[{"thought":true,"text":"dropped"},{"text":"vertex says "},{"text":"hi"}]}}]}"#;

fn raw(status: u16, content_type: Option<&'static str>, body: impl Into<Body>) -> Response {
    let mut res = Response::new(body.into());
    *res.status_mut() = StatusCode::from_u16(status).expect("status");
    if let Some(ct) = content_type {
        res.headers_mut()
            .insert("content-type", axum::http::HeaderValue::from_static(ct));
    }
    res
}

/// An empty redirect response.
fn redirect(status: u16, location: &str) -> Response {
    let mut res = raw(status, None, Body::empty());
    res.headers_mut()
        .insert("location", location.parse().expect("location"));
    res
}

/// Mock Vertex AI: `POST /vertex/{region}/{project}/{model}:{method}`; the project name selects
/// the scenario.
fn vertex(region: &str, project: &str) -> Response {
    let json = Some("application/json");
    match (project, region) {
        ("failover", "us-central1") | ("allretry", "us-central1" | "asia-northeast1") => {
            raw(429, json, r#"{"error":"quota"}"#)
        }
        ("failover", "us-west1") | ("allretry", _) => raw(503, json, r#"{"error":"down"}"#),
        ("notfound", _) => raw(404, json, r#"{"error":{"code":404,"status":"NOT_FOUND"}}"#),
        ("nullbody", _) => raw(400, Some("text/plain"), "oops"),
        ("garbage", _) => raw(200, json, r#"{"candidates":[]}"#),
        _ => raw(200, json, VERTEX_OK),
    }
}

async fn mock_handler(State(state): State<MockState>, req: Request) -> Response {
    let (parts, body) = req.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX)
        .await
        .unwrap_or_default();
    let path = parts.uri.path().to_owned();
    if let Ok(mut log) = state.log.lock() {
        log.push(Recorded {
            method: parts.method.to_string(),
            path: path.clone(),
            headers: parts.headers.clone(),
            body,
        });
    }
    let json = Some("application/json");
    if let Some(rest) = path.strip_prefix("/vertex/") {
        let mut segments = rest.split('/');
        let region = segments.next().unwrap_or_default();
        let project = segments.next().unwrap_or_default();
        return vertex(region, project);
    }
    // `/hop/{n}/{rest}`: a chain of n 307 redirects (relative `Location`), then a JSON reply.
    if let Some((n, rest)) = path
        .strip_prefix("/hop/")
        .and_then(|r| r.split_once('/'))
        .and_then(|(n, rest)| Some((n.parse::<u32>().ok()?, rest)))
    {
        return if n == 0 {
            raw(200, json, r#"{"embedding":[1],"hops":"done"}"#)
        } else {
            redirect(307, &format!("/hop/{}/{rest}", n - 1))
        };
    }
    match (parts.method.as_str(), path.as_str()) {
        ("GET", "/good/health") => raw(200, json, GOOD_HEALTH),
        ("POST", "/good/synthesize") => {
            raw(200, Some("audio/x-wav"), Bytes::from_static(WAV_BYTES))
        }
        ("POST", "/good/transcribe") => {
            raw(200, json, r#"{"text":"こんにちは","provider_used":"mock"}"#)
        }
        ("POST", "/good/embed") => raw(200, json, r#"{"embedding":[0.25,-0.5,1],"dim":3}"#),
        ("GET", "/bad/health") => raw(503, json, r#"{"error":"down"}"#),
        ("POST", "/bad/transcribe") => raw(422, json, r#"{"error":"bad audio"}"#),
        ("POST", "/bad/embed") => raw(500, Some("text/plain"), "embed exploded"),
        ("POST", "/bad/synthesize") => {
            raw(500, Some("text/plain; charset=utf-8"), "合成エラー detail")
        }
        ("GET", "/custom/health") => {
            let mut res = raw(599, json, "{}");
            res.extensions_mut()
                .insert(hyper::ext::ReasonPhrase::from_static(b"Custom Reason"));
            res
        }
        ("GET", "/array/health") => raw(200, json, r#"["a","b"]"#),
        ("GET", "/text/health") => raw(200, Some("text/plain"), "OK"),
        ("POST", "/nocontenttype/synthesize") => raw(200, None, Bytes::from_static(b"AUDIO")),
        ("POST", "/emptycontenttype/synthesize") => {
            raw(200, Some(""), Bytes::from_static(b"AUDIO"))
        }
        ("POST", "/empty/synthesize") => raw(200, Some("audio/wav"), Body::empty()),
        ("POST", "/detail/transcribe") => raw(422, json, r#"{"detail":"x"}"#),
        ("POST", "/objerror/embed") => raw(400, json, r#"{"error":{"code":7}}"#),
        ("POST", "/primitive/embed") => raw(200, json, "false"),
        ("POST", "/nullbody/embed") => raw(200, json, "null"),
        ("GET", "/redirect/health") => redirect(302, &format!("http://{}/good/health", state.addr)),
        // STT redirects: 307/308 must replay the multipart body, 303 switches to GET.
        ("POST", "/r307/transcribe") => {
            redirect(307, &format!("http://{}/good/transcribe", state.addr))
        }
        ("POST", "/r308/transcribe") => redirect(308, "/good/transcribe"),
        ("POST", "/r303/transcribe") => redirect(303, "/good/transcribe"),
        // 3xx statuses that cannot be followed (no `Location`) are relayed as is.
        ("POST", "/found/embed") => raw(302, None, r#"{"error":"moved"}"#),
        ("POST", "/multi/synthesize") => raw(300, json, r#"{"error":"multi"}"#),
        ("GET", "/slow/health") => {
            tokio::time::sleep(Duration::from_secs(7)).await;
            raw(200, json, "{}")
        }
        ("GET", "/slowbody/health") => {
            let stream = futures_util::stream::once(async {
                tokio::time::sleep(Duration::from_millis(1500)).await;
                Ok::<Bytes, std::convert::Infallible>(Bytes::from_static(br#"{"late":true}"#))
            });
            raw(200, json, Body::from_stream(stream))
        }
        ("POST", "/ollama/api/chat") => {
            let stream = futures_util::stream::iter(OLLAMA_LINES).then(|line| async move {
                tokio::time::sleep(Duration::from_millis(20)).await;
                Ok::<Bytes, std::convert::Infallible>(Bytes::from_static(line.as_bytes()))
            });
            raw(200, Some("application/x-ndjson"), Body::from_stream(stream))
        }
        ("POST", "/ollamaslow/api/chat") => {
            // First token immediately, the rest after a pause: proves the relay is not buffered.
            let stream =
                futures_util::stream::iter(OLLAMA_LINES)
                    .enumerate()
                    .then(|(i, line)| async move {
                        if i == 1 {
                            tokio::time::sleep(SLOW_TAIL).await;
                        }
                        Ok::<Bytes, std::convert::Infallible>(Bytes::from_static(line.as_bytes()))
                    });
            raw(200, Some("application/x-ndjson"), Body::from_stream(stream))
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Starts the mock upstream on an ephemeral loopback port.
pub async fn start_mock() -> MockUpstream {
    let log: Log = Arc::new(Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock");
    let addr = listener.local_addr().expect("mock addr");
    let app = Router::new().fallback(mock_handler).with_state(MockState {
        log: log.clone(),
        addr,
    });
    tokio::spawn(axum::serve(listener, app).into_future());
    MockUpstream {
        base: format!("http://{addr}"),
        log,
    }
}

/// A refused upstream (nothing listens on port 1).
pub const REFUSED: &str = "http://127.0.0.1:1";
