//! Web transport: `fetch` against the V2 server, which keeps V1's HTTP API contract
//! (`{base}/app/api/...`, JSON payloads in V1 key order, SSE frames `data: {json}\n\n`).

use amadeus_core::protocol::{
    paths, ChatRequestPayload, RagEmbedBody, ServerUrlBody, SttTranscribeFields, TtsSynthesizeBody,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AbortSignal, FormData, Headers, ReadableStreamDefaultReader, RequestCache, RequestInit,
    Response, TextDecodeOptions, TextDecoder,
};

use super::js::{failed, json_text, local_failure};
use super::sse::SseParser;
use super::{
    JsonReply, RequestCancel, StreamOutcome, StreamSink, TransportError, TtsReply,
    RECORDING_FILE_NAME,
};

/// `fetch`-based transport (stateless).
#[derive(Debug, Default, Clone, Copy)]
pub struct HttpTransport;

/// `window.fetch(url, init)` resolved to a `Response`.
async fn fetch(url: &str, init: &RequestInit) -> Result<Response, JsValue> {
    let window = web_sys::window()
        .ok_or_else(|| JsValue::from(js_sys::Error::new("window is not available")))?;
    let value = JsFuture::from(window.fetch_with_str_and_init(url, init)).await?;
    value.dyn_into::<Response>()
}

/// V1 `fetch(url, { method: "POST", headers: { "Content-Type": "application/json" }, body, signal })`.
async fn post_json(
    path: &str,
    body: &str,
    signal: Option<&AbortSignal>,
) -> Result<Response, JsValue> {
    let init = RequestInit::new();
    init.set_method("POST");
    let headers = Headers::new()?;
    headers.set("Content-Type", "application/json")?;
    init.set_headers(&headers);
    init.set_body(&JsValue::from_str(body));
    if let Some(signal) = signal {
        init.set_signal(Some(signal));
    }
    fetch(&crate::env::api_url(path), &init).await
}

/// V1 `await response.json().catch(() => null)` with the status.
async fn read_json_reply(response: &Response) -> JsonReply {
    let status = response.status();
    let text = match response.text() {
        Ok(promise) => JsFuture::from(promise)
            .await
            .ok()
            .and_then(|v| v.as_string()),
        Err(_) => None,
    };
    JsonReply::new(status, text.and_then(|t| serde_json::from_str(&t).ok()))
}

/// A rejected promise: [`TransportError::Aborted`] when the request was cancelled (V1 checks
/// `signal.aborted`), otherwise a failure with the JS error message.
fn classify(error: &JsValue, cancel: Option<&RequestCancel>) -> TransportError {
    if cancel.is_some_and(RequestCancel::is_cancelled) {
        TransportError::Aborted
    } else {
        failed(error)
    }
}

fn check(cancel: Option<&RequestCancel>) -> Result<(), TransportError> {
    cancel.map_or(Ok(()), RequestCancel::check)
}

impl HttpTransport {
    /// `POST app/api/chat`.
    pub async fn chat(
        &self,
        payload: &ChatRequestPayload,
        cancel: Option<&RequestCancel>,
    ) -> Result<JsonReply, TransportError> {
        let body = json_text(payload)?;
        let signal = cancel.and_then(RequestCancel::signal);
        let response = post_json(paths::API_CHAT, &body, signal.as_ref())
            .await
            .map_err(|e| classify(&e, cancel))?;
        let reply = read_json_reply(&response).await;
        check(cancel)?;
        Ok(reply)
    }

    /// `POST app/api/chat/stream`, read with `response.body.getReader()` and a streaming
    /// `TextDecoder` (UTF-8, invalid bytes → U+FFFD, never flushed), parsed by [`SseParser`].
    pub async fn chat_stream(
        &self,
        payload: &ChatRequestPayload,
        cancel: &RequestCancel,
        sink: StreamSink,
    ) -> Result<StreamOutcome, TransportError> {
        let body = json_text(payload)?;
        let signal = cancel.signal();
        let response = post_json(paths::API_CHAT_STREAM, &body, signal.as_ref())
            .await
            .map_err(|e| classify(&e, Some(cancel)))?;
        let stream = match response.body() {
            Some(stream) if response.ok() => stream,
            _ => {
                let reply = read_json_reply(&response).await;
                cancel.check()?;
                return Ok(StreamOutcome::Rejected(reply));
            }
        };
        let reader: ReadableStreamDefaultReader = stream.get_reader().unchecked_into();
        let decoder = TextDecoder::new().map_err(|e| failed(&e))?;
        let options = TextDecodeOptions::new();
        options.set_stream(true);
        let mut parser = SseParser::new();
        loop {
            let chunk = JsFuture::from(reader.read())
                .await
                .map_err(|e| classify(&e, Some(cancel)))?;
            cancel.check()?;
            let done = js_sys::Reflect::get(&chunk, &JsValue::from_str("done"))
                .map(|d| d.is_truthy())
                .unwrap_or(true);
            if done {
                break;
            }
            let value = js_sys::Reflect::get(&chunk, &JsValue::from_str("value"))
                .map_err(|e| failed(&e))?;
            let bytes: js_sys::Uint8Array = value
                .dyn_into()
                .map_err(|_| local_failure("The stream chunk is not a Uint8Array"))?;
            let text = decoder
                .decode_with_js_u8_array_and_options(&bytes, &options)
                .map_err(|e| failed(&e))?;
            for event in parser.push(&text) {
                cancel.check()?;
                let terminal = event.is_terminal();
                sink(event);
                if terminal {
                    // V1 returns without cancelling the reader; the server closes the stream.
                    return Ok(StreamOutcome::Finished);
                }
            }
        }
        if let Some(event) = parser.finish() {
            cancel.check()?;
            sink(event);
        }
        Ok(StreamOutcome::Finished)
    }

    /// `GET app/api/chat/quota` with `cache: "no-store"`.
    pub async fn chat_quota(&self) -> Result<JsonReply, TransportError> {
        let init = RequestInit::new();
        init.set_cache(RequestCache::NoStore);
        let response = fetch(&crate::env::api_url(paths::API_CHAT_QUOTA), &init)
            .await
            .map_err(|e| failed(&e))?;
        Ok(read_json_reply(&response).await)
    }

    /// `POST app/api/tts/synthesize`; an OK response is read with `response.blob()`.
    pub async fn tts_synthesize(
        &self,
        body: &TtsSynthesizeBody,
    ) -> Result<TtsReply, TransportError> {
        let response = post_json(paths::API_TTS_SYNTHESIZE, &json_text(body)?, None)
            .await
            .map_err(|e| failed(&e))?;
        if !response.ok() {
            return Ok(TtsReply::Failed(read_json_reply(&response).await));
        }
        let promise = response.blob().map_err(|e| failed(&e))?;
        let blob = JsFuture::from(promise).await.map_err(|e| failed(&e))?;
        let blob = blob
            .dyn_into::<web_sys::Blob>()
            .map_err(|_| local_failure("The response body is not a Blob"))?;
        Ok(TtsReply::Audio(blob))
    }

    /// The three `…/health` POSTs (`{"serverUrl"}`).
    pub async fn health(&self, path: &str, server_url: &str) -> Result<JsonReply, TransportError> {
        let body = json_text(&ServerUrlBody {
            server_url: server_url.to_owned(),
        })?;
        let response = post_json(path, &body, None).await.map_err(|e| failed(&e))?;
        Ok(read_json_reply(&response).await)
    }

    /// `POST app/api/stt/transcribe` as `multipart/form-data` (the UA sets the boundary):
    /// `audio` (filename `recording.webm`), `provider`, `language`, `serverUrl`, in this order.
    pub async fn stt_transcribe(
        &self,
        audio: &web_sys::Blob,
        fields: &SttTranscribeFields,
    ) -> Result<JsonReply, TransportError> {
        let form = FormData::new().map_err(|e| failed(&e))?;
        form.append_with_blob_and_filename("audio", audio, RECORDING_FILE_NAME)
            .map_err(|e| failed(&e))?;
        form.append_with_str("provider", &fields.provider)
            .map_err(|e| failed(&e))?;
        form.append_with_str("language", &fields.language)
            .map_err(|e| failed(&e))?;
        form.append_with_str("serverUrl", &fields.server_url)
            .map_err(|e| failed(&e))?;
        let init = RequestInit::new();
        init.set_method("POST");
        init.set_body(&form);
        let response = fetch(&crate::env::api_url(paths::API_STT_TRANSCRIBE), &init)
            .await
            .map_err(|e| failed(&e))?;
        Ok(read_json_reply(&response).await)
    }

    /// `POST app/api/rag/embed`.
    pub async fn rag_embed(&self, body: &RagEmbedBody) -> Result<JsonReply, TransportError> {
        let response = post_json(paths::API_RAG_EMBED, &json_text(body)?, None)
            .await
            .map_err(|e| failed(&e))?;
        Ok(read_json_reply(&response).await)
    }
}
