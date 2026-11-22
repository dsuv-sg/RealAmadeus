//! Library part of the `amadeus-local-proxy` binary: a port of V1
//! `scripts/amadeus-local-proxy/server.mjs` (spec-server-aux.md §6).
//!
//! A tiny standalone HTTP service that turns a chat request (the provider-7 payload forwarded
//! by `/app/api/chat`) into a Vertex AI Gemini `generateContent` call, authenticated with
//! `gcloud auth print-access-token` and failing over across regions on 429 / 5xx.
//!
//! This module is transport-agnostic: [`LocalProxy::handle`] takes the request method, target
//! and a body future and returns a [`ProxyResponse`]; the binary maps it to HTTP.
//!
//! Deviations from V1 (documented fixes allowed by the spec): the request body is decoded as a
//! whole (no UTF-8 corruption at chunk borders), gcloud's stderr is discarded instead of left
//! unread, the token refresh is single-flight, Windows invocations use Rust's batch-file
//! quoting / `raw_arg` with `CREATE_NO_WINDOW`, and a few extra gcloud locations are tried
//! after the V1 candidates (`/opt/homebrew/bin`, `/snap/bin`, `~/google-cloud-sdk/bin`).

use std::future::Future;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use amadeus_core::jscompat::{js_string, js_trim, js_truthy, utf16_len};
use bytes::Bytes;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Map, Value};
use tokio::time::Instant;
use url::Url;

use crate::aux::js::{js_index, js_member};
use crate::aux::js_json;
use crate::aux::upstream::{read_json_or_null, status_text, FetchFailure};

/// Default listen port (`DEFAULT_PORT`).
pub const DEFAULT_PORT: u16 = 5103;
/// Default bind host (`DEFAULT_HOST`).
pub const DEFAULT_HOST: &str = "0.0.0.0";
/// Model used when the payload has none or asks for a `gpt*` model.
pub const DEFAULT_MODEL: &str = "gemini-2.5-flash";
/// Default Vertex location (`VERTEX_LOCATION` fallback).
pub const DEFAULT_LOCATION: &str = "us-central1";
/// gcloud access tokens are reused for 50 minutes (`TOKEN_CACHE_MS`).
pub const TOKEN_CACHE_TTL: Duration = Duration::from_millis(50 * 60 * 1000);
/// Failover order (`VERTEX_REGIONS`).
pub const VERTEX_REGIONS: [&str; 5] = [
    "us-central1",
    "us-west1",
    "us-east4",
    "asia-northeast1",
    "northamerica-northeast1",
];
/// `gcloud auth print-access-token` is killed after 10 s.
pub const GCLOUD_TIMEOUT: Duration = Duration::from_millis(10_000);
/// Request body limit in UTF-16 code units (`data.length > 2_000_000`).
pub const BODY_LIMIT_UTF16: usize = 2_000_000;
/// Byte cap for reading a body: no body above it can be ≤ [`BODY_LIMIT_UTF16`] UTF-16 units
/// (a UTF-8 byte never yields more than one unit and three bytes yield at least one).
pub const BODY_LIMIT_BYTES: usize = 3 * BODY_LIMIT_UTF16;
/// `generationConfig.maxOutputTokens`.
pub const MAX_OUTPUT_TOKENS: u32 = 8192;
/// Prefix a valid gcloud access token must have.
pub const TOKEN_PREFIX: &str = "ya29.";
/// Error when no gcloud candidate produced a token (verbatim V1 text).
pub const GCLOUD_TOKEN_ERROR: &str =
    "gcloud auth print-access-token に失敗しました。gcloud auth login を確認してください。";
/// CORS headers sent with every response (`writeJson` and the OPTIONS reply).
pub const CORS_HEADERS: [(&str, &str); 3] = [
    ("Access-Control-Allow-Origin", "*"),
    ("Access-Control-Allow-Methods", "GET,POST,OPTIONS"),
    (
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization",
    ),
];
/// `Content-Type` of every JSON response.
pub const JSON_CONTENT_TYPE: &str = "application/json; charset=utf-8";

/// Startup configuration (`LOCAL_PROXY_HOST`, `LOCAL_PROXY_PORT`, `VERTEX_PROJECT`,
/// `VERTEX_LOCATION`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalProxyConfig {
    /// Bind host, echoed as `bindHost` (empty env → `0.0.0.0`).
    pub host: String,
    /// Bind port, echoed as `bindPort` (`Number.parseInt(env || "5103", 10)`).
    pub port: u16,
    /// Raw `VERTEX_PROJECT` (empty when unset).
    pub vertex_project: String,
    /// Raw `VERTEX_LOCATION` (empty when unset).
    pub vertex_location: String,
}

impl LocalProxyConfig {
    /// Reads the process environment.
    pub fn from_env() -> Result<Self, String> {
        Self::from_vars(|name| std::env::var(name).ok())
    }

    /// Builds the config from a variable lookup (testable). Fails like Node's `listen` would for
    /// a port that is not an integer in `0..=65535`.
    pub fn from_vars(get: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let non_empty = |name: &str| get(name).filter(|v| !v.is_empty());
        let host = non_empty("LOCAL_PROXY_HOST").unwrap_or_else(|| DEFAULT_HOST.to_owned());
        let raw_port = non_empty("LOCAL_PROXY_PORT").unwrap_or_else(|| DEFAULT_PORT.to_string());
        let port = js_parse_int(&raw_port)
            .and_then(|p| u16::try_from(p).ok())
            .ok_or_else(|| format!("invalid LOCAL_PROXY_PORT {raw_port:?}: options.port should be >= 0 and < 65536"))?;
        Ok(Self {
            host,
            port,
            vertex_project: get("VERTEX_PROJECT").unwrap_or_default(),
            vertex_location: get("VERTEX_LOCATION").unwrap_or_default(),
        })
    }

    /// The startup log line (`[local-proxy] listening on http://{host}:{port}`).
    pub fn listen_message(&self) -> String {
        format!(
            "[local-proxy] listening on http://{}:{}",
            self.host, self.port
        )
    }
}

/// `Number.parseInt(s, 10)`: leading JS whitespace, optional sign, then as many decimal digits
/// as possible; `None` = `NaN`. Values beyond `i64` saturate (they are invalid ports anyway).
pub fn js_parse_int(s: &str) -> Option<i64> {
    let s = amadeus_core::jscompat::js_trim_start(s);
    let (negative, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let digits: &str = &digits[..digits.bytes().take_while(u8::is_ascii_digit).count()];
    if digits.is_empty() {
        return None;
    }
    let value = digits.bytes().fold(0i64, |acc, d| {
        acc.saturating_mul(10).saturating_add(i64::from(d - b'0'))
    });
    Some(if negative { -value } else { value })
}

/// `getTargetRegions(preferredLocation)`: the preferred location first, then the remaining
/// [`VERTEX_REGIONS`] in order.
pub fn target_regions(preferred: &str) -> Vec<String> {
    let mut targets: Vec<String> = Vec::new();
    if !preferred.is_empty() {
        targets.push(preferred.to_owned());
    }
    for region in VERTEX_REGIONS {
        if !targets.iter().any(|t| t == region) {
            targets.push(region.to_owned());
        }
    }
    targets
}

/// `buildVertexUrl(projectId, location, model, method)` (values interpolated verbatim).
pub fn build_vertex_url(project_id: &str, location: &str, model: &str, method: &str) -> String {
    if location == "global" {
        format!("https://aiplatform.googleapis.com/v1/projects/{project_id}/locations/global/publishers/google/models/{model}:{method}")
    } else {
        format!(
            "https://{location}-aiplatform.googleapis.com/v1/projects/{project_id}/locations/{location}/publishers/google/models/{model}:{method}"
        )
    }
}

/// `buildGeminiBody(messages, useGrounding)`: system messages overwrite `system_instruction`
/// (last one wins, even when empty), every other message becomes a `user` / `model` turn.
/// Key order: `contents`, `generationConfig`, `system_instruction`, `tools`.
pub fn build_gemini_body(messages: &[Value], use_grounding: bool) -> Value {
    let mut contents = Vec::new();
    let mut system_instruction = String::new();
    for message in messages {
        let role = js_member(message, "role");
        let content = match js_member(message, "content") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        if role.and_then(Value::as_str) == Some("system") {
            system_instruction = content;
            continue;
        }
        let role = if role.and_then(Value::as_str) == Some("assistant") {
            "model"
        } else {
            "user"
        };
        contents.push(json!({ "role": role, "parts": [{ "text": content }] }));
    }
    let mut body = Map::new();
    body.insert("contents".to_owned(), Value::Array(contents));
    body.insert(
        "generationConfig".to_owned(),
        json!({ "maxOutputTokens": MAX_OUTPUT_TOKENS }),
    );
    if !system_instruction.is_empty() {
        body.insert(
            "system_instruction".to_owned(),
            json!({ "parts": [{ "text": system_instruction }] }),
        );
    }
    if use_grounding {
        body.insert("tools".to_owned(), json!([{ "googleSearch": {} }]));
    }
    Value::Object(body)
}

/// `extractGeminiResponse(payload)`: concatenates all non-thought `text` strings in
/// `candidates[0].content.parts`, or falls back to `"[Parse Error]"`.
pub fn extract_gemini_response(payload: &Value) -> String {
    let parts = js_member(payload, "candidates")
        .and_then(|c| js_index(c, 0))
        .and_then(|c| js_member(c, "content"))
        .and_then(|c| js_member(c, "parts"));
    if let Some(Value::Array(arr)) = parts {
        let mut combined = String::new();
        let mut found_text = false;
        for part in arr {
            if js_member(part, "thought") == Some(&Value::Bool(true)) {
                continue;
            }
            if let Some(Value::String(s)) = js_member(part, "text") {
                combined.push_str(s);
                found_text = true;
            }
        }
        return if found_text {
            combined
        } else {
            "[Parse Error]".to_owned()
        };
    }
    match parts
        .and_then(|p| js_index(p, 0))
        .and_then(|p| js_member(p, "text"))
    {
        Some(Value::String(s)) => s.clone(),
        _ => "[Parse Error]".to_owned(),
    }
}

/// `isRetryableStatus(statusCode)`: 429 or ≥ 500.
pub fn is_retryable_status(status: u16) -> bool {
    status == 429 || status >= 500
}

/// `getGcloudCandidates()` for the current OS, followed by a few V2 extra locations.
pub fn gcloud_candidates() -> Vec<String> {
    gcloud_candidates_for(cfg!(windows), |name| std::env::var(name).ok())
}

/// [`gcloud_candidates`] for an explicit platform / environment (testable).
pub fn gcloud_candidates_for(windows: bool, env: impl Fn(&str) -> Option<String>) -> Vec<String> {
    let non_empty = |name: &str| env(name).filter(|v| !v.is_empty());
    let mut candidates = vec!["gcloud".to_owned()];
    if windows {
        for var in ["LOCALAPPDATA", "ProgramFiles"] {
            if let Some(base) = non_empty(var) {
                candidates.push(format!(
                    "{}\\Google\\Cloud SDK\\google-cloud-sdk\\bin\\gcloud.cmd",
                    base.trim_end_matches(['\\', '/'])
                ));
            }
        }
        if let Some(base) = non_empty("USERPROFILE") {
            candidates.push(format!(
                "{}\\AppData\\Local\\Google\\Cloud SDK\\google-cloud-sdk\\bin\\gcloud.cmd",
                base.trim_end_matches(['\\', '/'])
            ));
        }
    } else {
        candidates.push("/usr/local/bin/gcloud".to_owned());
        candidates.push("/usr/bin/gcloud".to_owned());
        // V2 extras (GUI-launched apps have a minimal PATH); only tried after V1's list.
        candidates.push("/opt/homebrew/bin/gcloud".to_owned());
        candidates.push("/snap/bin/gcloud".to_owned());
        if let Some(home) = non_empty("HOME") {
            candidates.push(format!(
                "{}/google-cloud-sdk/bin/gcloud",
                home.trim_end_matches('/')
            ));
        }
    }
    candidates
}

/// Builds the `gcloud auth print-access-token` command for a candidate.
fn gcloud_command(candidate: &str) -> tokio::process::Command {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut cmd = if candidate.eq_ignore_ascii_case("gcloud") {
            // `gcloud` is `gcloud.cmd` on Windows; PATH lookup of batch files needs cmd.exe.
            let mut c = tokio::process::Command::new("cmd.exe");
            c.raw_arg("/d /s /c \"gcloud auth print-access-token\"");
            c
        } else {
            // Rust quotes `.cmd` / `.bat` invocations safely (routes them through cmd.exe).
            let mut c = tokio::process::Command::new(candidate);
            c.args(["auth", "print-access-token"]);
            c
        };
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = tokio::process::Command::new(candidate);
        cmd.args(["auth", "print-access-token"]);
        cmd
    }
}

/// `runGcloudAccessToken(path)`: the trimmed stdout when gcloud exits with 0 and prints a
/// `ya29.` token within 10 s, else `""` (spawn errors, timeouts and failures are silent).
pub async fn run_gcloud_access_token(candidate: &str) -> String {
    let mut cmd = gcloud_command(candidate);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    match tokio::time::timeout(GCLOUD_TIMEOUT, cmd.output()).await {
        Ok(Ok(output)) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let token = js_trim(&stdout);
            if token.starts_with(TOKEN_PREFIX) {
                token.to_owned()
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

/// gcloud token cache (`cachedToken` / `tokenExpiresAt`, 50 minutes), refreshed single-flight.
pub struct GcloudTokenCache {
    candidates: Vec<String>,
    state: tokio::sync::Mutex<Option<(String, Instant)>>,
}

impl std::fmt::Debug for GcloudTokenCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the cached token.
        f.debug_struct("GcloudTokenCache")
            .field("candidates", &self.candidates)
            .finish_non_exhaustive()
    }
}

impl Default for GcloudTokenCache {
    fn default() -> Self {
        Self::new()
    }
}

impl GcloudTokenCache {
    /// Cache using [`gcloud_candidates`].
    pub fn new() -> Self {
        Self::with_candidates(gcloud_candidates())
    }

    /// Cache trying exactly these candidates in order.
    pub fn with_candidates(candidates: Vec<String>) -> Self {
        Self {
            candidates,
            state: tokio::sync::Mutex::new(None),
        }
    }

    /// `getVertexAccessToken()`: the cached token while valid, else the first candidate that
    /// yields one (absolute candidates that do not exist are skipped).
    pub async fn get(&self) -> Result<String, String> {
        let mut state = self.state.lock().await;
        if let Some((token, expires_at)) = state.as_ref() {
            if !token.is_empty() && Instant::now() < *expires_at {
                return Ok(token.clone());
            }
        }
        for candidate in &self.candidates {
            if candidate != "gcloud" && !Path::new(candidate).exists() {
                continue;
            }
            let token = run_gcloud_access_token(candidate).await;
            if !token.is_empty() {
                *state = Some((token.clone(), Instant::now() + TOKEN_CACHE_TTL));
                return Ok(token);
            }
        }
        Err(GCLOUD_TOKEN_ERROR.to_owned())
    }
}

/// Where access tokens come from.
pub enum TokenSource {
    /// `gcloud auth print-access-token` with the 50-minute cache (V1 behavior).
    Gcloud(GcloudTokenCache),
    /// A fixed token (tests, externally managed credentials).
    Fixed(String),
}

impl std::fmt::Debug for TokenSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenSource::Gcloud(cache) => f.debug_tuple("Gcloud").field(cache).finish(),
            TokenSource::Fixed(_) => f.write_str("Fixed(<redacted>)"),
        }
    }
}

impl TokenSource {
    /// Current access token or the V1 error message.
    pub async fn get(&self) -> Result<String, String> {
        match self {
            TokenSource::Gcloud(cache) => cache.get().await,
            TokenSource::Fixed(token) => Ok(token.clone()),
        }
    }
}

/// Builds a Vertex URL from `(project, region, model, method)`.
pub type VertexUrlBuilder = Arc<dyn Fn(&str, &str, &str, &str) -> String + Send + Sync>;

/// Error while reading the request body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyReadError {
    /// More than [`BODY_LIMIT_BYTES`] bytes.
    TooLarge,
    /// The stream failed (message of the underlying error).
    Failed(String),
}

/// A transport-neutral response of the local proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyResponse {
    /// HTTP status.
    pub status: u16,
    /// `Content-Type` ([`JSON_CONTENT_TYPE`] for JSON replies, none for the OPTIONS reply).
    pub content_type: Option<&'static str>,
    /// Body bytes. Every response also carries [`CORS_HEADERS`].
    pub body: Vec<u8>,
}

impl ProxyResponse {
    /// `writeJson(res, status, body)`.
    pub fn json(status: u16, body: &Value) -> Self {
        Self {
            status,
            content_type: Some(JSON_CONTENT_TYPE),
            body: js_json::stringify(body).into_bytes(),
        }
    }

    /// The OPTIONS reply (204, CORS headers, no content type, empty body).
    pub fn preflight() -> Self {
        Self {
            status: 204,
            content_type: None,
            body: Vec::new(),
        }
    }

    fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(status, &json!({ "error": message.into() }))
    }
}

/// The local proxy service.
pub struct LocalProxy {
    config: LocalProxyConfig,
    http: reqwest::Client,
    tokens: TokenSource,
    vertex_url: VertexUrlBuilder,
}

impl LocalProxy {
    /// Production service: gcloud tokens and the real Vertex endpoints.
    pub fn new(config: LocalProxyConfig, http: reqwest::Client) -> Self {
        Self {
            config,
            http,
            tokens: TokenSource::Gcloud(GcloudTokenCache::new()),
            vertex_url: Arc::new(build_vertex_url),
        }
    }

    /// Replaces the token source.
    pub fn with_token_source(mut self, tokens: TokenSource) -> Self {
        self.tokens = tokens;
        self
    }

    /// Replaces the Vertex URL builder (tests point it at a mock server).
    pub fn with_vertex_url(mut self, builder: VertexUrlBuilder) -> Self {
        self.vertex_url = builder;
        self
    }

    /// The startup configuration.
    pub fn config(&self) -> &LocalProxyConfig {
        &self.config
    }

    /// `GET /health` body.
    pub fn health_body(&self) -> Value {
        json!({
            "available": true,
            "mode": "gcloud-vertex-proxy",
            "bindHost": self.config.host,
            "bindPort": self.config.port,
            "project": self.config.vertex_project,
            "location": if self.config.vertex_location.is_empty() { DEFAULT_LOCATION } else { self.config.vertex_location.as_str() },
        })
    }

    /// `new URL(req.url || "/", "http://{host}:{port}").pathname`.
    pub fn pathname(&self, target: &str) -> String {
        let target = if target.is_empty() { "/" } else { target };
        let joined = Url::parse(&format!("http://{}:{}", self.config.host, self.config.port))
            .or_else(|_| Url::parse("http://localhost/"))
            .and_then(|base| base.join(target));
        match joined {
            Ok(url) => url.path().to_owned(),
            Err(_) => target.split(['?', '#']).next().unwrap_or("/").to_owned(),
        }
    }

    /// Request dispatch (V1 order): `OPTIONS` (any path) → 204; `GET /health`; `POST /chat`;
    /// anything else (including `HEAD /health`) → 404 `{"error":"Not found"}`. The body future
    /// is only awaited for `POST /chat`.
    pub async fn handle<F>(&self, method: &str, target: &str, body: F) -> ProxyResponse
    where
        F: Future<Output = Result<Bytes, BodyReadError>>,
    {
        if method == "OPTIONS" {
            return ProxyResponse::preflight();
        }
        let path = self.pathname(target);
        if method == "GET" && path == "/health" {
            return ProxyResponse::json(200, &self.health_body());
        }
        if method == "POST" && path == "/chat" {
            return self.chat(body.await).await;
        }
        ProxyResponse::error(404, "Not found")
    }

    /// `POST /chat` (every failure inside the V1 `try` becomes 500 with the message).
    pub async fn chat(&self, body: Result<Bytes, BodyReadError>) -> ProxyResponse {
        let payload = match read_json_body(body) {
            Ok(payload) => payload,
            Err(message) => return ProxyResponse::error(500, message),
        };
        let messages: Vec<Value> = match js_member(&payload, "messages") {
            Some(Value::Array(items)) => items.clone(),
            _ => Vec::new(),
        };
        if messages.is_empty() {
            return ProxyResponse::error(400, "messages is required");
        }
        let truthy_string = |key: &str| {
            js_member(&payload, key)
                .filter(|v| js_truthy(v))
                .map(js_string)
        };
        let project_id = js_trim(
            &truthy_string("vertexProject")
                .or_else(|| Some(self.config.vertex_project.clone()).filter(|p| !p.is_empty()))
                .unwrap_or_default(),
        )
        .to_owned();
        if project_id.is_empty() {
            return ProxyResponse::error(
                400,
                "vertexProject is required (or set VERTEX_PROJECT env).",
            );
        }
        let location_raw = truthy_string("vertexLocation")
            .or_else(|| Some(self.config.vertex_location.clone()).filter(|l| !l.is_empty()))
            .unwrap_or_else(|| DEFAULT_LOCATION.to_owned());
        let location = match js_trim(&location_raw) {
            "" => DEFAULT_LOCATION.to_owned(),
            trimmed => trimmed.to_owned(),
        };
        let mut model = js_trim(&truthy_string("model").unwrap_or_default()).to_owned();
        if model.is_empty() || model.to_lowercase().starts_with("gpt") {
            model = DEFAULT_MODEL.to_owned();
        }
        let web_search = js_member(&payload, "webSearch").is_some_and(js_truthy);

        match self
            .call_vertex_with_fallback(&project_id, &location, &model, &messages, web_search)
            .await
        {
            Ok(text) => ProxyResponse::json(200, &json!({ "response": text })),
            Err(message) => ProxyResponse::error(500, message),
        }
    }

    /// `callVertexWithFallback`: one attempt per region; 429/5xx are recorded and the next
    /// region is tried, any other non-2xx fails immediately, a network error propagates (no
    /// failover, V1 parity).
    pub async fn call_vertex_with_fallback(
        &self,
        project_id: &str,
        location: &str,
        model: &str,
        messages: &[Value],
        web_search: bool,
    ) -> Result<String, String> {
        let token = self.tokens.get().await?;
        let body = js_json::stringify(&build_gemini_body(messages, web_search));
        let mut errors: Vec<String> = Vec::new();
        for region in target_regions(location) {
            let url = (self.vertex_url)(project_id, &region, model, "generateContent");
            let parsed =
                Url::parse(&url).map_err(|_| FetchFailure::BadUrl(url.clone()).message())?;
            if !parsed.username().is_empty() || parsed.password().is_some() {
                return Err(FetchFailure::Credentials(url).message());
            }
            let response = self
                .http
                .post(parsed)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(CONTENT_TYPE, "application/json")
                .body(body.clone())
                .send()
                .await
                .map_err(|_| FetchFailure::Failed.message())?;
            let status = response.status().as_u16();
            let status_text = status_text(&response);
            let payload = read_json_or_null(response).await;
            if (200..300).contains(&status) {
                return Ok(extract_gemini_response(&payload));
            }
            if is_retryable_status(status) {
                errors.push(format!("{region}:{status}"));
                continue;
            }
            return Err(format!(
                "Vertex Error ({region}) {status} {status_text}: {}",
                js_json::stringify(&payload)
            ));
        }
        Err(format!("Vertex retryable failures: {}", errors.join(", ")))
    }
}

/// `readJsonBody(req)`: `"Payload too large"` above 2 000 000 UTF-16 units, `{}` for an empty
/// body, `"Invalid JSON"` when `JSON.parse` fails.
pub fn read_json_body(body: Result<Bytes, BodyReadError>) -> Result<Value, String> {
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(BodyReadError::TooLarge) => return Err("Payload too large".to_owned()),
        Err(BodyReadError::Failed(message)) => return Err(message),
    };
    let text = String::from_utf8_lossy(&bytes);
    if utf16_len(&text) > BODY_LIMIT_UTF16 {
        return Err("Payload too large".to_owned());
    }
    if text.is_empty() {
        return Ok(Value::Object(Map::new()));
    }
    serde_json::from_str(&text).map_err(|_| "Invalid JSON".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proxy(project: &str, location: &str) -> LocalProxy {
        let config = LocalProxyConfig {
            host: "127.0.0.1".into(),
            port: 58103,
            vertex_project: project.into(),
            vertex_location: location.into(),
        };
        LocalProxy::new(config, reqwest::Client::new())
            .with_token_source(TokenSource::Fixed("ya29.test".into()))
    }

    fn body(s: &str) -> std::future::Ready<Result<Bytes, BodyReadError>> {
        std::future::ready(Ok(Bytes::from(s.to_owned())))
    }

    fn json_of(res: &ProxyResponse) -> Value {
        serde_json::from_slice(&res.body).unwrap_or(Value::Null)
    }

    #[test]
    fn config_from_vars() {
        let cfg = LocalProxyConfig::from_vars(|_| None).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((cfg.host.as_str(), cfg.port), ("0.0.0.0", 5103));
        assert_eq!(
            cfg.listen_message(),
            "[local-proxy] listening on http://0.0.0.0:5103"
        );
        let vars = |name: &str| match name {
            "LOCAL_PROXY_HOST" => Some(String::new()),
            "LOCAL_PROXY_PORT" => Some(" 5104abc".to_owned()),
            "VERTEX_PROJECT" => Some("p".to_owned()),
            _ => None,
        };
        let cfg = LocalProxyConfig::from_vars(vars).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            (cfg.host.as_str(), cfg.port, cfg.vertex_project.as_str()),
            ("0.0.0.0", 5104, "p")
        );
        assert!(LocalProxyConfig::from_vars(
            |n| (n == "LOCAL_PROXY_PORT").then(|| "abc".to_owned())
        )
        .is_err());
        assert!(LocalProxyConfig::from_vars(
            |n| (n == "LOCAL_PROXY_PORT").then(|| "70000".to_owned())
        )
        .is_err());
    }

    #[test]
    fn parse_int_matches_js() {
        assert_eq!(js_parse_int("5103"), Some(5103));
        assert_eq!(js_parse_int("  42px"), Some(42));
        assert_eq!(js_parse_int("+7"), Some(7));
        assert_eq!(js_parse_int("-3"), Some(-3));
        assert_eq!(js_parse_int("5103.9"), Some(5103));
        assert_eq!(js_parse_int("abc"), None);
        assert_eq!(js_parse_int(""), None);
        assert_eq!(js_parse_int("0x10"), Some(0));
    }

    #[test]
    fn regions_and_urls() {
        assert_eq!(
            target_regions("asia-northeast1"),
            [
                "asia-northeast1",
                "us-central1",
                "us-west1",
                "us-east4",
                "northamerica-northeast1"
            ]
        );
        assert_eq!(
            target_regions("global"),
            [
                "global",
                "us-central1",
                "us-west1",
                "us-east4",
                "asia-northeast1",
                "northamerica-northeast1"
            ]
        );
        assert_eq!(target_regions("us-central1").len(), 5);
        assert_eq!(
            build_vertex_url("p", "global", "m", "generateContent"),
            "https://aiplatform.googleapis.com/v1/projects/p/locations/global/publishers/google/models/m:generateContent"
        );
        assert_eq!(
            build_vertex_url("p", "us-east4", "m", "generateContent"),
            "https://us-east4-aiplatform.googleapis.com/v1/projects/p/locations/us-east4/publishers/google/models/m:generateContent"
        );
        assert!(is_retryable_status(429) && is_retryable_status(500) && is_retryable_status(503));
        assert!(!is_retryable_status(404) && !is_retryable_status(400));
    }

    #[test]
    fn gemini_body_matches_v1() {
        let messages: Vec<Value> = serde_json::from_str(
            r#"[{"role":"system","content":"first"},{"role":"user","content":"hi"},null,{"role":"assistant","content":5},{"role":"tool","content":"t"},{"role":"system","content":"sys"}]"#,
        )
        .unwrap_or_default();
        assert_eq!(
            js_json::stringify(&build_gemini_body(&messages, true)),
            r#"{"contents":[{"role":"user","parts":[{"text":"hi"}]},{"role":"user","parts":[{"text":""}]},{"role":"model","parts":[{"text":""}]},{"role":"user","parts":[{"text":"t"}]}],"generationConfig":{"maxOutputTokens":8192},"system_instruction":{"parts":[{"text":"sys"}]},"tools":[{"googleSearch":{}}]}"#
        );
        let only_empty_system: Vec<Value> = serde_json::from_str(
            r#"[{"role":"system","content":"x"},{"role":"system","content":""}]"#,
        )
        .unwrap_or_default();
        assert_eq!(
            js_json::stringify(&build_gemini_body(&only_empty_system, false)),
            r#"{"contents":[],"generationConfig":{"maxOutputTokens":8192}}"#
        );
    }

    #[test]
    fn extract_response() {
        let ok: Value = serde_json::from_str(
            r#"{"candidates":[{"content":{"parts":[{"text":"a"},{"text":"b"}]}}]}"#,
        )
        .unwrap_or(Value::Null);
        assert_eq!(extract_gemini_response(&ok), "ab");
        let with_thought = json!({"candidates":[{"content":{"parts":[{"thought":true,"text":"t"},{"thoughtSignature":"sig"},{"text":"tail"}]}}]});
        assert_eq!(extract_gemini_response(&with_thought), "tail");
        assert_eq!(extract_gemini_response(&Value::Null), "[Parse Error]");
        assert_eq!(
            extract_gemini_response(&json!({"candidates": []})),
            "[Parse Error]"
        );
        assert_eq!(
            extract_gemini_response(
                &json!({"candidates": [{"content": {"parts": [{"text": 1}]}}]})
            ),
            "[Parse Error]"
        );
        assert_eq!(
            extract_gemini_response(
                &json!({"candidates": {"0": {"content": {"parts": [{"text": "o"}]}}}})
            ),
            "o"
        );
    }

    #[test]
    fn candidates_per_platform() {
        let env = |name: &str| match name {
            "LOCALAPPDATA" => Some("C:\\Users\\x\\AppData\\Local".to_owned()),
            "USERPROFILE" => Some("C:\\Users\\x".to_owned()),
            "HOME" => Some("/home/x".to_owned()),
            _ => None,
        };
        assert_eq!(
            gcloud_candidates_for(true, env),
            [
                "gcloud",
                "C:\\Users\\x\\AppData\\Local\\Google\\Cloud SDK\\google-cloud-sdk\\bin\\gcloud.cmd",
                "C:\\Users\\x\\AppData\\Local\\Google\\Cloud SDK\\google-cloud-sdk\\bin\\gcloud.cmd",
            ]
        );
        let unix = gcloud_candidates_for(false, env);
        assert_eq!(
            &unix[..3],
            ["gcloud", "/usr/local/bin/gcloud", "/usr/bin/gcloud"]
        );
        assert_eq!(
            unix.last().map(String::as_str),
            Some("/home/x/google-cloud-sdk/bin/gcloud")
        );
    }

    #[test]
    fn body_reading() {
        assert_eq!(read_json_body(Ok(Bytes::new())), Ok(json!({})));
        assert_eq!(
            read_json_body(Ok(Bytes::from_static(b"{bad"))),
            Err("Invalid JSON".to_owned())
        );
        assert_eq!(
            read_json_body(Ok(Bytes::from_static(b"\xef\xbb\xbf{}"))),
            Err("Invalid JSON".to_owned())
        );
        assert_eq!(
            read_json_body(Err(BodyReadError::TooLarge)),
            Err("Payload too large".to_owned())
        );
        let big = format!("\"{}\"", "a".repeat(BODY_LIMIT_UTF16));
        assert_eq!(
            read_json_body(Ok(Bytes::from(big))),
            Err("Payload too large".to_owned())
        );
        let cjk = format!("\"{}\"", "あ".repeat(BODY_LIMIT_UTF16 - 2));
        assert!(read_json_body(Ok(Bytes::from(cjk))).is_ok());
    }

    #[tokio::test]
    async fn dispatch_matches_v1() {
        let p = proxy("", "");
        let res = p.handle("OPTIONS", "/anything", body("")).await;
        assert_eq!(
            (res.status, res.content_type, res.body.len()),
            (204, None, 0)
        );
        let res = p.handle("GET", "/health?x=1", body("")).await;
        assert_eq!(res.status, 200);
        assert_eq!(
            String::from_utf8_lossy(&res.body),
            r#"{"available":true,"mode":"gcloud-vertex-proxy","bindHost":"127.0.0.1","bindPort":58103,"project":"","location":"us-central1"}"#
        );
        for (method, target) in [
            ("GET", "/health/"),
            ("POST", "/health"),
            ("GET", "/chat"),
            ("HEAD", "/health"),
            ("GET", "//health"),
        ] {
            let res = p.handle(method, target, body("")).await;
            assert_eq!(
                (res.status, json_of(&res)),
                (404, json!({"error": "Not found"})),
                "{method} {target}"
            );
        }
        assert_eq!(p.pathname("/a/../health"), "/health");
    }

    #[tokio::test]
    async fn chat_validation_matches_v1() {
        let p = proxy("", "");
        for raw in [
            "",
            "null",
            "[]",
            "\"hi\"",
            "{\"messages\":[]}",
            "{\"messages\":\"x\"}",
        ] {
            let res = p.handle("POST", "/chat", body(raw)).await;
            assert_eq!(
                (res.status, json_of(&res)),
                (400, json!({"error": "messages is required"})),
                "{raw}"
            );
        }
        let res = p.handle("POST", "/chat", body("{bad")).await;
        assert_eq!(
            (res.status, json_of(&res)),
            (500, json!({"error": "Invalid JSON"}))
        );
        let msgs = r#""messages":[{"role":"user","content":"hi"}]"#;
        let res = p
            .handle("POST", "/chat", body(&format!("{{{msgs}}}")))
            .await;
        assert_eq!(
            (res.status, json_of(&res)),
            (
                400,
                json!({"error": "vertexProject is required (or set VERTEX_PROJECT env)."})
            )
        );
        let res = p
            .handle(
                "POST",
                "/chat",
                body(&format!("{{{msgs},\"vertexProject\":\"   \"}}")),
            )
            .await;
        assert_eq!(
            res.status, 400,
            "whitespace project does not fall back to env"
        );
        let with_env = proxy("env-project", "");
        let res = with_env
            .handle(
                "POST",
                "/chat",
                body(&format!("{{{msgs},\"vertexProject\":\"   \"}}")),
            )
            .await;
        assert_eq!(res.status, 400);
    }
}
