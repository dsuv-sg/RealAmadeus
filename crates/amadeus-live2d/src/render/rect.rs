//! `csmRect` — port of `Framework/src/type/csmrectf.ts` (f64 fields). Copyright(c) Live2D Inc.,
//! Live2D Open Software license (translated).

/// Rectangle with JS-number (f64) fields.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct CsmRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl CsmRect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// `new csmRect()` leaves every field `undefined`; any arithmetic on it yields `NaN`.
    /// Clipping contexts start like this (render spec §3.6 / §9.5 QUIRK B).
    pub const fn undefined() -> Self {
        Self::new(f64::NAN, f64::NAN, f64::NAN, f64::NAN)
    }

    /// `getCenterX()` = `x + 0.5 * width`.
    pub fn center_x(&self) -> f64 {
        self.x + 0.5 * self.width
    }

    /// `getCenterY()` = `y + 0.5 * height`.
    pub fn center_y(&self) -> f64 {
        self.y + 0.5 * self.height
    }

    /// `getRight()` = `x + width`.
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// `getBottom()` = `y + height`.
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// `setRect(r)`.
    pub fn set_rect(&mut self, r: &CsmRect) {
        *self = *r;
    }

    /// `expand(w, h)`: `x -= w; y -= h; width += w * 2; height += h * 2`.
    pub fn expand(&mut self, w: f64, h: f64) {
        self.x -= w;
        self.y -= h;
        self.width += w * 2.0;
        self.height += h * 2.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_ops() {
        let mut r = CsmRect::new(1.0, 2.0, 4.0, 6.0);
        assert_eq!(
            (r.center_x(), r.center_y(), r.right(), r.bottom()),
            (3.0, 5.0, 5.0, 8.0)
        );
        r.expand(0.5, 1.0);
        assert_eq!(r, CsmRect::new(0.5, 1.0, 5.0, 8.0));
        let mut s = CsmRect::undefined();
        assert!(s.right().is_nan());
        s.set_rect(&r);
        assert_eq!(s, r);
    }
}
