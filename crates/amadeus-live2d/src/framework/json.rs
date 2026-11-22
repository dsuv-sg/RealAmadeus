//! JSON layer — the observable semantics of `CubismJson` / `CubismJsonExtension` on top of
//! `serde_json` (spec §4.4).
//!
//! * Decoding: the deployed V1 bundle uses `new TextDecoder("utf-8").decode(buffer)` → a leading BOM
//!   is stripped and invalid UTF-8 becomes U+FFFD; then `JSON.parse` (a syntax error is an error).
//!   `serde_json` is built with `float_roundtrip` (correctly rounded numbers, like `JSON.parse`) and
//!   `preserve_order` (document order; duplicate keys keep the first position and the last value,
//!   like JS objects).
//! * Accessors take `Option<&Value>` so chains mirror `getValueByString(..).getValueByIndex(..)`:
//!   a missing key, a type mismatch or an out-of-range index all yield `None` (the Framework's shared
//!   null/error values) and the conversion falls back to its default, like `toFloat(d)`.
//!
//! Declared deviations for malformed files only: the Framework's placeholder texts for raw strings
//! of missing values (`"NullValue"`, `"Error: type mismatch"`, …) become `None`/`""`; numbers
//! outside the f64 range and lone surrogate escapes are parse errors instead of `Infinity`/U+FFFD.

use serde_json::{Map, Value};

use crate::core::model::Live2DError;
use crate::math::cubism_math::js_number_to_string;

/// `CubismJson.create(buffer, size)`: UTF-8 decode (BOM stripped, lossy) + `JSON.parse`.
pub fn parse_json(bytes: &[u8], what: &str) -> Result<Value, Live2DError> {
    let text = String::from_utf8_lossy(bytes);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    serde_json::from_str(text).map_err(|e| Live2DError::Load(format!("invalid {what}: {e}")))
}

/// `getValueByString(key)` on a map; `None` for anything else.
#[inline]
pub fn get<'a>(value: Option<&'a Value>, key: &str) -> Option<&'a Value> {
    match value {
        Some(Value::Object(map)) => map.get(key),
        _ => None,
    }
}

/// `getValueByIndex(index)` on an array; `None` for anything else or out of range.
#[inline]
pub fn index(value: Option<&Value>, i: usize) -> Option<&Value> {
    match value {
        Some(Value::Array(array)) => array.get(i),
        _ => None,
    }
}

/// `!isNull() && !isError()`: the value is present and not JSON `null`.
#[inline]
pub fn exists(value: Option<&Value>) -> bool {
    matches!(value, Some(v) if !v.is_null())
}

/// `isNull()` of the Framework value (missing key or JSON `null`); type-mismatch errors are not null.
#[inline]
pub fn is_null(value: Option<&Value>) -> bool {
    matches!(value, None | Some(Value::Null))
}

/// `toFloat(defaultValue)`: the number, or the default for every other type (no string coercion).
#[inline]
pub fn to_float(value: Option<&Value>, default: f64) -> f64 {
    match value {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(default),
        _ => default,
    }
}

/// `toBoolean(defaultValue)`.
#[inline]
pub fn to_bool(value: Option<&Value>, default: bool) -> bool {
    match value {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

/// `toInt(defaultValue)`: `parseInt(value.toString())` for numbers (truncation toward zero; numbers
/// printed in exponent form such as `1e+21` or `5e-7` yield their leading digit), the default
/// otherwise. Returned as a JS number (`f64`, may be `-0`).
pub fn to_int(value: Option<&Value>, default: f64) -> f64 {
    match value {
        Some(Value::Number(n)) => match n.as_f64() {
            Some(v) => js_parse_int_of_number(v),
            None => default,
        },
        _ => default,
    }
}

/// `toInt()` used as a loop bound / array size (`i < n`): negative or NaN → 0.
#[inline]
pub fn to_count(value: Option<&Value>) -> usize {
    let v = to_int(value, 0.0);
    if v.is_nan() || v <= 0.0 {
        0
    } else {
        v as usize
    }
}

/// `parseInt(Number(v).toString())`.
pub fn js_parse_int_of_number(v: f64) -> f64 {
    if !v.is_finite() {
        return f64::NAN; // parseInt("NaN") / parseInt("Infinity")
    }
    if v == 0.0 {
        return 0.0; // (-0).toString() === "0"
    }
    let a = v.abs();
    if !(1e-6..1e21).contains(&a) {
        let s = js_number_to_string(a);
        let digit = s.as_bytes().first().map_or(0, |b| b.wrapping_sub(b'0'));
        let d = f64::from(digit);
        return if v < 0.0 { -d } else { d };
    }
    v.trunc()
}

/// `getRawString()` / `getString()`: strings as-is; booleans `"true"`/`"false"`; numbers `"\0"`
/// (QUIRK of `JsonFloat.getString`); `None` for null/missing/maps/arrays (Framework placeholders).
pub fn raw_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Bool(b)) => Some(if *b { "true" } else { "false" }.to_owned()),
        Some(Value::Number(_)) => Some("\0".to_owned()),
        _ => None,
    }
}

/// [`raw_string`] or `""`.
#[inline]
pub fn raw_string_or_empty(value: Option<&Value>) -> String {
    raw_string(value).unwrap_or_default()
}

/// `getSize()`: array length, map key count, 0 otherwise.
#[inline]
pub fn size(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(a)) => a.len(),
        Some(Value::Object(m)) => m.len(),
        _ => 0,
    }
}

/// `getVector().length`: array length, 0 for everything else.
#[inline]
pub fn vector_len(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(a)) => a.len(),
        _ => 0,
    }
}

/// `true` for keys JS treats as array indices (`Object.keys` orders them first, ascending):
/// canonical decimal integers in `0..=4294967294`.
pub fn is_array_index_key(key: &str) -> bool {
    if key.is_empty() || key.len() > 10 || !key.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if key.len() > 1 && key.starts_with('0') {
        return false;
    }
    key.parse::<u64>().is_ok_and(|n| n <= 4_294_967_294)
}

/// `Object.keys(obj)` order of a parsed map: array-index keys ascending, then the remaining keys
/// in document order.
pub fn js_key_order(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let mut numeric: Vec<(u64, &String, &Value)> = Vec::new();
    let mut other: Vec<(&String, &Value)> = Vec::new();
    for (k, v) in map {
        if is_array_index_key(k) {
            numeric.push((k.parse::<u64>().unwrap_or(0), k, v));
        } else {
            other.push((k, v));
        }
    }
    numeric.sort_by_key(|(n, _, _)| *n);
    numeric
        .into_iter()
        .map(|(_, k, v)| (k, v))
        .chain(other)
        .collect()
}

/// Keys of a map value in `Object.keys` order (`JsonMap.getKeys()`); empty for non-maps.
pub fn js_keys(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Object(map)) => js_key_order(map)
            .into_iter()
            .map(|(k, _)| k.clone())
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_bom_and_lossy() {
        let v = parse_json(b"\xEF\xBB\xBF{\"a\": 1}", "test").unwrap_or(Value::Null);
        assert_eq!(to_float(get(Some(&v), "a"), 0.0), 1.0);
        let v = parse_json(b"{\"s\": \"x\xFFy\"}", "test").unwrap_or(Value::Null);
        assert_eq!(
            raw_string(get(Some(&v), "s")).as_deref(),
            Some("x\u{fffd}y")
        );
        assert!(parse_json(b"{bad", "test").is_err());
    }

    #[test]
    fn accessor_defaults() {
        let v: Value = serde_json::json!({"n": 2.5, "s": "1.5", "b": true, "z": null, "a": [1, 2]});
        let r = Some(&v);
        assert_eq!(to_float(get(r, "n"), 0.0), 2.5);
        assert_eq!(to_float(get(r, "s"), 7.0), 7.0); // no coercion of numeric strings
        assert_eq!(to_float(get(r, "missing"), -1.0), -1.0);
        assert!(to_bool(get(r, "b"), false));
        assert!(!exists(get(r, "z")));
        assert!(is_null(get(r, "z")) && is_null(get(r, "missing")));
        assert!(exists(get(r, "a")));
        assert_eq!(size(get(r, "a")), 2);
        assert_eq!(vector_len(get(r, "n")), 0);
        assert_eq!(raw_string(get(r, "n")).as_deref(), Some("\0"));
        assert_eq!(raw_string(index(get(r, "a"), 5)), None);
        assert_eq!(get(index(Some(&v), 0), "x"), None);
    }

    #[test]
    fn parse_int_quirks() {
        assert_eq!(js_parse_int_of_number(3.7), 3.0);
        assert!(js_parse_int_of_number(-0.5).is_sign_negative());
        assert_eq!(js_parse_int_of_number(1e21), 1.0);
        assert_eq!(js_parse_int_of_number(5e-7), 5.0);
        assert_eq!(js_parse_int_of_number(-2.5e-8), -2.0);
        assert_eq!(js_parse_int_of_number(0.000001), 0.0);
        assert_eq!(to_count(Some(&serde_json::json!(-3))), 0);
        assert_eq!(to_count(Some(&serde_json::json!(12))), 12);
    }

    #[test]
    fn object_keys_order() {
        let v = parse_json(
            br#"{"b": 1, "10": 2, "a": 3, "2": 4, "01": 5, "b": 6}"#,
            "t",
        )
        .unwrap_or(Value::Null);
        assert_eq!(js_keys(Some(&v)), vec!["2", "10", "b", "a", "01"]);
        assert_eq!(to_float(get(Some(&v), "b"), 0.0), 6.0);
        assert!(is_array_index_key("4294967294"));
        assert!(!is_array_index_key("4294967295"));
    }
}
