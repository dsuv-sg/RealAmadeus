//! TTS / STT / RAG proxy endpoints (`/tts/*`, `/stt/*`, `/rag/*`), ported from the V1 Next.js
//! route handlers (`app/amadeus/app/api/{tts,stt,rag}`), see porting spec `spec-server-aux.md`.
//!
//! Semantics reproduced exactly:
//! * request bodies are parsed leniently (any `Content-Type`; invalid JSON = every field
//!   missing), string fields go through JS `?.trim()` (a non-string → empty HTTP 500);
//! * upstream URLs are naive concatenations (`{serverUrl}/health`), errors carry Node's
//!   messages (`fetch failed`, `Failed to parse URL from …`, `This operation was aborted`);
//! * upstream statuses / content types / JSON bodies are passed through;
//! * the three health probes use a 5 s deadline and always answer 200 except for a missing
//!   `serverUrl` (400).
//!
//! V2 addition: the optional [`allowlist`] (`BackendConfig::proxy_allowlist`) → 403.

pub mod allowlist;
pub mod form;
pub mod health;
pub mod js;
pub mod js_json;
pub mod rag;
pub mod stt;
pub mod tts;
pub mod upstream;

use std::time::Duration;

pub use health::HealthBodyMode;
pub use upstream::FetchFailure;

use crate::Backend;

/// Deadline of the three `/health` probes (V1 `AbortController`, 5000 ms).
pub const HEALTH_TIMEOUT: Duration = Duration::from_millis(5000);
/// Default TTS speaker (`payload?.speaker?.trim() || "christina-jp"`).
pub const DEFAULT_TTS_SPEAKER: &str = "christina-jp";
/// Default STT provider (`form.get("provider") ?? "groq"`).
pub const DEFAULT_STT_PROVIDER: &str = "groq";
/// Default STT language (`form.get("language") ?? "ja"`).
pub const DEFAULT_STT_LANGUAGE: &str = "ja";
/// File name of the forwarded STT audio part (always, whatever the real format).
pub const STT_UPLOAD_FILENAME: &str = "recording.webm";

/// What an aux proxy call needs from the backend (lets tests call the handlers without a full
/// [`Backend`]).
#[derive(Clone, Copy)]
pub struct AuxContext<'a> {
    /// Shared HTTP client (rustls, no env proxy, 300 s read timeout).
    pub http: &'a reqwest::Client,
    /// `AMADEUS_PROXY_ALLOWLIST` entries; `None` = allow every URL (V1 behavior).
    pub allowlist: Option<&'a [String]>,
}

impl<'a> AuxContext<'a> {
    /// Context of a [`Backend`].
    pub fn from_backend(backend: &'a Backend) -> Self {
        Self {
            http: backend.http(),
            allowlist: backend.config().proxy_allowlist.as_deref(),
        }
    }
}
