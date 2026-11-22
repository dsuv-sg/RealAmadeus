//! FreeAPI (provider 8, `vertex-helpers.ts` §8.11-§8.13): Google AI Studio models first, then
//! the server's Vertex project through gcloud. The AI Studio key comes from
//! [`crate::BackendConfig::ai_studio_api_key`]; unlike V1 there is no hard-coded fallback key,
//! so without a key the Vertex fallback is used directly.

use amadeus_core::jscompat::js_trim;
use amadeus_core::protocol::StreamEvent;
use serde_json::Value;

use super::bodies::build_gemini_body;
use super::extract::{extract_gemini_response, log_abnormal_gemini_finish_reason};
use super::net::{self, content_json, StatusLine};
use super::payload::{Payload, Thrown};
use super::sink::{EventSink, StreamErr};
use super::sse::{data_payload, LineReader};
use super::texts::{FREE_FALLBACK_COMM_ERROR, PARSE_ERROR};
use super::{console_error, console_log, console_warn, state, vertex};
use crate::{Backend, BackendConfig};

/// `FREE_API_STUDIO_MODELS` (tried in order, one attempt each, no sleep).
pub const FREE_API_STUDIO_MODELS: [&str; 2] = ["gemini-3.5-flash-lite", "gemini-3.1-flash-lite"];
/// `FREE_API_GCP_MODEL`.
pub const FREE_API_GCP_MODEL: &str = "gemini-3.1-flash-lite";
/// Vertex project of the FreeAPI fallback.
pub const FREE_API_GCP_PROJECT: &str = "realamadeusfreecredit";
/// Vertex location of the FreeAPI fallback.
pub const FREE_API_GCP_LOCATION: &str = "global";

/// `getAiStudioApiKey()` minus `DEFAULT_AI_STUDIO_API_KEY`: `"undefined"` and blank keys are
/// treated as absent, the key is trimmed.
pub(crate) fn ai_studio_key(config: &BackendConfig) -> Option<String> {
    config
        .ai_studio_api_key
        .as_deref()
        .filter(|k| *k != "undefined")
        .map(js_trim)
        .filter(|k| !k.is_empty())
        .map(str::to_owned)
}

/// The payload mutation V1 performed before falling back to Vertex.
fn use_vertex_fallback(payload: &mut Payload) {
    payload.set_str("model", FREE_API_GCP_MODEL);
    payload.set_str("vertexProject", FREE_API_GCP_PROJECT);
    payload.set_str("vertexLocation", FREE_API_GCP_LOCATION);
}

/// `fetchFreeApiResponse(payload)`: `Ok(Err((error, status)))` is V1's `{ error, status }`.
pub(crate) async fn fetch_free_api_response(
    backend: &Backend,
    payload: &mut Payload,
) -> Result<Result<String, (String, u16)>, Thrown> {
    let web_search = payload.truthy("webSearch");
    if let Some(key) = ai_studio_key(backend.config()) {
        for model in FREE_API_STUDIO_MODELS {
            let url = state(backend).endpoints.gemini(model, false);
            let headers = [("x-goog-api-key", key.clone()), content_json()];
            let body = net::json_body(&build_gemini_body(payload.messages(), web_search));
            match net::post(backend.http(), &url, &headers, body).await {
                Err(Thrown(msg)) => {
                    console_warn(&format!("[FreeAPI] Google AI Studio ({model}) fetch error: {msg}. Trying next fallback..."));
                }
                Ok(resp) if resp.status().is_success() => {
                    let data = net::read_json_or_null(resp).await;
                    let text = extract_gemini_response(data.as_ref());
                    log_abnormal_gemini_finish_reason(
                        &format!("FreeAPI AI Studio ({model})"),
                        data.as_ref(),
                        text.chars().count(),
                    );
                    if !text.is_empty() && text != PARSE_ERROR {
                        return Ok(Ok(text));
                    }
                    console_warn(&format!(
                        "[FreeAPI] Google AI Studio ({model}) response parsing failed: {}",
                        net::json_stringify(data.as_ref())
                    ));
                }
                Ok(resp) => {
                    let line = StatusLine::of(&resp);
                    let error_text = net::read_text_or_empty(resp).await;
                    console_warn(&format!(
                        "[FreeAPI] Google AI Studio ({model}) returned {} ({}): {error_text}. Trying next fallback...",
                        line.code, line.text
                    ));
                }
            }
        }
    }
    console_log("[FreeAPI] Falling back to GCP Vertex AI (gemini-3.1-flash-lite)...");
    use_vertex_fallback(payload);
    vertex::fetch_vertex_response(backend, payload).await
}

/// Token of one AI Studio SSE data payload, if any.
fn studio_token(raw: &str, model: &str, accumulated_chars: usize) -> Option<String> {
    let parsed: Value = serde_json::from_str(raw).ok()?;
    let token = extract_gemini_response(Some(&parsed));
    let valid_len = if token != PARSE_ERROR {
        token.chars().count()
    } else {
        0
    };
    log_abnormal_gemini_finish_reason(
        &format!("FreeAPI AI Studio stream ({model})"),
        Some(&parsed),
        accumulated_chars + valid_len,
    );
    (!token.is_empty() && token != PARSE_ERROR).then_some(token)
}

/// `createFreeApiStream(payload)`. Partial AI Studio answers that break mid-stream end without a
/// `complete` event (V1 quirk Q9).
pub(crate) async fn create_free_api_stream(
    backend: &Backend,
    payload: &mut Payload,
    sink: &mut EventSink,
) -> Result<(), StreamErr> {
    let web_search = payload.truthy("webSearch");
    let mut got_tokens = false;
    if let Some(key) = ai_studio_key(backend.config()) {
        for model in FREE_API_STUDIO_MODELS {
            if got_tokens {
                break;
            }
            let url = state(backend).endpoints.gemini(model, true);
            let headers = [("x-goog-api-key", key.clone()), content_json()];
            let body = net::json_body(&build_gemini_body(payload.messages(), web_search));
            let mut resp = match net::post(backend.http(), &url, &headers, body).await {
                Ok(resp) => resp,
                Err(Thrown(msg)) => {
                    console_warn(&format!("[FreeAPI] Google AI Studio ({model}) stream exception: {msg}. Trying next fallback..."));
                    continue;
                }
            };
            let status = resp.status();
            if !(status.is_success() && net::has_body(status)) {
                let error_text = net::read_text_or_empty(resp).await;
                console_warn(&format!(
                    "[FreeAPI] Google AI Studio ({model}) stream error ({}): {error_text}. Trying next fallback...",
                    status.as_u16()
                ));
                continue;
            }
            let mut full = String::new();
            let mut reader = LineReader::new();
            let interrupted = loop {
                match net::next_chunk(&mut resp).await {
                    Ok(Some(chunk)) => {
                        for line in reader.push(&chunk) {
                            let Some(raw) = data_payload(&line) else {
                                continue;
                            };
                            if raw == "[DONE]" {
                                sink.emit(StreamEvent::complete(full)).await?;
                                return Ok(());
                            }
                            if let Some(token) = studio_token(raw, model, full.chars().count()) {
                                got_tokens = true;
                                full.push_str(&token);
                                sink.emit(StreamEvent::token(token)).await?;
                            }
                        }
                    }
                    Ok(None) => {
                        let left = reader.leftover();
                        if let Some(raw) =
                            data_payload(js_trim(&left)).filter(|raw| *raw != "[DONE]")
                        {
                            if let Some(token) = studio_token(raw, model, full.chars().count()) {
                                got_tokens = true;
                                full.push_str(&token);
                                sink.emit(StreamEvent::token(token)).await?;
                            }
                        }
                        break None;
                    }
                    Err(Thrown(msg)) => break Some(msg),
                }
            };
            if let Some(msg) = interrupted {
                console_warn(&format!("[FreeAPI] Google AI Studio ({model}) stream exception: {msg}. Trying next fallback..."));
                continue;
            }
            if got_tokens {
                sink.emit(StreamEvent::complete(full)).await?;
                return Ok(());
            }
            // A 200 without tokens falls through to the next model, silently.
        }
    }
    if !got_tokens {
        console_log("[FreeAPI] Using GCP Vertex AI stream fallback (gemini-3.1-flash-lite)...");
        use_vertex_fallback(payload);
        match vertex::create_vertex_stream(backend, payload, sink).await {
            Err(StreamErr::Thrown(msg)) => {
                console_error(&format!(
                    "[FreeAPI] GCP Vertex AI fallback stream error: {msg}"
                ));
                sink.emit(StreamEvent::token(FREE_FALLBACK_COMM_ERROR))
                    .await?;
                sink.emit(StreamEvent::complete(FREE_FALLBACK_COMM_ERROR))
                    .await?;
            }
            other => other?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_studio_key_rules() {
        let mut config = BackendConfig::server_default();
        assert_eq!(ai_studio_key(&config), None);
        config.ai_studio_api_key = Some("  key  ".into());
        assert_eq!(ai_studio_key(&config).as_deref(), Some("key"));
        config.ai_studio_api_key = Some("undefined".into());
        assert_eq!(ai_studio_key(&config), None);
        config.ai_studio_api_key = Some(" \u{3000} ".into());
        assert_eq!(ai_studio_key(&config), None);
    }
}
