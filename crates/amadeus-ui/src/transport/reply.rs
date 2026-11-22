//! Transport-neutral results (pure; host-tested).
//!
//! Every backend call reports what V1 code saw after `fetch`: `response.status`,
//! `response.ok` and `await response.json().catch(() => null)`. The services keep V1's
//! `data?.error ?? \`… (${status})\`` logic on top of [`JsonReply`], so error texts are
//! byte-identical on the web (HTTP) and on the desktop (IPC `ApiReply`).

use amadeus_core::jscompat::{js_string, js_truthy};
use amadeus_core::protocol::ApiReply;
use serde_json::Value;

/// V1 `(response.status, response.ok, response.json().catch(() => null))`.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonReply {
    /// HTTP status (`ApiReply::status` on the desktop).
    pub status: u16,
    /// `response.ok`: status 200–299.
    pub ok: bool,
    /// Parsed JSON body. `None` when the body was empty, not JSON, or JSON `null`.
    pub body: Option<Value>,
}

impl JsonReply {
    /// Reply with `ok` derived from `status`. A JSON `null` body is stored as `None`.
    pub fn new(status: u16, body: Option<Value>) -> Self {
        Self {
            status,
            ok: (200..=299).contains(&status),
            body: body.filter(|b| !b.is_null()),
        }
    }

    /// Desktop mapping of an IPC [`ApiReply`]: `json` → `Some(body)`, `empty`/`binary` → `None`.
    pub fn from_api_reply(reply: &ApiReply) -> Self {
        Self::new(reply.status(), reply.json_body().cloned())
    }

    /// V1 `data?.<name>`: the field of an object body; `None` when the body is missing or not an
    /// object, or the field is absent or `null` (so `?? fallback` applies).
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.body
            .as_ref()?
            .as_object()?
            .get(name)
            .filter(|v| !v.is_null())
    }

    /// V1 `data?.<name> ?? fallback`. V1 forwards non-string values unchanged and every
    /// consumer treats them as text, so they are converted with JS `String(x)`.
    pub fn field_or(&self, name: &str, fallback: impl FnOnce() -> String) -> String {
        self.field(name).map(js_string).unwrap_or_else(fallback)
    }

    /// V1 `data?.error ?? fallback`.
    pub fn error_or(&self, fallback: impl FnOnce() -> String) -> String {
        self.field_or("error", fallback)
    }

    /// V1 `Boolean(data?.<name>)` (JS truthiness).
    pub fn truthy(&self, name: &str) -> bool {
        self.field(name).is_some_and(js_truthy)
    }

    /// V1 health checks: `response.ok && Boolean(data?.available)`.
    pub fn available(&self) -> bool {
        self.ok && self.truthy("available")
    }
}

/// Why a backend call produced no reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// Cancelled through a `RequestCancel` (V1: `AbortController.abort()`, reported silently).
    Aborted,
    /// `fetch`/IPC failure. `Some(message)` when V1 would have caught an `Error`
    /// (`error.message`, e.g. `"Failed to fetch"`); `None` for non-`Error` rejections, where V1
    /// falls back to the call site's text.
    Failed(Option<String>),
}

impl TransportError {
    /// V1 `error instanceof Error ? error.message : fallback`.
    pub fn message_or(&self, fallback: &str) -> String {
        match self {
            TransportError::Failed(Some(message)) => message.clone(),
            TransportError::Failed(None) | TransportError::Aborted => fallback.to_owned(),
        }
    }

    /// True for [`TransportError::Aborted`].
    pub fn is_aborted(&self) -> bool {
        matches!(self, TransportError::Aborted)
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Aborted => f.write_str("request aborted"),
            TransportError::Failed(Some(message)) => f.write_str(message),
            TransportError::Failed(None) => f.write_str("request failed"),
        }
    }
}

impl std::error::Error for TransportError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ok_range_and_null_body() {
        assert!(JsonReply::new(200, None).ok);
        assert!(JsonReply::new(299, None).ok);
        assert!(!JsonReply::new(199, None).ok);
        assert!(!JsonReply::new(300, None).ok);
        assert!(!JsonReply::new(500, None).ok);
        assert_eq!(JsonReply::new(200, Some(Value::Null)).body, None);
    }

    #[test]
    fn v1_error_fallbacks() {
        let fallback = |r: &JsonReply| r.error_or(|| format!("Request failed ({})", r.status));
        assert_eq!(fallback(&JsonReply::new(502, None)), "Request failed (502)");
        assert_eq!(
            fallback(&JsonReply::new(400, Some(json!({"error": "bad"})))),
            "bad"
        );
        assert_eq!(
            fallback(&JsonReply::new(400, Some(json!({"error": null})))),
            "Request failed (400)"
        );
        assert_eq!(
            fallback(&JsonReply::new(400, Some(json!({"error": ""})))),
            "",
            "?? keeps empty strings"
        );
        assert_eq!(
            fallback(&JsonReply::new(400, Some(json!({"error": 7})))),
            "7"
        );
        assert_eq!(
            fallback(&JsonReply::new(400, Some(json!("text")))),
            "Request failed (400)"
        );
        assert_eq!(
            fallback(&JsonReply::new(400, Some(json!([1])))),
            "Request failed (400)"
        );
    }

    #[test]
    fn response_and_text_fields() {
        let reply = JsonReply::new(200, Some(json!({"response": "やあ", "text": "", "n": 1.5})));
        assert_eq!(reply.field_or("response", String::new), "やあ");
        assert_eq!(reply.field_or("text", || "fallback".into()), "");
        assert_eq!(reply.field_or("missing", String::new), "");
        assert_eq!(reply.field_or("n", String::new), "1.5");
    }

    #[test]
    fn health_availability_uses_js_truthiness() {
        let available = |status, body| JsonReply::new(status, body).available();
        assert!(available(200, Some(json!({"available": true}))));
        assert!(
            available(200, Some(json!({"available": "false"}))),
            "non-empty string is truthy"
        );
        assert!(!available(200, Some(json!({"available": 0}))));
        assert!(!available(
            200,
            Some(json!({"available": false, "error": "x"}))
        ));
        assert!(!available(200, None));
        assert!(!available(400, Some(json!({"available": true}))));
    }

    #[test]
    fn api_reply_mapping() {
        let json_reply = ApiReply::json(
            429,
            json!({"error": "Too many requests. Please wait a minute."}),
        );
        let mapped = JsonReply::from_api_reply(&json_reply);
        assert_eq!(mapped.status, 429);
        assert!(!mapped.ok);
        assert_eq!(
            mapped.error_or(String::new),
            "Too many requests. Please wait a minute."
        );
        let empty = JsonReply::from_api_reply(&ApiReply::Empty { status: 500 });
        assert_eq!((empty.status, empty.ok, empty.body), (500, false, None));
        let binary = ApiReply::Binary {
            status: 200,
            content_type: "audio/wav".into(),
            data: vec![1],
        };
        let mapped = JsonReply::from_api_reply(&binary);
        assert!(mapped.ok);
        assert_eq!(mapped.body, None);
    }

    #[test]
    fn transport_error_messages() {
        assert_eq!(
            TransportError::Failed(Some("Failed to fetch".into())).message_or("Unknown error"),
            "Failed to fetch"
        );
        assert_eq!(
            TransportError::Failed(Some(String::new())).message_or("Unknown error"),
            ""
        );
        assert_eq!(
            TransportError::Failed(None).message_or("Unknown error"),
            "Unknown error"
        );
        assert_eq!(TransportError::Aborted.message_or("x"), "x");
        assert!(TransportError::Aborted.is_aborted());
    }
}
