//! Wire protocol shared by the web server, the desktop IPC layer and the frontend.
//!
//! The HTTP contract is **frozen to V1** (see `docs/ARCHITECTURE.md`): same paths, same JSON
//! payloads (camelCase, same key order), same SSE event objects. The desktop build sends the
//! same payloads over Tauri IPC (see [`ipc`]).

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Chat
// ---------------------------------------------------------------------------

/// Role of a chat message (`lib/amadeus/types.ts` `ChatRole`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

impl ChatRole {
    pub fn as_str(self) -> &'static str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        }
    }
}

/// One chat message (`{ role, content }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(ChatRole::System, content)
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ChatRole::User, content)
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ChatRole::Assistant, content)
    }
}

/// Body of `POST {base}/app/api/chat` and `/chat/stream` as built by the V1 client
/// (`ai-service.ts` `buildPayload`). Field order == V1 serialization order.
///
/// The server side must NOT deserialize requests into this struct (V1 relies on JS
/// coercion of arbitrary JSON); it parses `serde_json::Value`. This type is what the
/// frontend sends and what the desktop IPC layer forwards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatRequestPayload {
    pub messages: Vec<ChatMessage>,
    pub provider: i64,
    pub api_key: String,
    pub model: String,
    pub web_search: bool,
    pub vertex_project: String,
    pub vertex_location: String,
    pub local_proxy_host: String,
    pub local_proxy_port: String,
    #[serde(rename = "useGCloudCLI")]
    pub use_gcloud_cli: bool,
    pub ollama_host: String,
    pub openai_compatible: bool,
    pub openai_base_url: String,
}

/// Success / error body of the non-streaming chat endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatResponseBody {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// One SSE event of `POST {base}/app/api/chat/stream`.
///
/// Serialized exactly like V1: `{"type":"token","token":"…"}`,
/// `{"type":"complete","fullResponse":"…"}`, `{"type":"error","error":"…"}`.
/// On the wire every event is `data: <json>\n\n` (see [`StreamEvent::to_sse_frame`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StreamEvent {
    Token {
        token: String,
    },
    Complete {
        #[serde(rename = "fullResponse")]
        full_response: String,
    },
    Error {
        error: String,
    },
}

impl StreamEvent {
    pub fn token(t: impl Into<String>) -> Self {
        StreamEvent::Token { token: t.into() }
    }
    pub fn complete(t: impl Into<String>) -> Self {
        StreamEvent::Complete {
            full_response: t.into(),
        }
    }
    pub fn error(t: impl Into<String>) -> Self {
        StreamEvent::Error { error: t.into() }
    }

    /// `data: {json}\n\n` — one SSE frame, byte-identical to V1's `JSON.stringify` output.
    pub fn to_sse_frame(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_owned());
        format!("data: {json}\n\n")
    }

    /// True for `complete` and `error` (the stream is finished after these).
    pub fn is_terminal(&self) -> bool {
        !matches!(self, StreamEvent::Token { .. })
    }
}

/// Body of `GET {base}/app/api/chat/quota` (FreeAPI global quota).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub count: i64,
    pub max: i64,
    pub remaining: i64,
}

// ---------------------------------------------------------------------------
// Auxiliary services (TTS / STT / RAG proxies). Field order == V1 client order.
// ---------------------------------------------------------------------------

/// `POST {base}/app/api/tts/synthesize` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsSynthesizeBody {
    pub server_url: String,
    pub speaker: String,
    pub text: String,
    pub use_cache: bool,
}

/// Body of the three `…/health` endpoints (`{ "serverUrl": … }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerUrlBody {
    pub server_url: String,
}

/// `POST {base}/app/api/rag/embed` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagEmbedBody {
    pub server_url: String,
    pub text: String,
}

/// Text fields of the multipart `POST {base}/app/api/stt/transcribe` request. The audio part
/// is named `audio` (filename `recording.webm`); fields are appended in this order after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SttTranscribeFields {
    pub provider: String,
    pub language: String,
    pub server_url: String,
}

// ---------------------------------------------------------------------------
// Transport-neutral reply
// ---------------------------------------------------------------------------

/// Result of every non-streaming backend operation. The web server maps it to an HTTP
/// response; the desktop IPC layer returns it to the frontend as JSON. Server→client JSON
/// bodies stay `serde_json::Value` because V1 passes upstream bodies through verbatim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ApiReply {
    /// JSON body with an HTTP status.
    Json {
        status: u16,
        body: serde_json::Value,
    },
    /// Binary body (e.g. synthesized audio). Base64 when serialized (IPC).
    Binary {
        status: u16,
        #[serde(rename = "contentType")]
        content_type: String,
        #[serde(with = "b64")]
        data: Vec<u8>,
    },
    /// Empty body (V1 cases such as a JS TypeError → 500 with no body, 204, 405).
    Empty { status: u16 },
}

impl ApiReply {
    pub fn json(status: u16, body: serde_json::Value) -> Self {
        ApiReply::Json { status, body }
    }
    /// `{ "error": msg }` with the given status.
    pub fn error(status: u16, msg: impl Into<String>) -> Self {
        ApiReply::Json {
            status,
            body: serde_json::json!({ "error": msg.into() }),
        }
    }
    pub fn status(&self) -> u16 {
        match self {
            ApiReply::Json { status, .. }
            | ApiReply::Binary { status, .. }
            | ApiReply::Empty { status } => *status,
        }
    }
    pub fn is_ok(&self) -> bool {
        (200..300).contains(&self.status())
    }
    /// JSON body if this is a JSON reply.
    pub fn json_body(&self) -> Option<&serde_json::Value> {
        match self {
            ApiReply::Json { body, .. } => Some(body),
            _ => None,
        }
    }
}

/// Minimal dependency-free base64 (standard alphabet, padded) for [`ApiReply::Binary`].
pub mod b64 {
    use serde::{Deserialize, Deserializer, Serializer};

    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(input: &[u8]) -> String {
        let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
        for chunk in input.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 {
                ALPHABET[(n >> 6) as usize & 63] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                ALPHABET[n as usize & 63] as char
            } else {
                '='
            });
        }
        out
    }

    pub fn decode(input: &str) -> Result<Vec<u8>, String> {
        fn val(c: u8) -> Option<u32> {
            match c {
                b'A'..=b'Z' => Some(u32::from(c - b'A')),
                b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
                b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
                b'+' | b'-' => Some(62),
                b'/' | b'_' => Some(63),
                _ => None,
            }
        }
        let clean: Vec<u8> = input
            .bytes()
            .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
            .collect();
        let mut out = Vec::with_capacity(clean.len() * 3 / 4);
        for chunk in clean.chunks(4) {
            let mut n = 0u32;
            for (i, &c) in chunk.iter().enumerate() {
                n |= val(c).ok_or_else(|| format!("invalid base64 byte {c}"))? << (18 - 6 * i);
            }
            out.push((n >> 16) as u8);
            if chunk.len() > 2 {
                out.push((n >> 8) as u8);
            }
            if chunk.len() > 3 {
                out.push(n as u8);
            }
        }
        Ok(out)
    }

    pub fn serialize<S: Serializer>(data: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&encode(data))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        decode(&s).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// Paths (relative to the frontend base URL, i.e. `document.baseURI`)
// ---------------------------------------------------------------------------

/// API paths **relative to the app base** (`{base}` = `/amadeus/` on the web, `/` on
/// desktop). With the default base these are exactly V1's `/amadeus/app/api/...` URLs.
pub mod paths {
    /// Default web base path (no trailing slash). Must match Trunk's `public_url`.
    pub const DEFAULT_BASE_PATH: &str = "/amadeus";

    pub const API_CHAT: &str = "app/api/chat";
    pub const API_CHAT_STREAM: &str = "app/api/chat/stream";
    pub const API_CHAT_QUOTA: &str = "app/api/chat/quota";
    pub const API_CHAT_VERTEX_PROXY: &str = "app/api/chat/vertex-proxy";
    pub const API_TTS_SYNTHESIZE: &str = "app/api/tts/synthesize";
    pub const API_TTS_HEALTH: &str = "app/api/tts/health";
    pub const API_STT_TRANSCRIBE: &str = "app/api/stt/transcribe";
    pub const API_STT_HEALTH: &str = "app/api/stt/health";
    pub const API_RAG_EMBED: &str = "app/api/rag/embed";
    pub const API_RAG_HEALTH: &str = "app/api/rag/health";

    /// Health probe of the V2 server itself (new in V2).
    pub const HEALTHZ: &str = "healthz";
    /// Web app manifest (fixes V1's 404).
    pub const MANIFEST: &str = "manifest.webmanifest";

    /// Static resources (V1 `/resources/...`), base-relative in V2.
    pub const RESOURCES_DIR: &str = "resources/";
    /// V1 root-level public files (`/amadeus_logo.png`, `/Gemini_STEP1.png`, …), base-relative in V2.
    pub const STATIC_DIR: &str = "static/";
}

// ---------------------------------------------------------------------------
// Desktop IPC (Tauri commands). Argument names are camelCase on the JS side.
// ---------------------------------------------------------------------------

/// Tauri command names and argument shapes used by the desktop transport.
///
/// | command              | JS args                                            | returns                 |
/// |----------------------|----------------------------------------------------|-------------------------|
/// | `chat`               | `{ payload }`                                      | [`ApiReply`]            |
/// | `chat_stream`        | `{ payload, streamId, onEvent: Channel<StreamEvent> }` | `null` when the stream ran, or [`ApiReply`] for a pre-stream error (400/429) |
/// | `chat_stream_cancel` | `{ streamId }`                                     | `null`                  |
/// | `chat_quota`         | `{}`                                               | [`ApiReply`]            |
/// | `tts_synthesize`     | `{ body: TtsSynthesizeBody }`                      | [`ApiReply`] (Binary on success) |
/// | `tts_health`         | `{ body: ServerUrlBody }`                          | [`ApiReply`]            |
/// | `stt_transcribe`     | `{ fields: SttTranscribeFields, audioBase64, mime }` | [`ApiReply`]          |
/// | `stt_health`         | `{ body: ServerUrlBody }`                          | [`ApiReply`]            |
/// | `rag_embed`          | `{ body: RagEmbedBody }`                           | [`ApiReply`]            |
/// | `rag_health`         | `{ body: ServerUrlBody }`                          | [`ApiReply`]            |
/// | `app_info`           | `{}`                                               | [`AppInfo`](ipc::AppInfo) |
///
/// `payload` is the JSON of a [`ChatRequestPayload`] (sent as a JSON value so the desktop
/// backend runs the exact same code path as the web server).
pub mod ipc {
    use serde::{Deserialize, Serialize};

    pub const CHAT: &str = "chat";
    pub const CHAT_STREAM: &str = "chat_stream";
    pub const CHAT_STREAM_CANCEL: &str = "chat_stream_cancel";
    pub const CHAT_QUOTA: &str = "chat_quota";
    pub const TTS_SYNTHESIZE: &str = "tts_synthesize";
    pub const TTS_HEALTH: &str = "tts_health";
    pub const STT_TRANSCRIBE: &str = "stt_transcribe";
    pub const STT_HEALTH: &str = "stt_health";
    pub const RAG_EMBED: &str = "rag_embed";
    pub const RAG_HEALTH: &str = "rag_health";
    pub const APP_INFO: &str = "app_info";

    /// Returned by `app_info`.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AppInfo {
        pub version: String,
        pub display_version: String,
        /// `windows` | `macos` | `linux`
        pub platform: String,
        pub arch: String,
        /// Base URL of the hosted web server used for the FreeAPI provider.
        pub remote_api_base: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_event_wire_format_matches_v1() {
        assert_eq!(
            StreamEvent::token("あ").to_sse_frame(),
            "data: {\"type\":\"token\",\"token\":\"あ\"}\n\n"
        );
        assert_eq!(
            StreamEvent::complete("a\"\\\nあ/<>").to_sse_frame(),
            "data: {\"type\":\"complete\",\"fullResponse\":\"a\\\"\\\\\\nあ/<>\"}\n\n"
        );
        assert_eq!(
            StreamEvent::error("x").to_sse_frame(),
            "data: {\"type\":\"error\",\"error\":\"x\"}\n\n"
        );
        let parsed: StreamEvent =
            serde_json::from_str("{\"type\":\"complete\",\"fullResponse\":\"ok\"}").unwrap();
        assert_eq!(parsed, StreamEvent::complete("ok"));
    }

    #[test]
    fn payload_key_order_matches_v1() {
        let p = ChatRequestPayload {
            messages: vec![ChatMessage::user("hi")],
            provider: 8,
            api_key: String::new(),
            model: String::new(),
            web_search: false,
            vertex_project: String::new(),
            vertex_location: "us-central1".into(),
            local_proxy_host: "127.0.0.1".into(),
            local_proxy_port: String::new(),
            use_gcloud_cli: true,
            ollama_host: "http://localhost:11434".into(),
            openai_compatible: false,
            openai_base_url: String::new(),
        };
        let s = serde_json::to_string(&p).unwrap();
        assert!(s.starts_with("{\"messages\":[{\"role\":\"user\",\"content\":\"hi\"}],\"provider\":8,\"apiKey\":\"\",\"model\":\"\",\"webSearch\":false,"));
        assert!(s.contains("\"useGCloudCLI\":true,\"ollamaHost\""));
        assert!(s.ends_with("\"openaiCompatible\":false,\"openaiBaseUrl\":\"\"}"));
    }

    #[test]
    fn base64_roundtrip() {
        for data in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            &[0u8, 255, 128, 7],
        ] {
            let e = b64::encode(data);
            assert_eq!(b64::decode(&e).unwrap(), data);
        }
        assert_eq!(b64::encode(b"foobar"), "Zm9vYmFy");
        let reply = ApiReply::Binary {
            status: 200,
            content_type: "audio/wav".into(),
            data: vec![1, 2, 3],
        };
        let json = serde_json::to_string(&reply).unwrap();
        assert_eq!(
            json,
            "{\"kind\":\"binary\",\"status\":200,\"contentType\":\"audio/wav\",\"data\":\"AQID\"}"
        );
        assert_eq!(serde_json::from_str::<ApiReply>(&json).unwrap(), reply);
    }
}
