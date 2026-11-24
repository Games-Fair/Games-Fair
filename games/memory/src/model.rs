// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Match {
    pub players: usize,
    pub turn: usize,
    pub scores: [u32; 4],
    pub cells: [u8; 30],
    pub matched: [bool; 30],
    #[serde(default)]
    seen: [bool; 30],
    #[serde(default = "default_seed")]
    rng: u64,
    pub first: Option<usize>,
    pub second: Option<usize>,
    pub wait: f64,
    pub moves: u32,
    pub finished: bool,
    #[serde(default)]
    pub round_wins: [u32; 4],
    #[serde(default)]
    pub recorded: bool,
}
const MISMATCH_SECONDS: f64 = 1.8;
impl Match {
    pub fn new(players: usize, seed: u64) -> Self {
        let mut game = Self {
            players: players.clamp(1, 4),
            turn: 0,
            scores: [0; 4],
            cells: [0; 30],
            matched: [false; 30],
            seen: [false; 30],
            rng: seed | 1,
            first: None,
            second: None,
            wait: 0.0,
            moves: 0,
            finished: false,
            round_wins: [0; 4],
            recorded: false,
        };
        let mut rng = seed | 1;
        let omitted = (random(&mut rng) % 16) as u8;
        for (i, card) in game.cells[..30].iter_mut().enumerate() {
            let pair = (i / 2) as u8;
            *card = pair + u8::from(pair >= omitted);
        }
        for i in (1..30).rev() {
            let j = random(&mut rng) as usize % (i + 1);
            game.cells.swap(i, j);
        }
        game.rng = rng;
        game
    }
    /// Reject malformed saves before any indexing, AI search, or timed work.
    pub fn valid(&self) -> bool {
        if !(1..=4).contains(&self.players)
            || self.turn >= self.players.max(2)
            || !self.wait.is_finite()
            || !(0.0..=MISMATCH_SECONDS).contains(&self.wait)
            || self.first.is_some_and(|i| i >= 30)
            || self.second.is_some_and(|i| i >= 30)
        {
            return false;
        }
        self.cells.iter().all(|&v| v < 16)
            && (0..16).all(|v| matches!(self.cells.iter().filter(|&&c| c == v).count(), 0 | 2))
            && (0..16).all(|v| {
                self.cells[..30]
                    .iter()
                    .enumerate()
                    .filter(|&(i, &c)| c == v && self.matched[i])
                    .count()
                    % 2
                    == 0
            })
            && self.first.is_none_or(|i| !self.matched[i])
            && self.second.is_none_or(|i| {
                self.first
                    .is_some_and(|j| i != j && self.cells[i] != self.cells[j])
                    && !self.matched[i]
            })
            && self.scores.iter().all(|&s| s <= 15)
            && self.scores.iter().map(|&s| u64::from(s)).sum::<u64>() * 2
                == self.matched.iter().filter(|&&m| m).count() as u64
            && self.finished == self.matched[..30].iter().all(|&m| m)
    }
    pub fn needs_clock(&self) -> bool {
        !self.finished && (self.second.is_some() || self.computer_turn())
    }
    fn computer_turn(&self) -> bool {
        self.players == 1 && self.turn == 1
    }
    pub fn play(&mut self, i: usize) -> bool {
        !self.computer_turn() && self.reveal(i)
    }
    fn reveal(&mut self, i: usize) -> bool {
        if self.finished || i >= 30 {
            return false;
        }
        if self.second.is_some() || self.matched[i] || self.first == Some(i) {
            return false;
        }
        self.seen[i] = true;
        if let Some(first) = self.first {
            self.moves = self.moves.saturating_add(1);
            if self.cells[first] == self.cells[i] {
                self.matched[first] = true;
                self.matched[i] = true;
                self.scores[self.turn] += 1;
                self.first = None;
                self.finished = self.matched[..30].iter().all(|&m| m);
            } else {
                self.second = Some(i);
                self.wait = 0.0;
            }
        } else {
            self.first = Some(i);
        }
        true
    }
    /// Choose using only previously revealed faces. Explore an unseen card when no
    /// known pair is available.
    fn choose_card(&mut self) -> Option<usize> {
        let available: Vec<_> = (0..self.cells.len())
            .filter(|&i| !self.matched[i] && self.first != Some(i))
            .collect();
        for &i in available.iter().filter(|&&i| self.seen[i]) {
            let mate = self.first.map_or_else(
                || {
                    available
                        .iter()
                        .any(|&j| j != i && self.seen[j] && self.cells[i] == self.cells[j])
                },
                |first| self.cells[i] == self.cells[first],
            );
            if mate {
                return Some(i);
            }
        }
        let unseen: Vec<_> = available
            .iter()
            .copied()
            .filter(|&i| !self.seen[i])
            .collect();
        let choices = if unseen.is_empty() {
            &available
        } else {
            &unseen
        };
        if choices.is_empty() {
            return None;
        }
        Some(choices[random(&mut self.rng) as usize % choices.len()])
    }
    /// Returns the card revealed by the computer so its flip uses the same feedback
    /// and collection animation as a human move.
    pub fn tick(&mut self, dt: f64) -> Option<usize> {
        if !self.needs_clock() || !dt.is_finite() || dt < 0.0 {
            return None;
        }
        self.wait += dt;
        if self.second.is_some() {
            if self.wait >= MISMATCH_SECONDS {
                self.first = None;
                self.second = None;
                self.wait = 0.0;
                self.turn = (self.turn + 1) % self.players.max(2);
            }
        } else if self.computer_turn() && self.wait >= 1.0 {
            self.wait = 0.0;
            let i = self.choose_card()?;
            return self.reveal(i).then_some(i);
        }
        None
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
fn default_seed() -> u64 {
    1
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
    fn computer_remembers_revealed_pairs_and_keeps_its_turn() {
        let mut g = Match::new(1, 15);
        let mate = (1..30).find(|&i| g.cells[i] == g.cells[0]).unwrap();
        let other = (1..30).find(|&i| i != mate).unwrap();
        g.seen[mate] = true;
        assert!(g.play(0) && g.play(other));
        g.tick(MISMATCH_SECONDS);
        assert_eq!(g.turn, 1);
        assert!(!g.play(mate));
        assert_eq!(g.tick(1.0), Some(0));
        assert_eq!(g.tick(1.0), Some(mate));
        assert_eq!(g.scores, [0, 1, 0, 0]);
        assert_eq!(g.turn, 1);
        assert!(g.valid());
    }
    #[test]
    fn memory_mismatch_blocks_input_then_passes_and_pairs_keep_turn() {
        let mut g = Match::new(4, 15);
        let other = (1..30).find(|&i| g.cells[i] != g.cells[0]).unwrap();
        assert!(g.play(0));
        assert!(!g.play(0));
        assert!(g.play(other));
        assert!(!g.play(15));
        g.tick(1.0);
        assert!(g.valid());
        assert_eq!(g.turn, 0);
        g.tick(0.8);
        assert_eq!(g.turn, 1);
        assert_eq!(g.first, None);
        for v in 0..16 {
            let pair: Vec<_> = (0..30).filter(|&i| g.cells[i] == v).collect();
            if pair.is_empty() {
                continue;
            }
            assert!(g.play(pair[0]));
            assert!(g.play(pair[1]));
            assert_eq!(g.turn, 1);
        }
        assert!(g.finished);
        assert_eq!(g.scores, [0, 15, 0, 0]);
        assert!(g.valid());
    }
}
