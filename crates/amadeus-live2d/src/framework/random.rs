//! `Math.random()` for the Framework (eye blink timing). On wasm this is `js_sys::Math::random()`
//! exactly like V1; host builds (unit tests only) use a deterministic xorshift generator because
//! JS imports cannot run there. Components that consume randomness accept an injectable source.

#[cfg(not(target_arch = "wasm32"))]
use std::cell::Cell;

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static STATE: Cell<u64> = const { Cell::new(0x9E37_79B9_7F4A_7C15) };
}

/// Uniform random number in `[0, 1)` (`Math.random()`).
#[cfg(target_arch = "wasm32")]
pub fn js_random() -> f64 {
    js_sys::Math::random()
}

/// Uniform random number in `[0, 1)` (deterministic host fallback).
#[cfg(not(target_arch = "wasm32"))]
pub fn js_random() -> f64 {
    STATE.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        (x >> 11) as f64 / (1u64 << 53) as f64
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn in_unit_interval() {
        for _ in 0..1000 {
            let r = super::js_random();
            assert!((0.0..1.0).contains(&r));
        }
    }
}
