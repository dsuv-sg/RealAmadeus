//! Routing table, Next.js-compatible method handling and redirects, SPA fallback, static files
//! and the V1 cache header policy.

use amadeus_backend::chat::REQUEST_COUNT_FILE;
use amadeus_core::protocol::paths;
use amadeus_server::static_files::{SERVICE_WORKER_JS, SPA_ROUTES};
use axum::body::Body;
use axum::http::{Method, StatusCode};

use crate::common::*;

/// Every API route with the methods Next.js auto-implemented for it.
const POST_ROUTES: [&str; 9] = [
    paths::API_CHAT,
    paths::API_CHAT_STREAM,
    paths::API_CHAT_VERTEX_PROXY,
    paths::API_TTS_SYNTHESIZE,
    paths::API_TTS_HEALTH,
    paths::API_STT_TRANSCRIBE,
    paths::API_STT_HEALTH,
    paths::API_RAG_EMBED,
    paths::API_RAG_HEALTH,
];

fn api(rel: &str) -> String {
    format!("/amadeus/{rel}")
}

#[tokio::test]
async fn spa_routes_serve_index_with_200() {
    let app = app();
    for rel in SPA_ROUTES {
        let uri = format!("/amadeus{rel}");
        let res = app.get(&uri).await;
        assert_eq!(res.status, StatusCode::OK, "{uri}");
        assert_eq!(
            res.header("content-type"),
            Some("text/html; charset=utf-8"),
            "{uri}"
        );
        assert_eq!(res.text(), INDEX_HTML, "{uri}");
        res.assert_headers(&NO_STORE);
    }
    // The query string is irrelevant for page routes.
    let res = app.get("/amadeus/app?foo=bar").await;
    assert_eq!(
        (res.status, res.text().as_str()),
        (StatusCode::OK, INDEX_HTML)
    );
}

#[tokio::test]
async fn unknown_pages_serve_index_with_404() {
    let app = app();
    for uri in [
        "/amadeus/nope",
        "/amadeus/docs/api/unknown",
        "/amadeus/app/settings",
        "/amadeus/index.htm",
    ] {
        let res = app.get(uri).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(res.header("content-type"), Some("text/html; charset=utf-8"));
        assert_eq!(res.text(), INDEX_HTML);
        assert_eq!(res.header("cache-control"), Some(NOT_FOUND_CC));
        res.assert_headers(&NO_STORE[1..]);
    }
}

/// Methods without special meaning for pages (Next rendered the page for all of them).
fn other_methods() -> Vec<Method> {
    let mut methods = vec![Method::POST, Method::PUT, Method::DELETE, Method::PATCH];
    for name in ["PROPFIND", "PURGE", "QUERY", "TRACE"] {
        methods.push(Method::from_bytes(name.as_bytes()).expect("method"));
    }
    methods
}

#[tokio::test]
async fn page_methods_match_next() {
    let app = app();
    // Observed on V1: every method renders a page, except OPTIONS (empty 400).
    for rel in SPA_ROUTES {
        let uri = format!("/amadeus{rel}");
        for method in other_methods() {
            let res = app.method(method.clone(), &uri).await;
            assert_eq!(res.status, StatusCode::OK, "{method} {uri}");
            assert_eq!(res.header("content-type"), Some("text/html; charset=utf-8"));
            assert_eq!(res.text(), INDEX_HTML, "{method} {uri}");
            assert!(res.header("allow").is_none());
            res.assert_headers(&NO_STORE);
        }
        let res = app.method(Method::OPTIONS, &uri).await;
        res.assert_empty(400);
        assert!(res.header("allow").is_none());
        res.assert_headers(&NO_STORE);
    }
    // A request body does not matter either.
    let res = app
        .send(request(
            Method::POST,
            "/amadeus/app",
            &[("content-type", "application/json")],
            Body::from(r#"{"x":1}"#),
        ))
        .await;
    assert_eq!(
        (res.status, res.text().as_str()),
        (StatusCode::OK, INDEX_HTML)
    );
    // Unknown pages are the 404 page for every method.
    for uri in [
        "/amadeus/nope",
        "/amadeus/app/settings",
        "/amadeus/resources/x.png",
    ] {
        let mut methods = other_methods();
        methods.push(Method::OPTIONS);
        for method in methods {
            let res = app.method(method.clone(), uri).await;
            assert_eq!(res.status, StatusCode::NOT_FOUND, "{method} {uri}");
            assert_eq!(res.text(), INDEX_HTML, "{method} {uri}");
            assert_eq!(res.header("cache-control"), Some(NOT_FOUND_CC));
        }
    }
}

#[tokio::test]
async fn files_reject_other_methods() {
    let app = app();
    // V1 answered writes to static files with a 500 from Next's static handler; V2 sends 405.
    for uri in [
        format!("/amadeus/{HASHED_JS}"),
        "/amadeus/resources/images/Amadeus_BG.png".to_owned(),
        "/amadeus/static/amadeus_logo.png".to_owned(),
        "/amadeus/manifest.webmanifest".to_owned(),
    ] {
        for method in [Method::POST, Method::PUT, Method::OPTIONS, Method::DELETE] {
            let res = app.method(method.clone(), &uri).await;
            assert_eq!(res.status, StatusCode::METHOD_NOT_ALLOWED, "{method} {uri}");
            assert_eq!(res.header("allow"), Some("GET, HEAD"));
            assert!(res.body.is_empty());
        }
    }
    // The probe resolves paths like GET does (percent-decoding, no directories, no traversal).
    let res = app
        .method(
            Method::POST,
            "/amadeus/resources/fonts/Eurostile%20Condensed%20Bold.otf",
        )
        .await;
    assert_eq!(res.status, StatusCode::METHOD_NOT_ALLOWED);
    for uri in [
        "/amadeus/resources/images",
        "/amadeus/resources/../../secret.txt",
        "/amadeus/%2e%2e/secret.txt",
    ] {
        let res = app.method(Method::POST, uri).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(res.text(), INDEX_HTML, "{uri}");
    }
}

#[tokio::test]
async fn head_works_everywhere() {
    let app = app();
    for (uri, content_type) in [
        ("/amadeus/app", "text/html; charset=utf-8"),
        ("/amadeus", "text/html; charset=utf-8"),
        ("/amadeus/healthz", "text/plain; charset=utf-8"),
        ("/amadeus/app/api/chat/quota", "application/json"),
        ("/amadeus/resources/images/Amadeus_BG.png", "image/png"),
        ("/sw.js", "application/javascript; charset=UTF-8"),
    ] {
        let res = app.method(Method::HEAD, uri).await;
        assert_eq!(res.status, StatusCode::OK, "{uri}");
        assert_eq!(res.header("content-type"), Some(content_type), "{uri}");
        assert!(res.body.is_empty(), "HEAD {uri} must not carry a body");
    }
    let res = app
        .method(Method::HEAD, &format!("/amadeus/{HASHED_JS}"))
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.header("content-length"), Some("4800"));
    assert!(res.body.is_empty());
    let res = app.method(Method::HEAD, "/amadeus/nope").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn trailing_slash_and_repeated_slash_redirects() {
    let app = app();
    for (uri, location) in [
        ("/amadeus/app/?x=1&y=2", "/amadeus/app?x=1&y=2"),
        ("/amadeus/app/", "/amadeus/app"),
        ("/amadeus/app//", "/amadeus/app/"),
        ("/amadeus//app", "/amadeus/app"),
        ("/amadeus/docs/api/", "/amadeus/docs/api"),
        ("/amadeus/resources/images/", "/amadeus/resources/images"),
        ("/resources/images/", "/resources/images"),
    ] {
        let res = app.get(uri).await;
        assert_eq!(res.status, StatusCode::PERMANENT_REDIRECT, "{uri}");
        assert_eq!(res.header("location"), Some(location), "{uri}");
        assert_eq!(
            res.header("refresh"),
            Some(format!("0;url={location}").as_str()),
            "{uri}"
        );
        assert_eq!(res.text(), location);
        // Next.js adds no rule headers to redirects.
        for (name, _) in NO_STORE {
            assert!(res.header(name).is_none(), "{uri}: unexpected {name}");
        }
    }
    // API routes redirect too, for every method (308 keeps it).
    for method in [Method::POST, Method::OPTIONS, Method::GET] {
        let res = app
            .method(method.clone(), "/amadeus/app/api/tts/health/")
            .await;
        assert_eq!(res.status, StatusCode::PERMANENT_REDIRECT, "{method}");
        assert_eq!(res.header("location"), Some("/amadeus/app/api/tts/health"));
    }
    // V2: the base itself is served with or without the trailing slash.
    for uri in ["/amadeus", "/amadeus/", "/amadeus/?lang=ja"] {
        let res = app.get(uri).await;
        assert_eq!(
            (res.status, res.text().as_str()),
            (StatusCode::OK, INDEX_HTML),
            "{uri}"
        );
    }
}

#[tokio::test]
async fn origin_root_and_paths_outside_the_base() {
    let app = app();
    let res = app.get("/").await;
    assert_eq!(res.status, StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(res.header("location"), Some("/amadeus"));
    for (name, _) in NO_STORE {
        assert!(res.header(name).is_none(), "GET /: unexpected {name}");
    }
    for uri in [
        "/other",
        "/amadeusx",
        "/resources/images/Amadeus_BG.png",
        "/amadeus_logo.png",
        "/favicon.ico",
    ] {
        let res = app.get(uri).await;
        assert_eq!(res.status, StatusCode::NOT_FOUND, "{uri}");
        assert!(res.body.is_empty());
        assert_eq!(res.header("cache-control"), Some(NOT_FOUND_CC));
    }
    // Routing is case-sensitive like V1.
    assert_eq!(app.get("/AMADEUS/APP").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn post_routes_auto_implement_options_and_405() {
    let app = app();
    for rel in POST_ROUTES {
        let uri = api(rel);
        let res = app.method(Method::OPTIONS, &uri).await;
        res.assert_empty(204);
        assert_eq!(res.header("allow"), Some("OPTIONS, POST"), "{uri}");
        res.assert_headers(&NO_STORE);
        assert!(
            res.header("access-control-allow-origin").is_none(),
            "no CORS on {uri}"
        );
        // A CORS preflight gets the same answer (same-origin only).
        let preflight = app
            .send(request(
                Method::OPTIONS,
                &uri,
                &[
                    ("origin", "https://evil.example"),
                    ("access-control-request-method", "POST"),
                ],
                Body::empty(),
            ))
            .await;
        preflight.assert_empty(204);
        assert!(preflight.header("access-control-allow-origin").is_none());
        for method in [
            Method::GET,
            Method::HEAD,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
        ] {
            let res = app.method(method.clone(), &uri).await;
            res.assert_empty(405);
            assert!(
                res.header("allow").is_none(),
                "{method} {uri}: V1 sent no Allow header"
            );
            res.assert_headers(&NO_STORE);
        }
        assert_non_next_methods(&app, &uri).await;
    }
}

/// Methods outside Next's `HTTP_METHODS` (observed on V1, every API route): an empty 400,
/// except `TRACE` (undici refuses to build the request: plain-text 500).
async fn assert_non_next_methods(app: &TestApp, uri: &str) {
    for name in ["PROPFIND", "PURGE", "SEARCH", "QUERY", "MKCOL"] {
        let method = Method::from_bytes(name.as_bytes()).expect("method");
        let res = app.method(method, uri).await;
        res.assert_empty(400);
        assert!(res.header("allow").is_none(), "{name} {uri}");
        res.assert_headers(&NO_STORE);
    }
    // Tokens Node's HTTP parser did not know got a bare 400 there; same status here.
    let res = app
        .method(Method::from_bytes(b"get").expect("method"), uri)
        .await;
    res.assert_empty(400);
    let res = app.method(Method::TRACE, uri).await;
    assert_eq!(res.status, StatusCode::INTERNAL_SERVER_ERROR, "TRACE {uri}");
    assert_eq!(res.header("content-type"), Some("text/plain"));
    assert_eq!(res.text(), "Internal Server Error");
    res.assert_headers(&NO_STORE);
}

#[tokio::test]
async fn quota_route_methods() {
    let app = app();
    let uri = api(paths::API_CHAT_QUOTA);
    let res = app.get(&uri).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.header("content-type"), Some("application/json"));
    res.assert_headers(&NO_STORE);
    let body = res.json();
    for key in ["count", "max", "remaining"] {
        assert!(
            body.get(key).is_some_and(serde_json::Value::is_number),
            "quota.{key}: {body}"
        );
    }
    let res = app.method(Method::OPTIONS, &uri).await;
    res.assert_empty(204);
    assert_eq!(res.header("allow"), Some("GET, HEAD, OPTIONS"));
    for method in [Method::POST, Method::PUT, Method::DELETE, Method::PATCH] {
        let res = app.method(method, &uri).await;
        res.assert_empty(405);
        assert!(res.header("allow").is_none());
    }
    assert_non_next_methods(&app, &uri).await;
}

/// API-013: `--free-api-max` / `AMADEUS_FREEAPI_MAX` is the cap the quota reports and both
/// chat routes enforce (V1 hard-coded 65000; the counter file keeps V1's format).
#[tokio::test]
async fn quota_and_cap_follow_free_api_max() {
    let app = app_full(&[(REQUEST_COUNT_FILE, b"65000")], |c| {
        c.free_api_max = 65_001
    });
    let quota = api(paths::API_CHAT_QUOTA);
    app.get(&quota)
        .await
        .assert_json(200, r#"{"count":65000,"max":65001,"remaining":1}"#);
    // A provider-less request is counted, then fails like V1 (OpenAI without a key)...
    let no_provider = r#"{"messages":[{"role":"user","content":"hi"}]}"#;
    app.post_json(&api(paths::API_CHAT), no_provider)
        .await
        .assert_json(
            400,
            r#"{"error":"API Key が設定されていません。CONFIGから設定してください。"}"#,
        );
    // ...and the next one hits the configured cap, on /chat and /chat/stream alike.
    let capped = r#"{"error":"FreeAPIの全体リクエスト上限（65001回）に達しました。サービスを停止しています。"}"#;
    app.post_json(&api(paths::API_CHAT), no_provider)
        .await
        .assert_json(429, capped);
    app.post_json(&api(paths::API_CHAT_STREAM), no_provider)
        .await
        .assert_json(429, capped);
    app.get(&quota)
        .await
        .assert_json(200, r#"{"count":65001,"max":65001,"remaining":0}"#);
    let counter = std::fs::read_to_string(app.data.path().join(REQUEST_COUNT_FILE));
    assert_eq!(counter.ok().as_deref(), Some("65001"));
    // The default stays V1's 65000.
    app_full(&[(REQUEST_COUNT_FILE, b"12")], |_| {})
        .get(&quota)
        .await
        .assert_json(200, r#"{"count":12,"max":65000,"remaining":64988}"#);
}

#[tokio::test]
async fn unknown_api_paths_are_json_404() {
    let app = app();
    for (method, uri) in [
        (Method::GET, "/amadeus/app/api/nope"),
        (Method::POST, "/amadeus/app/api/tts/nope"),
        (Method::OPTIONS, "/amadeus/app/api/chat/stream/x"),
        (Method::GET, "/amadeus/app/api"),
    ] {
        let res = app.method(method.clone(), uri).await;
        res.assert_json(404, r#"{"error":"Not found"}"#);
        assert_eq!(
            res.header("cache-control"),
            Some(NOT_FOUND_CC),
            "{method} {uri}"
        );
    }
}

#[tokio::test]
async fn static_files_and_cache_policy() {
    let app = app();

    let res = app.get("/amadeus/resources/images/Amadeus_BG.png").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.header("content-type"), Some("image/png"));
    assert_eq!(res.body.as_ref(), PNG_BYTES);
    res.assert_headers(&PUBLIC);
    assert!(res.header("last-modified").is_some());

    // Percent-encoded spaces and UTF-8, query strings ignored (`?v=18` cache busters).
    let res = app
        .get("/amadeus/resources/fonts/Eurostile%20Condensed%20Bold.otf?v=18")
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.header("content-type"), Some("font/otf"));
    res.assert_headers(&PUBLIC);
    let res = app
        .get("/amadeus/resources/models/%E7%B4%85%E8%8E%89%E6%A0%96/model.json")
        .await;
    assert_eq!(
        (res.status, res.text().as_str()),
        (StatusCode::OK, "{\"v\":1}")
    );
    assert_eq!(res.header("content-type"), Some("application/json"));
    res.assert_headers(&PUBLIC);

    // V1's mixed quirk: a /resources 404 has the private Cache-Control but the public rest.
    let res = app.get("/amadeus/resources/nonexistent.png").await;
    assert_eq!(res.status, StatusCode::NOT_FOUND);
    assert_eq!(res.header("cache-control"), Some(NOT_FOUND_CC));
    res.assert_headers(&PUBLIC[1..]);

    // Everything else (V1 root files, bundles) is no-store, like V1.
    let res = app.get("/amadeus/static/amadeus_logo.png").await;
    assert_eq!(
        (res.status, res.header("content-type")),
        (StatusCode::OK, Some("image/png"))
    );
    res.assert_headers(&NO_STORE);
    let res = app.get(&format!("/amadeus/{HASHED_JS}")).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.header("content-type"), Some("text/javascript"));
    res.assert_headers(&NO_STORE);
    let res = app.get(&format!("/amadeus/{HASHED_WASM}")).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.header("content-type"), Some("application/wasm"));
    assert_eq!(res.body.as_ref(), WASM_BYTES);
    res.assert_headers(&NO_STORE);
    let res = app.get("/amadeus/index.html").await;
    assert_eq!(
        (res.status, res.text().as_str()),
        (StatusCode::OK, INDEX_HTML)
    );
    res.assert_headers(&NO_STORE);
}

#[tokio::test]
async fn range_and_conditional_requests() {
    let app = app();
    let uri = "/amadeus/resources/images/Amadeus_BG.png";
    let res = app
        .send(request(
            Method::GET,
            uri,
            &[("range", "bytes=0-3")],
            Body::empty(),
        ))
        .await;
    assert_eq!(res.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(res.body.as_ref(), &PNG_BYTES[..4]);
    let last_modified = app
        .get(uri)
        .await
        .header("last-modified")
        .map(str::to_owned)
        .unwrap_or_default();
    let res = app
        .send(request(
            Method::GET,
            uri,
            &[("if-modified-since", &last_modified)],
            Body::empty(),
        ))
        .await;
    assert_eq!(res.status, StatusCode::NOT_MODIFIED);
    res.assert_headers(&PUBLIC);
}

#[tokio::test]
async fn path_traversal_never_leaves_the_dist() {
    let app = app();
    for uri in [
        "/amadeus/../secret.txt",
        "/amadeus/..%2Fsecret.txt",
        "/amadeus/%2e%2e/secret.txt",
        "/amadeus/resources/..%2F..%2Fsecret.txt",
        "/amadeus/static/%2E%2E/%2E%2E/secret.txt",
        "/amadeus/..%5Csecret.txt",
    ] {
        let res = app.get(uri).await;
        assert!(
            !res.text().contains(SECRET),
            "{uri} leaked a file outside the dist"
        );
        assert_ne!(res.status, StatusCode::OK, "{uri}");
    }
}

#[tokio::test]
async fn manifest_healthz_and_service_worker() {
    let app = app();
    let res = app.get("/amadeus/manifest.webmanifest").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.header("content-type"),
        Some("application/manifest+json")
    );
    assert_eq!(res.text(), MANIFEST);
    res.assert_headers(&NO_STORE);

    let res = app.get("/amadeus/healthz").await;
    assert_eq!((res.status, res.text().as_str()), (StatusCode::OK, "ok"));
    res.assert_headers(&NO_STORE);

    let res = app.get("/sw.js").await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(
        res.header("content-type"),
        Some("application/javascript; charset=UTF-8")
    );
    assert_eq!(res.body.len(), 243);
    assert_eq!(res.text(), SERVICE_WORKER_JS);
    res.assert_headers(&NO_STORE);
}

#[tokio::test]
async fn compression_for_compressible_responses_only() {
    let app = app();
    let uri = format!("/amadeus/{HASHED_JS}");
    for encoding in ["gzip", "br"] {
        let res = app
            .send(request(
                Method::GET,
                &uri,
                &[("accept-encoding", encoding)],
                Body::empty(),
            ))
            .await;
        assert_eq!(res.status, StatusCode::OK);
        assert_eq!(res.header("content-encoding"), Some(encoding));
        assert!(res.body.len() < 4800, "{encoding} body must be compressed");
        res.assert_headers(&NO_STORE);
    }
    // Tiny responses and images stay identity.
    let res = app
        .send(request(
            Method::GET,
            &format!("/amadeus/{SMALL_CSS}"),
            &[("accept-encoding", "gzip")],
            Body::empty(),
        ))
        .await;
    assert!(res.header("content-encoding").is_none());
    let res = app
        .send(request(
            Method::GET,
            "/amadeus/resources/images/Amadeus_BG.png",
            &[("accept-encoding", "gzip")],
            Body::empty(),
        ))
        .await;
    assert!(res.header("content-encoding").is_none());
}

#[tokio::test]
async fn immutable_hashed_assets_when_enabled() {
    let app = app_with(|c| c.immutable_assets = true);
    for asset in [HASHED_JS, HASHED_WASM, SMALL_CSS] {
        let res = app.get(&format!("/amadeus/{asset}")).await;
        assert_eq!(res.status, StatusCode::OK, "{asset}");
        assert_eq!(
            res.header("cache-control"),
            Some("public, max-age=31536000, immutable")
        );
        assert!(res.header("pragma").is_none() && res.header("expires").is_none());
    }
    // Pages, the manifest and resources keep their V1 policies.
    app.get("/amadeus/app").await.assert_headers(&NO_STORE);
    app.get("/amadeus/manifest.webmanifest")
        .await
        .assert_headers(&NO_STORE);
    app.get("/amadeus/resources/images/Amadeus_BG.png")
        .await
        .assert_headers(&PUBLIC);
}

#[tokio::test]
async fn root_base_path() {
    let app = app_with(|c| c.base_path = String::new());
    let res = app.get("/").await;
    assert_eq!(
        (res.status, res.text().as_str()),
        (StatusCode::OK, INDEX_HTML)
    );
    assert_eq!(app.get("/docs/api/vertex").await.status, StatusCode::OK);
    assert_eq!(app.get("/amadeus/app").await.status, StatusCode::NOT_FOUND);
    app.get("/resources/images/Amadeus_BG.png")
        .await
        .assert_headers(&PUBLIC);
    let res = app.method(Method::OPTIONS, "/app/api/tts/health").await;
    assert_eq!(
        (res.status, res.header("allow")),
        (StatusCode::NO_CONTENT, Some("OPTIONS, POST"))
    );
    let res = app.get("/app/").await;
    assert_eq!(
        (res.status, res.header("location")),
        (StatusCode::PERMANENT_REDIRECT, Some("/app"))
    );
    assert_eq!(app.get("/healthz").await.text(), "ok");
}

#[tokio::test]
async fn nested_custom_base_path() {
    let app = app_with(|c| c.base_path = "/v2/amadeus".to_owned());
    assert_eq!(app.get("/v2/amadeus/app").await.status, StatusCode::OK);
    assert_eq!(app.get("/v2/amadeus/").await.status, StatusCode::OK);
    assert_eq!(app.get("/amadeus/app").await.status, StatusCode::NOT_FOUND);
    let res = app.get("/").await;
    assert_eq!(
        (res.status, res.header("location")),
        (StatusCode::TEMPORARY_REDIRECT, Some("/v2/amadeus"))
    );
    app.get("/v2/amadeus/resources/images/Amadeus_BG.png")
        .await
        .assert_headers(&PUBLIC);
    assert_eq!(
        app.method(Method::OPTIONS, "/v2/amadeus/app/api/rag/embed")
            .await
            .status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn missing_index_html_is_a_clear_500() {
    let app = app();
    std::fs::remove_file(app.dist.path().join("index.html")).expect("remove index.html");
    let res = app.get("/amadeus/app").await;
    assert_eq!(res.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(res.text().contains("index.html"));
}

#[tokio::test]
async fn chat_routes_parse_bodies_leniently() {
    let app = app();
    let invalid = r#"{"error":"Invalid request payload"}"#;
    for rel in [
        paths::API_CHAT,
        paths::API_CHAT_STREAM,
        paths::API_CHAT_VERTEX_PROXY,
    ] {
        for body in ["", "not json", "null", "[]", "{\"messages\":\"x\"}"] {
            app.post_json(&api(rel), body)
                .await
                .assert_json(400, invalid);
        }
    }
    // The stream flag does not change pre-stream validation errors.
    let res = app
        .send(request(
            Method::POST,
            &api(paths::API_CHAT_VERTEX_PROXY),
            &[("x-vertex-stream", "true")],
            Body::from("not json"),
        ))
        .await;
    res.assert_json(400, invalid);
    res.assert_headers(&NO_STORE);
}
