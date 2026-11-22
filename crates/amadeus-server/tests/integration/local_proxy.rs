//! `amadeus-local-proxy` (V1 `scripts/amadeus-local-proxy/server.mjs`, spec §6): the HTTP
//! adapter driven with `oneshot`, Vertex calls answered by the mock upstream. The token source
//! is fixed, so gcloud is never spawned.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use amadeus_backend::local_proxy::{LocalProxy, LocalProxyConfig, TokenSource, BODY_LIMIT_BYTES};
use amadeus_server::local_proxy_http;
use axum::body::Body;
use axum::http::Method;
use axum::Router;
use serde_json::json;
use tower::ServiceExt;

use crate::common::*;

const CORS: [(&str, &str); 3] = [
    ("access-control-allow-origin", "*"),
    ("access-control-allow-methods", "GET,POST,OPTIONS"),
    (
        "access-control-allow-headers",
        "Content-Type, Authorization",
    ),
];
const JSON_UTF8: &str = "application/json; charset=utf-8";
const TOKEN: &str = "ya29.test-token";

fn proxy_with(vertex_base: &str, env_project: &str) -> Router {
    let config = LocalProxyConfig {
        host: "127.0.0.1".into(),
        port: 58103,
        vertex_project: env_project.into(),
        vertex_location: String::new(),
    };
    let base = vertex_base.to_owned();
    let http = amadeus_backend::http::build_client().expect("client");
    let proxy = LocalProxy::new(config, http)
        .with_token_source(TokenSource::Fixed(TOKEN.into()))
        .with_vertex_url(Arc::new(
            move |project: &str, region: &str, model: &str, method: &str| {
                format!("{base}/vertex/{region}/{project}/{model}:{method}")
            },
        ));
    local_proxy_http::router(Arc::new(proxy))
}

async fn call(router: &Router, method: Method, uri: &str, body: impl Into<Body>) -> TestResponse {
    let res = router
        .clone()
        .oneshot(request(method, uri, &[], body.into()))
        .await
        .expect("infallible");
    buffer(res).await
}

async fn chat(router: &Router, payload: serde_json::Value) -> TestResponse {
    call(router, Method::POST, "/chat", payload.to_string()).await
}

fn assert_proxy_json(res: &TestResponse, status: u16, body: &str) {
    assert_eq!(res.status.as_u16(), status, "body {}", res.text());
    assert_eq!(res.header("content-type"), Some(JSON_UTF8));
    res.assert_headers(&CORS);
    assert_eq!(res.text(), body);
}

fn hi() -> serde_json::Value {
    json!([{"role": "user", "content": "hi"}])
}

#[tokio::test]
async fn dispatch_preflight_health_and_404() {
    let router = proxy_with(REFUSED, "");
    let res = call(&router, Method::OPTIONS, "/anything?x=1", Body::empty()).await;
    assert_eq!(res.status.as_u16(), 204);
    res.assert_headers(&CORS);
    assert!(res.header("content-type").is_none() && res.body.is_empty());

    let health = r#"{"available":true,"mode":"gcloud-vertex-proxy","bindHost":"127.0.0.1","bindPort":58103,"project":"","location":"us-central1"}"#;
    for uri in ["/health", "/health?x=1"] {
        assert_proxy_json(
            &call(&router, Method::GET, uri, Body::empty()).await,
            200,
            health,
        );
    }
    for (method, uri) in [
        (Method::GET, "/health/"),
        (Method::POST, "/health"),
        (Method::GET, "/chat"),
        (Method::GET, "/"),
    ] {
        let res = call(&router, method.clone(), uri, Body::empty()).await;
        assert_proxy_json(&res, 404, r#"{"error":"Not found"}"#);
    }
    // V1 routed methods by hand: HEAD /health is a 404 too (Node sends no body for HEAD).
    let res = call(&router, Method::HEAD, "/health", Body::empty()).await;
    assert_eq!(res.status.as_u16(), 404);
    assert_eq!(res.header("content-type"), Some(JSON_UTF8));
    res.assert_headers(&CORS);
    assert!(res.body.is_empty());
}

#[tokio::test]
async fn chat_validation_and_body_errors() {
    let router = proxy_with(REFUSED, "");
    for raw in ["", "null", "[]", "\"hi\"", r#"{"messages":[]}"#] {
        let res = call(&router, Method::POST, "/chat", raw).await;
        assert_proxy_json(&res, 400, r#"{"error":"messages is required"}"#);
    }
    // V1 quirk: invalid JSON and oversized bodies are 500, not 400.
    assert_proxy_json(
        &call(&router, Method::POST, "/chat", "{bad").await,
        500,
        r#"{"error":"Invalid JSON"}"#,
    );
    let too_long = format!("\"{}\"", "a".repeat(2_000_000));
    assert_proxy_json(
        &call(&router, Method::POST, "/chat", too_long).await,
        500,
        r#"{"error":"Payload too large"}"#,
    );
    let huge = vec![b' '; BODY_LIMIT_BYTES + 1];
    assert_proxy_json(
        &call(&router, Method::POST, "/chat", huge).await,
        500,
        r#"{"error":"Payload too large"}"#,
    );
    let res = chat(&router, json!({"messages": hi()})).await;
    assert_proxy_json(
        &res,
        400,
        r#"{"error":"vertexProject is required (or set VERTEX_PROJECT env)."}"#,
    );
    // A whitespace-only project is truthy: no env fallback, then empty after trim.
    let res = chat(
        &proxy_with(REFUSED, "env-project"),
        json!({"messages": hi(), "vertexProject": "  "}),
    )
    .await;
    assert_eq!(res.status.as_u16(), 400);
}

#[tokio::test]
async fn region_failover_on_429_and_5xx() {
    let mock = start_mock().await;
    let router = proxy_with(&mock.base, "");
    let res = chat(
        &router,
        json!({"messages": hi(), "vertexProject": "failover", "apiKey": "ignored"}),
    )
    .await;
    assert_proxy_json(&res, 200, r#"{"response":"vertex says hi"}"#);
    let requests = mock.requests();
    let paths: Vec<&str> = requests.iter().map(|r| r.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/vertex/us-central1/failover/gemini-2.5-flash:generateContent",
            "/vertex/us-west1/failover/gemini-2.5-flash:generateContent",
            "/vertex/us-east4/failover/gemini-2.5-flash:generateContent",
        ]
    );
    for r in &requests {
        assert_eq!(r.method, "POST");
        assert_eq!(r.header("authorization"), Some("Bearer ya29.test-token"));
        assert_eq!(r.header("content-type"), Some("application/json"));
        assert_eq!(
            r.text(),
            r#"{"contents":[{"role":"user","parts":[{"text":"hi"}]}],"generationConfig":{"maxOutputTokens":8192}}"#
        );
    }
}

#[tokio::test]
async fn retryable_failures_exhaust_every_region() {
    let mock = start_mock().await;
    let res = chat(
        &proxy_with(&mock.base, ""),
        json!({"messages": hi(), "vertexProject": "allretry"}),
    )
    .await;
    assert_proxy_json(
        &res,
        500,
        r#"{"error":"Vertex retryable failures: us-central1:429, us-west1:503, us-east4:503, asia-northeast1:429, northamerica-northeast1:503"}"#,
    );
    assert_eq!(mock.requests().len(), 5);
}

#[tokio::test]
async fn other_statuses_fail_immediately() {
    let mock = start_mock().await;
    let router = proxy_with(&mock.base, "");
    let res = chat(&router, json!({"messages": hi(), "vertexProject": "notfound", "vertexLocation": " asia-northeast1 "})).await;
    assert_proxy_json(
        &res,
        500,
        r#"{"error":"Vertex Error (asia-northeast1) 404 Not Found: {\"error\":{\"code\":404,\"status\":\"NOT_FOUND\"}}"}"#,
    );
    assert_eq!(mock.requests().len(), 1);
    let res = chat(
        &router,
        json!({"messages": hi(), "vertexProject": "nullbody"}),
    )
    .await;
    assert_proxy_json(
        &res,
        500,
        r#"{"error":"Vertex Error (us-central1) 400 Bad Request: null"}"#,
    );
    // A network error propagates without failover (V1 parity).
    let res = chat(
        &proxy_with(REFUSED, ""),
        json!({"messages": hi(), "vertexProject": "p"}),
    )
    .await;
    assert_proxy_json(&res, 500, r#"{"error":"fetch failed"}"#);
}

#[tokio::test]
async fn payload_options_like_v1() {
    let mock = start_mock().await;
    // Env project fallback, `gpt*` model replaced, grounding, system instruction, `global` first.
    let router = proxy_with(&mock.base, "env-project");
    let res = chat(
        &router,
        json!({
            "messages": [{"role": "system", "content": "be kurisu"}, {"role": "user", "content": "hi"}, {"role": "assistant", "content": "yo"}],
            "vertexProject": "",
            "vertexLocation": "global",
            "model": "GPT-4o",
            "webSearch": 1,
        }),
    )
    .await;
    assert_proxy_json(&res, 200, r#"{"response":"vertex says hi"}"#);
    let req = mock.last(":generateContent");
    assert_eq!(
        req.path,
        "/vertex/global/env-project/gemini-2.5-flash:generateContent"
    );
    assert_eq!(
        req.text(),
        r#"{"contents":[{"role":"user","parts":[{"text":"hi"}]},{"role":"model","parts":[{"text":"yo"}]}],"generationConfig":{"maxOutputTokens":8192},"system_instruction":{"parts":[{"text":"be kurisu"}]},"tools":[{"googleSearch":{}}]}"#
    );
    // Explicit model (trimmed) and a response without text.
    let res = chat(
        &router,
        json!({"messages": hi(), "vertexProject": "garbage", "model": " gemini-2.0-pro "}),
    )
    .await;
    assert_proxy_json(&res, 200, r#"{"response":"[Parse Error]"}"#);
    assert_eq!(
        mock.last(":generateContent").path,
        "/vertex/us-central1/garbage/gemini-2.0-pro:generateContent"
    );
}

// ----- command line of the binary (PLAT-009) ------------------------------------------------

/// The real `amadeus-local-proxy` binary, bound (if it starts) to an ephemeral loopback port.
fn spawn_binary(args: &[&str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_amadeus-local-proxy"))
        .args(args)
        .env("LOCAL_PROXY_HOST", "127.0.0.1")
        .env("LOCAL_PROXY_PORT", "0")
        .env_remove("RUST_LOG")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn amadeus-local-proxy")
}

/// Exit status and stdout of a run that must end on its own within a few seconds.
fn run_to_exit(args: &[&str]) -> (ExitStatus, String) {
    let mut child = spawn_binary(args);
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("amadeus-local-proxy {args:?} did not exit (it started the proxy)");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut out = String::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout.read_to_string(&mut out).expect("stdout");
    }
    (status, out)
}

#[test]
fn help_and_version_exit_without_listening() {
    let version = format!("amadeus-local-proxy {}", env!("CARGO_PKG_VERSION"));
    for flag in ["--help", "-h"] {
        let (status, out) = run_to_exit(&["--port", "1", flag]);
        assert!(status.success(), "{flag}: {status}");
        assert!(out.starts_with(&format!("{version}\n\n")), "{flag}: {out}");
        for setting in [
            "LOCAL_PROXY_HOST",
            "LOCAL_PROXY_PORT",
            "VERTEX_PROJECT",
            "VERTEX_LOCATION",
        ] {
            assert!(out.contains(setting), "{flag}: {out}");
        }
        assert!(!out.contains("listening"), "{flag}: {out}");
    }
    for flag in ["--version", "-V"] {
        let (status, out) = run_to_exit(&[flag]);
        assert!(status.success(), "{flag}: {status}");
        assert_eq!(out, format!("{version}\n"));
    }
}

#[test]
fn other_arguments_are_ignored_like_v1() {
    let mut child = spawn_binary(&["--port", "9", "serve"]);
    let stdout = child.stdout.take().expect("stdout");
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = BufReader::new(stdout).read_line(&mut line);
        let _ = tx.send(line);
    });
    let line = rx.recv_timeout(Duration::from_secs(10));
    let still_running = child.try_wait().ok().flatten().is_none();
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(
        line.as_deref().ok(),
        Some("[local-proxy] listening on http://127.0.0.1:0\n")
    );
    assert!(still_running, "the proxy should keep serving");
}
