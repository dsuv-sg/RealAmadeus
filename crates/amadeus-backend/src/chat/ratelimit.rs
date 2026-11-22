//! Per-IP sliding window and the global FreeAPI counter (`rate-limit.ts`, spec §10). One
//! instance is shared by `/chat`, `/chat/stream` and `/chat/quota` (web server only).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::payload::js_parse_int;
use super::texts::{global_cap_message, ERR_RATE_LIMITED};

/// `RATE_LIMIT_WINDOW_MS`.
pub const RATE_LIMIT_WINDOW_MS: u64 = 60_000;
/// `MAX_REQUESTS_PER_WINDOW`.
pub const MAX_REQUESTS_PER_WINDOW: usize = 20;
/// V1's `MAX_GLOBAL_REQUESTS` (FreeAPI cap for the whole server): the default of
/// [`crate::BackendConfig::free_api_max`] (`AMADEUS_FREEAPI_MAX`).
pub const MAX_GLOBAL_REQUESTS: i64 = 65_000;
/// File name of the persisted global counter (`String(count)`, no newline).
pub const REQUEST_COUNT_FILE: &str = ".amadeus_request_count";

/// Result of [`RateLimiter::check`] (`{ allowed, errorMsg, count }`; V1's `remaining` was
/// never read by any caller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RateDecision {
    pub(crate) allowed: bool,
    pub(crate) error_msg: Option<String>,
    pub(crate) count: i64,
}

struct RateState {
    per_ip: HashMap<String, Vec<u64>>,
    global: i64,
    last_prune: u64,
}

/// Shared limiter state behind a mutex (V1 relied on single-threaded JS for atomicity).
pub(crate) struct RateLimiter {
    state: Mutex<RateState>,
    state_file: Option<PathBuf>,
    window_ms: u64,
    per_window: usize,
    global_max: i64,
}

impl RateLimiter {
    /// Limiter with V1's per-IP window, the configured global cap `global_max`
    /// ([`crate::BackendConfig::free_api_max`]) and the counter loaded from `state_file`
    /// (module-load logic).
    pub(crate) fn load(state_file: PathBuf, global_max: i64) -> Self {
        let global = load_count(&state_file);
        Self::with_limits(
            Some(state_file),
            global,
            RATE_LIMIT_WINDOW_MS,
            MAX_REQUESTS_PER_WINDOW,
            global_max,
        )
    }

    /// Limiter with explicit limits (`state_file: None` = no persistence).
    pub(crate) fn with_limits(
        state_file: Option<PathBuf>,
        global: i64,
        window_ms: u64,
        per_window: usize,
        global_max: i64,
    ) -> Self {
        Self {
            state: Mutex::new(RateState {
                per_ip: HashMap::new(),
                global,
                last_prune: 0,
            }),
            state_file,
            window_ms,
            per_window,
            global_max,
        }
    }

    /// Configured global cap (quota `max`).
    pub(crate) fn global_max(&self) -> i64 {
        self.global_max
    }

    /// `getGlobalRequestCount()`.
    pub(crate) fn global_count(&self) -> i64 {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).global
    }

    /// `checkRateLimit(req, provider)` at monotonic time `now_ms`.
    pub(crate) fn check(&self, ip: &str, counts_global: bool, now_ms: u64) -> RateDecision {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if counts_global && st.global >= self.global_max {
            return RateDecision {
                allowed: false,
                error_msg: Some(global_cap_message(self.global_max)),
                count: st.global,
            };
        }
        self.prune(&mut st, now_ms);
        let window = self.window_ms;
        let mut valid: Vec<u64> = st
            .per_ip
            .get(ip)
            .map(|ts| {
                ts.iter()
                    .copied()
                    .filter(|&t| now_ms.saturating_sub(t) < window)
                    .collect()
            })
            .unwrap_or_default();
        if valid.len() >= self.per_window {
            // Denied attempts are not recorded.
            st.per_ip.insert(ip.to_owned(), valid);
            return RateDecision {
                allowed: false,
                error_msg: Some(ERR_RATE_LIMITED.to_owned()),
                count: st.global,
            };
        }
        valid.push(now_ms);
        st.per_ip.insert(ip.to_owned(), valid);
        if counts_global {
            st.global = st.global.saturating_add(1);
            if let Some(path) = &self.state_file {
                save_count(path, st.global);
            }
        }
        RateDecision {
            allowed: true,
            error_msg: None,
            count: st.global,
        }
    }

    /// Drop IPs without any timestamp inside the window, at most once per window (V2 Q16).
    fn prune(&self, st: &mut RateState, now_ms: u64) {
        if now_ms.saturating_sub(st.last_prune) < self.window_ms {
            return;
        }
        st.last_prune = now_ms;
        let window = self.window_ms;
        st.per_ip
            .retain(|_, ts| ts.iter().any(|&t| now_ms.saturating_sub(t) < window));
    }

    #[cfg(test)]
    fn tracked_ips(&self) -> usize {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .per_ip
            .len()
    }
}

/// Startup load: `parseInt(file, 10)` if the file exists; errors are logged and ignored.
fn load_count(path: &Path) -> i64 {
    match std::fs::read(path) {
        Ok(bytes) => js_parse_int(&String::from_utf8_lossy(&bytes)).unwrap_or(0),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => {
            super::console_error(&format!("Failed to load request count {e}"));
            0
        }
    }
}

/// `saveState()`: same format as V1, written atomically (temp file + rename, V2 Q17).
fn save_count(path: &Path, count: i64) {
    let tmp = path.with_extension("tmp");
    let result = std::fs::write(&tmp, count.to_string()).and_then(|()| std::fs::rename(&tmp, path));
    if let Err(e) = result {
        super::console_error(&format!("Failed to save request count {e}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limiter() -> RateLimiter {
        RateLimiter::with_limits(
            None,
            0,
            RATE_LIMIT_WINDOW_MS,
            MAX_REQUESTS_PER_WINDOW,
            MAX_GLOBAL_REQUESTS,
        )
    }

    #[test]
    fn twenty_allowed_twenty_first_denied() {
        let rl = limiter();
        for i in 0..20 {
            let d = rl.check("1.2.3.4", false, 1000 + i);
            assert!(d.allowed, "request {i}");
        }
        let d = rl.check("1.2.3.4", false, 2000);
        assert!(!d.allowed);
        assert_eq!(d.error_msg.as_deref(), Some(ERR_RATE_LIMITED));
        // Other IPs are independent.
        assert!(rl.check("5.6.7.8", false, 2000).allowed);
    }

    #[test]
    fn denied_attempts_are_not_recorded_and_window_is_strict() {
        let rl = limiter();
        for _ in 0..20 {
            assert!(rl.check("ip", false, 0).allowed);
        }
        for t in [1, 10, 59_999] {
            assert!(!rl.check("ip", false, t).allowed);
        }
        // now - t < 60000 is strict: at exactly 60000 ms the first timestamps expire.
        assert!(rl.check("ip", false, 60_000).allowed);
    }

    #[test]
    fn global_counter_only_for_free_or_missing() {
        let rl = RateLimiter::with_limits(
            None,
            64_998,
            RATE_LIMIT_WINDOW_MS,
            MAX_REQUESTS_PER_WINDOW,
            MAX_GLOBAL_REQUESTS,
        );
        let d = rl.check("a", true, 0);
        assert_eq!((d.allowed, d.count), (true, 64_999));
        let d = rl.check("b", false, 0);
        assert_eq!(
            (d.allowed, d.count),
            (true, 64_999),
            "non-free providers report the current count"
        );
        let d = rl.check("c", true, 0);
        assert_eq!((d.allowed, d.count), (true, 65_000));
        let d = rl.check("d", true, 0);
        assert!(!d.allowed);
        assert_eq!(d.error_msg, Some(global_cap_message(65_000)));
        assert_eq!(d.count, 65_000);
        // The cap does not apply to other providers, and the denial was not recorded for "d".
        assert!(rl.check("d", false, 0).allowed);
        assert_eq!(rl.global_count(), 65_000);
    }

    #[test]
    fn cap_message_is_verbatim() {
        assert_eq!(
            global_cap_message(65_000),
            "FreeAPIの全体リクエスト上限（65000回）に達しました。サービスを停止しています。"
        );
    }

    #[test]
    fn stale_ips_are_pruned() {
        let rl = limiter();
        rl.check("old", false, 0);
        rl.check("new", false, 70_000);
        assert_eq!(rl.tracked_ips(), 1);
    }

    #[test]
    fn counter_file_roundtrip() {
        let dir = std::env::temp_dir().join(format!("amadeus-rl-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join(REQUEST_COUNT_FILE);
        let _ = std::fs::write(&file, "41\n");
        let rl = RateLimiter::load(file.clone(), MAX_GLOBAL_REQUESTS);
        assert_eq!(rl.global_count(), 41);
        assert_eq!(rl.global_max(), MAX_GLOBAL_REQUESTS);
        assert!(rl.check("x", true, 0).allowed);
        assert_eq!(std::fs::read_to_string(&file).ok().as_deref(), Some("42"));
        let _ = std::fs::write(&file, "garbage");
        assert_eq!(
            RateLimiter::load(file, MAX_GLOBAL_REQUESTS).global_count(),
            0
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn configured_cap_applies_and_is_interpolated() {
        let dir = std::env::temp_dir().join(format!("amadeus-rl-cap-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join(REQUEST_COUNT_FILE);
        let _ = std::fs::write(&file, "2");
        let rl = RateLimiter::load(file.clone(), 3);
        assert_eq!((rl.global_count(), rl.global_max()), (2, 3));
        let d = rl.check("a", true, 0);
        assert_eq!((d.allowed, d.count), (true, 3));
        let d = rl.check("b", true, 0);
        assert!(!d.allowed);
        assert_eq!(
            d.error_msg.as_deref(),
            Some("FreeAPIの全体リクエスト上限（3回）に達しました。サービスを停止しています。")
        );
        assert_eq!(d.count, 3);
        // Other providers are not capped; the file holds the counter, not the cap.
        assert!(rl.check("b", false, 0).allowed);
        assert_eq!(std::fs::read_to_string(&file).ok().as_deref(), Some("3"));
        // A higher cap re-enables FreeAPI for a counter V1 left at 65000.
        let _ = std::fs::write(&file, "65000");
        let rl = RateLimiter::load(file, 70_000);
        let d = rl.check("c", true, 0);
        assert_eq!((d.allowed, d.count), (true, 65_001));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
