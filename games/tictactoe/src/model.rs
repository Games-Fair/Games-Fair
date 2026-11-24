// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Match {
    pub players: usize,
    pub turn: usize,
    pub scores: [u32; 4],
    pub cells: [u8; 9],
    pub wait: f64,
    pub moves: u32,
    pub finished: bool,
    #[serde(default)]
    pub round_wins: [u32; 4],
    #[serde(default)]
    pub recorded: bool,
}
impl Match {
    pub fn new(players: usize, _seed: u64) -> Self {
        Self {
            players: players.clamp(1, 2),
            turn: 0,
            scores: [0; 4],
            cells: [0; 9],
            wait: 0.0,
            moves: 0,
            finished: false,
            round_wins: [0; 4],
            recorded: false,
        }
    }
    /// Reject malformed saves before any indexing, AI search, or timed work.
    pub fn valid(&self) -> bool {
        if !(1..=2).contains(&self.players)
            || self.turn >= 2
            || !self.wait.is_finite()
            || !(0.0..=1.0).contains(&self.wait)
        {
            return false;
        }
        let x = self.cells[..9].iter().filter(|&&c| c == 1).count();
        let o = self.cells[..9].iter().filter(|&&c| c == 2).count();
        self.cells[..9].iter().all(|&c| c <= 2)
            && (x == o || x == o + 1)
            && self.moves == (x + o) as u32
            && self.scores.iter().all(|&s| s <= 1)
            && self.finished == (winner(&self.cells).is_some() || x + o == 9)
            && (self.finished || self.turn == usize::from(x != o))
    }
    pub fn computer_turn(&self) -> bool {
        self.players == 1 && self.turn == 1 && !self.finished
    }
    pub fn needs_clock(&self) -> bool {
        !self.finished && (self.computer_turn())
    }
    pub fn play(&mut self, i: usize) -> bool {
        if self.finished || self.computer_turn() || i >= 9 {
            return false;
        }
        self.mark(i)
    }
    fn mark(&mut self, i: usize) -> bool {
        if self.cells[i] != 0 {
            return false;
        }
        self.cells[i] = self.turn as u8 + 1;
        self.moves += 1;
        if let Some(w) = winner(&self.cells) {
            self.scores[w] = 1;
            self.finished = true;
        } else if self.moves == 9 {
            self.finished = true;
        } else {
            self.turn = 1 - self.turn;
        }
        self.wait = 0.0;
        true
    }
    /// Raw elapsed time advances the round fairly even if rendering stalls.
    pub fn tick(&mut self, dt: f64) {
        if !self.computer_turn() || !dt.is_finite() || dt < 0.0 {
            return;
        }
        self.wait += dt;
        if self.wait >= 0.45
            && let Some(i) = best_move(self.cells)
        {
            self.mark(i);
        }
    }
    pub fn winning_line(&self) -> Option<[usize; 3]> {
        [
            [0, 1, 2],
            [3, 4, 5],
            [6, 7, 8],
            [0, 3, 6],
            [1, 4, 7],
            [2, 5, 8],
            [0, 4, 8],
            [2, 4, 6],
        ]
        .into_iter()
        .find(|&[a, b, c]| {
            self.cells[a] != 0 && self.cells[a] == self.cells[b] && self.cells[b] == self.cells[c]
        })
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
fn winner(cells: &[u8; 9]) -> Option<usize> {
    for [a, b, c] in [
        [0, 1, 2],
        [3, 4, 5],
        [6, 7, 8],
        [0, 3, 6],
        [1, 4, 7],
        [2, 5, 8],
        [0, 4, 8],
        [2, 4, 6],
    ] {
        if cells[a] != 0 && cells[a] == cells[b] && cells[b] == cells[c] {
            return Some(cells[a] as usize - 1);
        }
    }
    None
}
fn minimax(mut cells: [u8; 9], turn: u8) -> i8 {
    if let Some(w) = winner(&cells) {
        return if w == 1 { 1 } else { -1 };
    }
    let mut values = Vec::new();
    for i in 0..9 {
        if cells[i] == 0 {
            cells[i] = turn;
            values.push(minimax(cells, 3 - turn));
            cells[i] = 0;
        }
    }
    if turn == 2 {
        values.into_iter().max().unwrap_or(0)
    } else {
        values.into_iter().min().unwrap_or(0)
    }
}
fn best_move(mut cells: [u8; 9]) -> Option<usize> {
    let mut best = None;
    let mut value = -2;
    for i in [4, 0, 2, 6, 8, 1, 3, 5, 7] {
        if cells[i] == 0 {
            cells[i] = 2;
            let score = minimax(cells, 1);
            cells[i] = 0;
            if score > value {
                value = score;
                best = Some(i);
            }
        }
    }
    best
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tic_tac_toe_win_draw_and_illegal_moves() {
        let mut g = Match::new(2, 15);
        for i in [0, 3, 1, 4, 2] {
            assert!(g.play(i));
        }
        assert!(g.finished);
        assert_eq!(g.winners(), vec![0]);
        g.record_result();
        g.record_result();
        assert_eq!(g.round_wins, [1, 0, 0, 0]);
        let mut restored: Match =
            serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        restored.record_result();
        assert_eq!(restored.round_wins, [1, 0, 0, 0]);
        assert!(!g.play(5));
        assert!(g.valid());
        let mut g = Match::new(2, 15);
        assert!(g.play(0));
        assert!(!g.play(0));
        for i in [1, 2, 4, 3, 5, 7, 6, 8] {
            assert!(g.play(i));
        }
        assert!(g.finished);
        assert_eq!(g.winners(), vec![0, 1]);
    }
    #[test]
    fn computer_never_loses_against_any_human_sequence() {
        fn explore(g: Match) {
            assert!(g.valid());
            if g.finished {
                assert_ne!(g.scores[0], 1);
                return;
            }
            for i in 0..9 {
                let mut next = g.clone();
                if next.play(i) {
                    if !next.finished {
                        assert!(!next.play((i + 1) % 9));
                        next.tick(0.5);
                    }
                    explore(next);
                }
            }
        }
        explore(Match::new(1, 15));
    }
}
