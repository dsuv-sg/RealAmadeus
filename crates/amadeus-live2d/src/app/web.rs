//! Browser clocks and randomness exactly as V1 used them (`Date.now()`, `performance.now()`,
//! `Math.random()`), so headless tests can shim them from JS.

/// `Date.now()` (integer milliseconds) — Clock 1 of the SDK updaters.
pub fn date_now_ms() -> f64 {
    js_sys::Date::now()
}

/// `performance.now()` (falls back to `Date.now()` without a `Performance` object) — Clock 2.
pub fn performance_now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or_else(date_now_ms, |p| p.now())
}

/// `Math.random()`.
pub fn math_random() -> f64 {
    crate::framework::js_random()
}

/// `window.devicePixelRatio` (1 without a window).
pub fn device_pixel_ratio() -> f64 {
    web_sys::window().map_or(1.0, |w| w.device_pixel_ratio())
}
