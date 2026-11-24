// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub humans: usize,
    pub count: usize,
    pub turn: usize,
    pub edges: Vec<i8>,
    pub boxes: Vec<i8>,
    pub scores: [u32; 4],
    pub delay: f64,
    pub last: Option<usize>,
    pub pulse: f64,
    pub finished: bool,
    pub seed: u64,
}
impl Game {
    pub fn new(humans: usize, seed: u64) -> Self {
        Self {
            humans: humans.clamp(1, 4),
            count: humans.clamp(2, 4),
            turn: 0,
            edges: vec![-1; 60],
            boxes: vec![-1; 25],
            scores: [0; 4],
            delay: 0.6,
            last: None,
            pulse: 1.0,
            finished: false,
            seed,
        }
    }
    pub fn valid(&self) -> bool {
        (1..=4).contains(&self.humans)
            && self.count == self.humans.max(2)
            && self.turn < self.count
            && self.edges.len() == 60
            && self.boxes.len() == 25
            && self
                .edges
                .iter()
                .chain(&self.boxes)
                .all(|&v| v >= -1 && v < self.count as i8)
            && self.delay.is_finite()
            && self.pulse.is_finite()
            && self.scores.iter().sum::<u32>() <= 25
    }
    pub fn boundaries(b: usize) -> [usize; 4] {
        let x = b % 5;
        let y = b / 5;
        [
            y * 5 + x,
            (y + 1) * 5 + x,
            30 + y * 6 + x,
            30 + y * 6 + x + 1,
        ]
    }
    pub fn play(&mut self, e: usize) -> Option<usize> {
        if self.finished || self.delay > 0.0 || e >= 60 || self.edges[e] >= 0 {
            return None;
        }
        self.edges[e] = self.turn as i8;
        self.last = Some(e);
        self.pulse = 0.0;
        let mut captured = 0;
        for b in 0..25 {
            if self.boxes[b] < 0 && Self::boundaries(b).iter().all(|&e| self.edges[e] >= 0) {
                self.boxes[b] = self.turn as i8;
                captured += 1;
            }
        }
        self.scores[self.turn] += captured as u32;
        self.finished = self.boxes.iter().all(|&b| b >= 0);
        if captured == 0 {
            self.turn = (self.turn + 1) % self.count;
        }
        self.delay = if captured > 0 { 0.4 } else { 0.6 };
        Some(captured)
    }
    fn random(&mut self, n: usize) -> usize {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed as usize % n
    }
    pub fn ai(&mut self) -> usize {
        let mut captures = Vec::new();
        let mut safe = Vec::new();
        let mut rest = Vec::new();
        for e in 0..60 {
            if self.edges[e] >= 0 {
                continue;
            }
            let mut gain = 0;
            let mut danger = false;
            for b in 0..25 {
                let edges = Self::boundaries(b);
                if !edges.contains(&e) {
                    continue;
                }
                let n = edges.iter().filter(|&&i| self.edges[i] >= 0).count();
                gain += usize::from(n == 3);
                danger |= n == 2;
            }
            if gain > 0 {
                captures.push((e, gain));
            } else if !danger {
                safe.push(e);
            } else {
                rest.push(e);
            }
        }
        if !captures.is_empty() {
            captures.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
            return captures[0].0;
        }
        if !safe.is_empty() {
            let i = self.random(safe.len());
            return safe[i];
        }
        // Sacrifice the smallest forced chain, rather than opening an arbitrary long chain.
        rest.into_iter()
            .min_by_key(|&e| {
                let mut g = self.clone();
                g.delay = 0.0;
                g.play(e);
                let mut cost = 0;
                loop {
                    let edge = (0..60).find(|&i| {
                        g.edges[i] < 0
                            && (0..25).any(|b| {
                                let es = Self::boundaries(b);
                                g.boxes[b] < 0
                                    && es.contains(&i)
                                    && es.iter().filter(|&&j| g.edges[j] >= 0).count() == 3
                            })
                    });
                    let Some(i) = edge else {
                        break;
                    };
                    g.delay = 0.0;
                    cost += g.play(i).unwrap_or(0);
                }
                cost
            })
            .unwrap_or(0)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captures_two_adjacent_boxes_and_keeps_turn() {
        let mut g = Game::new(2, 4);
        for e in [0, 1, 5, 6, 30, 32] {
            g.edges[e] = 1;
        }
        g.delay = 0.0;
        assert_eq!(g.play(31), Some(2));
        assert_eq!(g.scores, [2, 0, 0, 0]);
        assert_eq!(g.turn, 0);
        assert_eq!(g.play(31), None);
    }
    #[test]
    fn whole_board_has_exactly_25_points() {
        let mut g = Game::new(4, 7);
        for e in 0..60 {
            g.delay = 0.0;
            g.play(e);
        }
        assert!(g.finished);
        assert_eq!(g.scores.iter().sum::<u32>(), 25);
        assert!(g.valid());
    }
    #[test]
    fn computer_takes_available_box() {
        let mut g = Game::new(1, 8);
        for e in [0, 5, 30] {
            g.edges[e] = 0;
        }
        assert_eq!(g.ai(), 31);
    }
}
