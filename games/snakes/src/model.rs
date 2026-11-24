// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use serde::{Deserialize, Serialize};
use std::f64::consts::{PI, TAU};
pub const WIDTH: f64 = 360.0;
pub const HEIGHT: f64 = 560.0;
pub const BODY_SPACING: f64 = 3.0;
pub const SEGMENT_LENGTH: f64 = 11.0;
pub use gamekit::multiplayer::Vec2;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rider {
    pub pos: Vec2,
    pub angle: f64,
    pub alive: bool,
    pub body: Vec<Vec2>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub humans: usize,
    pub count: usize,
    pub riders: Vec<Rider>,
    pub scores: [u32; 4],
    pub elapsed: f64,
    pub countdown: f64,
    pub finished: bool,
    #[serde(default)]
    pub round_wins: [u32; 4],
    #[serde(default)]
    pub recorded: bool,
    pub steps: u64,
    pub growth: usize,
    pub next_growth: f64,
    accumulator: f64,
}
impl Game {
    pub fn new(humans: usize) -> Self {
        let humans = humans.clamp(1, 4);
        let count = humans.max(2);
        let positions = [
            Vec2::new(290.0, 460.0),
            Vec2::new(70.0, 100.0),
            Vec2::new(290.0, 100.0),
            Vec2::new(70.0, 460.0),
        ];
        let angles = [-PI / 2.0, PI / 2.0, PI / 2.0, -PI / 2.0];
        let riders = (0..count)
            .map(|i| Rider {
                pos: positions[i],
                angle: angles[i],
                alive: true,
                body: (0..7)
                    .map(|j| {
                        Vec2::new(
                            positions[i].x,
                            positions[i].y - angles[i].sin() * j as f64 * BODY_SPACING,
                        )
                    })
                    .collect(),
            })
            .collect();
        Self {
            humans,
            count,
            riders,
            scores: [0; 4],
            elapsed: 0.0,
            countdown: 3.0,
            finished: false,
            round_wins: [0; 4],
            recorded: false,
            steps: 0,
            growth: 0,
            next_growth: 1.0,
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
            && self.growth <= 1000
            && self.next_growth.is_finite()
            && self.riders.len() == self.count
            && self.riders.iter().all(|r| {
                r.pos.valid()
                    && r.angle.is_finite()
                    && r.body.len() <= 4096
                    && r.body.iter().all(|p| p.valid())
            })
            && self.scores.iter().all(|&s| s <= 1)
    }
    pub fn needs_clock(&self) -> bool {
        !self.finished
    }
    pub fn tick(&mut self, dt: f64, controls: [Option<Vec2>; 4]) {
        if self.finished || !dt.is_finite() || dt <= 0.0 {
            return;
        }
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

        self.accumulator += active.min(0.1);
        while self.accumulator >= 1.0 / 120.0 && !self.finished {
            self.accumulator -= 1.0 / 120.0;
            self.steps += 1;
            self.elapsed += 1.0 / 120.0;
            if self.elapsed >= self.next_growth {
                self.growth = (self.growth + 1).min(1000);
                // Growth intervals shorten from one second to 0.6 seconds over the first 30 seconds.
                self.next_growth += 1.0 - self.elapsed.min(30.0) / 75.0;
            }
            self.rider_step(1.0 / 120.0, controls)
        }
    }
    fn rider_step(&mut self, dt: f64, controls: [Option<Vec2>; 4]) {
        for (i, control) in controls.iter().enumerate().take(self.count) {
            if !self.riders[i].alive {
                continue;
            }
            let target = if i >= self.humans {
                Some(self.ai_direction(i))
            } else {
                *control
            };
            let direction = target.filter(|v| v.x.hypot(v.y) > 0.1);
            let r = &mut self.riders[i];
            if let Some(dir) = direction {
                let desired = dir.y.atan2(dir.x);
                let delta = (desired - r.angle + PI).rem_euclid(TAU) - PI;
                r.angle += delta.sin().atan2(1.0 / (4.0 * dt) - 1.0 + delta.cos());
            }
            let speed = 76.0;
            r.pos.x += r.angle.cos() * speed * dt;
            r.pos.y += r.angle.sin() * speed * dt;
            // Wrap the trail across the same seam as its head.
            r.pos.x = r.pos.x.rem_euclid(WIDTH);
            r.pos.y = r.pos.y.rem_euclid(HEIGHT);
            r.angle = r.angle.rem_euclid(TAU);
            if r.body
                .first()
                .is_none_or(|p| distance(*p, r.pos) >= BODY_SPACING)
            {
                r.body.insert(0, r.pos);
                r.body
                    .truncate(7 + (self.growth as f64 * SEGMENT_LENGTH / BODY_SPACING) as usize);
            }
        }
        let mut hit = [false; 4];
        for (i, value) in hit.iter_mut().enumerate().take(self.count) {
            if !self.riders[i].alive {
                continue;
            }
            *value = (0..self.count).any(|j| {
                i != j
                    && self.riders[j].alive
                    && (distance(self.riders[i].pos, self.riders[j].pos) < 21.0
                        || self.riders[j]
                            .body
                            .iter()
                            .any(|&p| distance(self.riders[i].pos, p) < 19.0))
            });
        }
        for (i, &hit) in hit.iter().enumerate().take(self.count) {
            if hit {
                self.riders[i].alive = false;
            }
        }
        if self.riders.iter().filter(|r| r.alive).count() <= 1 {
            self.finished = true;
            for (i, r) in self.riders.iter().enumerate() {
                self.scores[i] = u32::from(r.alive);
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
                let obstacle = self
                    .riders
                    .iter()
                    .enumerate()
                    .filter(|&(j, other)| j != i && other.alive)
                    .flat_map(|(_, other)| other.body.iter())
                    .map(|&q| distance(p, q))
                    .fold(200.0, f64::min);
                edge.min(obstacle - 18.0) - offset.abs() * 3.0
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
/// Shortest distance across either wrapped edge.
fn distance(a: Vec2, b: Vec2) -> f64 {
    let dx = (a.x - b.x).abs().rem_euclid(WIDTH);
    let dy = (a.y - b.y).abs().rem_euclid(HEIGHT);
    dx.min(WIDTH - dx).hypot(dy.min(HEIGHT - dy))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn growth_extends_the_trail_and_accelerates() {
        let mut g = Game::new(2);
        g.countdown = 0.0;
        for _ in 0..120 {
            g.tick(1.0 / 120.0, [None; 4]);
        }
        let first = g.riders[0].body.len();
        for _ in 0..1200 {
            g.tick(1.0 / 120.0, [None; 4]);
        }
        assert!(!g.finished);
        assert!(g.riders[0].body.len() > first + 30);
        assert!(g.growth >= 11);
        assert!(g.valid());
    }
    #[test]
    fn wraps_without_turning_or_drawing_a_cross_arena_trail() {
        let mut g = Game::new(2);
        g.countdown = 0.0;
        g.riders[0].pos = Vec2::new(WIDTH - 0.1, 300.0);
        g.riders[0].angle = 0.0;
        g.tick(0.05, [None; 4]);
        assert!(g.riders[0].pos.x < 4.0);
        assert_eq!(g.riders[0].angle, 0.0);
        assert!(distance(Vec2::new(359.0, 300.0), Vec2::new(1.0, 300.0)) < 3.0);
    }
    #[test]
    fn simultaneous_snake_collisions_are_a_draw() {
        let mut g = Game::new(2);
        g.countdown = 0.0;
        g.riders[0].pos = Vec2::new(180.0, 280.0);
        g.riders[1].pos = Vec2::new(181.0, 280.0);
        g.tick(0.02, [None; 4]);
        assert!(g.finished);
        assert_eq!(g.winners(), vec![0, 1]);
    }
    #[test]
    fn snake_hits_other_body_but_not_its_own() {
        let mut g = Game::new(2);
        g.countdown = 0.0;
        g.riders[1].body = vec![g.riders[0].pos];
        g.tick(0.01, [None; 4]);
        assert!(g.finished);
        assert_eq!(g.winners(), vec![1]);
    }
}
