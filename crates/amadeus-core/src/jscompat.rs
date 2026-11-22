//! Helpers that reproduce JavaScript semantics V1 relied on (`String.prototype.trim`,
//! truthiness, `String(x)`), so ported logic behaves byte-for-byte like V1.
//!
//! `String(x)` of JSON values delegates to [`crate::settings::jsnum`], the one implementation
//! of ECMAScript number formatting (`ryu-js`), so every caller prints numbers like JS.

use serde_json::Value;

use crate::settings::jsnum;

/// 2^53: every integer up to this magnitude is an exact `f64`, so its decimal digits are
/// exactly what `String(x)` prints.
const EXACT_INTEGER_LIMIT: u64 = 1 << 53;

/// Whitespace as defined by ECMAScript `String.prototype.trim` (WhiteSpace + LineTerminator).
/// Differs from Rust's `char::is_whitespace`: includes U+FEFF, excludes U+0085.
pub fn js_is_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

/// `s.trim()` with JS semantics.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(js_is_whitespace)
}

/// `s.trimStart()` with JS semantics.
pub fn js_trim_start(s: &str) -> &str {
    s.trim_start_matches(js_is_whitespace)
}

/// `s.trimEnd()` with JS semantics.
pub fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(js_is_whitespace)
}

/// JS truthiness of a JSON value (`Boolean(x)`).
pub fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0 && !f.is_nan()).unwrap_or(true),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// JS `String(x)` (ECMAScript `Number::toString`) for a `serde_json` number: `8.0` → `"8"`,
/// `1e21` → `"1e+21"`, `1.5e-7` → `"1.5e-7"`, `-0.0` → `"0"`. Integers up to 2^53 in magnitude
/// take a fast path. Every other value is rounded to the nearest `f64`, as `JSON.parse` does
/// (`9007199254740993` → `"9007199254740992"`), and formatted by
/// [`jsnum::js_number_to_string`]. JSON has no `NaN` or `Infinity`, so they never reach here.
pub fn js_number_to_string(n: &serde_json::Number) -> String {
    if let Some(i) = n
        .as_i64()
        .filter(|i| i.unsigned_abs() <= EXACT_INTEGER_LIMIT)
    {
        return i.to_string();
    }
    n.as_f64()
        .map(jsnum::js_number_to_string)
        .unwrap_or_else(|| n.to_string())
}

/// `String(x)` for a JSON value: strings verbatim, numbers as [`js_number_to_string`],
/// `null` → `"null"`, objects → `"[object Object]"`, arrays joined with `","` (`null` elements
/// become `""`). Everything except a top-level number is [`jsnum::js_value_to_string`].
pub fn js_string(v: &Value) -> String {
    match v {
        Value::Number(n) => js_number_to_string(n),
        other => jsnum::js_value_to_string(other),
    }
}

/// Length in UTF-16 code units (JS `string.length`).
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn trim_matches_js() {
        assert_eq!(js_trim("\u{FEFF} a \u{3000}"), "a");
        assert_eq!(js_trim("\u{0085}a"), "\u{0085}a");
        assert_eq!(js_trim_start("  a "), "a ");
        assert_eq!(js_trim_end("  a "), "  a");
    }

    #[test]
    fn truthiness() {
        assert!(!js_truthy(&json!(null)));
        assert!(!js_truthy(&json!(0)));
        assert!(!js_truthy(&json!("")));
        assert!(js_truthy(&json!(" ")));
        assert!(js_truthy(&json!([])));
        assert!(js_truthy(&json!({})));
        assert!(js_truthy(&json!(1.5)));
    }

    #[test]
    fn string_conversion() {
        assert_eq!(js_string(&json!(8.0)), "8");
        assert_eq!(js_string(&json!(1.5)), "1.5");
        assert_eq!(js_string(&json!([1, null, "a"])), "1,,a");
        assert_eq!(js_string(&json!({"a": 1})), "[object Object]");
        assert_eq!(js_string(&json!(null)), "null");
        assert_eq!(js_string(&json!(false)), "false");
        assert_eq!(js_string(&json!("1e21")), "1e21");
        assert_eq!(
            js_string(&json!([1e21, null, -0.0, [1e-7, 2], "a"])),
            "1e+21,,0,1e-7,2,a"
        );
        assert_eq!(utf16_len("あ😀"), 3);
    }

    /// Golden values: Node 22 `String(JSON.parse(text))`.
    #[test]
    fn numbers_print_like_js_string() {
        let cases = [
            ("0", "0"),
            ("-0", "0"),
            ("-0.0", "0"),
            ("8.0", "8"),
            ("123.5", "123.5"),
            ("0.30000000000000004", "0.30000000000000004"),
            ("1e20", "100000000000000000000"),
            ("999999999999999900000", "999999999999999900000"),
            ("1e21", "1e+21"),
            ("-1e21", "-1e+21"),
            ("1e300", "1e+300"),
            ("1.7976931348623157e308", "1.7976931348623157e+308"),
            ("1e-6", "0.000001"),
            ("1e-7", "1e-7"),
            ("1.5e-7", "1.5e-7"),
            ("-1.5e-7", "-1.5e-7"),
            ("5e-324", "5e-324"),
            // Integers: exact up to 2^53, then rounded to the nearest f64 like JSON.parse.
            ("9007199254740991", "9007199254740991"),
            ("9007199254740992", "9007199254740992"),
            ("-9007199254740992", "-9007199254740992"),
            ("9007199254740993", "9007199254740992"),
            ("12345678901234567890", "12345678901234567000"),
            ("18446744073709551615", "18446744073709552000"),
            ("9223372036854775807", "9223372036854776000"),
            ("-9223372036854775808", "-9223372036854776000"),
        ];
        for (text, expected) in cases {
            let value: Value = serde_json::from_str(text).unwrap();
            let Value::Number(number) = &value else {
                panic!("{text} is not a number");
            };
            assert_eq!(js_number_to_string(number), expected, "String({text})");
            assert_eq!(js_string(&value), expected, "String({text})");
            assert_eq!(
                js_string(&value),
                jsnum::js_value_to_string(&value),
                "{text}"
            );
        }
        // The values from the JSC-001 audit, built with `json!` (f64-backed numbers).
        assert_eq!(js_string(&json!(1e21)), "1e+21");
        assert_eq!(js_string(&json!(1.5e-7)), "1.5e-7");
        assert_eq!(js_string(&json!(1e-7)), "1e-7");
        assert_eq!(js_string(&json!(-0.0)), "0");
        assert_eq!(js_string(&json!(1e300)), "1e+300");
        assert_eq!(js_string(&json!(123.5)), "123.5");
        assert_eq!(js_string(&json!(u64::MAX)), "18446744073709552000");
        assert_eq!(js_string(&json!(i64::MIN)), "-9223372036854776000");
        // JSON has no NaN / Infinity: serde_json cannot hold them (`json!` turns them into
        // `null`), so `String(NaN)` / `String(Infinity)` cannot arise from these helpers.
        for f in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(serde_json::Number::from_f64(f).is_none(), "{f}");
        }
    }
}
