//! Upstream HTTP helpers reproducing what V1 observed through undici `fetch`: the exact error
//! strings (`fetch failed`, `terminated`, `Failed to parse URL from …`), header value coercion,
//! body parsing (`.json().catch(() => null)`, `.text().catch(() => "")`) and the E_NS / E_S
//! upstream error formats.

use amadeus_core::protocol::ApiReply;
use bytes::Bytes;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Response, StatusCode};
use serde_json::{json, Value};

use super::payload::{parse_json_body, Thrown};
use super::texts::{FETCH_FAILED, TERMINATED};

/// One request header: lowercase static name, JS string value.
pub(crate) type Header = (&'static str, String);

/// Header list of one upstream request.
pub(crate) type Headers = Vec<Header>;

/// `Content-Type: application/json` header entry.
pub(crate) fn content_json() -> Header {
    ("content-type", "application/json".to_owned())
}

/// Parse a request URL like undici's `new Request(url)`.
pub(crate) fn parse_fetch_url(url: &str) -> Result<url::Url, Thrown> {
    let parsed =
        url::Url::parse(url).map_err(|_| Thrown::new(format!("Failed to parse URL from {url}")))?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(Thrown::new(format!(
            "Request cannot be constructed from a URL that includes credentials: {url}"
        )));
    }
    Ok(parsed)
}

/// WebIDL `ByteString` conversion of a header value (code units above 0xFF throw).
fn check_byte_string(value: &str) -> Result<(), Thrown> {
    match value.encode_utf16().enumerate().find(|(_, u)| *u > 0xFF) {
        Some((i, u)) => Err(Thrown::new(format!(
            "Cannot convert argument to a ByteString because the character at index {i} has a value of {u} which is greater than 255."
        ))),
        None => Ok(()),
    }
}

/// undici `headerValueNormalize` + `isValidHeaderValue`, then Latin-1 encoding.
fn header_value(value: &str) -> Result<HeaderValue, Thrown> {
    let normalized = value.trim_matches(|c| matches!(c, '\t' | '\n' | '\r' | ' '));
    if normalized.contains(['\0', '\n', '\r']) {
        return Err(Thrown::new(format!(
            "Headers.append: \"{normalized}\" is an invalid header value."
        )));
    }
    // Every char is <= U+00FF after `check_byte_string`, so it maps to one Latin-1 byte.
    let latin1: Vec<u8> = normalized
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect();
    HeaderValue::from_bytes(&latin1).map_err(|_| Thrown::new(FETCH_FAILED))
}

/// Build the header map in undici's order of checks (ByteString conversion happens before URL
/// parsing; header validation after it).
fn build_headers(url: &str, headers: &[Header]) -> Result<(url::Url, HeaderMap), Thrown> {
    for (_, v) in headers {
        check_byte_string(v)?;
    }
    let parsed = parse_fetch_url(url)?;
    let mut map = HeaderMap::new();
    for (name, v) in headers {
        map.append(HeaderName::from_static(name), header_value(v)?);
    }
    Ok((parsed, map))
}

/// `await fetch(url, { method: "POST", headers, body })`. Every transport failure maps to
/// `"fetch failed"` like undici.
pub(crate) async fn post(
    http: &reqwest::Client,
    url: &str,
    headers: &[Header],
    body: impl Into<reqwest::Body>,
) -> Result<Response, Thrown> {
    let (parsed, map) = build_headers(url, headers)?;
    http.post(parsed)
        .headers(map)
        .body(body)
        .send()
        .await
        .map_err(|_| Thrown::new(FETCH_FAILED))
}

/// `await fetch(url)` (GET, used for the desktop quota relay).
pub(crate) async fn get(http: &reqwest::Client, url: &str) -> Result<Response, Thrown> {
    let (parsed, map) = build_headers(url, &[])?;
    http.get(parsed)
        .headers(map)
        .send()
        .await
        .map_err(|_| Thrown::new(FETCH_FAILED))
}

/// `JSON.stringify(v)` (V8 key order, `Number#toString` numbers), byte-identical to the upstream
/// request bodies V1 sent, including the verbatim client `messages`.
pub(crate) fn json_body(v: &Value) -> String {
    crate::aux::js_json::stringify(v)
}

/// `response.status` and `response.statusText` of an upstream response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusLine {
    /// Numeric status.
    pub(crate) code: u16,
    /// Reason phrase as received (HTTP/1.1), else the canonical one (HTTP/2 has none).
    pub(crate) text: String,
}

impl StatusLine {
    /// Status line of a response. hyper keeps the received reason phrase whenever it differs
    /// from the canonical one, which is what undici exposes as `statusText`.
    pub(crate) fn of(resp: &Response) -> Self {
        let status = resp.status();
        let text = match resp.extensions().get::<hyper::ext::ReasonPhrase>() {
            Some(reason) => reason.as_bytes().iter().map(|&b| char::from(b)).collect(),
            None => status.canonical_reason().unwrap_or("").to_owned(),
        };
        Self {
            code: status.as_u16(),
            text,
        }
    }

    /// Status line with the canonical reason phrase.
    #[cfg(test)]
    pub(crate) fn canonical(code: u16) -> Self {
        let text = StatusCode::from_u16(code)
            .ok()
            .and_then(|s| s.canonical_reason())
            .unwrap_or("")
            .to_owned();
        Self { code, text }
    }
}

/// `response.body !== null` (null-body statuses per the Fetch standard).
pub(crate) fn has_body(status: StatusCode) -> bool {
    !matches!(status.as_u16(), 101 | 103 | 204 | 205 | 304)
}

/// `await response.json().catch(() => null)`.
pub(crate) async fn read_json_or_null(resp: Response) -> Option<Value> {
    let bytes = resp.bytes().await.ok()?;
    parse_json_body(&bytes)
}

/// `await response.text().catch(() => "")`.
pub(crate) async fn read_text_or_empty(resp: Response) -> String {
    match resp.bytes().await {
        Ok(bytes) => decode_text(&bytes),
        Err(_) => String::new(),
    }
}

/// `await response.json()` without a catch: body errors throw `terminated`, invalid JSON throws
/// V8's `SyntaxError` message.
pub(crate) async fn read_json_strict(resp: Response) -> Result<Value, Thrown> {
    let bytes = resp.bytes().await.map_err(|_| Thrown::new(TERMINATED))?;
    let text = decode_text(&bytes);
    serde_json::from_str(&text).map_err(|e| Thrown::new(js_json_parse_error(&text, &e)))
}

/// UTF-8 body text with replacement characters and a leading BOM removed (`utf8DecodeBytes`).
pub(crate) fn decode_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    text.strip_prefix('\u{FEFF}').unwrap_or(&text).to_owned()
}

/// Next body chunk of a streaming response; read failures are undici's `terminated`.
pub(crate) async fn next_chunk(resp: &mut Response) -> Result<Option<Bytes>, Thrown> {
    resp.chunk().await.map_err(|_| Thrown::new(TERMINATED))
}

/// `JSON.stringify(parsedBodyOrNull)` with JS semantics (upstream error bodies re-serialized by
/// V1 keep V8's key order and number formatting).
pub(crate) fn json_stringify(v: Option<&Value>) -> String {
    v.map_or_else(|| "null".to_owned(), crate::aux::js_json::stringify)
}

/// E_NS (`/chat`): `` `${status} ${statusText}\n${JSON.stringify(body)}` ``.
pub(crate) fn e_ns(status: &StatusLine, body: Option<&Value>) -> String {
    format!("{} {}\n{}", status.code, status.text, json_stringify(body))
}

/// E_S (`/chat/stream`): `` `${status} ${statusText}: ${JSON.stringify(json)}` `` with every LF
/// replaced by a space.
pub(crate) fn e_s(status: &StatusLine, body: Option<&Value>) -> String {
    format!("{} {}: {}", status.code, status.text, json_stringify(body)).replace('\n', " ")
}

/// Why `new Response(body, { status })` would throw for an upstream status passed through.
pub(crate) fn response_status_error(status: u16) -> Option<String> {
    if !(200..=599).contains(&status) {
        return Some("init[\"status\"] must be in the range of 200 to 599, inclusive.".to_owned());
    }
    if matches!(status, 101 | 103 | 204 | 205 | 304) {
        return Some(format!(
            "Response constructor: Invalid response status code {status}"
        ));
    }
    None
}

/// `NextResponse.json({ error }, { status })` inside the `/chat` try block: an invalid status
/// throws, which the catch turns into `500 {"error": message}`.
pub(crate) fn error_reply(status: u16, error: Value) -> ApiReply {
    match response_status_error(status) {
        Some(msg) => ApiReply::error(500, msg),
        None => ApiReply::json(status, json!({ "error": error })),
    }
}

/// Approximation of V8's `JSON.parse` `SyntaxError` messages (only reachable when a token
/// endpoint answers with non-JSON).
pub(crate) fn js_json_parse_error(text: &str, err: &serde_json::Error) -> String {
    const EOS: &str = "Unexpected end of JSON input";
    let chars: Vec<char> = text.chars().collect();
    if err.is_eof() || chars.iter().all(|c| matches!(c, ' ' | '\t' | '\n' | '\r')) {
        return EOS.to_owned();
    }
    // serde_json reports a 1-based line and column; convert to a char index.
    let mut line = 1usize;
    let mut pos = 0usize;
    while line < err.line() && pos < chars.len() {
        if chars[pos] == '\n' {
            line += 1;
        }
        pos += 1;
    }
    let pos = (pos + err.column().saturating_sub(1)).min(chars.len().saturating_sub(1));
    let token = chars[pos];
    let slice = |a: usize, b: usize| chars[a..b.min(chars.len())].iter().collect::<String>();
    let len = chars.len();
    const CONTEXT: usize = 10;
    if len <= 2 * CONTEXT {
        format!("Unexpected token '{token}', \"{text}\" is not valid JSON")
    } else if pos < CONTEXT {
        format!(
            "Unexpected token '{token}', \"{}\"... is not valid JSON",
            slice(0, pos + CONTEXT)
        )
    } else if pos < len - CONTEXT {
        format!(
            "Unexpected token '{token}', ...\"{}\"... is not valid JSON",
            slice(pos - CONTEXT, pos + CONTEXT)
        )
    } else {
        format!(
            "Unexpected token '{token}', ...\"{}\" is not valid JSON",
            slice(pos - CONTEXT, len)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_errors_match_undici() {
        assert_eq!(
            parse_fetch_url("/chat/completions").err(),
            Some(Thrown::new("Failed to parse URL from /chat/completions"))
        );
        assert_eq!(
            parse_fetch_url("http://a b/x").err(),
            Some(Thrown::new("Failed to parse URL from http://a b/x"))
        );
        assert_eq!(
            parse_fetch_url("http://u:p@127.0.0.1:5/x").err(),
            Some(Thrown::new("Request cannot be constructed from a URL that includes credentials: http://u:p@127.0.0.1:5/x"))
        );
        assert!(parse_fetch_url("http://localhost:11434//api/chat").is_ok());
    }

    #[test]
    fn header_values_follow_undici() {
        assert_eq!(
            header_value("Bearer   abc  ").ok(),
            HeaderValue::from_str("Bearer   abc").ok()
        );
        assert_eq!(
            header_value("Bearer a\nb").err(),
            Some(Thrown::new(
                "Headers.append: \"Bearer a\nb\" is an invalid header value."
            ))
        );
        assert_eq!(
            header_value("abcé").ok().map(|v| v.as_bytes().to_vec()),
            Some(b"abc\xe9".to_vec())
        );
        assert_eq!(
            check_byte_string("abcあ").err(),
            Some(Thrown::new(
                "Cannot convert argument to a ByteString because the character at index 3 has a value of 12354 which is greater than 255."
            ))
        );
        assert!(header_value("a\tb").is_ok());
    }

    #[test]
    fn error_formats_golden() {
        assert_eq!(
            e_ns(&StatusLine::canonical(400), None),
            "400 Bad Request\nnull"
        );
        let body = json!({"error": {"code": 400, "message": "x\ny"}});
        assert_eq!(
            e_s(&StatusLine::canonical(400), Some(&body)),
            r#"400 Bad Request: {"error":{"code":400,"message":"x\ny"}}"#
        );
        assert_eq!(
            e_s(&StatusLine::canonical(429), Some(&Value::Null)),
            "429 Too Many Requests: null"
        );
        assert_eq!(e_ns(&StatusLine::canonical(529), None), "529 \nnull");
        let custom = StatusLine {
            code: 529,
            text: "Overloaded".into(),
        };
        assert_eq!(e_ns(&custom, None), "529 Overloaded\nnull");
    }

    #[test]
    fn response_status_validation() {
        assert_eq!(response_status_error(429), None);
        assert_eq!(response_status_error(300), None);
        assert_eq!(
            response_status_error(304),
            Some("Response constructor: Invalid response status code 304".to_owned())
        );
        assert_eq!(
            response_status_error(600),
            Some("init[\"status\"] must be in the range of 200 to 599, inclusive.".to_owned())
        );
        assert_eq!(
            error_reply(304, json!("x")),
            ApiReply::error(
                500,
                "Response constructor: Invalid response status code 304"
            )
        );
    }

    #[test]
    fn json_parse_errors_approximate_v8() {
        let msg = |s: &str| match serde_json::from_str::<Value>(s) {
            Err(e) => js_json_parse_error(s, &e),
            Ok(_) => String::new(),
        };
        assert_eq!(msg(""), "Unexpected end of JSON input");
        assert_eq!(msg("<"), "Unexpected token '<', \"<\" is not valid JSON");
        assert_eq!(
            msg("<abcdefghijklmnopqrs"),
            "Unexpected token '<', \"<abcdefghijklmnopqrs\" is not valid JSON"
        );
        assert_eq!(
            msg("<!DOCTYPE html><html>"),
            "Unexpected token '<', \"<!DOCTYPE \"... is not valid JSON"
        );
        assert_eq!(
            msg("<html>\n<body>\n</body>\n</html>\n<!-- padding padding -->"),
            "Unexpected token '<', \"<html>\n<bo\"... is not valid JSON"
        );
        assert_eq!(msg("{\"a\":1"), "Unexpected end of JSON input");
    }

    #[test]
    fn json_serialization_follows_json_stringify() {
        // `JSON.stringify(JSON.parse(raw))` in Node: array-index keys first, `1.0` → `1`.
        let parsed = parse_json_body(br#"{"model":"m","2":"b","1":"a","n":1.0,"t":0.85}"#);
        assert_eq!(
            json_stringify(parsed.as_ref()),
            r#"{"1":"a","2":"b","model":"m","n":1,"t":0.85}"#
        );
        assert_eq!(
            json_body(&json!({"messages": [{"content": "あ\n", "role": "user"}]})),
            r#"{"messages":[{"content":"あ\n","role":"user"}]}"#
        );
        assert_eq!(json_stringify(None), "null");
        let body = parse_json_body(br#"{"error":{"code":400.0,"details":{"10":1,"9":2}}}"#);
        assert_eq!(
            e_ns(&StatusLine::canonical(400), body.as_ref()),
            "400 Bad Request\n{\"error\":{\"code\":400,\"details\":{\"9\":2,\"10\":1}}}"
        );
    }

    #[test]
    fn text_decoding_strips_bom() {
        assert_eq!(decode_text("\u{FEFF}{}".as_bytes()), "{}");
        assert_eq!(decode_text(b"a\xffb"), "a\u{FFFD}b");
    }
}
