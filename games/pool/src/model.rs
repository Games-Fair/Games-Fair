// SPDX-License-Identifier: GPL-2.0-or-later
use crate::rolling::Rotation;
use serde::{Deserialize, Serialize};
pub const WIDTH: f64 = 360.0;
pub const HEIGHT: f64 = 560.0;
pub const BALL_R: f64 = 15.0;
pub use gamekit::multiplayer::Vec2;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ball {
    pub pos: Vec2,
    pub velocity: Vec2,
    pub pocketed: bool,
    pub orientation: Rotation,
}
/// Transient presentation event, captured before a pocketed cue ball can be respotted.
#[derive(Clone, Debug)]
pub struct PocketEvent {
    pub ball: usize,
    pub pocket: usize,
    pub entry: Vec2,
    pub orientation: Rotation,
    pub velocity: Vec2,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub humans: usize,
    pub count: usize,
    pub balls: Vec<Ball>,
    pub scores: [u32; 4],
    pub turn: usize,
    pub elapsed: f64,
    pub countdown: f64,
    pub finished: bool,
    #[serde(default)]
    pub round_wins: [u32; 4],
    #[serde(default)]
    pub recorded: bool,
    pub moving: bool,
    pub scratch: bool,
    pub shot_score: u32,
    pub ai_wait: f64,
    pub winner: Option<usize>,
    pub black_early: bool,
    accumulator: f64,
    #[serde(skip)]
    pub impact: f64,
    #[serde(skip)]
    pub ball_impact: f64,
    #[serde(skip)]
    pub cushion_impact: f64,
    #[serde(skip)]
    pub cue_impact: f64,
    #[serde(skip)]
    pub pocket_events: Vec<PocketEvent>,
}
impl Game {
    pub fn new(humans: usize) -> Self {
        let humans = humans.clamp(1, 2);
        let count = humans.max(2);
        let mut balls = vec![
            Ball {
                pos: Vec2::default(),
                velocity: Vec2::default(),
                pocketed: false,
                orientation: Rotation::default()
            };
            16
        ];
        balls[0].pos = Vec2::new(180.0, 410.0);
        let rack = rack_order(gamekit::seed());
        let spacing = BALL_R * 2.0 + 0.02;
        let mut slot = 0;
        for row in 0..5 {
            for col in 0..=row {
                balls[rack[slot]].pos = Vec2::new(
                    WIDTH / 2.0 + (col as f64 - row as f64 / 2.0) * spacing,
                    26.0 + (HEIGHT - 52.0) * 0.25 - row as f64 * spacing * 3.0_f64.sqrt() / 2.0,
                );
                slot += 1;
            }
        }
        Self {
            humans,
            count,
            balls,
            scores: [0; 4],
            turn: 0,
            elapsed: 0.0,
            countdown: 0.0,
            finished: false,
            round_wins: [0; 4],
            recorded: false,
            moving: false,
            scratch: false,
            shot_score: 0,
            ai_wait: 0.0,
            winner: None,
            black_early: false,
            accumulator: 0.0,
            impact: 0.0,
            ball_impact: 0.0,
            cushion_impact: 0.0,
            cue_impact: 0.0,
            pocket_events: Vec::new(),
        }
    }
    pub fn valid(&self) -> bool {
        (1..=2).contains(&self.humans)
            && self.count == self.humans.max(2)
            && self.turn < self.count
            && self.elapsed.is_finite()
            && (0.0..=3600.0).contains(&self.elapsed)
            && self.countdown.is_finite()
            && (0.0..=3.0).contains(&self.countdown)
            && self.accumulator.is_finite()
            && (0.0..=1.0).contains(&self.accumulator)
            && self.balls.len() == 16
            && self
                .balls
                .iter()
                .all(|b| b.pos.valid() && b.velocity.valid() && b.orientation.valid())
            && self.winner.is_none_or(|i| i < self.count)
            && self.scores.iter().all(|&s| s <= 15)
            && self.shot_score <= 15
            && self.ai_wait.is_finite()
            && (0.0..=1.0).contains(&self.ai_wait)
    }
    pub fn computer_turn(&self) -> bool {
        self.humans == 1 && self.turn == 1 && !self.moving && !self.finished
    }
    pub fn needs_clock(&self) -> bool {
        !self.finished && (self.moving || self.computer_turn())
    }
    pub fn tick(&mut self, dt: f64, _controls: [Option<Vec2>; 4]) {
        self.pocket_events.clear();
        self.impact = 0.0;
        self.ball_impact = 0.0;
        self.cushion_impact = 0.0;
        self.cue_impact = 0.0;
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
        self.elapsed += active;
        self.accumulator += active.min(0.1);
        while self.accumulator >= 1.0 / 120.0 && !self.finished {
            self.accumulator -= 1.0 / 120.0;
            self.pool_step(1.0 / 120.0)
        }
        if self.computer_turn() {
            self.ai_wait += active;
            if self.ai_wait >= 0.8 {
                self.ai_wait = 0.0;
                self.computer_shot();
            }
        }
    }
    pub fn shoot(&mut self, direction: Vec2) -> bool {
        if self.finished || self.moving || self.countdown > 0.0 || !direction.valid() {
            return false;
        }
        let length = direction.x.hypot(direction.y);
        if length < 4.0 {
            return false;
        }
        let speed = (length * 4.5).clamp(70.0, 700.0);
        self.balls[0].velocity =
            Vec2::new(direction.x / length * speed, direction.y / length * speed);
        self.cue_impact = speed;
        self.moving = true;
        self.scratch = false;
        self.shot_score = 0;
        self.black_early = false;
        true
    }
    pub fn shot_guide(&self, direction: Vec2) -> Vec2 {
        let cue = self.balls[0].pos;
        let length = direction.x.hypot(direction.y).max(0.001);
        let v = Vec2::new(direction.x / length, direction.y / length);
        for distance in 1..700 {
            let p = Vec2::new(cue.x + v.x * distance as f64, cue.y + v.y * distance as f64);
            if p.x <= 26.0
                || p.x >= WIDTH - 26.0
                || p.y <= 26.0
                || p.y >= HEIGHT - 26.0
                || self
                    .balls
                    .iter()
                    .skip(1)
                    .any(|b| !b.pocketed && b.pos.distance(p) <= BALL_R * 2.0)
            {
                return p;
            }
        }
        cue
    }
    fn computer_shot(&mut self) {
        let cue = self.balls[0].pos;
        let mut best = (f64::INFINITY, Vec2::new(0.0, -100.0));
        for (i, b) in self.balls.iter().enumerate().skip(1).filter(|(i, b)| {
            !b.pocketed
                && (if self.scores[self.turn] == 7 {
                    *i == 8
                } else {
                    *i != 8 && usize::from(*i > 8) == self.turn
                })
        }) {
            let _ = i;
            for pocket in pockets() {
                let length = b.pos.distance(pocket).max(1.0);
                let ghost = Vec2::new(
                    b.pos.x - (pocket.x - b.pos.x) / length * (BALL_R * 2.0),
                    b.pos.y - (pocket.y - b.pos.y) / length * (BALL_R * 2.0),
                );
                let cost = cue.distance(ghost) + length * 0.4;
                if cost < best.0 {
                    best = (cost, Vec2::new(ghost.x - cue.x, ghost.y - cue.y));
                }
            }
        }
        self.shoot(best.1);
    }
    fn pool_step(&mut self, dt: f64) {
        if !self.moving {
            return;
        }
        for (i, b) in self.balls.iter_mut().enumerate() {
            if b.pocketed {
                continue;
            }
            let previous = b.pos;
            b.pos.x += b.velocity.x * dt;
            b.pos.y += b.velocity.y * dt;
            if let Some(pocket) = pockets().iter().position(|&p| b.pos.distance(p) < 25.0) {
                b.orientation
                    .roll(b.pos.x - previous.x, b.pos.y - previous.y);
                self.pocket_events.push(PocketEvent {
                    ball: i,
                    pocket,
                    entry: b.pos,
                    orientation: b.orientation,
                    velocity: b.velocity,
                });
                b.pocketed = true;
                b.velocity = Vec2::default();
                if i == 0 {
                    self.scratch = true;
                } else if i == 8 {
                    self.black_early = self.scores[self.turn] != 7;
                } else {
                    let owner = usize::from(i > 8);
                    self.scores[owner] += 1;
                    if owner == self.turn {
                        self.shot_score += 1;
                    }
                }
                continue;
            }
            if b.pos.x < 26.0 || b.pos.x > WIDTH - 26.0 {
                self.impact = self.impact.max(b.velocity.x.abs());
                self.cushion_impact = self.cushion_impact.max(b.velocity.x.abs());
                b.velocity.x *= -0.88;
                b.pos.x = b.pos.x.clamp(26.0, WIDTH - 26.0);
            }
            if b.pos.y < 26.0 || b.pos.y > HEIGHT - 26.0 {
                self.impact = self.impact.max(b.velocity.y.abs());
                self.cushion_impact = self.cushion_impact.max(b.velocity.y.abs());
                b.velocity.y *= -0.88;
                b.pos.y = b.pos.y.clamp(26.0, HEIGHT - 26.0);
            }
            b.orientation
                .roll(b.pos.x - previous.x, b.pos.y - previous.y);
            let speed = b.velocity.x.hypot(b.velocity.y);
            let scale = ((speed - 65.0 * dt) / speed.max(0.001)).max(0.0);
            b.velocity.x *= scale;
            b.velocity.y *= scale;
        }
        for i in 0..self.balls.len() {
            let (left, right) = self.balls.split_at_mut(i + 1);
            let a = &mut left[i];
            if a.pocketed {
                continue;
            }
            for b in right {
                if b.pocketed {
                    continue;
                }
                let dist = a.pos.distance(b.pos);
                if dist >= BALL_R * 2.0 {
                    continue;
                }
                let (nx, ny) = if dist < 0.001 {
                    (1.0, 0.0)
                } else {
                    ((b.pos.x - a.pos.x) / dist, (b.pos.y - a.pos.y) / dist)
                };
                let overlap = (BALL_R * 2.0 - dist) / 2.0 + 0.001;
                a.pos.x -= nx * overlap;
                a.pos.y -= ny * overlap;
                b.pos.x += nx * overlap;
                b.pos.y += ny * overlap;
                let approach =
                    (a.velocity.x - b.velocity.x) * nx + (a.velocity.y - b.velocity.y) * ny;
                if approach > 0.0 {
                    self.impact = self.impact.max(approach);
                    self.ball_impact = self.ball_impact.max(approach);
                    let impulse = approach * 0.96;
                    a.velocity.x -= impulse * nx;
                    a.velocity.y -= impulse * ny;
                    b.velocity.x += impulse * nx;
                    b.velocity.y += impulse * ny;
                }
            }
        }
        if self
            .balls
            .iter()
            .all(|b| b.pocketed || b.velocity.x.hypot(b.velocity.y) < 1.0)
        {
            self.moving = false;
            for b in &mut self.balls {
                b.velocity = Vec2::default();
            }
            if self.balls[8].pocketed {
                self.finished = true;
                self.winner = Some(if self.black_early || self.scratch {
                    1 - self.turn
                } else {
                    self.turn
                });
            }
            if self.scratch {
                self.balls[0].pocketed = false;
                self.balls[0].pos = self.respot();
            }
            if self.shot_score == 0 || self.scratch {
                self.turn = 1 - self.turn;
            }
            self.ai_wait = 0.0;
        }
    }
    fn respot(&self) -> Vec2 {
        for y in (60..510).rev().step_by(25) {
            for x in (45..320).step_by(25) {
                let p = Vec2::new(x as f64, y as f64);
                if self
                    .balls
                    .iter()
                    .skip(1)
                    .all(|b| b.pocketed || p.distance(b.pos) > BALL_R * 2.0 + 2.0)
                {
                    return p;
                }
            }
        }
        Vec2::new(180.0, 410.0)
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
        if let Some(winner) = self.winner {
            return vec![winner];
        }
        let best = self.scores[..self.count].iter().copied().max().unwrap_or(0);
        (0..self.count)
            .filter(|&i| self.scores[i] == best)
            .collect()
    }
}
/// WPA rack constraints: eight in the center and opposite groups in the back corners;
/// the remaining positions have no intentional pattern. The ball vector stays indexed by number.
fn rack_order(mut seed: u64) -> [usize; 15] {
    seed |= 1;
    let mut random = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let solid = 1 + random() as usize % 7;
    let stripe = 9 + random() as usize % 7;
    let mut remaining: Vec<_> = (1..=15)
        .filter(|&n| n != 8 && n != solid && n != stripe)
        .collect();
    for i in (1..remaining.len()).rev() {
        let j = random() as usize % (i + 1);
        remaining.swap(i, j);
    }
    let corners = if random() & 1 == 0 {
        [solid, stripe]
    } else {
        [stripe, solid]
    };
    let mut rack = [0; 15];
    rack[4] = 8;
    rack[10] = corners[0];
    rack[14] = corners[1];
    for (slot, ball) in rack.iter_mut().filter(|slot| **slot == 0).zip(remaining) {
        *slot = ball;
    }
    rack
}
pub fn pockets() -> [Vec2; 6] {
    [
        Vec2::new(18.0, 18.0),
        Vec2::new(WIDTH - 18.0, 18.0),
        Vec2::new(16.0, HEIGHT / 2.0),
        Vec2::new(WIDTH - 16.0, HEIGHT / 2.0),
        Vec2::new(18.0, HEIGHT - 18.0),
        Vec2::new(WIDTH - 18.0, HEIGHT - 18.0),
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pool_shots_settle_and_pass_on_miss() {
        let mut g = Game::new(2);
        assert!(g.shoot(Vec2::new(20.0, 0.0)));
        assert!(!g.shoot(Vec2::new(20.0, 0.0)));
        for _ in 0..2000 {
            g.tick(1.0 / 120.0, [None; 4]);
        }
        assert!(!g.moving);
        assert_eq!(g.turn, 1);
        assert!(g.valid());
    }
    #[test]
    fn pool_pot_scores_retains_turn_and_scratch_respots() {
        let mut g = Game::new(2);
        g.balls[1].pos = pockets()[0];
        g.moving = true;
        g.tick(0.02, [None; 4]);
        assert_eq!(g.scores[0], 1);
        assert_eq!(g.turn, 0);
        g.balls[0].pos = pockets()[0];
        g.moving = true;
        g.shot_score = 0;
        g.tick(0.02, [None; 4]);
        assert_eq!(g.scores[0], 1);
        assert_eq!(g.turn, 1);
        assert!(!g.balls[0].pocketed);
        assert!(
            g.balls
                .iter()
                .skip(1)
                .all(|b| b.pocketed || b.pos.distance(g.balls[0].pos) > 20.0)
        );
    }
}

#[cfg(test)]
mod impact_tests {
    use super::*;
    #[test]
    fn rail_contact_reports_strength_once_for_feedback() {
        let mut game = Game::new(2);
        for ball in game.balls.iter_mut().skip(1) {
            ball.pocketed = true;
        }
        game.balls[0].pos = Vec2::new(27.0, 200.0);
        game.balls[0].velocity = Vec2::new(-240.0, 0.0);
        game.moving = true;
        game.tick(0.03, [None; 4]);
        assert!(game.impact > 200.0);
        assert!(game.balls[0].velocity.x > 0.0);
        game.tick(0.01, [None; 4]);
        assert_eq!(game.impact, 0.0);
    }
}

#[cfg(test)]
mod feedback_tests {
    use super::*;
    #[test]
    fn simultaneous_pots_emit_once_and_scratch_keeps_its_entry_before_respot() {
        let mut g = Game::new(2);
        g.balls[0].pos = pockets()[2];
        g.balls[0].velocity = Vec2::new(-120.0, 0.0);
        g.balls[1].pos = pockets()[0];
        g.balls[9].pos = pockets()[1];
        g.moving = true;
        g.tick(0.02, [None; 4]);
        assert_eq!(g.pocket_events.len(), 3);
        let cue = g.pocket_events.iter().find(|e| e.ball == 0).unwrap();
        assert_eq!(cue.pocket, 2);
        assert!(cue.entry.distance(pockets()[2]) < 2.0);
        assert_eq!(cue.velocity.x, -120.0);
        assert!(!g.balls[0].pocketed);
        assert!(g.balls[0].pos.distance(cue.entry) > 100.0);
        assert_eq!(g.scores, [1, 1, 0, 0]);
        assert_eq!(g.turn, 1);
        let save = serde_json::to_string(&g).unwrap();
        assert!(!save.contains("pocket_events"));
        let restored: Game = serde_json::from_str(&save).unwrap();
        assert!(restored.pocket_events.is_empty());
        g.tick(0.02, [None; 4]);
        assert!(g.pocket_events.is_empty());
        assert_eq!(g.scores, [1, 1, 0, 0]);
    }
    #[test]
    fn final_black_ball_keeps_its_drop_event_after_round_ends() {
        let mut g = Game::new(2);
        g.scores[0] = 7;
        g.balls[8].pos = pockets()[5];
        g.moving = true;
        g.tick(0.02, [None; 4]);
        assert!(g.finished);
        assert_eq!(g.winner, Some(0));
        assert_eq!(g.pocket_events[0].ball, 8);
        g.tick(0.02, [None; 4]);
        assert!(g.pocket_events.is_empty());
        assert_eq!(g.impact, 0.0);
    }
    #[test]
    fn ball_feedback_uses_normal_approach_speed_and_ignores_separating_contacts() {
        for (speed, expected) in [(80.0, 80.0), (600.0, 600.0), (-200.0, 0.0)] {
            let mut g = Game::new(2);
            g.balls[0].pos = Vec2::new(150.0, 350.0);
            g.balls[1].pos = Vec2::new(179.0, 350.0);
            // Tangential motion must not inflate the contact strength.
            g.balls[0].velocity = Vec2::new(speed, 300.0);
            g.balls[1].velocity = Vec2::new(0.0, 300.0);
            g.moving = true;
            g.pool_step(0.000001);
            assert!((g.ball_impact - expected).abs() < 0.01);
            assert_eq!(g.cushion_impact, 0.0);
        }
    }
    #[test]
    fn computer_and_human_shots_report_actual_clamped_cue_strength() {
        let mut g = Game::new(2);
        assert!(g.shoot(Vec2::new(8.0, 0.0)));
        assert_eq!(g.cue_impact, 70.0);
        g.moving = false;
        assert!(g.shoot(Vec2::new(180.0, 0.0)));
        assert_eq!(g.cue_impact, 700.0);
        let mut ai = Game::new(1);
        ai.turn = 1;
        for _ in 0..9 {
            ai.tick(0.1, [None; 4]);
        }
        assert!(ai.cue_impact > 0.0);
        assert!(ai.moving);
        ai.tick(0.01, [None; 4]);
        assert_eq!(ai.cue_impact, 0.0);
    }
    #[test]
    fn rolling_orientation_survives_pots_pause_and_save_without_teleport_spin() {
        let mut g = Game::new(2);
        g.balls[1].pos = Vec2::new(42.0, 280.0);
        g.balls[1].velocity = Vec2::new(-120.0, 0.0);
        g.moving = true;
        g.tick(0.02, [None; 4]);
        let event = g.pocket_events.iter().find(|p| p.ball == 1).unwrap();
        let marker = event.orientation.apply([0.0, 0.0, 1.0]);
        assert!(marker[0] < 0.0);
        assert_eq!(marker, g.balls[1].orientation.apply([0.0, 0.0, 1.0]));
        let saved: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert!(saved.valid());
        assert_eq!(marker, saved.balls[1].orientation.apply([0.0, 0.0, 1.0]));
        g.tick(0.1, [None; 4]);
        assert_eq!(marker, g.balls[1].orientation.apply([0.0, 0.0, 1.0]));
    }
    #[test]
    fn racks_keep_the_eight_centered_and_opposite_groups_in_the_rear_corners() {
        let mut variants = std::collections::HashSet::new();
        for seed in 1..128 {
            let rack = rack_order(seed);
            assert_eq!(rack[4], 8);
            assert_ne!(rack[10] > 8, rack[14] > 8);
            let mut numbers = rack;
            numbers.sort_unstable();
            assert_eq!(numbers, std::array::from_fn(|i| i + 1));
            variants.insert(rack);
        }
        assert!(variants.len() > 50);
        let game = Game::new(2);
        let center = game.balls[8].pos;
        assert_eq!(center.x, WIDTH / 2.0);
        let mut neighbors = 0;
        for (i, a) in game.balls.iter().enumerate().skip(1) {
            for b in game.balls.iter().skip(i + 1) {
                let distance = a.pos.distance(b.pos);
                assert!(distance >= BALL_R * 2.0 - 1e-6);
                if distance < BALL_R * 2.0 + 0.03 {
                    neighbors += 1;
                }
            }
        }
        assert_eq!(neighbors, 30);
    }
}
