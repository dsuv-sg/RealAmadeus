//! Small JS interop helpers shared by the transports and the services.

use serde::Serialize;
use wasm_bindgen::{JsCast, JsValue};

use super::TransportError;

/// V1 `error instanceof Error ? error.message : <fallback>`: `Some(message)` for `Error`
/// objects (including `DOMException`s such as `AbortError` / `NotAllowedError`), else `None`.
pub(crate) fn error_message(value: &JsValue) -> Option<String> {
    value
        .dyn_ref::<js_sys::Error>()
        .map(|error| String::from(error.message()))
}

/// A rejected `fetch`/`read`/`blob` promise as a [`TransportError`] (not an abort).
pub(crate) fn failed(value: &JsValue) -> TransportError {
    TransportError::Failed(error_message(value))
}

/// A rejected `invoke`: Tauri rejects with a plain message string (e.g. an ACL error), keep it.
pub(crate) fn invoke_failed(value: &JsValue) -> TransportError {
    TransportError::Failed(value.as_string().or_else(|| error_message(value)))
}

/// A local failure (serialization, missing API) with a message.
pub(crate) fn local_failure(message: impl Into<String>) -> TransportError {
    TransportError::Failed(Some(message.into()))
}

/// `serde_json` text of `value` (V1 `JSON.stringify` output for our DTOs: same key order and
/// escaping).
pub(crate) fn json_text<T: Serialize + ?Sized>(value: &T) -> Result<String, TransportError> {
    serde_json::to_string(value)
        .map_err(|e| local_failure(format!("cannot encode the request: {e}")))
}

/// `JSON.parse(serde_json::to_string(value))`: a plain JS object with JSON semantics.
pub(crate) fn to_js_object<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, TransportError> {
    let text = json_text(value)?;
    js_sys::JSON::parse(&text).map_err(|e| failed(&e))
}

/// `JSON.stringify(value)` (`None` for `undefined`, functions or cycles).
pub(crate) fn stringify(value: &JsValue) -> Option<String> {
    js_sys::JSON::stringify(value)
        .ok()
        .and_then(|s| s.as_string())
}

/// A `Blob` of `bytes` with MIME type `content_type`.
pub(crate) fn bytes_to_blob(
    bytes: &[u8],
    content_type: &str,
) -> Result<web_sys::Blob, TransportError> {
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::of1(&array);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(content_type);
    web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(|e| failed(&e))
}
