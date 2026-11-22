//! `CubismMath` — port of `Framework/src/math/cubismmath.ts` (Cubism Web Framework 5-r.5).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! Every function works on JS numbers (`f64`) with the exact JS operation order. The JS `Math`
//! transcendental functions are reproduced by [`js_sin`], [`js_cos`], [`js_atan2`], [`js_acos`],
//! [`js_exp`] and [`js_log`] so that host unit tests and the wasm build compute identical values:
//!
//! * `atan2`, `acos`, `exp`: the `libm` crate (fdlibm/musl) — measured bit-identical to V8 (Node 22)
//!   on 200 000 random inputs;
//! * `log`: a port of the original fdlibm `e_log.c` (V8 `ieee754::log`), bit-identical as well (the
//!   `libm` crate uses musl's re-ordered variant, which differs in ~0.4 % of the inputs);
//! * `sin`, `cos`: the `libm` crate (fdlibm). V8 uses glibc-derived (LGPL) code for these, which
//!   differs by at most 1 ulp in ~1 % of the inputs; golden comparisons use tolerances.
//!
//! Free functions are the primary API; [`CubismMath`] mirrors them as associated functions for code
//! ported from `CubismMath.xxx(...)` call sites.

use super::vector2::CubismVector2;
use std::f64::consts::PI;

/// `CubismMath.Epsilon`.
pub const EPSILON: f64 = 0.00001;

/// `Math.sin` (fdlibm).
#[inline]
pub fn js_sin(x: f64) -> f64 {
    libm::sin(x)
}

/// `Math.cos` (fdlibm).
#[inline]
pub fn js_cos(x: f64) -> f64 {
    libm::cos(x)
}

/// `Math.atan2(y, x)` (fdlibm, bit-identical to V8).
#[inline]
pub fn js_atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

/// `Math.acos` (fdlibm, bit-identical to V8).
#[inline]
pub fn js_acos(x: f64) -> f64 {
    libm::acos(x)
}

/// `Math.exp` (fdlibm, bit-identical to V8).
#[inline]
pub fn js_exp(x: f64) -> f64 {
    libm::exp(x)
}

/// `Math.log` — port of fdlibm `e_log.c` exactly as V8's `base::ieee754::log` (bit-identical).
///
/// ```text
/// Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
/// Developed at SunSoft, a Sun Microsystems, Inc. business.
/// Permission to use, copy, modify, and distribute this software is freely granted,
/// provided that this notice is preserved.
/// ```
pub fn js_log(x: f64) -> f64 {
    const LN2_HI: f64 = f64::from_bits(0x3fe6_2e42_fee0_0000);
    const LN2_LO: f64 = f64::from_bits(0x3dea_39ef_3579_3c76);
    const TWO54: f64 = f64::from_bits(0x4350_0000_0000_0000);
    const LG1: f64 = f64::from_bits(0x3fe5_5555_5555_5593);
    const LG2: f64 = f64::from_bits(0x3fd9_9999_9997_fa04);
    const LG3: f64 = f64::from_bits(0x3fd2_4924_9422_9359);
    const LG4: f64 = f64::from_bits(0x3fcc_71c5_1d8e_78af);
    const LG5: f64 = f64::from_bits(0x3fc7_4664_96cb_03de);
    const LG6: f64 = f64::from_bits(0x3fc3_9a09_d078_c69f);
    const LG7: f64 = f64::from_bits(0x3fc2_f112_df3e_5244);

    let mut x = x;
    let bits = x.to_bits();
    let mut hx = (bits >> 32) as i32;
    let lx = bits as u32;
    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        // x < 2**-1022
        if ((hx & 0x7fff_ffff) as u32 | lx) == 0 {
            return f64::NEG_INFINITY; // log(+-0) = -inf
        }
        if hx < 0 {
            return f64::NAN; // log(-#) = NaN
        }
        k -= 54;
        x *= TWO54; // subnormal number, scale up x
        hx = (x.to_bits() >> 32) as i32;
    }
    if hx >= 0x7ff0_0000 {
        return x + x;
    }
    k += (hx >> 20) - 1023;
    hx &= 0x000f_ffff;
    let i = (hx + 0x95f64) & 0x0010_0000;
    // normalize x or x/2
    let high = (hx | (i ^ 0x3ff0_0000)) as u32 as u64;
    x = f64::from_bits((high << 32) | (x.to_bits() & 0xffff_ffff));
    k += i >> 20;
    let f = x - 1.0;
    if (0x000f_ffff & (2 + hx)) < 3 {
        // -2**-20 <= f < 2**-20
        if f == 0.0 {
            if k == 0 {
                return 0.0;
            }
            let dk = f64::from(k);
            return dk * LN2_HI + dk * LN2_LO;
        }
        const ONE_THIRD: f64 = f64::from_bits(0x3fd5_5555_5555_5555); // fdlibm 0.33333333333333333
        let r = f * f * (0.5 - ONE_THIRD * f);
        if k == 0 {
            return f - r;
        }
        let dk = f64::from(k);
        return dk * LN2_HI - ((r - dk * LN2_LO) - f);
    }
    let s = f / (2.0 + f);
    let dk = f64::from(k);
    let z = s * s;
    let mut i = hx - 0x6147a;
    let w = z * z;
    let j = 0x6b851 - hx;
    let t1 = w * (LG2 + w * (LG4 + w * LG6));
    let t2 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    i |= j;
    let r = t2 + t1;
    if i > 0 {
        let hfsq = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

/// `Math.sign` (keeps `-0`/`+0`, NaN → NaN).
#[inline]
pub fn js_sign(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        x
    } else if x > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// `Number.prototype.toString()` for log messages and `parseInt(n.toString())` emulation:
/// `NaN`, `Infinity`, integers without a fraction, exponent form (`1e+21`, `1e-7`) outside
/// `[1e-6, 1e21)`.
pub fn js_number_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v > 0.0 {
            "Infinity".to_owned()
        } else {
            "-Infinity".to_owned()
        };
    }
    if v == 0.0 {
        return "0".to_owned();
    }
    let a = v.abs();
    if !(1e-6..1e21).contains(&a) {
        let s = format!("{v:e}");
        // Rust prints "1e21" / "1.5e-7"; JS prints "1e+21" / "1.5e-7".
        return match s.split_once('e') {
            Some((m, e)) if !e.starts_with('-') => format!("{m}e+{e}"),
            _ => s,
        };
    }
    format!("{v}")
}

/// `CubismMath.range(value, min, max)`.
#[inline]
pub fn range(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// `CubismMath.sin` (= `Math.sin`).
#[inline]
pub fn sin(x: f64) -> f64 {
    js_sin(x)
}

/// `CubismMath.cos` (= `Math.cos`).
#[inline]
pub fn cos(x: f64) -> f64 {
    js_cos(x)
}

/// `CubismMath.abs`.
#[inline]
pub fn abs(x: f64) -> f64 {
    x.abs()
}

/// `CubismMath.sqrt` (IEEE correctly rounded, identical to `Math.sqrt`).
#[inline]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// `CubismMath.cbrt` — NOT `Math.cbrt`: `exp(log(|x|) / 3)` refined by one Newton step.
pub fn cbrt(x: f64) -> f64 {
    if x == 0.0 {
        return x;
    }
    let mut cx = x;
    let is_negative_number = cx < 0.0;
    if is_negative_number {
        cx = -cx;
    }
    let ret = if cx == f64::INFINITY {
        f64::INFINITY
    } else {
        let r = js_exp(js_log(cx) / 3.0);
        (cx / (r * r) + 2.0 * r) / 3.0
    };
    if is_negative_number {
        -ret
    } else {
        ret
    }
}

/// `CubismMath.getEasingSine(value)`: `0.5 - 0.5 * cos(value * PI)` clamped to `[0, 1]` input.
#[inline]
pub fn get_easing_sine(value: f64) -> f64 {
    if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        0.5 - 0.5 * js_cos(value * PI)
    }
}

/// `CubismMath.max(l, r)` = `l > r ? l : r` (NaN-order sensitive).
#[inline]
pub fn max(left: f64, right: f64) -> f64 {
    if left > right {
        left
    } else {
        right
    }
}

/// `CubismMath.min(l, r)` = `l > r ? r : l` (NaN-order sensitive).
#[inline]
pub fn min(left: f64, right: f64) -> f64 {
    if left > right {
        right
    } else {
        left
    }
}

/// `CubismMath.clamp(val, min, max)`: `val < min → min`, `max < val → max`, else `val`
/// (NaN passes through; never panics, unlike `f64::clamp`).
#[inline]
pub fn clamp(val: f64, min: f64, max: f64) -> f64 {
    if val < min {
        min
    } else if max < val {
        max
    } else {
        val
    }
}

/// `CubismMath.degreesToRadian(d)` = `d / 180 * PI`.
#[inline]
pub fn degrees_to_radian(degrees: f64) -> f64 {
    degrees / 180.0 * PI
}

/// `CubismMath.radianToDegrees(r)` = `r * 180 / PI`.
#[inline]
pub fn radian_to_degrees(radian: f64) -> f64 {
    radian * 180.0 / PI
}

/// `CubismMath.directionToRadian(from, to)`: `atan2(to) - atan2(from)` wrapped into `[-PI, PI]`.
pub fn direction_to_radian(from: CubismVector2, to: CubismVector2) -> f64 {
    let q1 = js_atan2(to.y, to.x);
    let q2 = js_atan2(from.y, from.x);
    let mut ret = q1 - q2;
    while ret < -PI {
        ret += PI * 2.0;
    }
    while ret > PI {
        ret -= PI * 2.0;
    }
    ret
}

/// `CubismMath.directionToDegrees(from, to)`.
pub fn direction_to_degrees(from: CubismVector2, to: CubismVector2) -> f64 {
    let radian = direction_to_radian(from, to);
    let mut degree = radian_to_degrees(radian);
    if to.x - from.x > 0.0 {
        degree = -degree;
    }
    degree
}

/// `CubismMath.radianToDirection(a)` = `(sin a, cos a)`.
pub fn radian_to_direction(total_angle: f64) -> CubismVector2 {
    CubismVector2::new(js_sin(total_angle), js_cos(total_angle))
}

/// `CubismMath.quadraticEquation(a, b, c)`.
pub fn quadratic_equation(a: f64, b: f64, c: f64) -> f64 {
    if a.abs() < EPSILON {
        if b.abs() < EPSILON {
            return -c;
        }
        return -c / b;
    }
    -(b + (b * b - 4.0 * a * c).sqrt()) / (2.0 * a)
}

/// `CubismMath.cardanoAlgorithmForBezier(a, b, c, d)`: the root of `a t³ + b t² + c t + d` closest
/// to the middle of `[0, 1]`, clamped to `[0, 1]`.
pub fn cardano_algorithm_for_bezier(a: f64, b: f64, c: f64, d: f64) -> f64 {
    if a.abs() < EPSILON {
        return range(quadratic_equation(b, c, d), 0.0, 1.0);
    }
    let ba = b / a;
    let ca = c / a;
    let da = d / a;
    let p = (3.0 * ca - ba * ba) / 3.0;
    let p3 = p / 3.0;
    let q = (2.0 * ba * ba * ba - 9.0 * ba * ca + 27.0 * da) / 27.0;
    let q2 = q / 2.0;
    let discriminant = q2 * q2 + p3 * p3 * p3;
    let center = 0.5;
    let threshold = center + 0.01;
    if discriminant < 0.0 {
        let mp3 = -p / 3.0;
        let mp33 = mp3 * mp3 * mp3;
        let r = mp33.sqrt();
        let t = -q / (2.0 * r);
        let cosphi = range(t, -1.0, 1.0);
        let phi = js_acos(cosphi);
        let crtr = cbrt(r);
        let t1 = 2.0 * crtr;
        let root1 = t1 * js_cos(phi / 3.0) - ba / 3.0;
        if (root1 - center).abs() < threshold {
            return range(root1, 0.0, 1.0);
        }
        let root2 = t1 * js_cos((phi + 2.0 * PI) / 3.0) - ba / 3.0;
        if (root2 - center).abs() < threshold {
            return range(root2, 0.0, 1.0);
        }
        let root3 = t1 * js_cos((phi + 4.0 * PI) / 3.0) - ba / 3.0;
        return range(root3, 0.0, 1.0);
    }
    if discriminant == 0.0 {
        let u1 = if q2 < 0.0 { cbrt(-q2) } else { -cbrt(q2) };
        let root1 = 2.0 * u1 - ba / 3.0;
        if (root1 - center).abs() < threshold {
            return range(root1, 0.0, 1.0);
        }
        let root2 = -u1 - ba / 3.0;
        return range(root2, 0.0, 1.0);
    }
    let sd = discriminant.sqrt();
    let u1 = cbrt(sd - q2);
    let v1 = cbrt(sd + q2);
    let root1 = u1 - v1 - ba / 3.0;
    range(root1, 0.0, 1.0)
}

/// `CubismMath.mod(dividend, divisor)` (`mod` is a Rust keyword): a remainder with the sign of the
/// dividend; returns NaN (and warns like V1's `console.warn`) for non-finite / zero inputs.
pub fn modulo(dividend: f64, divisor: f64) -> f64 {
    if !dividend.is_finite() || divisor == 0.0 || dividend.is_nan() || divisor.is_nan() {
        log::warn!(
            "divided: {}, divisor: {} mod() returns 'NaN'.",
            js_number_to_string(dividend),
            js_number_to_string(divisor)
        );
        return f64::NAN;
    }
    let abs_dividend = dividend.abs();
    let abs_divisor = divisor.abs();
    let mut result = abs_dividend - (abs_dividend / abs_divisor).floor() * abs_divisor;
    result *= js_sign(dividend);
    result
}

/// Associated-function mirror of the free functions (`CubismMath.xxx` call sites).
#[derive(Debug, Clone, Copy, Default)]
pub struct CubismMath;

impl CubismMath {
    pub const EPSILON: f64 = EPSILON;
    pub fn range(value: f64, min: f64, max: f64) -> f64 {
        range(value, min, max)
    }
    pub fn sin(x: f64) -> f64 {
        sin(x)
    }
    pub fn cos(x: f64) -> f64 {
        cos(x)
    }
    pub fn abs(x: f64) -> f64 {
        abs(x)
    }
    pub fn sqrt(x: f64) -> f64 {
        sqrt(x)
    }
    pub fn cbrt(x: f64) -> f64 {
        cbrt(x)
    }
    pub fn get_easing_sine(value: f64) -> f64 {
        get_easing_sine(value)
    }
    pub fn max(left: f64, right: f64) -> f64 {
        max(left, right)
    }
    pub fn min(left: f64, right: f64) -> f64 {
        min(left, right)
    }
    pub fn clamp(val: f64, min: f64, max: f64) -> f64 {
        clamp(val, min, max)
    }
    pub fn degrees_to_radian(degrees: f64) -> f64 {
        degrees_to_radian(degrees)
    }
    pub fn radian_to_degrees(radian: f64) -> f64 {
        radian_to_degrees(radian)
    }
    pub fn direction_to_radian(from: CubismVector2, to: CubismVector2) -> f64 {
        direction_to_radian(from, to)
    }
    pub fn direction_to_degrees(from: CubismVector2, to: CubismVector2) -> f64 {
        direction_to_degrees(from, to)
    }
    pub fn radian_to_direction(total_angle: f64) -> CubismVector2 {
        radian_to_direction(total_angle)
    }
    pub fn quadratic_equation(a: f64, b: f64, c: f64) -> f64 {
        quadratic_equation(a, b, c)
    }
    pub fn cardano_algorithm_for_bezier(a: f64, b: f64, c: f64, d: f64) -> f64 {
        cardano_algorithm_for_bezier(a, b, c, d)
    }
    pub fn modulo(dividend: f64, divisor: f64) -> f64 {
        modulo(dividend, divisor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_log_special_values() {
        assert_eq!(js_log(1.0), 0.0);
        assert_eq!(js_log(0.0), f64::NEG_INFINITY);
        assert!(js_log(-1.0).is_nan());
        assert_eq!(js_log(f64::INFINITY), f64::INFINITY);
        assert_eq!(js_log(std::f64::consts::E), 1.0);
        // values measured in Node 22 (Math.log) — bit patterns
        assert_eq!(js_log(10.0).to_bits(), 0x4002_6bb1_bbb5_5516);
        assert_eq!(js_log(5e-324).to_bits(), 0xc087_4385_446d_71c3);
    }

    #[test]
    fn cbrt_and_easing() {
        assert_eq!(cbrt(0.0), 0.0);
        assert!(cbrt(-0.0).is_sign_negative());
        assert_eq!(cbrt(27.0), 3.0);
        assert_eq!(cbrt(-8.0), -2.0);
        assert_eq!(cbrt(f64::INFINITY), f64::INFINITY);
        assert_eq!(get_easing_sine(-1.0), 0.0);
        assert_eq!(get_easing_sine(2.0), 1.0);
        assert_eq!(get_easing_sine(0.5), 0.5 - 0.5 * js_cos(0.5 * PI));
    }

    #[test]
    fn min_max_clamp_nan_order() {
        assert!(max(f64::NAN, 1.0) == 1.0);
        assert!(max(1.0, f64::NAN).is_nan());
        assert!(min(f64::NAN, 1.0).is_nan());
        assert!(min(1.0, f64::NAN) == 1.0);
        assert!(clamp(f64::NAN, 0.0, 1.0).is_nan());
        assert_eq!(clamp(5.0, 0.0, 1.0), 1.0);
        assert_eq!(clamp(-5.0, 0.0, 1.0), 0.0);
        assert_eq!(range(0.5, 0.0, 1.0), 0.5);
    }

    #[test]
    fn modulo_semantics() {
        assert_eq!(modulo(7.5, 2.0), 1.5);
        assert_eq!(modulo(-7.5, 2.0), -1.5);
        assert!(modulo(-0.0, 2.0).is_sign_negative());
        assert!(modulo(1.0, 0.0).is_nan());
        assert!(modulo(f64::INFINITY, 2.0).is_nan());
        assert!(modulo(1.0, f64::NAN).is_nan());
    }

    #[test]
    fn direction_helpers() {
        let up = CubismVector2::new(0.0, 1.0);
        let right = CubismVector2::new(1.0, 0.0);
        assert_eq!(direction_to_radian(up, right), -PI / 2.0);
        assert_eq!(direction_to_degrees(up, right), 90.0);
        let d = radian_to_direction(0.0);
        assert_eq!((d.x, d.y), (0.0, 1.0));
        assert_eq!(degrees_to_radian(180.0), PI);
        assert_eq!(radian_to_degrees(PI), 180.0);
    }

    #[test]
    fn cardano_finds_middle_root() {
        // Linear bezier timing (control points at thirds) → t == x.
        for k in 0..=10 {
            let x = f64::from(k) / 10.0;
            let (x1, cx1, cx2, x2) = (0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0);
            let a = x2 - 3.0 * cx2 + 3.0 * cx1 - x1;
            let b = 3.0 * cx2 - 6.0 * cx1 + 3.0 * x1;
            let c = 3.0 * cx1 - 3.0 * x1;
            let d = x1 - x;
            assert!((cardano_algorithm_for_bezier(a, b, c, d) - x).abs() < 1e-12);
        }
        // a == 0 → quadratic branch, clamped.
        assert_eq!(cardano_algorithm_for_bezier(0.0, 0.0, 1.0, -2.0), 1.0);
        assert_eq!(quadratic_equation(0.0, 0.0, 3.0), -3.0);
    }

    #[test]
    fn number_to_string() {
        assert_eq!(js_number_to_string(1.5), "1.5");
        assert_eq!(js_number_to_string(3.0), "3");
        assert_eq!(js_number_to_string(1e21), "1e+21");
        assert_eq!(js_number_to_string(1.5e-7), "1.5e-7");
        assert_eq!(js_number_to_string(0.000001), "0.000001");
        assert_eq!(js_number_to_string(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(js_number_to_string(-0.0), "0");
    }
}
