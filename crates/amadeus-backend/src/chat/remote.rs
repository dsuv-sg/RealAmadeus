//! Desktop relay of FreeAPI (provider 8) and its quota to the hosted web server
//! (`{remote_api_base}/app/api/chat[/stream|/quota]`, same HTTP contract). FreeAPI needs the
//! server's credentials and its global counter, so the desktop never calls Google for it.
//!
//! JSON answers and SSE events of the hosted server are passed through unchanged. When it cannot
//! be reached (network error, unreadable or non-JSON answer, a 200 that is not an event stream)
//! the user gets the reply V1 gave for FreeAPI communication errors (`FREE_FALLBACK_COMM_ERROR`).

use amadeus_core::jscompat::js_truthy;
use amadeus_core::protocol::{paths, ApiReply, StreamEvent};
use serde_json::{json, Value};

use super::net::{self, content_json};
use super::payload::{non_nullish, parse_json_body, prop, value_text};
use super::sink::{fixed_stream, spawn_stream, EventSink, StreamErr};
use super::sse::{data_payload, LineReader};
use super::texts::FREE_FALLBACK_COMM_ERROR;
use crate::{Backend, ChatStreamStart};

/// `{base}/{path}` with at most one slash in between.
pub(crate) fn remote_url(base: &str, path: &str) -> String {
    format!("{}/{}", base.trim_end_matches('/'), path)
}

/// Status and JSON body of a relayed response. `None` when the body cannot be read or is not
/// JSON: the hosted server always answers these routes with JSON, so anything else (a CDN or
/// proxy error page while it is down, a captive portal) means it was not reached.
async fn relayed_json(resp: reqwest::Response) -> Option<ApiReply> {
    let status = resp.status().as_u16();
    let bytes = resp.bytes().await.ok()?;
    parse_json_body(&bytes).map(|body| ApiReply::json(status, body))
}

/// `Content-Type: text/event-stream…` (what the hosted `/chat/stream` sends with a 200).
fn is_event_stream(resp: &reqwest::Response) -> bool {
    resp.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| {
            ct.trim_start()
                .to_ascii_lowercase()
                .starts_with("text/event-stream")
        })
}

/// What V1 answered for provider 8 when the upstream could not be reached.
fn comm_error_reply() -> ApiReply {
    ApiReply::json(200, json!({ "response": FREE_FALLBACK_COMM_ERROR }))
}

fn comm_error_events() -> Vec<StreamEvent> {
    vec![
        StreamEvent::token(FREE_FALLBACK_COMM_ERROR),
        StreamEvent::complete(FREE_FALLBACK_COMM_ERROR),
    ]
}

/// `POST {remote}/app/api/chat` with the original body. JSON answers (any status) are passed
/// through; an unreachable server gives the FreeAPI communication-error reply.
pub(crate) async fn forward_chat(backend: &Backend, base: &str, body: &[u8]) -> ApiReply {
    let url = remote_url(base, paths::API_CHAT);
    match net::post(backend.http(), &url, &[content_json()], body.to_vec()).await {
        Ok(resp) => relayed_json(resp).await.unwrap_or_else(comm_error_reply),
        Err(_) => comm_error_reply(),
    }
}

/// `GET {remote}/app/api/chat/quota`; `502 {"error":"fetch failed"}` when unreachable (the UI
/// treats any non-OK quota answer as "unknown").
pub(crate) async fn forward_quota(backend: &Backend, base: &str) -> ApiReply {
    let url = remote_url(base, paths::API_CHAT_QUOTA);
    let failed = || ApiReply::error(502, super::texts::FETCH_FAILED);
    match net::get(backend.http(), &url).await {
        Ok(resp) => relayed_json(resp).await.unwrap_or_else(failed),
        Err(_) => failed(),
    }
}

/// `POST {remote}/app/api/chat/stream`: JSON error answers are returned as replies, SSE events
/// are passed through, an unreachable server becomes the FreeAPI fallback (token + complete).
pub(crate) async fn forward_stream(backend: &Backend, base: &str, body: &[u8]) -> ChatStreamStart {
    let url = remote_url(base, paths::API_CHAT_STREAM);
    let resp = match net::post(backend.http(), &url, &[content_json()], body.to_vec()).await {
        Ok(resp) => resp,
        Err(_) => return ChatStreamStart::Stream(fixed_stream(comm_error_events())),
    };
    if !resp.status().is_success() {
        return match relayed_json(resp).await {
            Some(reply) => ChatStreamStart::Reply(reply),
            None => ChatStreamStart::Stream(fixed_stream(comm_error_events())),
        };
    }
    if !is_event_stream(&resp) {
        return ChatStreamStart::Stream(fixed_stream(comm_error_events()));
    }
    ChatStreamStart::Stream(spawn_stream(None, move |mut sink| async move {
        let _ = relay_events(resp, &mut sink).await;
    }))
}

/// Re-emit the remote events with the V1 client's parsing rules (`ai-service.ts`): unparseable
/// payloads (and `null`) are raw tokens, unknown types are ignored, the stream ends at the
/// first `complete`/`error`.
async fn relay_events(mut resp: reqwest::Response, sink: &mut EventSink) -> Result<(), StreamErr> {
    let mut reader = LineReader::new();
    let mut full = String::new();
    loop {
        let chunk = match net::next_chunk(&mut resp).await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => return Ok(()),
            Err(_) => {
                for event in comm_error_events() {
                    sink.emit(event).await?;
                }
                return Ok(());
            }
        };
        for line in reader.push(&chunk) {
            let Some(raw) = data_payload(&line).filter(|raw| !raw.is_empty()) else {
                continue;
            };
            let event = match serde_json::from_str::<Value>(raw) {
                Ok(v) if !v.is_null() => v,
                _ => {
                    full.push_str(raw);
                    sink.emit(StreamEvent::token(raw)).await?;
                    continue;
                }
            };
            match prop(Some(&event), "type").and_then(Value::as_str) {
                Some("token") => {
                    if let Some(token) = prop(Some(&event), "token").filter(|t| js_truthy(t)) {
                        let token = value_text(token);
                        full.push_str(&token);
                        sink.emit(StreamEvent::token(token)).await?;
                    }
                }
                Some("complete") => {
                    let text = non_nullish(prop(Some(&event), "fullResponse"))
                        .map_or_else(|| full.clone(), value_text);
                    sink.emit(StreamEvent::complete(text)).await?;
                    return Ok(());
                }
                Some("error") => {
                    let error = non_nullish(prop(Some(&event), "error"))
                        .map_or_else(|| "Streaming error".to_owned(), value_text);
                    sink.emit(StreamEvent::error(error)).await?;
                    return Ok(());
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_urls() {
        assert_eq!(
            remote_url("https://dsuv.elvelt.com/amadeus", paths::API_CHAT),
            "https://dsuv.elvelt.com/amadeus/app/api/chat"
        );
        assert_eq!(
            remote_url("https://h/amadeus/", paths::API_CHAT_QUOTA),
            "https://h/amadeus/app/api/chat/quota"
        );
    }
}
