//! `CubismTargetPoint` — port of `Framework/src/math/cubismtargetpoint.ts` (face direction
//! smoothing used by the look updater).
//!
//! Copyright(c) Live2D Inc. All rights reserved. Use of the original source code is governed by the
//! Live2D Open Software license (<https://www.live2d.com/eula/live2d-open-software-license-agreement_en.html>).
//!
//! All fields are JS numbers (`f64`); the update is copied operation by operation (including the
//! literal `+ 16·a·d − 8·a·d` of the original).

/// `FrameRate` of the original.
const FRAME_RATE: f64 = 30.0;
/// `Epsilon` of the original (target reached).
const EPSILON: f64 = 0.01;

/// Smoothly follows a drag target (`CubismTargetPoint`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CubismTargetPoint {
    face_target_x: f64,
    face_target_y: f64,
    face_x: f64,
    face_y: f64,
    face_vx: f64,
    face_vy: f64,
    last_time_seconds: f64,
    user_time_seconds: f64,
}

impl CubismTargetPoint {
    pub fn new() -> Self {
        Self::default()
    }

    /// `update(deltaTimeSeconds)`.
    pub fn update(&mut self, delta_time_seconds: f64) {
        self.user_time_seconds += delta_time_seconds;

        let face_param_max_v = 40.0 / 10.0;
        let max_v = face_param_max_v * 1.0 / FRAME_RATE;

        if self.last_time_seconds == 0.0 {
            self.last_time_seconds = self.user_time_seconds;
            return;
        }

        let delta_time_weight = (self.user_time_seconds - self.last_time_seconds) * FRAME_RATE;
        self.last_time_seconds = self.user_time_seconds;

        let time_to_max_speed = 0.15;
        let frame_to_max_speed = time_to_max_speed * FRAME_RATE;
        let max_a = delta_time_weight * max_v / frame_to_max_speed;

        let dx = self.face_target_x - self.face_x;
        let dy = self.face_target_y - self.face_y;

        if dx.abs() <= EPSILON && dy.abs() <= EPSILON {
            return;
        }

        let d = (dx * dx + dy * dy).sqrt();

        let vx = max_v * dx / d;
        let vy = max_v * dy / d;

        let mut ax = vx - self.face_vx;
        let mut ay = vy - self.face_vy;

        let a = (ax * ax + ay * ay).sqrt();

        if a < -max_a || a > max_a {
            ax *= max_a / a;
            ay *= max_a / a;
        }

        self.face_vx += ax;
        self.face_vy += ay;

        {
            // literally `+ 16·a·d − 8·a·d` like the original
            let max_v2 =
                0.5 * ((max_a * max_a + 16.0 * max_a * d - 8.0 * max_a * d).sqrt() - max_a);
            let cur_v = (self.face_vx * self.face_vx + self.face_vy * self.face_vy).sqrt();

            if cur_v > max_v2 {
                self.face_vx *= max_v2 / cur_v;
                self.face_vy *= max_v2 / cur_v;
            }
        }

        self.face_x += self.face_vx;
        self.face_y += self.face_vy;
    }

    /// `getX()`: the smoothed X in `[-1, 1]`.
    pub fn get_x(&self) -> f64 {
        self.face_x
    }

    /// `getY()`: the smoothed Y in `[-1, 1]`.
    pub fn get_y(&self) -> f64 {
        self.face_y
    }

    /// Alias of [`CubismTargetPoint::get_x`].
    pub fn x(&self) -> f64 {
        self.face_x
    }

    /// Alias of [`CubismTargetPoint::get_y`].
    pub fn y(&self) -> f64 {
        self.face_y
    }

    /// `set(x, y)`: the drag target.
    pub fn set(&mut self, x: f64, y: f64) {
        self.face_target_x = x;
        self.face_target_y = y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_update_only_records_time() {
        let mut tp = CubismTargetPoint::new();
        tp.set(1.0, 0.0);
        tp.update(1.0 / 60.0);
        assert_eq!(tp.get_x(), 0.0);
        tp.update(1.0 / 60.0);
        assert!(tp.get_x() > 0.0);
        assert_eq!(tp.get_y(), 0.0);
    }

    #[test]
    fn zero_delta_keeps_waiting() {
        let mut tp = CubismTargetPoint::new();
        tp.set(0.5, 0.5);
        for _ in 0..10 {
            tp.update(0.0);
        }
        assert_eq!((tp.get_x(), tp.get_y()), (0.0, 0.0));
    }

    #[test]
    fn converges_to_target() {
        let mut tp = CubismTargetPoint::new();
        tp.set(0.8, -0.5);
        for _ in 0..600 {
            tp.update(1.0 / 60.0);
        }
        assert!((tp.get_x() - 0.8).abs() <= 0.02);
        assert!((tp.get_y() + 0.5).abs() <= 0.02);
    }
}
