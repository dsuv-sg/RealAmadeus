//! Application state and router assembly.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use amadeus_backend::Backend;
use amadeus_core::protocol::paths;
use axum::extract::DefaultBodyLimit;
use axum::middleware::from_fn_with_state;
use axum::routing::{any, get};
use axum::Router;
use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::config::ServerConfig;
use crate::{api, policy, static_files};

/// Body limit of the JSON API routes (V1 had none; axum's default is 2 MiB).
pub const JSON_BODY_LIMIT: usize = 16 * 1024 * 1024;
/// Body limit of `stt/transcribe` (long / uncompressed recordings).
pub const STT_BODY_LIMIT: usize = 32 * 1024 * 1024;
/// Responses smaller than this are not compressed (Next's `compression` threshold).
pub const COMPRESSION_MIN_SIZE: u64 = 1024;

/// Shared state of the web server (cheap to clone).
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    backend: Backend,
    base: String,
    dist_dir: PathBuf,
    immutable_assets: bool,
    dist_service: ServeDir,
}

impl AppState {
    /// State for a backend and a resolved configuration.
    pub fn new(backend: Backend, config: &ServerConfig) -> Self {
        Self {
            inner: Arc::new(Inner {
                backend,
                base: config.base_path.clone(),
                dist_dir: config.dist_dir.clone(),
                immutable_assets: config.immutable_assets,
                dist_service: static_files::dist_service(&config.dist_dir),
            }),
        }
    }

    /// The backend.
    pub fn backend(&self) -> &Backend {
        &self.inner.backend
    }

    /// Normalized base path (`""` = root).
    pub fn base(&self) -> &str {
        &self.inner.base
    }

    /// The Trunk dist directory.
    pub fn dist_dir(&self) -> &Path {
        &self.inner.dist_dir
    }

    /// Whether hashed assets are served as immutable.
    pub fn immutable_assets(&self) -> bool {
        self.inner.immutable_assets
    }

    pub(crate) fn dist_service(&self) -> ServeDir {
        self.inner.dist_service.clone()
    }

    /// `{base}/resources` or `{base}/resources/…` (V1 rule `/resources/:path*`).
    pub fn is_resources_path(&self, path: &str) -> bool {
        static_files::strip_base(path, self.base())
            .and_then(|rel| rel.strip_prefix("/resources"))
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    }

    /// `{base}/{relative}` for a base-relative path constant.
    pub fn url(&self, relative: &str) -> String {
        format!("{}/{relative}", self.base())
    }
}

/// Compression policy: gzip/br for compressible responses of at least 1 KiB, never images,
/// audio/video, gRPC or `text/event-stream`.
fn compression_predicate() -> impl Predicate {
    SizeAbove::new(COMPRESSION_MIN_SIZE)
        .and(NotForContentType::GRPC)
        .and(NotForContentType::IMAGES)
        .and(NotForContentType::SSE)
        .and(NotForContentType::const_new("audio/"))
        .and(NotForContentType::const_new("video/"))
}

/// Builds the complete router (routes, SPA fallback and the V1 policy layers).
///
/// Layer order (outermost first): tracing → cache headers → Next.js redirects → compression →
/// routes. Next.js' internal redirects are therefore never compressed, and they are tagged
/// [`policy::InternalRedirect`] so the cache layer leaves them bare (every other response,
/// including 3xx replies of the API routes, gets the V1 rule headers).
pub fn build_router(state: AppState) -> Router {
    let json_limit = DefaultBodyLimit::max(JSON_BODY_LIMIT);
    Router::new()
        .route(
            &state.url(paths::API_CHAT),
            any(api::chat).layer(json_limit),
        )
        .route(
            &state.url(paths::API_CHAT_STREAM),
            any(api::chat_stream).layer(json_limit),
        )
        .route(&state.url(paths::API_CHAT_QUOTA), any(api::chat_quota))
        .route(
            &state.url(paths::API_CHAT_VERTEX_PROXY),
            any(api::vertex_proxy).layer(json_limit),
        )
        .route(
            &state.url(paths::API_TTS_SYNTHESIZE),
            any(api::tts_synthesize).layer(json_limit),
        )
        .route(
            &state.url(paths::API_TTS_HEALTH),
            any(api::tts_health).layer(json_limit),
        )
        .route(
            &state.url(paths::API_STT_TRANSCRIBE),
            any(api::stt_transcribe).layer(DefaultBodyLimit::max(STT_BODY_LIMIT)),
        )
        .route(
            &state.url(paths::API_STT_HEALTH),
            any(api::stt_health).layer(json_limit),
        )
        .route(
            &state.url(paths::API_RAG_EMBED),
            any(api::rag_embed).layer(json_limit),
        )
        .route(
            &state.url(paths::API_RAG_HEALTH),
            any(api::rag_health).layer(json_limit),
        )
        .route(&state.url(paths::HEALTHZ), get(static_files::healthz))
        .route("/sw.js", get(static_files::service_worker))
        .fallback(static_files::fallback)
        .layer(CompressionLayer::new().compress_when(compression_predicate()))
        .layer(from_fn_with_state(state.clone(), policy::redirects))
        .layer(from_fn_with_state(state.clone(), policy::cache_headers))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
