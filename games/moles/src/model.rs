// SPDX-License-Identifier: GPL-2.0-or-later
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Match {
    pub players: usize,
    pub scores: [u32; 4],
    pub cells: [u8; 11],
    pub finished: bool,
    #[serde(default)]
    pub round_wins: [u32; 4],
    #[serde(default)]
    pub recorded: bool,
    pub elapsed: f64,
    seed: u64,
    pub mole_age: [f64; 11],
    pub countdown: f64,
}
impl Match {
    pub fn new(players: usize, seed: u64) -> Self {
        let mut game = Self {
            players: players.clamp(1, 4),
            scores: [0; 4],
            cells: [0; 11],
            finished: false,
            round_wins: [0; 4],
            recorded: false,
            elapsed: 0.0,
            seed: seed | 1,
            mole_age: [0.0; 11],
            countdown: 3.0,
        };
        for i in 0..game.players.max(2) {
            game.spawn_mole(i);
        }
        game
    }
    /// Reject malformed saves before any indexing, AI search, or timed work.
    pub fn valid(&self) -> bool {
        if !(1..=4).contains(&self.players)
            || !self.elapsed.is_finite()
            || !(0.0..=86400.0).contains(&self.elapsed)
            || !self.countdown.is_finite()
            || !(0.0..=3.0).contains(&self.countdown)
            || self
                .mole_age
                .iter()
                .any(|t| !t.is_finite() || !(0.0..=2.0).contains(t))
            || self.seed == 0
        {
            return false;
        }
        self.scores.iter().all(|&s| s <= 30)
            && self.cells[..11]
                .iter()
                .all(|&c| c as usize <= self.players.max(2))
            && self.finished == self.scores.contains(&30)
    }
    pub fn needs_clock(&self) -> bool {
        !self.finished
    }
    pub fn play(&mut self, i: usize) -> bool {
        if self.finished || self.countdown > 0.0 || i >= 11 {
            return false;
        }
        let owner = self.cells[i];
        if owner == 0 || self.mole_age[i] < 0.12 {
            return false;
        }
        self.cells[i] = 0;
        self.mole_age[i] = 0.0;
        self.scores[owner as usize - 1] += 1;
        self.finished = self.scores[owner as usize - 1] >= 30;
        true
    }
    /// Raw elapsed time advances the round fairly even if rendering stalls.
    pub fn tick(&mut self, dt: f64) {
        if !self.needs_clock() || !dt.is_finite() || dt < 0.0 {
            return;
        }
        if self.countdown > 0.0 {
            self.countdown = (self.countdown - dt).max(0.0);
            return;
        }
        let dt = dt.min(0.1);
        self.elapsed = (self.elapsed + dt).min(86400.0);
        for i in 0..11 {
            self.mole_age[i] += dt;
            if self.cells[i] != 0 && self.mole_age[i] >= 1.4 {
                self.cells[i] = 0;
                self.mole_age[i] = 0.0;
            }
            if self.cells[i] == 0 {
                self.mole_age[i] = self.mole_age[i].min(1.5);
            }
        }
        if self.players == 1 {
            for i in 0..11 {
                if self.cells[i] == 2 && self.mole_age[i] > 0.95 {
                    self.play(i);
                }
            }
        }
        if !self.finished {
            for owner in 0..self.players.max(2) {
                if !self.cells[..11].contains(&(owner as u8 + 1)) {
                    self.spawn_mole(owner);
                }
            }
        }
    }
    fn spawn_mole(&mut self, owner: usize) {
        let vacant: Vec<_> = (0..11).filter(|&i| self.cells[i] == 0).collect();
        if !vacant.is_empty() {
            let i = vacant[random(&mut self.seed) as usize % vacant.len()];
            self.cells[i] = owner as u8 + 1;
            self.mole_age[i] = 0.0;
        }
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
        let n = self.players.max(2);
        let best = self.scores[..n].iter().copied().max().unwrap_or(0);
        (0..n).filter(|&i| self.scores[i] == best).collect()
    }
}
fn random(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moles_simultaneous_colors_race_to_thirty_and_cannot_double_score() {
        let mut g = Match::new(4, 15);
        assert!(!g.play(0));
        g.tick(3.0);
        for point in 1..=30 {
            g.tick(0.1);
            g.tick(0.1);
            g.tick(0.1);
            let red = (0..11).find(|&i| g.cells[i] == 1).unwrap();
            assert!(g.play(red));
            assert!(!g.play(red));
            assert_eq!(g.scores[0], point);
            if point < 30 {
                let blue = (0..11).find(|&i| g.cells[i] == 2).unwrap();
                assert!(g.play(blue));
                assert!(g.valid());
            }
        }
        assert!(g.finished);
        assert_eq!(g.winners(), vec![0]);
        assert!(g.valid());
    }
}
