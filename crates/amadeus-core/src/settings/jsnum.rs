//! ECMAScript number semantics used by the settings store and the other ported V1 logic:
//! `Number.parseInt(s, 10)`, `Number.parseFloat(s)`, `String(number)`, `Math.trunc` and
//! `Number#toFixed`.
//!
//! [`crate::jscompat`] (a shared contract) has no number *parsing*; its `String(x)` helpers
//! (`js_string`, `js_number_to_string(&serde_json::Number)`) delegate to the formatting here.
//! The functions here are exact (formatting is done by `ryu-js`, which implements ECMAScript
//! `Number::toString`).

use serde_json::Value;

use crate::jscompat::js_trim_start;

/// `Number.parseInt(s, 10)`. `None` stands for `NaN`.
///
/// Skips leading JS whitespace (incl. U+FEFF, excl. U+0085), accepts one optional sign, then
/// the longest run of ASCII digits (at least one). Huge values are rounded to the nearest
/// `f64` like V8 (`"99999999999999999999"` → `1e20`, 400 digits → `Infinity`); `"-0"` → `-0`.
pub fn js_parse_int(s: &str) -> Option<f64> {
    let t = js_trim_start(s);
    let (negative, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let magnitude: f64 = rest[..digits].parse().ok()?;
    Some(if negative { -magnitude } else { magnitude })
}

/// `Number.parseFloat(s)`. `None` stands for `NaN`; `Some(±inf)` is possible
/// (`"Infinity"`, `"1e400"`).
///
/// Grammar of the accepted prefix (after leading JS whitespace):
/// `[+-]? ( "Infinity" | ( [0-9]+ ("." [0-9]*)? | "." [0-9]+ ) ([eE] [+-]? [0-9]+)? )`.
pub fn js_parse_float(s: &str) -> Option<f64> {
    let t = js_trim_start(s);
    let b = t.as_bytes();
    let mut i = 0;
    let negative = match b.first() {
        Some(b'-') => {
            i = 1;
            true
        }
        Some(b'+') => {
            i = 1;
            false
        }
        _ => false,
    };
    if t[i..].starts_with("Infinity") {
        return Some(if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    let int_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let int_part = &t[int_start..i];
    let mut frac_part = "";
    if i < b.len() && b[i] == b'.' {
        let frac_start = i + 1;
        let mut j = frac_start;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        // A lone "." (no digit on either side) is not part of a number.
        if !int_part.is_empty() || j > frac_start {
            frac_part = &t[frac_start..j];
            i = j;
        }
    }
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    let mut exp_part = "";
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let digits_start = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > digits_start {
            exp_part = &t[i + 1..j];
        }
    }
    // Canonical form accepted by Rust's correctly rounding parser.
    let mut canonical =
        String::with_capacity(int_part.len() + frac_part.len() + exp_part.len() + 6);
    if negative {
        canonical.push('-');
    }
    canonical.push_str(if int_part.is_empty() { "0" } else { int_part });
    canonical.push('.');
    canonical.push_str(if frac_part.is_empty() { "0" } else { frac_part });
    if !exp_part.is_empty() {
        canonical.push('e');
        canonical.push_str(exp_part);
    }
    canonical.parse().ok()
}

/// `String(n)` — ECMAScript `Number::toString(n)` (`1` → `"1"`, `0.8` → `"0.8"`,
/// `1e21` → `"1e+21"`, `1e-7` → `"1e-7"`, `-0` → `"0"`, `NaN` → `"NaN"`, `∞` → `"Infinity"`).
pub fn js_number_to_string(n: f64) -> String {
    ryu_js::Buffer::new().format(n).to_owned()
}

/// `String(Math.trunc(n))`, the value V1 `AppSettings.setInt` writes.
pub fn js_trunc_to_string(n: f64) -> String {
    js_number_to_string(n.trunc())
}

/// `n.toFixed(digits)` — ECMAScript `Number#toFixed` (exact decimal value, ties away from
/// zero, `|n| >= 1e21` → `String(n)`). `digits` is clamped to 100 like JS' upper bound.
pub fn js_to_fixed(n: f64, digits: u8) -> String {
    ryu_js::Buffer::new().format_to_fixed(n, digits).to_owned()
}

/// Converts a JS integer-valued number (result of `parseInt`, `Math.trunc`, …) to `i64`,
/// saturating at the `i64` bounds (`NaN` → 0). Values beyond ±2^63 cannot occur in V1 data
/// except through hand-edited storage.
pub fn js_number_to_i64(n: f64) -> i64 {
    // `as` saturates and maps NaN to 0, which is exactly what we want here.
    n as i64
}

/// `String(value)` for a JSON value with exact JS number formatting: strings verbatim,
/// numbers via [`js_number_to_string`], `true`/`false`, `null` → `"null"`, objects →
/// `"[object Object]"`, arrays → elements joined with `","` (null elements become `""`).
pub fn js_value_to_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n
            .as_f64()
            .map(js_number_to_string)
            .unwrap_or_else(|| n.to_string()),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                if item.is_null() {
                    String::new()
                } else {
                    js_value_to_string(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// `String(value)` for a value produced by `JSON.parse`, or `None` where JS throws a
/// `TypeError` ("Cannot convert object to primitive value"). That happens for an object
/// with an own `toString` key: a JSON value is never callable, and the inherited `valueOf`
/// returns the object itself. It also happens when such an object is inside an array,
/// because `Array#join` converts every element. Otherwise the result equals
/// [`js_value_to_string`].
pub fn js_value_to_string_checked(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) if map.contains_key("toString") => None,
        Value::Array(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                if item.is_null() {
                    parts.push(String::new());
                } else {
                    parts.push(js_value_to_string_checked(item)?);
                }
            }
            Some(parts.join(","))
        }
        other => Some(js_value_to_string(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fmt(v: Option<f64>) -> String {
        match v {
            None => "NaN".to_owned(),
            Some(x) if x == 0.0 && x.is_sign_negative() => "-0".to_owned(),
            Some(x) => js_number_to_string(x),
        }
    }

    /// Golden values: V1 `Number.parseInt(s, 10)` under Node 22.
    #[test]
    fn parse_int_matches_v8() {
        let cases = [
            ("1", "1"),
            (" 2 ", "2"),
            ("1.9", "1"),
            ("3abc", "3"),
            ("1e3", "1"),
            ("0x10", "0"),
            ("", "NaN"),
            ("abc", "NaN"),
            ("-0", "-0"),
            ("+5", "5"),
            ("-12px", "-12"),
            ("\u{3000} 12", "12"),
            ("\u{FEFF}7", "7"),
            ("\u{0085}7", "NaN"),
            ("99999999999999999999", "100000000000000000000"),
            ("Infinity", "NaN"),
            ("- 1", "NaN"),
            ("+-1", "NaN"),
            ("007", "7"),
            ("\n\t-3", "-3"),
            ("\u{0663}", "NaN"),
        ];
        for (input, expected) in cases {
            assert_eq!(fmt(js_parse_int(input)), expected, "parseInt({input:?})");
        }
        assert_eq!(js_parse_int(&"9".repeat(400)), Some(f64::INFINITY));
    }

    /// Golden values: V1 `Number.parseFloat(s)` under Node 22.
    #[test]
    fn parse_float_matches_v8() {
        let cases = [
            ("0.8", "0.8"),
            (".5", "0.5"),
            ("5.", "5"),
            ("1e", "1"),
            ("0.8abc", "0.8"),
            ("Infinity", "Infinity"),
            ("-Infinity", "-Infinity"),
            ("+Infinity", "Infinity"),
            ("Infinityx", "Infinity"),
            ("1e400", "Infinity"),
            ("-1e400", "-Infinity"),
            (".", "NaN"),
            ("-.5e-3abc", "-0.0005"),
            ("1.e5", "100000"),
            ("1e+", "1"),
            ("1e-7", "1e-7"),
            ("0x1A", "0"),
            ("", "NaN"),
            ("abc", "NaN"),
            (" \n 3.25 ", "3.25"),
            ("1.5.3", "1.5"),
            ("1e5e3", "100000"),
            ("+.5", "0.5"),
            ("-0", "-0"),
            ("00.10", "0.1"),
            ("123456789012345678901234567890", "1.2345678901234568e+29"),
            ("4.9e-324", "5e-324"),
            ("1e-400", "0"),
            ("e5", "NaN"),
            ("-", "NaN"),
            ("0.1e+2", "10"),
            ("infinity", "NaN"),
            ("nan", "NaN"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                fmt(js_parse_float(input)),
                expected,
                "parseFloat({input:?})"
            );
        }
    }

    /// Golden values: V1 `String(n)` under Node 22.
    #[test]
    fn number_to_string_matches_v8() {
        let cases: [(f64, &str); 21] = [
            (1.0, "1"),
            (0.8, "0.8"),
            (3.0, "3"),
            (0.7, "0.7"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e21, "1e+21"),
            (1e-7, "1e-7"),
            (-0.0, "0"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
            (1.2000000000000002, "1.2000000000000002"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e-6, "0.000001"),
            (0.000001234, "0.000001234"),
            (5e-324, "5e-324"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (1.5, "1.5"),
            (-2.5, "-2.5"),
            (9007199254740992.0, "9007199254740992"),
            (12345.678, "12345.678"),
        ];
        for (n, expected) in cases {
            assert_eq!(js_number_to_string(n), expected, "String({n:?})");
        }
        assert_eq!(js_trunc_to_string(1.9), "1");
        assert_eq!(js_trunc_to_string(-1.9), "-1");
        assert_eq!(js_trunc_to_string(-0.5), "0");
        assert_eq!(js_trunc_to_string(1e21 + 0.5), "1e+21");
        assert_eq!(js_trunc_to_string(2.5), "2");
    }

    /// Golden values: V1 `n.toFixed(2)` / `n.toFixed(0)` under Node 22.
    #[test]
    fn to_fixed_matches_v8() {
        let cases: [(f64, &str, &str); 13] = [
            (0.125, "0.13", "0"),
            (1.005, "1.00", "1"),
            (0.7, "0.70", "1"),
            (-0.0, "0.00", "0"),
            (1e21, "1e+21", "1e+21"),
            (-1.5, "-1.50", "-2"),
            (2.5, "2.50", "3"),
            (-0.0001, "-0.00", "-0"),
            (0.6999999999999998, "0.70", "1"),
            (0.705, "0.70", "1"),
            (123.456, "123.46", "123"),
            (1.45, "1.45", "1"),
            (8.345, "8.35", "8"),
        ];
        for (n, two, zero) in cases {
            assert_eq!(js_to_fixed(n, 2), two, "({n:?}).toFixed(2)");
            assert_eq!(js_to_fixed(n, 0), zero, "({n:?}).toFixed(0)");
        }
    }

    #[test]
    fn value_to_string_matches_js() {
        assert_eq!(js_value_to_string(&json!(3)), "3");
        assert_eq!(js_value_to_string(&json!(1.5)), "1.5");
        assert_eq!(js_value_to_string(&json!(1e21)), "1e+21");
        assert_eq!(js_value_to_string(&json!(null)), "null");
        assert_eq!(js_value_to_string(&json!(true)), "true");
        assert_eq!(js_value_to_string(&json!([1, 2])), "1,2");
        assert_eq!(js_value_to_string(&json!([1, null, [2, 3]])), "1,,2,3");
        assert_eq!(js_value_to_string(&json!({"k": 1})), "[object Object]");
        assert_eq!(
            js_value_to_string(&json!(12345678901234567890u64)),
            "12345678901234567000"
        );
        assert_eq!(js_number_to_i64(1e20), i64::MAX);
        assert_eq!(js_number_to_i64(-0.0), 0);
    }

    /// Golden values: V1 `String(JSON.parse(x))` under Node 22 (`None` = `TypeError`).
    #[test]
    fn value_to_string_checked_matches_js() {
        let cases: [(&str, Option<&str>); 8] = [
            (r#"{"toString":1}"#, None),
            (r#"{"toString":null}"#, None),
            (r#"[1,[{"toString":"x"}]]"#, None),
            (r#"{"valueOf":1}"#, Some("[object Object]")),
            (r#"[1,null,{"a":1},[2,3]]"#, Some("1,,[object Object],2,3")),
            ("100499.99999999999", Some("100499.99999999999")),
            ("0.9095702657941729", Some("0.9095702657941729")),
            ("-0", Some("0")),
        ];
        for (json, expected) in cases {
            let value: Value = serde_json::from_str(json).unwrap_or(Value::Null);
            assert_eq!(
                js_value_to_string_checked(&value).as_deref(),
                expected,
                "String({json})"
            );
        }
    }
}
