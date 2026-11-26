// SPDX-License-Identifier: GPL-2.0-or-later
use gamekit::{board_game::Feedback, multiplayer::Vec2};
use serde::{Deserialize, Serialize};
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Circle {
    pub p: [f64; 2],
    pub r: f64,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Course {
    pub polygons: Vec<Vec<[f64; 2]>>,
    pub circles: Vec<Circle>,
    pub wind: Vec<Vec<[f64; 2]>>,
    pub rotating: Vec<Vec<[f64; 2]>>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub humans: usize,
    pub count: usize,
    pub turn: usize,
    pub scores: [u32; 4],
    pub round: usize,
    pub order: Vec<usize>,
    pub courses: Vec<Course>,
    pub ball: Vec2,
    pub velocity: Vec2,
    pub moving: bool,
    pub bounces: u32,
    pub brake: f64,
    pub sink: f64,
    pub delay: f64,
    pub shot_age: f64,
    pub elapsed: f64,
    pub finished: bool,
    pub drag: Option<Vec2>,
    pub trail: Vec<Vec2>,
    pub seed: u64,
    accumulator: f64,
}
impl Game {
    pub fn new(humans: usize, seed: u64, courses: Vec<Course>) -> Self {
        let mut g = Self {
            humans: humans.clamp(1, 4),
            count: humans.clamp(2, 4),
            turn: 0,
            scores: [0; 4],
            round: 0,
            order: (0..9).collect(),
            courses,
            ball: Vec2::new(0.0, -5.0),
            velocity: Vec2::default(),
            moving: false,
            bounces: 0,
            brake: 0.0,
            sink: 0.0,
            delay: 0.8,
            shot_age: 0.0,
            elapsed: 0.0,
            finished: false,
            drag: None,
            trail: vec![],
            seed: seed.max(1),
            accumulator: 0.0,
        };
        for i in (2..9).rev() {
            g.seed = g.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = 1 + (g.seed as usize % i);
            g.order.swap(i, j);
        }
        g
    }
    pub fn valid(&self) -> bool {
        (1..=4).contains(&self.humans)
            && self.count == self.humans.max(2)
            && self.turn < self.count
            && self.round < 7
            && self.courses.len() == 9
            && self.order.len() == 9
            && self.order.iter().all(|&i| i < 9)
            && self.ball.valid()
            && self.velocity.valid()
            && self.scores.iter().all(|&s| s <= 7)
            && [
                self.elapsed,
                self.delay,
                self.sink,
                self.brake,
                self.shot_age,
                self.accumulator,
            ]
            .iter()
            .all(|v| v.is_finite())
    }
    pub fn side(&self) -> f64 {
        if self.turn == 1 || self.turn == 2 {
            1.0
        } else {
            -1.0
        }
    }
    pub fn hole(&self) -> Vec2 {
        Vec2::new(0.0, -5.0 * self.side())
    }
    pub fn course(&self) -> &Course {
        &self.courses[self.order[self.round]]
    }
    pub fn shoot(&mut self, pull: Vec2) -> bool {
        if self.moving
            || self.finished
            || self.sink > 0.0
            || self.delay > 0.0
            || !pull.valid()
            || pull.length() < 0.15
        {
            return false;
        }
        self.velocity = pull.normalized() * (pull.length().clamp(1.0, 5.0) * 5.0);
        self.moving = true;
        self.bounces = 0;
        self.brake = 0.0;
        self.shot_age = 0.0;
        self.drag = None;
        self.trail.clear();
        true
    }
    pub fn advance(&mut self) {
        self.turn += 1;
        if self.turn == self.count {
            self.turn = 0;
            self.round += 1;
            if self.round == 7 {
                self.round = 6;
                self.finished = true;
                return;
            }
        }
        self.ball = Vec2::new(0.0, 5.0 * self.side());
        self.velocity = Vec2::default();
        self.moving = false;
        self.sink = 0.0;
        self.delay = 0.8;
        self.drag = None;
        self.trail.clear();
    }
    pub fn tick(&mut self, dt: f64) -> Vec<Feedback> {
        let mut events = vec![];
        if self.finished {
            return events;
        }
        self.elapsed += dt;
        self.delay = (self.delay - dt).max(0.0);
        if self.sink > 0.0 {
            self.sink += dt;
            self.ball = self.ball + (self.hole() - self.ball) * (dt * 14.0).min(1.0);
            if self.sink > 0.6 {
                self.advance();
                events.push(Feedback::Turn);
            }
            return events;
        }
        if self.humans == 1 && self.turn == 1 && !self.moving && self.delay == 0.0 {
            let p = self.ai();
            self.shoot(p);
            events.push(Feedback::Tap);
        }
        if !self.moving {
            return events;
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1.0 / 240.0 {
            self.accumulator -= 1.0 / 240.0;
            self.step(1.0 / 240.0, &mut events);
            if !self.moving || self.sink > 0.0 {
                break;
            }
        }
        self.trail.push(self.ball);
        if self.trail.len() > 16 {
            self.trail.remove(0);
        }
        events
    }
    fn ai(&self) -> Vec2 {
        let mut best = (f64::MAX, Vec2::new(0.0, -3.0));
        for angle in -24..=24 {
            let a = angle as f64 * 0.055;
            for power in [1.5, 2.5, 3.5, 4.5] {
                let pull = Vec2::new(a.sin(), -a.cos()) * power;
                let mut g = self.clone();
                g.humans = 2;
                g.shoot(pull);
                let mut distance = f64::MAX;
                let mut events = vec![];
                for _ in 0..900 {
                    g.step(1.0 / 120.0, &mut events);
                    distance = distance.min(g.ball.distance(g.hole()));
                    if g.sink > 0.0 {
                        distance = 0.0;
                        break;
                    }
                    if !g.moving {
                        break;
                    }
                }
                if distance < best.0 {
                    best = (distance, pull);
                }
                if distance == 0.0 {
                    return pull;
                }
            }
        }
        best.1
    }
    pub fn polygons(&self) -> Vec<Vec<Vec2>> {
        let mut result = self
            .course()
            .polygons
            .iter()
            .map(|ps| ps.iter().map(|p| Vec2::new(p[0], p[1])).collect())
            .collect::<Vec<Vec<Vec2>>>();
        let a = self.elapsed * 1.5;
        for ps in &self.course().rotating {
            result.push(
                ps.iter()
                    .map(|p| {
                        Vec2::new(
                            p[0] * a.cos() - p[1] * a.sin(),
                            p[0] * a.sin() + p[1] * a.cos(),
                        )
                    })
                    .collect(),
            );
        }
        result
    }
    fn step(&mut self, dt: f64, events: &mut Vec<Feedback>) {
        self.shot_age += dt;
        if self
            .course()
            .wind
            .iter()
            .any(|polygon| contains(polygon, self.ball))
        {
            self.velocity.x -= 43.47 * dt;
            self.velocity.y += 11.65 * dt;
        }
        let previous = self.ball;
        self.ball = self.ball + self.velocity * dt;
        let hole = self.hole();
        let motion = self.ball - previous;
        let t = ((hole - previous).dot(motion) / motion.dot(motion).max(1e-9)).clamp(0.0, 1.0);
        if (previous + motion * t).distance(hole) < 0.65 {
            self.scores[self.turn] += 1;
            self.velocity = Vec2::default();
            self.sink = 0.001;
            self.moving = false;
            events.push(Feedback::Score);
            return;
        }
        let mut hits = 0;
        for (coord, vel, limit) in [
            (&mut self.ball.x, &mut self.velocity.x, 2.2),
            (&mut self.ball.y, &mut self.velocity.y, 6.6),
        ] {
            if coord.abs() > limit {
                *coord = coord.clamp(-limit, limit);
                *vel = -*vel;
                hits += 1;
            }
        }
        let polygons = self.polygons();
        for poly in polygons {
            for i in 0..poly.len() {
                let a = poly[i];
                let b = poly[(i + 1) % poly.len()];
                let edge = b - a;
                let t = ((self.ball - a).dot(edge) / edge.dot(edge).max(1e-9)).clamp(0.0, 1.0);
                let delta = self.ball - (a + edge * t);
                let dist = delta.length();
                if dist < 0.5 && dist > 1e-8 {
                    let n = delta / dist;
                    self.ball = self.ball + n * (0.5 - dist + 0.001);
                    let vn = self.velocity.dot(n);
                    if vn < 0.0 {
                        self.velocity = self.velocity - n * (2.0 * vn);
                        hits += 1;
                    }
                }
            }
        }
        for c in self.course().circles.clone() {
            let delta = self.ball - Vec2::new(c.p[0], c.p[1]);
            let dist = delta.length();
            let r = 0.5 + c.r;
            if dist < r {
                let n = delta.normalized();
                self.ball = self.ball + n * (r - dist + 0.001);
                let vn = self.velocity.dot(n);
                if vn < 0.0 {
                    self.velocity = self.velocity - n * (2.0 * vn);
                    hits += 1;
                }
            }
        }
        if hits > 0 {
            self.bounces += 1;
            events.push(Feedback::Impact(self.velocity.length() / 25.0));
        }
        if self.bounces >= 4 {
            self.brake += dt;
            self.velocity = self.velocity * (1.0 - dt * 8.0).max(0.0);
        }
        if self.brake > 0.5 || self.shot_age > 12.0 {
            self.moving = false;
            self.advance();
            events.push(Feedback::Turn);
        }
    }
}
// Apply wind only inside the course’s marked wind zone.
fn contains(polygon: &[[f64; 2]], p: Vec2) -> bool {
    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if (a[1] > p.y) != (b[1] > p.y) && p.x < (b[0] - a[0]) * (p.y - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        Game::new(2, 8, vec![Course::default(); 9])
    }
    #[test]
    fn swept_hole_detection_prevents_fast_ball_tunnelling() {
        let mut g = game();
        g.delay = 0.0;
        g.shoot(Vec2::new(0.0, 5.0));
        for _ in 0..60 {
            g.tick(0.02);
        }
        assert_eq!(g.scores, [1, 0, 0, 0]);
        assert_eq!(g.turn, 1);
    }
    #[test]
    fn seven_rounds_give_every_player_equal_shots() {
        let mut g = game();
        for _ in 0..14 {
            g.advance();
        }
        assert!(g.finished);
        assert_eq!(g.round, 6);
        assert!(g.valid());
    }
    #[test]
    fn bouncing_miss_ends_turn() {
        let mut g = game();
        g.delay = 0.0;
        g.shoot(Vec2::new(5.0, 0.0));
        for _ in 0..500 {
            g.tick(0.02);
        }
        assert_eq!(g.turn, 1);
        assert_eq!(g.scores[0], 0);
    }
}
