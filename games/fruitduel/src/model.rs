// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use gamekit::{board_game::Feedback, multiplayer::Vec2};
use serde::{Deserialize, Serialize};

pub const TARGET: u32 = 10;
pub struct Sprite {
    pub size: [f64; 2],
    pub pivot: [f64; 2],
}

const ART_UNIT: f64 = 0.016;
const GRIP: [f64; 2] = [39., 208.];
pub const SCIMITAR: Sprite = Sprite {
    size: [100. * ART_UNIT, 256. * ART_UNIT],
    pivot: [GRIP[0] / 100., 1. - GRIP[1] / 256.],
};
pub const FRUIT: Sprite = Sprite {
    size: [3.2, 3.2],
    pivot: [0.5, 0.5],
};
pub const WINDUP: f64 = 0.072;
pub const FOLLOW_THROUGH: f64 = 0.32;

// Samples of the sharpened edge in the illustration, from the guard to the tip.
pub const EDGE: [[f64; 2]; 9] = [
    [31., 180.],
    [29., 155.],
    [25., 126.],
    [26., 99.],
    [32., 78.],
    [42., 57.],
    [56., 37.],
    [73., 19.],
    [92., 5.],
];
pub fn blade_point([x, y]: [f64; 2]) -> Vec2 {
    Vec2::new((x - GRIP[0]) * ART_UNIT, (GRIP[1] - y) * ART_UNIT)
}
fn ease(t: f64) -> f64 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
#[derive(Clone, Copy, Serialize, Deserialize, Default, Debug)]
pub struct Sword {
    pub age: f64,
    pub active: bool,
    pub failed: bool,
    pub hit: bool,
    pub hit_age: f64,
    #[serde(default)]
    pub contact: Vec2,
}
impl Sword {
    pub fn duration(&self) -> f64 {
        FOLLOW_THROUGH + if self.failed { 1.78 } else { 0.34 }
    }

    pub fn angle_at(&self, age: f64) -> f64 {
        let age = if self.failed && self.hit && age > self.hit_age {
            // Rewind along the incoming path: each vibration backs the edge away
            // from the obstacle, so even the first rebound cannot penetrate it.
            let elapsed = age - self.hit_age;
            let judder = 0.025
                * (-12.0 * elapsed).exp()
                * (std::f64::consts::PI * 18.0 * elapsed).sin().abs();
            let retreat = ease((elapsed - 0.20) / (self.duration() - self.hit_age - 0.20));
            (self.hit_age - judder).max(0.) * (1. - retreat)
        } else {
            age
        };
        let rest = -80_f64;
        let drawn = -90_f64;
        let end = 68_f64;
        let swing = ((age - WINDUP) / (FOLLOW_THROUGH - WINDUP)).clamp(0., 1.);
        if age <= WINDUP {
            rest + (drawn - rest) * ease(age / WINDUP)
        } else if age <= FOLLOW_THROUGH {
            drawn + (end - drawn) * ease(swing)
        } else {
            end + (rest - end) * ease((age - FOLLOW_THROUGH) / (self.duration() - FOLLOW_THROUGH))
        }
        .to_radians()
    }

    pub fn angle(&self) -> f64 {
        self.angle_at(if self.active { self.age } else { 0. })
    }
}
/// Project a swing from its corner into the common playfield.
pub fn world(player: usize, angle: f64, v: Vec2) -> Vec2 {
    // The grips face each player's end of the phone. The drawn blade and its
    // collision outline both rotate about (0, 0), the center of the wrapping.
    let (s, c) = (241_f64.to_radians() - angle).sin_cos();
    let x = 2.11 + c * v.x + s * v.y;
    let y = -1.13 + s * v.x - c * v.y;
    match player {
        0 => Vec2::new(x, y),
        1 => Vec2::new(-x, -y),
        2 => Vec2::new(x, -y),
        _ => Vec2::new(-x, y),
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub humans: usize,
    pub count: usize,
    pub scores: [u32; 4],
    pub difficulty: usize,
    pub item: usize,
    pub item_age: f64,
    pub item_angle: f64,
    pub item_duration: f64,
    pub first: Option<usize>,
    pub played: [bool; 4],
    pub swords: [Sword; 4],
    pub reset_in: f64,
    pub split_age: f64,
    pub split_offset: Vec2,
    pub split_rotation: f64,
    pub finished: bool,
    pub seed: u64,
    pub cpu_wait: f64,
    pub cpu_will_tap: bool,
    accumulator: f64,
}
impl Game {
    pub fn new(humans: usize, seed: u64) -> Self {
        let mut g = Self {
            humans: humans.clamp(1, 4),
            count: humans.clamp(2, 4),
            scores: [0; 4],
            difficulty: 1,
            item: 6,
            item_age: 0.0,
            item_angle: 0.0,
            item_duration: 2.0,
            first: None,
            played: [false; 4],
            swords: [Sword::default(); 4],
            reset_in: 0.0,
            split_age: -1.0,
            split_offset: Vec2::default(),
            split_rotation: 0.0,
            finished: false,
            seed: seed.max(1),
            cpu_wait: 0.0,
            cpu_will_tap: false,
            accumulator: 0.0,
        };
        g.next_item();
        g
    }
    fn random(&mut self) -> f64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn valid(&self) -> bool {
        (1..=4).contains(&self.humans)
            && self.count == self.humans.max(2)
            && self.difficulty < 3
            && self.item < 6
            && self.first.is_none_or(|p| p < self.count)
            && self.scores.iter().all(|s| *s <= TARGET)
            && self.swords.iter().all(|s| {
                s.age.is_finite()
                    && s.age >= 0.0
                    && s.hit_age.is_finite()
                    && (0.0..=s.age).contains(&s.hit_age)
                    && s.contact.valid()
            })
            && self.split_offset.valid()
            && [
                self.item_age,
                self.item_duration,
                self.reset_in,
                self.split_age,
                self.split_rotation,
                self.cpu_wait,
                self.accumulator,
            ]
            .iter()
            .all(|v| v.is_finite())
    }
    fn cpu_config(&mut self) {
        // Adjust reaction time within the selected difficulty band as the score gap changes.
        let bands = [(0.1, 0.4), (0.4, 0.7), (0.7, 1.0)];
        let (lo, hi) = bands[self.difficulty];
        let gap = (self.scores[0] as f64 - self.scores[1] as f64) / 5.0;
        let level = lo + (hi - lo) * ((gap + 1.0) * 0.5).clamp(0.0, 1.0);
        self.cpu_wait = 0.75 + (0.30 - 0.75) * level + (self.random() - 0.5) * 0.10;
        let chance = 0.5 + self.random() * [0.2, 0.3, 0.4][self.difficulty];
        let correct = self.random() < chance;
        self.cpu_will_tap = if correct {
            self.item < 3
        } else {
            self.item >= 3
        };
    }
    pub fn next_item(&mut self) {
        // Draw uniformly from the five items which did not just appear.
        let next = (self.random() * if self.item < 6 { 5.0 } else { 6.0 }) as usize;
        self.item = if next >= self.item { next + 1 } else { next };
        self.item_age = 0.0;
        self.item_angle = (self.random() * 360.0).floor();
        self.split_offset = Vec2::default();
        let low = if self.humans == 1 {
            [2.0, 1.0, 0.5][self.difficulty]
        } else {
            1.0
        };
        self.item_duration = low + self.random() * (3.0 - low);
        self.first = None;
        self.played = [false; 4];
        self.reset_in = 0.0;
        self.split_age = -1.0;
        self.split_rotation = 0.0;
        self.cpu_config();
    }
    pub fn press(&mut self, p: usize) -> Vec<Feedback> {
        if p >= self.count || self.finished || self.played[p] || self.swords[p].active {
            return vec![];
        }
        self.played[p] = true;
        if self.first.is_none() {
            self.first = Some(p);
            self.reset_in = 1.0 + FOLLOW_THROUGH;
        }
        self.swords[p] = Sword {
            active: true,
            failed: self.item >= 3,
            ..Sword::default()
        };
        vec![Feedback::Tap]
    }
    fn cuts_fruit(p: usize, angle: f64) -> bool {
        EDGE.windows(2).any(|edge| {
            let a = world(p, angle, blade_point(edge[0]));
            let b = world(p, angle, blade_point(edge[1]));
            let v = b - a;
            let t = (-a.dot(v) / v.dot(v).max(1e-10)).clamp(0.0, 1.0);
            (a + v * t).length() < 0.18
        })
    }
    fn step(&mut self, dt: f64, events: &mut Vec<Feedback>) {
        self.item_age += dt;
        if self.split_age >= 0.0 {
            self.split_age += dt;
            let step = (self.split_age * 2.5).clamp(0.0, 1.0);
            self.split_rotation += 900.0 * (1.0 - step).powi(2) * dt;
        }
        for p in 0..self.count {
            if !self.swords[p].active {
                continue;
            }
            self.swords[p].age += dt;
            let sword = self.swords[p];
            if !sword.hit && (WINDUP..=FOLLOW_THROUGH).contains(&sword.age) {
                if sword.failed {
                    let angle = self.item_angle.to_radians().sin_cos();
                    let contact =
                        |age| crate::decoy::contact(self.item, angle, p, sword.angle_at(age));
                    if let Some(mut point) = contact(sword.age) {
                        // Bracket first contact within this physics tick. Render the last
                        // clear angle and place the sparks on the surface immediately beyond it.
                        let mut clear = sword.age - dt;
                        let mut blocked = sword.age;
                        for _ in 0..8 {
                            let mid = (clear + blocked) * 0.5;
                            if let Some(at) = contact(mid) {
                                blocked = mid;
                                point = at;
                            } else {
                                clear = mid;
                            }
                        }
                        self.swords[p].hit_age = clear;
                        self.swords[p].contact = point;
                        self.swords[p].hit = true;
                        events.push(Feedback::Miss);
                    }
                } else if Self::cuts_fruit(p, sword.angle()) {
                    self.swords[p].hit = true;
                    self.swords[p].hit_age = sword.age;
                    if self.first == Some(p) && self.split_age < 0.0 {
                        self.scores[p] += 1;
                        self.split_age = 0.0;
                        let sword = self.swords[p];
                        let belly = blade_point(EDGE[5]);
                        let velocity = world(p, sword.angle(), belly)
                            - world(p, sword.angle_at(sword.age - dt), belly);
                        self.split_offset = velocity * (2.2 / velocity.length().max(1e-8));
                        events.push(Feedback::Score);
                        if self.scores[p] >= TARGET {
                            self.finished = true;
                        }
                    }
                }
            }
            if self.swords[p].age >= self.swords[p].duration() {
                self.swords[p].active = false;
            }
        }
        if self.finished {
            return;
        }
        if self.humans == 1 && self.cpu_will_tap && self.item_age >= self.cpu_wait {
            self.cpu_will_tap = false;
            let result = self.press(1);
            if !result.is_empty() {
                self.cpu_will_tap = false;
                events.extend(result);
            }
        }
        if self.first.is_some() {
            self.reset_in -= dt;
            if self.reset_in <= 0.0
                && !self
                    .swords
                    .iter()
                    .any(|s| s.active && !s.hit && s.age <= FOLLOW_THROUGH)
            {
                self.next_item();
            }
        } else if self.item_age >= self.item_duration {
            self.next_item();
        }
    }
    pub fn tick(&mut self, dt: f64) -> Vec<Feedback> {
        let mut events = vec![];
        if !dt.is_finite() || dt <= 0.0 {
            return events;
        }
        // Keep slice animation running behind the victory card; no new input/items after a win.
        self.accumulator += dt.min(0.1);
        while self.accumulator >= 1.0 / 240.0 {
            self.accumulator -= 1.0 / 240.0;
            self.step(1.0 / 240.0, &mut events);
        }
        events
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simultaneous_blades_award_only_the_first_player() {
        let mut g = Game::new(4, 7);
        g.item = 0;
        g.press(3);
        g.press(0);
        g.press(1);
        for _ in 0..60 {
            g.tick(0.01);
        }
        assert_eq!(g.scores, [0, 0, 0, 1]);
        assert!(g.split_age > 0.0);
    }
    #[test]
    fn belly_of_the_blade_sweeps_through_the_fruit_from_every_corner() {
        for player in 0..4 {
            let mut game = Game::new(4, 7);
            game.item = 0;
            game.press(player);
            let mut crossed = false;
            for _ in 0..170 {
                game.tick(1. / 240.);
                let sword = game.swords[player];
                let angle = sword.angle();
                let belly = world(player, angle, blade_point(EDGE[5]));
                let tip = world(player, angle, blade_point(EDGE[8]));
                let grip = world(player, angle, Vec2::default());
                if sword.age < FOLLOW_THROUGH
                    && belly.length() < 0.2
                    && tip.dot(grip.normalized()) < -0.778
                {
                    crossed = true;
                }
            }
            assert!(
                crossed,
                "player {player}'s blade failed to clear the far rim"
            );
            assert_eq!(game.scores[player], 1);
        }
    }
    #[test]
    fn ten_points_finishes_and_cannot_accept_another_tap() {
        let mut g = Game::new(2, 7);
        g.item = 0;
        g.scores[0] = TARGET - 1;
        g.press(0);
        for _ in 0..60 {
            g.tick(0.01);
        }
        assert!(g.finished);
        assert_eq!(g.scores[0], TARGET);
        assert!(g.press(1).is_empty());
        let item = g.item;
        for _ in 0..300 {
            g.tick(0.01);
        }
        assert_eq!(g.item, item);
        assert!(g.valid());
    }
    #[test]
    fn wrist_sweeps_the_tip_through_contact_and_returns_to_rest() {
        let mut g = Game::new(4, 7);
        g.item = 0;
        g.press(0);
        let sword = g.swords[0];
        let wound = sword.angle_at(WINDUP);
        let end = sword.angle_at(FOLLOW_THROUGH);
        assert!((end - wound).to_degrees() >= 155.);
        for player in 0..4 {
            let at = |angle, pixel| world(player, angle, blade_point(pixel));
            let ready = sword.angle_at(0.);
            let grip = at(ready, GRIP);
            let tip = at(ready, EDGE[8]) - grip;
            // The resting tip points more than 90 degrees away from the target.
            assert!(tip.dot(Vec2::default() - grip) < 0.);
            for frame in 0..=180 {
                let angle = sword.angle_at(f64::from(frame) / 240.);
                assert_eq!(at(angle, GRIP).distance(grip), 0.);
                assert!(((at(angle, EDGE[8]) - grip).length() - tip.length()).abs() < 1e-12);
            }
        }
        for _ in 0..70 {
            g.tick(0.01);
        }
        assert!((WINDUP + 0.03..FOLLOW_THROUGH - 0.03).contains(&g.swords[0].hit_age));
        assert_eq!(g.swords[0].angle(), sword.angle_at(0.));
    }
    #[test]
    fn repeat_presses_and_late_players_cannot_steal_first_claim() {
        let mut g = Game::new(4, 1);
        g.item = 0;
        assert!(!g.press(2).is_empty());
        assert!(g.press(2).is_empty());
        g.press(0);
        assert_eq!(g.first, Some(2));
        assert!(g.played[0] && g.played[2]);
    }
    #[test]
    fn decoys_stop_the_cut_and_judder_on_the_approach_side() {
        let dt = 1. / 240.;
        for item in 3..6 {
            for player in 0..4 {
                for degrees in [0_f64, 32., 91., 217.] {
                    let mut game = Game::new(4, 7);
                    game.item = item;
                    game.item_angle = degrees;
                    game.press(player);
                    let mut clangs = 0;
                    for _ in 0..90 {
                        clangs += game
                            .tick(dt)
                            .iter()
                            .filter(|e| matches!(e, Feedback::Miss))
                            .count();
                    }
                    assert_eq!(
                        clangs, 1,
                        "item {item}, player {player}, rotation {degrees}"
                    );
                    assert_eq!(game.scores, [0; 4]);
                    let sword = game.swords[player];
                    let at = sword.hit_age;
                    let angle = |t| sword.angle_at(t);
                    assert!(angle(at) - angle(at + 0.026) > 0.075);
                    assert!(angle(at + 0.055) > angle(at + 0.026) + 0.05);
                    for tick in 0..=480 {
                        let angle = sword.angle_at(at + f64::from(tick) * dt);
                        assert!(
                            crate::decoy::contact(
                                item,
                                degrees.to_radians().sin_cos(),
                                player,
                                angle
                            )
                            .is_none(),
                            "penetrated item {item} after impact, player {player}, tick {tick}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn bad_tap_recovers_across_the_next_item_without_losing_points() {
        let mut g = Game::new(2, 4);
        g.item = 4;
        g.scores[0] = 3;
        g.press(0);
        for _ in 0..135 {
            g.tick(0.01);
        }
        assert!(g.first.is_none());
        assert!(g.swords[0].active);
        assert!(g.press(0).is_empty());
        assert_eq!(g.scores[0], 3);
        for _ in 0..100 {
            g.tick(0.01);
        }
        assert!(!g.swords[0].active);
    }
    #[test]
    fn targets_never_repeat_and_pause_restore_preserves_race() {
        let mut g = Game::new(4, 98);
        for _ in 0..100 {
            let last = g.item;
            g.next_item();
            assert_ne!(g.item, last);
            assert!(g.item < 6);
        }
        g.press(3);
        g.tick(0.1);
        let mut restored: Game =
            serde_json::from_str(&serde_json::to_string(&g).expect("fixture")).expect("fixture");
        g.tick(0.1);
        restored.tick(0.1);
        assert_eq!(g.first, restored.first);
        assert_eq!(g.scores, restored.scores);
        assert_eq!(g.seed, restored.seed);
    }
}
