//! View and projection math of the sample layer (pipeline spec §8): `LAppView.initialize` and the
//! projection `LAppLive2DManager.onUpdate` builds every frame (upper-body zoom, D8). Pure; the
//! matrices follow the Cubism f32-storage / f64-arithmetic rules of [`CubismMatrix44`].

use super::define::{
    DESKTOP_SCALE, DESKTOP_TRANSLATE_Y, MOBILE_SCALE, MOBILE_TRANSLATE_Y, VIEW_LOGICAL_LEFT,
    VIEW_LOGICAL_MAX_BOTTOM, VIEW_LOGICAL_MAX_LEFT, VIEW_LOGICAL_MAX_RIGHT, VIEW_LOGICAL_MAX_TOP,
    VIEW_LOGICAL_RIGHT, VIEW_MAX_SCALE, VIEW_MIN_SCALE, VIEW_SCALE,
};
use crate::math::matrix44::CubismMatrix44;
use crate::math::model_matrix::CubismModelMatrix;
use crate::math::view_matrix::CubismViewMatrix;

/// `LAppView` without the (dead) sprites and touch handling: the view matrix (identity for the
/// whole session in V1) and the device→screen matrix (only used by the dead touch transforms).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppView {
    pub view_matrix: CubismViewMatrix,
    pub device_to_screen: CubismMatrix44,
}

impl AppView {
    pub fn new() -> Self {
        Self::default()
    }

    /// `LAppView.initialize(subdelegate)` for a `width × height` canvas backing store (run at init
    /// and on every resize). A 0×0 canvas yields NaN rectangles, harmlessly, like V1.
    pub fn initialize(&mut self, width: u32, height: u32) {
        let (width, height) = (f64::from(width), f64::from(height));
        let ratio = width / height;
        let left = -ratio;
        let right = ratio;
        let bottom = VIEW_LOGICAL_LEFT; // sic: V1 uses the Left/Right constants for bottom/top
        let top = VIEW_LOGICAL_RIGHT;
        self.view_matrix.set_screen_rect(left, right, bottom, top);
        self.view_matrix.scale(VIEW_SCALE, VIEW_SCALE);
        self.device_to_screen.load_identity();
        if width > height {
            let screen_w = (right - left).abs();
            self.device_to_screen
                .scale_relative(screen_w / width, -screen_w / width);
        } else {
            let screen_h = (top - bottom).abs();
            self.device_to_screen
                .scale_relative(screen_h / height, -screen_h / height);
        }
        self.device_to_screen
            .translate_relative(-width * 0.5, -height * 0.5);
        self.view_matrix.set_max_scale(VIEW_MAX_SCALE);
        self.view_matrix.set_min_scale(VIEW_MIN_SCALE);
        self.view_matrix.set_max_screen_rect(
            VIEW_LOGICAL_MAX_LEFT,
            VIEW_LOGICAL_MAX_RIGHT,
            VIEW_LOGICAL_MAX_BOTTOM,
            VIEW_LOGICAL_MAX_TOP,
        );
    }
}

/// First half of the `onUpdate` projection, built before the model update: identity, then — only
/// while the Core model exists — `scale(h / w, 1)` (or, for a model wider than one unit in a
/// portrait canvas, `modelMatrix.setWidth(2)` + `scale(1, w / h)`, a QUIRK that permanently
/// mutates the model matrix; never taken for the shipped model), then `multiplyByMatrix(view)`.
pub fn base_projection(
    canvas_width: u32,
    canvas_height: u32,
    model: Option<(f64, &mut CubismModelMatrix)>,
    view_matrix: &CubismMatrix44,
) -> CubismMatrix44 {
    let mut projection = CubismMatrix44::new();
    if let Some((model_canvas_width, model_matrix)) = model {
        let (width, height) = (f64::from(canvas_width), f64::from(canvas_height));
        if model_canvas_width > 1.0 && width < height {
            model_matrix.set_width(2.0);
            projection.scale(1.0, width / height);
        } else {
            projection.scale(height / width, 1.0);
        }
        projection.multiply_by_matrix(view_matrix);
    }
    projection
}

/// Second half, applied after the driver: `width < height` (portrait backing store) →
/// `scaleRelative(3.3, 3.3); translateY(-2.1)`, otherwise (also square) →
/// `scaleRelative(4.1, 4.1); translateY(-2.75)`. `translateY` is absolute.
pub fn apply_upper_body_zoom(
    projection: &mut CubismMatrix44,
    canvas_width: u32,
    canvas_height: u32,
) {
    if canvas_width < canvas_height {
        projection.scale_relative(MOBILE_SCALE, MOBILE_SCALE);
        projection.translate_y_to(MOBILE_TRANSLATE_Y);
    } else {
        projection.scale_relative(DESKTOP_SCALE, DESKTOP_SCALE);
        projection.translate_y_to(DESKTOP_TRANSLATE_Y);
    }
}

/// `LAppModel.draw(matrix)`: `matrix.multiplyByMatrix(modelMatrix)` → the MVP the renderer gets.
pub fn model_view_projection(
    mut projection: CubismMatrix44,
    model_matrix: &CubismModelMatrix,
) -> CubismMatrix44 {
    projection.multiply_by_matrix(model_matrix.matrix());
    projection
}

/// The whole per-frame MVP for a model of `model_canvas_size` units (pipeline spec §8.2/§8.3).
pub fn frame_mvp(
    canvas_width: u32,
    canvas_height: u32,
    model_canvas_size: (f64, f64),
    model_matrix: &mut CubismModelMatrix,
    view_matrix: &CubismMatrix44,
) -> CubismMatrix44 {
    let mut projection = base_projection(
        canvas_width,
        canvas_height,
        Some((model_canvas_size.0, model_matrix)),
        view_matrix,
    );
    apply_upper_body_zoom(&mut projection, canvas_width, canvas_height);
    model_view_projection(projection, model_matrix)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KURISU: (f64, f64) = (1.0, 1.4285714285714286);

    fn mvp(w: u32, h: u32) -> [f32; 16] {
        let mut view = AppView::new();
        view.initialize(w, h);
        let mut model_matrix = CubismModelMatrix::new(KURISU.0, KURISU.1);
        *frame_mvp(w, h, KURISU, &mut model_matrix, view.view_matrix.matrix()).array()
    }

    /// Pipeline spec §8.3 (real bundle) and render spec §3.9 (faithful Float32Array replica).
    #[test]
    fn mvp_golden_values() {
        #[rustfmt::skip]
        let cases: &[(u32, u32, f64, f64, f64)] = &[
            (1920, 1045, 3.12411451, 5.73999977, -2.75),
            (2880, 1567, 3.12311816, 5.73999977, -2.75),
            (1280, 720, 3.22874975, 5.73999977, -2.75),
            (1000, 1000, 5.73999977, 5.73999977, -2.75),
            (1170, 2532, 9.99815369, 4.61999989, -2.0999999),
            (750, 1334, 8.21743965, 4.61999989, -2.0999999),
            (1920, 1080, 3.2287497520446777, 5.739999771118164, -2.75),
            (2560, 1440, 3.2287497520446777, 5.739999771118164, -2.75),
            (1366, 768, 3.2271740436553955, 5.739999771118164, -2.75),
            (800, 800, 5.739999771118164, 5.739999771118164, -2.75),
            (390, 844, 9.998153686523438, 4.619999885559082, -2.0999999046325684),
            (1080, 1920, 8.213333129882812, 4.619999885559082, -2.0999999046325684),
        ];
        for &(w, h, tr0, tr5, tr13) in cases {
            let m = mvp(w, h);
            assert_eq!(m[0], tr0 as f32, "{w}x{h} tr[0]");
            assert_eq!(m[5], tr5 as f32, "{w}x{h} tr[5]");
            assert_eq!(m[13], tr13 as f32, "{w}x{h} tr[13]");
            assert_eq!(m[12], 0.0, "{w}x{h} tr[12]");
            for (i, v) in m.iter().enumerate() {
                if ![0, 5, 12, 13].contains(&i) {
                    let identity = if i % 5 == 0 { 1.0 } else { 0.0 };
                    assert_eq!(*v, identity, "{w}x{h} tr[{i}]");
                }
            }
        }
    }

    #[test]
    fn view_initialize_is_identity_with_limits() {
        let mut view = AppView::new();
        view.initialize(1920, 1045);
        assert_eq!(*view.view_matrix.array(), CubismMatrix44::IDENTITY);
        assert_eq!(view.view_matrix.max_scale(), 2.0);
        assert_eq!(view.view_matrix.min_scale(), 0.8);
        assert_eq!(view.view_matrix.screen_bottom(), -1.0);
        assert_eq!(view.view_matrix.max_top(), 2.0);
        let r = 1920.0 / 1045.0;
        assert_eq!(view.view_matrix.screen_left(), -r);
        // device → screen: x' = 2r/w · (x − w/2)
        let d = view.device_to_screen;
        assert_eq!(d.scale_x(), (2.0 * r / 1920.0) as f32);
        assert_eq!(d.scale_y(), (-(2.0 * r) / 1920.0) as f32);
        // Portrait uses the height.
        view.initialize(390, 844);
        assert_eq!(view.device_to_screen.scale_x(), (2.0 / 844.0) as f32);
    }

    #[test]
    fn base_projection_without_model_is_identity() {
        let m = base_projection(1920, 1080, None, &CubismMatrix44::new());
        assert_eq!(*m.array(), CubismMatrix44::IDENTITY);
    }

    #[test]
    fn wide_model_in_portrait_mutates_model_matrix() {
        let mut model_matrix = CubismModelMatrix::new(2.0, 1.0);
        let before = *model_matrix.array();
        let p = base_projection(
            500,
            1000,
            Some((2.0, &mut model_matrix)),
            &CubismMatrix44::new(),
        );
        assert_eq!(p.scale_y(), 0.5);
        assert_eq!(p.scale_x(), 1.0);
        assert_ne!(*model_matrix.array(), before);
        assert_eq!(model_matrix.scale_x(), 1.0); // setWidth(2) on a 2-unit model
    }
}
