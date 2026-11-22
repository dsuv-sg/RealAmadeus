//! Display-related settings rules from `RealAmadeusApp.tsx`: resolution table, screen mode,
//! the 1920×1080 virtual stage scaling and the mobile-layout predicate.

/// `Config_ScreenMode` value meaning fullscreen.
pub const SCREEN_MODE_FULLSCREEN: i64 = 0;
/// `Config_ScreenMode` value meaning windowed (the V1 default).
pub const SCREEN_MODE_WINDOWED: i64 = 1;

/// Labels of the desktop config "Resolution" select, index = `Config_Resolution`.
pub const RESOLUTION_LABELS: [&str; 3] = ["1920x1080", "1600x900", "1280x720"];

/// Width/height pair returned by [`resolve_resolution`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resolution {
    /// Width in CSS px.
    pub width: u32,
    /// Height in CSS px.
    pub height: u32,
}

/// V1 `resolveResolution(index)`: 1 → 1600×900, 2 → 1280×720, anything else → 1920×1080.
///
/// V1 web computes this but never uses the result (the stage always scales to the window);
/// the desktop build may use it for the window size.
pub fn resolve_resolution(index: i64) -> Resolution {
    match index {
        1 => Resolution {
            width: 1600,
            height: 900,
        },
        2 => Resolution {
            width: 1280,
            height: 720,
        },
        _ => Resolution {
            width: 1920,
            height: 1080,
        },
    }
}

/// Width of the virtual stage every V1 layout is authored for.
pub const STAGE_WIDTH: f64 = 1920.0;
/// Height of the virtual stage.
pub const STAGE_HEIGHT: f64 = 1080.0;

/// V1 root `updateDimensions`: `scale = min(innerWidth / 1920, innerHeight / 1080)`; the stage
/// is `1920 * scale` × `1080 * scale`. Returns `(width, height, scale)`.
pub fn fit_stage(inner_width: f64, inner_height: f64) -> (f64, f64, f64) {
    let scale = js_math_min(inner_width / STAGE_WIDTH, inner_height / STAGE_HEIGHT);
    (STAGE_WIDTH * scale, STAGE_HEIGHT * scale, scale)
}

/// `Math.min(a, b)` (NaN-propagating, unlike `f64::min`).
fn js_math_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// Visual-viewport width at or below which V1 switches to the mobile layout.
pub const MOBILE_MAX_WIDTH: f64 = 900.0;
/// Width limit for the mobile layout when the primary pointer is coarse (touch).
pub const MOBILE_MAX_WIDTH_COARSE: f64 = 1200.0;

/// V1 `isMobileViewportSnapshot` rule:
/// `visualWidth <= 900 || (coarsePointer && visualWidth <= 1200)` (both bounds inclusive).
/// `visual_width` is `visualViewport.width ?? innerWidth`; `coarse_pointer` is
/// `matchMedia("(pointer: coarse)").matches`.
pub fn is_mobile_viewport(visual_width: f64, coarse_pointer: bool) -> bool {
    visual_width <= MOBILE_MAX_WIDTH || (coarse_pointer && visual_width <= MOBILE_MAX_WIDTH_COARSE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_table_matches_v1() {
        assert_eq!(
            resolve_resolution(0),
            Resolution {
                width: 1920,
                height: 1080
            }
        );
        assert_eq!(
            resolve_resolution(1),
            Resolution {
                width: 1600,
                height: 900
            }
        );
        assert_eq!(
            resolve_resolution(2),
            Resolution {
                width: 1280,
                height: 720
            }
        );
        assert_eq!(
            resolve_resolution(3),
            Resolution {
                width: 1920,
                height: 1080
            }
        );
        assert_eq!(
            resolve_resolution(-1),
            Resolution {
                width: 1920,
                height: 1080
            }
        );
    }

    #[test]
    fn mobile_viewport_boundaries() {
        assert!(is_mobile_viewport(900.0, false));
        assert!(!is_mobile_viewport(900.5, false));
        assert!(is_mobile_viewport(900.5, true));
        assert!(is_mobile_viewport(1200.0, true));
        assert!(!is_mobile_viewport(1200.5, true));
        assert!(!is_mobile_viewport(1366.0, true));
        assert!(is_mobile_viewport(0.0, false));
    }

    #[test]
    fn stage_fit() {
        assert_eq!(fit_stage(1920.0, 1080.0), (1920.0, 1080.0, 1.0));
        assert_eq!(fit_stage(960.0, 1080.0), (960.0, 540.0, 0.5));
        assert_eq!(fit_stage(3840.0, 1080.0), (1920.0, 1080.0, 1.0));
    }
}
