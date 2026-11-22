//! Backend configuration (filled by `amadeus-server` from env/CLI, or by the desktop app).

use std::path::PathBuf;

use crate::chat::MAX_GLOBAL_REQUESTS;

/// Where the backend runs. Some V1 server behaviors only make sense on the shared web server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendMode {
    /// Web server (V1 parity): per-IP + global rate limits, chat log, request counter,
    /// FreeAPI served with server-side credentials.
    Server,
    /// Desktop app: no rate limits / chat log / counter. The FreeAPI provider (and its quota)
    /// is forwarded to the hosted web server at `remote_api_base` (e.g.
    /// `https://dsuv.elvelt.com/amadeus`) using the same HTTP API contract.
    Desktop { remote_api_base: String },
}

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub mode: BackendMode,
    /// Directory for `.amadeus_chat_log.jsonl`, `.amadeus_request_count`, rate-limit state.
    pub data_dir: PathBuf,
    /// Google service-account JSON used by FreeAPI / Vertex (V1 `realamadeusmain-*.json`).
    /// Loaded from `AMADEUS_VERTEX_CREDENTIALS`; never embedded in binaries.
    pub vertex_credentials_path: Option<PathBuf>,
    /// Google AI Studio key for FreeAPI (env `AI_STUDIO_API_KEY`, fallback `GEMINI_API_KEY`).
    /// V1 also had a hard-coded fallback key; V2 intentionally does NOT.
    pub ai_studio_api_key: Option<String>,
    /// Optional SSRF guard for the TTS/STT/RAG proxies (`AMADEUS_PROXY_ALLOWLIST`, comma
    /// separated URL prefixes). `None` = allow all (V1 behavior).
    pub proxy_allowlist: Option<Vec<String>>,
    /// Global FreeAPI cap of the web server (`AMADEUS_FREEAPI_MAX`, default
    /// [`MAX_GLOBAL_REQUESTS`] = V1's 65000): provider-8 / provider-less requests are refused
    /// once `.amadeus_request_count` reaches it; the quota's `max` and the cap message report it.
    /// Must be positive ([`crate::Backend::new`] rejects anything else in server mode). Unused on
    /// the desktop, which relays FreeAPI to the hosted server and has no limiter.
    pub free_api_max: i64,
}

impl BackendConfig {
    /// Defaults for the web server (data dir `./data`).
    pub fn server_default() -> Self {
        Self {
            mode: BackendMode::Server,
            data_dir: PathBuf::from("data"),
            vertex_credentials_path: None,
            ai_studio_api_key: None,
            proxy_allowlist: None,
            free_api_max: MAX_GLOBAL_REQUESTS,
        }
    }

    /// Defaults for the desktop app.
    pub fn desktop(data_dir: PathBuf, remote_api_base: impl Into<String>) -> Self {
        Self {
            mode: BackendMode::Desktop {
                remote_api_base: remote_api_base.into(),
            },
            data_dir,
            vertex_credentials_path: None,
            ai_studio_api_key: None,
            proxy_allowlist: None,
            free_api_max: MAX_GLOBAL_REQUESTS,
        }
    }

    pub fn is_server(&self) -> bool {
        self.mode == BackendMode::Server
    }

    pub fn is_desktop(&self) -> bool {
        matches!(self.mode, BackendMode::Desktop { .. })
    }
}
