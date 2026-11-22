//! The two V1 clocks (pipeline spec §7.2, §9.7 step 3; D8). Pure: the callers pass the time.

use super::define::PLAYER_DELTA_CAP;
use super::jsnum::{js_min, js_truthy};

/// Clock 1 — `LAppPal`: `Date.now()` deltas in seconds, **uncapped**, used by the SDK updaters
/// (look, breath, physics). V1 kept one page-global clock; V2 keeps one per player, primed with
/// the creation time (spec §7.2 fix; a single player behaves identically).
#[derive(Debug, Clone, PartialEq)]
pub struct FrameClock {
    last_frame_ms: f64,
    delta_seconds: f64,
}

impl FrameClock {
    /// A clock whose first [`FrameClock::update`] measures from `now_ms`.
    pub fn new(now_ms: f64) -> Self {
        Self {
            last_frame_ms: now_ms,
            delta_seconds: 0.0,
        }
    }

    /// `LAppPal.updateTime()`: `deltaTime = (now - lastFrame) / 1e3; lastFrame = now`.
    pub fn update(&mut self, now_ms: f64) {
        self.delta_seconds = (now_ms - self.last_frame_ms) / 1e3;
        self.last_frame_ms = now_ms;
    }

    /// `LAppPal.getDeltaTime()`.
    pub fn delta_seconds(&self) -> f64 {
        self.delta_seconds
    }
}

/// Clock 2 — the driver's `performance.now()` delta: the first call yields 0 (`if
/// (!this._lastUpdateTime) this._lastUpdateTime = now`, a JS falsy check), then
/// `min(now - last, 0.1)`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerClock {
    /// `_lastUpdateTime` (`undefined` and `0` are both falsy, so `0.0` stands for "unset").
    last_update_time: f64,
}

impl PlayerClock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances to `now_seconds` (`performance.now() / 1000.0`) and returns the capped delta.
    pub fn tick(&mut self, now_seconds: f64) -> f64 {
        if !js_truthy(self.last_update_time) {
            self.last_update_time = now_seconds;
        }
        let delta = js_min(now_seconds - self.last_update_time, PLAYER_DELTA_CAP);
        self.last_update_time = now_seconds;
        delta
    }

    /// The last time passed to [`PlayerClock::tick`] (0 before the first call).
    pub fn last_update_time(&self) -> f64 {
        self.last_update_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_clock_is_uncapped_ms_resolution() {
        let mut c = FrameClock::new(1_763_000_000_000.0);
        c.update(1_763_000_000_016.0);
        assert_eq!(c.delta_seconds(), 0.016);
        c.update(1_763_000_000_033.0);
        assert_eq!(c.delta_seconds(), 0.017);
        // A 6-s background-tab stall is passed through (physics has its own 5-s guard).
        c.update(1_763_000_006_033.0);
        assert_eq!(c.delta_seconds(), 6.0);
    }

    #[test]
    fn player_clock_first_call_zero_then_capped() {
        let mut c = PlayerClock::new();
        assert_eq!(c.tick(1000.0), 0.0);
        let d = c.tick(1000.0 + 1.0 / 60.0);
        assert!((d - 1.0 / 60.0).abs() < 1e-12);
        assert_eq!(c.tick(1005.0), 0.1);
        assert_eq!(c.last_update_time(), 1005.0);
    }
}
