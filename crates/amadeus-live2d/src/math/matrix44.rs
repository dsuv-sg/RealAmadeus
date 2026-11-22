//! `CubismMatrix44` — port of `Framework/src/math/cubismmatrix44.ts` (Cubism SDK for Web 5-r.5).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by
//! the Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>);
//! this file is a Rust translation of it.
//!
//! # Numerics (render spec §3.1–3.3)
//!
//! The TypeScript class stores its 16 elements in a `Float32Array`, so every element is an f32,
//! while all arithmetic happens on JS numbers (f64) and is rounded to f32 only when stored back.
//! This port reproduces that bit-exactly: elements are `[f32; 16]`, every expression is evaluated
//! in `f64` and cast with `as f32` (round-to-nearest-even, identical to JS `ToFloat32`) exactly
//! where the TypeScript stores into the typed array.
//!
//! # Layout
//!
//! `tr[r * 4 + c]` is a **row-major matrix for row vectors** (`v' = v · M`), translation in
//! `tr[12..15]`. Byte for byte this is the OpenGL column-major layout of `Mᵀ`, so the array is
//! uploaded with `uniformMatrix4fv(loc, false, tr)` and used as `u_matrix * a_position` in GLSL.
//! Never transpose. All `*_relative` operations and [`CubismMatrix44::multiply_by_matrix`]
//! pre-multiply (`M ← T · M`), i.e. the new operation is applied to the vertex *before* the
//! existing transform.

/// 4×4 matrix with the Cubism storage and precision conventions (see module docs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubismMatrix44 {
    tr: [f32; 16],
}

impl Default for CubismMatrix44 {
    fn default() -> Self {
        Self::new()
    }
}

impl CubismMatrix44 {
    /// The identity matrix array.
    pub const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];

    /// `new CubismMatrix44()` — the identity matrix.
    pub const fn new() -> Self {
        Self { tr: Self::IDENTITY }
    }

    /// A matrix holding a copy of `tr`.
    pub const fn from_array(tr: [f32; 16]) -> Self {
        Self { tr }
    }

    /// `CubismMatrix44.multiply(a, b, dst)` returning `dst`: row-major `c = a · b`.
    ///
    /// Exact TS semantics: `c` starts as a zero `Float32Array`; for `i, j` and `k = 0..4` in that
    /// order, `c[j + i*4] += a[k + i*4] * b[j + k*4]` — the f32 accumulator is widened, the exact
    /// f64 product is added in f64, and the sum is rounded back to f32 on every step. All 64
    /// terms are evaluated (zeros included) so NaN/Inf propagate exactly like the original.
    pub fn multiply(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
        let mut c = [0.0_f32; 16];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    let idx = j + i * 4;
                    c[idx] = (f64::from(c[idx]) + f64::from(a[k + i * 4]) * f64::from(b[j + k * 4]))
                        as f32;
                }
            }
        }
        c
    }

    /// `loadIdentity()`.
    pub fn load_identity(&mut self) {
        self.tr = Self::IDENTITY;
    }

    /// `setMatrix(tr)` — copies 16 elements.
    pub fn set_matrix(&mut self, tr: &[f32; 16]) {
        self.tr = *tr;
    }

    /// `getArray()` — the live element array (upload with `transpose = false`).
    pub fn array(&self) -> &[f32; 16] {
        &self.tr
    }

    /// Mutable access to the element array (TS callers mutate `getArray()` in place, e.g.
    /// `LAppLive2DManager.setViewMatrix`).
    pub fn array_mut(&mut self) -> &mut [f32; 16] {
        &mut self.tr
    }

    /// `getScaleX()` = `tr[0]`.
    pub fn scale_x(&self) -> f32 {
        self.tr[0]
    }

    /// `getScaleY()` = `tr[5]`.
    pub fn scale_y(&self) -> f32 {
        self.tr[5]
    }

    /// `getTranslateX()` = `tr[12]`.
    pub fn translate_x(&self) -> f32 {
        self.tr[12]
    }

    /// `getTranslateY()` = `tr[13]`.
    pub fn translate_y(&self) -> f32 {
        self.tr[13]
    }

    /// `transformX(src)` = `tr[0] * src + tr[12]` (f64 result, not rounded).
    pub fn transform_x(&self, src: f64) -> f64 {
        f64::from(self.tr[0]) * src + f64::from(self.tr[12])
    }

    /// `transformY(src)` = `tr[5] * src + tr[13]`.
    pub fn transform_y(&self, src: f64) -> f64 {
        f64::from(self.tr[5]) * src + f64::from(self.tr[13])
    }

    /// `invertTransformX(src)` = `(src - tr[12]) / tr[0]`.
    pub fn invert_transform_x(&self, src: f64) -> f64 {
        (src - f64::from(self.tr[12])) / f64::from(self.tr[0])
    }

    /// `invertTransformY(src)` = `(src - tr[13]) / tr[5]`.
    pub fn invert_transform_y(&self, src: f64) -> f64 {
        (src - f64::from(self.tr[13])) / f64::from(self.tr[5])
    }

    /// `translateRelative(x, y)`: `M ← T(x, y) · M` (`x`, `y` rounded to f32 first, as the TS
    /// builds `T` as a `Float32Array`).
    pub fn translate_relative(&mut self, x: f64, y: f64) {
        let t = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            x as f32, y as f32, 0.0, 1.0,
        ];
        self.tr = Self::multiply(&t, &self.tr);
    }

    /// `translate(x, y)`: absolute translation (`tr[12] = x; tr[13] = y`).
    pub fn translate(&mut self, x: f64, y: f64) {
        self.tr[12] = x as f32;
        self.tr[13] = y as f32;
    }

    /// `translateX(x)`: `tr[12] = x` (absolute). Named `*_to` because `translate_x()` is the getter.
    pub fn translate_x_to(&mut self, x: f64) {
        self.tr[12] = x as f32;
    }

    /// `translateY(y)`: `tr[13] = y` (absolute).
    pub fn translate_y_to(&mut self, y: f64) {
        self.tr[13] = y as f32;
    }

    /// `scaleRelative(x, y)`: `M ← S(x, y) · M` (rows 0/1 scaled, translation untouched).
    pub fn scale_relative(&mut self, x: f64, y: f64) {
        let s = [
            x as f32, 0.0, 0.0, 0.0, //
            0.0, y as f32, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ];
        self.tr = Self::multiply(&s, &self.tr);
    }

    /// `scale(x, y)`: absolute scale (`tr[0] = x; tr[5] = y`).
    pub fn scale(&mut self, x: f64, y: f64) {
        self.tr[0] = x as f32;
        self.tr[5] = y as f32;
    }

    /// `multiplyByMatrix(m)`: `self ← m · self` (`m` is applied to the vertex **first**; the TS
    /// documentation admits the name does not match the computation).
    pub fn multiply_by_matrix(&mut self, m: &CubismMatrix44) {
        self.tr = Self::multiply(&m.tr, &self.tr);
    }

    /// `getInvert()`: affine inverse (f64 math, f32 store). Returns the identity when
    /// `|det| < CubismMath.Epsilon` (0.00001). `tr[3]`, `tr[7]`, `tr[11]` of the input are ignored.
    pub fn get_invert(&self) -> CubismMatrix44 {
        const EPSILON: f64 = 0.00001;
        let t = |i: usize| f64::from(self.tr[i]);
        let (r00, r10, r20) = (t(0), t(1), t(2));
        let (r01, r11, r21) = (t(4), t(5), t(6));
        let (r02, r12, r22) = (t(8), t(9), t(10));
        let (tx, ty, tz) = (t(12), t(13), t(14));
        let det = r00 * (r11 * r22 - r12 * r21) - r01 * (r10 * r22 - r12 * r20)
            + r02 * (r10 * r21 - r11 * r20);
        let mut dst = CubismMatrix44::new();
        if det.abs() < EPSILON {
            dst.load_identity();
            return dst;
        }
        let inv_det = 1.0 / det;
        let inv00 = (r11 * r22 - r12 * r21) * inv_det;
        let inv01 = -(r01 * r22 - r02 * r21) * inv_det;
        let inv02 = (r01 * r12 - r02 * r11) * inv_det;
        let inv10 = -(r10 * r22 - r12 * r20) * inv_det;
        let inv11 = (r00 * r22 - r02 * r20) * inv_det;
        let inv12 = -(r00 * r12 - r02 * r10) * inv_det;
        let inv20 = (r10 * r21 - r11 * r20) * inv_det;
        let inv21 = -(r00 * r21 - r01 * r20) * inv_det;
        let inv22 = (r00 * r11 - r01 * r10) * inv_det;
        dst.tr = [
            inv00 as f32,
            inv10 as f32,
            inv20 as f32,
            0.0,
            inv01 as f32,
            inv11 as f32,
            inv21 as f32,
            0.0,
            inv02 as f32,
            inv12 as f32,
            inv22 as f32,
            0.0,
            // The translation uses the f64 inverse terms, not the f32-rounded stored ones.
            (-(inv00 * tx + inv01 * ty + inv02 * tz)) as f32,
            (-(inv10 * tx + inv11 * ty + inv12 * tz)) as f32,
            (-(inv20 * tx + inv21 * ty + inv22 * tz)) as f32,
            1.0,
        ];
        dst
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JS replica used to cross-check: `Math.fround`.
    fn fr(v: f64) -> f32 {
        v as f32
    }

    #[test]
    fn identity_and_accessors() {
        let mut m = CubismMatrix44::new();
        assert_eq!(m.array(), &CubismMatrix44::IDENTITY);
        m.scale(2.5, 3.0);
        m.translate(0.25, -1.5);
        assert_eq!(
            (m.scale_x(), m.scale_y(), m.translate_x(), m.translate_y()),
            (2.5, 3.0, 0.25, -1.5)
        );
        assert_eq!(m.transform_x(2.0), 5.25);
        assert_eq!(m.transform_y(1.0), 1.5);
        assert_eq!(m.invert_transform_x(5.25), 2.0);
        assert_eq!(m.invert_transform_y(1.5), 1.0);
        m.translate_x_to(1.0);
        m.translate_y_to(2.0);
        assert_eq!((m.translate_x(), m.translate_y()), (1.0, 2.0));
        m.load_identity();
        assert_eq!(m, CubismMatrix44::default());
    }

    #[test]
    fn multiply_is_row_major_a_times_b() {
        let a: [f32; 16] = std::array::from_fn(|i| i as f32 + 1.0);
        let b: [f32; 16] = std::array::from_fn(|i| (16 - i) as f32);
        let c = CubismMatrix44::multiply(&a, &b);
        for i in 0..4 {
            for j in 0..4 {
                let expected: f32 = (0..4).map(|k| a[k + i * 4] * b[j + k * 4]).sum();
                assert_eq!(c[j + i * 4], expected);
            }
        }
    }

    #[test]
    fn multiply_rounds_to_f32_after_every_term() {
        // 1 + 2^-30 is not representable in f32: an f64 accumulator would keep it, the
        // Float32Array accumulator (TS) drops it at every step.
        let tiny = 2f32.powi(-30);
        let mut a = [0.0_f32; 16];
        let mut b = [0.0_f32; 16];
        a[0] = 1.0;
        a[1] = tiny;
        a[2] = -1.0;
        b[0] = 1.0;
        b[4] = 1.0;
        b[8] = 1.0;
        let c = CubismMatrix44::multiply(&a, &b);
        // k=0: 1; k=1: f32(1 + 2^-30) = 1; k=2: f32(1 - 1) = 0.
        assert_eq!(c[0], 0.0);
    }

    #[test]
    fn relative_ops_premultiply() {
        let mut m = CubismMatrix44::new();
        m.scale(2.0, 4.0);
        m.translate_relative(1.0, 1.0); // T·M: translation += x*row0 + y*row1
        assert_eq!((m.translate_x(), m.translate_y()), (2.0, 4.0));
        m.scale_relative(3.0, 0.5); // S·M: rows 0/1 scaled, translation untouched
        assert_eq!(
            (m.scale_x(), m.scale_y(), m.translate_x(), m.translate_y()),
            (6.0, 2.0, 2.0, 4.0)
        );
        let mut n = CubismMatrix44::new();
        n.translate(10.0, 20.0);
        let mut p = CubismMatrix44::new();
        p.scale(2.0, 2.0);
        p.multiply_by_matrix(&n); // p = n · p : n (translate) applies to the vertex first, then the scale
        assert_eq!(
            (p.scale_x(), p.translate_x(), p.translate_y()),
            (2.0, 20.0, 40.0)
        );
    }

    #[test]
    fn translate_relative_rounds_arguments_to_f32() {
        let mut m = CubismMatrix44::new();
        m.translate_relative(0.1, 1.0 / 3.0);
        assert_eq!(m.translate_x(), fr(0.1));
        assert_eq!(m.translate_y(), fr(1.0 / 3.0));
    }

    #[test]
    fn invert_affine() {
        let mut m = CubismMatrix44::new();
        m.scale(5.697883605957031, 11.0469970703125);
        m.translate(-0.5137413740158081, -6.793132781982422);
        let inv = m.get_invert();
        let back = CubismMatrix44::multiply(inv.array(), m.array());
        for (i, v) in back.iter().enumerate() {
            let e = CubismMatrix44::IDENTITY[i];
            assert!((v - e).abs() < 1e-6, "element {i}: {v} vs {e}");
        }
        // f32 stores of f64 terms, translation from f64 inverse terms.
        let inv00 = 1.0 / f64::from(m.array()[0]);
        assert_eq!(inv.array()[0], inv00 as f32);
        assert_eq!(
            inv.array()[12],
            (-(inv00 * f64::from(m.array()[12]))) as f32
        );
        // Singular → identity.
        let mut s = CubismMatrix44::new();
        s.scale(0.0, 1.0);
        assert_eq!(s.get_invert(), CubismMatrix44::new());
    }

    /// Render spec §3.9 / pipeline spec §8.3: the exact f32 MVP V1 feeds the renderer
    /// (projection built per frame, identity view matrix, model matrix from the canvas size).
    #[test]
    fn golden_mvp_table() {
        use crate::math::model_matrix::CubismModelMatrix;
        // model3.json canvas: 7000 × 10000 px, PixelsPerUnit 7000 (f64 division like the TS).
        let canvas_w = 7000.0_f64 / 7000.0;
        let canvas_h = 10000.0_f64 / 7000.0;
        // Expected values as printed by JS (exact f32 values written as f64 literals).
        let cases: &[(u32, u32, f64, f64, f64)] = &[
            (1280, 720, 3.2287497520446777, 5.739999771118164, -2.75),
            (1920, 1080, 3.2287497520446777, 5.739999771118164, -2.75),
            (2560, 1440, 3.2287497520446777, 5.739999771118164, -2.75),
            (1366, 768, 3.2271740436553955, 5.739999771118164, -2.75),
            (800, 800, 5.739999771118164, 5.739999771118164, -2.75),
            (1000, 1000, 5.739999771118164, 5.739999771118164, -2.75),
            (1920, 1045, 3.124114513397217, 5.739999771118164, -2.75),
            (2880, 1567, 3.1231181621551514, 5.739999771118164, -2.75),
            (
                390,
                844,
                9.998153686523438,
                4.619999885559082,
                -2.0999999046325684,
            ),
            (
                1170,
                2532,
                9.998153686523438,
                4.619999885559082,
                -2.0999999046325684,
            ),
            (
                1080,
                1920,
                8.213333129882812,
                4.619999885559082,
                -2.0999999046325684,
            ),
            (
                750,
                1334,
                8.217439651489258,
                4.619999885559082,
                -2.0999999046325684,
            ),
        ];
        for &(w, h, tr0, tr5, tr13) in cases {
            let mut model_matrix = CubismModelMatrix::new(canvas_w, canvas_h);
            let view = CubismMatrix44::new();
            let (w, h) = (f64::from(w), f64::from(h));
            // LAppLive2DManager.onUpdate (BUNDLE 16465–16509)
            let mut projection = CubismMatrix44::new();
            if canvas_w > 1.0 && w < h {
                model_matrix.set_width(2.0);
                projection.scale(1.0, w / h);
            } else {
                projection.scale(h / w, 1.0);
            }
            projection.multiply_by_matrix(&view);
            if w < h {
                projection.scale_relative(3.3, 3.3);
                projection.translate_y_to(-2.1);
            } else {
                projection.scale_relative(4.1, 4.1);
                projection.translate_y_to(-2.75);
            }
            // LAppModel.draw
            projection.multiply_by_matrix(&model_matrix);
            let t = projection.array();
            assert_eq!(
                (t[0], t[5], t[12], t[13]),
                (fr(tr0), fr(tr5), 0.0, fr(tr13)),
                "canvas {w}x{h}"
            );
            assert_eq!(f64::from(t[0]), tr0, "exact f32 value {w}x{h}");
            for (i, v) in t.iter().enumerate() {
                if ![0, 5, 12, 13].contains(&i) {
                    assert_eq!(
                        *v,
                        CubismMatrix44::IDENTITY[i],
                        "canvas {w}x{h} element {i}"
                    );
                }
            }
        }
    }
}
