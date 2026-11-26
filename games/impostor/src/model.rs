// SPDX-License-Identifier: GPL-2.0-or-later
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: usize,
    pub title: String,
    pub words: Vec<String>,
}
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Reveal,
    Discussion,
    Voting,
    Result,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub roles: usize,
    pub hint: bool,
    pub enabled: Vec<usize>,
    pub language: usize,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            roles: 1,
            hint: false,
            enabled: (1..=12).collect(),
            language: 0,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub count: usize,
    pub selected_players: usize,
    pub turn: usize,
    pub names: Vec<String>,
    pub scores: Vec<u32>,
    pub phase: Phase,
    pub impostors: Vec<usize>,
    pub word: String,
    pub category: String,
    pub round: u32,
    pub caught: bool,
    pub seen: bool,
    pub seed: u64,
    // Secrets never remain revealed when state is restored, paused, or backgrounded.
    #[serde(skip)]
    pub held: bool,
    #[serde(skip)]
    pub hold_age: f64,
    #[serde(skip)]
    pub flip: f64,
    pub previous_word: String,
}
impl Game {
    pub fn new(players: usize, seed: u64) -> Self {
        let count = players.clamp(3, 10);
        Self {
            count,
            selected_players: players,
            turn: 0,
            names: vec![String::new(); count],
            scores: vec![0; count],
            phase: Phase::Reveal,
            impostors: vec![],
            word: String::new(),
            category: String::new(),
            round: 0,
            caught: false,
            seen: false,
            seed: seed.max(1),
            held: false,
            hold_age: 0.0,
            flip: 0.0,
            previous_word: String::new(),
        }
    }
    fn random(&mut self, n: usize) -> usize {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.seed >> 32) as usize) % n
    }
    pub fn start(&mut self, categories: &[Category], settings: &Settings) -> bool {
        let available = categories
            .iter()
            .filter(|c| settings.enabled.contains(&c.id) && !c.words.is_empty())
            .collect::<Vec<_>>();
        if available.is_empty() {
            return false;
        }
        let category = available[self.random(available.len())];
        let mut word = self.random(category.words.len());
        if category.words[word] == self.previous_word && category.words.len() > 1 {
            word = (word + 1) % category.words.len();
        }
        self.word = category.words[word].clone();
        self.previous_word = self.word.clone();
        self.category = category.title.clone();
        self.impostors.clear();
        let mut shuffled = (0..self.count).collect::<Vec<_>>();
        for i in (1..self.count).rev() {
            let j = self.random(i + 1);
            shuffled.swap(i, j);
        }
        self.impostors
            .extend_from_slice(&shuffled[..settings.roles.clamp(1, 2).min(self.count - 1)]);
        self.turn = 0;
        self.phase = Phase::Reveal;
        self.round += 1;
        self.caught = false;
        self.seen = false;
        self.hide();
        true
    }
    pub fn valid(&self) -> bool {
        (3..=10).contains(&self.count)
            && self.names.len() == self.count
            && self.scores.len() == self.count
            && self.turn < self.count
            && (1..=2).contains(&self.impostors.len())
            && self.impostors.iter().all(|&p| p < self.count)
            && !self.word.is_empty()
    }
    pub fn hide(&mut self) {
        self.held = false;
        self.flip = 0.0;
        self.hold_age = 0.0;
    }
    pub fn tick(&mut self, dt: f64) -> bool {
        if self.phase != Phase::Reveal || !self.held {
            return false;
        }
        self.hold_age += dt;
        self.flip = (self.flip + dt * 4.0).min(1.0);
        if self.hold_age >= 0.25 && !self.seen {
            self.seen = true;
            return true;
        }
        false
    }
    pub fn next(&mut self) -> bool {
        if self.phase != Phase::Reveal || !self.seen || self.held {
            return false;
        }
        self.hide();
        self.seen = false;
        self.turn += 1;
        if self.turn == self.count {
            self.turn = 0;
            self.phase = Phase::Discussion;
        }
        true
    }
    pub fn vote(&mut self, p: usize) -> bool {
        if self.phase != Phase::Voting || p >= self.count {
            return false;
        }
        self.caught = self.impostors.contains(&p);
        for i in 0..self.count {
            if self.impostors.contains(&i) != self.caught {
                self.scores[i] += 1;
            }
        }
        self.phase = Phase::Result;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<Category> {
        vec![Category {
            id: 1,
            title: "fixture".into(),
            words: vec!["alpha".into(), "beta".into()],
        }]
    }
    #[test]
    fn private_reveals_require_holding_and_reset_between_players() {
        let mut g = Game::new(4, 2);
        g.start(&fixture(), &Settings::default());
        assert!(!g.next());
        g.held = true;
        assert!(!g.tick(0.1));
        assert!(g.tick(0.2));
        assert!(!g.next());
        g.hide();
        assert!(g.next());
        assert!(!g.held);
        assert!(!g.seen);
        assert_eq!(g.turn, 1);
        let saved = serde_json::to_string(&g).expect("validated game invariant");
        g = serde_json::from_str(&saved).expect("validated game invariant");
        assert!(!g.held);
    }
    #[test]
    fn collective_vote_scores_winning_team_once() {
        let mut g = Game::new(4, 9);
        g.start(&fixture(), &Settings::default());
        g.phase = Phase::Voting;
        let i = g.impostors[0];
        assert!(g.vote(i));
        assert!(!g.vote(i));
        assert_eq!(g.scores[i], 0);
        assert_eq!(g.scores.iter().sum::<u32>(), 3);
    }
    #[test]
    fn missing_categories_cannot_start_and_roles_are_unique() {
        let mut g = Game::new(3, 5);
        let mut s = Settings {
            roles: 2,
            ..Default::default()
        };
        assert!(g.start(&fixture(), &s));
        assert_ne!(g.impostors[0], g.impostors[1]);
        s.enabled.clear();
        assert!(!g.start(&fixture(), &s));
    }
}
