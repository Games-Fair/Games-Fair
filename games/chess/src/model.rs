// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use cozy_chess::{Board, Color, File, Move, Piece, Square, util};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pace {
    Untimed,
    Blitz,
    Rapid,
}
impl Pace {
    pub fn initial(self) -> f64 {
        match self {
            Self::Untimed => 0.,
            Self::Blitz => 300.,
            Self::Rapid => 600.,
        }
    }
    fn increment(self) -> f64 {
        if self == Self::Rapid { 5. } else { 0. }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reason {
    Mate,
    Stalemate,
    Dead,
    Repetition,
    Fifty,
    Fivefold,
    SeventyFive,
    Flag,
    Resigned,
    Agreement,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct End {
    pub winner: Option<usize>,
    pub reason: Reason,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Saved {
    pub players: usize,
    pub pace: Pace,
    pub remaining: [f64; 2],
    pub moves: Vec<String>,
    pub end: Option<End>,
}
pub struct Game {
    pub board: Board,
    pub saved: Saved,
    pub previous: Vec<Board>,
    pub quiet_plies: u16,
    pub last: Option<(Square, Square)>,
    pub notation: String,
    pub taken: [Vec<Piece>; 2],
}

pub fn legal(b: &Board) -> Vec<Move> {
    let mut v = Vec::with_capacity(40);
    b.generate_moves(|moves| {
        v.extend(moves);
        false
    });
    v
}
pub fn destination(b: &Board, m: Move) -> Square {
    if b.color_on(m.to) == Some(b.side_to_move()) {
        Square::new(
            if m.to.file() == File::H {
                File::G
            } else {
                File::C
            },
            m.from.rank(),
        )
    } else {
        m.to
    }
}
pub fn captured(b: &Board, m: Move) -> Option<Piece> {
    if b.color_on(m.to) == Some(!b.side_to_move()) {
        b.piece_on(m.to)
    } else if b.piece_on(m.from) == Some(Piece::Pawn) && m.from.file() != m.to.file() {
        Some(Piece::Pawn)
    } else {
        None
    }
}
// Use the same position, including castling and en passant rights, for the
// helpmate solver. Unlike a playing engine, it considers *cooperative* play.
fn cannot_mate(board: &chess::Board, color: Color) -> bool {
    let side = if color == Color::White {
        chess::Color::White
    } else {
        chess::Color::Black
    };
    prove_unwinnable(board, |pos| match chasolver::winnability(pos, side) {
        Some(chasolver::Winnability::Unwinnable) => Some(true),
        Some(chasolver::Winnability::Winnable { .. }) => Some(false),
        None if chasolver::is_unwinnable_fast(pos, side) => Some(true),
        None => None,
    })
}

// A solver budget is not a chess rule. If a composed position exhausts its
// search, split it into continuations. A single reachable helpmate keeps the
// game alive. Only a closed set of proven-drawn branches and cycles is dead.
// Board equality includes the mover, castling and en passant rights.
fn prove_unwinnable(board: &chess::Board, prove: impl Fn(&chess::Board) -> Option<bool>) -> bool {
    if let Some(answer) = prove(board) {
        return answer;
    }
    let mut pending = vec![*board];
    let mut seen = std::collections::HashSet::from([*board]);
    while let Some(pos) = pending.pop() {
        for mv in chess::MoveGen::new_legal(&pos) {
            let next = pos.make_move_new(mv);
            if !seen.insert(next) {
                continue;
            }
            match prove(&next) {
                Some(false) => return false,
                Some(true) => {}
                None => pending.push(next),
            }
        }
    }
    true
}
fn adjudication_board(board: &Board) -> chess::Board {
    board.to_string().parse().expect("standard legal position")
}
fn dead(board: &Board) -> bool {
    let position = adjudication_board(board);
    cannot_mate(&position, board.side_to_move()) && cannot_mate(&position, !board.side_to_move())
}
impl Game {
    pub fn new(players: usize, pace: Pace) -> Self {
        let board = Board::default();
        Self {
            previous: vec![board.clone()],
            board,
            saved: Saved {
                players,
                pace,
                remaining: [pace.initial(); 2],
                moves: Vec::new(),
                end: None,
            },
            quiet_plies: 0,
            last: None,
            notation: String::new(),
            taken: Default::default(),
        }
    }
    // Replay the same legal moves used during play to restore castling rights,
    // en passant, repetition history and the clock-independent halfmove counter.
    pub fn restore(s: Saved, players: usize) -> Option<Self> {
        if s.players != players || s.remaining.iter().any(|x| !x.is_finite() || *x < 0.) {
            return None;
        }
        let mut g = Self::new(players, s.pace);
        for m in &s.moves {
            let mv = util::parse_uci_move(&g.board, m).ok()?;
            if !g.play(mv) {
                return None;
            }
        }
        if let Some(end) = s.end {
            if end.winner.is_some_and(|x| x > 1) || g.saved.end.is_some_and(|x| x != end) {
                return None;
            }
            g.saved.end = Some(end);
        }
        g.saved.remaining = s.remaining;
        Some(g)
    }
    pub fn end(&mut self, reason: Reason, winner: Option<Color>) {
        self.saved.end = Some(End {
            winner: winner.map(|s| s as usize),
            reason,
        });
    }
    pub fn lose(&mut self, reason: Reason) {
        let other = !self.board.side_to_move();
        self.end(
            reason,
            (!cannot_mate(&adjudication_board(&self.board), other)).then_some(other),
        )
    }
    pub fn elapse(&mut self, seconds: f64) {
        if self.saved.end.is_some() || self.saved.pace == Pace::Untimed {
            return;
        }
        let t = &mut self.saved.remaining[self.board.side_to_move() as usize];
        *t = (*t - seconds.max(0.)).max(0.);
        if *t == 0. {
            self.lose(Reason::Flag);
        }
    }
    pub fn claim(&self) -> Option<Reason> {
        if self.quiet_plies >= 100 {
            Some(Reason::Fifty)
        } else if self
            .previous
            .iter()
            .filter(|b| b.same_position(&self.board))
            .take(3)
            .count()
            == 3
        {
            Some(Reason::Repetition)
        } else {
            None
        }
    }
    pub fn intended(&self, m: Move) -> Option<Reason> {
        if self.board.piece_on(m.from) == Some(Piece::Pawn) || captured(&self.board, m).is_some() {
            return None;
        }
        if self.quiet_plies >= 99 {
            return Some(Reason::Fifty);
        }
        let mut b = self.board.clone();
        b.play_unchecked(m);
        (self
            .previous
            .iter()
            .filter(|p| p.same_position(&b))
            .take(2)
            .count()
            == 2)
            .then_some(Reason::Repetition)
    }
    pub fn play(&mut self, m: Move) -> bool {
        if self.saved.end.is_some() || !self.board.is_legal(m) {
            return false;
        }
        let who = self.board.side_to_move();
        if let Some(p) = captured(&self.board, m) {
            let tray = &mut self.taken[who as usize];
            tray.insert(tray.partition_point(|&q| q <= p), p);
        }
        self.notation = util::display_san_move(&self.board, m).to_string();
        self.saved
            .moves
            .push(util::display_uci_move(&self.board, m).to_string());
        self.last = Some((m.from, destination(&self.board, m)));
        if self.board.piece_on(m.from) == Some(Piece::Pawn) || captured(&self.board, m).is_some() {
            self.quiet_plies = 0;
            self.previous.clear();
        } else {
            self.quiet_plies += 1;
        }
        self.board.play_unchecked(m);
        self.previous.push(self.board.clone());
        self.saved.remaining[who as usize] += self.saved.pace.increment();
        let checked = !self.board.checkers().is_empty();
        if !self.board.generate_moves(|_| true) {
            self.end(
                if checked {
                    Reason::Mate
                } else {
                    Reason::Stalemate
                },
                checked.then_some(who),
            );
        } else if dead(&self.board) {
            self.end(Reason::Dead, None);
        } else if self.quiet_plies >= 150 {
            self.end(Reason::SeventyFive, None);
        } else if self
            .previous
            .iter()
            .filter(|b| b.same_position(&self.board))
            .take(5)
            .count()
            == 5
        {
            self.end(Reason::Fivefold, None);
        }
        true
    }
}

#[cfg(test)]
mod adjudication_tests {
    use super::*;
    #[test]
    fn inconclusive_root_checks_continuations_before_deciding_a_timeout() {
        let pos: chess::Board = "5b2/p7/Pp3k2/1Pp1pBp1/2P1P1P1/5K2/8/8 w - - 0 1"
            .parse()
            .expect("blocked test position");
        for color in Color::ALL {
            let result = prove_unwinnable(&pos, |b| {
                if *b == pos {
                    None
                } else {
                    Some(cannot_mate(b, color))
                }
            });
            assert_eq!(result, color == Color::Black);
        }
    }
}
