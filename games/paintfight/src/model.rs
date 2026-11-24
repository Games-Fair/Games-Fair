// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};
pub const WIDTH: f64 = 360.0;
pub const HEIGHT: f64 = 560.0;
pub const PAINT_W: usize = 180;
pub const PAINT_H: usize = 280;
pub use gamekit::multiplayer::Vec2;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rider {
    pub pos: Vec2,
    pub angle: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub humans: usize,
    pub count: usize,
    pub riders: Vec<Rider>,
    pub paint: Vec<u8>,
    pub scores: [u32; 4],
    pub elapsed: f64,
    pub countdown: f64,
    pub finished: bool,
    #[serde(default)]
    pub round_wins: [u32; 4],
    #[serde(default)]
    pub recorded: bool,
    pub steps: u64,
    #[serde(default)]
    pub pickup: Option<Vec2>,
    #[serde(default)]
    pub pickup_burst: bool,
    #[serde(default)]
    pub burst: Option<(Vec2, f64, usize)>,
    #[serde(default)]
    pub next_pickup: f64,
    #[serde(default)]
    pub boosted_until: [f64; 4],
    #[serde(skip)]
    pub collected: bool,
    accumulator: f64,
}
impl Game {
    pub fn new(humans: usize) -> Self {
        let humans = humans.clamp(1, 4);
        let count = humans.max(2);
        let mut positions = [
            Vec2::new(290.0, 460.0),
            Vec2::new(70.0, 100.0),
            Vec2::new(290.0, 100.0),
            Vec2::new(70.0, 460.0),
        ];
        let angles = {
            for p in &mut positions {
                p.y = if p.y < HEIGHT / 2.0 { 145.0 } else { 415.0 };
            }
            [-3.0 * PI / 4.0, PI / 4.0, 3.0 * PI / 4.0, -PI / 4.0]
        };
        let riders = (0..count)
            .map(|i| Rider {
                pos: positions[i],
                angle: angles[i],
            })
            .collect();
        Self {
            humans,
            count,
            riders,
            paint: vec![0; PAINT_W * PAINT_H],
            scores: [0; 4],
            elapsed: 0.0,
            countdown: 3.0,
            finished: false,
            round_wins: [0; 4],
            recorded: false,
            steps: 0,
            pickup: None,
            pickup_burst: false,
            burst: None,
            next_pickup: 5.0,
            boosted_until: [0.0; 4],
            collected: false,
            accumulator: 0.0,
        }
    }
    pub fn valid(&self) -> bool {
        (1..=4).contains(&self.humans)
            && self.count == self.humans.max(2)
            && self.elapsed.is_finite()
            && (0.0..=3600.0).contains(&self.elapsed)
            && self.countdown.is_finite()
            && (0.0..=3.0).contains(&self.countdown)
            && self.accumulator.is_finite()
            && (0.0..=1.0).contains(&self.accumulator)
            && self.riders.len() == self.count
            && self
                .riders
                .iter()
                .all(|r| r.pos.valid() && r.angle.is_finite())
            && self.paint.len() == PAINT_W * PAINT_H
            && self.paint.iter().all(|&v| v as usize <= self.count)
            && self.scores.iter().all(|&s| s <= (PAINT_W * PAINT_H) as u32)
            && self.pickup.is_none_or(|p| {
                p.valid() && (0.0..=WIDTH).contains(&p.x) && (0.0..=HEIGHT).contains(&p.y)
            })
            && self.burst.is_none_or(|(p, t, i)| {
                p.valid() && t.is_finite() && (0.0..=45.0).contains(&t) && i < self.count
            })
            && self.next_pickup.is_finite()
            && (0.0..=3600.0).contains(&self.next_pickup)
            && self
                .boosted_until
                .iter()
                .all(|t| t.is_finite() && (0.0..=3600.0).contains(t))
    }
    pub fn needs_clock(&self) -> bool {
        !self.finished
    }
    pub fn tick(&mut self, dt: f64, controls: [Option<Vec2>; 4]) {
        if self.finished || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.collected = false;
        let active = if self.countdown > 0.0 {
            let used = dt.min(self.countdown);
            self.countdown -= used;
            dt - used
        } else {
            dt
        };
        if active <= 0.0 {
            return;
        }
        self.elapsed += active;
        self.accumulator += active.min(0.1);
        while self.accumulator >= 1.0 / 120.0 && !self.finished {
            self.accumulator -= 1.0 / 120.0;
            self.steps += 1;
            self.rider_step(1.0 / 120.0, controls)
        }
        if self.pickup.is_none() && self.elapsed >= self.next_pickup && self.elapsed < 40.0 {
            let n = (self.elapsed / 5.0) as usize;
            self.pickup_burst = n.is_multiple_of(2);
            self.pickup = Some(Vec2::new(
                90.0 + (n % 3) as f64 * 90.0,
                175.0 + (n % 2) as f64 * 210.0,
            ));
        }
        self.scores = [0; 4];
        for &owner in &self.paint {
            if owner > 0 {
                self.scores[owner as usize - 1] += 1;
            }
        }
        if self.elapsed >= 45.0 {
            self.elapsed = 45.0;
            self.finished = true;
        }
    }
    fn rider_step(&mut self, dt: f64, controls: [Option<Vec2>; 4]) {
        for (i, control) in controls.iter().enumerate().take(self.count) {
            let target = if i >= self.humans {
                Some(self.ai_direction(i))
            } else {
                *control
            };
            let direction = target.filter(|v| v.x.hypot(v.y) > 0.1);
            if direction.is_none() {
                continue;
            }
            let r = &mut self.riders[i];
            if let Some(dir) = direction {
                let desired = dir.y.atan2(dir.x);
                r.angle = desired;
            }
            let speed = if self.boosted_until[i] > self.elapsed {
                180.0
            } else {
                100.0
            };
            r.pos.x += r.angle.cos() * speed * dt;
            r.pos.y += r.angle.sin() * speed * dt;
            r.pos.x = r.pos.x.clamp(0.0, WIDTH);
            r.pos.y = r.pos.y.clamp(0.0, HEIGHT);
            r.angle = r.angle.rem_euclid(TAU);
        }
        for offset in 0..self.count {
            let i = (offset + self.steps as usize) % self.count;
            if i >= self.humans || controls[i].is_some_and(|v| v.x.hypot(v.y) > 0.1) {
                self.paint_at(i);
                if self
                    .pickup
                    .is_some_and(|p| p.distance(self.riders[i].pos) < 33.0)
                {
                    self.pickup = None;
                    if self.pickup_burst {
                        self.paint_burst(i);
                    } else {
                        self.boosted_until[i] = self.elapsed + 5.0;
                    }
                    self.next_pickup = self.elapsed + 5.0;
                    self.collected = true;
                }
            }
        }
    }
    fn paint_at(&mut self, i: usize) {
        let r = &self.riders[i];
        let cx = r.pos.x / WIDTH * PAINT_W as f64;
        let cy = r.pos.y / HEIGHT * PAINT_H as f64;
        for y in (cy as isize - 16).max(0)..=(cy as isize + 16).min(PAINT_H as isize - 1) {
            for x in (cx as isize - 16).max(0)..=(cx as isize + 16).min(PAINT_W as isize - 1) {
                let dx = (x as f64 - cx) * WIDTH / PAINT_W as f64;
                let dy = (y as f64 - cy) * HEIGHT / PAINT_H as f64;
                let along = dx * r.angle.cos() + dy * r.angle.sin();
                let across = -dx * r.angle.sin() + dy * r.angle.cos();
                if along.abs() <= 7.0 && across.abs() <= 29.0 {
                    self.paint[y as usize * PAINT_W + x as usize] = i as u8 + 1;
                }
            }
        }
    }
    fn paint_burst(&mut self, i: usize) {
        let center = self.riders[i].pos;
        self.burst = Some((center, self.elapsed, i));
        for y in 0..PAINT_H {
            for x in 0..PAINT_W {
                let dx = (x as f64 + 0.5) * WIDTH / PAINT_W as f64 - center.x;
                let dy = (y as f64 + 0.5) * HEIGHT / PAINT_H as f64 - center.y;
                let a = dy.atan2(dx);
                let ray = (0.5 + 0.5 * (a * 11.0 + i as f64).cos()).powi(5);
                if dx.hypot(dy) < 70.0 + 250.0 * ray {
                    self.paint[y * PAINT_W + x] = i as u8 + 1;
                }
            }
        }
    }
    fn ai_direction(&self, i: usize) -> Vec2 {
        let r = &self.riders[i];
        let mut best = (f64::NEG_INFINITY, r.angle);
        for offset in [-1.1, -0.55, 0.0, 0.55, 1.1] {
            let a = r.angle + offset;
            let p = Vec2::new(r.pos.x + a.cos() * 55.0, r.pos.y + a.sin() * 55.0);
            let edge = p.x.min(WIDTH - p.x).min(p.y).min(HEIGHT - p.y);
            let score = {
                let x = (p.x / WIDTH * PAINT_W as f64).clamp(0.0, (PAINT_W - 1) as f64) as usize;
                let y = (p.y / HEIGHT * PAINT_H as f64).clamp(0.0, (PAINT_H - 1) as f64) as usize;
                edge.min(30.0)
                    + if self.paint[y * PAINT_W + x] == i as u8 + 1 {
                        -30.0
                    } else {
                        25.0
                    }
                    - offset.abs()
            };
            if score > best.0 {
                best = (score, a);
            }
        }
        Vec2::new(best.1.cos(), best.1.sin())
    }
    /// A completed round contributes once, including when restored before the result animation.
    pub fn record_result(&mut self) {
        if self.finished && !self.recorded {
            let winners = self.winners();
            if winners.len() == 1 {
                self.round_wins[winners[0]] = self.round_wins[winners[0]].saturating_add(1);
            }
            self.recorded = true;
        }
    }
    pub fn winners(&self) -> Vec<usize> {
        let best = self.scores[..self.count].iter().copied().max().unwrap_or(0);
        (0..self.count)
            .filter(|&i| self.scores[i] == best)
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paint_timer_ownership_and_multiplayer() {
        let mut g = Game::new(4);
        g.tick(2.0, [None; 4]);
        assert_eq!(g.elapsed, 0.0);
        for _ in 0..5400 {
            g.tick(1.0 / 120.0, [Some(Vec2::new(1.0, 0.0)); 4]);
        }
        g.tick(3.0, [None; 4]);
        assert!(g.finished);
        assert!(g.scores.iter().all(|&s| s > 0));
        assert!(g.scores.iter().sum::<u32>() <= (PAINT_W * PAINT_H) as u32);
        assert!(g.valid());
    }
    #[test]
    fn paint_stops_on_release_and_speed_pickups_expire() {
        let mut g = Game::new(2);
        g.countdown = 0.0;
        let start = g.riders[0].pos;
        g.tick(0.1, [None; 4]);
        assert_eq!(g.riders[0].pos.distance(start), 0.0);
        assert_eq!(g.scores, [0; 4]);
        g.pickup = Some(start);
        g.tick(0.1, [Some(Vec2::new(-1.0, 0.0)), None, None, None]);
        assert!(g.collected);
        assert!(g.pickup.is_none());
        assert!(g.boosted_until[0] > g.elapsed);
        let stopped = g.riders[0].pos;
        let paint = g.paint.clone();
        for _ in 0..60 {
            g.tick(0.1, [None; 4]);
        }
        assert_eq!(g.riders[0].pos.distance(stopped), 0.0);
        assert_eq!(g.paint, paint);
        assert!(g.boosted_until[0] < g.elapsed);
        assert!(g.pickup.is_some());
        let restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert!(restored.valid());
        assert_eq!(restored.boosted_until, g.boosted_until);
    }
    #[test]
    fn paint_burst_reclaims_rival_territory() {
        let mut g = Game::new(2);
        g.countdown = 0.0;
        g.paint.fill(2);
        g.riders[0].pos = Vec2::new(WIDTH / 2.0, HEIGHT / 2.0);
        g.pickup = Some(g.riders[0].pos);
        g.pickup_burst = true;
        g.tick(0.01, [Some(Vec2::new(1.0, 0.0)), None, None, None]);
        assert!(g.collected && g.burst.is_some());
        assert!(g.scores[0] > (PAINT_W * PAINT_H / 4) as u32);
        assert_eq!(g.scores[0] + g.scores[1], (PAINT_W * PAINT_H) as u32);
        assert_eq!(g.boosted_until[0], 0.0);
        assert!(g.valid());
    }
}
