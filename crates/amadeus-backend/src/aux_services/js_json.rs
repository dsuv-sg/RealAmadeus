//! `JSON.stringify`-compatible serialization of [`serde_json::Value`].
//!
//! Every JSON body V1 emitted went through a JS object and `JSON.stringify`, which differs from
//! `serde_json::to_string` in two observable ways:
//!
//! * **Numbers** use `Number.prototype.toString` (`1.0` → `1`, `1.2e-5` → `0.000012`,
//!   `1e21` → `1e+21`, integers beyond 2^53 lose precision like a JS double);
//! * **Key order**: V8 enumerates array-index keys (`"0"`, `"10"`, …) first in ascending
//!   numeric order, then the remaining keys in insertion order.
//!
//! String escaping is identical to `JSON.stringify` (short escapes for `\b \f \n \r \t`,
//! `\u00xx` for other control characters, everything else raw). Requires `serde_json`'s
//! `preserve_order` feature for insertion order (enabled by this crate).

use std::fmt::Write as _;

use serde_json::{Number, Value};

use super::js::array_index;

/// Largest integer magnitude a JS double represents exactly (2^53).
const MAX_SAFE_MAGNITUDE: u64 = 1 << 53;

/// `JSON.stringify(value)`.
pub fn stringify(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value);
    out
}

/// `JSON.stringify(text)` for a plain string.
pub fn stringify_str(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    write_string(&mut out, text);
    out
}

/// Appends `JSON.stringify(value)` to `out`.
pub fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => write_number(out, n),
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut index_keys: Vec<(u32, &String, &Value)> = map
                .iter()
                .filter_map(|(k, v)| array_index(k).map(|i| (i, k, v)))
                .collect();
            index_keys.sort_by_key(|(i, _, _)| *i);
            let others = map.iter().filter(|(k, _)| array_index(k).is_none());
            out.push('{');
            let mut first = true;
            for (key, v) in index_keys.into_iter().map(|(_, k, v)| (k, v)).chain(others) {
                if !first {
                    out.push(',');
                }
                first = false;
                write_string(out, key);
                out.push(':');
                write_value(out, v);
            }
            out.push('}');
        }
    }
}

/// JSON string literal exactly as `JSON.stringify` escapes it.
pub fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_number(out: &mut String, n: &Number) {
    if let Some(u) = n.as_u64() {
        if u <= MAX_SAFE_MAGNITUDE {
            let _ = write!(out, "{u}");
        } else {
            // JSON.parse would have rounded it to the nearest double.
            write_js_double(out, u as f64);
        }
    } else if let Some(i) = n.as_i64() {
        if i.unsigned_abs() <= MAX_SAFE_MAGNITUDE {
            let _ = write!(out, "{i}");
        } else {
            write_js_double(out, i as f64);
        }
    } else if let Some(f) = n.as_f64() {
        write_js_double(out, f);
    } else {
        out.push_str("null");
    }
}

/// `Number.prototype.toString()` (radix 10) for a double, as used by `JSON.stringify`.
/// Non-finite values serialize as `null` like in JS.
pub fn write_js_double(out: &mut String, f: f64) {
    if !f.is_finite() {
        out.push_str("null");
        return;
    }
    if f == 0.0 {
        // Covers -0, which JS prints as "0".
        out.push('0');
        return;
    }
    if f < 0.0 {
        out.push('-');
    }
    // Rust's `{:e}` prints the shortest round-trip digits, like ECMAScript Number::toString.
    let sci = format!("{:e}", f.abs());
    let (mantissa, exponent) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let k = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let n = exponent + 1;
    if k <= n && n <= 21 {
        out.push_str(&digits);
        for _ in 0..(n - k) {
            out.push('0');
        }
    } else if 0 < n && n <= 21 {
        let split = usize::try_from(n).unwrap_or(0).min(digits.len());
        out.push_str(&digits[..split]);
        out.push('.');
        out.push_str(&digits[split..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        for _ in 0..(-n) {
            out.push('0');
        }
        out.push_str(&digits);
    } else {
        let e = n - 1;
        let sign = if e >= 0 { '+' } else { '-' };
        let (head, tail) = digits.split_at(1.min(digits.len()));
        out.push_str(head);
        if !tail.is_empty() {
            out.push('.');
            out.push_str(tail);
        }
        let _ = write!(out, "e{sign}{}", e.unsigned_abs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn js_double(f: f64) -> String {
        let mut s = String::new();
        write_js_double(&mut s, f);
        s
    }

    #[test]
    fn numbers_match_node() {
        // Golden output of Node 22: JSON.stringify(JSON.parse("[1.0, 1e5, 1e-7, 1.2e-5,
        // 123456789012345678901234, 0.1, -0, 1e21, 5e-324, 2e-7, 0.000001]"))
        let parsed: Value = serde_json::from_str(
            "[1.0, 1e5, 1e-7, 1.2e-5, 123456789012345678901234, 0.1, -0, 1e21, 5e-324, 2e-7, 0.000001]",
        )
        .unwrap_or(Value::Null);
        assert_eq!(
            stringify(&parsed),
            "[1,100000,1e-7,0.000012,1.2345678901234569e+23,0.1,0,1e+21,5e-324,2e-7,0.000001]"
        );
        assert_eq!(js_double(123.456), "123.456");
        assert_eq!(js_double(-1.5), "-1.5");
        assert_eq!(js_double(1e20), "100000000000000000000");
        assert_eq!(js_double(1.5e300), "1.5e+300");
        assert_eq!(js_double(f64::NAN), "null");
        assert_eq!(
            stringify(&json!(12_345_678_901_234_567_890_u64)),
            "12345678901234567000"
        );
        assert_eq!(
            stringify(&json!(-9_007_199_254_740_993_i64)),
            "-9007199254740992"
        );
        assert_eq!(stringify(&json!([0.25, -0.5, 1])), "[0.25,-0.5,1]");
    }

    #[test]
    fn key_order_matches_v8() {
        let v: Value = serde_json::from_str(
            r#"{"status":"ok","10":"a","2":"b","available":"x","4294967295":1,"01":2}"#,
        )
        .unwrap_or(Value::Null);
        assert_eq!(
            stringify(&v),
            r#"{"2":"b","10":"a","status":"ok","available":"x","4294967295":1,"01":2}"#
        );
        let dup: Value = serde_json::from_str(r#"{"a":1,"a":2,"b":3}"#).unwrap_or(Value::Null);
        assert_eq!(stringify(&dup), r#"{"a":2,"b":3}"#);
    }

    #[test]
    fn strings_match_json_stringify() {
        assert_eq!(
            stringify_str("\u{0}\u{1f}\u{7f}\u{2028}\u{8}\u{c}\n\r\t\"\\/<>あ"),
            "\"\\u0000\\u001f\u{7f}\u{2028}\\b\\f\\n\\r\\t\\\"\\\\/<>あ\""
        );
        assert_eq!(
            stringify(&json!({"text": "こんにちは", "n": null, "b": true})),
            r#"{"text":"こんにちは","n":null,"b":true}"#
        );
    }
}
