//! `POST {base}/app/api/stt/transcribe` and `/stt/health` (V1 `app/amadeus/app/api/stt/*`).

use std::hash::{BuildHasher, Hasher, RandomState};
use std::sync::atomic::{AtomicU64, Ordering};

use amadeus_core::jscompat::js_trim;
use amadeus_core::protocol::{ApiReply, SttTranscribeFields};
use bytes::Bytes;
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};

use super::form::{parse_form, FormData, FormValue};
use super::health::{self, HealthBodyMode};
use super::js::{blob_type, js_member, non_nullish, normalize_linefeeds};
use super::upstream::{
    fetch_error, forbidden, json_with_upstream_status, prepare, read_json_or_null, FetchFailure,
    Prepared,
};
use super::{AuxContext, DEFAULT_STT_LANGUAGE, DEFAULT_STT_PROVIDER, STT_UPLOAD_FILENAME};
use crate::Backend;

/// Content type undici sends for a file part whose Blob type is empty.
const OCTET_STREAM: &str = "application/octet-stream";

/// Fixed start of undici's `FormData` boundary; 11 random decimal digits follow.
pub const UNDICI_BOUNDARY_PREFIX: &str = "----formdata-undici-0";

/// `POST /stt/transcribe` with the raw request body and its `Content-Type` header (combined
/// value, latin1-decoded). Accepts `multipart/form-data` and urlencoded bodies like undici.
pub async fn transcribe(backend: &Backend, content_type: Option<&str>, body: Bytes) -> ApiReply {
    transcribe_with(AuxContext::from_backend(backend), content_type, &body).await
}

/// Desktop variant of [`transcribe`] for callers that already hold the parts.
///
/// Mirrors what the web path forwards for a browser upload: `serverUrl` is trimmed and
/// required, `provider`/`language` are forwarded verbatim, the audio part's type is the
/// Blob-normalized `mime` (empty → `application/octet-stream`).
pub async fn transcribe_parts(
    backend: &Backend,
    fields: &SttTranscribeFields,
    audio: Vec<u8>,
    mime: Option<String>,
) -> ApiReply {
    transcribe_parts_with(AuxContext::from_backend(backend), fields, audio, mime).await
}

/// `POST /stt/health` (body read only after a 2xx status, spread on success).
pub async fn health(backend: &Backend, body: &[u8]) -> ApiReply {
    health_with(AuxContext::from_backend(backend), body).await
}

/// [`health()`] with an explicit context.
pub async fn health_with(ctx: AuxContext<'_>, body: &[u8]) -> ApiReply {
    health::route(ctx, body, HealthBodyMode::ReadAfterOk).await
}

/// `String(form.get(name) ?? default)`.
fn form_string(form: &FormData, name: &str, default: &str) -> String {
    form.get(name)
        .map_or_else(|| default.to_owned(), FormValue::to_js_string)
}

/// [`transcribe`] with an explicit context.
pub async fn transcribe_with(
    ctx: AuxContext<'_>,
    content_type: Option<&str>,
    body: &[u8],
) -> ApiReply {
    let Ok(form) = parse_form(content_type, body) else {
        return ApiReply::error(400, "Invalid form data");
    };
    let provider = form_string(&form, "provider", DEFAULT_STT_PROVIDER);
    let language = form_string(&form, "language", DEFAULT_STT_LANGUAGE);
    let server_url = js_trim(&form_string(&form, "serverUrl", "")).to_owned();
    if server_url.is_empty() {
        return ApiReply::error(400, "serverUrl is required");
    }
    let Some(FormValue::File(audio)) = form.get("audio") else {
        return ApiReply::error(400, "audio file is required");
    };
    upload(
        ctx,
        &server_url,
        &audio.data,
        &audio.content_type,
        &provider,
        &language,
    )
    .await
}

/// [`transcribe_parts`] with an explicit context.
pub async fn transcribe_parts_with(
    ctx: AuxContext<'_>,
    fields: &SttTranscribeFields,
    audio: Vec<u8>,
    mime: Option<String>,
) -> ApiReply {
    let server_url = js_trim(&fields.server_url);
    if server_url.is_empty() {
        return ApiReply::error(400, "serverUrl is required");
    }
    let content_type = blob_type(mime.as_deref().unwrap_or(""));
    upload(
        ctx,
        server_url,
        &audio,
        &content_type,
        &fields.provider,
        &fields.language,
    )
    .await
}

/// A fresh undici-style boundary: [`UNDICI_BOUNDARY_PREFIX`] +
/// `` `${random(1e11)}`.padStart(11, "0") ``.
///
/// The digits come from std's randomly keyed SipHash (`RandomState`, seeded from the OS once
/// per thread) over a process-wide call counter: unpredictable enough for a delimiter, no
/// extra dependency. Like undici, the payload is not scanned for the boundary.
pub fn undici_boundary() -> String {
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(CALLS.fetch_add(1, Ordering::Relaxed));
    format!(
        "{UNDICI_BOUNDARY_PREFIX}{:011}",
        hasher.finish() % 100_000_000_000
    )
}

/// The upstream `FormData` of V1 (`audio` as `recording.webm`, then `provider`, `language`)
/// serialized byte for byte like undici's `extractBody(FormData)`:
/// `--{boundary}\r\nContent-Disposition: form-data; name="audio"; filename="recording.webm"\r\n`
/// `Content-Type: {type}\r\n\r\n{audio}\r\n`, one `--{boundary}\r\nContent-Disposition:
/// form-data; name="{name}"\r\n\r\n{value}\r\n` per text field (line breaks normalized to
/// CRLF), then `--{boundary}--\r\n`. `blob_type` is the Blob-normalized type (printable ASCII
/// only); empty → `application/octet-stream`.
///
/// The body is built in memory so reqwest can replay it on 307/308 redirects (undici
/// re-extracts a `FormData` body on every hop; a streamed multipart body is not replayable).
pub fn undici_multipart_body(
    boundary: &str,
    audio: &[u8],
    blob_type: &str,
    provider: &str,
    language: &str,
) -> Vec<u8> {
    let content_type = if blob_type.is_empty() {
        OCTET_STREAM
    } else {
        blob_type
    };
    let mut out = Vec::with_capacity(audio.len() + 4 * boundary.len() + 256);
    out.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"audio\"; \
             filename=\"{STT_UPLOAD_FILENAME}\"\r\nContent-Type: {content_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    out.extend_from_slice(audio);
    out.extend_from_slice(b"\r\n");
    for (name, value) in [("provider", provider), ("language", language)] {
        out.extend_from_slice(
            format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{}\r\n",
                normalize_linefeeds(value)
            )
            .as_bytes(),
        );
    }
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    out
}

/// Forwards the recording to `{serverUrl}/transcribe` (parts `audio`, `provider`, `language`;
/// `serverUrl` itself is not forwarded) and maps the reply like V1.
async fn upload(
    ctx: AuxContext<'_>,
    server_url: &str,
    audio: &[u8],
    blob_type: &str,
    provider: &str,
    language: &str,
) -> ApiReply {
    let url = format!("{server_url}/transcribe");
    let (client, parsed) = match prepare(ctx, &url) {
        Prepared::Ready(client, parsed) => (client, parsed),
        Prepared::Failed(failure) => return fetch_error(&failure),
        Prepared::Forbidden => return forbidden(),
    };
    let boundary = undici_boundary();
    let body = undici_multipart_body(&boundary, audio, blob_type, provider, language);
    let request = client
        .post(parsed)
        .header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body);
    let response = match request.send().await {
        Ok(response) => response,
        Err(_) => return fetch_error(&FetchFailure::Failed),
    };
    let status = response.status().as_u16();
    let body = read_json_or_null(response).await;
    if !(200..300).contains(&status) {
        let error = non_nullish(js_member(&body, "error"))
            .cloned()
            .unwrap_or_else(|| Value::String(format!("Transcription failed ({status})")));
        return json_with_upstream_status(status, json!({ "error": error }));
    }
    ApiReply::json(200, if body.is_null() { json!({}) } else { body })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What V1 (Node 22.22.0, undici 6.23.0) sent upstream for a browser upload of `AUDIO` typed
    /// `audio/webm;codecs=opus` with `provider` = `"whi\r\nsper"` and no `language` (captured raw
    /// at the socket; `content-length: 383`).
    const V1_UPSTREAM_BODY: &[u8] = b"------formdata-undici-090776481865\r\n\
Content-Disposition: form-data; name=\"audio\"; filename=\"recording.webm\"\r\n\
Content-Type: audio/webm;codecs=opus\r\n\
\r\n\
AUDIO\r\n\
------formdata-undici-090776481865\r\n\
Content-Disposition: form-data; name=\"provider\"\r\n\
\r\n\
whi\r\nsper\r\n\
------formdata-undici-090776481865\r\n\
Content-Disposition: form-data; name=\"language\"\r\n\
\r\n\
ja\r\n\
------formdata-undici-090776481865--\r\n";

    #[test]
    fn multipart_body_matches_v1_bytes() {
        let body = undici_multipart_body(
            "----formdata-undici-090776481865",
            b"AUDIO",
            "audio/webm;codecs=opus",
            "whi\r\nsper",
            DEFAULT_STT_LANGUAGE,
        );
        assert_eq!(body.len(), 383);
        assert_eq!(
            String::from_utf8_lossy(&body),
            String::from_utf8_lossy(V1_UPSTREAM_BODY)
        );
        assert_eq!(body, V1_UPSTREAM_BODY);
    }

    #[test]
    fn multipart_body_defaults_and_linefeeds() {
        let body = undici_multipart_body("B", b"\r\n--B\0", "", "a\rb\nc\r\n", "");
        assert_eq!(
            body,
            b"--B\r\nContent-Disposition: form-data; name=\"audio\"; filename=\"recording.webm\"\r\n\
Content-Type: application/octet-stream\r\n\r\n\r\n--B\0\r\n\
--B\r\nContent-Disposition: form-data; name=\"provider\"\r\n\r\na\r\nb\r\nc\r\n\r\n\
--B\r\nContent-Disposition: form-data; name=\"language\"\r\n\r\n\r\n\
--B--\r\n"
                .as_slice()
        );
        // Non-ASCII text is written as UTF-8 (TextEncoder).
        let body = undici_multipart_body("B", b"", "audio/wav", "ローカル", "日本語");
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\r\n\r\nローカル\r\n"));
        assert!(text.contains("\r\n\r\n日本語\r\n"));
    }

    #[test]
    fn boundary_has_undicis_shape() {
        let a = undici_boundary();
        let b = undici_boundary();
        for boundary in [&a, &b] {
            assert_eq!(
                boundary.len(),
                UNDICI_BOUNDARY_PREFIX.len() + 11,
                "{boundary}"
            );
            let digits = boundary
                .strip_prefix(UNDICI_BOUNDARY_PREFIX)
                .unwrap_or_default();
            assert!(digits.bytes().all(|c| c.is_ascii_digit()), "{boundary}");
        }
        assert_ne!(a, b, "every request gets a fresh boundary");
    }
}
