// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use gamekit::{board_game::Feedback, multiplayer::Vec2};
use serde::{Deserialize, Serialize};
const BALL: usize = 10;
pub fn radius(index: usize) -> f64 {
    if index == BALL { 0.79 } else { 1.012 }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Body {
    pub p: Vec2,
    pub v: Vec2,
    pub mass: f64,
    pub spin: f64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub humans: usize,
    pub count: usize,
    pub turn: usize,
    pub scores: [u32; 4],
    pub bodies: Vec<Body>,
    pub moving: bool,
    pub shot_time: f64,
    pub settle: f64,
    pub goal_age: f64,
    pub goal_side: usize,
    pub last_scorer: usize,
    pub finished: bool,
    pub elapsed: f64,
    pub selected: Option<usize>,
    pub drag: Option<Vec2>,
    pub delay: f64,
    pub seed: u64,
    pub formation: usize,
    pub formations: Vec<[[f64; 2]; 5]>,
    pub reset_from: Vec<Vec2>,
    pub reset_progress: f64,
    pub trail: Vec<(Vec2, f64)>,
    accumulator: f64,
}
impl Game {
    pub fn new(humans: usize, seed: u64) -> Self {
        let mut g = Self {
            humans: humans.clamp(1, 4),
            count: humans.clamp(2, 4),
            turn: 0,
            scores: [0; 4],
            bodies: vec![],
            moving: false,
            shot_time: 0.0,
            settle: 0.0,
            goal_age: 0.0,
            goal_side: 0,
            last_scorer: 0,
            finished: false,
            elapsed: 0.0,
            selected: None,
            drag: None,
            delay: 0.7,
            seed,
            formation: 0,
            formations: vec![],
            reset_from: vec![],
            reset_progress: 1.0,
            trail: vec![],
            accumulator: 0.0,
        };
        g.reset();
        g
    }
    pub fn side(p: usize) -> usize {
        usize::from(p == 1 || p == 2)
    }
    pub fn reset(&mut self) {
        self.reset_from = self.bodies.iter().map(|b| b.p).collect();
        self.reset_progress = if self.reset_from.len() == 11 {
            0.0
        } else {
            1.0
        };
        self.trail.clear();
        self.bodies.clear();
        for side in 0..2 {
            let positions = self.formations.get(self.formation).copied().unwrap_or([
                [-4., -11.],
                [4., -11.],
                [-2.4, -6.],
                [2.4, -6.],
                [0., -14.],
            ]);
            for [x, y] in positions {
                self.bodies.push(Body {
                    p: Vec2::new(x, if side == 0 { y } else { -y }),
                    v: Vec2::new(0.0, 0.0),
                    mass: 3.0,
                    spin: 0.0,
                });
            }
        }
        self.bodies.push(Body {
            p: Vec2::new(0.0, 0.0),
            v: Vec2::new(0.0, 0.0),
            mass: 1.0,
            spin: 0.0,
        });
        self.moving = false;
        self.selected = None;
        self.drag = None;
    }
    pub fn valid(&self) -> bool {
        (1..=4).contains(&self.humans)
            && self.count == self.humans.max(2)
            && self.turn < self.count
            && self.bodies.len() == 11
            && self
                .bodies
                .iter()
                .all(|b| b.p.valid() && b.v.valid() && b.mass > 0.0 && b.spin.is_finite())
            && [
                self.elapsed,
                self.delay,
                self.goal_age,
                self.shot_time,
                self.settle,
                self.accumulator,
            ]
            .iter()
            .all(|v| v.is_finite())
            && self.selected.is_none_or(|i| i < 10)
    }
    pub fn shoot(&mut self, i: usize, pull: Vec2) -> bool {
        if self.moving
            || self.goal_age > 0.0
            || self.finished
            || self.delay > 0.0
            || i >= 10
            || i / 5 != Self::side(self.turn)
            || !pull.valid()
        {
            return false;
        }
        let length = pull.length();
        if length <= 1.0 {
            return false;
        }
        let force = (length.min(4.0) - 1.0) / 3.0 * 100.0;
        self.bodies[i].v = pull * (force / (length * 3.0));
        self.moving = true;
        self.shot_time = 0.0;
        self.settle = 0.0;
        self.selected = None;
        self.drag = None;
        true
    }
    pub fn flick_feedback(&self, disc: usize) -> Feedback {
        Feedback::Flick((self.bodies[disc].v.length() / (100.0 / 3.0)).clamp(0., 1.))
    }
    pub fn tick(&mut self, dt: f64) -> Vec<Feedback> {
        let mut events = vec![];
        if self.finished {
            return events;
        }
        self.elapsed += dt;
        self.reset_progress = (self.reset_progress + dt / 0.5).min(1.0);
        for (_, age) in &mut self.trail {
            *age += dt;
        }
        self.trail.retain(|(_, age)| *age < 0.22);
        if self.moving {
            self.trail.push((self.bodies[10].p, 0.0));
        }
        self.delay = (self.delay - dt).max(0.0);
        if self.goal_age > 0.0 {
            self.goal_age = (self.goal_age - dt).max(0.0);
            if self.goal_age == 0.0 {
                self.finished = self.scores.iter().any(|&s| s >= 3);
                if !self.finished {
                    self.reset();
                    self.delay = 0.7;
                    events.push(Feedback::Turn);
                }
            }
            return events;
        }
        if self.humans == 1 && self.turn == 1 && !self.moving && self.delay == 0.0 {
            let (i, pull) = self.aim_ai();
            if self.shoot(i, pull) {
                events.push(self.flick_feedback(i));
            }
        }
        if !self.moving {
            return events;
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1.0 / 240.0 {
            self.accumulator -= 1.0 / 240.0;
            self.step(1.0 / 240.0, &mut events);
            if !self.moving {
                break;
            }
        }
        events
    }
    fn aim_ai(&self) -> (usize, Vec2) {
        let ball = self.bodies[10].p;
        let goal = Vec2::new(0.0, -17.0);
        let to_goal = (goal - ball).normalized();
        let target = ball - to_goal * (radius(0) + radius(BALL));
        let i = (5..10)
            .min_by(|&a, &b| {
                self.bodies[a]
                    .p
                    .distance(target)
                    .total_cmp(&self.bodies[b].p.distance(target))
            })
            .expect("validated game invariant");
        let delta = target - self.bodies[i].p;
        let dist = delta.length();
        (i, delta.normalized() * (dist * 0.23 + 1.4).clamp(1.2, 4.0))
    }
    fn step(&mut self, dt: f64, events: &mut Vec<Feedback>) {
        self.shot_time += dt;
        for (i, b) in self.bodies.iter_mut().enumerate() {
            let r = radius(i);
            b.p = b.p + b.v * dt;
            b.spin += b.v.length() * dt / r;
            b.v = b.v * (-dt * if i == 10 { 0.7 } else { 1.0 }).exp();
            if b.v.length() < 2.0 {
                b.v = b.v * 0.95_f64.powf(dt * 50.0);
            }
            if b.v.length() < 0.08 {
                b.v = Vec2::new(0.0, 0.0);
            }
            let limit = 9.22 - r;
            if b.p.x.abs() > limit {
                b.p.x = b.p.x.clamp(-limit, limit);
                if b.v.x * b.p.x.signum() > 0.0 {
                    if b.v.x.abs() > 0.8 {
                        events.push(Feedback::Wall(b.v.x.abs() / 30.0));
                    }
                    b.v.x *= -0.8;
                }
            }
            let mouth = b.p.x.abs() < 3.23 - r;
            let y_limit = if mouth { 18.48 - r } else { 15.84 - r };
            if b.p.y.abs() > y_limit {
                b.p.y = b.p.y.clamp(-y_limit, y_limit);
                if b.v.y * b.p.y.signum() > 0.0 {
                    if b.v.y.abs() > 0.8 {
                        events.push(Feedback::Wall(b.v.y.abs() / 30.0));
                    }
                    b.v.y *= -0.8;
                }
            }
            if b.p.y.abs() > 15.84 && b.p.x.abs() > 3.23 - r {
                b.p.x = b.p.x.signum() * (3.23 - r);
                b.v.x = -b.v.x * 0.7;
            }
        }
        for _ in 0..3 {
            for i in 0..11 {
                for j in i + 1..11 {
                    let (a, b) = self.bodies.split_at_mut(j);
                    let a = &mut a[i];
                    let b = &mut b[0];
                    let delta = b.p - a.p;
                    let dist = delta.length();
                    let radius = radius(i) + radius(j);
                    if dist >= radius {
                        continue;
                    }
                    let n = if dist > 1e-7 {
                        delta / dist
                    } else {
                        Vec2::new(1.0, 0.0)
                    };
                    let inv = 1.0 / a.mass + 1.0 / b.mass;
                    let overlap = (radius - dist + 0.0001) / inv;
                    a.p = a.p - n * (overlap / a.mass);
                    b.p = b.p + n * (overlap / b.mass);
                    let v = (b.v - a.v).dot(n);
                    if v < 0.0 {
                        let impulse = -v * 1.82 / inv;
                        a.v = a.v - n * (impulse / a.mass);
                        b.v = b.v + n * (impulse / b.mass);
                        if v.abs() > 0.8 {
                            events.push(if j == 10 {
                                Feedback::Impact(v.abs() / 35.0)
                            } else {
                                Feedback::Disc(v.abs() / 35.0)
                            });
                        }
                    }
                }
            }
        }
        let ball = self.bodies[10].p;
        if ball.y.abs() > 17.1 && ball.x.abs() < 3.2 {
            self.goal_side = usize::from(ball.y < 0.0);
            let scorer = if Self::side(self.turn) == self.goal_side {
                self.turn
            } else {
                (0..self.count)
                    .find(|&p| Self::side(p) == self.goal_side)
                    .expect("validated game invariant")
            };
            self.last_scorer = scorer;
            self.scores[scorer] += 1;
            self.goal_age = 2.0;
            self.moving = false;
            events.push(if scorer == self.turn {
                Feedback::Score
            } else {
                Feedback::OwnGoal
            });
            self.turn = (self.turn + 1) % self.count;
            return;
        }
        if self.bodies.iter().all(|b| b.v.length() < 0.15) {
            self.settle += dt;
        } else {
            self.settle = 0.0;
        }
        if self.settle > 0.2 || self.shot_time > 12.0 {
            for b in &mut self.bodies {
                b.v = Vec2::new(0.0, 0.0);
                if b.p.y.abs() > 15.0 {
                    b.p.y = b.p.y.signum() * 14.65;
                }
            }
            self.moving = false;
            self.turn = (self.turn + 1) % self.count;
            self.delay = 0.6;
            events.push(Feedback::Turn);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disallows_opponent_and_zero_shots() {
        let mut g = Game::new(2, 1);
        g.delay = 0.0;
        assert!(!g.shoot(5, Vec2::new(0.0, 4.0)));
        assert!(!g.shoot(0, Vec2::new(0.0, 0.0)));
        assert!(g.shoot(0, Vec2::new(0.0, 4.0)));
        assert!(!g.shoot(1, Vec2::new(0.0, 4.0)));
    }
    #[test]
    fn flick_strength_and_contact_match_the_visible_ball() {
        let mut g = Game::new(2, 1);
        g.delay = 0.;
        g.bodies[0].p = Vec2::new(0., -(radius(0) + radius(BALL) + 0.04));
        g.shoot(0, Vec2::new(0., 4.));
        assert!(matches!(g.flick_feedback(0), Feedback::Flick(v) if v > 0.99));
        assert!(
            g.tick(1. / 120.)
                .iter()
                .any(|e| matches!(e, Feedback::Impact(v) if *v > 0.8))
        );
        assert!(g.bodies[BALL].v.y > 0.);
    }

    #[test]
    fn goal_counts_once_and_resets() {
        let mut g = Game::new(2, 1);
        g.bodies[10].p = Vec2::new(0.0, 17.2);
        g.moving = true;
        g.tick(0.02);
        assert_eq!(g.scores[0], 1);
        for _ in 0..150 {
            g.tick(0.02);
        }
        assert_eq!(g.scores[0], 1);
        assert_eq!(g.bodies[10].p.y, 0.0);
    }
    #[test]
    fn own_goal_awards_the_opposing_side_and_emits_its_cue() {
        let mut g = Game::new(2, 1);
        g.bodies[10].p = Vec2::new(0.0, -17.2);
        g.moving = true;
        assert!(g.tick(0.02).iter().any(|e| matches!(e, Feedback::OwnGoal)));
        assert_eq!(g.scores, [0, 1, 0, 0]);
    }
    #[test]
    fn turn_waits_until_physics_settles() {
        let mut g = Game::new(2, 1);
        g.delay = 0.0;
        g.shoot(0, Vec2::new(1.0, 3.0));
        g.tick(0.1);
        assert_eq!(g.turn, 0);
        for _ in 0..700 {
            g.tick(0.02);
        }
        assert!(!g.moving);
        assert_eq!(g.turn, 1);
        assert!(g.valid());
    }
}
