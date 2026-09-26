// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Deterministic match-N engine. Resolution is atomic; animation replays immutable snapshots.
use serde::{Deserialize, Serialize};
pub const N: usize = 7;
pub const LEN: usize = N * N;
pub const LEVELS: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Special {
    Plain,
    Row,
    Column,
    Wrapped,
    Rainbow,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candy {
    pub color: u8,
    pub special: Special,
}
impl Candy {
    pub fn plain(color: u8) -> Self {
        Self {
            color,
            special: Special::Plain,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Level {
    pub name: &'static str,
    pub detail: &'static str,
    pub moves: u32,
    pub target: u32,
    pub collect: u32,
    pub colors: u8,
}
pub const STAGES: [Level; LEVELS] = [
    Level {
        name: "mt_classic",
        detail: "mt_goal_score",
        moves: 22,
        target: 1200,
        collect: 0,
        colors: 5,
    },
    Level {
        name: "mt_corners",
        detail: "mt_goal_collect",
        moves: 26,
        target: 1600,
        collect: 18,
        colors: 5,
    },
    Level {
        name: "mt_bridges",
        detail: "mt_goal_frost",
        moves: 28,
        target: 1600,
        collect: 0,
        colors: 5,
    },
    Level {
        name: "mt_diamond",
        detail: "mt_goal_collect",
        moves: 28,
        target: 2200,
        collect: 24,
        colors: 6,
    },
    Level {
        name: "mt_windows",
        detail: "mt_goal_frost",
        moves: 32,
        target: 2600,
        collect: 0,
        colors: 5,
    },
    Level {
        name: "mt_mixed",
        detail: "mt_goal_final",
        moves: 34,
        target: 3200,
        collect: 28,
        colors: 6,
    },
];
pub fn active(level: usize, i: usize) -> bool {
    let (r, c) = (i / N, i % N);
    match level {
        1 => !((r == 0 || r == 6) && (c == 0 || c == 6)),
        2 => !(r == 3 && (c == 2 || c == 4)),
        3 => r.abs_diff(3) + c.abs_diff(3) <= 4,
        4 => !((r == 2 || r == 4) && (c == 2 || c == 4)),
        5 => !((r == 0 || r == 6) && (c == 0 || c == 6)) && !(r == 3 && c == 3),
        _ => true,
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub level: usize,
    pub grid: Vec<Option<Candy>>,
    pub frost: Vec<u8>,
    pub moves: u32,
    pub score: u32,
    pub collected: u32,
    pub rng: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Save {
    pub game: Game,
    pub stars: [u8; LEVELS],
}
impl Save {
    pub fn new(seed: u64) -> Self {
        Self {
            game: Game::new(0, seed),
            stars: [0; LEVELS],
        }
    }
    pub fn valid(self) -> Option<Self> {
        let g = &self.game;
        (g.level < LEVELS
            && g.grid.len() == LEN
            && g.frost.len() == LEN
            && g.rng != 0
            && g.moves <= STAGES[g.level].moves
            && g.score < 10_000_000
            && self.stars.iter().all(|s| *s <= 3)
            && (0..LEN).all(|i| {
                g.grid[i].is_some() == active(g.level, i)
                    && g.grid[i].is_none_or(|c| c.color < STAGES[g.level].colors)
                    && g.frost[i] <= 2
                    && (active(g.level, i) || g.frost[i] == 0)
            })
            && g.groups().is_empty())
        .then_some(self)
    }
    pub fn unlocked(&self, level: usize) -> bool {
        level == 0 || self.stars[level - 1] > 0
    }
    pub fn record(&mut self) {
        if self.game.won() {
            let i = self.game.level;
            self.stars[i] = self.stars[i].max(self.game.stars());
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Swap,
    Return,
    Burst,
    Fall,
    Shuffle,
}
#[derive(Clone)]
pub struct Frame {
    pub grid: Vec<Option<Candy>>,
    pub frost: Vec<u8>,
    pub phase: Phase,
    pub motion: Vec<(usize, usize)>,
    pub cleared: Vec<usize>,
    pub chain: u32,
    pub points: u32,
}
pub struct Turn {
    pub valid: bool,
    pub frames: Vec<Frame>,
}
impl Game {
    pub fn new(level: usize, seed: u64) -> Self {
        let mut g = Self {
            level: level.min(LEVELS - 1),
            grid: vec![None; LEN],
            frost: vec![0; LEN],
            moves: STAGES[level.min(LEVELS - 1)].moves,
            score: 0,
            collected: 0,
            rng: seed | 1,
        };
        for i in 0..LEN {
            if active(g.level, i)
                && matches!(g.level, 2 | 4 | 5)
                && (if g.level == 4 {
                    i / N % 2 == 1 && i % N % 2 == 1
                } else {
                    (i / N + i % N).is_multiple_of(2)
                })
            {
                g.frost[i] = if g.level == 4 { 2 } else { 1 };
            }
        }
        g.redeal();
        g
    }
    fn random(&mut self) -> u8 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % STAGES[self.level].colors as u64) as u8
    }
    fn redeal(&mut self) {
        loop {
            self.grid.fill(None);
            for i in 0..LEN {
                if active(self.level, i) {
                    let mut c = self.random();
                    loop {
                        let equal = |j: usize| self.grid[j].is_some_and(|x| x.color == c);
                        if !(i % N >= 2 && equal(i - 1) && equal(i - 2)
                            || i >= 2 * N && equal(i - N) && equal(i - 2 * N))
                        {
                            break;
                        }
                        c = (c + 1) % STAGES[self.level].colors;
                    }
                    self.grid[i] = Some(Candy::plain(c));
                }
            }
            if self.hint().is_some() {
                break;
            }
        }
    }
    pub fn frosting(&self) -> u32 {
        self.frost.iter().map(|x| *x as u32).sum()
    }
    pub fn won(&self) -> bool {
        self.score >= STAGES[self.level].target
            && self.collected >= STAGES[self.level].collect
            && self.frosting() == 0
    }
    pub fn over(&self) -> bool {
        self.won() || self.moves == 0
    }
    pub fn stars(&self) -> u8 {
        if !self.won() {
            0
        } else if self.moves >= 10 {
            3
        } else if self.moves >= 5 {
            2
        } else {
            1
        }
    }
    pub fn adjacent(a: usize, b: usize) -> bool {
        a < LEN && b < LEN && a / N == b / N && a.abs_diff(b) == 1
            || a < LEN && b < LEN && a.abs_diff(b) == N
    }
    fn groups(&self) -> Vec<Vec<usize>> {
        let mut runs = Vec::new();
        for vertical in [false, true] {
            for lane in 0..N {
                let mut p = 0;
                while p < N {
                    let i = if vertical { p * N + lane } else { lane * N + p };
                    let Some(c) = self.grid[i].filter(|c| c.special != Special::Rainbow) else {
                        p += 1;
                        continue;
                    };
                    let mut run = vec![i];
                    p += 1;
                    while p < N {
                        let j = if vertical { p * N + lane } else { lane * N + p };
                        if !self.grid[j]
                            .is_some_and(|x| x.color == c.color && x.special != Special::Rainbow)
                        {
                            break;
                        }
                        run.push(j);
                        p += 1;
                    }
                    if run.len() >= 3 {
                        runs.push(run);
                    }
                }
            }
        }
        runs
    }
    fn combo(&self, a: usize, b: usize) -> bool {
        let (Some(x), Some(y)) = (self.grid[a], self.grid[b]) else {
            return false;
        };
        x.special == Special::Rainbow
            || y.special == Special::Rainbow
            || x.special != Special::Plain && y.special != Special::Plain
    }
    pub fn hint(&self) -> Option<(usize, usize)> {
        let mut copy = self.clone();
        for a in 0..LEN {
            for b in [a + 1, a + N] {
                if !Self::adjacent(a, b) || self.grid[a].is_none() || self.grid[b].is_none() {
                    continue;
                }
                if self.combo(a, b) {
                    return Some((a, b));
                }
                copy.grid.swap(a, b);
                let yes = !copy.groups().is_empty();
                copy.grid.swap(a, b);
                if yes {
                    return Some((a, b));
                }
            }
        }
        None
    }
    fn frame(&self, phase: Phase, chain: u32) -> Frame {
        Frame {
            grid: self.grid.clone(),
            frost: self.frost.clone(),
            phase,
            motion: vec![],
            cleared: vec![],
            chain,
            points: 0,
        }
    }
    pub fn swap(&mut self, a: usize, b: usize) -> Turn {
        let mut out = Turn {
            valid: false,
            frames: vec![],
        };
        if self.over() || !Self::adjacent(a, b) || self.grid[a].is_none() || self.grid[b].is_none()
        {
            return out;
        }
        let combo = self.combo(a, b);
        self.grid.swap(a, b);
        let mut f = self.frame(Phase::Swap, 0);
        f.motion = vec![(a, b), (b, a)];
        out.frames.push(f);
        if !combo && self.groups().is_empty() {
            self.grid.swap(a, b);
            let mut f = self.frame(Phase::Return, 0);
            f.motion = vec![(a, b), (b, a)];
            out.frames.push(f);
            return out;
        }
        self.moves -= 1;
        out.valid = true;
        let mut forced = vec![];
        if combo {
            let (x, y) = (self.grid[a].unwrap(), self.grid[b].unwrap());
            forced.extend([a, b]);
            if x.special == Special::Rainbow || y.special == Special::Rainbow {
                let other = if x.special == Special::Rainbow { y } else { x };
                for i in 0..LEN {
                    if self.grid[i].is_some_and(|c| {
                        c.color == other.color || other.special == Special::Rainbow
                    }) {
                        forced.push(i);
                        if !matches!(other.special, Special::Plain | Special::Rainbow) {
                            self.grid[i].as_mut().unwrap().special = other.special;
                        }
                    }
                }
            } else {
                let radius = if x.special == Special::Wrapped || y.special == Special::Wrapped {
                    1
                } else {
                    0
                };
                for i in 0..LEN {
                    if (i / N).abs_diff(b / N) <= radius || (i % N).abs_diff(b % N) <= radius {
                        forced.push(i);
                    }
                }
            }
        }
        for chain in 1..=64 {
            let runs = self.groups();
            if runs.is_empty() && forced.is_empty() {
                break;
            }
            let mut hit = [false; LEN];
            for i in forced.drain(..) {
                hit[i] = true;
            }
            for run in &runs {
                for &i in run {
                    hit[i] = true;
                }
            }
            // Merge intersecting runs, so a T/L creates one wrapped candy, not two stripes.
            let mut components: Vec<Vec<usize>> = vec![];
            for run in &runs {
                let mut group = run.clone();
                let mut j = 0;
                while j < components.len() {
                    if components[j].iter().any(|i| group.contains(i)) {
                        for i in components.remove(j) {
                            if !group.contains(&i) {
                                group.push(i);
                            }
                        }
                        j = 0;
                    } else {
                        j += 1;
                    }
                }
                components.push(group);
            }
            let mut created = vec![];
            for group in components {
                let cross = runs
                    .iter()
                    .filter(|r| r.iter().any(|i| group.contains(i)))
                    .count()
                    > 1;
                let longest = runs
                    .iter()
                    .filter(|r| r.iter().any(|i| group.contains(i)))
                    .max_by_key(|r| r.len())
                    .unwrap();
                let special = if longest.len() >= 5 {
                    Special::Rainbow
                } else if cross {
                    Special::Wrapped
                } else if longest.len() == 4 {
                    if longest[0] / N == longest[1] / N {
                        Special::Row
                    } else {
                        Special::Column
                    }
                } else {
                    continue;
                };
                let at = if chain == 1 && group.contains(&b) {
                    b
                } else if chain == 1 && group.contains(&a) {
                    a
                } else {
                    group[group.len() / 2]
                };
                created.push((
                    at,
                    Candy {
                        color: self.grid[at].unwrap().color,
                        special,
                    },
                ));
            }
            // Queue special explosions; each existing special fires at most once.
            let mut fired = [false; LEN];
            loop {
                let next = (0..LEN).find(|&i| hit[i] && !fired[i] && self.grid[i].is_some());
                let Some(i) = next else { break };
                fired[i] = true;
                let c = self.grid[i].unwrap();
                for (j, hit) in hit.iter_mut().enumerate() {
                    let blast = match c.special {
                        Special::Plain => false,
                        Special::Row => j / N == i / N,
                        Special::Column => j % N == i % N,
                        Special::Wrapped => {
                            (j / N).abs_diff(i / N) <= 1 && (j % N).abs_diff(i % N) <= 1
                        }
                        Special::Rainbow => self.grid[j].is_some_and(|x| x.color == c.color),
                    };
                    if blast {
                        *hit = true;
                    }
                }
            }
            let mut f = self.frame(Phase::Burst, chain);
            for (i, &yes) in hit.iter().enumerate() {
                if yes && self.grid[i].is_some() {
                    f.cleared.push(i);
                    self.frost[i] = self.frost[i].saturating_sub(1);
                    if self.grid[i].unwrap().color == 0 {
                        self.collected += 1;
                    }
                    self.grid[i] = None;
                }
            }
            f.points = f.cleared.len() as u32 * 60 * chain.min(5);
            self.score += f.points;
            out.frames.push(f);
            for (i, c) in created {
                self.grid[i] = Some(c);
            }
            let motion = self.fall();
            let mut f = self.frame(Phase::Fall, chain);
            f.motion = motion;
            out.frames.push(f);
            if chain == 64 {
                self.redeal();
                out.frames.push(self.frame(Phase::Shuffle, chain));
            }
        }
        if self.hint().is_none() && !self.over() {
            self.redeal();
            out.frames.push(self.frame(Phase::Shuffle, 0));
        }
        out
    }
    /// Holes split each column into independent gravity wells; candies never cross a hole.
    fn fall(&mut self) -> Vec<(usize, usize)> {
        let mut motion = vec![];
        for col in 0..N {
            let mut row = 0;
            while row < N {
                if !active(self.level, row * N + col) {
                    row += 1;
                    continue;
                }
                let start = row;
                while row < N && active(self.level, row * N + col) {
                    row += 1;
                }
                let end = row;
                let old: Vec<_> = (start..end)
                    .rev()
                    .filter_map(|r| self.grid[r * N + col].map(|c| (r * N + col, c)))
                    .collect();
                for r in start..end {
                    self.grid[r * N + col] = None;
                }
                for (offset, r) in (start..end).rev().enumerate() {
                    let to = r * N + col;
                    if let Some(&(from, c)) = old.get(offset) {
                        self.grid[to] = Some(c);
                        motion.push((from, to));
                    } else {
                        let c = self.random();
                        self.grid[to] = Some(Candy::plain(c));
                        // Spawn every replacement above this gravity well, preserving the spacing.
                        motion.push((LEN + (end - start - old.len()), to));
                    }
                }
            }
        }
        motion
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn blank() -> Game {
        let mut g = Game::new(0, 15);
        g.grid = (0..LEN)
            .map(|i| Some(Candy::plain(((i / N * 2 + i % N) % 5) as u8)))
            .collect();
        g
    }
    #[test]
    fn generated_boards_are_stable_playable_and_valid() {
        for level in 0..LEVELS {
            for seed in 1..100 {
                let g = Game::new(level, seed);
                assert!(g.groups().is_empty());
                assert!(g.hint().is_some());
                assert!(
                    Save {
                        game: g,
                        stars: [0; LEVELS]
                    }
                    .valid()
                    .is_some()
                );
            }
        }
    }
    #[test]
    fn invalid_swap_restores_everything_and_costs_no_move() {
        let mut g = blank();
        let before = serde_json::to_string(&g).unwrap();
        let turn = g.swap(0, 1);
        assert!(!turn.valid);
        assert_eq!(turn.frames.len(), 2);
        assert_eq!(serde_json::to_string(&g).unwrap(), before);
        assert!(!g.swap(6, 7).valid);
    }
    #[test]
    fn four_five_and_cross_make_specials() {
        for (count, expected) in [(4, Special::Row), (5, Special::Rainbow)] {
            let mut g = blank();
            for c in 0..count {
                g.grid[2 * N + c] = Some(Candy::plain(0));
            }
            g.grid[2 * N + 1] = Some(Candy::plain(1));
            g.grid[N + 1] = Some(Candy::plain(0));
            let turn = g.swap(N + 1, 2 * N + 1);
            assert!(turn.valid);
            assert!(turn.frames.iter().any(|f| f.phase == Phase::Fall
                && f.grid.iter().flatten().any(|c| c.special == expected)));
        }
        let mut g = blank();
        for i in [17, 23, 24, 26, 31] {
            g.grid[i] = Some(Candy::plain(0));
        }
        g.grid[25] = Some(Candy::plain(1));
        let t = g.swap(26, 25);
        assert!(t.valid);
        assert!(t.frames.iter().any(|f| {
            f.phase == Phase::Fall
                && f.grid
                    .iter()
                    .flatten()
                    .any(|c| c.special == Special::Wrapped)
        }));
    }
    #[test]
    fn rainbow_combo_and_frosting_resolve_without_double_counting() {
        let mut g = blank();
        g.grid[0].as_mut().unwrap().special = Special::Rainbow;
        g.grid[1].as_mut().unwrap().special = Special::Rainbow;
        g.frost.fill(2);
        let t = g.swap(0, 1);
        assert!(t.valid);
        let burst = t.frames.iter().find(|f| f.phase == Phase::Burst).unwrap();
        assert_eq!(burst.cleared.len(), LEN);
        assert_eq!(burst.points, LEN as u32 * 60);
        assert!(g.frost.iter().all(|&x| x <= 1));
        assert_eq!(g.moves, 21);
    }
    #[test]
    fn seeded_play_survives_roundtrip_and_preserves_holes() {
        for level in 0..LEVELS {
            let mut g = Game::new(level, 15);
            for _ in 0..34 {
                if g.over() {
                    break;
                }
                let (a, b) = g.hint().unwrap();
                assert!(g.swap(a, b).valid);
                assert!(g.groups().is_empty());
                for i in 0..LEN {
                    assert_eq!(g.grid[i].is_some(), active(level, i));
                }
                let s = Save {
                    game: g.clone(),
                    stars: [0; LEVELS],
                };
                assert!(
                    serde_json::from_str::<Save>(&serde_json::to_string(&s).unwrap())
                        .unwrap()
                        .valid()
                        .is_some()
                );
            }
        }
    }
    #[test]
    fn win_on_last_move_precedes_failure_and_unlocks() {
        let mut s = Save::new(15);
        s.game.score = STAGES[0].target;
        s.game.moves = 0;
        assert!(s.game.won());
        s.record();
        assert_eq!(s.stars[0], 1);
        assert!(s.unlocked(1));
        assert!(!s.unlocked(2));
    }
    #[test]
    fn malformed_save_rejected() {
        let mut s = Save::new(15);
        s.game.grid.pop();
        assert!(s.valid().is_none());
        let mut s = Save::new(15);
        s.game.level = 99;
        assert!(s.valid().is_none());
    }
    #[test]
    fn holes_split_gravity_wells() {
        let mut g = Game::new(2, 15);
        g.grid[2] = None;
        g.grid[4 * N + 2] = None;
        let motion = g.fall();
        for (from, to) in motion {
            if from < LEN {
                assert_eq!(from % N, to % N);
                assert!((from..=to).step_by(N).all(|i| active(g.level, i)));
            }
        }
    }
    #[test]
    fn stripes_and_wrapped_combinations_chain() {
        for special in [Special::Row, Special::Column, Special::Wrapped] {
            let mut g = blank();
            g.grid[23].as_mut().unwrap().special = special;
            g.grid[24].as_mut().unwrap().special = Special::Row;
            let t = g.swap(23, 24);
            assert!(t.valid);
            let burst = t.frames.iter().find(|f| f.phase == Phase::Burst).unwrap();
            assert!(burst.cleared.contains(&3));
            assert!(burst.cleared.contains(&21));
            if special == Special::Wrapped {
                assert!(burst.cleared.contains(&16));
                assert!(burst.cleared.contains(&32));
            }
        }
    }
    #[test]
    fn deterministic_levels_are_winnable() {
        // A greedy objective-aware player proves every authored stage can be completed.
        // The printed moves are also the source of the native DayScript level fixture.
        for (level, stage) in STAGES.iter().enumerate() {
            let mut g = Game::new(level, 15);
            let mut moves = vec![];
            while !g.over() {
                let mut best = None;
                for a in 0..LEN {
                    for b in [a + 1, a + N] {
                        if !Game::adjacent(a, b) {
                            continue;
                        }
                        let mut next = g.clone();
                        let turn = next.swap(a, b);
                        if !turn.valid {
                            continue;
                        }
                        let gain = (g.frosting() - next.frosting()) as u64 * 10000
                            + (next.collected.min(stage.collect) - g.collected.min(stage.collect))
                                as u64
                                * 2000
                            + (next.score - g.score) as u64;
                        if best.as_ref().is_none_or(
                            |(score, _, _, _): &(u64, Game, usize, usize)| gain > *score,
                        ) {
                            best = Some((gain, next, a, b));
                        }
                    }
                }
                let (_, next, a, b) = best.expect("a playable board");
                g = next;
                moves.push((a, b, g.score, g.moves));
            }
            println!("LEVEL {level}: {moves:?}");
            assert!(
                g.won(),
                "stage {level}: score {}, hearts {}, frost {}",
                g.score,
                g.collected,
                g.frosting()
            );
        }
    }
}
