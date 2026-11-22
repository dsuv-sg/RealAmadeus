//! Outbound `fetch` semantics shared by the aux proxies (and the local proxy).

use std::sync::{LazyLock, Mutex, PoisonError};

use amadeus_core::protocol::ApiReply;
use reqwest::header::HeaderMap;
use serde_json::{json, Value};
use url::Url;

use super::allowlist;
use super::js::latin1_decode;
use super::AuxContext;

/// Why an upstream `fetch` failed, carrying V1's (Node 22 / undici) `error.message` strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchFailure {
    /// `new URL()` failed.
    BadUrl(String),
    /// The URL contains user info (undici refuses to build the `Request`).
    Credentials(String),
    /// The health probe's 5 s `AbortController` fired.
    Aborted,
    /// Any connection / protocol / scheme error before the response headers.
    Failed,
    /// The response body stream broke after the headers arrived.
    Terminated,
}

impl FetchFailure {
    /// The exact V1 `error.message`.
    pub fn message(&self) -> String {
        match self {
            FetchFailure::BadUrl(url) => format!("Failed to parse URL from {url}"),
            FetchFailure::Credentials(url) => {
                format!("Request cannot be constructed from a URL that includes credentials: {url}")
            }
            FetchFailure::Aborted => "This operation was aborted".to_owned(),
            FetchFailure::Failed => "fetch failed".to_owned(),
            FetchFailure::Terminated => "terminated".to_owned(),
        }
    }
}

/// Result of validating an upstream URL before the request is sent.
pub(crate) enum Prepared {
    /// Send with this client to this URL.
    Ready(reqwest::Client, Url),
    /// V1 would have thrown before sending.
    Failed(FetchFailure),
    /// Rejected by `AMADEUS_PROXY_ALLOWLIST` (V2 only).
    Forbidden,
}

/// Error string of the V2-only allowlist rejection.
pub const NOT_ALLOWED_ERROR: &str = "serverUrl not allowed";

/// Redirects one upstream `fetch` follows (fetch spec / undici: the 21st redirect is a network
/// error, i.e. `fetch failed`); same limit as [`crate::http::build_client`].
pub use crate::http::MAX_REDIRECTS;

/// Parses the naively concatenated upstream URL (`{serverUrl}/health` …) like undici's
/// `new Request(url)` and applies the optional allowlist.
pub(crate) fn prepare(ctx: AuxContext<'_>, url: &str) -> Prepared {
    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return Prepared::Failed(FetchFailure::BadUrl(url.to_owned())),
    };
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Prepared::Failed(FetchFailure::Credentials(url.to_owned()));
    }
    match ctx.allowlist {
        None => Prepared::Ready(ctx.http.clone(), parsed),
        Some(list) if allowlist::is_allowed(&parsed, list) => match guarded_client(list) {
            Ok(client) => Prepared::Ready(client, parsed),
            Err(_) => Prepared::Failed(FetchFailure::Failed),
        },
        Some(_) => Prepared::Forbidden,
    }
}

/// A client bound to the allowlist it enforces on redirects.
type GuardedClient = (Vec<String>, reqwest::Client);

/// Clients whose redirect policy re-checks every hop against an allowlist (one per distinct
/// allowlist; in practice exactly one per process).
static GUARDED_CLIENTS: LazyLock<Mutex<Vec<GuardedClient>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

/// What the allowlist client does with redirect number `redirect` (1-based) to `target`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedirectDecision {
    /// Follow it.
    Follow,
    /// More than [`MAX_REDIRECTS`]: fail like undici.
    TooMany,
    /// The target leaves the allowlist: fail without contacting it.
    NotAllowed,
}

/// The allowlist client's redirect rule (limit first, then the allowlist).
fn redirect_decision(redirect: usize, target: &Url, list: &[String]) -> RedirectDecision {
    if redirect > MAX_REDIRECTS {
        RedirectDecision::TooMany
    } else if allowlist::is_allowed(target, list) {
        RedirectDecision::Follow
    } else {
        RedirectDecision::NotAllowed
    }
}

/// A client equivalent to [`crate::http::build_client`] (the shared [`crate::http::client_builder`]
/// settings: same UA, no env proxy, 10 s connect / 300 s read timeout, no `Referer` on redirect
/// hops; ≤ [`MAX_REDIRECTS`] redirects) that refuses redirects leaving the allowlist.
fn guarded_client(list: &[String]) -> reqwest::Result<reqwest::Client> {
    let mut cache = GUARDED_CLIENTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some((_, client)) = cache.iter().find(|(l, _)| l.as_slice() == list) {
        return Ok(client.clone());
    }
    let owned = list.to_vec();
    let policy = reqwest::redirect::Policy::custom(move |attempt| {
        // reqwest records the redirecting URL before asking, so `previous().len()` is the
        // 1-based number of this redirect (the check `Policy::limited` makes).
        match redirect_decision(attempt.previous().len(), attempt.url(), &owned) {
            RedirectDecision::Follow => attempt.follow(),
            RedirectDecision::TooMany => attempt.error("too many redirects"),
            RedirectDecision::NotAllowed => attempt.error("redirect target not allowed"),
        }
    });
    let client = crate::http::client_builder().redirect(policy).build()?;
    cache.push((list.to_vec(), client.clone()));
    Ok(client)
}

/// `response.statusText`: the HTTP/1 reason phrase as sent (hyper keeps non-canonical ones in
/// an extension), else the canonical phrase (HTTP/2 has none), else empty.
pub(crate) fn status_text(response: &reqwest::Response) -> String {
    if let Some(reason) = response.extensions().get::<hyper::ext::ReasonPhrase>() {
        return latin1_decode(reason.as_bytes());
    }
    response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_owned()
}

/// `headers.get(name)`: all values latin1-decoded and joined with `", "`; `None` if absent.
pub fn header_get(headers: &HeaderMap, name: &str) -> Option<String> {
    let values: Vec<String> = headers
        .get_all(name)
        .iter()
        .map(|v| latin1_decode(v.as_bytes()))
        .collect();
    if values.is_empty() {
        None
    } else {
        Some(values.join(", "))
    }
}

/// `await response.json().catch(() => null)` without a deadline (read errors → `null`).
pub(crate) async fn read_json_or_null(response: reqwest::Response) -> Value {
    match response.bytes().await {
        Ok(bytes) => super::js::parse_json_like_undici(&bytes).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

/// `NextResponse.json(body, { status })` with an upstream status. undici's `Response`
/// constructor throws for statuses outside 200..=599 (`RangeError`) and for null-body statuses
/// (`TypeError`); the V1 handlers' `catch` turned both into 500 with the message.
pub(crate) fn json_with_upstream_status(status: u16, body: Value) -> ApiReply {
    if !(200..=599).contains(&status) {
        return ApiReply::error(
            500,
            "init[\"status\"] must be in the range of 200 to 599, inclusive.",
        );
    }
    if matches!(status, 204 | 205 | 304) {
        return ApiReply::error(
            500,
            format!("Response constructor: Invalid response status code {status}"),
        );
    }
    ApiReply::json(status, body)
}

/// 500 `{"error": message}` for a failed fetch.
pub(crate) fn fetch_error(failure: &FetchFailure) -> ApiReply {
    ApiReply::error(500, failure.message())
}

/// 403 reply for an allowlist rejection on the non-health routes.
pub(crate) fn forbidden() -> ApiReply {
    ApiReply::error(403, NOT_ALLOWED_ERROR)
}

/// 403 reply for an allowlist rejection on the health routes (keeps the health shape).
pub(crate) fn forbidden_health() -> ApiReply {
    ApiReply::json(
        403,
        json!({ "available": false, "error": NOT_ALLOWED_ERROR }),
    )
}

/// The JS `TypeError` from `payload?.x?.trim()`: HTTP 500 with no body (Next.js behavior).
pub(crate) fn type_error_reply() -> ApiReply {
    ApiReply::Empty { status: 500 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_messages_match_node() {
        assert_eq!(
            FetchFailure::BadUrl("abc/health".into()).message(),
            "Failed to parse URL from abc/health"
        );
        assert_eq!(
            FetchFailure::Credentials("http://u:p@h/health".into()).message(),
            "Request cannot be constructed from a URL that includes credentials: http://u:p@h/health"
        );
        assert_eq!(
            FetchFailure::Aborted.message(),
            "This operation was aborted"
        );
        assert_eq!(FetchFailure::Failed.message(), "fetch failed");
        assert_eq!(FetchFailure::Terminated.message(), "terminated");
    }

    #[test]
    fn upstream_status_passthrough_quirks() {
        assert_eq!(
            json_with_upstream_status(422, json!({"error": "x"})),
            ApiReply::json(422, json!({"error": "x"}))
        );
        assert_eq!(json_with_upstream_status(300, json!({})).status(), 300);
        assert_eq!(
            json_with_upstream_status(600, json!({})),
            ApiReply::error(
                500,
                "init[\"status\"] must be in the range of 200 to 599, inclusive."
            )
        );
        assert_eq!(
            json_with_upstream_status(304, json!({})),
            ApiReply::error(
                500,
                "Response constructor: Invalid response status code 304"
            )
        );
    }

    #[test]
    fn allowlist_client_follows_twenty_redirects() {
        let list = vec!["127.0.0.1:*".to_owned()];
        let inside = Url::parse("http://127.0.0.1:5100/hop").unwrap_or_else(|_| unreachable!());
        let outside = Url::parse("http://10.0.0.1/hop").unwrap_or_else(|_| unreachable!());
        for redirect in 1..=MAX_REDIRECTS {
            assert_eq!(
                redirect_decision(redirect, &inside, &list),
                RedirectDecision::Follow,
                "redirect #{redirect}"
            );
        }
        assert_eq!(
            redirect_decision(MAX_REDIRECTS + 1, &inside, &list),
            RedirectDecision::TooMany
        );
        assert_eq!(
            redirect_decision(1, &outside, &list),
            RedirectDecision::NotAllowed
        );
        assert_eq!(MAX_REDIRECTS, 20);
    }

    #[tokio::test]
    async fn allowlist_client_sends_no_referer_on_redirect_hops() {
        let client = guarded_client(&["127.0.0.1:*".to_owned()]).expect("guarded client");
        let hops = crate::http::redirect_chain::run(&client).await;
        assert_eq!(
            hops,
            [
                ("/start?key=secret".to_owned(), None),
                ("/next".to_owned(), None),
                ("/end".to_owned(), None),
            ]
        );
    }

    #[test]
    fn header_join_and_latin1() {
        let mut h = HeaderMap::new();
        h.append(
            "content-type",
            reqwest::header::HeaderValue::from_static("audio/wav"),
        );
        h.append(
            "content-type",
            reqwest::header::HeaderValue::from_bytes(b"x/\xe9")
                .unwrap_or(reqwest::header::HeaderValue::from_static("x")),
        );
        assert_eq!(
            header_get(&h, "content-type"),
            Some("audio/wav, x/\u{e9}".to_owned())
        );
        assert_eq!(header_get(&h, "x-missing"), None);
    }
}
