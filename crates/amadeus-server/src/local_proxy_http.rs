//! HTTP adapter of the `amadeus-local-proxy` binary around
//! [`amadeus_backend::local_proxy::LocalProxy`] (manual method/path dispatch, so e.g.
//! `HEAD /health` is a 404 exactly like V1's Node server).

use std::sync::Arc;

use amadeus_backend::local_proxy::{
    BodyReadError, LocalProxy, ProxyResponse, BODY_LIMIT_BYTES, CORS_HEADERS,
};
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::Response;
use axum::Router;
use bytes::{Bytes, BytesMut};
use futures_util::StreamExt;

/// Router answering every request through [`LocalProxy::handle`].
pub fn router(proxy: Arc<LocalProxy>) -> Router {
    Router::new().fallback(handle).with_state(proxy)
}

async fn handle(State(proxy): State<Arc<LocalProxy>>, req: Request) -> Response {
    let method = req.method().as_str().to_owned();
    let target = req.uri().to_string();
    let body = req.into_body();
    to_response(
        proxy
            .handle(&method, &target, read_limited(body, BODY_LIMIT_BYTES))
            .await,
    )
}

/// Reads the whole body, failing fast once it exceeds `limit` bytes.
pub async fn read_limited(body: Body, limit: usize) -> Result<Bytes, BodyReadError> {
    let mut stream = body.into_data_stream();
    let mut buf = BytesMut::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| BodyReadError::Failed(err.to_string()))?;
        if buf.len() + chunk.len() > limit {
            return Err(BodyReadError::TooLarge);
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf.freeze())
}

/// Status, CORS headers, optional content type and body.
pub fn to_response(res: ProxyResponse) -> Response {
    let mut out = Response::new(Body::from(res.body));
    *out.status_mut() =
        StatusCode::from_u16(res.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    if let Some(ct) = res.content_type {
        out.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static(ct));
    }
    for (name, value) in CORS_HEADERS {
        if let Ok(name) = HeaderName::from_bytes(name.as_bytes()) {
            out.headers_mut()
                .insert(name, HeaderValue::from_static(value));
        }
    }
    out
}
