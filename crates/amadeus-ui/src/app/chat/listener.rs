//! Browser helpers of the chat panel: `window` listeners that own their closure (dropping one
//! removes it and frees the closure; `leptos::prelude::window_event_listener` leaks one closure
//! per registration, and the keyboard listener is re-registered on every keystroke like V1's
//! effect E14) and the JS number semantics of V1's timers and inline styles.

use std::time::Duration;

use leptos::prelude::{set_timeout_with_handle, TimeoutHandle};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

/// `window.addEventListener(event, handler)` until dropped (`removeEventListener`). Like a DOM
/// listener, one added while an event is being dispatched does not see that event, and one
/// removed before its turn is skipped.
pub struct WindowListener {
    event: &'static str,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

impl WindowListener {
    /// Adds the listener (bubble phase, no options — V1 passed none). `None` without a window.
    pub fn new(event: &'static str, handler: impl FnMut(web_sys::Event) + 'static) -> Option<Self> {
        let window = web_sys::window()?;
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(handler);
        window
            .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
            .ok()?;
        Some(Self { event, closure })
    }

    /// A `keydown` listener receiving the [`web_sys::KeyboardEvent`].
    pub fn keydown(mut handler: impl FnMut(&web_sys::KeyboardEvent) + 'static) -> Option<Self> {
        Self::new("keydown", move |event: web_sys::Event| {
            if let Some(event) = event.dyn_ref::<web_sys::KeyboardEvent>() {
                handler(event);
            }
        })
    }
}

impl Drop for WindowListener {
    fn drop(&mut self) {
        if let Some(window) = web_sys::window() {
            let _ = window.remove_event_listener_with_callback(
                self.event,
                self.closure.as_ref().unchecked_ref(),
            );
        }
    }
}

/// The delay `window.setTimeout(fn, delay)` really uses: WebIDL `long` conversion (ToInt32:
/// NaN/±∞ → 0, truncation toward zero, modulo 2^32), then HTML's "negative → 0". V1 passed
/// fractional typewriter delays (e.g. `50 / 0.7`) and `autoSpeed * 1000`.
pub fn js_timeout_ms(delay: f64) -> u64 {
    if !delay.is_finite() {
        return 0;
    }
    const TWO_32: f64 = 4_294_967_296.0;
    const TWO_31: f64 = 2_147_483_648.0;
    let modulo = delay.trunc().rem_euclid(TWO_32);
    let int32 = if modulo >= TWO_31 {
        modulo - TWO_32
    } else {
        modulo
    };
    if int32 <= 0.0 {
        0
    } else {
        // `int32` is an integer in 1..2^31, exactly representable.
        int32 as u64
    }
}

/// `window.setTimeout(callback, delay)` with V1's delay conversion ([`js_timeout_ms`]).
pub fn set_js_timeout(callback: impl FnOnce() + 'static, delay: f64) -> Option<TimeoutHandle> {
    set_timeout_with_handle(callback, Duration::from_millis(js_timeout_ms(delay))).ok()
}

/// JS `Math.round(x)`: the nearest integer, ties toward +∞ (`2.5 → 3`, `-2.5 → -2`); NaN and
/// ±∞ unchanged.
pub fn js_math_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let floor = x.floor();
    if x - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_conversion_matches_webidl_long() {
        assert_eq!(js_timeout_ms(50.0), 50);
        assert_eq!(js_timeout_ms(50.0 / 0.7), 71, "fractional delays truncate");
        assert_eq!(js_timeout_ms(1000.0 / 20.0 / 3.0), 16);
        assert_eq!(js_timeout_ms(3000.0), 3000);
        assert_eq!(js_timeout_ms(0.9), 0);
        assert_eq!(js_timeout_ms(-5.0), 0);
        assert_eq!(js_timeout_ms(f64::NAN), 0);
        assert_eq!(js_timeout_ms(f64::INFINITY), 0);
        assert_eq!(js_timeout_ms(2_147_483_647.0), 2_147_483_647);
        assert_eq!(
            js_timeout_ms(2_147_483_648.0),
            0,
            "wraps to a negative long"
        );
        assert_eq!(js_timeout_ms(4_294_967_296.0 + 7.0), 7, "modulo 2^32");
    }

    #[test]
    fn math_round_ties_toward_positive_infinity() {
        assert_eq!(js_math_round(49.5), 50.0);
        assert_eq!(js_math_round(-2.5), -2.0);
        assert_eq!(js_math_round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(js_math_round(73.2), 73.0);
        assert!(js_math_round(f64::NAN).is_nan());
    }
}
