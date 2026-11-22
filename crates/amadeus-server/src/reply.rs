//! Mapping of backend results to HTTP responses.

use std::convert::Infallible;

use amadeus_backend::aux::js_json;
use amadeus_core::protocol::{ApiReply, StreamEvent};
use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONNECTION, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode, Version};
use axum::response::Response;
use bytes::Bytes;
use futures_util::stream::{BoxStream, StreamExt};

/// `Content-Type` of every JSON reply (`NextResponse.json`, no charset).
pub const JSON_CONTENT_TYPE: &str = "application/json";
/// `Content-Type` of the SSE responses.
pub const SSE_CONTENT_TYPE: &str = "text/event-stream; charset=utf-8";

fn status_code(status: u16) -> StatusCode {
    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

/// A header value from a latin1 string (how undici exposed upstream header bytes).
fn latin1_header_value(value: &str) -> Option<HeaderValue> {
    let bytes: Option<Vec<u8>> = value
        .chars()
        .map(|c| u8::try_from(u32::from(c)).ok())
        .collect();
    HeaderValue::from_bytes(&bytes?).ok()
}

/// Converts an [`ApiReply`] to the response V1 sent:
/// * `Json` → `Content-Type: application/json`, compact `JSON.stringify`-identical body;
/// * `Binary` → the given content type (verbatim) plus `Cache-Control: no-store` (overridden by
///   the cache policy layer, as in V1);
/// * `Empty` → status only, no body, no content type.
pub fn api_reply(reply: ApiReply) -> Response {
    match reply {
        ApiReply::Json { status, body } => {
            let mut res = Response::new(Body::from(js_json::stringify(&body)));
            *res.status_mut() = status_code(status);
            res.headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static(JSON_CONTENT_TYPE));
            res
        }
        ApiReply::Binary {
            status,
            content_type,
            data,
        } => {
            let mut res = Response::new(Body::from(data));
            *res.status_mut() = status_code(status);
            if let Some(value) = latin1_header_value(&content_type) {
                res.headers_mut().insert(CONTENT_TYPE, value);
            }
            res.headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
            res
        }
        ApiReply::Empty { status } => {
            let mut res = Response::new(Body::empty());
            *res.status_mut() = status_code(status);
            res
        }
    }
}

/// HTTP 200 SSE response: one `data: {json}\n\n` frame per event, each sent as its own body
/// chunk (hyper flushes it immediately). Headers are V1's effective ones:
/// `Content-Type: text/event-stream; charset=utf-8` and, on HTTP/1.1, `Connection: keep-alive`
/// (the no-store set is added by the cache layer; compression skips `text/event-stream`).
pub fn sse_response(events: BoxStream<'static, StreamEvent>, version: Version) -> Response {
    let body = Body::from_stream(
        events.map(|event| Ok::<Bytes, Infallible>(Bytes::from(event.to_sse_frame()))),
    );
    let mut res = Response::new(body);
    res.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(SSE_CONTENT_TYPE));
    if version == Version::HTTP_11 {
        res.headers_mut()
            .insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn body_text(res: Response) -> String {
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap_or_default();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[tokio::test]
    async fn json_reply_uses_js_serialization() {
        let res = api_reply(ApiReply::json(
            422,
            json!({"embedding": [1.0, 0.5], "10": "a"}),
        ));
        assert_eq!(res.status(), 422);
        assert_eq!(
            res.headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("application/json")
        );
        assert_eq!(body_text(res).await, r#"{"10":"a","embedding":[1,0.5]}"#);
    }

    #[tokio::test]
    async fn binary_and_empty_replies() {
        let res = api_reply(ApiReply::Binary {
            status: 200,
            content_type: "audio/x-wav".into(),
            data: b"RIFF".to_vec(),
        });
        assert_eq!(
            res.headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some("audio/x-wav")
        );
        assert_eq!(body_text(res).await, "RIFF");
        let res = api_reply(ApiReply::Binary {
            status: 200,
            content_type: "x/\u{e9}".into(),
            data: vec![],
        });
        assert_eq!(
            res.headers().get(CONTENT_TYPE).map(HeaderValue::as_bytes),
            Some(&b"x/\xe9"[..])
        );
        let res = api_reply(ApiReply::Empty { status: 500 });
        assert_eq!(res.status(), 500);
        assert!(res.headers().get(CONTENT_TYPE).is_none());
        assert_eq!(body_text(res).await, "");
        assert_eq!(api_reply(ApiReply::Empty { status: 1000 }).status(), 500);
    }

    #[tokio::test]
    async fn sse_frames() {
        let events = futures_util::stream::iter(vec![
            StreamEvent::token("あ"),
            StreamEvent::complete("a\"\\\nあ/<>"),
        ])
        .boxed();
        let res = sse_response(events, Version::HTTP_11);
        assert_eq!(
            res.headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok()),
            Some(SSE_CONTENT_TYPE)
        );
        assert_eq!(
            res.headers().get(CONNECTION).and_then(|v| v.to_str().ok()),
            Some("keep-alive")
        );
        assert_eq!(
            body_text(res).await,
            "data: {\"type\":\"token\",\"token\":\"あ\"}\n\ndata: {\"type\":\"complete\",\"fullResponse\":\"a\\\"\\\\\\nあ/<>\"}\n\n"
        );
        let res = sse_response(futures_util::stream::empty().boxed(), Version::HTTP_2);
        assert!(res.headers().get(CONNECTION).is_none());
    }
}
