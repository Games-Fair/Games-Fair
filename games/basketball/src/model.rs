// SPDX-License-Identifier: GPL-2.0-or-later
//! Perspective court and fixed-step sphere physics.
pub use gamekit::multiplayer::Vec2;
use serde::{Deserialize, Serialize};
pub const WIDTH: f64 = 360.0;
pub const HEIGHT: f64 = 780.0;
pub const RADIUS: f64 = 0.65;
const GRAVITY: f64 = 75.0;
const FLOOR: f64 = -12.25;
const HOOP_Y: f64 = -0.5;
const HOOP_Z: f64 = 23.0;
const RIM_RADIUS: f64 = 1.144;
/// Controls rim size, lateral swipe sensitivity and ball placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Easy,
    #[default]
    Medium,
    Hard,
}
impl Difficulty {
    pub fn hoop_scale(self) -> f64 {
        match self {
            Self::Easy => 1.10,
            Self::Medium => 1.0,
            Self::Hard => 0.95,
        }
    }
    fn swipe_gain(self) -> f64 {
        match self {
            Self::Easy => 0.8,
            Self::Medium => 1.0,
            Self::Hard => 1.1,
        }
    }
    fn spawn(self, uniform: f64) -> f64 {
        let (range, power) = match self {
            Self::Easy => (1.2, 1.0),
            Self::Medium => (1.5, 0.5),
            Self::Hard => (2.0, 1.0 / 3.0),
        };
        range * uniform.signum() * uniform.abs().powf(power)
    }
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct V3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl V3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    fn valid(self) -> bool {
        [self.x, self.y, self.z]
            .iter()
            .all(|v| v.is_finite() && v.abs() < 10000.0)
    }
}
pub fn project(p: V3) -> (Vec2, f64) {
    let scale = HEIGHT * 0.866025403784 / (p.z + 5.0).max(2.0);
    (
        Vec2::new(
            WIDTH * 0.5 + p.x * scale,
            HEIGHT * 0.5 - (p.y + 6.5) * scale,
        ),
        scale,
    )
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ball {
    pub p: V3,
    pub v: V3,
    pub age: f64,
    pub spin: f64,
    pub scored: bool,
    pub below: bool,
    pub fade: f64,
}
#[derive(Clone, Copy, Debug)]
pub enum Event {
    Throw,
    Bounce(f64),
    Rim(f64),
    Score,
    Turn,
    End,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub difficulty: Difficulty,
    pub humans: usize,
    pub count: usize,
    pub turn: usize,
    pub scores: [u32; 4],
    pub round_wins: [u32; 4],
    pub recorded: bool,
    pub remaining: f64,
    pub running: bool,
    pub finished: bool,
    pub elapsed: f64,
    pub countdown: f64,
    pub balls: Vec<Ball>,
    pub ready_x: f64,
    pub ready_age: f64,
    pub net_age: f64,
    pub rim_age: f64,
    pub rotation: f64,
    pub transition: f64,
    pub end_wait: f64,
    pub shot_index: u32,
    accumulator: f64,
    #[serde(skip)]
    pub events: Vec<Event>,
}
impl Game {
    pub fn new(humans: usize) -> Self {
        let humans = humans.clamp(1, 4);
        Self {
            difficulty: Difficulty::default(),
            humans,
            count: humans,
            turn: 0,
            scores: [0; 4],
            round_wins: [0; 4],
            recorded: false,
            remaining: if humans == 1 { 45.0 } else { 30.0 },
            running: false,
            finished: false,
            elapsed: 0.0,
            countdown: 0.0,
            balls: vec![],
            ready_x: Difficulty::default().spawn(-0.7),
            ready_age: 0.0,
            net_age: 10.0,
            rim_age: 10.0,
            rotation: 0.0,
            transition: 0.0,
            end_wait: 0.0,
            shot_index: 0,
            accumulator: 0.0,
            events: vec![],
        }
    }
    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.difficulty = difficulty;
        self.ready_x = self.spawn_x();
    }
    fn spawn_x(&self) -> f64 {
        // Keep the same samples for every player; difficulty changes their spread.
        let uniform = if self.shot_index == 0 {
            -0.7
        } else {
            (self.shot_index as f64 * 0.61803398875).fract() * 2.0 - 1.0
        };
        self.difficulty.spawn(uniform)
    }
    pub fn valid(&self) -> bool {
        (1..=4).contains(&self.humans)
            && self.count == self.humans
            && self.turn < self.count
            && self.balls.len() <= 32
            && self.scores.iter().all(|&v| v < 10000)
            && [
                self.remaining,
                self.elapsed,
                self.ready_x,
                self.ready_age,
                self.net_age,
                self.rim_age,
                self.rotation,
                self.transition,
                self.end_wait,
                self.accumulator,
            ]
            .iter()
            .all(|v| v.is_finite() && v.abs() < 1e6)
            && self.balls.iter().all(|b| {
                b.p.valid() && b.v.valid() && [b.age, b.spin, b.fade].iter().all(|v| v.is_finite())
            })
    }
    pub fn ready_position(&self) -> V3 {
        V3::new(self.ready_x, -11.6, 7.5)
    }
    pub fn can_shoot(&self) -> bool {
        !self.finished
            && self.remaining > 0.0
            && self.transition <= 0.0
            && self.ready_age >= 0.16
            && self.balls.len() < 32
    }
    pub fn rotated(&self) -> bool {
        self.turn == 1 || self.turn == 2
    }
    pub fn to_court(&self, p: Vec2) -> Vec2 {
        if self.rotated() {
            Vec2::new(WIDTH - p.x, HEIGHT - p.y)
        } else {
            p
        }
    }
    pub fn shoot(&mut self, start: Vec2, end: Vec2, duration: f64) -> bool {
        let dy = start.y - end.y;
        if !self.can_shoot()
            || !start.valid()
            || !end.valid()
            || !duration.is_finite()
            || dy < 18.0
            || duration <= 0.0
        {
            return false;
        }
        let original = self.ready_position();
        let (screen, scale) = project(original);
        // Dragging keeps the ball in the camera-facing touch plane. Release it from that
        // position, so the ball never snaps back to its spawn point when the finger lifts.
        let p = V3::new(
            (end.x - WIDTH * 0.5) / scale,
            (HEIGHT * 0.5 - end.y) / scale - 6.5,
            original.z,
        );
        let (rim, _) = project(V3::new(0.0, HOOP_Y, HOOP_Z));
        let slope = (end.x - start.x) / dy * self.difficulty.swipe_gain();
        let ideal = (rim.x - screen.x) / (screen.y - rim.y);
        // Shots use a fixed apex over the basket, with modest horizontal attraction.
        let error = (slope - ideal) * 15.5;
        let target_x = if (slope.atan() - ideal.atan()).abs() < 10.0_f64.to_radians() {
            error * 0.12
        } else {
            error
        };
        let speed = (dy / duration).clamp(180.0, 2600.0);
        let strength = (speed / 700.0).clamp(0.60, 1.0);
        let apex = (HOOP_Y + 4.5).max(p.y + 0.5);
        let vy = (2.0 * GRAVITY * (apex - p.y)).sqrt();
        let flight = vy / GRAVITY + (2.0 * (apex - HOOP_Y) / GRAVITY).sqrt();
        self.balls.push(Ball {
            p,
            v: V3::new(
                (target_x - p.x) / flight,
                vy,
                (HOOP_Z - p.z) / flight * strength,
            ),
            age: 0.0,
            spin: 0.0,
            scored: false,
            below: false,
            fade: 0.0,
        });
        self.running = true;
        self.shot_index += 1;
        self.ready_age = 0.0;
        // Both players receive the same deterministic sequence of spawn locations.
        self.ready_x = self.spawn_x();
        self.events.push(Event::Throw);
        true
    }
    pub fn needs_clock(&self) -> bool {
        !self.finished
    }
    pub fn tick(&mut self, dt: f64) {
        if self.finished || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        // Match time uses wall time even when rendering drops frames.
        if self.running {
            self.remaining = (self.remaining - dt).max(0.0);
        }
        self.accumulator += dt.min(0.25);
        while self.accumulator >= 1.0 / 240.0 && !self.finished {
            self.accumulator -= 1.0 / 240.0;
            self.step(1.0 / 240.0);
        }
    }
    fn step(&mut self, dt: f64) {
        self.elapsed += dt;
        self.net_age += dt;
        self.rim_age += dt;
        self.ready_age += dt;
        if self.transition > 0.0 {
            self.transition = (self.transition - dt).max(0.0);
            let target = if self.rotated() {
                std::f64::consts::PI
            } else {
                0.0
            };
            self.rotation += (target - self.rotation) * (dt * 10.0).min(1.0);
            if self.transition == 0.0 {
                self.rotation = target;
            }
            return;
        }
        let rim_radius = RIM_RADIUS * self.difficulty.hoop_scale();
        for ball in &mut self.balls {
            let old = ball.p;
            ball.age += dt;
            ball.v.y -= GRAVITY * dt;
            ball.p.x += ball.v.x * dt;
            ball.p.y += ball.v.y * dt;
            ball.p.z += ball.v.z * dt;
            ball.spin += dt * (ball.v.z.hypot(ball.v.y)) * 0.22;
            // Backboard is a solid rectangle behind the rim; rebounds can still score.
            if ball.p.x.abs() < 3.0 + RADIUS
                && (-1.5 - RADIUS..=2.7 + RADIUS).contains(&ball.p.y)
                && old.z < 24.26 - RADIUS
                && ball.p.z >= 24.26 - RADIUS
            {
                ball.p.z = 24.26 - RADIUS;
                ball.v.z = -ball.v.z.abs() * 0.68;
                self.events.push(Event::Bounce(ball.v.z.abs()));
            }
            // Continuous torus contact against the nearest point on its centerline.
            let dx = ball.p.x;
            let dz = ball.p.z - HOOP_Z;
            let radial = dx.hypot(dz);
            if radial > 0.0001 {
                let cx = dx / radial * rim_radius;
                let cz = HOOP_Z + dz / radial * rim_radius;
                let nx = ball.p.x - cx;
                let ny = ball.p.y - HOOP_Y;
                let nz = ball.p.z - cz;
                let dist = (nx * nx + ny * ny + nz * nz).sqrt();
                if dist < RADIUS + 0.075 && dist > 0.0001 {
                    let nx = nx / dist;
                    let ny = ny / dist;
                    let nz = nz / dist;
                    let dot = ball.v.x * nx + ball.v.y * ny + ball.v.z * nz;
                    let push = RADIUS + 0.075 - dist;
                    ball.p.x += nx * push;
                    ball.p.y += ny * push;
                    ball.p.z += nz * push;
                    if dot < 0.0 {
                        ball.v.x -= 1.68 * dot * nx;
                        ball.v.y -= 1.68 * dot * ny;
                        ball.v.z -= 1.68 * dot * nz;
                        if dot < -1.0 {
                            self.events.push(Event::Rim(-dot));
                            self.rim_age = 0.0;
                        }
                    }
                }
            }
            if !ball.scored && !ball.below && old.y > HOOP_Y && ball.p.y <= HOOP_Y && ball.v.y < 0.0
            {
                let t = (old.y - HOOP_Y) / (old.y - ball.p.y);
                let x = old.x + (ball.p.x - old.x) * t;
                let z = old.z + (ball.p.z - old.z) * t;
                if x.hypot(z - HOOP_Z) < rim_radius - RADIUS * 0.5 {
                    ball.scored = true;
                    self.scores[self.turn] += 1;
                    self.net_age = 0.0;
                    self.events.push(Event::Score);
                    ball.v.x *= 0.3;
                    ball.v.z *= 0.3;
                    ball.v.y *= 0.55;
                }
                ball.below = true;
            }
            if ball.p.y < FLOOR + RADIUS {
                ball.p.y = FLOOR + RADIUS;
                if ball.v.y < -1.5 {
                    self.events.push(Event::Bounce(-ball.v.y));
                }
                ball.v.y = ball.v.y.abs() * 0.61;
                ball.v.x *= 0.86;
                ball.v.z *= 0.86;
            }
            if ball.p.x.abs() > 5.7 - RADIUS {
                ball.p.x = ball.p.x.signum() * (5.7 - RADIUS);
                ball.v.x = -ball.v.x * 0.7;
            }
            if ball.age > 2.8 {
                ball.fade += dt / 0.45;
            }
        }
        // Equal-mass ball contacts, including simultaneous shots and loose balls under the net.
        for i in 0..self.balls.len() {
            let (left, right) = self.balls.split_at_mut(i + 1);
            let a = &mut left[i];
            for b in right {
                let dx = b.p.x - a.p.x;
                let dy = b.p.y - a.p.y;
                let dz = b.p.z - a.p.z;
                let len = (dx * dx + dy * dy + dz * dz).sqrt();
                if len > 0.001 && len < 2.0 * RADIUS {
                    let n = V3::new(dx / len, dy / len, dz / len);
                    let push = (2.0 * RADIUS - len) * 0.5;
                    a.p.x -= n.x * push;
                    a.p.y -= n.y * push;
                    a.p.z -= n.z * push;
                    b.p.x += n.x * push;
                    b.p.y += n.y * push;
                    b.p.z += n.z * push;
                    let dot = (b.v.x - a.v.x) * n.x + (b.v.y - a.v.y) * n.y + (b.v.z - a.v.z) * n.z;
                    if dot < 0.0 {
                        let j = -dot * 0.82;
                        a.v.x -= j * n.x;
                        a.v.y -= j * n.y;
                        a.v.z -= j * n.z;
                        b.v.x += j * n.x;
                        b.v.y += j * n.y;
                        b.v.z += j * n.z;
                    }
                }
            }
        }
        self.balls
            .retain(|b| b.fade < 1.0 && b.p.z > -2.5 && b.p.z < 60.0);
        if self.running && self.remaining <= 0.0 {
            self.end_wait += dt;
            if self.end_wait >= 2.0 {
                self.balls.clear();
                self.running = false;
                self.end_wait = 0.0;
                if self.turn + 1 == self.count {
                    self.finished = true;
                    self.events.push(Event::End);
                    self.record_result();
                } else {
                    self.turn += 1;
                    self.remaining = 30.0;
                    self.transition = 0.8;
                    self.ready_age = 0.0;
                    self.shot_index = 0;
                    self.ready_x = self.spawn_x();
                    self.events.push(Event::Turn);
                }
            }
        }
    }
    pub fn winners(&self) -> Vec<usize> {
        let best = *self.scores[..self.count].iter().max().unwrap_or(&0);
        (0..self.count)
            .filter(|&i| self.scores[i] == best)
            .collect()
    }
    pub fn record_result(&mut self) {
        if self.finished && !self.recorded {
            let w = self.winners();
            if w.len() == 1 {
                self.round_wins[w[0]] = self.round_wins[w[0]].saturating_add(1);
            }
            self.recorded = true;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn difficulty_changes_spawn_distribution_sensitivity_and_rim_geometry() {
        assert!(Difficulty::Easy.spawn(0.25) < Difficulty::Medium.spawn(0.25));
        assert!(Difficulty::Medium.spawn(0.25) < Difficulty::Hard.spawn(0.25));
        for level in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
            let mut g = Game::new(2);
            g.set_difficulty(level);
            g.ready_age = 1.0;
            let (start, _) = project(g.ready_position());
            assert!(g.shoot(start, Vec2::new(180.0, 245.0), 0.38));
            for _ in 0..240 {
                g.tick(1.0 / 120.0);
            }
            assert_eq!(g.scores[0], 1, "Aimed shot must work at {level:?}");
            let restored: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
            assert_eq!(restored.difficulty, level);
            assert!(restored.valid());
        }
        // Same downward crossing clears the wider easy rim, but hits the hard rim.
        let rim_contact = |difficulty| {
            let mut g = Game::new(1);
            g.set_difficulty(difficulty);
            g.balls.push(Ball {
                p: V3::new(0.52, 0.0, 23.0),
                v: V3::new(0.0, -3.0, 0.0),
                age: 0.0,
                spin: 0.0,
                scored: false,
                below: false,
                fade: 0.0,
            });
            for _ in 0..60 {
                g.tick(0.01);
            }
            g.events.iter().any(|e| matches!(e, Event::Rim(_)))
        };
        assert!(!rim_contact(Difficulty::Easy));
        assert!(rim_contact(Difficulty::Hard));
    }
    #[test]
    fn an_aimed_swipe_scores_with_the_real_collision_geometry() {
        for duration in [0.25, 0.38, 0.45] {
            let mut g = Game::new(2);
            g.ready_age = 1.0;
            let (start, _) = project(g.ready_position());
            assert!(g.shoot(start, Vec2::new(180.0, 245.0), duration));
            for _ in 0..240 {
                g.tick(1.0 / 120.0);
            }
            assert_eq!(g.scores[0], 1, "duration {duration}: {:?}", g.balls);
        }
    }
    #[test]
    fn clock_starts_on_shot_and_flips_after_turn() {
        let mut g = Game::new(2);
        for _ in 0..60 {
            g.tick(0.1);
        }
        assert_eq!(g.remaining, 30.0);
        let (s, _) = project(g.ready_position());
        assert!(g.shoot(s, Vec2::new(180.0, 240.0), 0.38));
        for _ in 0..330 {
            g.tick(0.1);
        }
        assert_eq!(g.turn, 1);
        assert!(!g.running);
        assert_eq!(g.remaining, 30.0);
        assert!(
            g.to_court(Vec2::new(20.0, 30.0))
                .distance(Vec2::new(340.0, 750.0))
                < 0.001
        );
    }
    #[test]
    fn downward_crossing_scores_once_but_upward_does_not() {
        let mut g = Game::new(1);
        g.balls.push(Ball {
            p: V3::new(0.0, 0.0, 23.0),
            v: V3::new(0.0, -3.0, 0.0),
            age: 0.0,
            spin: 0.0,
            scored: false,
            below: false,
            fade: 0.0,
        });
        for _ in 0..30 {
            g.tick(0.02);
        }
        assert_eq!(g.scores[0], 1);
        g.balls[0].p.y = -1.0;
        g.balls[0].v.y = 20.0;
        for _ in 0..10 {
            g.tick(0.02);
        }
        assert_eq!(g.scores[0], 1);
    }
    #[test]
    fn solo_is_45_seconds_and_simulation_is_frame_independent() {
        let mut a = Game::new(1);
        assert_eq!(a.remaining, 45.0);
        a.ready_age = 1.0;
        let (p, _) = project(a.ready_position());
        a.shoot(p, Vec2::new(180.0, 245.0), 0.38);
        let mut b = a.clone();
        for _ in 0..120 {
            a.tick(1.0 / 120.0);
        }
        for _ in 0..30 {
            b.tick(1.0 / 30.0);
        }
        assert!((a.balls[0].p.y - b.balls[0].p.y).abs() < 0.001);
        assert!(a.valid());
    }
}
