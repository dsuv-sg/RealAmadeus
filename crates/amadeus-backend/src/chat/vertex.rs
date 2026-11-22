//! Vertex AI (`spec-server-chat.md` §5.1, §8, §12): region lists, URL building, the
//! `vertex-helpers.ts` access-token resolution and multi-region failover (stream and
//! non-stream), and the gcloud-only `vertex-proxy` endpoint.

use amadeus_core::protocol::{ApiReply, StreamEvent};
use reqwest::StatusCode;
use serde_json::{json, Value};

use super::bodies::build_gemini_body;
use super::extract::{
    extract_gemini_response, extract_vertex_stream_tokens_like_unity,
    log_abnormal_gemini_finish_reason,
};
use super::google::{ServiceAccount, TokenReuse};
use super::net::{self, content_json, Headers};
use super::payload::{Payload, Thrown};
use super::sink::{EventSink, StreamErr};
use super::sse::{data_payload, LineReader, Utf8StreamDecoder};
use super::texts::{
    ERR_FREEAPI_NO_GCLOUD, ERR_VERTEX_DESKTOP_NO_TOKEN, ERR_VERTEX_NO_CREDENTIALS,
    ERR_VERTEX_PROJECT_MISSING, FREE_FALLBACK_COMM_ERROR, PARSE_ERROR,
};
use super::{delay_float_ms, delay_int_ms, state};
use crate::Backend;

/// `vertex-helpers.ts` region list (us-central1 is never attempted there).
pub(crate) const VERTEX_REGIONS: [&str; 9] = [
    "global",
    "us-west1",
    "us-west4",
    "us-east1",
    "us-east4",
    "us-east5",
    "us-south1",
    "asia-northeast1",
    "northamerica-northeast1",
];

/// `vertex-proxy/route.ts` region list.
pub(crate) const VERTEX_PROXY_REGIONS: [&str; 5] = [
    "us-central1",
    "us-west1",
    "us-east4",
    "asia-northeast1",
    "northamerica-northeast1",
];

/// Retry attempts per region in `createVertexStream`.
const STREAM_ATTEMPTS: u32 = 3;

/// Vertex REST method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VertexMethod {
    /// `generateContent`.
    Generate,
    /// `streamGenerateContent` (`?alt=sse`).
    Stream,
}

impl VertexMethod {
    fn as_str(self) -> &'static str {
        match self {
            VertexMethod::Generate => "generateContent",
            VertexMethod::Stream => "streamGenerateContent",
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            VertexMethod::Generate => "",
            VertexMethod::Stream => "?alt=sse",
        }
    }
}

/// `buildVertexUrl(projectId, location, model, method)` (values interpolated raw).
pub(crate) fn build_vertex_url(
    project: &str,
    location: &str,
    model: &str,
    method: VertexMethod,
) -> String {
    let (m, suffix) = (method.as_str(), method.suffix());
    if location == "global" {
        format!("https://aiplatform.googleapis.com/v1/projects/{project}/locations/global/publishers/google/models/{model}:{m}{suffix}")
    } else {
        format!(
            "https://{location}-aiplatform.googleapis.com/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:{m}{suffix}"
        )
    }
}

/// Same path under a fixed origin (test override of the Google hosts).
pub(crate) fn build_vertex_url_at(
    base: &str,
    project: &str,
    location: &str,
    model: &str,
    method: VertexMethod,
) -> String {
    format!(
        "{base}/v1/projects/{project}/locations/{location}/publishers/google/models/{model}:{}{}",
        method.as_str(),
        method.suffix()
    )
}

/// `getVertexTargetRegions` of `vertex-helpers.ts`.
pub(crate) fn target_regions(preferred: &str) -> Vec<String> {
    let preferred = amadeus_core::jscompat::js_trim(preferred);
    let mut targets: Vec<String> = Vec::new();
    if !preferred.is_empty() && preferred != "us-central1" {
        targets.push(preferred.to_owned());
    } else if preferred.is_empty() {
        targets.push("global".to_owned());
    }
    for region in VERTEX_REGIONS {
        if region != "us-central1" && !targets.iter().any(|t| t == region) {
            targets.push(region.to_owned());
        }
    }
    targets
}

/// `getVertexTargetRegions` of `vertex-proxy/route.ts` (us-central1 allowed).
pub(crate) fn proxy_target_regions(preferred: &str) -> Vec<String> {
    let preferred = amadeus_core::jscompat::js_trim(preferred);
    let mut targets: Vec<String> = Vec::new();
    if !preferred.is_empty() {
        targets.push(preferred.to_owned());
    }
    for region in VERTEX_PROXY_REGIONS {
        if !targets.iter().any(|t| t == region) {
            targets.push(region.to_owned());
        }
    }
    targets
}

/// `isRetryableStatus`.
pub(crate) fn is_retryable_status(status: u16) -> bool {
    status == 429 || status >= 500
}

/// `status === 401 || status === 403`: Vertex rejected the credential.
pub(crate) fn is_auth_failure(status: u16) -> bool {
    status == 401 || status == 403
}

/// `resolveVertexModel(payload)`.
pub(crate) fn resolve_vertex_model(payload: &Payload) -> Result<String, Thrown> {
    let model = payload.trim_or_empty("model", "payload.model?.trim")?;
    Ok(
        if model.is_empty() || model.to_lowercase().starts_with("gpt") {
            "gemini-2.5-flash".to_owned()
        } else {
            model
        },
    )
}

/// Auth headers chosen by the `ya29.` prefix (OAuth token vs. API key).
fn auth_headers(token: &str) -> Headers {
    let mut headers: Headers = vec![content_json()];
    if token.starts_with("ya29.") {
        headers.push(("authorization", format!("Bearer {token}")));
    } else {
        headers.push(("x-goog-api-key", token.to_owned()));
    }
    headers
}

/// Credential chosen by the vertex-helpers paths.
pub(crate) struct VertexToken {
    /// OAuth access token (`ya29.…`) or a raw API key.
    token: String,
    /// Service account the token was exchanged for (forgotten from the cache after a 401/403).
    sa: Option<ServiceAccount>,
    /// The token came from the V2 service-account cache (V1 exchanged one per request).
    cached: bool,
}

impl VertexToken {
    fn plain(token: String) -> Self {
        Self {
            token,
            sa: None,
            cached: false,
        }
    }
}

/// `getVertexAccessToken(payload.apiKey)`. The outer `Err` is the `TypeError` thrown for a
/// non-string key; the inner `Err` is V1's `{ error }` result.
pub(crate) async fn get_vertex_access_token(
    backend: &Backend,
    payload: &Payload,
    reuse: TokenReuse,
) -> Result<Result<VertexToken, String>, Thrown> {
    let cred_raw = payload.trim_or_empty("apiKey", "payloadApiKey?.trim")?;
    let google = &state(backend).google;
    let sa = match super::google::parse_service_account_loose(&cred_raw) {
        Some(sa) => Some(sa),
        None => google.embedded_loose().await,
    };
    if let Some(sa) = sa {
        return Ok(
            match google.exchange_loose(backend.http(), &sa, reuse).await {
                Ok(token) => Ok(VertexToken {
                    token: token.token,
                    sa: Some(sa),
                    cached: token.cached,
                }),
                Err(msg) => Err(format!("Vertex token error: {msg}")),
            },
        );
    }
    if !cred_raw.is_empty() {
        return Ok(Ok(VertexToken::plain(cred_raw)));
    }
    Ok(Err(ERR_VERTEX_NO_CREDENTIALS.to_owned()))
}

/// Token for the vertex-helpers paths: gcloud for FreeAPI (provider 8), otherwise the payload
/// credential chain. `Ok(Err(..))` is a V1 `{ error }` result.
async fn helper_token(
    backend: &Backend,
    payload: &Payload,
    reuse: TokenReuse,
) -> Result<Result<VertexToken, String>, Thrown> {
    if payload.provider().is(8) {
        let token = state(backend).google.gcloud_token().await;
        return Ok(if token.is_empty() {
            Err(ERR_FREEAPI_NO_GCLOUD.to_owned())
        } else {
            Ok(VertexToken::plain(token))
        });
    }
    get_vertex_access_token(backend, payload, reuse).await
}

/// A 401/403 from Vertex clears the gcloud cache (V1 `clearGcloudTokenCache()`) and forgets the
/// service-account token that was sent. Returns `true` when that token came from the V2 cache:
/// the caller then redoes the request with a freshly exchanged token, as V1 would have used.
fn after_auth_failure(backend: &Backend, cred: &VertexToken, status: u16) -> bool {
    if !is_auth_failure(status) {
        return false;
    }
    let google = &state(backend).google;
    google.clear_gcloud_token();
    if let Some(sa) = &cred.sa {
        google.forget_sa_token(sa, &cred.token);
    }
    cred.cached
}

/// `fetchVertexResponse(payload)`: one attempt per region. `Ok(Err((error, status)))` is V1's
/// `{ error, status }` result.
pub(crate) async fn fetch_vertex_response(
    backend: &Backend,
    payload: &Payload,
) -> Result<Result<String, (String, u16)>, Thrown> {
    let web_search = payload.truthy("webSearch");
    let model = resolve_vertex_model(payload)?;
    let location = payload.trim_or("vertexLocation", "payload.vertexLocation?.trim", "global")?;
    let project = payload.trim_or_empty("vertexProject", "payload.vertexProject?.trim")?;
    let st = state(backend);
    let mut reuse = TokenReuse::Allow;
    'fresh: loop {
        let cred = match helper_token(backend, payload, reuse).await? {
            Ok(cred) => cred,
            Err(error) => return Ok(Err((error, 503))),
        };
        if project.is_empty() {
            return Ok(Err((ERR_VERTEX_PROJECT_MISSING.to_owned(), 400)));
        }
        let headers = auth_headers(&cred.token);
        let body = net::json_body(&build_gemini_body(payload.messages(), web_search));
        let regions = target_regions(&location);
        let mut errors: Vec<String> = Vec::new();
        for region in &regions {
            let url = st
                .endpoints
                .vertex(&project, region, &model, VertexMethod::Generate);
            let resp = match net::post(backend.http(), &url, &headers, body.clone()).await {
                Ok(resp) => resp,
                Err(Thrown(msg)) => {
                    errors.push(format!("{region}: {msg}"));
                    continue;
                }
            };
            let status = resp.status();
            if status.is_success() {
                let data = net::read_json_or_null(resp).await;
                let text = extract_gemini_response(data.as_ref());
                log_abnormal_gemini_finish_reason(
                    &format!("Vertex AI ({region})"),
                    data.as_ref(),
                    text.chars().count(),
                );
                return Ok(Ok(text));
            }
            if is_retryable_status(status.as_u16()) {
                errors.push(format!("{region}: {}", status.as_u16()));
                st.jitter.sleep(delay_int_ms(100, 300)).await;
                continue;
            }
            if after_auth_failure(backend, &cred, status.as_u16()) {
                // A cached service-account token was rejected: start over with a fresh one.
                reuse = TokenReuse::Fresh;
                continue 'fresh;
            }
            let line = net::StatusLine::of(&resp);
            let err_body = net::read_text_or_empty(resp).await;
            errors.push(format!(
                "Vertex AI Error ({region}): {} {}\n{err_body}",
                line.code, line.text
            ));
        }
        return Ok(Err((
            format!(
                "Vertex AI: All regions failed ({}). Errors: {}",
                regions.join(", "),
                errors.join("; ")
            ),
            502,
        )));
    }
}

/// `emitError` of `createVertexStream`: FreeAPI hides errors behind the fallback reply.
async fn emit_vertex_error(sink: &mut EventSink, free: bool, msg: String) -> Result<(), StreamErr> {
    if free {
        sink.emit(StreamEvent::token(FREE_FALLBACK_COMM_ERROR))
            .await?;
        sink.emit(StreamEvent::complete(FREE_FALLBACK_COMM_ERROR))
            .await?;
    } else {
        sink.emit(StreamEvent::error(msg)).await?;
    }
    Ok(())
}

/// Parse one Gemini SSE data payload and emit its token (`createVertexStream` rules).
async fn emit_gemini_line(
    raw: &str,
    sink: &mut EventSink,
    full: &mut String,
    got_tokens: &mut bool,
) -> Result<(), StreamErr> {
    if raw.is_empty() {
        return Ok(());
    }
    if let Ok(parsed) = serde_json::from_str::<Value>(raw) {
        let token = extract_gemini_response(Some(&parsed));
        let valid_len = if token != PARSE_ERROR {
            token.chars().count()
        } else {
            0
        };
        log_abnormal_gemini_finish_reason(
            "Vertex AI stream",
            Some(&parsed),
            full.chars().count() + valid_len,
        );
        if !token.is_empty() && token != PARSE_ERROR {
            *got_tokens = true;
            full.push_str(&token);
            sink.emit(StreamEvent::token(token)).await?;
        }
    }
    Ok(())
}

/// `createVertexStream(payload)` writing directly into the sink (V1's relays were byte-identical
/// pass-throughs). `Err(Thrown)` is an exception that errors the V1 stream.
pub(crate) async fn create_vertex_stream(
    backend: &Backend,
    payload: &Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let web_search = payload.truthy("webSearch");
    let model = resolve_vertex_model(payload)?;
    let project = payload.trim_or_empty("vertexProject", "payload.vertexProject?.trim")?;
    let location = payload.trim_or("vertexLocation", "payload.vertexLocation?.trim", "global")?;
    let free = payload.provider().is(8);
    let st = state(backend);
    let mut reuse = TokenReuse::Allow;

    'fresh: loop {
        let cred = match helper_token(backend, payload, reuse).await? {
            Ok(cred) => cred,
            Err(error) => return emit_vertex_error(sink, free, error).await,
        };
        if project.is_empty() {
            return emit_vertex_error(sink, free, ERR_VERTEX_PROJECT_MISSING.to_owned()).await;
        }
        let is_api_key = !cred.token.starts_with("ya29.");
        let headers = auth_headers(&cred.token);
        if reuse == TokenReuse::Allow {
            // V1 printed the first 15 characters of the credential; V2 masks it (spec open
            // question 2). A fresh-token restart would print the same masked line again.
            let shown: String = cred.token.chars().take(5).collect();
            super::console_log(&format!(
                "Vertex Request: Using token starting with {shown}*** (isApiKey: {is_api_key})"
            ));
        }

        let body = net::json_body(&build_gemini_body(payload.messages(), web_search));
        let regions = target_regions(&location);
        let mut success = false;
        let mut errors: Vec<String> = Vec::new();

        'regions: for region in &regions {
            if success {
                break;
            }
            let mut attempt = 0;
            let mut response: Option<reqwest::Response> = None;
            while attempt < STREAM_ATTEMPTS && !success {
                attempt += 1;
                let url = st
                    .endpoints
                    .vertex(&project, region, &model, VertexMethod::Stream);
                match net::post(backend.http(), &url, &headers, body.clone()).await {
                    Ok(resp) => response = Some(resp),
                    Err(Thrown(msg)) => {
                        errors.push(format!("{region} (attempt {attempt}): {msg}"));
                        if attempt < STREAM_ATTEMPTS {
                            st.jitter.sleep(delay_float_ms(200.0, 500.0)).await;
                            continue;
                        }
                        break;
                    }
                }
                let status = response
                    .as_ref()
                    .map_or(StatusCode::OK, reqwest::Response::status);
                if !status.is_success() {
                    if after_auth_failure(backend, &cred, status.as_u16()) {
                        // A cached service-account token was rejected. Nothing has been emitted
                        // yet (events only follow a 2xx), so start over with a fresh token.
                        reuse = TokenReuse::Fresh;
                        continue 'fresh;
                    }
                    if is_retryable_status(status.as_u16()) {
                        errors.push(format!("{region} (attempt {attempt}): {}", status.as_u16()));
                        if attempt < STREAM_ATTEMPTS {
                            st.jitter.sleep(delay_int_ms(300, 500)).await;
                            continue;
                        }
                    }
                    break;
                }
                success = true;
                break;
            }

            // NB: after a retryable status followed by network errors, `response` still holds
            // the stale error response (V1 behaves the same).
            let Some(mut resp) = response else {
                continue;
            };
            let status = resp.status();
            if !status.is_success() {
                let err_text = net::read_text_or_empty(resp).await;
                errors.push(format!(
                    "Vertex AI Error ({region}): {}\n{err_text}",
                    status.as_u16()
                ));
                continue;
            }
            if !net::has_body(status) {
                // V1 quirk Q5: `success` is already true, so the stream ends without
                // complete/error.
                errors.push(format!("{region}: no response body"));
                continue;
            }

            let mut full = String::new();
            let mut got_tokens = false;
            let mut reader = LineReader::new();
            loop {
                match net::next_chunk(&mut resp).await {
                    Ok(Some(chunk)) => {
                        for line in reader.push(&chunk) {
                            if let Some(raw) = data_payload(&line) {
                                emit_gemini_line(raw, sink, &mut full, &mut got_tokens).await?;
                            }
                        }
                    }
                    Ok(None) => {
                        let left = reader.leftover();
                        if let Some(raw) = data_payload(amadeus_core::jscompat::js_trim(&left)) {
                            emit_gemini_line(raw, sink, &mut full, &mut got_tokens).await?;
                        }
                        break;
                    }
                    Err(Thrown(msg)) => {
                        if got_tokens {
                            return emit_vertex_error(
                                sink,
                                free,
                                format!("Vertex AI Stream Interrupted ({region}): {msg}"),
                            )
                            .await;
                        }
                        // V1 quirk Q5: interrupted before the first token → silent close.
                        errors.push(format!("{region}: stream interrupted"));
                        continue 'regions;
                    }
                }
            }
            sink.emit(StreamEvent::complete(full)).await?;
            success = true;
        }

        if !success {
            let msg = format!(
                "Vertex AI Stream: All regions failed ({}). Errors: {}",
                regions.join(", "),
                errors.join("; ")
            );
            emit_vertex_error(sink, free, msg).await?;
        }
        return Ok(());
    }
}

/// Validated `vertex-proxy` request.
pub(crate) struct ProxyRequest {
    project: String,
    model: String,
    token: String,
    regions: Vec<String>,
    body: String,
}

/// `POST /chat/vertex-proxy` up to the point where it answers or starts streaming.
pub(crate) async fn proxy_prepare(
    backend: &Backend,
    body: &[u8],
) -> Result<ProxyRequest, ApiReply> {
    let Some(payload) = Payload::parse(body) else {
        return Err(ApiReply::error(400, super::texts::ERR_INVALID_PAYLOAD));
    };
    let web_search = payload.truthy("webSearch");
    // Non-string fields throw before any try block in V1: 500 with an empty body.
    let empty_500 = |_| ApiReply::Empty { status: 500 };
    let model = payload
        .trim_or_empty("model", "payload.model?.trim")
        .map_err(empty_500)?;
    let model = if model.is_empty() || model.to_lowercase().starts_with("gpt") {
        "gemini-2.5-flash".to_owned()
    } else {
        model
    };
    let project = payload
        .trim_or_empty("vertexProject", "payload.vertexProject?.trim")
        .map_err(empty_500)?;
    let location = payload
        .trim_or(
            "vertexLocation",
            "payload.vertexLocation?.trim",
            "us-central1",
        )
        .map_err(empty_500)?;
    let token = state(backend).google.gcloud_token().await;
    if token.is_empty() {
        return Err(ApiReply::error(
            503,
            super::texts::ERR_VERTEX_PROXY_NO_TOKEN,
        ));
    }
    if project.is_empty() {
        return Err(ApiReply::error(400, ERR_VERTEX_PROJECT_MISSING));
    }
    // `buildGeminiBody` reads `message.role`: a `null` entry throws a TypeError outside any try
    // block (`/chat` and `/chat/stream` die earlier, in the question lookup).
    if payload.messages().iter().any(Value::is_null) {
        return Err(ApiReply::Empty { status: 500 });
    }
    let body = net::json_body(&build_gemini_body(payload.messages(), web_search));
    Ok(ProxyRequest {
        project,
        model,
        token,
        regions: proxy_target_regions(&location),
        body,
    })
}

/// `nonStreamVertexResponse`: retryable statuses move on, others fail immediately; network
/// errors are uncaught in V1 (500, empty body).
pub(crate) async fn proxy_once(backend: &Backend, req: &ProxyRequest) -> ApiReply {
    let st = state(backend);
    let headers = [
        ("authorization", format!("Bearer {}", req.token)),
        content_json(),
    ];
    let mut errors: Vec<String> = Vec::new();
    for region in &req.regions {
        let url = st
            .endpoints
            .vertex(&req.project, region, &req.model, VertexMethod::Generate);
        let Ok(resp) = net::post(backend.http(), &url, &headers, req.body.clone()).await else {
            return ApiReply::Empty { status: 500 };
        };
        let status = resp.status();
        if status.is_success() {
            let data = net::read_json_or_null(resp).await;
            return ApiReply::json(
                200,
                json!({ "response": extract_gemini_response(data.as_ref()) }),
            );
        }
        if is_retryable_status(status.as_u16()) {
            errors.push(format!("{region}: {}", status.as_u16()));
            st.jitter.sleep(delay_int_ms(100, 300)).await;
            continue;
        }
        let line = net::StatusLine::of(&resp);
        let err_body = net::read_text_or_empty(resp).await;
        let error = format!(
            "Vertex AI Error ({region}): {} {}\n{err_body}",
            line.code, line.text
        );
        // An invalid status would make `NextResponse.json` throw outside any try block.
        if net::response_status_error(status.as_u16()).is_some() {
            return ApiReply::Empty { status: 500 };
        }
        return ApiReply::error(status.as_u16(), error);
    }
    ApiReply::error(
        502,
        format!(
            "Vertex AI: All regions failed ({}). Errors: {}",
            req.regions.join(", "),
            errors.join("; ")
        ),
    )
}

/// `streamVertexResponse`: chunk-level regex parsing (V1 quirk Q8).
pub(crate) async fn proxy_stream(
    backend: &Backend,
    req: &ProxyRequest,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let st = state(backend);
    let headers = [
        ("authorization", format!("Bearer {}", req.token)),
        content_json(),
    ];
    let mut success = false;
    let mut errors: Vec<String> = Vec::new();
    'regions: for region in &req.regions {
        if success {
            break;
        }
        let url = st
            .endpoints
            .vertex(&req.project, region, &req.model, VertexMethod::Stream);
        let mut resp = match net::post(backend.http(), &url, &headers, req.body.clone()).await {
            Ok(resp) => resp,
            Err(Thrown(msg)) => {
                errors.push(format!("{region}: {msg}"));
                continue;
            }
        };
        let status = resp.status();
        if !status.is_success() {
            if is_retryable_status(status.as_u16()) {
                errors.push(format!("{region}: {}", status.as_u16()));
                st.jitter.sleep(delay_int_ms(100, 300)).await;
                continue;
            }
            let err_text = net::read_text_or_empty(resp).await;
            sink.emit(StreamEvent::error(format!(
                "Vertex AI Error ({region}): {}\n{err_text}",
                status.as_u16()
            )))
            .await?;
            return Ok(());
        }
        if !net::has_body(status) {
            errors.push(format!("{region}: no response body"));
            continue;
        }
        let mut full = String::new();
        let mut got_tokens = false;
        let mut decoder = Utf8StreamDecoder::new();
        loop {
            match net::next_chunk(&mut resp).await {
                Ok(Some(chunk)) => {
                    for tok in extract_vertex_stream_tokens_like_unity(&decoder.decode(&chunk)) {
                        got_tokens = true;
                        full.push_str(&tok);
                        sink.emit(StreamEvent::token(tok)).await?;
                    }
                }
                Ok(None) => break,
                Err(Thrown(msg)) => {
                    if got_tokens {
                        sink.emit(StreamEvent::error(format!(
                            "Vertex AI Stream Interrupted ({region}): {msg}"
                        )))
                        .await?;
                        return Ok(());
                    }
                    errors.push(format!("{region}: stream interrupted"));
                    continue 'regions;
                }
            }
        }
        sink.emit(StreamEvent::complete(full)).await?;
        success = true;
    }
    if !success {
        let msg = format!(
            "Vertex AI Stream: All regions failed ({}). Errors: {}",
            req.regions.join(", "),
            errors.join("; ")
        );
        sink.emit(StreamEvent::error(msg)).await?;
    }
    Ok(())
}

/// Default model for the desktop (Qt/Unity parity) Vertex AI flow.
pub(crate) fn resolve_desktop_vertex_model(payload: &Payload) -> Result<String, Thrown> {
    let model = payload.trim_or_empty("model", "payload.model?.trim")?;
    Ok(
        if model.is_empty() || model.to_lowercase().starts_with("gpt") {
            "gemini-2.0-flash".to_owned()
        } else {
            model
        },
    )
}

/// Desktop non-streaming Vertex AI (`RealAmadeusQT` / `RealAmadeus` Unity parity):
/// authenticates with local `gcloud auth print-access-token` and fails over across regions
/// starting at `vertexLocation` (default `us-central1`) on 429 / 5xx / network errors.
pub(crate) async fn desktop_vertex_once(
    backend: &Backend,
    payload: &Payload,
) -> Result<Result<String, (String, u16)>, Thrown> {
    let web_search = payload.truthy("webSearch");
    let model = resolve_desktop_vertex_model(payload)?;
    let project = payload.trim_or_empty("vertexProject", "payload.vertexProject?.trim")?;
    let location = payload.trim_or(
        "vertexLocation",
        "payload.vertexLocation?.trim",
        "us-central1",
    )?;
    if project.is_empty() {
        return Ok(Err((ERR_VERTEX_PROJECT_MISSING.to_owned(), 400)));
    }
    let body = net::json_body(&build_gemini_body(payload.messages(), web_search));
    let regions = proxy_target_regions(&location);
    let st = state(backend);
    let mut reuse = TokenReuse::Allow;

    'fresh: loop {
        let Some(gcloud) = st.google.gcloud_token_with_meta(reuse).await else {
            return Ok(Err((ERR_VERTEX_DESKTOP_NO_TOKEN.to_owned(), 503)));
        };
        let headers = [
            ("authorization", format!("Bearer {}", gcloud.token)),
            content_json(),
        ];
        let mut errors: Vec<String> = Vec::new();
        for region in &regions {
            let url = st
                .endpoints
                .vertex(&project, region, &model, VertexMethod::Generate);
            let resp = match net::post(backend.http(), &url, &headers, body.clone()).await {
                Ok(resp) => resp,
                Err(Thrown(msg)) => {
                    errors.push(format!("{region}: {msg}"));
                    st.jitter.sleep(delay_int_ms(100, 301)).await;
                    continue;
                }
            };
            let status = resp.status();
            if status.is_success() {
                let data = net::read_json_or_null(resp).await;
                let text = extract_gemini_response(data.as_ref());
                log_abnormal_gemini_finish_reason(
                    &format!("Vertex AI ({region})"),
                    data.as_ref(),
                    text.chars().count(),
                );
                return Ok(Ok(text));
            }
            if is_retryable_status(status.as_u16()) {
                errors.push(format!("{region}: {}", status.as_u16()));
                st.jitter.sleep(delay_int_ms(100, 301)).await;
                continue;
            }
            if is_auth_failure(status.as_u16()) {
                st.google.clear_gcloud_token();
                if gcloud.cached {
                    reuse = TokenReuse::Fresh;
                    continue 'fresh;
                }
            }
            let line = net::StatusLine::of(&resp);
            let err_body = net::read_text_or_empty(resp).await;
            return Ok(Err((
                format!(
                    "Vertex AI Error ({region}): {} {}\n{err_body}",
                    line.code, line.text
                ),
                status.as_u16(),
            )));
        }
        return Ok(Err((
            format!(
                "Vertex AI: All regions failed ({}). Errors: {}",
                regions.join(", "),
                errors.join("; ")
            ),
            502,
        )));
    }
}

/// Desktop streaming Vertex AI (`RealAmadeusQT` / `RealAmadeus` Unity parity):
/// authenticates with local `gcloud auth print-access-token` and streams SSE chunks via
/// `streamGenerateContent?alt=sse`, failing over across regions only before tokens arrive.
pub(crate) async fn desktop_vertex_stream(
    backend: &Backend,
    payload: &Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let web_search = payload.truthy("webSearch");
    let model = resolve_desktop_vertex_model(payload)?;
    let project = payload.trim_or_empty("vertexProject", "payload.vertexProject?.trim")?;
    let location = payload.trim_or(
        "vertexLocation",
        "payload.vertexLocation?.trim",
        "us-central1",
    )?;
    if project.is_empty() {
        sink.emit(StreamEvent::error(ERR_VERTEX_PROJECT_MISSING))
            .await?;
        return Ok(());
    }
    let body = net::json_body(&build_gemini_body(payload.messages(), web_search));
    let regions = proxy_target_regions(&location);
    let st = state(backend);
    let mut reuse = TokenReuse::Allow;

    'fresh: loop {
        let Some(gcloud) = st.google.gcloud_token_with_meta(reuse).await else {
            sink.emit(StreamEvent::error(ERR_VERTEX_DESKTOP_NO_TOKEN))
                .await?;
            return Ok(());
        };
        let headers = [
            ("authorization", format!("Bearer {}", gcloud.token)),
            content_json(),
        ];
        let mut errors: Vec<String> = Vec::new();
        'regions: for region in &regions {
            let url = st
                .endpoints
                .vertex(&project, region, &model, VertexMethod::Stream);
            let mut resp = match net::post(backend.http(), &url, &headers, body.clone()).await {
                Ok(resp) => resp,
                Err(Thrown(msg)) => {
                    errors.push(format!("{region}: {msg}"));
                    st.jitter.sleep(delay_int_ms(100, 301)).await;
                    continue 'regions;
                }
            };
            let status = resp.status();
            if !status.is_success() {
                if is_auth_failure(status.as_u16()) {
                    st.google.clear_gcloud_token();
                    if gcloud.cached {
                        reuse = TokenReuse::Fresh;
                        continue 'fresh;
                    }
                }
                if is_retryable_status(status.as_u16()) {
                    errors.push(format!("{region}: {}", status.as_u16()));
                    st.jitter.sleep(delay_int_ms(100, 301)).await;
                    continue 'regions;
                }
                let line = net::StatusLine::of(&resp);
                let err_body = net::read_text_or_empty(resp).await;
                sink.emit(StreamEvent::error(format!(
                    "Vertex AI Stream Error ({region}): {} {}\n{err_body}",
                    line.code, line.text
                )))
                .await?;
                return Ok(());
            }
            if !net::has_body(status) {
                errors.push(format!("{region}: no response body"));
                continue 'regions;
            }
            let mut full = String::new();
            let mut got_tokens = false;
            let mut reader = LineReader::new();
            loop {
                match net::next_chunk(&mut resp).await {
                    Ok(Some(chunk)) => {
                        for line in reader.push(&chunk) {
                            if let Some(raw) = data_payload(&line) {
                                emit_gemini_line(raw, sink, &mut full, &mut got_tokens).await?;
                            }
                        }
                    }
                    Ok(None) => {
                        let left = reader.leftover();
                        if let Some(raw) = data_payload(amadeus_core::jscompat::js_trim(&left)) {
                            emit_gemini_line(raw, sink, &mut full, &mut got_tokens).await?;
                        }
                        break;
                    }
                    Err(Thrown(msg)) => {
                        if got_tokens {
                            sink.emit(StreamEvent::error(format!(
                                "Vertex AI Stream Interrupted ({region}): {msg}"
                            )))
                            .await?;
                            return Ok(());
                        }
                        errors.push(format!("{region}: {msg}"));
                        st.jitter.sleep(delay_int_ms(100, 301)).await;
                        continue 'regions;
                    }
                }
            }
            sink.emit(StreamEvent::complete(full)).await?;
            return Ok(());
        }
        sink.emit(StreamEvent::error(format!(
            "Vertex AI Stream: All regions failed ({}). Errors: {}",
            regions.join(", "),
            errors.join("; ")
        )))
        .await?;
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_url_golden() {
        assert_eq!(
            build_vertex_url("p", "global", "gemini-2.5-flash", VertexMethod::Stream),
            "https://aiplatform.googleapis.com/v1/projects/p/locations/global/publishers/google/models/gemini-2.5-flash:streamGenerateContent?alt=sse"
        );
        assert_eq!(
            build_vertex_url("p", "asia-northeast1", "m", VertexMethod::Generate),
            "https://asia-northeast1-aiplatform.googleapis.com/v1/projects/p/locations/asia-northeast1/publishers/google/models/m:generateContent"
        );
    }

    #[test]
    fn helper_regions_golden() {
        let nine = [
            "global",
            "us-west1",
            "us-west4",
            "us-east1",
            "us-east4",
            "us-east5",
            "us-south1",
            "asia-northeast1",
            "northamerica-northeast1",
        ];
        assert_eq!(target_regions(""), nine);
        assert_eq!(target_regions("us-central1"), nine);
        assert_eq!(target_regions("global"), nine);
        assert_eq!(
            target_regions("asia-northeast1"),
            [
                "asia-northeast1",
                "global",
                "us-west1",
                "us-west4",
                "us-east1",
                "us-east4",
                "us-east5",
                "us-south1",
                "northamerica-northeast1"
            ]
        );
        let europe = target_regions("europe-west4");
        assert_eq!(europe.len(), 10);
        assert_eq!(europe[0], "europe-west4");
        assert_eq!(europe[1], "global");
        assert_eq!(
            target_regions(" us-west4 "),
            [
                "us-west4",
                "global",
                "us-west1",
                "us-east1",
                "us-east4",
                "us-east5",
                "us-south1",
                "asia-northeast1",
                "northamerica-northeast1"
            ]
        );
    }

    #[test]
    fn proxy_regions_golden() {
        assert_eq!(proxy_target_regions("us-central1"), VERTEX_PROXY_REGIONS);
        assert_eq!(
            proxy_target_regions("asia-east1"),
            [
                "asia-east1",
                "us-central1",
                "us-west1",
                "us-east4",
                "asia-northeast1",
                "northamerica-northeast1"
            ]
        );
    }

    #[test]
    fn retryable_statuses() {
        assert!(is_retryable_status(429));
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(503));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(404));
    }
}
