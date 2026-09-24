// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! A reversible gesture preview. It never mutates the game or consumes RNG/moves.
use crate::model::{Game, N};
use day_pieces::prelude::Point;

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
    pub fn update(&mut self, game: &Game, point: Point, cell: f64, inside: bool) {
        if cell <= 0.0 {
            return;
        }
        let dx = (point.x - self.start.x) / cell;
        let dy = (point.y - self.start.y) / cell;
        let distance = dx.abs().max(dy.abs());
        self.moved |= distance > 0.12;
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
        self.preview.to = to.filter(|&to| {
            inside
                && (0.3..=1.65).contains(&distance)
                && Game::adjacent(from, to)
                && game.grid[to].is_some()
        });
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
        hold.update(&g, Point::new(145.0, 100.0), 50.0, true);
        assert_eq!(hold.release(), Some((24, 25)));
        assert_eq!(hold.preview.offset(24), Point::new(0.9, 0.0));
        assert_eq!(hold.preview.offset(25), Point::new(-0.9, 0.0));
        hold.update(&g, Point::new(100.0, 50.0), 50.0, true);
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
    fn returning_home_or_leaving_the_board_cancels() {
        let g = Game::new(0, 15);
        let mut hold = Hold::new(24, Point::ZERO);
        hold.update(&g, Point::new(50.0, 0.0), 50.0, true);
        hold.update(&g, Point::new(3.0, 2.0), 50.0, true);
        assert!(hold.release().is_none());
        assert!(hold.moved);
        hold.update(&g, Point::new(50.0, 0.0), 50.0, false);
        assert!(hold.release().is_none());
        hold.update(&g, Point::new(100.0, 0.0), 50.0, true);
        assert!(
            hold.release().is_none(),
            "only adjacent pieces can be swapped"
        );
    }
    #[test]
    fn holes_and_row_edges_are_not_preview_targets() {
        let g = Game::new(2, 15);
        let mut hold = Hold::new(24, Point::ZERO);
        hold.update(&g, Point::new(-50.0, 0.0), 50.0, true);
        assert!(hold.release().is_none(), "cell 23 is a hole");
        let mut hold = Hold::new(6, Point::ZERO);
        hold.update(&g, Point::new(50.0, 0.0), 50.0, true);
        assert!(hold.release().is_none(), "cannot wrap into the next row");
    }
    #[test]
    fn only_release_commits_and_spends_one_move() {
        let mut g = Game::new(0, 15);
        let before = g.grid.clone();
        let mut hold = Hold::new(46, Point::ZERO);
        hold.update(&g, Point::new(50.0, 0.0), 50.0, true);
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
        hold.update(&g, Point::new(50.0, 0.0), 50.0, true);
        let half = hold.preview.scaled(0.5);
        assert_eq!(half.offset(24), Point::new(0.5, 0.0));
        assert_eq!(half.offset(25), Point::new(-0.5, 0.0));
        assert_eq!(hold.preview.scaled(0.0).offset(24), Point::ZERO);
        assert_eq!(hold.preview.scaled(0.0).offset(25), Point::ZERO);
    }
}
