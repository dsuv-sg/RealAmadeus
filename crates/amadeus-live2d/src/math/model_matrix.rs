//! `CubismModelMatrix` — port of `Framework/src/math/cubismmodelmatrix.ts` (Cubism SDK for Web 5-r.5).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by
//! the Live2D Open Software license; this file is a Rust translation of it.
//!
//! The TypeScript class `extends CubismMatrix44`; here the matrix is a field and the type derefs
//! to [`CubismMatrix44`], so every matrix method is available on a model matrix. `_width` /
//! `_height` are JS numbers (f64); stores into the matrix round to f32 (see `matrix44`).

use std::ops::{Deref, DerefMut};

use super::matrix44::CubismMatrix44;

/// Model matrix: model units → the space the projection expects (render spec §3.4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubismModelMatrix {
    matrix: CubismMatrix44,
    width: f64,
    height: f64,
}

impl Default for CubismModelMatrix {
    /// `new CubismModelMatrix()` without arguments: width = height = 0, then `setHeight(2)`
    /// (scale = 2 / 0 = +∞, faithfully).
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl Deref for CubismModelMatrix {
    type Target = CubismMatrix44;
    fn deref(&self) -> &CubismMatrix44 {
        &self.matrix
    }
}

impl DerefMut for CubismModelMatrix {
    fn deref_mut(&mut self) -> &mut CubismMatrix44 {
        &mut self.matrix
    }
}

impl CubismModelMatrix {
    /// `new CubismModelMatrix(w, h)`: identity, stores the model size (model units), then
    /// `setHeight(2.0)`.
    ///
    /// V1 passes `model.getCanvasWidth()` / `getCanvasHeight()` (f64 `CanvasWidth / PixelsPerUnit`);
    /// for the shipped model `(1.0, 1.4285714285714286)` → scale `1.399999976158142` (f32).
    pub fn new(width: f64, height: f64) -> Self {
        let mut m = Self {
            matrix: CubismMatrix44::new(),
            width,
            height,
        };
        m.set_height(2.0);
        m
    }

    /// The wrapped matrix.
    pub fn matrix(&self) -> &CubismMatrix44 {
        &self.matrix
    }

    /// Stored model width (`_width`).
    pub fn width(&self) -> f64 {
        self.width
    }

    /// Stored model height (`_height`).
    pub fn height(&self) -> f64 {
        self.height
    }

    /// `setWidth(w)`: `s = w / _width; scale(s, s)` (absolute; no division guard, like the TS).
    pub fn set_width(&mut self, w: f64) {
        let scale_x = w / self.width;
        let scale_y = scale_x;
        self.matrix.scale(scale_x, scale_y);
    }

    /// `setHeight(h)`: `s = h / _height; scale(s, s)`.
    pub fn set_height(&mut self, h: f64) {
        let scale_x = h / self.height;
        let scale_y = scale_x;
        self.matrix.scale(scale_x, scale_y);
    }

    /// `setPosition(x, y)` = `translate(x, y)`.
    pub fn set_position(&mut self, x: f64, y: f64) {
        self.matrix.translate(x, y);
    }

    /// `setCenterPosition(x, y)` = `centerX(x); centerY(y)` (call after the size is set).
    pub fn set_center_position(&mut self, x: f64, y: f64) {
        self.center_x(x);
        self.center_y(y);
    }

    /// `top(y)` = `setY(y)`.
    pub fn top(&mut self, y: f64) {
        self.set_y(y);
    }

    /// `bottom(y)`: `h = _height * getScaleY(); translateY(y - h)`.
    pub fn bottom(&mut self, y: f64) {
        let h = self.height * f64::from(self.matrix.scale_y());
        self.matrix.translate_y_to(y - h);
    }

    /// `left(x)` = `setX(x)`.
    pub fn left(&mut self, x: f64) {
        self.set_x(x);
    }

    /// `right(x)`: `w = _width * getScaleX(); translateX(x - w)`.
    pub fn right(&mut self, x: f64) {
        let w = self.width * f64::from(self.matrix.scale_x());
        self.matrix.translate_x_to(x - w);
    }

    /// `centerX(x)`: `w = _width * getScaleX(); translateX(x - w / 2)`.
    pub fn center_x(&mut self, x: f64) {
        let w = self.width * f64::from(self.matrix.scale_x());
        self.matrix.translate_x_to(x - w / 2.0);
    }

    /// `setX(x)` = `translateX(x)`.
    pub fn set_x(&mut self, x: f64) {
        self.matrix.translate_x_to(x);
    }

    /// `centerY(y)`: `h = _height * getScaleY(); translateY(y - h / 2)`.
    pub fn center_y(&mut self, y: f64) {
        let h = self.height * f64::from(self.matrix.scale_y());
        self.matrix.translate_y_to(y - h / 2.0);
    }

    /// `setY(y)` = `translateY(y)`.
    pub fn set_y(&mut self, y: f64) {
        self.matrix.translate_y_to(y);
    }

    /// `setupFromLayout(layout)`: `layout` is the model3.json `Layout` map **in insertion
    /// order**. Pass 1 applies `width` / `height`; pass 2 applies `x`, `y`, `center_x`,
    /// `center_y`, `top`, `bottom`, `left`, `right` (lowercase keys only; others ignored).
    pub fn setup_from_layout<K: AsRef<str>>(&mut self, layout: &[(K, f64)]) {
        for (key, value) in layout {
            match key.as_ref() {
                "width" => self.set_width(*value),
                "height" => self.set_height(*value),
                _ => {}
            }
        }
        for (key, value) in layout {
            match key.as_ref() {
                "x" => self.set_x(*value),
                "y" => self.set_y(*value),
                "center_x" => self.center_x(*value),
                "center_y" => self.center_y(*value),
                "top" => self.top(*value),
                "bottom" => self.bottom(*value),
                "left" => self.left(*value),
                "right" => self.right(*value),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_model_scale() {
        // Render spec §3.4 / §14.2: new CubismModelMatrix(1.0, 1.4285714285714286).
        let m = CubismModelMatrix::new(7000.0 / 7000.0, 10000.0 / 7000.0);
        assert_eq!(f64::from(m.scale_x()), 1.399999976158142);
        assert_eq!(f64::from(m.scale_y()), 1.399999976158142);
        assert_eq!((m.translate_x(), m.translate_y()), (0.0, 0.0));
        // The CubismModel contract returns the canvas size as f32; the rounded height still
        // yields the same f32 scale for this model.
        let m32 = CubismModelMatrix::new(1.0, f64::from((10000.0_f64 / 7000.0) as f32));
        assert_eq!(m32.scale_x(), m.scale_x());
        // Empty layout (model3.json has none) is a no-op.
        let mut l = m;
        l.setup_from_layout::<&str>(&[]);
        assert_eq!(l, m);
    }

    #[test]
    fn default_has_infinite_scale_like_ts() {
        let m = CubismModelMatrix::default();
        assert!(m.scale_x().is_infinite());
    }

    #[test]
    fn layout_two_passes_in_order() {
        let mut m = CubismModelMatrix::new(2.0, 4.0);
        // "center_x" listed before "width": pass 1 still applies width first.
        m.setup_from_layout(&[
            ("center_x", 0.0),
            ("width", 1.0),
            ("bottom", -1.0),
            ("Width", 9.0),
        ]);
        // width 1.0 / 2.0 → scale 0.5 on both axes
        assert_eq!((m.scale_x(), m.scale_y()), (0.5, 0.5));
        // center_x: tx = 0 - 2.0*0.5/2 = -0.5 ; bottom: ty = -1 - 4.0*0.5 = -3
        assert_eq!((m.translate_x(), m.translate_y()), (-0.5, -3.0));
        m.set_center_position(1.0, 1.0);
        assert_eq!((m.translate_x(), m.translate_y()), (0.5, 0.0));
        m.right(1.0);
        m.top(2.0);
        assert_eq!((m.translate_x(), m.translate_y()), (0.0, 2.0));
        m.left(-1.0);
        m.set_position(3.0, 4.0);
        assert_eq!((m.translate_x(), m.translate_y()), (3.0, 4.0));
        m.set_height(8.0);
        assert_eq!((m.scale_x(), m.scale_y()), (2.0, 2.0));
    }
}
