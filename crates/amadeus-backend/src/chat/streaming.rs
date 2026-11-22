//! `/chat/stream` provider handlers (`stream/route.ts` §7.4-§7.10).

use amadeus_core::jscompat::{js_trim, js_truthy};
use amadeus_core::protocol::StreamEvent;
use serde_json::Value;

use super::bodies::{
    build_gemini_body, build_groq_compound_body, build_ollama_body, build_openai_body,
    is_valid_groq_model,
};
use super::extract::{
    extract_gemini_response, extract_openai_stream_token, log_abnormal_gemini_finish_reason,
};
use super::net::{self, content_json, Header, StatusLine};
use super::payload::{non_nullish, prop, value_text, Payload};
use super::providers::{ollama_host, openai_url};
use super::sink::{EventSink, StreamErr};
use super::sse::{data_payload, LineReader};
use super::texts::{
    ERR_NO_API_KEY, ERR_UNSUPPORTED_OPENAI_LIKE, FREE_FALLBACK_COMM_ERROR, PARSE_ERROR,
};
use super::{free_api, state, vertex};
use crate::{Backend, ClientMeta};

/// Dispatch on `payload.provider` with strict equality (§7.2 step b).
pub(crate) async fn dispatch(
    backend: &Backend,
    payload: &mut Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let p = payload.provider();
    if p.is(3) {
        stream_groq(backend, payload, sink).await
    } else if p.is(1) {
        stream_gemini(backend, payload, sink).await
    } else if p.is(4) {
        stream_vertex(backend, payload, sink).await
    } else if p.is(8) {
        stream_free(backend, payload, sink).await
    } else if p.is(0) || p.is(5) || p.is(6) {
        stream_openai_like(backend, payload, sink).await
    } else {
        non_streaming_fallback(backend, payload, sink).await
    }
}

/// How tokens are pulled out of a parsed SSE data payload.
#[derive(Debug, Clone, Copy)]
enum TokenKind {
    /// `extractGeminiResponse`, skipping `[Parse Error]`.
    Gemini,
    /// `extractOpenAIStreamToken`.
    OpenAi,
}

fn token_of(kind: TokenKind, raw: &str, accumulated_chars: usize) -> Option<String> {
    let parsed: Value = serde_json::from_str(raw).ok()?;
    match kind {
        TokenKind::Gemini => {
            let token = extract_gemini_response(Some(&parsed));
            let valid_len = if token != PARSE_ERROR {
                token.chars().count()
            } else {
                0
            };
            log_abnormal_gemini_finish_reason(
                "Gemini stream",
                Some(&parsed),
                accumulated_chars + valid_len,
            );
            (!token.is_empty() && token != PARSE_ERROR).then_some(token)
        }
        TokenKind::OpenAi => {
            let token = extract_openai_stream_token(Some(&parsed));
            (!token.is_empty()).then_some(token)
        }
    }
}

/// E_S error event for a non-OK (or body-less) upstream response.
async fn emit_upstream_error(
    resp: reqwest::Response,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let line = StatusLine::of(&resp);
    let json = net::read_json_or_null(resp).await;
    sink.emit(StreamEvent::error(net::e_s(&line, json.as_ref())))
        .await?;
    Ok(())
}

fn usable(resp: &reqwest::Response) -> bool {
    resp.status().is_success() && net::has_body(resp.status())
}

/// Gemini / Groq relay: `[DONE]` completes immediately, a final unterminated `data:` line is
/// still processed (leftover variant L1).
async fn relay_sse(
    mut resp: reqwest::Response,
    kind: TokenKind,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let mut full = String::new();
    let mut reader = LineReader::new();
    while let Some(chunk) = net::next_chunk(&mut resp).await? {
        for line in reader.push(&chunk) {
            let Some(raw) = data_payload(&line) else {
                continue;
            };
            if raw == "[DONE]" {
                sink.emit(StreamEvent::complete(full)).await?;
                return Ok(());
            }
            if let Some(token) = token_of(kind, raw, full.chars().count()) {
                full.push_str(&token);
                sink.emit(StreamEvent::token(token)).await?;
            }
        }
    }
    let left = reader.leftover();
    if let Some(raw) = data_payload(js_trim(&left)).filter(|raw| *raw != "[DONE]") {
        if let Some(token) = token_of(kind, raw, full.chars().count()) {
            full.push_str(&token);
            sink.emit(StreamEvent::token(token)).await?;
        }
    }
    sink.emit(StreamEvent::complete(full)).await?;
    Ok(())
}

/// `streamGemini` (provider 1).
async fn stream_gemini(
    backend: &Backend,
    payload: &Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    if !payload.has_api_key() {
        sink.emit(StreamEvent::error(ERR_NO_API_KEY)).await?;
        return Ok(());
    }
    let model = payload.trim_or_empty("model", "payload.model?.trim")?;
    // NB: the streaming default differs from `/chat` (gemini-2.5-flash), V1 quirk Q1.
    let model = if model.is_empty() || model.to_lowercase().starts_with("gpt") {
        "gemini-2.0-flash".to_owned()
    } else {
        model
    };
    let url = state(backend).endpoints.gemini(&model, true);
    let headers = [("x-goog-api-key", payload.api_key_text()), content_json()];
    let body = build_gemini_body(payload.messages(), payload.truthy("webSearch"));
    let resp = net::post(backend.http(), &url, &headers, net::json_body(&body)).await?;
    if !usable(&resp) {
        return emit_upstream_error(resp, sink).await;
    }
    relay_sse(resp, TokenKind::Gemini, sink).await
}

/// `streamGroq` (provider 3).
async fn stream_groq(
    backend: &Backend,
    payload: &Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    if !payload.has_api_key() {
        sink.emit(StreamEvent::error(ERR_NO_API_KEY)).await?;
        return Ok(());
    }
    let model = payload.trim_or_empty("model", "payload.model?.trim")?;
    let model = if model.is_empty() || !is_valid_groq_model(&model, false) {
        "qwen3-32b".to_owned()
    } else {
        model
    };
    let body = if payload.truthy("webSearch") {
        build_groq_compound_body(payload.messages(), true)
    } else {
        build_openai_body(payload.messages(), &model, true, true)
    };
    let headers = [
        (
            "authorization",
            format!("Bearer {}", payload.api_key_text()),
        ),
        content_json(),
        ("accept", "text/event-stream".to_owned()),
    ];
    let url = state(backend).endpoints.groq.clone();
    let resp = net::post(backend.http(), &url, &headers, net::json_body(&body)).await?;
    if !usable(&resp) {
        return emit_upstream_error(resp, sink).await;
    }
    relay_sse(resp, TokenKind::OpenAi, sink).await
}

/// `streamOpenAILike` (OpenAI 0, Ollama 5, OpenRouter 6). A final line without LF is dropped
/// (leftover variant L3, V1 quirk Q7).
async fn stream_openai_like(
    backend: &Backend,
    payload: &Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let provider = payload.provider();
    let mut headers: Vec<Header> = vec![content_json(), ("accept", "text/event-stream".to_owned())];
    let model = payload.trim_or_empty("model", "payload.model?.trim")?;
    let ollama = provider.is(5);
    let (url, model) = if provider.is(6) {
        if !payload.has_api_key() {
            sink.emit(StreamEvent::error(ERR_NO_API_KEY)).await?;
            return Ok(());
        }
        // NB: differs from the `/chat` default (google/gemini-2.5-flash), V1 quirk Q1.
        let model = if model.is_empty() || model.to_lowercase().starts_with("gpt") {
            "openai/gpt-4o".to_owned()
        } else {
            model
        };
        headers.push((
            "authorization",
            format!("Bearer {}", payload.api_key_text()),
        ));
        headers.push(("http-referer", "https://dsuv.vercel.app".to_owned()));
        headers.push(("x-title", "RealAmadeus".to_owned()));
        (state(backend).endpoints.openrouter.clone(), model)
    } else if provider.is(0) {
        if !payload.has_api_key() {
            sink.emit(StreamEvent::error(ERR_NO_API_KEY)).await?;
            return Ok(());
        }
        let model = if model.is_empty() {
            "gpt-4o".to_owned()
        } else {
            model
        };
        let url = openai_url(backend, payload)?;
        headers.push((
            "authorization",
            format!("Bearer {}", payload.api_key_text()),
        ));
        (url, model)
    } else if ollama {
        let host = ollama_host(payload)?;
        let model = if model.is_empty() || model.to_lowercase().starts_with("gpt") {
            "mistral".to_owned()
        } else {
            model
        };
        (format!("{host}/api/chat"), model)
    } else {
        sink.emit(StreamEvent::error(ERR_UNSUPPORTED_OPENAI_LIKE))
            .await?;
        return Ok(());
    };
    let body = if ollama {
        build_ollama_body(&model, payload.messages(), true)
    } else {
        build_openai_body(payload.messages(), &model, true, false)
    };
    let mut resp = net::post(backend.http(), &url, &headers, net::json_body(&body)).await?;
    if !usable(&resp) {
        return emit_upstream_error(resp, sink).await;
    }

    let mut full = String::new();
    let mut reader = LineReader::new();
    while let Some(chunk) = net::next_chunk(&mut resp).await? {
        for line in reader.push(&chunk) {
            if ollama {
                // NDJSON: `parsed.message?.content` if truthy; the `done` flag is ignored.
                if line.is_empty() {
                    continue;
                }
                let Ok(parsed) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if let Some(content) =
                    prop(prop(Some(&parsed), "message"), "content").filter(|c| js_truthy(c))
                {
                    let token = value_text(content);
                    full.push_str(&token);
                    sink.emit(StreamEvent::token(token)).await?;
                }
                continue;
            }
            let Some(raw) = data_payload(&line) else {
                continue;
            };
            if raw == "[DONE]" {
                sink.emit(StreamEvent::complete(full)).await?;
                return Ok(());
            }
            if let Some(token) = token_of(TokenKind::OpenAi, raw, full.chars().count()) {
                full.push_str(&token);
                sink.emit(StreamEvent::token(token)).await?;
            }
        }
    }
    sink.emit(StreamEvent::complete(full)).await?;
    Ok(())
}

/// `streamVertex` (provider 4): exceptions become an `error` event.
async fn stream_vertex(
    backend: &Backend,
    payload: &Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let result = if backend.config().is_desktop() {
        vertex::desktop_vertex_stream(backend, payload, sink).await
    } else {
        vertex::create_vertex_stream(backend, payload, sink).await
    };
    match result {
        Err(StreamErr::Thrown(msg)) => {
            sink.emit(StreamEvent::error(msg)).await?;
            Ok(())
        }
        other => other,
    }
}

/// `streamFree` (provider 8): exceptions become the FreeAPI fallback reply.
async fn stream_free(
    backend: &Backend,
    payload: &mut Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    match free_api::create_free_api_stream(backend, payload, sink).await {
        Err(StreamErr::Thrown(_)) => {
            sink.emit(StreamEvent::token(FREE_FALLBACK_COMM_ERROR))
                .await?;
            sink.emit(StreamEvent::complete(FREE_FALLBACK_COMM_ERROR))
                .await?;
            Ok(())
        }
        other => other,
    }
}

/// `nonStreamingFallback` (Claude 2, local proxy 7, unknown/missing provider): re-enters the
/// complete `/chat` algorithm with a fake request that has no client IP (`"unknown"`), exactly
/// like V1 (second rate-limit check, second log line; quirk Q6 kept for parity).
///
/// V1 awaited the imported route handler directly, not through Next.js: an exception thrown
/// outside the handler's `try` block (a non-string `model`) rejects that call and reaches the
/// top-level catch, which emits its message as an `error` event.
async fn non_streaming_fallback(
    backend: &Backend,
    payload: &Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let fake_client = ClientMeta {
        ip: "unknown".to_owned(),
    };
    let reply = Box::pin(super::chat_route(backend, payload.clone(), &fake_client)).await?;
    let status = reply.status();
    let json = reply.json_body();
    if !(200..300).contains(&status) {
        let error = non_nullish(prop(json, "error"))
            .map_or_else(|| format!("Request failed ({status})"), value_text);
        sink.emit(StreamEvent::error(error)).await?;
        return Ok(());
    }
    let text = non_nullish(prop(json, "response")).map_or_else(String::new, value_text);
    sink.emit(StreamEvent::token(text.clone())).await?;
    sink.emit(StreamEvent::complete(text)).await?;
    Ok(())
}
