//! `POST {base}/app/api/rag/embed` and `/rag/health` (V1 `app/amadeus/app/api/rag/*`).

use amadeus_core::protocol::ApiReply;
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};

use super::health::{self, HealthBodyMode};
use super::js::{js_member, non_nullish, opt_trimmed, parse_json_lenient};
use super::js_json::stringify_str;
use super::upstream::{
    fetch_error, forbidden, json_with_upstream_status, prepare, read_json_or_null,
    type_error_reply, FetchFailure, Prepared,
};
use super::AuxContext;
use crate::Backend;

/// `POST /rag/embed`.
pub async fn embed(backend: &Backend, body: &[u8]) -> ApiReply {
    embed_with(AuxContext::from_backend(backend), body).await
}

/// `POST /rag/health` (status only; success is exactly `{"available":true}`).
pub async fn health(backend: &Backend, body: &[u8]) -> ApiReply {
    health_with(AuxContext::from_backend(backend), body).await
}

/// [`health()`] with an explicit context.
pub async fn health_with(ctx: AuxContext<'_>, body: &[u8]) -> ApiReply {
    health::route(ctx, body, HealthBodyMode::Ignore).await
}

/// [`embed`] with an explicit context.
///
/// `serverUrl` and `text` are trimmed (`?.trim()`), `POST {serverUrl}/embed` gets
/// `{"text":<trimmed>}`; the upstream JSON is passed through (`body ?? {}`) on 2xx, otherwise
/// the upstream status with `body?.error ?? "Embedding failed (<status>)"`.
pub async fn embed_with(ctx: AuxContext<'_>, body: &[u8]) -> ApiReply {
    let payload = parse_json_lenient(body);
    let (Ok(server_url), Ok(text)) = (
        opt_trimmed(&payload, "serverUrl"),
        opt_trimmed(&payload, "text"),
    ) else {
        return type_error_reply();
    };
    let Some(server_url) = server_url.filter(|s| !s.is_empty()) else {
        return ApiReply::error(400, "serverUrl is required");
    };
    let Some(text) = text.filter(|s| !s.is_empty()) else {
        return ApiReply::error(400, "text is required");
    };

    let url = format!("{server_url}/embed");
    let (client, parsed) = match prepare(ctx, &url) {
        Prepared::Ready(client, parsed) => (client, parsed),
        Prepared::Failed(failure) => return fetch_error(&failure),
        Prepared::Forbidden => return forbidden(),
    };
    let upstream_body = format!("{{\"text\":{}}}", stringify_str(&text));
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
    let status = response.status().as_u16();
    let body = read_json_or_null(response).await;
    if !(200..300).contains(&status) {
        let error = non_nullish(js_member(&body, "error"))
            .cloned()
            .unwrap_or_else(|| Value::String(format!("Embedding failed ({status})")));
        return json_with_upstream_status(status, json!({ "error": error }));
    }
    ApiReply::json(200, if body.is_null() { json!({}) } else { body })
}
