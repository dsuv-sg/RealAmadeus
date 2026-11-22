//! JavaScript / undici semantics the V1 route handlers relied on.
//!
//! V1 ran on Node 22 (undici `fetch`, `Request.json()`, `Response.text()`, optional chaining on
//! arbitrary JSON). These helpers reproduce exactly the observable behavior so the ported
//! handlers return byte-identical results.

use amadeus_core::jscompat::js_trim;
use serde_json::Value;

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// `new TextDecoder().decode(bytes)`: strips ONE leading UTF-8 BOM and replaces invalid
/// sequences with U+FFFD.
pub fn text_decoder_decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    String::from_utf8_lossy(bytes).into_owned()
}

/// undici `utf8DecodeBytes` (used by `Body.text()`, `Body.json()` and multipart text fields):
/// strips one BOM itself and the shared `TextDecoder` strips a second one, so up to TWO leading
/// BOMs disappear. Invalid sequences become U+FFFD.
pub fn utf8_decode_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    text_decoder_decode(bytes)
}

/// Isomorphic (latin1) decoding of header bytes, i.e. how undici/Node expose header values.
pub fn latin1_decode(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

/// `await body.json()` (undici `parseJSONFromBytes`): `JSON.parse(utf8DecodeBytes(bytes))`.
/// `None` when the text is not valid JSON (the V1 `.catch(() => null)` case).
pub fn parse_json_like_undici(bytes: &[u8]) -> Option<Value> {
    serde_json::from_str(&utf8_decode_bytes(bytes)).ok()
}

/// `(await request.json().catch(() => null))`: the body is parsed regardless of `Content-Type`;
/// empty or invalid JSON gives `null`.
pub fn parse_json_lenient(bytes: &[u8]) -> Value {
    parse_json_like_undici(bytes).unwrap_or(Value::Null)
}

/// The JS `TypeError` thrown by `payload?.x?.trim()` when `x` is not a string. V1 throws it
/// before the handler's `try`, which Next.js turns into HTTP 500 with an empty body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimTypeError;

/// Mirrors `payload?.[key]?.trim()`: `Ok(None)` for a missing / `null` field or a non-object
/// payload, `Ok(Some(trimmed))` for a string, `Err` for any other JSON type.
pub fn opt_trimmed(payload: &Value, key: &str) -> Result<Option<String>, TrimTypeError> {
    match payload.as_object().and_then(|o| o.get(key)) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(js_trim(s).to_owned())),
        Some(_) => Err(TrimTypeError),
    }
}

/// `v[key]` for a JSON value with JS property semantics: object property, array element for
/// canonical array-index keys, `None` (undefined) for everything else. String indexing is not
/// modelled because callers only read named properties afterwards (which are `undefined` on a
/// one-character string anyway).
pub fn js_member<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(map) => map.get(key),
        Value::Array(items) => array_index(key).and_then(|i| items.get(usize::try_from(i).ok()?)),
        _ => None,
    }
}

/// `v?.[index]` with JS semantics (arrays by position, objects by the property `"index"`).
pub fn js_index(v: &Value, index: usize) -> Option<&Value> {
    match v {
        Value::Array(items) => items.get(index),
        Value::Object(map) => map.get(&index.to_string()),
        _ => None,
    }
}

/// `x ?? fallback` for an optional JSON value: `None` and `null` select the fallback.
pub fn non_nullish(v: Option<&Value>) -> Option<&Value> {
    v.filter(|v| !v.is_null())
}

/// ECMAScript "array index": a canonical numeric string for an integer in `0..=2^32 - 2`.
/// V8 enumerates such keys first (ascending), before all other keys in insertion order.
pub fn array_index(key: &str) -> Option<u32> {
    let bytes = key.as_bytes();
    if bytes.is_empty() || bytes.len() > 10 || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }
    if bytes.len() > 1 && bytes[0] == b'0' {
        return None;
    }
    let value: u64 = key.parse().ok()?;
    if value <= u64::from(u32::MAX - 1) {
        u32::try_from(value).ok()
    } else {
        None
    }
}

/// undici FormData serialization of string values: `value.replace(/\r?\n|\r/g, "\r\n")`.
pub fn normalize_linefeeds(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str("\r\n");
            }
            '\n' => out.push_str("\r\n"),
            c => out.push(c),
        }
    }
    out
}

/// `new Blob([], { type }).type`: empty when any character is outside U+0020..=U+007E,
/// otherwise lower-cased.
pub fn blob_type(type_: &str) -> String {
    if type_.chars().any(|c| !(' '..='~').contains(&c)) {
        String::new()
    } else {
        type_.to_ascii_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn utf8_decode_strips_up_to_two_boms() {
        assert_eq!(utf8_decode_bytes(b"\xef\xbb\xbf\xef\xbb\xbfgroq"), "groq");
        assert_eq!(
            utf8_decode_bytes(b"\xef\xbb\xbf\xef\xbb\xbf\xef\xbb\xbfx"),
            "\u{feff}x"
        );
        assert_eq!(
            text_decoder_decode(b"\xef\xbb\xbf\xef\xbb\xbfx"),
            "\u{feff}x"
        );
        assert_eq!(utf8_decode_bytes(b"a\xff\xfeb"), "a\u{fffd}\u{fffd}b");
        assert_eq!(utf8_decode_bytes(b""), "");
    }

    #[test]
    fn lenient_json() {
        assert_eq!(parse_json_lenient(b""), Value::Null);
        assert_eq!(parse_json_lenient(b"not json"), Value::Null);
        assert_eq!(
            parse_json_lenient(b"\xef\xbb\xbf{\"a\":1}"),
            json!({"a": 1})
        );
        assert_eq!(parse_json_lenient(b" [1] "), json!([1]));
    }

    #[test]
    fn opt_trimmed_mirrors_optional_chaining() {
        let p = json!({"a": "  x\u{3000}", "b": null, "c": 5, "d": "   "});
        assert_eq!(opt_trimmed(&p, "a"), Ok(Some("x".to_owned())));
        assert_eq!(opt_trimmed(&p, "b"), Ok(None));
        assert_eq!(opt_trimmed(&p, "missing"), Ok(None));
        assert_eq!(opt_trimmed(&p, "c"), Err(TrimTypeError));
        assert_eq!(opt_trimmed(&p, "d"), Ok(Some(String::new())));
        assert_eq!(opt_trimmed(&json!([1]), "a"), Ok(None));
        assert_eq!(opt_trimmed(&json!("s"), "a"), Ok(None));
        assert_eq!(opt_trimmed(&Value::Null, "a"), Ok(None));
        assert_eq!(opt_trimmed(&json!({"a": {}}), "a"), Err(TrimTypeError));
    }

    #[test]
    fn member_access() {
        let v = json!({"error": "x", "0": "zero"});
        assert_eq!(js_member(&v, "error"), Some(&json!("x")));
        assert_eq!(js_index(&v, 0), Some(&json!("zero")));
        assert_eq!(js_member(&json!(["a"]), "0"), Some(&json!("a")));
        assert_eq!(js_member(&json!(["a"]), "error"), None);
        assert_eq!(js_member(&json!("str"), "error"), None);
        assert_eq!(non_nullish(Some(&Value::Null)), None);
    }

    #[test]
    fn array_index_rules() {
        assert_eq!(array_index("0"), Some(0));
        assert_eq!(array_index("10"), Some(10));
        assert_eq!(array_index("4294967294"), Some(4_294_967_294));
        assert_eq!(array_index("4294967295"), None);
        assert_eq!(array_index("01"), None);
        assert_eq!(array_index("-1"), None);
        assert_eq!(array_index("1.5"), None);
        assert_eq!(array_index(""), None);
        assert_eq!(array_index("99999999999"), None);
    }

    #[test]
    fn linefeeds_and_blob_types() {
        assert_eq!(normalize_linefeeds("a\nb\r\nc\rd"), "a\r\nb\r\nc\r\nd");
        assert_eq!(
            blob_type("Audio/WebM; Codecs=OPUS"),
            "audio/webm; codecs=opus"
        );
        assert_eq!(blob_type("audio/webm;\tx=1"), "");
        assert_eq!(blob_type("a/b\u{7f}"), "");
        assert_eq!(blob_type("audio/\u{e9}"), "");
        assert_eq!(latin1_decode(b"a\xe9"), "a\u{e9}");
    }
}
