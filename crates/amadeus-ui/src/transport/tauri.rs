//! Desktop transport: Tauri 2 IPC through `window.__TAURI__.core` (`app.withGlobalTauri`).
//!
//! Command names and argument shapes are fixed by `amadeus_core::protocol::ipc` and
//! `docs/DESKTOP_IPC.md`. Backend commands never reject: every outcome is an [`ApiReply`],
//! mapped to the same [`JsonReply`] the web build gets from HTTP.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use amadeus_core::protocol::ipc::{self, AppInfo};
use amadeus_core::protocol::{
    b64, ApiReply, ChatRequestPayload, RagEmbedBody, ServerUrlBody, SttTranscribeFields,
    TtsSynthesizeBody,
};
use serde::Serialize;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use super::js::{bytes_to_blob, failed, invoke_failed, local_failure, stringify, to_js_object};
use super::sse::SseParser;
use super::{JsonReply, RequestCancel, StreamOutcome, StreamSink, TransportError, TtsReply};

/// How long to wait for a terminal event after `chat_stream` resolved without one. Channel
/// messages of 8 KiB or more need an extra IPC round trip and can arrive after the promise
/// (DESKTOP_IPC §2); a stream that really ended without `complete`/`error` (V1 quirks) then
/// finishes like V1's end of body, with `Complete(<accumulated tokens>)`.
pub const TERMINAL_GRACE_MS: u32 = 1_000;

/// Tauri IPC transport. Stream ids increase per page load (DESKTOP_IPC §2).
#[derive(Debug, Default)]
pub struct TauriTransport {
    next_stream_id: Cell<u32>,
}

/// `window.__TAURI__.core`.
fn tauri_core() -> Result<JsValue, TransportError> {
    let window = web_sys::window().ok_or_else(|| local_failure("window is not available"))?;
    let tauri =
        js_sys::Reflect::get(&window, &JsValue::from_str("__TAURI__")).map_err(|e| failed(&e))?;
    let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core")).map_err(|e| failed(&e))?;
    if core.is_undefined() || core.is_null() {
        return Err(local_failure("window.__TAURI__.core is not available"));
    }
    Ok(core)
}

/// `await window.__TAURI__.core.invoke(command, args)`.
async fn invoke(command: &str, args: &JsValue) -> Result<JsValue, TransportError> {
    let core = tauri_core()?;
    let invoke = js_sys::Reflect::get(&core, &JsValue::from_str("invoke"))
        .map_err(|e| failed(&e))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| local_failure("window.__TAURI__.core.invoke is not a function"))?;
    let promise = invoke
        .call2(&core, &JsValue::from_str(command), args)
        .map_err(|e| invoke_failed(&e))?;
    let promise = promise
        .dyn_into::<js_sys::Promise>()
        .map_err(|_| local_failure("invoke did not return a Promise"))?;
    JsFuture::from(promise).await.map_err(|e| invoke_failed(&e))
}

/// Decodes an [`ApiReply`] (`{"kind":"json"|"binary"|"empty",…}`) with JSON semantics.
fn decode_api_reply(value: &JsValue) -> Result<ApiReply, TransportError> {
    let text = stringify(value).ok_or_else(|| local_failure("Invalid desktop reply"))?;
    serde_json::from_str(&text).map_err(|e| local_failure(format!("Invalid desktop reply: {e}")))
}

async fn invoke_reply(command: &str, args: &JsValue) -> Result<ApiReply, TransportError> {
    let value = invoke(command, args).await?;
    decode_api_reply(&value)
}

/// `{ body }` arguments (`tts_synthesize`, the health commands, `rag_embed`).
#[derive(Serialize)]
struct BodyArgs<'a, T: Serialize> {
    body: &'a T,
}

/// `{ payload }` / `{ payload, streamId }` arguments of `chat` / `chat_stream`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatArgs<'a> {
    payload: &'a ChatRequestPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_id: Option<u32>,
}

/// `{ streamId }` of `chat_stream_cancel`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StreamIdArgs {
    stream_id: u32,
}

/// `{ fields, audioBase64, mime }` of `stt_transcribe`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SttArgs<'a> {
    fields: &'a SttTranscribeFields,
    audio_base64: String,
    mime: String,
}

fn body_args<T: Serialize>(body: &T) -> Result<JsValue, TransportError> {
    to_js_object(&BodyArgs { body })
}

// ---------------------------------------------------------------------------------------------
// Streaming
// ---------------------------------------------------------------------------------------------

/// One running `chat_stream`.
struct TauriStream {
    parser: RefCell<SseParser>,
    sink: StreamSink,
    cancel: RequestCancel,
    terminal: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl TauriStream {
    /// A Channel message (one V1 SSE event object), parsed exactly like an HTTP frame.
    fn deliver(&self, message: &JsValue) {
        if self.cancel.is_cancelled() || self.terminal.get() {
            return; // V1 parity: no events after cancel or after the terminal event.
        }
        let Some(json) = message.as_string().or_else(|| stringify(message)) else {
            return;
        };
        let events = self.parser.borrow_mut().push(&format!("data: {json}\n\n"));
        for event in events {
            if self.cancel.is_cancelled() {
                return;
            }
            let terminal = event.is_terminal();
            if terminal {
                self.terminal.set(true);
            }
            (self.sink)(event);
            if terminal {
                self.wake();
                return;
            }
        }
    }

    fn settled(&self) -> bool {
        self.terminal.get() || self.cancel.is_cancelled()
    }

    fn wake(&self) {
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// Resolves once the stream saw its terminal event or was cancelled.
struct Settled(Rc<TauriStream>);

impl Future for Settled {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0.settled() {
            Poll::Ready(())
        } else {
            *self.0.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

thread_local! {
    static STREAMS: RefCell<HashMap<u32, Rc<TauriStream>>> = RefCell::new(HashMap::new());
    /// One page-lifetime dispatcher; each Channel gets `dispatcher.bind(null, streamId)`, so no
    /// per-stream closure can be called after it was dropped (late messages are ignored).
    static DISPATCHER: Closure<dyn Fn(JsValue, JsValue)> = Closure::new(|id: JsValue, message: JsValue| {
        let Some(id) = id.as_f64() else { return };
        let stream = STREAMS.with(|streams| streams.borrow().get(&(id as u32)).cloned());
        if let Some(stream) = stream {
            stream.deliver(&message);
        }
    });
}

/// Removes the stream from the dispatcher table when the request future ends.
struct Registration(u32);

impl Drop for Registration {
    fn drop(&mut self) {
        STREAMS.with(|streams| streams.borrow_mut().remove(&self.0));
    }
}

/// `new window.__TAURI__.core.Channel()` with `onmessage` routed to the dispatcher.
fn new_channel(stream_id: u32) -> Result<JsValue, TransportError> {
    let core = tauri_core()?;
    let ctor = js_sys::Reflect::get(&core, &JsValue::from_str("Channel"))
        .map_err(|e| failed(&e))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| local_failure("window.__TAURI__.core.Channel is not available"))?;
    let channel =
        js_sys::Reflect::construct(&ctor, &js_sys::Array::new()).map_err(|e| failed(&e))?;
    let on_message = DISPATCHER.with(|dispatcher| {
        dispatcher
            .as_ref()
            .unchecked_ref::<js_sys::Function>()
            .bind1(&JsValue::NULL, &JsValue::from(stream_id))
    });
    js_sys::Reflect::set(&channel, &JsValue::from_str("onmessage"), &on_message)
        .map_err(|e| failed(&e))?;
    Ok(channel)
}

impl TauriTransport {
    /// Transport with stream ids starting at 1.
    pub fn new() -> Self {
        Self::default()
    }

    fn next_stream_id(&self) -> u32 {
        let id = self.next_stream_id.get().wrapping_add(1).max(1);
        self.next_stream_id.set(id);
        id
    }

    /// `chat { payload }`. The invoke cannot be aborted; after a cancel its reply is dropped.
    pub async fn chat(
        &self,
        payload: &ChatRequestPayload,
        cancel: Option<&RequestCancel>,
    ) -> Result<JsonReply, TransportError> {
        let args = to_js_object(&ChatArgs {
            payload,
            stream_id: None,
        })?;
        let reply = invoke_reply(ipc::CHAT, &args).await;
        if let Some(cancel) = cancel {
            cancel.check()?;
        }
        Ok(JsonReply::from_api_reply(&reply?))
    }

    /// `chat_stream { payload, streamId, onEvent: Channel<StreamEvent> }`. Finishes on the
    /// terminal event (not on the promise); `chat_stream_cancel { streamId }` on cancel.
    pub async fn chat_stream(
        &self,
        payload: &ChatRequestPayload,
        cancel: &RequestCancel,
        sink: StreamSink,
    ) -> Result<StreamOutcome, TransportError> {
        cancel.check()?;
        let stream_id = self.next_stream_id();
        let stream = Rc::new(TauriStream {
            parser: RefCell::new(SseParser::new()),
            sink,
            cancel: cancel.clone(),
            terminal: Cell::new(false),
            waker: RefCell::new(None),
        });
        STREAMS.with(|streams| streams.borrow_mut().insert(stream_id, Rc::clone(&stream)));
        let _registration = Registration(stream_id);

        let channel = new_channel(stream_id)?;
        let args = to_js_object(&ChatArgs {
            payload,
            stream_id: Some(stream_id),
        })?;
        js_sys::Reflect::set(&args, &JsValue::from_str("onEvent"), &channel)
            .map_err(|e| failed(&e))?;

        let weak = Rc::downgrade(&stream);
        cancel.on_cancel(move || {
            if let Some(stream) = weak.upgrade() {
                stream.wake();
            }
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(args) = to_js_object(&StreamIdArgs { stream_id }) {
                    if let Err(error) = invoke(ipc::CHAT_STREAM_CANCEL, &args).await {
                        log::warn!("chat_stream_cancel {stream_id} failed: {error}");
                    }
                }
            });
        });

        let result = invoke(ipc::CHAT_STREAM, &args).await;
        cancel.check()?;
        let value = result?;
        if !value.is_null() && !value.is_undefined() {
            let reply = decode_api_reply(&value)?;
            return Ok(StreamOutcome::Rejected(JsonReply::from_api_reply(&reply)));
        }
        if !stream.settled() {
            let settled = Settled(Rc::clone(&stream));
            let grace = gloo_timers::future::TimeoutFuture::new(TERMINAL_GRACE_MS);
            futures::future::select(settled, grace).await;
        }
        cancel.check()?;
        if !stream.terminal.replace(true) {
            let end = stream.parser.borrow_mut().finish();
            if let Some(event) = end {
                (stream.sink)(event);
            }
        }
        Ok(StreamOutcome::Finished)
    }

    /// `chat_quota {}`.
    pub async fn chat_quota(&self) -> Result<JsonReply, TransportError> {
        let reply = invoke_reply(ipc::CHAT_QUOTA, &js_sys::Object::new()).await?;
        Ok(JsonReply::from_api_reply(&reply))
    }

    /// `tts_synthesize { body }`: a 2xx `binary` reply becomes a `Blob` of its content type.
    pub async fn tts_synthesize(
        &self,
        body: &TtsSynthesizeBody,
    ) -> Result<TtsReply, TransportError> {
        let reply = invoke_reply(ipc::TTS_SYNTHESIZE, &body_args(body)?).await?;
        if !reply.is_ok() {
            return Ok(TtsReply::Failed(JsonReply::from_api_reply(&reply)));
        }
        // V1 reads every OK body with `response.blob()`.
        let blob = match &reply {
            ApiReply::Binary {
                content_type, data, ..
            } => bytes_to_blob(data, content_type)?,
            ApiReply::Json { body, .. } => {
                bytes_to_blob(body.to_string().as_bytes(), "application/json")?
            }
            ApiReply::Empty { .. } => bytes_to_blob(&[], "")?,
        };
        Ok(TtsReply::Audio(blob))
    }

    /// `tts_health` / `stt_health` / `rag_health` with `{ body: { serverUrl } }`.
    pub async fn health(
        &self,
        command: &str,
        server_url: &str,
    ) -> Result<JsonReply, TransportError> {
        let args = body_args(&ServerUrlBody {
            server_url: server_url.to_owned(),
        })?;
        let reply = invoke_reply(command, &args).await?;
        Ok(JsonReply::from_api_reply(&reply))
    }

    /// `stt_transcribe { fields, audioBase64, mime }`.
    pub async fn stt_transcribe(
        &self,
        audio: &web_sys::Blob,
        fields: &SttTranscribeFields,
    ) -> Result<JsonReply, TransportError> {
        let buffer = JsFuture::from(audio.array_buffer())
            .await
            .map_err(|e| failed(&e))?;
        let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
        let args = to_js_object(&SttArgs {
            fields,
            audio_base64: b64::encode(&bytes),
            mime: audio.type_(),
        })?;
        let reply = invoke_reply(ipc::STT_TRANSCRIBE, &args).await?;
        Ok(JsonReply::from_api_reply(&reply))
    }

    /// `rag_embed { body }`.
    pub async fn rag_embed(&self, body: &RagEmbedBody) -> Result<JsonReply, TransportError> {
        let reply = invoke_reply(ipc::RAG_EMBED, &body_args(body)?).await?;
        Ok(JsonReply::from_api_reply(&reply))
    }

    /// `app_info {}`.
    pub async fn app_info(&self) -> Option<AppInfo> {
        let value = invoke(ipc::APP_INFO, &js_sys::Object::new()).await.ok()?;
        serde_json::from_str(&stringify(&value)?).ok()
    }
}
