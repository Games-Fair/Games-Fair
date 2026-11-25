// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use super::{engine::Search, model::*};
use cozy_chess::{Board, Color, Move, Piece, Square, util};

fn fixture(fen: &str) -> Game {
    let board: Board = fen.parse().expect("test position");
    let mut game = Game::new(2, Pace::Untimed);
    game.quiet_plies = u16::from(board.halfmove_clock());
    game.previous = vec![board.clone()];
    game.board = board;
    game
}
fn mv(g: &Game, uci: &str) -> Move {
    util::parse_uci_move(&g.board, uci).expect("test move")
}
fn play(g: &mut Game, uci: &str) {
    assert!(g.play(mv(g, uci)), "{uci} in {}", g.board);
}
fn count(b: &Board, depth: u8) -> u64 {
    let moves = legal(b);
    if depth == 1 {
        return moves.len() as u64;
    }
    moves
        .into_iter()
        .map(|m| {
            let mut next = b.clone();
            next.play_unchecked(m);
            count(&next, depth - 1)
        })
        .sum()
}
#[test]
fn reference_move_trees() {
    assert_eq!(count(&Board::default(), 4), 197281);
    let g = fixture("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    assert_eq!(count(&g.board, 3), 97862);
}
#[test]
fn castle_both_wings_and_never_through_check_or_after_rook_returns() {
    let mut g = fixture("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    play(&mut g, "e1g1");
    assert_eq!(g.last, Some((Square::E1, Square::G1)));
    assert_eq!(g.board.piece_on(Square::F1), Some(Piece::Rook));
    play(&mut g, "e8c8");
    assert_eq!(g.notation, "O-O-O");
    assert_eq!(g.board.piece_on(Square::D8), Some(Piece::Rook));
    let mut g = fixture("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    for m in ["h1h2", "h8h7", "h2h1", "h7h8"] {
        play(&mut g, m);
    }
    assert!(!g.play(mv(&g, "e1g1")));
    let mut g = fixture("4kr2/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    assert!(!g.play(mv(&g, "e1g1")));
}
#[test]
fn en_passant_capture_must_be_immediate_and_leave_king_safe() {
    let fen = "k7/8/8/3pP3/8/8/8/4K3 w - d6 0 1";
    let mut g = fixture(fen);
    let m = mv(&g, "e5d6");
    assert_eq!(captured(&g.board, m), Some(Piece::Pawn));
    play(&mut g, "e5d6");
    assert_eq!(g.board.piece_on(Square::D5), None);
    let mut g = fixture(fen);
    play(&mut g, "e1f1");
    play(&mut g, "a8b8");
    assert!(!g.play(mv(&g, "e5d6")));
    let mut g = fixture("k3r3/8/8/3pP3/8/8/8/4K3 w - d6 0 1");
    assert!(!g.play(mv(&g, "e5d6")));
}
#[test]
fn promotions_include_all_four_choices() {
    for (letter, p) in [
        ('q', Piece::Queen),
        ('r', Piece::Rook),
        ('b', Piece::Bishop),
        ('n', Piece::Knight),
    ] {
        let mut g = fixture("7k/P7/8/8/8/8/8/4K3 w - - 0 1");
        play(&mut g, &format!("a7a8{letter}"));
        assert_eq!(g.board.piece_on(Square::A8), Some(p));
    }
}
#[test]
fn claim_before_or_after_move_and_automatic_fivefold() {
    let mut g = Game::new(2, Pace::Untimed);
    let cycle = ["g1f3", "g8f6", "f3g1", "f6g8"];
    for &m in cycle.iter().cycle().take(7) {
        play(&mut g, m);
    }
    assert_eq!(g.claim(), None);
    assert_eq!(g.intended(mv(&g, "f6g8")), Some(Reason::Repetition));
    play(&mut g, "f6g8");
    assert_eq!(g.claim(), Some(Reason::Repetition));
    assert!(g.saved.end.is_none());
    let data = serde_json::to_string(&g.saved).expect("serialize");
    g = Game::restore(serde_json::from_str(&data).expect("deserialize"), 2).expect("restore");
    assert_eq!(g.claim(), Some(Reason::Repetition));
    for &m in cycle.iter().cycle().take(8) {
        play(&mut g, m);
    }
    assert_eq!(
        g.saved.end,
        Some(End {
            winner: None,
            reason: Reason::Fivefold
        })
    );
}
#[test]
fn fifty_is_claimable_seventy_five_automatic_and_mate_has_priority() {
    let mut g = fixture("4k3/8/8/8/8/8/P7/R3K3 w - - 99 1");
    assert_eq!(g.intended(mv(&g, "a2a3")), None);
    assert_eq!(g.intended(mv(&g, "a1b1")), Some(Reason::Fifty));
    play(&mut g, "a1b1");
    assert_eq!(g.claim(), Some(Reason::Fifty));
    assert!(g.saved.end.is_none());
    g.quiet_plies = 149;
    play(&mut g, "e8e7");
    assert_eq!(g.saved.end.expect("draw").reason, Reason::SeventyFive);
    let mut g = fixture("7k/8/5KQ1/8/8/8/8/8 w - - 0 1");
    g.quiet_plies = 149;
    play(&mut g, "g6g7");
    assert_eq!(
        g.saved.end,
        Some(End {
            winner: Some(0),
            reason: Reason::Mate
        })
    );
    let mut g = fixture("7k/8/5KQ1/8/8/8/8/8 w - - 0 1");
    play(&mut g, "g6f7");
    assert_eq!(
        g.saved.end,
        Some(End {
            winner: None,
            reason: Reason::Stalemate
        })
    );
}
#[test]
fn flag_fall_and_mating_material() {
    let mut g = Game::new(2, Pace::Rapid);
    g.elapse(11.5);
    play(&mut g, "e2e4");
    assert_eq!(g.saved.remaining, [593.5, 600.]);
    g.elapse(601.);
    assert_eq!(
        g.saved.end,
        Some(End {
            winner: Some(0),
            reason: Reason::Flag
        })
    );
    for (row, draw) in [
        ("4K3", true),
        ("2B1K3", true),
        ("1N2K3", true),
        ("1N2K1N1", false),
        ("2B1KB2", false),
    ] {
        let mut g = fixture(&format!("4k3/8/8/8/8/8/8/{row} b - - 0 1"));
        g.lose(Reason::Flag);
        assert_eq!(g.saved.end.expect("timeout").winner.is_none(), draw);
    }
    let mut g = fixture("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1");
    play(&mut g, "c1d2");
    assert_eq!(g.saved.end.expect("draw").reason, Reason::Dead);
    let mut g = Game::new(2, Pace::Untimed);
    g.elapse(99999.);
    assert!(g.saved.end.is_none());
}
#[test]
fn pawn_wall_cannot_be_called_dead_if_a_king_can_break_in() {
    let mut wall = fixture("k7/8/8/p1p1p1p1/P1P1P1P1/8/8/7K w - - 0 1");
    play(&mut wall, "h1g1");
    assert_eq!(wall.saved.end.expect("locked wall").reason, Reason::Dead);
    let mut gap = fixture("k7/8/8/p1p5/P1P5/8/8/7K w - - 0 1");
    play(&mut gap, "h1g1");
    assert!(gap.saved.end.is_none());
}
fn choose(g: &Game) -> Move {
    let history = super::engine::prior_hashes(&g.previous);
    let mut s = Search::new(&g.board, &history).expect("legal moves");
    s.analyse(1000, 110_000);
    s.best().mv
}
#[test]
fn computer_finds_mate_and_wins_undefended_queen() {
    let mut g = fixture("7k/8/5KQ1/8/8/8/8/8 w - - 0 1");
    let m = choose(&g);
    assert!(g.play(m));
    assert_eq!(g.saved.end.expect("mate").reason, Reason::Mate);
    let g = fixture("4k3/8/8/3q4/8/2N5/8/4K3 w - - 0 1");
    let m = choose(&g);
    assert_eq!(m.to, Square::D5);
    assert_eq!(g.board.side_to_move(), Color::White);
}

#[test]
fn timeout_with_locked_bishops_depends_on_which_side_can_ever_mate() {
    // Both sides have pawns and a bishop, yet only White can ever mate.
    let mut g = fixture("5b2/p7/Pp3k2/1Pp1pBp1/2P1P1P1/5K2/8/8 w - - 0 1");
    g.lose(Reason::Flag);
    assert_eq!(
        g.saved.end,
        Some(End {
            winner: None,
            reason: Reason::Flag
        })
    );
}

#[test]
fn computer_finds_the_bishop_check_that_forces_mate() {
    // From a comparison game. Bf5+ forces a reply and then mate in one.
    let mut g = fixture("6k1/pp2pp1p/4b1p1/8/1nP5/1P2b3/r5PP/RK3B1R b - - 1 19");
    let m = choose(&g);
    assert_eq!(m, mv(&g, "e6f5"));
    assert!(g.play(m));
    for reply in legal(&g.board) {
        let mut b = g.board.clone();
        b.play_unchecked(reply);
        assert!(legal(&b).into_iter().any(|m| {
            let mut next = b.clone();
            next.play_unchecked(m);
            !next.checkers().is_empty() && !next.generate_moves(|_| true)
        }));
    }
}

#[test]
fn computer_prefers_mate_to_a_draw_even_on_its_last_clock_tick() {
    let g = fixture("7k/8/5KQ1/8/8/8/8/8 w - - 99 1");
    let choice = Search::new(&g.board, &[]).expect("legal moves").best();
    assert_eq!(choice.mv, mv(&g, "g6g7"));
    assert!(choice.score > 0);
}
