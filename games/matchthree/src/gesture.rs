// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! A reversible gesture preview. It never mutates the game or consumes RNG/moves.
use crate::model::{Game, N};
use day_pieces::prelude::Point;

/// How far, in cells, a finger must slide before its gesture aims at a neighbour. Deliberately
/// small: any slide left, right, up or down is a swap attempt, and only a finger that comes back
/// to (or never really left) its starting cell cancels. A native pan recognizer has already
/// required its own touch slop before the gesture starts, so this is not what tells a tap from a
/// slide.
pub const AIM: f64 = 0.15;
/// Past this the finger counts as having moved, so the release cannot become a tap.
const MOVED: f64 = 0.12;

#[derive(Clone, Copy, Debug)]
pub struct Preview {
    pub from: usize,
    pub to: Option<usize>,
    /// Finger displacement measured in cells, independent of the canvas size.
    pub delta: Point,
    pub amount: f64,
}
impl Preview {
    pub fn offset(self, i: usize) -> Point {
        if i == self.from {
            return self.delta;
        }
        if self.to == Some(i) {
            return Point::new(
                ((self.from % N) as f64 - (i % N) as f64) * self.amount,
                ((self.from / N) as f64 - (i / N) as f64) * self.amount,
            );
        }
        Point::ZERO
    }
    pub fn scaled(mut self, scale: f64) -> Self {
        self.delta = Point::new(self.delta.x * scale, self.delta.y * scale);
        self.amount *= scale;
        self
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Hold {
    pub start: Point,
    pub preview: Preview,
    /// Remains set when a finger returns home, so its release cannot turn into a tap.
    pub moved: bool,
}
impl Hold {
    pub fn new(from: usize, start: Point) -> Self {
        Self {
            start,
            preview: Preview {
                from,
                to: None,
                delta: Point::ZERO,
                amount: 0.0,
            },
            moved: false,
        }
    }
    /// Follow the finger. The direction it has moved furthest in picks the neighbour, however far
    /// it goes and wherever it goes, off the board included: a long, fast flick is as good a
    /// swipe as a careful one.
    pub fn update(&mut self, game: &Game, point: Point, cell: f64) {
        if cell <= 0.0 {
            return;
        }
        let dx = (point.x - self.start.x) / cell;
        let dy = (point.y - self.start.y) / cell;
        let distance = dx.abs().max(dy.abs());
        self.moved |= distance > MOVED;
        self.preview.delta = Point::new(dx.clamp(-1.15, 1.15), dy.clamp(-1.15, 1.15));
        self.preview.amount = distance.min(1.0);
        let from = self.preview.from;
        let to = if dx.abs() > dy.abs() {
            if dx > 0.0 {
                from.checked_add(1)
            } else {
                from.checked_sub(1)
            }
        } else if dy > 0.0 {
            from.checked_add(N)
        } else {
            from.checked_sub(N)
        };
        self.preview.to =
            to.filter(|&to| distance >= AIM && Game::adjacent(from, to) && game.grid[to].is_some());
    }
    pub fn release(self) -> Option<(usize, usize)> {
        self.preview.to.map(|to| (self.preview.from, to))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exploring_neighbors_never_changes_the_board_score_moves_or_rng() {
        let g = Game::new(0, 15);
        let before = serde_json::to_string(&g).unwrap();
        let mut hold = Hold::new(24, Point::new(100.0, 100.0));
        hold.update(&g, Point::new(145.0, 100.0), 50.0);
        assert_eq!(hold.release(), Some((24, 25)));
        assert_eq!(hold.preview.offset(24), Point::new(0.9, 0.0));
        assert_eq!(hold.preview.offset(25), Point::new(-0.9, 0.0));
        hold.update(&g, Point::new(100.0, 50.0), 50.0);
        assert_eq!(hold.release(), Some((24, 17)));
        assert_eq!(
            hold.preview.offset(25),
            Point::ZERO,
            "the previous neighbor returns home"
        );
        assert_eq!(hold.preview.offset(17), Point::new(0.0, 1.0));
        assert_eq!(serde_json::to_string(&g).unwrap(), before);
    }
    #[test]
    fn only_returning_home_cancels() {
        let g = Game::new(0, 15);
        let mut hold = Hold::new(24, Point::ZERO);
        hold.update(&g, Point::new(50.0, 0.0), 50.0);
        hold.update(&g, Point::new(3.0, 2.0), 50.0);
        assert!(hold.release().is_none(), "back at the start");
        assert!(hold.moved);
        // A long flick, even one that carries the finger far off the board, still swaps with
        // the neighbour on that side, never with anything further away.
        hold.update(&g, Point::new(400.0, 30.0), 50.0);
        assert_eq!(hold.release(), Some((24, 25)));
        hold.update(&g, Point::new(-20.0, -900.0), 50.0);
        assert_eq!(hold.release(), Some((24, 17)));
    }

    /// Any small slide in any of the four directions is a swap attempt with that neighbour,
    /// including a sloppy diagonal-ish one: the axis moved furthest along decides.
    #[test]
    fn any_short_slide_aims_at_the_neighbour_on_that_side() {
        let g = Game::new(0, 15);
        let cell = 50.0;
        let nudge = cell * (AIM + 0.02);
        for (dx, dy, to) in [
            (nudge, 0.0, 25),
            (-nudge, 0.0, 23),
            (0.0, nudge, 31),
            (0.0, -nudge, 17),
            (nudge, nudge * 0.8, 25),
            (-nudge * 0.6, -nudge, 17),
        ] {
            let mut hold = Hold::new(24, Point::ZERO);
            hold.update(&g, Point::new(dx, dy), cell);
            assert_eq!(hold.release(), Some((24, to)), "slide ({dx}, {dy})");
        }
        let mut hold = Hold::new(24, Point::ZERO);
        hold.update(&g, Point::new(cell * AIM * 0.5, 0.0), cell);
        assert!(
            hold.release().is_none(),
            "a wobble under the threshold is not a swipe"
        );
    }
    #[test]
    fn holes_and_row_edges_are_not_preview_targets() {
        let g = Game::new(2, 15);
        let mut hold = Hold::new(24, Point::ZERO);
        hold.update(&g, Point::new(-50.0, 0.0), 50.0);
        assert!(hold.release().is_none(), "cell 23 is a hole");
        let mut hold = Hold::new(6, Point::ZERO);
        hold.update(&g, Point::new(50.0, 0.0), 50.0);
        assert!(hold.release().is_none(), "cannot wrap into the next row");
    }
    #[test]
    fn only_release_commits_and_spends_one_move() {
        let mut g = Game::new(0, 15);
        let before = g.grid.clone();
        let mut hold = Hold::new(46, Point::ZERO);
        hold.update(&g, Point::new(50.0, 0.0), 50.0);
        assert_eq!(g.grid, before);
        assert_eq!(g.moves, 22);
        assert_eq!(g.score, 0);
        let (a, b) = hold.release().unwrap();
        assert!(g.swap(a, b).valid);
        assert_eq!(g.moves, 21);
        assert_eq!(g.score, 12540);
    }
    #[test]
    fn cancelled_preview_settles_both_pieces_home() {
        let g = Game::new(0, 15);
        let mut hold = Hold::new(24, Point::ZERO);
        hold.update(&g, Point::new(50.0, 0.0), 50.0);
        let half = hold.preview.scaled(0.5);
        assert_eq!(half.offset(24), Point::new(0.5, 0.0));
        assert_eq!(half.offset(25), Point::new(-0.5, 0.0));
        assert_eq!(hold.preview.scaled(0.0).offset(24), Point::ZERO);
        assert_eq!(hold.preview.scaled(0.0).offset(25), Point::ZERO);
    }
}
