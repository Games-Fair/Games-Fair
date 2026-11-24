// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use gamekit::{board_game::Feedback, multiplayer::Vec2};
use serde::{Deserialize, Serialize};
pub const WIDTH: f64 = 731.0;
pub const FLOOR: f64 = 903.0;
pub const SIZES: [f64; 11] = [
    52.0, 67.0, 98.0, 116.0, 139.0, 177.0, 210.0, 255.0, 290.0, 356.0, 424.0,
];
#[derive(Clone, Serialize, Deserialize)]
pub struct Fruit {
    pub kind: usize,
    pub p: Vec2,
    pub v: Vec2,
    pub angle: f64,
    pub spin: f64,
    pub age: f64,
    pub overflow: f64,
    pub merge_age: f64,
}
impl Fruit {
    pub fn radius(&self) -> f64 {
        SIZES[self.kind] * 0.5
    }
    pub fn mass(&self) -> f64 {
        (self.radius() / 26.0).powf(1.3)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub fruits: Vec<Fruit>,
    pub score: u32,
    pub next: [usize; 4],
    pub seed: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Burst {
    pub p: Vec2,
    pub kind: usize,
    pub age: f64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub shapes: Vec<Vec<Vec2>>,
    pub fruits: Vec<Fruit>,
    pub next: [usize; 4],
    pub score: u32,
    pub best: u32,
    pub difficulty: usize,
    pub aim: f64,
    pub delay: f64,
    pub elapsed: f64,
    pub finished: bool,
    pub held: bool,
    pub seed: u64,
    pub history: Vec<Snapshot>,
    pub swept: bool,
    pub bursts: Vec<Burst>,
    accumulator: f64,
}
impl Game {
    pub fn new(seed: u64) -> Self {
        let mut g = Self {
            shapes: vec![],
            fruits: vec![],
            next: [0; 4],
            score: 0,
            best: 0,
            difficulty: 1,
            aim: WIDTH * 0.5,
            delay: 0.4,
            elapsed: 0.0,
            finished: false,
            held: false,
            seed: seed.max(1),
            history: vec![],
            swept: false,
            bursts: vec![],
            accumulator: 0.0,
        };
        for i in 0..4 {
            g.next[i] = g.random(5);
        }
        g
    }
    fn random(&mut self, n: usize) -> usize {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed as usize % n
    }
    pub fn valid(&self) -> bool {
        self.difficulty < 3
            && self.shapes.len() <= 11
            && self
                .shapes
                .iter()
                .all(|s| s.len() <= 256 && s.iter().all(|p| p.valid()))
            && self
                .history
                .iter()
                .all(|h| h.fruits.len() <= 160 && h.next.iter().all(|&k| k < 5))
            && self.fruits.len() <= 160
            && self.history.len() <= 32
            && self.next.iter().all(|&k| k < 5)
            && self
                .fruits
                .iter()
                .chain(self.history.iter().flat_map(|h| &h.fruits))
                .all(|f| {
                    f.kind < 11
                        && f.p.valid()
                        && f.v.valid()
                        && [f.angle, f.spin, f.age, f.overflow, f.merge_age]
                            .iter()
                            .all(|v| v.is_finite())
                })
            && [self.aim, self.delay, self.elapsed, self.accumulator]
                .iter()
                .all(|v| v.is_finite())
    }
    pub fn drop(&mut self) -> bool {
        if self.delay > 0.0 || self.finished || self.fruits.len() >= 160 {
            return false;
        }
        self.history.push(Snapshot {
            fruits: self.fruits.clone(),
            score: self.score,
            next: self.next,
            seed: self.seed,
        });
        if self.history.len() > 32 {
            self.history.remove(0);
        }
        let k = self.next[0];
        let r = SIZES[k] * 0.5;
        self.fruits.push(Fruit {
            kind: k,
            p: Vec2::new(self.aim.clamp(r, WIDTH - r), -46.0),
            v: Vec2::new(0.0, 274.0),
            angle: 0.0,
            spin: 0.0,
            age: 0.0,
            overflow: 0.0,
            merge_age: 1.0,
        });
        self.next[0] = self.next[1];
        self.next[1] = self.next[2];
        self.next[2] = self.next[3];
        self.next[3] = self.random(5);
        self.delay = 0.4;
        self.aim = WIDTH * 0.5;
        self.held = false;
        true
    }
    pub fn undo(&mut self) -> bool {
        if let Some(s) = self.history.pop() {
            self.fruits = s.fruits;
            self.score = s.score;
            self.next = s.next;
            self.seed = s.seed;
            self.finished = false;
            self.delay = 0.15;
            self.bursts.clear();
            self.held = false;
            return true;
        }
        false
    }
    pub fn sweep(&mut self) -> bool {
        if self.swept {
            return false;
        }
        self.swept = true;
        self.history.clear();
        for f in &self.fruits {
            if f.kind < 3 {
                self.bursts.push(Burst {
                    p: f.p,
                    kind: f.kind,
                    age: 0.0,
                });
            }
        }
        self.fruits.retain(|f| f.kind >= 3);
        for f in &mut self.fruits {
            f.overflow = 0.0;
        }
        self.finished = false;
        true
    }
    pub fn tick(&mut self, dt: f64) -> Vec<Feedback> {
        let mut events = vec![];
        self.elapsed += dt;
        self.delay = (self.delay - dt).max(0.0);
        for b in &mut self.bursts {
            b.age += dt;
        }
        self.bursts.retain(|b| b.age < 0.5);
        if self.finished {
            return events;
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1.0 / 120.0 {
            self.accumulator -= 1.0 / 120.0;
            self.step(1.0 / 120.0, &mut events);
            if self.finished {
                break;
            }
        }
        events
    }
    fn step(&mut self, dt: f64, events: &mut Vec<Feedback>) {
        for f in &mut self.fruits {
            f.age += dt;
            f.merge_age += dt;
            f.v.y += 896.0 * dt;
            f.v = f.v * (-0.1 * dt).exp();
            f.p = f.p + f.v * dt;
            f.angle += f.spin * dt;
            f.spin *= (-0.6 * dt).exp();
            let points = crate::collision::outline(f, &self.shapes);
            let impact = crate::collision::walls(f, &points, WIDTH, FLOOR);
            if impact > 220.0 {
                events.push(Feedback::Wall(impact / 900.0));
            }
        }
        let mut pairs = vec![];
        for _ in 0..6 {
            for i in 0..self.fruits.len() {
                for j in i + 1..self.fruits.len() {
                    let (a, b) = self.fruits.split_at_mut(j);
                    let a = &mut a[i];
                    let b = &mut b[0];
                    if a.p.distance(b.p) > (a.radius() + b.radius()) * 1.42 {
                        continue;
                    }
                    let ap = crate::collision::outline(a, &self.shapes);
                    let bp = crate::collision::outline(b, &self.shapes);
                    let Some((n, depth, point)) = crate::collision::contact(a, b, &ap, &bp) else {
                        continue;
                    };
                    if a.kind == b.kind && a.merge_age >= 0.11 && b.merge_age >= 0.11 {
                        if !pairs
                            .iter()
                            .any(|&(x, y)| x == i || y == i || x == j || y == j)
                        {
                            pairs.push((i, j));
                        }
                        continue;
                    }
                    let speed = crate::collision::resolve(a, b, n, depth, point);
                    if speed > 360.0 && events.len() < 10 {
                        events.push(Feedback::Wall(speed / 1200.0));
                    }
                }
            }
        }
        if !pairs.is_empty() {
            let mut remove = vec![];
            let mut born = vec![];
            for (i, j) in pairs {
                let a = &self.fruits[i];
                let b = &self.fruits[j];
                let kind = a.kind + 1;
                let moving = if a.v.length() > b.v.length() { a } else { b };
                let stationary = if a.v.length() > b.v.length() { b } else { a };
                let p = Vec2::new(
                    moving.p.x * 0.1 + stationary.p.x * 0.9,
                    moving.p.y * 0.4 + stationary.p.y * 0.6,
                );
                self.score += (kind * (kind + 1) / 2) as u32;
                self.best = self.best.max(self.score);
                self.bursts.push(Burst {
                    p,
                    kind: kind.min(10),
                    age: 0.0,
                });
                if kind < 11 {
                    born.push(Fruit {
                        kind,
                        p,
                        v: Vec2::new(0.0, 91.0),
                        angle: (a.angle + b.angle) * 0.5,
                        spin: (a.spin + b.spin) * 0.25,
                        age: 0.0,
                        overflow: 0.0,
                        merge_age: 0.0,
                    });
                }
                remove.extend([i, j]);
            }
            remove.sort_unstable();
            for i in remove.into_iter().rev() {
                self.fruits.remove(i);
            }
            self.fruits.extend(born);
            events.push(Feedback::Score);
        }
        for f in &mut self.fruits {
            if f.age > 1.0 && f.p.y - f.radius() < [-22.0, -8.0, 6.0][self.difficulty] {
                f.overflow += dt;
            } else {
                f.overflow = (f.overflow - dt * 2.0).max(0.0);
            }
            if f.overflow >= 2.2 {
                self.finished = true;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn undo_restores_rng_preview_and_score() {
        let mut g = Game::new(42);
        g.delay = 0.0;
        let next = g.next;
        let seed = g.seed;
        g.drop();
        for _ in 0..100 {
            g.tick(0.02);
        }
        assert!(g.undo());
        assert_eq!(g.next, next);
        assert_eq!(g.seed, seed);
        assert!(g.fruits.is_empty());
        assert_eq!(g.score, 0);
    }
    #[test]
    fn touching_pair_merges_once() {
        let mut g = Game::new(2);
        for x in [300.0, 340.0] {
            g.fruits.push(Fruit {
                kind: 0,
                p: Vec2::new(x, 800.0),
                v: Vec2::default(),
                angle: 0.0,
                spin: 0.0,
                age: 1.0,
                overflow: 0.0,
                merge_age: 1.0,
            });
        }
        g.tick(0.02);
        assert_eq!(g.fruits.len(), 1);
        assert_eq!(g.fruits[0].kind, 1);
        assert_eq!(g.score, 1);
    }
    #[test]
    fn drop_cooldown_and_stable_pile() {
        let mut g = Game::new(14);
        for i in 0..25 {
            g.delay = 0.0;
            g.aim = 100.0 + (i % 4) as f64 * 170.0;
            g.drop();
            assert!(!g.drop());
            for _ in 0..80 {
                g.tick(0.02);
            }
        }
        assert!(g.valid());
        assert!(g.fruits.iter().all(|f| f.p.y < 920.0));
    }
}
