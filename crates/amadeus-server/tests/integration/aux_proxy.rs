//! TTS / STT / RAG proxies through the full router against a recording mock upstream
//! (spec-server-aux.md §14 regression matrix, rows 1-40, plus V2 additions).

use std::time::{Duration, Instant};

use amadeus_backend::aux::form::{parse_form, FormValue};
use amadeus_backend::aux::health::{probe_with_timeout, HealthBodyMode};
use amadeus_backend::aux::AuxContext;
use amadeus_backend::Backend;
use amadeus_core::protocol::{ApiReply, SttTranscribeFields};
use amadeus_server::ServerConfig;
use axum::body::Body;
use axum::http::Method;
use serde_json::json;

use crate::common::*;

const TTS_HEALTH: &str = "/amadeus/app/api/tts/health";
const STT_HEALTH: &str = "/amadeus/app/api/stt/health";
const RAG_HEALTH: &str = "/amadeus/app/api/rag/health";
const TTS_SYNTH: &str = "/amadeus/app/api/tts/synthesize";
const STT_TRANSCRIBE: &str = "/amadeus/app/api/stt/transcribe";
const RAG_EMBED: &str = "/amadeus/app/api/rag/embed";

const GOOD_SPREAD: &str = r#"{"available":"yes-from-upstream","status":"ok","runtime":{"using_gpu":true,"device":"cuda","gpu_name":"MockGPU","attn_implementation":"sdpa"},"torch":{"cuda_available":true,"detected_gpu_name":"MockGPU"}}"#;
const MISSING_URL_HEALTH: &str = r#"{"available":false,"error":"serverUrl is required"}"#;
const DOWN: &str = r#"{"available":false,"error":"503 Service Unavailable"}"#;
const FETCH_FAILED_HEALTH: &str = r#"{"available":false,"error":"fetch failed"}"#;
const ABORTED: &str = r#"{"available":false,"error":"This operation was aborted"}"#;

fn server_url(url: &str) -> String {
    json!({ "serverUrl": url }).to_string()
}

async fn transcribe(app: &TestApp, content_type: &str, body: Vec<u8>) -> TestResponse {
    app.send(request(
        Method::POST,
        STT_TRANSCRIBE,
        &[("content-type", content_type)],
        Body::from(body),
    ))
    .await
}

// ----- health probes ---------------------------------------------------------------------

#[tokio::test]
async fn tts_health_matrix() {
    let mock = start_mock().await;
    let app = app();
    // Rows 1-3: missing / invalid / blank serverUrl.
    for body in [
        "",
        "not json",
        r#"{"serverUrl":"   "}"#,
        "[1]",
        r#"{"serverUrl":null}"#,
        r#""x""#,
    ] {
        app.post_json(TTS_HEALTH, body)
            .await
            .assert_json(400, MISSING_URL_HEALTH);
    }
    // Row 4: spread with `available` overridden in place.
    app.post_json(TTS_HEALTH, &server_url(&mock.url("good")))
        .await
        .assert_json(200, GOOD_SPREAD);
    assert_eq!(mock.last("/good/health").method, "GET");
    assert!(mock.last("/good/health").body.is_empty());
    // Row 5: non-2xx → 200 + reason phrase; also the trimmed URL and a custom reason.
    app.post_json(
        TTS_HEALTH,
        &server_url(&format!("  {}\u{3000}", mock.url("bad"))),
    )
    .await
    .assert_json(200, DOWN);
    app.post_json(TTS_HEALTH, &server_url(&mock.url("custom")))
        .await
        .assert_json(200, r#"{"available":false,"error":"599 Custom Reason"}"#);
    // Row 6: refused.
    app.post_json(TTS_HEALTH, &server_url(REFUSED))
        .await
        .assert_json(200, FETCH_FAILED_HEALTH);
    // Row 7: unparsable URL (naive concatenation).
    app.post_json(TTS_HEALTH, &server_url("abc"))
        .await
        .assert_json(
            200,
            r#"{"available":false,"error":"Failed to parse URL from abc/health"}"#,
        );
    // Row 8: non-string serverUrl → the JS TypeError → empty 500.
    for body in [
        r#"{"serverUrl":5}"#,
        r#"{"serverUrl":true}"#,
        r#"{"serverUrl":{}}"#,
        r#"{"serverUrl":["x"]}"#,
    ] {
        app.post_json(TTS_HEALTH, body).await.assert_empty(500);
    }
    // Array bodies spread their indices; primitives add nothing.
    app.post_json(TTS_HEALTH, &server_url(&mock.url("array")))
        .await
        .assert_json(200, r#"{"0":"a","1":"b","available":true}"#);
    app.post_json(TTS_HEALTH, &server_url(&mock.url("text")))
        .await
        .assert_json(200, r#"{"available":true}"#);
    // A body that arrives late (still inside the 5 s deadline) is spread.
    app.post_json(TTS_HEALTH, &server_url(&mock.url("slowbody")))
        .await
        .assert_json(200, r#"{"available":true,"late":true}"#);
    // Redirects are followed (≤ 20, like undici).
    app.post_json(TTS_HEALTH, &server_url(&mock.url("redirect")))
        .await
        .assert_json(200, GOOD_SPREAD);
}

#[tokio::test]
async fn health_ignores_the_request_content_type() {
    let mock = start_mock().await;
    let app = app();
    // Row 15: any Content-Type is parsed as JSON (no axum `Json` extractor).
    for content_type in ["text/plain", "application/x-www-form-urlencoded", ""] {
        let res = app
            .send(request(
                Method::POST,
                RAG_HEALTH,
                &[("content-type", content_type)],
                Body::from(server_url(REFUSED)),
            ))
            .await;
        res.assert_json(200, FETCH_FAILED_HEALTH);
    }
    let res = app
        .send(request(
            Method::POST,
            STT_HEALTH,
            &[],
            Body::from(server_url(&mock.url("good"))),
        ))
        .await;
    res.assert_json(200, GOOD_SPREAD);
    // A leading UTF-8 BOM is tolerated like undici's `request.json()`.
    let res = app
        .send(request(
            Method::POST,
            RAG_HEALTH,
            &[],
            Body::from(format!("\u{feff}{}", server_url(&mock.url("good")))),
        ))
        .await;
    res.assert_json(200, r#"{"available":true}"#);
}

#[tokio::test]
async fn stt_and_rag_health() {
    let mock = start_mock().await;
    let app = app();
    // Row 12 (STT): spread after the OK check.
    app.post_json(STT_HEALTH, &server_url(&mock.url("good")))
        .await
        .assert_json(200, GOOD_SPREAD);
    app.post_json(STT_HEALTH, &server_url(&mock.url("bad")))
        .await
        .assert_json(200, DOWN);
    app.post_json(STT_HEALTH, "")
        .await
        .assert_json(400, MISSING_URL_HEALTH);
    app.post_json(STT_HEALTH, r#"{"serverUrl":1}"#)
        .await
        .assert_empty(500);
    // Rows 13-14 (RAG): the body is never read.
    app.post_json(RAG_HEALTH, &server_url(&mock.url("good")))
        .await
        .assert_json(200, r#"{"available":true}"#);
    app.post_json(RAG_HEALTH, &server_url(&mock.url("bad")))
        .await
        .assert_json(200, DOWN);
    app.post_json(RAG_HEALTH, &server_url(REFUSED))
        .await
        .assert_json(200, FETCH_FAILED_HEALTH);
    app.post_json(RAG_HEALTH, r#"{}"#)
        .await
        .assert_json(400, MISSING_URL_HEALTH);
}

#[tokio::test]
async fn health_deadline_is_five_seconds() {
    let mock = start_mock().await;
    let app = app();
    let url = server_url(&mock.url("slow"));
    let started = Instant::now();
    let (tts, stt, rag) = tokio::join!(
        app.post_json(TTS_HEALTH, &url),
        app.post_json(STT_HEALTH, &url),
        app.post_json(RAG_HEALTH, &url)
    );
    let elapsed = started.elapsed();
    for res in [tts, stt, rag] {
        res.assert_json(200, ABORTED);
    }
    assert!(
        elapsed >= Duration::from_millis(4900),
        "aborted too early: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(6900),
        "aborted too late: {elapsed:?}"
    );
}

#[tokio::test]
async fn health_deadline_also_covers_the_body_read() {
    let mock = start_mock().await;
    let http = amadeus_backend::http::build_client().expect("client");
    let ctx = AuxContext {
        http: &http,
        allowlist: None,
    };
    let short = Duration::from_millis(300);
    // Headers arrive in time, the body does not: `body = null` (not an error).
    for mode in [
        HealthBodyMode::ReadBeforeStatus,
        HealthBodyMode::ReadAfterOk,
        HealthBodyMode::Ignore,
    ] {
        let reply = probe_with_timeout(ctx, &mock.url("slowbody"), mode, short).await;
        assert_eq!(
            reply,
            ApiReply::json(200, json!({"available": true})),
            "{mode:?}"
        );
    }
    let reply = probe_with_timeout(ctx, &mock.url("slow"), HealthBodyMode::Ignore, short).await;
    assert_eq!(
        reply,
        ApiReply::json(
            200,
            json!({"available": false, "error": "This operation was aborted"})
        )
    );
}

// ----- rag/embed -------------------------------------------------------------------------

#[tokio::test]
async fn rag_embed_matrix() {
    let mock = start_mock().await;
    let app = app();
    // Row 16: trimmed text forwarded, upstream JSON passed through verbatim.
    let body = json!({"serverUrl": mock.url("good"), "text": "  hello 世界  "}).to_string();
    app.post_json(RAG_EMBED, &body)
        .await
        .assert_json(200, r#"{"embedding":[0.25,-0.5,1],"dim":3}"#);
    let upstream = mock.last("/good/embed");
    assert_eq!(upstream.method, "POST");
    assert_eq!(upstream.header("content-type"), Some("application/json"));
    assert_eq!(upstream.text(), r#"{"text":"hello 世界"}"#);
    // Row 17: unparsable URL.
    let body = json!({"serverUrl": "http://127.0.0.1:58100x", "text": "a"}).to_string();
    app.post_json(RAG_EMBED, &body).await.assert_json(
        500,
        r#"{"error":"Failed to parse URL from http://127.0.0.1:58100x/embed"}"#,
    );
    // Row 18: upstream 500 with a text body.
    let body = json!({"serverUrl": mock.url("bad"), "text": "a"}).to_string();
    app.post_json(RAG_EMBED, &body)
        .await
        .assert_json(500, r#"{"error":"Embedding failed (500)"}"#);
    // Upstream `error` values pass through as any JSON value, with the upstream status.
    let body = json!({"serverUrl": mock.url("objerror"), "text": "a"}).to_string();
    app.post_json(RAG_EMBED, &body)
        .await
        .assert_json(400, r#"{"error":{"code":7}}"#);
    // `body ?? {}`: JSON null becomes {}, other primitives pass through.
    let body = json!({"serverUrl": mock.url("nullbody"), "text": "a"}).to_string();
    app.post_json(RAG_EMBED, &body).await.assert_json(200, "{}");
    let body = json!({"serverUrl": mock.url("primitive"), "text": "a"}).to_string();
    app.post_json(RAG_EMBED, &body)
        .await
        .assert_json(200, "false");
    // Rows 19-21.
    let body = json!({"serverUrl": mock.url("good"), "text": "   "}).to_string();
    app.post_json(RAG_EMBED, &body)
        .await
        .assert_json(400, r#"{"error":"text is required"}"#);
    app.post_json(RAG_EMBED, r#"{"text":"a"}"#)
        .await
        .assert_json(400, r#"{"error":"serverUrl is required"}"#);
    let body = json!({"serverUrl": REFUSED, "text": "a"}).to_string();
    app.post_json(RAG_EMBED, &body)
        .await
        .assert_json(500, r#"{"error":"fetch failed"}"#);
    // Every field is read before any check: a non-string text is the empty 500.
    app.post_json(RAG_EMBED, r#"{"text":5}"#)
        .await
        .assert_empty(500);
}

// ----- tts/synthesize --------------------------------------------------------------------

#[tokio::test]
async fn tts_synthesize_matrix() {
    let mock = start_mock().await;
    let app = app();
    let good = mock.url("good");
    // Row 22: audio bytes and content type passed through, defaults applied.
    let res = app
        .post_json(
            TTS_SYNTH,
            &json!({"serverUrl": good, "text": "  こんにちは  "}).to_string(),
        )
        .await;
    assert_eq!(res.status.as_u16(), 200);
    assert_eq!(res.header("content-type"), Some("audio/x-wav"));
    assert_eq!(res.body.as_ref(), WAV_BYTES);
    // The route's `Cache-Control: no-store` is replaced by the global rule (V1).
    res.assert_headers(&NO_STORE);
    let upstream = mock.last("/good/synthesize");
    assert_eq!(upstream.header("content-type"), Some("application/json"));
    assert_eq!(
        upstream.text(),
        r#"{"text":"こんにちは","speaker":"christina-jp","use_cache":true}"#
    );
    // Rows 23-25: speaker trimming / fallback and `useCache ?? true` passed through untouched.
    for (extra, expected) in [
        (
            json!({"speaker": "  christina ", "useCache": false}),
            r#"{"text":"x","speaker":"christina","use_cache":false}"#,
        ),
        (
            json!({"speaker": "   ", "useCache": null}),
            r#"{"text":"x","speaker":"christina-jp","use_cache":true}"#,
        ),
        (
            json!({"useCache": "nope"}),
            r#"{"text":"x","speaker":"christina-jp","use_cache":"nope"}"#,
        ),
        (
            json!({"useCache": {"a": [1.0, 2.5]}}),
            r#"{"text":"x","speaker":"christina-jp","use_cache":{"a":[1,2.5]}}"#,
        ),
    ] {
        let mut body = json!({"serverUrl": good, "text": "x"});
        if let (Some(map), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            map.extend(extra.clone());
        }
        let res = app.post_json(TTS_SYNTH, &body.to_string()).await;
        assert_eq!(res.status.as_u16(), 200);
        assert_eq!(mock.last("/good/synthesize").text(), expected);
    }
    // Row 26: upstream error → upstream status + body text.
    let res = app
        .post_json(
            TTS_SYNTH,
            &json!({"serverUrl": mock.url("bad"), "text": "x"}).to_string(),
        )
        .await;
    res.assert_json(
        500,
        r#"{"error":"Synthesis failed (500): 合成エラー detail"}"#,
    );
    // Row 27-29.
    app.post_json(
        TTS_SYNTH,
        &json!({"serverUrl": REFUSED, "text": "x"}).to_string(),
    )
    .await
    .assert_json(500, r#"{"error":"fetch failed"}"#);
    app.post_json(TTS_SYNTH, &json!({"serverUrl": good}).to_string())
        .await
        .assert_json(400, r#"{"error":"text is required"}"#);
    app.post_json(TTS_SYNTH, "{}")
        .await
        .assert_json(400, r#"{"error":"serverUrl is required"}"#);
    // A non-string speaker throws before the serverUrl check (empty 500).
    app.post_json(TTS_SYNTH, r#"{"speaker":1}"#)
        .await
        .assert_empty(500);
    // Content type: absent → audio/wav, present-but-empty stays empty; empty audio → empty 200.
    let res = app
        .post_json(
            TTS_SYNTH,
            &json!({"serverUrl": mock.url("nocontenttype"), "text": "x"}).to_string(),
        )
        .await;
    assert_eq!(
        (res.status.as_u16(), res.header("content-type")),
        (200, Some("audio/wav"))
    );
    assert_eq!(res.body.as_ref(), b"AUDIO");
    let res = app
        .post_json(
            TTS_SYNTH,
            &json!({"serverUrl": mock.url("emptycontenttype"), "text": "x"}).to_string(),
        )
        .await;
    assert_eq!(
        (res.status.as_u16(), res.header("content-type")),
        (200, Some(""))
    );
    let res = app
        .post_json(
            TTS_SYNTH,
            &json!({"serverUrl": mock.url("empty"), "text": "x"}).to_string(),
        )
        .await;
    assert_eq!((res.status.as_u16(), res.body.len()), (200, 0));
    // User info in the URL: undici refuses to build the request.
    app.post_json(TTS_SYNTH, &json!({"serverUrl": "http://u:p@127.0.0.1:1", "text": "x"}).to_string())
        .await
        .assert_json(
            500,
            r#"{"error":"Request cannot be constructed from a URL that includes credentials: http://u:p@127.0.0.1:1/synthesize"}"#,
        );
}

// ----- stt/transcribe --------------------------------------------------------------------

#[tokio::test]
async fn stt_transcribe_matrix() {
    let mock = start_mock().await;
    let app = app();
    let good = mock.url("good");
    let audio = b"OggS-fake-opus-audio".as_slice();

    // Row 30: parts forwarded (audio renamed, serverUrl dropped), upstream JSON passed through.
    let body = multipart(&[
        FormPart::File("audio", "blob", Some("audio/webm;codecs=opus"), audio),
        FormPart::Text("provider", "openai"),
        FormPart::Text("language", "en"),
        FormPart::Text("serverUrl", &format!("  {good}  ")),
    ]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(200, r#"{"text":"こんにちは","provider_used":"mock"}"#);
    let upstream = mock.last("/good/transcribe");
    let form =
        parse_form(upstream.header("content-type"), &upstream.body).expect("upstream multipart");
    let names: Vec<&str> = form.entries().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, ["audio", "provider", "language"]);
    match form.get("audio") {
        Some(FormValue::File(file)) => {
            assert_eq!(file.filename, "recording.webm");
            assert_eq!(file.content_type, "audio/webm;codecs=opus");
            assert_eq!(file.data.as_ref(), audio);
        }
        other => panic!("audio part: {other:?}"),
    }
    assert!(
        upstream
            .text()
            .contains("Content-Type: audio/webm;codecs=opus\r\n"),
        "verbatim part type"
    );
    assert_eq!(
        form.get("provider"),
        Some(&FormValue::Text("openai".into()))
    );
    assert_eq!(form.get("language"), Some(&FormValue::Text("en".into())));

    // Row 31: defaults; a part without Content-Type is a text/plain Blob.
    let body = multipart(&[
        FormPart::File("audio", "a.mp4", None, audio),
        FormPart::Text("serverUrl", &good),
    ]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(200, r#"{"text":"こんにちは","provider_used":"mock"}"#);
    let form = {
        let up = mock.last("/good/transcribe");
        parse_form(up.header("content-type"), &up.body).expect("upstream multipart")
    };
    assert_eq!(form.get("provider"), Some(&FormValue::Text("groq".into())));
    assert_eq!(form.get("language"), Some(&FormValue::Text("ja".into())));
    assert!(
        matches!(form.get("audio"), Some(FormValue::File(f)) if f.content_type == "text/plain")
    );

    // Row 32: present-but-empty fields stay empty; an empty part type → octet-stream.
    let body = multipart(&[
        FormPart::File("audio", "a", Some(""), audio),
        FormPart::Text("provider", ""),
        FormPart::Text("language", ""),
        FormPart::Text("serverUrl", &good),
    ]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(200, r#"{"text":"こんにちは","provider_used":"mock"}"#);
    let up = mock.last("/good/transcribe");
    assert!(up
        .text()
        .contains("Content-Type: application/octet-stream\r\n"));
    let form = parse_form(up.header("content-type"), &up.body).expect("upstream multipart");
    assert_eq!(form.get("provider"), Some(&FormValue::Text(String::new())));
    assert_eq!(form.get("language"), Some(&FormValue::Text(String::new())));

    // Row 33: upstream error JSON → upstream status; FastAPI `detail` → fallback text.
    let bad = |url: String| {
        multipart(&[
            FormPart::File("audio", "a", Some("audio/ogg"), audio),
            FormPart::Text("serverUrl", &url),
        ])
    };
    transcribe(&app, &multipart_type(), bad(mock.url("bad")))
        .await
        .assert_json(422, r#"{"error":"bad audio"}"#);
    transcribe(&app, &multipart_type(), bad(mock.url("detail")))
        .await
        .assert_json(422, r#"{"error":"Transcription failed (422)"}"#);
    // Row 40: refused.
    transcribe(&app, &multipart_type(), bad(REFUSED.to_owned()))
        .await
        .assert_json(500, r#"{"error":"fetch failed"}"#);

    // Row 34-35: serverUrl is checked before the audio part.
    let body = multipart(&[
        FormPart::File("audio", "a", Some("audio/ogg"), audio),
        FormPart::Text("serverUrl", "  "),
    ]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(400, r#"{"error":"serverUrl is required"}"#);
    let body = multipart(&[
        FormPart::Text("audio", "hello"),
        FormPart::Text("serverUrl", &good),
    ]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(400, r#"{"error":"audio file is required"}"#);
    let body = multipart(&[FormPart::Text("serverUrl", &good)]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(400, r#"{"error":"audio file is required"}"#);

    // Row 36-39: form parsing like undici's `request.formData()`.
    let invalid = r#"{"error":"Invalid form data"}"#;
    transcribe(&app, "application/json", br#"{"serverUrl":"x"}"#.to_vec())
        .await
        .assert_json(400, invalid);
    let res = app
        .send(request(
            Method::POST,
            STT_TRANSCRIBE,
            &[],
            Body::from("a=b"),
        ))
        .await;
    res.assert_json(400, invalid);
    transcribe(
        &app,
        "application/x-www-form-urlencoded",
        format!("serverUrl={REFUSED}").into_bytes(),
    )
    .await
    .assert_json(400, r#"{"error":"audio file is required"}"#);
    transcribe(&app, "application/x-www-form-urlencoded", b"a=b".to_vec())
        .await
        .assert_json(400, r#"{"error":"serverUrl is required"}"#);
    transcribe(
        &app,
        "multipart/form-data; boundary=xyz",
        b"garbage".to_vec(),
    )
    .await
    .assert_json(400, invalid);

    // A file in a text field stringifies to "[object File]".
    let body = multipart(&[
        FormPart::File("audio", "a", Some("audio/ogg"), audio),
        FormPart::File("provider", "p.txt", Some("text/plain"), b"local"),
        FormPart::Text("serverUrl", &good),
    ]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(200, r#"{"text":"こんにちは","provider_used":"mock"}"#);
    let up = mock.last("/good/transcribe");
    let form = parse_form(up.header("content-type"), &up.body).expect("upstream multipart");
    assert_eq!(
        form.get("provider"),
        Some(&FormValue::Text("[object File]".into()))
    );
}

#[tokio::test]
async fn desktop_transcribe_parts_matches_the_web_path() {
    let mock = start_mock().await;
    let data = tempfile::tempdir().expect("tempdir");
    let backend = Backend::new(ServerConfig::new(data.path(), data.path()).backend_config())
        .expect("backend");
    let fields = SttTranscribeFields {
        provider: "local".into(),
        language: "ja".into(),
        server_url: format!(" {} ", mock.url("good")),
    };
    let reply = backend
        .stt_transcribe_parts(
            &fields,
            b"AUDIO".to_vec(),
            Some("Audio/Webm;Codecs=Opus".into()),
        )
        .await;
    assert_eq!(
        reply,
        ApiReply::json(200, json!({"text": "こんにちは", "provider_used": "mock"}))
    );
    let up = mock.last("/good/transcribe");
    let form = parse_form(up.header("content-type"), &up.body).expect("upstream multipart");
    assert!(
        matches!(form.get("audio"), Some(FormValue::File(f)) if f.content_type == "audio/webm;codecs=opus" && f.filename == "recording.webm")
    );
    assert_eq!(form.get("provider"), Some(&FormValue::Text("local".into())));
    let missing = SttTranscribeFields {
        server_url: "  ".into(),
        ..fields
    };
    let reply = backend
        .stt_transcribe_parts(&missing, Vec::new(), None)
        .await;
    assert_eq!(reply, ApiReply::error(400, "serverUrl is required"));
}

// ----- upstream redirects and relayed 3xx statuses ---------------------------------------

/// The open app (V1 behavior) and one whose allowlist admits the mock (guarded client).
fn open_and_guarded(mock: &MockUpstream) -> [TestApp; 2] {
    let entry = format!("127.0.0.1:{}", mock.port());
    [app(), app_with(|c| c.proxy_allowlist = Some(vec![entry]))]
}

/// Review defect 1: undici re-sends a `FormData` body on 307/308, so V1 followed them
/// (observed on V1: 200 after re-POSTing the same multipart); 301/302/303 turn the POST into
/// a GET without body.
#[tokio::test]
async fn stt_transcribe_follows_redirects_like_undici() {
    let mock = start_mock().await;
    let audio = b"OggS-redirected-audio".as_slice();
    for app in open_and_guarded(&mock) {
        // 307 with an absolute Location, 308 with a relative one.
        for kind in ["r307", "r308"] {
            let body = multipart(&[
                FormPart::File("audio", "blob", Some("audio/webm;codecs=opus"), audio),
                FormPart::Text("provider", "openai"),
                FormPart::Text("serverUrl", &mock.url(kind)),
            ]);
            let before = mock.requests().len();
            transcribe(&app, &multipart_type(), body)
                .await
                .assert_json(200, r#"{"text":"こんにちは","provider_used":"mock"}"#);
            let hops = mock.requests_since(before);
            let paths: Vec<&str> = hops.iter().map(|r| r.path.as_str()).collect();
            let first_path = format!("/{kind}/transcribe");
            assert_eq!(paths, [first_path.as_str(), "/good/transcribe"], "{kind}");
            let (first, replay) = (&hops[0], &hops[1]);
            assert_eq!(
                (first.method.as_str(), replay.method.as_str()),
                ("POST", "POST")
            );
            assert_eq!(
                replay.body, first.body,
                "{kind}: the multipart body is replayed"
            );
            assert_eq!(replay.header("content-type"), first.header("content-type"));
            assert!(replay.header("content-type").is_some_and(
                |ct| ct.starts_with("multipart/form-data; boundary=----formdata-undici-0")
            ));
            // Sized like undici's FormData body (content-length, never chunked).
            let length = replay.body.len().to_string();
            assert_eq!(replay.header("content-length"), Some(length.as_str()));
            assert!(replay.header("transfer-encoding").is_none());
            let form = parse_form(replay.header("content-type"), &replay.body)
                .expect("replayed multipart");
            let names: Vec<&str> = form.entries().iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(names, ["audio", "provider", "language"]);
            assert!(matches!(
                form.get("audio"),
                Some(FormValue::File(f)) if f.data.as_ref() == audio
                    && f.filename == "recording.webm"
                    && f.content_type == "audio/webm;codecs=opus"
            ));
            assert_eq!(
                form.get("provider"),
                Some(&FormValue::Text("openai".into()))
            );
            assert_eq!(form.get("language"), Some(&FormValue::Text("ja".into())));
        }
        // 303: the follow-up is a bodiless GET (the mock answers it with an empty 404).
        let body = multipart(&[
            FormPart::File("audio", "blob", Some("audio/ogg"), audio),
            FormPart::Text("serverUrl", &mock.url("r303")),
        ]);
        let before = mock.requests().len();
        transcribe(&app, &multipart_type(), body)
            .await
            .assert_json(404, r#"{"error":"Transcription failed (404)"}"#);
        let hops = mock.requests_since(before);
        assert_eq!(hops.len(), 2);
        assert_eq!(
            (hops[1].method.as_str(), hops[1].path.as_str()),
            ("GET", "/good/transcribe")
        );
        assert!(hops[1].body.is_empty());
        assert!(hops[1].header("content-type").is_none());
    }
}

/// Like undici, redirect hops carry no `Referer` (reqwest's default would send the previous
/// URL, query string included), with or without the allowlist.
#[tokio::test]
async fn redirect_hops_send_no_referer() {
    let mock = start_mock().await;
    let embed = json!({"serverUrl": format!("{}/hop/3", mock.base), "text": "x"}).to_string();
    for app in open_and_guarded(&mock) {
        let before = mock.requests().len();
        app.post_json(RAG_EMBED, &embed)
            .await
            .assert_json(200, r#"{"embedding":[1],"hops":"done"}"#);
        let hops = mock.requests_since(before);
        let paths: Vec<&str> = hops.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "/hop/3/embed",
                "/hop/2/embed",
                "/hop/1/embed",
                "/hop/0/embed"
            ]
        );
        for hop in &hops {
            assert_eq!(hop.header("referer"), None, "{}", hop.path);
        }
    }
}

/// Review defect 3: a fetch follows 20 redirects and fails on the 21st (`fetch failed`
/// without requesting its target), with or without `AMADEUS_PROXY_ALLOWLIST`.
#[tokio::test]
async fn redirect_limit_is_twenty_with_or_without_the_allowlist() {
    let mock = start_mock().await;
    // `{base}/hop/{n}` + `/embed` → n 307 hops, then `{"embedding":[1],"hops":"done"}`.
    let hops = |n: u32| format!("{}/hop/{n}", mock.base);
    let embed = |n: u32| json!({"serverUrl": hops(n), "text": "x"}).to_string();
    for app in open_and_guarded(&mock) {
        let before = mock.requests().len();
        app.post_json(RAG_EMBED, &embed(20))
            .await
            .assert_json(200, r#"{"embedding":[1],"hops":"done"}"#);
        assert_eq!(mock.requests_since(before).len(), 21);

        let before = mock.requests().len();
        app.post_json(RAG_EMBED, &embed(21))
            .await
            .assert_json(500, r#"{"error":"fetch failed"}"#);
        let paths: Vec<String> = mock
            .requests_since(before)
            .into_iter()
            .map(|r| r.path)
            .collect();
        assert_eq!(paths.len(), 21, "{paths:?}");
        assert_eq!(paths.last().map(String::as_str), Some("/hop/1/embed"));

        // The health probes share the limit.
        app.post_json(RAG_HEALTH, &server_url(&hops(20)))
            .await
            .assert_json(200, r#"{"available":true}"#);
        app.post_json(RAG_HEALTH, &server_url(&hops(21)))
            .await
            .assert_json(200, FETCH_FAILED_HEALTH);
    }
}

/// Review defect 2: an upstream 3xx that cannot be followed (no `Location`) is relayed with its
/// status, and the `next.config.ts` rule headers still apply to it (observed on V1); only
/// Next's internal 308 redirects go out bare (see `routing.rs`).
#[tokio::test]
async fn relayed_3xx_statuses_keep_the_rule_headers() {
    let mock = start_mock().await;
    let app = app();
    let res = app
        .post_json(
            RAG_EMBED,
            &json!({"serverUrl": mock.url("found"), "text": "x"}).to_string(),
        )
        .await;
    res.assert_json(302, r#"{"error":"moved"}"#);
    res.assert_headers(&NO_STORE);
    assert!(res.header("location").is_none() && res.header("refresh").is_none());
    let res = app
        .post_json(
            TTS_SYNTH,
            &json!({"serverUrl": mock.url("multi"), "text": "x"}).to_string(),
        )
        .await;
    res.assert_json(
        300,
        r#"{"error":"Synthesis failed (300): {\"error\":\"multi\"}"}"#,
    );
    res.assert_headers(&NO_STORE);
}

// ----- body limits -----------------------------------------------------------------------

#[tokio::test]
async fn body_limits() {
    let mock = start_mock().await;
    let app = app();
    // JSON routes accept more than axum's 2 MiB default (up to 16 MiB)...
    let text = "a".repeat(3 * 1024 * 1024);
    let body = json!({"serverUrl": mock.url("good"), "text": text}).to_string();
    assert_eq!(app.post_json(RAG_EMBED, &body).await.status.as_u16(), 200);
    // ...but not more.
    let res = app
        .post_json(TTS_HEALTH, &"x".repeat(17 * 1024 * 1024))
        .await;
    assert_eq!(res.status.as_u16(), 413);
    // stt/transcribe accepts long recordings (≥ 32 MiB limit).
    let audio = vec![7u8; 20 * 1024 * 1024];
    let body = multipart(&[
        FormPart::File("audio", "a", Some("audio/wav"), &audio),
        FormPart::Text("serverUrl", &mock.url("good")),
    ]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(200, r#"{"text":"こんにちは","provider_used":"mock"}"#);
    assert_eq!(
        parse_form(
            mock.last("/good/transcribe").header("content-type"),
            &mock.last("/good/transcribe").body
        )
        .ok()
        .and_then(|f| match f.get("audio") {
            Some(FormValue::File(file)) => Some(file.data.len()),
            _ => None,
        }),
        Some(audio.len())
    );
    let res = transcribe(&app, &multipart_type(), vec![0u8; 33 * 1024 * 1024]).await;
    assert_eq!(res.status.as_u16(), 413);
}

// ----- AMADEUS_PROXY_ALLOWLIST (V2) ------------------------------------------------------

#[tokio::test]
async fn proxy_allowlist() {
    let mock = start_mock().await;
    let port = mock.port().to_owned();
    let allow = vec![
        format!("http://127.0.0.1:{port}/good"),
        format!("http://127.0.0.1:{port}/redirect"),
    ];
    let app = app_with(|c| c.proxy_allowlist = Some(allow));
    // Allowed prefixes work as before.
    app.post_json(TTS_HEALTH, &server_url(&mock.url("good")))
        .await
        .assert_json(200, GOOD_SPREAD);
    let body = json!({"serverUrl": mock.url("good"), "text": "x"}).to_string();
    app.post_json(RAG_EMBED, &body)
        .await
        .assert_json(200, r#"{"embedding":[0.25,-0.5,1],"dim":3}"#);
    // Everything else is refused before any request is sent.
    let before = mock.requests().len();
    app.post_json(TTS_HEALTH, &server_url(&mock.url("bad")))
        .await
        .assert_json(
            403,
            r#"{"available":false,"error":"serverUrl not allowed"}"#,
        );
    app.post_json(
        TTS_SYNTH,
        &json!({"serverUrl": "http://169.254.169.254/latest", "text": "x"}).to_string(),
    )
    .await
    .assert_json(403, r#"{"error":"serverUrl not allowed"}"#);
    let body = multipart(&[
        FormPart::File("audio", "a", Some("audio/ogg"), b"x"),
        FormPart::Text("serverUrl", &mock.url("bad")),
    ]);
    transcribe(&app, &multipart_type(), body)
        .await
        .assert_json(403, r#"{"error":"serverUrl not allowed"}"#);
    app.post_json(
        RAG_EMBED,
        &json!({"serverUrl": format!("http://localhost:{port}/good"), "text": "x"}).to_string(),
    )
    .await
    .assert_json(403, r#"{"error":"serverUrl not allowed"}"#);
    assert_eq!(
        mock.requests().len(),
        before,
        "refused targets must not be contacted"
    );
    // V1 validation still comes first.
    app.post_json(TTS_HEALTH, "{}")
        .await
        .assert_json(400, MISSING_URL_HEALTH);
    // A redirect inside the allowlist is followed (/redirect → /good/health)...
    app.post_json(TTS_HEALTH, &server_url(&mock.url("redirect")))
        .await
        .assert_json(200, GOOD_SPREAD);
    // ...but every hop is re-checked: with only /redirect allowed, the hop to /good is refused.
    let strict =
        app_with(|c| c.proxy_allowlist = Some(vec![format!("http://127.0.0.1:{port}/redirect")]));
    let before = mock.requests().len();
    strict
        .post_json(TTS_HEALTH, &server_url(&mock.url("redirect")))
        .await
        .assert_json(200, FETCH_FAILED_HEALTH);
    let after: Vec<String> = mock.requests()[before..]
        .iter()
        .map(|r| r.path.clone())
        .collect();
    assert_eq!(
        after,
        ["/redirect/health"],
        "the refused hop must not be requested"
    );
}
