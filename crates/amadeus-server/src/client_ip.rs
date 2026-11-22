//! Request header helpers with Node.js semantics.

use amadeus_backend::aux::js::latin1_decode;
use amadeus_core::jscompat::js_trim;
use axum::http::HeaderMap;

/// `request.headers.get(name)` as Next.js saw it: every value latin1-decoded and duplicates
/// joined with `", "` (Node's rule for headers outside its "discard duplicates" list).
pub fn header_joined(headers: &HeaderMap, name: &str) -> Option<String> {
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

/// The first value of a header (Node keeps only the first `Content-Type`, `Authorization`, …).
pub fn header_first(headers: &HeaderMap, name: &str) -> Option<String> {
    headers.get(name).map(|v| latin1_decode(v.as_bytes()))
}

/// V1 `getClientIp`: first non-empty `X-Forwarded-For` entry (trimmed), else the trimmed
/// `X-Real-IP` when the header is non-empty (even if that trims to `""`), else `"unknown"`.
/// The TCP peer address is never used.
pub fn client_ip(headers: &HeaderMap) -> String {
    if let Some(forwarded) = header_joined(headers, "x-forwarded-for").filter(|v| !v.is_empty()) {
        let first = forwarded.split(',').next().map(js_trim).unwrap_or("");
        if !first.is_empty() {
            return first.to_owned();
        }
    }
    if let Some(real_ip) = header_joined(headers, "x-real-ip").filter(|v| !v.is_empty()) {
        return js_trim(&real_ip).to_owned();
    }
    "unknown".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.append(*k, HeaderValue::from_static(v));
        }
        h
    }

    #[test]
    fn client_ip_matches_v1() {
        assert_eq!(
            client_ip(&headers(&[("x-forwarded-for", " 1.2.3.4 , 5.6.7.8")])),
            "1.2.3.4"
        );
        assert_eq!(
            client_ip(&headers(&[
                ("x-forwarded-for", "1.1.1.1"),
                ("x-forwarded-for", "2.2.2.2")
            ])),
            "1.1.1.1"
        );
        assert_eq!(
            client_ip(&headers(&[
                ("x-forwarded-for", ", 1.2.3.4"),
                ("x-real-ip", " 9.9.9.9 ")
            ])),
            "9.9.9.9"
        );
        assert_eq!(
            client_ip(&headers(&[("x-forwarded-for", ", 1.2.3.4")])),
            "unknown"
        );
        assert_eq!(client_ip(&headers(&[("x-real-ip", "   ")])), "");
        assert_eq!(client_ip(&headers(&[])), "unknown");
    }

    #[test]
    fn header_helpers() {
        let h = headers(&[
            ("x-vertex-stream", "true"),
            ("x-vertex-stream", "true"),
            ("content-type", "a/b"),
            ("content-type", "c/d"),
        ]);
        assert_eq!(
            header_joined(&h, "x-vertex-stream").as_deref(),
            Some("true, true")
        );
        assert_eq!(header_first(&h, "content-type").as_deref(), Some("a/b"));
        assert_eq!(header_joined(&h, "missing"), None);
    }
}
