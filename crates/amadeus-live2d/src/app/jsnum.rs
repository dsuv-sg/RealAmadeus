//! JavaScript number/string semantics the player code relies on (pure, host-testable).
//!
//! `Math.min`/`Math.max` differ from `f64::min`/`f64::max`: they propagate NaN and order signed
//! zeros (`-0 < +0`). V1's driver math uses them everywhere, so the port uses these helpers instead
//! of the Rust intrinsics.

/// `Math.min(a, b)`: NaN if either is NaN; `-0` wins over `+0`.
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() {
            a
        } else {
            b
        }
    } else if a < b {
        a
    } else {
        b
    }
}

/// `Math.max(a, b)`: NaN if either is NaN; `+0` wins over `-0`.
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_positive() {
            a
        } else {
            b
        }
    } else if a > b {
        a
    } else {
        b
    }
}

/// `x || 0` for a number: `0`, `-0` and NaN become `+0`.
pub fn js_or_zero(x: f64) -> f64 {
    if x == 0.0 || x.is_nan() {
        0.0
    } else {
        x
    }
}

/// `x || fallback` truthiness of a number (`0`, `-0`, NaN are falsy).
pub fn js_truthy(x: f64) -> bool {
    !(x == 0.0 || x.is_nan())
}

/// Characters `String.prototype.trim()` removes: ECMAScript WhiteSpace + LineTerminator, i.e. the
/// Unicode `White_Space` set without U+0085 (NEL) plus U+FEFF (ZWNBSP).
pub fn is_js_whitespace(c: char) -> bool {
    c == '\u{FEFF}' || (c.is_whitespace() && c != '\u{0085}')
}

/// `s.trim()` with JS semantics.
pub fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_max_follow_js() {
        assert!(js_min(f64::NAN, 1.0).is_nan());
        assert!(js_max(1.0, f64::NAN).is_nan());
        assert!(js_min(0.0, -0.0).is_sign_negative());
        assert!(js_min(-0.0, 0.0).is_sign_negative());
        assert!(js_max(-0.0, 0.0).is_sign_positive());
        assert!(js_max(0.0, -0.0).is_sign_positive());
        assert_eq!(js_min(0.2, 0.1), 0.1);
        assert_eq!(js_max(0.2, 0.1), 0.2);
        assert_eq!(js_min(f64::INFINITY, 0.1), 0.1);
        assert_eq!(js_max(f64::NEG_INFINITY, -3.0), -3.0);
    }

    #[test]
    fn or_zero_and_truthy() {
        assert_eq!(js_or_zero(f64::NAN), 0.0);
        assert!(js_or_zero(-0.0).is_sign_positive());
        assert_eq!(js_or_zero(0.25), 0.25);
        assert!(!js_truthy(0.0) && !js_truthy(-0.0) && !js_truthy(f64::NAN));
        assert!(js_truthy(1e-300) && js_truthy(-1.0));
    }

    #[test]
    fn trim_matches_ecmascript() {
        assert_eq!(js_trim("  smile\n\t"), "smile");
        assert_eq!(js_trim("\u{FEFF}SAD\u{3000}"), "SAD");
        assert_eq!(js_trim("\u{2028}x\u{2029}"), "x");
        // NEL is not JS whitespace (Rust's str::trim would remove it).
        assert_eq!(js_trim("\u{0085}x"), "\u{0085}x");
        assert_eq!(js_trim("\u{00A0}\u{1680}a\u{202F}"), "a");
    }
}
