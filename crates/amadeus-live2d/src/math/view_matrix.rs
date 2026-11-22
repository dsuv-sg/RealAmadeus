//! `CubismViewMatrix` — port of `Framework/src/math/cubismviewmatrix.ts` (Cubism SDK for Web 5-r.5).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by
//! the Live2D Open Software license; this file is a Rust translation of it.
//!
//! Screen / limit fields are JS numbers (f64). In V1 `adjustTranslate` / `adjustScale` are never
//! called, so the view matrix stays the identity for the whole session (render spec §3.5); the
//! class is ported completely anyway.

use std::ops::{Deref, DerefMut};

use super::matrix44::CubismMatrix44;

/// View matrix with screen / max-screen rectangles and zoom limits.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CubismViewMatrix {
    matrix: CubismMatrix44,
    screen_left: f64,
    screen_right: f64,
    screen_top: f64,
    screen_bottom: f64,
    max_left: f64,
    max_right: f64,
    max_top: f64,
    max_bottom: f64,
    max_scale: f64,
    min_scale: f64,
}

impl Deref for CubismViewMatrix {
    type Target = CubismMatrix44;
    fn deref(&self) -> &CubismMatrix44 {
        &self.matrix
    }
}

impl DerefMut for CubismViewMatrix {
    fn deref_mut(&mut self) -> &mut CubismMatrix44 {
        &mut self.matrix
    }
}

impl CubismViewMatrix {
    /// `new CubismViewMatrix()`: identity matrix, all limits 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// The wrapped matrix.
    pub fn matrix(&self) -> &CubismMatrix44 {
        &self.matrix
    }

    /// `adjustTranslate(x, y)` — clamps the translation against the screen / max rectangles,
    /// then `M ← T(x, y) · M`.
    ///
    /// QUIRK (kept): the clamp assumes the new translation is `tr[12] + x`, but `T · M` adds
    /// `x * tr[0]`; both only agree at scale 1.
    pub fn adjust_translate(&mut self, mut x: f64, mut y: f64) {
        let tr = *self.matrix.array();
        let t0 = f64::from(tr[0]);
        let t5 = f64::from(tr[5]);
        let t12 = f64::from(tr[12]);
        let t13 = f64::from(tr[13]);
        if t0 * self.max_left + (t12 + x) > self.screen_left {
            x = self.screen_left - t0 * self.max_left - t12;
        }
        if t0 * self.max_right + (t12 + x) < self.screen_right {
            x = self.screen_right - t0 * self.max_right - t12;
        }
        if t5 * self.max_top + (t13 + y) < self.screen_top {
            y = self.screen_top - t5 * self.max_top - t13;
        }
        if t5 * self.max_bottom + (t13 + y) > self.screen_bottom {
            y = self.screen_bottom - t5 * self.max_bottom - t13;
        }
        let t = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            x as f32, y as f32, 0.0, 1.0,
        ];
        let next = CubismMatrix44::multiply(&t, self.matrix.array());
        self.matrix.set_matrix(&next);
    }

    /// `adjustScale(cx, cy, scale)` — clamps `scale * tr[0]` to `[min, max]` (only when
    /// `tr[0] > 0`), then left-multiplies `T(-cx, -cy)`, `S(scale)`, `T(cx, cy)` in that order
    /// (each built as a `Float32Array`, so the arguments are rounded to f32).
    pub fn adjust_scale(&mut self, cx: f64, cy: f64, mut scale: f64) {
        let max_scale = self.max_scale();
        let min_scale = self.min_scale();
        let t0 = f64::from(self.matrix.array()[0]);
        let target_scale = scale * t0;
        if target_scale < min_scale {
            if t0 > 0.0 {
                scale = min_scale / t0;
            }
        } else if target_scale > max_scale && t0 > 0.0 {
            scale = max_scale / t0;
        }
        let (cx32, cy32, s32) = (cx as f32, cy as f32, scale as f32);
        let tr1 = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, cx32, cy32, 0.0, 1.0,
        ];
        let tr2 = [
            s32, 0.0, 0.0, 0.0, 0.0, s32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let tr3 = [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -cx32, -cy32, 0.0, 1.0,
        ];
        let mut m = CubismMatrix44::multiply(&tr3, self.matrix.array());
        m = CubismMatrix44::multiply(&tr2, &m);
        m = CubismMatrix44::multiply(&tr1, &m);
        self.matrix.set_matrix(&m);
    }

    /// `setScreenRect(left, right, bottom, top)`.
    pub fn set_screen_rect(&mut self, left: f64, right: f64, bottom: f64, top: f64) {
        self.screen_left = left;
        self.screen_right = right;
        self.screen_bottom = bottom;
        self.screen_top = top;
    }

    /// `setMaxScreenRect(left, right, bottom, top)`.
    pub fn set_max_screen_rect(&mut self, left: f64, right: f64, bottom: f64, top: f64) {
        self.max_left = left;
        self.max_right = right;
        self.max_top = top;
        self.max_bottom = bottom;
    }

    /// `setMaxScale(v)`.
    pub fn set_max_scale(&mut self, max_scale: f64) {
        self.max_scale = max_scale;
    }

    /// `setMinScale(v)`.
    pub fn set_min_scale(&mut self, min_scale: f64) {
        self.min_scale = min_scale;
    }

    /// `getMaxScale()`.
    pub fn max_scale(&self) -> f64 {
        self.max_scale
    }

    /// `getMinScale()`.
    pub fn min_scale(&self) -> f64 {
        self.min_scale
    }

    /// `isMaxScale()`: `getScaleX() >= _maxScale`.
    pub fn is_max_scale(&self) -> bool {
        f64::from(self.matrix.scale_x()) >= self.max_scale
    }

    /// `isMinScale()`: `getScaleX() <= _minScale`.
    pub fn is_min_scale(&self) -> bool {
        f64::from(self.matrix.scale_x()) <= self.min_scale
    }

    /// `getScreenLeft()`.
    pub fn screen_left(&self) -> f64 {
        self.screen_left
    }

    /// `getScreenRight()`.
    pub fn screen_right(&self) -> f64 {
        self.screen_right
    }

    /// `getScreenBottom()`.
    pub fn screen_bottom(&self) -> f64 {
        self.screen_bottom
    }

    /// `getScreenTop()`.
    pub fn screen_top(&self) -> f64 {
        self.screen_top
    }

    /// `getMaxLeft()`.
    pub fn max_left(&self) -> f64 {
        self.max_left
    }

    /// `getMaxRight()`.
    pub fn max_right(&self) -> f64 {
        self.max_right
    }

    /// `getMaxBottom()`.
    pub fn max_bottom(&self) -> f64 {
        self.max_bottom
    }

    /// `getMaxTop()`.
    pub fn max_top(&self) -> f64 {
        self.max_top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `LAppView.initialize` (render spec §3.5) for a 1280×720 canvas.
    fn v1_view(width: f64, height: f64) -> CubismViewMatrix {
        let ratio = width / height;
        let mut v = CubismViewMatrix::new();
        v.set_screen_rect(-ratio, ratio, -1.0, 1.0);
        v.scale(1.0, 1.0);
        v.set_max_scale(2.0);
        v.set_min_scale(0.8);
        v.set_max_screen_rect(-2.0, 2.0, -2.0, 2.0);
        v
    }

    #[test]
    fn v1_view_is_identity() {
        let v = v1_view(1280.0, 720.0);
        assert_eq!(v.array(), &CubismMatrix44::IDENTITY);
        assert_eq!(v.screen_left(), -1280.0 / 720.0);
        assert_eq!(v.screen_right(), 1280.0 / 720.0);
        assert_eq!((v.screen_bottom(), v.screen_top()), (-1.0, 1.0));
        assert_eq!(
            (v.max_left(), v.max_right(), v.max_bottom(), v.max_top()),
            (-2.0, 2.0, -2.0, 2.0)
        );
        assert!(!v.is_max_scale());
        assert!(!v.is_min_scale());
    }

    #[test]
    fn adjust_scale_clamps_and_pivots() {
        let mut v = v1_view(1280.0, 720.0);
        v.adjust_scale(0.5, 0.25, 10.0); // clamped to max 2.0
        assert_eq!(v.scale_x(), 2.0);
        assert_eq!(v.scale_y(), 2.0);
        // Row vectors: v' = ((v + c)·s) − c → translation c·(s − 1) (fixed point −c, as in the TS).
        assert_eq!((v.translate_x(), v.translate_y()), (0.5, 0.25));
        assert!(v.is_max_scale());
        v.adjust_scale(0.0, 0.0, 0.1); // 0.2 < 0.8 → scale = 0.8 / 2.0
        assert_eq!(v.scale_x(), 0.8_f32);
        // f32(0.8) = 0.800000011920929 > 0.8 (f64): JS compares the same way → false.
        assert!(!v.is_min_scale());
    }

    #[test]
    fn adjust_translate_clamps_with_quirk() {
        let mut v = v1_view(1000.0, 1000.0);
        // screen [-1,1]², max rect [-2,2]²: x clamp: -2 + (0 + x) > -1 → x = 1
        v.adjust_translate(5.0, -5.0);
        assert_eq!((v.translate_x(), v.translate_y()), (1.0, -1.0));
        // At scale 2 the clamp still reasons with tr[12] + x, but T·M adds x * tr[0] (QUIRK).
        let mut w = v1_view(1000.0, 1000.0);
        w.scale(2.0, 2.0);
        w.adjust_translate(0.5, 0.0);
        assert_eq!(w.translate_x(), 1.0);
    }
}
