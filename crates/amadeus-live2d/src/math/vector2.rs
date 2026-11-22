//! `CubismVector2` — port of `Framework/src/math/cubismvector2.ts` (Cubism Web Framework 5-r.5).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! Components are JS numbers (`f64`). Every operation except [`CubismVector2::normalize`] returns a
//! new vector, exactly like the TypeScript class; `normalize` works in place and turns a zero vector
//! into `(NaN, NaN)` (division by a zero length), which the physics relies on reproducing.

use std::ops::{Add, Div, Mul, Sub};

/// 2D vector with `f64` components (`CubismVector2`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CubismVector2 {
    pub x: f64,
    pub y: f64,
}

impl CubismVector2 {
    /// `new CubismVector2(x, y)`.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// `substract(v)` (sic): component-wise `self - v`.
    pub fn substract(&self, v: &CubismVector2) -> CubismVector2 {
        CubismVector2::new(self.x - v.x, self.y - v.y)
    }

    /// Alias of [`CubismVector2::substract`] with the correct spelling.
    pub fn subtract(&self, v: &CubismVector2) -> CubismVector2 {
        self.substract(v)
    }

    /// `multiply(v)`: component-wise product.
    pub fn multiply(&self, v: &CubismVector2) -> CubismVector2 {
        CubismVector2::new(self.x * v.x, self.y * v.y)
    }

    /// `multiplyByScaler(s)` (sic) = `multiply((s, s))`.
    pub fn multiply_by_scaler(&self, scalar: f64) -> CubismVector2 {
        self.multiply(&CubismVector2::new(scalar, scalar))
    }

    /// `division(v)`: component-wise quotient.
    pub fn division(&self, v: &CubismVector2) -> CubismVector2 {
        CubismVector2::new(self.x / v.x, self.y / v.y)
    }

    /// `divisionByScalar(s)` = `division((s, s))`.
    pub fn division_by_scalar(&self, scalar: f64) -> CubismVector2 {
        self.division(&CubismVector2::new(scalar, scalar))
    }

    /// `getLength()` = `sqrt(x² + y²)`.
    pub fn get_length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// `getDistanceWith(a)`.
    pub fn get_distance_with(&self, a: &CubismVector2) -> f64 {
        ((self.x - a.x) * (self.x - a.x) + (self.y - a.y) * (self.y - a.y)).sqrt()
    }

    /// `dot(a)`.
    pub fn dot(&self, a: &CubismVector2) -> f64 {
        self.x * a.x + self.y * a.y
    }

    /// `normalize()` in place. `Math.pow(v, 0.5)` equals `Math.sqrt(v)` for `v >= +0` in V8, and
    /// `x² + y²` is never `-0`, so `sqrt` is exact here. A zero vector becomes `(NaN, NaN)`.
    pub fn normalize(&mut self) {
        let length = (self.x * self.x + self.y * self.y).sqrt();
        self.x /= length;
        self.y /= length;
    }

    /// `isEqual(rhs)` (`==` on both components; NaN is never equal).
    pub fn is_equal(&self, rhs: &CubismVector2) -> bool {
        self.x == rhs.x && self.y == rhs.y
    }

    /// `isNotEqual(rhs)`.
    pub fn is_not_equal(&self, rhs: &CubismVector2) -> bool {
        !self.is_equal(rhs)
    }
}

/// `add(v)`: component-wise sum.
impl Add for CubismVector2 {
    type Output = CubismVector2;
    fn add(self, rhs: CubismVector2) -> CubismVector2 {
        CubismVector2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

/// `substract(v)`.
impl Sub for CubismVector2 {
    type Output = CubismVector2;
    fn sub(self, rhs: CubismVector2) -> CubismVector2 {
        CubismVector2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

/// `multiplyByScaler(s)`.
impl Mul<f64> for CubismVector2 {
    type Output = CubismVector2;
    fn mul(self, rhs: f64) -> CubismVector2 {
        self.multiply_by_scaler(rhs)
    }
}

/// `divisionByScalar(s)`.
impl Div<f64> for CubismVector2 {
    type Output = CubismVector2;
    fn div(self, rhs: f64) -> CubismVector2 {
        self.division_by_scalar(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_matches_ts() {
        let a = CubismVector2::new(3.0, 4.0);
        let b = CubismVector2::new(1.0, -2.0);
        assert_eq!(a + b, CubismVector2::new(4.0, 2.0));
        assert_eq!(a.substract(&b), CubismVector2::new(2.0, 6.0));
        assert_eq!(a.multiply(&b), CubismVector2::new(3.0, -8.0));
        assert_eq!(a * 2.0, CubismVector2::new(6.0, 8.0));
        assert_eq!(a / 2.0, CubismVector2::new(1.5, 2.0));
        assert_eq!(a.get_length(), 5.0);
        assert_eq!(a.dot(&b), -5.0);
        assert_eq!(a.get_distance_with(&CubismVector2::new(0.0, 0.0)), 5.0);
        let mut n = a;
        n.normalize();
        assert_eq!(n, CubismVector2::new(0.6, 0.8));
        let mut z = CubismVector2::default();
        z.normalize();
        assert!(z.x.is_nan() && z.y.is_nan());
        assert!(z.is_not_equal(&z));
        // -1 * 0 keeps the IEEE sign like JS.
        let g = CubismVector2::new(0.0, -1.0).multiply_by_scaler(-1.0);
        assert!(g.x == 0.0 && g.x.is_sign_negative() && g.y == 1.0);
    }
}
