//! # amadeus-server
//!
//! The web server of RealAmadeus V2 (axum 0.8). It reproduces the HTTP surface of the V1
//! Next.js deployment under a configurable base path (default `/amadeus`):
//!
//! * `{base}/app/api/...`: the frozen V1 API (chat, SSE stream, quota, vertex-proxy,
//!   TTS/STT/RAG proxies) backed by [`amadeus_backend::Backend`];
//! * the Trunk `dist/` (WASM frontend) with SPA fallback, `{base}/resources/**` with V1's
//!   public cache policy and everything else with V1's `no-store` header set;
//! * Next.js-compatible 308 redirects (trailing slash, repeated slashes) and method handling
//!   (API: auto-implemented `OPTIONS` 204 + `Allow`, 405 / 400 for other methods; pages:
//!   rendered for every method but `OPTIONS`).
//!
//! The library exposes the router so integration tests can drive it with
//! `tower::ServiceExt::oneshot`, plus the HTTP adapter of the `amadeus-local-proxy` binary.

pub mod app;
pub mod client_ip;
pub mod config;
pub mod local_proxy_http;
pub mod policy;
pub mod reply;
pub mod shutdown;
pub mod static_files;

mod api;

pub use app::{build_router, AppState};
pub use config::ServerConfig;
