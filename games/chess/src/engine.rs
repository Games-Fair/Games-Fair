// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! The offline computer player. Open Chess Analysis searches the same cozy-chess
//! position the game adjudicates, including its castling and en passant rights.
use crate::model::legal;
use cozy_chess::{Board, Move};
use open_chess_analysis::{Limits, Score, Searcher};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub use open_chess_analysis::eval::evaluate;

const TABLE_ENTRIES: usize = 1 << 15;
const MAX_DEPTH: u8 = 18;
const MAX_NODES: u64 = 600_000;

fn millis() -> u64 {
    (day_core::frame::now_secs() * 1000.) as u64
}

// The game stores the current position as the last entry. The searcher
// adds the root itself and needs only positions preceding it.
pub fn prior_hashes(previous: &[Board]) -> Vec<u64> {
    previous.split_last().map_or_else(Vec::new, |(_, prior)| {
        prior.iter().map(Board::hash).collect()
    })
}

#[derive(Clone, Copy)]
pub struct Choice {
    pub mv: Move,
    // Centipawns from the computer's point of view (includes forced mates).
    pub score: i32,
}

pub(super) struct Search<'a> {
    root: &'a Board,
    history: &'a [u64],
    engine: Searcher,
    best: Choice,
    depth: u8,
    remaining_nodes: u64,
}

impl<'a> Search<'a> {
    pub fn new(root: &'a Board, history: &'a [u64]) -> Option<Self> {
        // If the clock is about to fall, even our zero-depth fallback should
        // develop a piece or take free material instead of choosing by square order.
        let moves = legal(root);
        let only_move = moves.len() == 1;
        let best = moves
            .into_iter()
            .map(|mv| {
                let mut next = root.clone();
                next.play_unchecked(mv);
                // Mate on this move also takes precedence at a draw-claim threshold.
                let score = if !next.checkers().is_empty() && !next.generate_moves(|_| true) {
                    open_chess_analysis::search::MATE
                } else {
                    -evaluate(&next)
                };
                Choice { mv, score }
            })
            .max_by_key(|c| c.score)?;
        let mut engine = Searcher::new(TABLE_ENTRIES);
        engine.set_clock(millis);
        // On-demand tablebase generation can take longer than an entire turn.
        engine.use_tablebases = false;
        Some(Self {
            root,
            history,
            engine,
            best,
            depth: 0,
            remaining_nodes: if only_move || best.score == open_chess_analysis::search::MATE {
                0
            } else {
                MAX_NODES
            },
        })
    }

    pub fn best(&self) -> Choice {
        self.best
    }

    /// Reuse the transposition table across slices. An interrupted or shallower
    /// iteration can never displace an answer from a deeper, completed search.
    pub fn analyse(&mut self, milliseconds: u64, node_budget: u64) -> bool {
        if self.remaining_nodes == 0 {
            return true;
        }
        let result = self.engine.search(
            self.root,
            self.history,
            Limits {
                depth: MAX_DEPTH,
                nodes: self.remaining_nodes.min(node_budget),
                multipv: 1,
                time_ms: Some(milliseconds),
            },
        );
        self.remaining_nodes = self.remaining_nodes.saturating_sub(result.nodes);
        if let Some(line) = result.lines.first()
            && let Some(&m) = line.moves.first()
            && result.depth >= self.depth
        {
            self.best = Choice {
                mv: m,
                score: line.score.as_cp(),
            };
            self.depth = result.depth;
            if matches!(line.score, Score::Mate { .. }) {
                return true;
            }
        }
        self.remaining_nodes == 0 || self.depth == MAX_DEPTH
    }
}

pub async fn reply(
    root: Board,
    history: Vec<u64>,
    seconds: f64,
    cancelled: Arc<AtomicBool>,
) -> Option<Choice> {
    // Reserve most of the remaining time for later moves. The deadline also
    // includes worker scheduling and engine initialization.
    let deadline = millis() + (seconds * 100.).min(1250.) as u64;
    #[cfg(not(target_arch = "wasm32"))]
    {
        day_core::spawn_blocking(move || {
            let mut search = Search::new(&root, &history)?;
            while !cancelled.load(Ordering::Relaxed) {
                let left = deadline.saturating_sub(millis());
                if left == 0 || search.analyse(left.min(160), MAX_NODES) {
                    return Some(search.best());
                }
            }
            None
        })
        .await
        .ok()
        .flatten()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let mut search = Search::new(&root, &history)?;
        // Browser hosts share a thread with input, animation and the game clock.
        // Cap nodes as well as time: the engine reads the clock every 2048 nodes.
        let mut nodes_per_ms = 64;
        while !cancelled.load(Ordering::Relaxed) {
            let before = millis();
            let slice = deadline.saturating_sub(before).min(18);
            let nodes = search.remaining_nodes;
            if slice == 0 || search.analyse(slice, (slice * nodes_per_ms).max(64)) {
                return Some(search.best());
            }
            nodes_per_ms = ((nodes - search.remaining_nodes) / (millis() - before).max(1)).max(1);
            day_core::sleep(0).await;
        }
        None
    }
}
