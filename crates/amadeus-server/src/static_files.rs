//! Static serving of the Trunk `dist/` with SPA fallback.
//!
//! Everything under `{base}` that is not an API route lands here:
//! * a known page route (V1 pages: landing, app, docs + subpages, download, changelog) →
//!   `index.html` with 200 for every method but `OPTIONS`, which gets an empty 400 (Next
//!   rendered pages for any method and rejected `OPTIONS` on them, `base-server.js`);
//! * an existing file in `dist/` (hashed wasm/js/css, `resources/`, `static/`,
//!   `manifest.webmanifest`) → the file (Range / conditional GET / precompressed variants);
//!   other methods than GET/HEAD get 405 with `Allow: GET, HEAD` (V1: a 500 from Next's static
//!   handler, not reproduced);
//! * an unknown `{base}/app/api/**` path → JSON 404 `{"error":"Not found"}` (any method);
//! * anything else → `index.html` with **404**, for every method, so the client router can
//!   show its not-found view (V1 answered such paths with Next's HTML 404 page).
//!
//! Outside `{base}`: `GET /` redirects (307) to `{base}`, other paths are an empty 404.

use std::path::Path;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{ALLOW, CONTENT_TYPE, LOCATION};
use axum::http::{HeaderValue, Method, StatusCode, Uri};
use axum::response::Response;
use tower::ServiceExt;
use tower_http::services::ServeDir;

use crate::app::AppState;
use crate::policy::{ImmutableAsset, InternalRedirect, NotFoundPage};

/// Page routes of V1 (`app/amadeus/**/page.tsx`), relative to `{base}`; served with 200.
pub const SPA_ROUTES: [&str; 15] = [
    "",
    "/",
    "/app",
    "/docs",
    "/docs/api",
    "/docs/api/claude",
    "/docs/api/gemini",
    "/docs/api/groq",
    "/docs/api/openai",
    "/docs/api/vertex",
    "/docs/controls",
    "/docs/download",
    "/docs/login",
    "/download",
    "/changelog",
];

/// The kill-switch service worker V1 served at `/sw.js` (verbatim 243 bytes).
pub const SERVICE_WORKER_JS: &str = include_str!("sw.js");

/// Content-type overrides where `mime_guess` differs from V1.
fn content_type_override(rel: &str) -> Option<&'static str> {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with(".otf").then_some("font/otf")
}

/// The path relative to the base (`""` for the base itself, else starting with `/`), or
/// `None` when outside it.
pub fn strip_base<'a>(path: &'a str, base: &str) -> Option<&'a str> {
    if base.is_empty() {
        return Some(path);
    }
    let rest = path.strip_prefix(base)?;
    (rest.is_empty() || rest.starts_with('/')).then_some(rest)
}

/// True for `/app/api` and everything below it (base-relative).
pub fn is_api_path(rel: &str) -> bool {
    rel.strip_prefix("/app/api")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// True for Trunk's content-hashed top-level assets (`name-<16 hex>[_bg].js|wasm|css`).
pub fn is_hashed_asset(rel: &str) -> bool {
    let Some(name) = rel.strip_prefix('/') else {
        return false;
    };
    if name.contains('/') {
        return false;
    }
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return false;
    };
    if !matches!(ext, "js" | "wasm" | "css") {
        return false;
    }
    let stem = stem.strip_suffix("_bg").unwrap_or(stem);
    stem.rsplit_once('-')
        .is_some_and(|(_, hash)| hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// The `ServeDir` used for `dist/` (no directory listing / index, precompressed variants).
pub fn dist_service(dist_dir: &Path) -> ServeDir {
    ServeDir::new(dist_dir)
        .append_index_html_on_directories(false)
        .precompressed_br()
        .precompressed_gzip()
}

fn empty(status: StatusCode) -> Response {
    let mut res = Response::new(Body::empty());
    *res.status_mut() = status;
    res
}

fn method_not_allowed() -> Response {
    let mut res = empty(StatusCode::METHOD_NOT_ALLOWED);
    res.headers_mut()
        .insert(ALLOW, HeaderValue::from_static("GET, HEAD"));
    res
}

fn not_found_outside_base() -> Response {
    let mut res = empty(StatusCode::NOT_FOUND);
    res.extensions_mut().insert(NotFoundPage);
    res
}

/// `dist/index.html` with the given status (404 responses get the not-found cache header).
pub async fn index_response(app: &AppState, status: StatusCode) -> Response {
    match tokio::fs::read(app.dist_dir().join("index.html")).await {
        Ok(html) => {
            let mut res = Response::new(Body::from(html));
            *res.status_mut() = status;
            res.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
            if status == StatusCode::NOT_FOUND {
                res.extensions_mut().insert(NotFoundPage);
            }
            res
        }
        Err(err) => {
            tracing::error!(dist = %app.dist_dir().display(), %err, "index.html is missing from the dist directory");
            let mut res =
                Response::new(Body::from("index.html is missing from the dist directory"));
            *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            res.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("text/plain; charset=utf-8"),
            );
            res
        }
    }
}

/// `GET /sw.js` (origin root): the V1 kill-switch service worker.
pub async fn service_worker() -> Response {
    let mut res = Response::new(Body::from(SERVICE_WORKER_JS));
    res.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/javascript; charset=UTF-8"),
    );
    res
}

/// `GET {base}/healthz`.
pub async fn healthz() -> &'static str {
    "ok"
}

/// True when `dist/` has a file at `uri` (base-relative), checked with a bare `HEAD` through the
/// same [`ServeDir`] that serves it (same decoding, traversal and directory rules).
async fn dist_file_exists(app: &AppState, uri: Uri) -> bool {
    let mut probe = Request::new(Body::empty());
    *probe.method_mut() = Method::HEAD;
    *probe.uri_mut() = uri;
    match app.dist_service().oneshot(probe).await {
        Ok(res) => res.status() != StatusCode::NOT_FOUND,
        Err(never) => match never {},
    }
}

/// Router fallback: SPA pages, dist files and 404s.
pub async fn fallback(State(app): State<AppState>, req: Request) -> Response {
    let method = req.method().clone();
    let is_get = method == Method::GET || method == Method::HEAD;
    let path = req.uri().path().to_owned();
    let base = app.base().to_owned();

    if !base.is_empty() && path == "/" && is_get {
        let mut res = empty(StatusCode::TEMPORARY_REDIRECT);
        if let Ok(location) = HeaderValue::from_str(&base) {
            res.headers_mut().insert(LOCATION, location);
        }
        res.extensions_mut().insert(InternalRedirect);
        return res;
    }
    let Some(rel) = strip_base(&path, &base) else {
        return not_found_outside_base();
    };
    let rel = rel.to_owned();
    if is_api_path(&rel) {
        // Known API routes are matched before the fallback, so this one does not exist.
        return crate::api::unknown_api();
    }
    if SPA_ROUTES.contains(&rel.as_str()) {
        // Next rendered a page for any method (POST, PUT, PROPFIND, TRACE … → 200 HTML) and
        // answered `OPTIONS` with an empty 400 ("An OPTIONS request to a page handler is
        // invalid", `next/dist/server/base-server.js`).
        return if method == Method::OPTIONS {
            empty(StatusCode::BAD_REQUEST)
        } else {
            index_response(&app, StatusCode::OK).await
        };
    }

    let target = match req.uri().query() {
        Some(query) => format!("{rel}?{query}"),
        None => rel.clone(),
    };
    let Ok(uri) = target.parse::<Uri>() else {
        return index_response(&app, StatusCode::NOT_FOUND).await;
    };
    if !is_get {
        // V1: an unknown page is a 404 for every method; a write to an existing static file
        // was a 500 from Next's static handler, answered here with a plain 405.
        return if dist_file_exists(&app, uri).await {
            method_not_allowed()
        } else {
            index_response(&app, StatusCode::NOT_FOUND).await
        };
    }
    let (mut parts, _body) = req.into_parts();
    parts.uri = uri;
    let file_req = Request::from_parts(parts, Body::empty());
    let res = match app.dist_service().oneshot(file_req).await {
        Ok(res) => res,
        Err(never) => match never {},
    };
    if res.status() == StatusCode::NOT_FOUND {
        return index_response(&app, StatusCode::NOT_FOUND).await;
    }
    let mut res = res.map(Body::new);
    if res.status().is_success() || res.status() == StatusCode::NOT_MODIFIED {
        if let Some(ct) = content_type_override(&rel) {
            res.headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static(ct));
        }
        if app.immutable_assets() && is_hashed_asset(&rel) {
            res.extensions_mut().insert(ImmutableAsset);
        }
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_stripping() {
        assert_eq!(strip_base("/amadeus", "/amadeus"), Some(""));
        assert_eq!(strip_base("/amadeus/", "/amadeus"), Some("/"));
        assert_eq!(strip_base("/amadeus/app", "/amadeus"), Some("/app"));
        assert_eq!(strip_base("/amadeusx", "/amadeus"), None);
        assert_eq!(strip_base("/other", "/amadeus"), None);
        assert_eq!(strip_base("/app", ""), Some("/app"));
    }

    #[test]
    fn api_path_detection() {
        assert!(is_api_path("/app/api"));
        assert!(is_api_path("/app/api/"));
        assert!(is_api_path("/app/api/unknown/x"));
        assert!(!is_api_path("/app/apix"));
        assert!(!is_api_path("/app"));
        assert!(!is_api_path("/docs/api"));
    }

    #[test]
    fn hashed_asset_detection() {
        assert!(is_hashed_asset("/amadeus-web-ca2c2e22ebf7e45f.js"));
        assert!(is_hashed_asset("/amadeus-web-3f7ec5f2d6a42ec1_bg.wasm"));
        assert!(is_hashed_asset("/tailwind-8921a2a50c97750b.css"));
        assert!(!is_hashed_asset("/index.html"));
        assert!(!is_hashed_asset("/resources/a-0123456789abcdef.js"));
        assert!(!is_hashed_asset("/app-123.js"));
        assert!(!is_hashed_asset("/manifest.webmanifest"));
    }

    #[test]
    fn service_worker_is_verbatim() {
        assert_eq!(SERVICE_WORKER_JS.len(), 243);
        assert!(SERVICE_WORKER_JS.starts_with("self.addEventListener('install'"));
    }
}
