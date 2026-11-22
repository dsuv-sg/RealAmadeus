//! `POST {base}/app/api/chat/stream`: V1's effective SSE headers, one `data: …\n\n` frame per
//! event, streamed without buffering or compression.
//!
//! The provider logic belongs to the chat module; these tests only use deterministic paths that
//! need no external network (the FreeAPI fallback when the global quota is exhausted, and the
//! Ollama provider pointed at the local mock upstream) and assert the transport contract.

use std::time::{Duration, Instant};

use amadeus_backend::chat::texts::FREE_FALLBACK_RATE_LIMITED;
use amadeus_backend::chat::{CHAT_LOG_FILE, MAX_GLOBAL_REQUESTS, REQUEST_COUNT_FILE};
use amadeus_core::protocol::StreamEvent;
use axum::body::Body;
use axum::http::{Method, StatusCode, Version};
use futures_util::StreamExt;
use serde_json::json;

use crate::common::*;

const STREAM: &str = "/amadeus/app/api/chat/stream";
const SSE_TYPE: &str = "text/event-stream; charset=utf-8";

fn exhausted_quota_app() -> TestApp {
    let count = MAX_GLOBAL_REQUESTS.to_string();
    app_full(&[(REQUEST_COUNT_FILE, count.as_bytes())], |_| {})
}

fn free_api_payload() -> String {
    json!({"messages": [{"role": "user", "content": "hi"}], "provider": 8}).to_string()
}

fn ollama_payload(host: &str) -> String {
    json!({
        "messages": [{"role": "system", "content": "sys"}, {"role": "user", "content": "hello?"}],
        "provider": 5,
        "model": "llama3",
        "ollamaHost": host,
    })
    .to_string()
}

#[tokio::test]
async fn sse_headers_and_frames() {
    let app = exhausted_quota_app();
    let req = request(
        Method::POST,
        STREAM,
        &[
            ("content-type", "text/plain"),
            ("accept-encoding", "gzip, br"),
        ],
        Body::from(free_api_payload()),
    );
    let res = app.send_raw(req).await;
    assert_eq!(res.status(), StatusCode::OK);
    let header = |name: &str| {
        res.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    assert_eq!(header("content-type").as_deref(), Some(SSE_TYPE));
    assert_eq!(header("connection").as_deref(), Some("keep-alive"));
    // V1's effective headers: the global no-store rule replaced `no-cache, no-transform`.
    for (name, value) in NO_STORE {
        assert_eq!(header(name).as_deref(), Some(value), "{name}");
    }
    assert!(
        header("content-encoding").is_none(),
        "SSE must never be compressed"
    );
    assert!(header("content-length").is_none(), "SSE is streamed");
    let frames = frames(res).await;
    let expected = [
        StreamEvent::token(FREE_FALLBACK_RATE_LIMITED),
        StreamEvent::complete(FREE_FALLBACK_RATE_LIMITED),
    ];
    assert_eq!(
        frames.len(),
        expected.len(),
        "one body frame per event: {frames:?}"
    );
    for (frame, event) in frames.iter().zip(&expected) {
        assert_eq!(String::from_utf8_lossy(frame), event.to_sse_frame());
    }
}

#[tokio::test]
async fn sse_over_http2_has_no_connection_header() {
    let app = exhausted_quota_app();
    let mut req = request(Method::POST, STREAM, &[], Body::from(free_api_payload()));
    *req.version_mut() = Version::HTTP_2;
    let res = app.send_raw(req).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some(SSE_TYPE)
    );
    assert!(
        res.headers().get("connection").is_none(),
        "connection-specific header on HTTP/2"
    );
}

#[tokio::test]
async fn upstream_stream_is_relayed_event_by_event() {
    let mock = start_mock().await;
    let app = app();
    let req = request(
        Method::POST,
        STREAM,
        &[
            ("content-type", "application/json"),
            ("x-forwarded-for", " 203.0.113.7 , 10.0.0.1"),
        ],
        Body::from(ollama_payload(&mock.url("ollama"))),
    );
    let res = app.send_raw(req).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok()),
        Some(SSE_TYPE)
    );
    let frames: Vec<String> = frames(res)
        .await
        .iter()
        .map(|f| String::from_utf8_lossy(f).into_owned())
        .collect();
    assert_eq!(
        frames,
        [
            StreamEvent::token("Hel"),
            StreamEvent::token("lo"),
            StreamEvent::complete("Hello")
        ]
        .iter()
        .map(StreamEvent::to_sse_frame)
        .collect::<Vec<_>>()
    );
    assert_eq!(mock.last("/ollama/api/chat").method, "POST");

    // The chat log records the client IP derived like V1 `getClientIp` (first X-Forwarded-For
    // entry, trimmed). The log line is written before the `complete` frame is forwarded.
    let log = std::fs::read_to_string(app.data.path().join(CHAT_LOG_FILE)).unwrap_or_default();
    let entry: serde_json::Value = log
        .lines()
        .last()
        .and_then(|l| serde_json::from_str(l).ok())
        .unwrap_or(serde_json::Value::Null);
    assert_eq!(
        entry.get("ip").and_then(|v| v.as_str()),
        Some("203.0.113.7"),
        "chat log: {log}"
    );
}

#[tokio::test]
async fn sse_frames_are_not_buffered() {
    let mock = start_mock().await;
    let app = app();
    let req = request(
        Method::POST,
        STREAM,
        &[("accept-encoding", "gzip")],
        Body::from(ollama_payload(&mock.url("ollamaslow"))),
    );
    let started = Instant::now();
    let res = app.send_raw(req).await;
    let mut body = res.into_body().into_data_stream();
    let first = body.next().await.and_then(Result::ok).unwrap_or_default();
    let first_at = started.elapsed();
    assert_eq!(
        String::from_utf8_lossy(&first),
        StreamEvent::token("Hel").to_sse_frame()
    );
    assert!(
        first_at < SLOW_TAIL / 2,
        "first frame held back for {first_at:?}"
    );
    let mut rest = Vec::new();
    while let Some(frame) = body.next().await {
        rest.push(frame.unwrap_or_default());
    }
    assert!(started.elapsed() >= SLOW_TAIL - Duration::from_millis(50));
    assert_eq!(rest.len(), 2, "{rest:?}");
    assert_eq!(
        String::from_utf8_lossy(&rest[1]),
        StreamEvent::complete("Hello").to_sse_frame()
    );
}

#[tokio::test]
async fn pre_stream_errors_are_json() {
    let app = app();
    app.post_json(STREAM, "{}")
        .await
        .assert_json(400, r#"{"error":"Invalid request payload"}"#);
    // Not compressed / not SSE: plain JSON with the no-store set.
    let res = app
        .send(request(
            Method::POST,
            STREAM,
            &[("accept-encoding", "gzip")],
            Body::from("[]"),
        ))
        .await;
    res.assert_json(400, r#"{"error":"Invalid request payload"}"#);
    res.assert_headers(&NO_STORE);
}
