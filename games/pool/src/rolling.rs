// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! World-space rolling orientation, integrated from travel rather than frame count or speed.
use crate::model::BALL_R;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Rotation([f64; 4]);
impl Default for Rotation {
    fn default() -> Self {
        Self([0.0, 0.0, 0.0, 1.0])
    }
}
impl Rotation {
    pub fn valid(self) -> bool {
        self.0.iter().all(|v| v.is_finite())
            && (self.0.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() < 0.001
    }
    pub fn roll(&mut self, dx: f64, dy: f64) {
        let distance = dx.hypot(dy);
        if distance < 1e-10 {
            return;
        }
        let (s, c) = (distance / (2.0 * BALL_R)).sin_cos();
        let [x, y, z, w] = self.0;
        let a = -dy / distance * s;
        let b = dx / distance * s;
        // Pre-multiply the existing orientation by a world-space rotation about (-dy, dx, 0).
        let q = [
            c * x + a * w + b * z,
            c * y + b * w - a * z,
            c * z + a * y - b * x,
            c * w - a * x - b * y,
        ];
        let norm = q.iter().map(|v| v * v).sum::<f64>().sqrt();
        self.0 = q.map(|v| v / norm);
    }
    pub fn apply(self, v: [f64; 3]) -> [f64; 3] {
        let [x, y, z, w] = self.0;
        let t = [
            2.0 * (y * v[2] - z * v[1]),
            2.0 * (z * v[0] - x * v[2]),
            2.0 * (x * v[1] - y * v[0]),
        ];
        [
            v[0] + w * t[0] + y * t[2] - z * t[1],
            v[1] + w * t[1] + z * t[0] - x * t[2],
            v[2] + w * t[2] + x * t[1] - y * t[0],
        ]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{PI, TAU};
    fn close(a: [f64; 3], b: [f64; 3]) {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-9, "{a:?} != {b:?}");
        }
    }
    #[test]
    fn markings_follow_distance_and_direction_and_complete_one_revolution() {
        let mut q = Rotation::default();
        q.roll(BALL_R * PI / 2.0, 0.0);
        close(q.apply([0.0, 0.0, 1.0]), [1.0, 0.0, 0.0]);
        q.roll(-BALL_R * PI / 2.0, 0.0);
        close(q.apply([0.0, 0.0, 1.0]), [0.0, 0.0, 1.0]);
        q.roll(0.0, BALL_R * TAU);
        close(q.apply([0.0, 0.0, 1.0]), [0.0, 0.0, 1.0]);
    }
    #[test]
    fn rolling_is_frame_independent_and_stable_after_many_direction_changes() {
        let mut a = Rotation::default();
        let mut b = a;
        a.roll(80.0, -30.0);
        for _ in 0..100 {
            b.roll(0.8, -0.3);
        }
        close(a.apply([0.0, 0.0, 1.0]), b.apply([0.0, 0.0, 1.0]));
        for i in 0..10000 {
            a.roll((i as f64).sin(), (i as f64).cos());
        }
        assert!(a.valid());
        let restored: Rotation = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        close(a.apply([1.0, 0.0, 0.0]), restored.apply([1.0, 0.0, 0.0]));
    }
}
