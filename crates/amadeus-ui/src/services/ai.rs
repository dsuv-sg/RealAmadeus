//! Port of V1 `lib/amadeus/services/ai-service.ts`: the LLM client. The payload is built from
//! the settings at call time (`amadeus_core::settings::chat_request_payload`, V1
//! `buildPayload`); `send_chat` is the non-streaming POST, `send_chat_streaming` the SSE
//! stream (desktop: IPC Channel) parsed with V1's client quirks.
//!
//! V1 signal semantics: every request ends with exactly one of `response_received`,
//! `stream_complete` or `error_occurred` — except a cancelled stream, which ends silently, and
//! no event is emitted after [`cancel_streaming`](AiService::cancel_streaming) returns.
//! `cancel_streaming` does **not** affect `send_chat` (V1 Q-C1); call
//! [`cancel_chat`](AiService::cancel_chat) as well to drop a pending non-streaming reply.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use amadeus_core::protocol::{ChatMessage, ChatRequestPayload};
use amadeus_core::settings::chat_request_payload;
use amadeus_core::types::Emitter;

use super::settings;
use crate::transport::{
    transport, RequestCancel, SseEvent, StreamOutcome, StreamSink, TransportError,
};

/// V1 `"Unknown error"` (non-`Error` exception in `sendChat`).
pub const UNKNOWN_ERROR: &str = "Unknown error";
/// V1 `"Unknown streaming error"`.
pub const UNKNOWN_STREAMING_ERROR: &str = "Unknown streaming error";

/// A pending request and its cancel handle.
struct Pending {
    id: u64,
    cancel: RequestCancel,
}

/// V1 `AIService`.
pub struct AiService {
    /// `responseReceived(response)` — non-streaming success (`data.response ?? ""`).
    pub response_received: Emitter<str>,
    /// `streamToken(token)`.
    pub stream_token: Emitter<str>,
    /// `streamComplete(fullResponse)`.
    pub stream_complete: Emitter<str>,
    /// `errorOccurred(message)` — mapped to the user text by the chat panel (§7.6 table).
    pub error_occurred: Emitter<str>,
    /// V1 `streamAbortController`.
    stream: RefCell<Option<Pending>>,
    chats: RefCell<Vec<Pending>>,
    next_id: Cell<u64>,
    weak_self: Weak<AiService>,
}

impl AiService {
    pub(crate) fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            response_received: Emitter::new(),
            stream_token: Emitter::new(),
            stream_complete: Emitter::new(),
            error_occurred: Emitter::new(),
            stream: RefCell::new(None),
            chats: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
            weak_self: weak.clone(),
        })
    }

    /// V1 `buildPayload(messages)` from the current settings.
    pub fn build_payload(&self, messages: Vec<ChatMessage>) -> ChatRequestPayload {
        chat_request_payload(settings(), messages)
    }

    fn next_id(&self) -> u64 {
        let id = self.next_id.get().wrapping_add(1);
        self.next_id.set(id);
        id
    }

    /// True while a stream started by [`send_chat_streaming`](Self::send_chat_streaming) runs.
    pub fn is_streaming(&self) -> bool {
        self.stream.borrow().is_some()
    }

    /// V1 `sendChat(messages)`: `POST app/api/chat`; emits `response_received(data.response ?? "")`
    /// or `error_occurred(data.error ?? "Request failed (<status>)")` / the network error message.
    pub fn send_chat(&self, messages: Vec<ChatMessage>) {
        let payload = self.build_payload(messages);
        let id = self.next_id();
        let cancel = RequestCancel::new();
        self.chats.borrow_mut().push(Pending {
            id,
            cancel: cancel.clone(),
        });
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = transport().chat(&payload, Some(&cancel)).await;
            let Some(this) = weak.upgrade() else { return };
            this.chats.borrow_mut().retain(|p| p.id != id);
            if cancel.is_cancelled() {
                return;
            }
            match result {
                Err(TransportError::Aborted) => {}
                Err(error) => this.error_occurred.emit(&error.message_or(UNKNOWN_ERROR)),
                Ok(reply) if !reply.ok => {
                    let message = reply.error_or(|| format!("Request failed ({})", reply.status));
                    this.error_occurred.emit(&message);
                }
                Ok(reply) => this
                    .response_received
                    .emit(&reply.field_or("response", String::new)),
            }
        });
    }

    /// Drops every pending [`send_chat`](Self::send_chat) reply (and aborts its `fetch` on the
    /// web). Not in V1, where a non-streaming reply always arrives (Q-C1); for the chat panel's
    /// cancel if it decides to fix that quirk.
    pub fn cancel_chat(&self) {
        let pending = std::mem::take(&mut *self.chats.borrow_mut());
        for request in pending {
            request.cancel.cancel();
        }
    }

    /// V1 `sendChatStreaming(messages)`: cancels the previous stream, then streams
    /// `app/api/chat/stream` (desktop: `chat_stream`). Emits `stream_token` per token, then
    /// `stream_complete(fullResponse ?? tokens)` or `error_occurred`; a non-OK response emits
    /// `error_occurred(data.error ?? "Request failed (<status>)")`.
    pub fn send_chat_streaming(&self, messages: Vec<ChatMessage>) {
        self.cancel_streaming();
        let payload = self.build_payload(messages);
        let id = self.next_id();
        let cancel = RequestCancel::new();
        *self.stream.borrow_mut() = Some(Pending {
            id,
            cancel: cancel.clone(),
        });

        let weak = self.weak_self.clone();
        let sink: StreamSink = Rc::new(move |event: SseEvent| {
            let Some(this) = weak.upgrade() else { return };
            match event {
                SseEvent::Token(token) => this.stream_token.emit(&token),
                SseEvent::Complete(full) => {
                    this.stream_complete.emit(&full);
                    this.clear_stream(id);
                }
                SseEvent::Error(error) => {
                    this.error_occurred.emit(&error);
                    this.clear_stream(id);
                }
            }
        });
        let weak = self.weak_self.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let result = transport().chat_stream(&payload, &cancel, sink).await;
            let Some(this) = weak.upgrade() else { return };
            if !cancel.is_cancelled() {
                match result {
                    Ok(StreamOutcome::Finished) | Err(TransportError::Aborted) => {}
                    Ok(StreamOutcome::Rejected(reply)) => {
                        let message =
                            reply.error_or(|| format!("Request failed ({})", reply.status));
                        this.error_occurred.emit(&message);
                    }
                    Err(error) => this
                        .error_occurred
                        .emit(&error.message_or(UNKNOWN_STREAMING_ERROR)),
                }
            }
            this.clear_stream(id); // V1 `finally`
        });
    }

    /// V1 `cancelStreaming()`: aborts the running stream (silently; nothing is emitted for it
    /// afterwards). No-op without a stream. Does not touch `send_chat`.
    pub fn cancel_streaming(&self) {
        let pending = self.stream.borrow_mut().take();
        if let Some(pending) = pending {
            pending.cancel.cancel();
        }
    }

    fn clear_stream(&self, id: u64) {
        let mut stream = self.stream.borrow_mut();
        if stream.as_ref().is_some_and(|p| p.id == id) {
            *stream = None;
        }
    }
}
