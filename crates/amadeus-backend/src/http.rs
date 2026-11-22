//! Shared HTTP client settings.
//!
//! Every upstream request (chat providers, Vertex / Google token exchange, the TTS/STT/RAG
//! proxies, the FreeAPI relay of the desktop app and the local proxy) goes through a client made
//! by `client_builder`, so the timeouts and implicit headers are the same everywhere.

use std::time::Duration;

/// Connect timeout of every upstream request: undici's default (`connect.timeout`, 10 s), which
/// V1's `fetch` used for the providers and the proxies alike.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Read timeout (longest pause between two body chunks); generous for long LLM streams.
pub const READ_TIMEOUT: Duration = Duration::from_secs(300);

/// Redirects one upstream request follows (fetch spec / undici: the 21st redirect is a network
/// error, i.e. `fetch failed`).
pub const MAX_REDIRECTS: usize = 20;

/// `User-Agent` of every upstream request.
pub const USER_AGENT: &str = concat!("RealAmadeus/", env!("CARGO_PKG_VERSION"));

/// The settings every upstream client shares: rustls, no system proxy, [`CONNECT_TIMEOUT`],
/// [`READ_TIMEOUT`], [`USER_AGENT`], and no automatic `Referer` (reqwest would otherwise send the
/// previous URL, query string included, on every redirect hop; undici never does). Callers add
/// their redirect policy.
pub(crate) fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .no_proxy()
        .referer(false)
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
}

/// Build the process-wide reqwest client: `client_builder` following up to [`MAX_REDIRECTS`]
/// redirects.
pub fn build_client() -> anyhow::Result<reqwest::Client> {
    let client = client_builder()
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .build()?;
    Ok(client)
}

/// A local redirect chain for client tests: `POST {base}/start?key=secret` → 307 `/next` →
/// 302 `/end` → 200 `done`; every request is recorded as `(path and query, Referer)`.
#[cfg(test)]
pub(crate) mod redirect_chain {
    use std::sync::{Arc, Mutex};

    use axum::http::{header, HeaderMap, StatusCode, Uri};
    use axum::response::{IntoResponse, Response};
    use axum::Router;

    /// `(path and query, Referer header)` of every request received.
    pub(crate) type Log = Arc<Mutex<Vec<(String, Option<String>)>>>;

    /// Starts the chain on 127.0.0.1 and returns its base URL and request log.
    pub(crate) async fn start() -> (String, Log) {
        let log: Log = Arc::default();
        let recorder = log.clone();
        let app = Router::new().fallback(move |uri: Uri, headers: HeaderMap| {
            let log = recorder.clone();
            async move {
                let path = uri
                    .path_and_query()
                    .map(ToString::to_string)
                    .unwrap_or_default();
                let referer = headers
                    .get(header::REFERER)
                    .and_then(|v| v.to_str().ok())
                    .map(str::to_owned);
                log.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push((path, referer));
                let hop = |status: StatusCode, to: &'static str| -> Response {
                    (status, [(header::LOCATION, to)]).into_response()
                };
                match uri.path() {
                    "/start" => hop(StatusCode::TEMPORARY_REDIRECT, "/next"),
                    "/next" => hop(StatusCode::FOUND, "/end"),
                    _ => "done".into_response(),
                }
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind redirect chain");
        let addr = listener.local_addr().expect("redirect chain addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        (format!("http://{addr}"), log)
    }

    /// Sends the chain's first request with `client` and returns the recorded hops.
    pub(crate) async fn run(client: &reqwest::Client) -> Vec<(String, Option<String>)> {
        let (base, log) = start().await;
        let response = client
            .post(format!("{base}/start?key=secret"))
            .body("payload")
            .send()
            .await
            .expect("redirect chain request");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.text().await.ok().as_deref(), Some("done"));
        let hops = log.lock().unwrap_or_else(|e| e.into_inner()).clone();
        hops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_settings_match_undici() {
        assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(10));
        assert_eq!(READ_TIMEOUT, Duration::from_secs(300));
        assert_eq!(MAX_REDIRECTS, 20);
        assert_eq!(
            USER_AGENT,
            format!("RealAmadeus/{}", env!("CARGO_PKG_VERSION"))
        );
    }

    #[tokio::test]
    async fn redirect_hops_carry_no_referer() {
        let client = build_client().expect("client");
        let hops = redirect_chain::run(&client).await;
        assert_eq!(
            hops,
            [
                ("/start?key=secret".to_owned(), None),
                ("/next".to_owned(), None),
                ("/end".to_owned(), None),
            ]
        );
    }
}
