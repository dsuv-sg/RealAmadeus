//! V1 HTTP policies: the `next.config.ts` cache header rules and Next.js' internal 308
//! redirects (repeated slashes, trailing slash).

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::{CACHE_CONTROL, LOCATION};
use axum::http::{HeaderName, HeaderValue, StatusCode, Uri};
use axum::middleware::Next;
use axum::response::Response;

use crate::app::AppState;

/// Header set of the `/amadeus/:path*` and `/(.*)` rules (applied to every response).
pub const NO_STORE_SET: [(&str, &str); 5] = [
    (
        "cache-control",
        "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0",
    ),
    ("pragma", "no-cache"),
    ("expires", "0"),
    ("surrogate-control", "no-store"),
    ("cdn-cache-control", "no-store"),
];

/// Header set of the `/resources/:path*` rule (V2: `{base}/resources[/…]`).
pub const PUBLIC_SET: [(&str, &str); 5] = [
    (
        "cache-control",
        "public, max-age=86400, stale-while-revalidate=604800",
    ),
    ("pragma", "public"),
    ("expires", "86400"),
    ("surrogate-control", "public, max-age=86400"),
    ("cdn-cache-control", "public, max-age=86400"),
];

/// `Cache-Control` of Next's not-found page (replaces only that key, so `/resources` 404s keep
/// the other four public keys, like V1).
pub const NOT_FOUND_CACHE_CONTROL: &str = "private, no-cache, no-store, max-age=0, must-revalidate";

/// `Cache-Control` for Trunk's content-hashed assets when `--immutable-assets` is on (V2 only).
pub const IMMUTABLE_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// Response extension marking a "page not found" response (gets [`NOT_FOUND_CACHE_CONTROL`]).
#[derive(Debug, Clone, Copy)]
pub struct NotFoundPage;

/// Response extension marking a content-hashed asset that may be cached forever.
#[derive(Debug, Clone, Copy)]
pub struct ImmutableAsset;

/// Response extension marking a redirect the server generates before routing: Next's internal
/// 308s ([`permanent_redirect`]) and V2's `GET /` → `{base}` 307. Only these skip the rule
/// headers; Next ran its internal redirects before the `headers()` rules, but applied the rules
/// to everything a handler returned, whatever the status (an upstream 302 relayed by
/// `rag/embed` keeps the no-store set).
#[derive(Debug, Clone, Copy)]
pub struct InternalRedirect;

fn set_headers(headers: &mut axum::http::HeaderMap, set: &[(&'static str, &'static str)]) {
    for (name, value) in set {
        headers.insert(
            HeaderName::from_static(name),
            HeaderValue::from_static(value),
        );
    }
}

/// Applies the V1 cache headers to every response except an [`InternalRedirect`], overriding
/// handler values (Next applied the config headers first and handler headers could not replace
/// them). 3xx replies of the routes (statuses relayed from an upstream) and `304 Not Modified`
/// get them too.
pub async fn cache_headers(State(app): State<AppState>, req: Request, next: Next) -> Response {
    let resources = app.is_resources_path(req.uri().path());
    let mut res = next.run(req).await;
    if res.extensions().get::<InternalRedirect>().is_some() {
        return res;
    }
    let immutable = res.extensions().get::<ImmutableAsset>().is_some();
    let not_found = res.extensions().get::<NotFoundPage>().is_some();
    let headers = res.headers_mut();
    if immutable {
        for name in ["pragma", "expires"] {
            headers.remove(name);
        }
        set_headers(
            headers,
            &[
                ("cache-control", IMMUTABLE_CACHE_CONTROL),
                ("surrogate-control", "public, max-age=31536000"),
                ("cdn-cache-control", "public, max-age=31536000"),
            ],
        );
    } else {
        set_headers(
            headers,
            if resources {
                &PUBLIC_SET
            } else {
                &NO_STORE_SET
            },
        );
    }
    if not_found {
        headers.insert(
            CACHE_CONTROL,
            HeaderValue::from_static(NOT_FOUND_CACHE_CONTROL),
        );
    }
    res
}

/// Answers Next.js' internal redirects before routing.
pub async fn redirects(State(app): State<AppState>, req: Request, next: Next) -> Response {
    if let Some(location) = redirect_target(req.uri(), app.base()) {
        if let Some(res) = permanent_redirect(&location) {
            return res;
        }
    }
    next.run(req).await
}

/// Where Next.js would 308-redirect this request, if anywhere.
///
/// 1. A path containing `\` or `//` → backslashes become `/` and slash runs collapse; the
///    query is kept unless its first `?`-separated part is empty (`normalizeRepeatedSlashes`).
/// 2. A path other than `/` ending with `/` → exactly one trailing slash removed, non-empty
///    query kept. V2 exception: `{base}/` itself serves the app (no redirect).
pub fn redirect_target(uri: &Uri, base: &str) -> Option<String> {
    let path = uri.path();
    let query = uri.query().unwrap_or("");
    if path.contains("//") || path.contains('\\') {
        let mut clean = String::with_capacity(path.len());
        for c in path.chars().map(|c| if c == '\\' { '/' } else { c }) {
            if c == '/' && clean.ends_with('/') {
                continue;
            }
            clean.push(c);
        }
        let first_part_empty = query.split('?').next().is_none_or(str::is_empty);
        if !first_part_empty {
            clean.push('?');
            clean.push_str(query);
        }
        return Some(clean);
    }
    if path.len() > 1 && path.ends_with('/') {
        if !base.is_empty() && path.len() == base.len() + 1 && path.starts_with(base) {
            return None;
        }
        let mut location = path[..path.len() - 1].to_owned();
        if !query.is_empty() {
            location.push('?');
            location.push_str(query);
        }
        return Some(location);
    }
    None
}

/// Next's 308: `Location`, `Refresh: 0;url=…`, the destination as body, no cache headers
/// (tagged [`InternalRedirect`]).
pub fn permanent_redirect(location: &str) -> Option<Response> {
    let location_value = HeaderValue::from_str(location).ok()?;
    let refresh = HeaderValue::from_str(&format!("0;url={location}")).ok()?;
    let mut res = Response::new(Body::from(location.to_owned()));
    *res.status_mut() = StatusCode::PERMANENT_REDIRECT;
    res.headers_mut().insert(LOCATION, location_value);
    res.headers_mut()
        .insert(HeaderName::from_static("refresh"), refresh);
    res.extensions_mut().insert(InternalRedirect);
    Some(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(uri: &str, base: &str) -> Option<String> {
        redirect_target(&uri.parse::<Uri>().unwrap_or_default(), base)
    }

    #[test]
    fn redirects_match_v1() {
        // Observed on the V1 production server.
        assert_eq!(
            target("/amadeus/app/?x=1&y=2", "/amadeus").as_deref(),
            Some("/amadeus/app?x=1&y=2")
        );
        assert_eq!(
            target("/amadeus/app/", "/amadeus").as_deref(),
            Some("/amadeus/app")
        );
        assert_eq!(
            target("/amadeus//app", "/amadeus").as_deref(),
            Some("/amadeus/app")
        );
        assert_eq!(
            target("/amadeus/app//", "/amadeus").as_deref(),
            Some("/amadeus/app/")
        );
        assert_eq!(
            target("/amadeus/%E7%B4%85/", "/amadeus").as_deref(),
            Some("/amadeus/%E7%B4%85")
        );
        assert_eq!(
            target("/amadeus/a%20b/?q=%20x", "/amadeus").as_deref(),
            Some("/amadeus/a%20b?q=%20x")
        );
        assert_eq!(
            target("/amadeus/app/?", "/amadeus").as_deref(),
            Some("/amadeus/app")
        );
        assert_eq!(
            target("/amadeus//app?", "/amadeus").as_deref(),
            Some("/amadeus/app")
        );
        assert_eq!(
            target("/amadeus//app?a=1?b=2", "/amadeus").as_deref(),
            Some("/amadeus/app?a=1?b=2")
        );
        assert_eq!(
            target("/amadeus/app/api/rag/health/", "/amadeus").as_deref(),
            Some("/amadeus/app/api/rag/health")
        );
        assert_eq!(
            target("/resources/images/", "/amadeus").as_deref(),
            Some("/resources/images")
        );
        // No redirect.
        assert_eq!(target("/amadeus/", "/amadeus"), None);
        assert_eq!(target("/amadeus/?x=1", "/amadeus"), None);
        assert_eq!(target("/amadeus", "/amadeus"), None);
        assert_eq!(target("/", "/amadeus"), None);
        assert_eq!(target("/", ""), None);
        assert_eq!(target("/amadeus/app", "/amadeus"), None);
        assert_eq!(
            target("/amadeusx/", "/amadeus").as_deref(),
            Some("/amadeusx")
        );
    }

    #[test]
    fn redirect_response_shape() {
        let res = permanent_redirect("/amadeus/app?x=1").unwrap_or_default();
        assert_eq!(res.status(), StatusCode::PERMANENT_REDIRECT);
        assert!(res.extensions().get::<InternalRedirect>().is_some());
        assert_eq!(
            res.headers().get(LOCATION).and_then(|v| v.to_str().ok()),
            Some("/amadeus/app?x=1")
        );
        assert_eq!(
            res.headers().get("refresh").and_then(|v| v.to_str().ok()),
            Some("0;url=/amadeus/app?x=1")
        );
        assert!(res.headers().get("content-type").is_none());
    }
}
