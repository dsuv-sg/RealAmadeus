//! `POST {base}/app/api/tts/synthesize` and `/tts/health` (V1 `app/amadeus/app/api/tts/*`).

use amadeus_core::protocol::ApiReply;
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};

use super::health::{self, HealthBodyMode};
use super::js::{js_member, non_nullish, opt_trimmed, parse_json_lenient, utf8_decode_bytes};
use super::js_json::{stringify, stringify_str};
use super::upstream::{
    fetch_error, forbidden, header_get, json_with_upstream_status, prepare, type_error_reply,
    FetchFailure, Prepared,
};
use super::{AuxContext, DEFAULT_TTS_SPEAKER};
use crate::Backend;

/// Content type used when the upstream sends none (`?? "audio/wav"`).
pub const DEFAULT_AUDIO_CONTENT_TYPE: &str = "audio/wav";

/// `POST /tts/synthesize`.
pub async fn synthesize(backend: &Backend, body: &[u8]) -> ApiReply {
    synthesize_with(AuxContext::from_backend(backend), body).await
}

/// `POST /tts/health` (body read before the status check, spread on success).
pub async fn health(backend: &Backend, body: &[u8]) -> ApiReply {
    health_with(AuxContext::from_backend(backend), body).await
}

/// [`health()`] with an explicit context.
pub async fn health_with(ctx: AuxContext<'_>, body: &[u8]) -> ApiReply {
    health::route(ctx, body, HealthBodyMode::ReadBeforeStatus).await
}

/// [`synthesize`] with an explicit context.
///
/// 1. `serverUrl`, `text`, `speaker` are read (in that order) with `?.trim()` semantics; a
///    non-string gives the empty 500.
/// 2. Missing `serverUrl` → 400, missing `text` → 400; empty `speaker` → `christina-jp`.
/// 3. `POST {serverUrl}/synthesize` with `{"text","speaker","use_cache"}` (`useCache ?? true`,
///    passed through untouched).
/// 4. Non-2xx → upstream status + `Synthesis failed (<status>): <body text>`; 2xx → the bytes
///    with the upstream `Content-Type` verbatim (default `audio/wav`).
pub async fn synthesize_with(ctx: AuxContext<'_>, body: &[u8]) -> ApiReply {
    let payload = parse_json_lenient(body);
    let (Ok(server_url), Ok(text), Ok(speaker)) = (
        opt_trimmed(&payload, "serverUrl"),
        opt_trimmed(&payload, "text"),
        opt_trimmed(&payload, "speaker"),
    ) else {
        return type_error_reply();
    };
    let speaker = speaker
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_TTS_SPEAKER.to_owned());
    let Some(server_url) = server_url.filter(|s| !s.is_empty()) else {
        return ApiReply::error(400, "serverUrl is required");
    };
    let Some(text) = text.filter(|s| !s.is_empty()) else {
        return ApiReply::error(400, "text is required");
    };
    let use_cache = non_nullish(js_member(&payload, "useCache"))
        .cloned()
        .unwrap_or(Value::Bool(true));
    let upstream_body = format!(
        "{{\"text\":{},\"speaker\":{},\"use_cache\":{}}}",
        stringify_str(&text),
        stringify_str(&speaker),
        stringify(&use_cache)
    );

    let url = format!("{server_url}/synthesize");
    let (client, parsed) = match prepare(ctx, &url) {
        Prepared::Ready(client, parsed) => (client, parsed),
        Prepared::Failed(failure) => return fetch_error(&failure),
        Prepared::Forbidden => return forbidden(),
    };
    let response = match client
        .post(parsed)
        .header(CONTENT_TYPE, "application/json")
        .body(upstream_body)
        .send()
        .await
    {
        Ok(response) => response,
        Err(_) => return fetch_error(&FetchFailure::Failed),
    };
    let status = response.status();
    if !status.is_success() {
        let error_body = match response.bytes().await {
            Ok(bytes) => utf8_decode_bytes(&bytes),
            Err(_) => return fetch_error(&FetchFailure::Terminated),
        };
        return json_with_upstream_status(
            status.as_u16(),
            json!({ "error": format!("Synthesis failed ({}): {}", status.as_u16(), error_body) }),
        );
    }
    let content_type = header_get(response.headers(), CONTENT_TYPE.as_str())
        .unwrap_or_else(|| DEFAULT_AUDIO_CONTENT_TYPE.to_owned());
    match response.bytes().await {
        Ok(bytes) => ApiReply::Binary {
            status: 200,
            content_type,
            data: bytes.to_vec(),
        },
        Err(_) => fetch_error(&FetchFailure::Terminated),
    }
}
