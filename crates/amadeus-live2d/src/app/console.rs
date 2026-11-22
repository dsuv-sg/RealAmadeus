//! Console output. V1's `[ReamaCubism] …` lines are printed verbatim with `console.*` (pipeline spec
//! §11 — they do not depend on a `log` backend being installed); per-frame failures are reported
//! once per distinct message (D5). On host builds (unit tests) printing is a no-op.

use std::collections::HashSet;

use wasm_bindgen::JsValue;

/// `console.log(message)`.
pub fn log(message: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&JsValue::from_str(message));
    #[cfg(not(target_arch = "wasm32"))]
    let _ = message;
}

/// `console.warn(message)`.
pub fn warn(message: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&JsValue::from_str(message));
    #[cfg(not(target_arch = "wasm32"))]
    let _ = message;
}

/// `console.error(message)`.
pub fn error(message: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::error_1(&JsValue::from_str(message));
    #[cfg(not(target_arch = "wasm32"))]
    let _ = message;
}

/// `console.error(message, detail)` (V1 passes the error object as a second argument).
pub fn error_with(message: &str, detail: &JsValue) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::error_2(&JsValue::from_str(message), detail);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (message, detail);
}

/// Remembers which messages were already reported (D5: "log once per distinct message").
#[derive(Debug, Default, Clone)]
pub struct LogOnce {
    seen: HashSet<String>,
}

impl LogOnce {
    pub fn new() -> Self {
        Self::default()
    }

    /// True the first time `message` is seen (the caller then prints it).
    pub fn first(&mut self, message: &str) -> bool {
        if self.seen.contains(message) {
            false
        } else {
            // Bounded: a pathological stream of distinct messages must not grow without limit.
            if self.seen.len() >= 64 {
                self.seen.clear();
            }
            self.seen.insert(message.to_owned());
            true
        }
    }

    /// `console.error(message)` once per distinct message.
    pub fn error(&mut self, message: &str) {
        if self.first(message) {
            error(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LogOnce;

    #[test]
    fn log_once_per_message() {
        let mut once = LogOnce::new();
        assert!(once.first("a"));
        assert!(!once.first("a"));
        assert!(once.first("b"));
    }
}
