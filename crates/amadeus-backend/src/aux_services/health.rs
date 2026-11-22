//! The three `POST …/health` proxies (`tts`, `stt`, `rag`).
//!
//! All of them `GET {serverUrl}/health` under a 5 s deadline and answer **200** for every
//! upstream outcome (`available:false` + error string); only a missing `serverUrl` is 400 and a
//! non-string `serverUrl` is the empty 500 (JS TypeError). They differ in when the upstream
//! body is read ([`HealthBodyMode`]).

use std::time::Duration;

use amadeus_core::protocol::ApiReply;
use serde_json::{json, Map, Value};
use tokio::time::{timeout_at, Instant};

use super::js::{opt_trimmed, parse_json_lenient, parse_json_like_undici};
use super::upstream::{
    forbidden_health, prepare, status_text, type_error_reply, FetchFailure, Prepared,
};
use super::{AuxContext, HEALTH_TIMEOUT};

/// When the upstream `/health` body is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthBodyMode {
    /// TTS: body read BEFORE the status check, result spread into the reply on 2xx.
    ReadBeforeStatus,
    /// STT: body read only after a 2xx status, then spread into the reply.
    ReadAfterOk,
    /// RAG: body never read; 2xx gives exactly `{"available":true}`.
    Ignore,
}

/// Full route handler: lenient body parsing, `serverUrl` validation, then [`probe`].
pub(crate) async fn route(ctx: AuxContext<'_>, body: &[u8], mode: HealthBodyMode) -> ApiReply {
    route_with_timeout(ctx, body, mode, HEALTH_TIMEOUT).await
}

/// [`route`] with an explicit deadline (tests use a short one).
pub(crate) async fn route_with_timeout(
    ctx: AuxContext<'_>,
    body: &[u8],
    mode: HealthBodyMode,
    timeout: Duration,
) -> ApiReply {
    let payload = parse_json_lenient(body);
    let server_url = match opt_trimmed(&payload, "serverUrl") {
        Ok(v) => v.filter(|s| !s.is_empty()),
        Err(_) => return type_error_reply(),
    };
    let Some(server_url) = server_url else {
        return ApiReply::json(
            400,
            json!({ "available": false, "error": "serverUrl is required" }),
        );
    };
    probe_with_timeout(ctx, &server_url, mode, timeout).await
}

/// Probes `{server_url}/health` with V1's 5 s deadline.
pub async fn probe(ctx: AuxContext<'_>, server_url: &str, mode: HealthBodyMode) -> ApiReply {
    probe_with_timeout(ctx, server_url, mode, HEALTH_TIMEOUT).await
}

/// [`probe`] with an explicit deadline. The deadline starts before the request and also covers
/// the body read (a timeout while reading the body yields `body = null`, not an error).
pub async fn probe_with_timeout(
    ctx: AuxContext<'_>,
    server_url: &str,
    mode: HealthBodyMode,
    timeout: Duration,
) -> ApiReply {
    let deadline = Instant::now() + timeout;
    let url = format!("{server_url}/health");
    let (client, parsed) = match prepare(ctx, &url) {
        Prepared::Ready(client, parsed) => (client, parsed),
        Prepared::Failed(failure) => return unavailable(&failure),
        Prepared::Forbidden => return forbidden_health(),
    };
    let response = match timeout_at(deadline, client.get(parsed).send()).await {
        Err(_) => return unavailable(&FetchFailure::Aborted),
        Ok(Err(_)) => return unavailable(&FetchFailure::Failed),
        Ok(Ok(response)) => response,
    };
    let status = response.status();
    let reason = status_text(&response);
    let not_ok = || {
        ApiReply::json(
            200,
            json!({ "available": false, "error": format!("{} {}", status.as_u16(), reason) }),
        )
    };
    match mode {
        HealthBodyMode::ReadBeforeStatus => {
            let body = read_json_until(response, deadline).await;
            if !status.is_success() {
                return not_ok();
            }
            ApiReply::json(200, spread_into_available(&body))
        }
        HealthBodyMode::ReadAfterOk => {
            if !status.is_success() {
                return not_ok();
            }
            let body = read_json_until(response, deadline).await;
            ApiReply::json(200, spread_into_available(&body))
        }
        HealthBodyMode::Ignore => {
            if !status.is_success() {
                return not_ok();
            }
            ApiReply::json(200, json!({ "available": true }))
        }
    }
}

/// `await response.json().catch(() => null)` still under the probe deadline.
async fn read_json_until(response: reqwest::Response, deadline: Instant) -> Value {
    match timeout_at(deadline, response.bytes()).await {
        Ok(Ok(bytes)) => parse_json_like_undici(&bytes).unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn unavailable(failure: &FetchFailure) -> ApiReply {
    ApiReply::json(
        200,
        json!({ "available": false, "error": failure.message() }),
    )
}

/// `{ available: true, ...(body && typeof body === "object" ? body : {}) }`.
///
/// Spread keys override `available` but keep its first position (insertion order; the JS
/// array-index-first ordering is applied when serializing with [`super::js_json`]). Arrays
/// spread their indices as `"0"`, `"1"`, …; primitives and `null` add nothing.
pub fn spread_into_available(body: &Value) -> Value {
    let mut map = Map::new();
    map.insert("available".to_owned(), Value::Bool(true));
    match body {
        Value::Object(obj) => {
            for (k, v) in obj {
                map.insert(k.clone(), v.clone());
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                map.insert(i.to_string(), v.clone());
            }
        }
        _ => {}
    }
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::super::js_json::stringify;
    use super::*;

    #[test]
    fn spread_matches_v1() {
        let upstream: Value = serde_json::from_str(
            r#"{"status":"ok","available":"yes-from-upstream","runtime":{"using_gpu":true,"device":"cuda","gpu_name":"MockGPU","attn_implementation":"sdpa"},"torch":{"cuda_available":true,"detected_gpu_name":"MockGPU"}}"#,
        )
        .unwrap_or(Value::Null);
        assert_eq!(
            stringify(&spread_into_available(&upstream)),
            r#"{"available":"yes-from-upstream","status":"ok","runtime":{"using_gpu":true,"device":"cuda","gpu_name":"MockGPU","attn_implementation":"sdpa"},"torch":{"cuda_available":true,"detected_gpu_name":"MockGPU"}}"#
        );
        // Node: JSON.stringify({available:true, ...JSON.parse('{"status":"ok","10":"a","available":"x"}')})
        let numeric: Value = serde_json::from_str(r#"{"status":"ok","10":"a","available":"x"}"#)
            .unwrap_or(Value::Null);
        assert_eq!(
            stringify(&spread_into_available(&numeric)),
            r#"{"10":"a","available":"x","status":"ok"}"#
        );
        assert_eq!(
            stringify(&spread_into_available(&json!(["a", "b"]))),
            r#"{"0":"a","1":"b","available":true}"#
        );
        assert_eq!(
            stringify(&spread_into_available(&json!("str"))),
            r#"{"available":true}"#
        );
        assert_eq!(
            stringify(&spread_into_available(&Value::Null)),
            r#"{"available":true}"#
        );
        assert_eq!(
            stringify(&spread_into_available(&json!(0))),
            r#"{"available":true}"#
        );
    }
}
