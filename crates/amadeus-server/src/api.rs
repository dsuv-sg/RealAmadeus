//! `{base}/app/api/...` handlers.
//!
//! Every route is registered with `any()` and dispatches on the method itself so the
//! responses match Next.js' app-route method handling exactly (see [`method_gate`]): `OPTIONS`
//! → 204 with `Allow`, the other unimplemented methods of Next's list → 405 with an empty body
//! and NO `Allow` header (axum's own 405 would add one), any other method → empty 400. Bodies
//! are read as raw bytes (never the `Json`/`Multipart` extractors: V1 ignored `Content-Type`).

use amadeus_backend::{ChatStreamStart, ClientMeta};
use amadeus_core::protocol::ApiReply;
use axum::body::Body;
use axum::extract::rejection::BytesRejection;
use axum::extract::{FromRequest, Request, State};
use axum::http::header::{ALLOW, CONTENT_TYPE};
use axum::http::{HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;

use crate::app::AppState;
use crate::client_ip::{client_ip, header_first, header_joined};
use crate::policy::NotFoundPage;
use crate::reply::{api_reply, sse_response};

/// The methods a Next.js route module can implement (`HTTP_METHODS`,
/// `next/dist/server/web/http.js`); any other method never reaches the handlers.
pub(crate) const NEXT_HTTP_METHODS: [Method; 7] = [
    Method::GET,
    Method::HEAD,
    Method::OPTIONS,
    Method::POST,
    Method::PUT,
    Method::DELETE,
    Method::PATCH,
];

/// Body of the plain-text 500 Next.js sends when a request cannot be handled at all.
pub(crate) const INTERNAL_SERVER_ERROR_TEXT: &str = "Internal Server Error";

/// Methods of a route (Next.js auto-implementation).
#[derive(Clone, Copy)]
enum Methods {
    /// POST-only route: `Allow: OPTIONS, POST`.
    Post,
    /// GET route (HEAD reuses GET): `Allow: GET, HEAD, OPTIONS`.
    Get,
}

fn empty(status: StatusCode) -> Response {
    let mut res = Response::new(Body::empty());
    *res.status_mut() = status;
    res
}

/// `None` when the handler should run, otherwise what Next.js 16 answered:
/// * `OPTIONS` → 204 with the sorted `Allow` list (auto-implemented);
/// * another unimplemented member of [`NEXT_HTTP_METHODS`] → empty 405, no `Allow`;
/// * `TRACE` → 500 `text/plain` "Internal Server Error" (undici's `Request` constructor rejects
///   the forbidden method before Next can resolve a handler);
/// * any other method (`PROPFIND`, `PURGE`, `QUERY`, …) → empty 400 (`AppRouteRouteModule
///   .resolve`: "Ensure that the requested method is a valid method").
fn method_gate(method: &Method, methods: Methods) -> Option<Response> {
    let (allowed, allow_header) = match methods {
        Methods::Post => (*method == Method::POST, "OPTIONS, POST"),
        Methods::Get => (
            *method == Method::GET || *method == Method::HEAD,
            "GET, HEAD, OPTIONS",
        ),
    };
    if allowed {
        return None;
    }
    if *method == Method::OPTIONS {
        let mut res = empty(StatusCode::NO_CONTENT);
        res.headers_mut()
            .insert(ALLOW, HeaderValue::from_static(allow_header));
        return Some(res);
    }
    if NEXT_HTTP_METHODS.contains(method) {
        return Some(empty(StatusCode::METHOD_NOT_ALLOWED));
    }
    if *method == Method::TRACE {
        let mut res = Response::new(Body::from(INTERNAL_SERVER_ERROR_TEXT));
        *res.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        res.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        return Some(res);
    }
    Some(empty(StatusCode::BAD_REQUEST))
}

/// Buffers the request body (limit set per route with `DefaultBodyLimit`; overflow → 413 via
/// the rejection's `IntoResponse`).
async fn body_bytes(req: Request) -> Result<Bytes, BytesRejection> {
    Bytes::from_request(req, &()).await
}

/// Reply for an unknown `{base}/app/api/**` path (any method). V1 answered with Next's HTML
/// not-found page; the spec allows any 404, and JSON is what API clients can read.
pub(crate) fn unknown_api() -> Response {
    let mut res = api_reply(ApiReply::error(404, "Not found"));
    res.extensions_mut().insert(NotFoundPage);
    res
}

macro_rules! post_json_route {
    ($(#[$doc:meta])* $name:ident, $method:ident) => {
        $(#[$doc])*
        pub(crate) async fn $name(State(app): State<AppState>, req: Request) -> Response {
            if let Some(res) = method_gate(req.method(), Methods::Post) {
                return res;
            }
            match body_bytes(req).await {
                Ok(body) => api_reply(app.backend().$method(&body).await),
                Err(rejection) => rejection.into_response(),
            }
        }
    };
}

/// `POST /chat` (non-streaming).
pub(crate) async fn chat(State(app): State<AppState>, req: Request) -> Response {
    if let Some(res) = method_gate(req.method(), Methods::Post) {
        return res;
    }
    let client = ClientMeta {
        ip: client_ip(req.headers()),
    };
    match body_bytes(req).await {
        Ok(body) => api_reply(app.backend().chat(&body, &client).await),
        Err(rejection) => rejection.into_response(),
    }
}

/// `POST /chat/stream` (SSE, or a JSON reply for pre-stream errors).
pub(crate) async fn chat_stream(State(app): State<AppState>, req: Request) -> Response {
    if let Some(res) = method_gate(req.method(), Methods::Post) {
        return res;
    }
    let client = ClientMeta {
        ip: client_ip(req.headers()),
    };
    let version = req.version();
    match body_bytes(req).await {
        Ok(body) => match app.backend().chat_stream(&body, &client).await {
            ChatStreamStart::Reply(reply) => api_reply(reply),
            ChatStreamStart::Stream(events) => sse_response(events, version),
        },
        Err(rejection) => rejection.into_response(),
    }
}

/// `GET|HEAD /chat/quota`.
pub(crate) async fn chat_quota(State(app): State<AppState>, req: Request) -> Response {
    if let Some(res) = method_gate(req.method(), Methods::Get) {
        return res;
    }
    api_reply(app.backend().chat_quota().await)
}

/// `POST /chat/vertex-proxy` (SSE when the header `x-vertex-stream` is exactly `true`).
pub(crate) async fn vertex_proxy(State(app): State<AppState>, req: Request) -> Response {
    if let Some(res) = method_gate(req.method(), Methods::Post) {
        return res;
    }
    let stream = header_joined(req.headers(), "x-vertex-stream").as_deref() == Some("true");
    let version = req.version();
    match body_bytes(req).await {
        Ok(body) => match app.backend().vertex_proxy(&body, stream).await {
            ChatStreamStart::Reply(reply) => api_reply(reply),
            ChatStreamStart::Stream(events) => sse_response(events, version),
        },
        Err(rejection) => rejection.into_response(),
    }
}

post_json_route!(
    /// `POST /tts/synthesize`.
    tts_synthesize,
    tts_synthesize
);
post_json_route!(
    /// `POST /tts/health`.
    tts_health,
    tts_health
);
post_json_route!(
    /// `POST /stt/health`.
    stt_health,
    stt_health
);
post_json_route!(
    /// `POST /rag/embed`.
    rag_embed,
    rag_embed
);
post_json_route!(
    /// `POST /rag/health`.
    rag_health,
    rag_health
);

/// `POST /stt/transcribe` (multipart or urlencoded; Node keeps the first `Content-Type`).
pub(crate) async fn stt_transcribe(State(app): State<AppState>, req: Request) -> Response {
    if let Some(res) = method_gate(req.method(), Methods::Post) {
        return res;
    }
    let content_type = header_first(req.headers(), CONTENT_TYPE.as_str());
    match body_bytes(req).await {
        Ok(body) => api_reply(
            app.backend()
                .stt_transcribe(content_type.as_deref(), body)
                .await,
        ),
        Err(rejection) => rejection.into_response(),
    }
}
