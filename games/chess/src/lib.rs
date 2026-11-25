// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
mod art;
mod engine;
mod model;
pub use art::preview;
use cozy_chess::{Color as Side, Move, Piece as Man, Square};
use day_localization_ui::LocalizedText;
use day_pieces::prelude::*;
use day_spec::Key;
use game_resources::res;
use gamekit::chrome;
use model::{End, Game, Pace, Reason, Saved, captured, destination, legal};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub const SURFACE: Color = Color::hex(0x172a25);
const JADE: Color = Color::hex(0x366958);
const BRASS: Color = Color::hex(0xdabc7d);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Overlay {
    Setup,
    Play,
    Pause,
    Help,
    Settings,
    Promotion,
    Result,
    Offer,
    Resign,
}
fn side(s: Side) -> LocalizedText {
    if s == Side::White {
        res::str::chess_white()
    } else {
        res::str::chess_black()
    }
}
fn kind(p: Man) -> LocalizedText {
    match p {
        Man::Pawn => res::str::chess_pawn(),
        Man::Knight => res::str::chess_knight(),
        Man::Bishop => res::str::chess_bishop(),
        Man::Rook => res::str::chess_rook(),
        Man::Queen => res::str::chess_queen(),
        Man::King => res::str::chess_king(),
    }
}
fn outcome(e: End) -> LocalizedText {
    e.winner.map_or_else(res::str::party_draw, |i| {
        res::str::chess_wins(side(Side::ALL[i]).format())
    })
}
fn reason(r: Reason) -> LocalizedText {
    match r {
        Reason::Mate => res::str::chess_mate(),
        Reason::Stalemate => res::str::chess_stalemate(),
        Reason::Dead => res::str::chess_dead(),
        Reason::Repetition => res::str::chess_three(),
        Reason::Fivefold => res::str::chess_five(),
        Reason::Fifty => res::str::chess_fifty(),
        Reason::SeventyFive => res::str::chess_seventyfive(),
        Reason::Flag => res::str::chess_flag(),
        Reason::Agreement => res::str::chess_agreed(),
        Reason::Resigned => res::str::chess_resigned(),
    }
}
#[derive(Clone, Copy)]
struct Motion {
    from: Square,
    to: Square,
    start: f64,
    rook: Option<(Square, Square)>,
    moving: Man,
    taken: Option<(Man, Square)>,
}
struct Ui {
    game: RefCell<Game>,
    overlay: Signal<Overlay>,
    back: Cell<Overlay>,
    started: Cell<bool>,
    players: Signal<usize>,
    desktop: bool,
    flip: Signal<bool>,
    sounds: Signal<bool>,
    vibrations: Signal<bool>,
    repaint: Trigger,
    revision: Signal<u32>,
    focus: Signal<bool>,
    size: Cell<Size>,
    selected: Cell<Option<Square>>,
    options: RefCell<Vec<Move>>,
    cursor: Cell<Square>,
    keyboard: Cell<bool>,
    contact: Cell<Option<(u64, Square, Point)>>,
    drag: Cell<Option<Point>>,
    motion: Cell<Option<Motion>>,
    promotion: RefCell<Vec<Move>>,
    claiming: Cell<bool>,
    declined: Cell<bool>,
    stamp: Cell<f64>,
}
fn now() -> f64 {
    day_core::frame::now_secs()
}
impl Ui {
    fn advance(&self) {
        let t = now();
        let elapsed = t - self.stamp.replace(t);
        if matches!(
            self.overlay.get_untracked(),
            Overlay::Play | Overlay::Promotion
        ) {
            self.game.borrow_mut().elapse(elapsed);
        }
    }
    fn finish(&self) {
        self.motion.set(None);
        self.overlay.set(Overlay::Result);
        self.focus.set(false);
        let e = self.game.borrow().saved.end.expect("finished match");
        let lost = self.game.borrow().saved.players == 1 && e.winner == Some(1);
        let (cue, clip) = match (e.winner, lost) {
            (None, _) => (
                chrome::cues::MEDIUM_BEAT,
                res::assets::sounds::shared::pluck_wav,
            ),
            (_, true) => (chrome::LETDOWN, res::assets::sounds::shared::letdown_wav),
            _ => (chrome::CELEBRATE, res::assets::sounds::shared::success_wav),
        };
        self.feedback(cue, clip);
        self.repaint.notify();
    }
    fn feedback(&self, pattern: chrome::Pattern, clip: chrome::Sfx) {
        chrome::haptic_pattern(self.vibrations.get_untracked(), pattern);
        chrome::sound(self.sounds.get_untracked(), &clip, 0.58);
    }
    fn show(&self, o: Overlay) {
        self.advance();
        if self.game.borrow().saved.end.is_some()
            && matches!(
                self.overlay.get_untracked(),
                Overlay::Play | Overlay::Promotion
            )
        {
            self.finish();
            return;
        }
        self.contact.set(None);
        self.drag.set(None);
        self.overlay.set(o);
        self.focus.set(o == Overlay::Play);
        self.repaint.notify();
    }
    fn pause(&self) {
        if matches!(
            self.overlay.get_untracked(),
            Overlay::Play | Overlay::Promotion
        ) {
            if self.claiming.replace(false) {
                self.select(None);
            }
            self.show(Overlay::Pause);
        }
    }
    fn resume(&self) {
        self.show(if self.promotion.borrow().is_empty() {
            Overlay::Play
        } else {
            Overlay::Promotion
        });
    }
    fn start(&self, pace: Pace) {
        *self.game.borrow_mut() = Game::new(self.players.get_untracked(), pace);
        self.started.set(true);
        self.selected.set(None);
        self.options.borrow_mut().clear();
        self.motion.set(None);
        self.promotion.borrow_mut().clear();
        self.cursor.set(Square::E2);
        self.claiming.set(false);
        self.declined.set(false);
        self.show(Overlay::Play);
        self.revision.update(|v| *v += 1);
    }
    fn computer(&self) -> bool {
        let g = self.game.borrow();
        g.saved.players == 1 && g.board.side_to_move() == Side::Black
    }
    fn reversed(&self) -> bool {
        let g = self.game.borrow();
        self.flip.get()
            && g.saved.players == 2
            && ((g.board.side_to_move() == Side::Black) != self.motion.get().is_some())
    }
    fn status(&self) -> String {
        self.repaint.track();
        let g = self.game.borrow();
        let s = side(g.board.side_to_move()).format();
        if let Some(e) = g.saved.end {
            outcome(e)
        } else if self.claiming.get() {
            res::str::chess_intended()
        } else if self.declined.get() {
            res::str::chess_declined()
        } else if !g.board.checkers().is_empty() {
            res::str::chess_check(s)
        } else if self.computer() {
            res::str::chess_thinking()
        } else {
            res::str::chess_turn(s)
        }
        .format()
    }
    fn selection(&self) -> String {
        self.repaint.track();
        let g = self.game.borrow();
        let sq = self.cursor.get();
        let occupant = g
            .board
            .piece_on(sq)
            .map_or_else(res::str::chess_empty, |p| {
                res::str::chess_piece(res::str::ChessPiece {
                    side: side(g.board.color_on(sq).expect("occupied square")).format(),
                    kind: kind(p).format(),
                })
            });
        res::str::chess_square(res::str::ChessSquare {
            square: sq.to_string(),
            occupant: occupant.format(),
        })
        .format()
    }
    fn select(&self, sq: Option<Square>) {
        self.selected.set(sq);
        let g = self.game.borrow();
        *self.options.borrow_mut() = sq.map_or_else(Vec::new, |s| {
            legal(&g.board)
                .into_iter()
                .filter(|m| m.from == s && (!self.claiming.get() || g.intended(*m).is_some()))
                .collect()
        });
        if sq.is_some() {
            chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::cues::TICK_BEAT);
        }
        self.repaint.notify();
    }
    fn tap(&self, sq: Square) {
        if self.overlay.get_untracked() != Overlay::Play || self.computer() {
            return;
        }
        self.cursor.set(sq);
        let g = self.game.borrow();
        let candidates: Vec<_> = self
            .options
            .borrow()
            .iter()
            .copied()
            .filter(|&m| destination(&g.board, m) == sq)
            .collect();
        let own = g.board.color_on(sq) == Some(g.board.side_to_move());
        drop(g);
        match candidates.as_slice() {
            [m] => self.commit(*m),
            [] => self.select((own && self.selected.get() != Some(sq)).then_some(sq)),
            _ => {
                *self.promotion.borrow_mut() = candidates;
                self.show(Overlay::Promotion);
            }
        }
    }
    fn commit(&self, m: Move) {
        self.advance();
        if self.game.borrow().saved.end.is_some() {
            self.finish();
            return;
        }
        let mut g = self.game.borrow_mut();
        if self.claiming.get()
            && let Some(r) = g.intended(m)
        {
            g.end(r, None);
            drop(g);
            self.finish();
            return;
        }
        let moving = g.board.piece_on(m.from).expect("legal mover");
        let taken = captured(&g.board, m).map(|p| {
            (
                p,
                if g.board.piece_on(m.to).is_none() {
                    Square::new(m.to.file(), m.from.rank())
                } else {
                    m.to
                },
            )
        });
        let to = destination(&g.board, m);
        let rook = (to != m.to).then(|| {
            (
                m.to,
                Square::new(
                    if to.file() == cozy_chess::File::G {
                        cozy_chess::File::F
                    } else {
                        cozy_chess::File::D
                    },
                    to.rank(),
                ),
            )
        });
        if !g.play(m) {
            return;
        }
        let check = !g.board.checkers().is_empty();
        let end = g.saved.end.is_some();
        drop(g);
        self.selected.set(None);
        self.options.borrow_mut().clear();
        self.promotion.borrow_mut().clear();
        self.claiming.set(false);
        self.declined.set(false);
        self.cursor.set(to);
        self.stamp.set(now());
        self.motion
            .set((!day_core::reduce_motion()).then_some(Motion {
                from: m.from,
                to,
                start: now(),
                rook,
                moving,
                taken,
            }));
        if end {
            self.finish();
        } else {
            self.overlay.set(Overlay::Play);
            self.focus.set(true);
            let (pattern, clip) = if check {
                (
                    chrome::cues::WARNING_BEAT,
                    res::assets::sounds::shared::hint_wav,
                )
            } else if taken.is_some() {
                (chrome::THUD, res::assets::sounds::shared::thud_wav)
            } else if rook.is_some() || m.promotion.is_some() {
                (
                    chrome::cues::SUCCESS_BEAT,
                    res::assets::sounds::shared::pluck_wav,
                )
            } else {
                (
                    chrome::cues::LIGHT_BEAT,
                    res::assets::sounds::shared::tap_wav,
                )
            };
            self.feedback(pattern, clip);
        }
        self.revision.update(|v| *v += 1);
        self.repaint.notify();
    }
    fn key(&self, k: Key) {
        if self.overlay.get_untracked() != Overlay::Play {
            return;
        }
        if k == Key::Escape {
            self.pause();
            return;
        }
        let (x, y) = match k {
            Key::ArrowUp => (0, 1),
            Key::ArrowDown => (0, -1),
            Key::ArrowLeft => (-1, 0),
            Key::ArrowRight => (1, 0),
            Key::Enter | Key::Character(' ') => {
                self.tap(self.cursor.get());
                return;
            }
            _ => return,
        };
        let dir = if self.reversed() { -1 } else { 1 };
        self.keyboard.set(true);
        if let Some(s) = self.cursor.get().try_offset(x * dir, y * dir) {
            self.cursor.set(s);
        }
        self.repaint.notify();
    }
}
fn opponent(ui: &Rc<Ui>) {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let pending = Rc::new(RefCell::new(
        None::<(day_core::TaskHandle, Arc<AtomicBool>)>,
    ));
    let cancel = {
        let p = pending.clone();
        move || {
            if let Some((h, flag)) = p.borrow_mut().take() {
                flag.store(true, Ordering::Relaxed);
                h.abort();
            }
        }
    };
    Scope::current().on_cleanup(cancel.clone());
    let weak = Rc::downgrade(ui);
    let (overlay, revision) = (ui.overlay, ui.revision);
    Effect::new(move || {
        let state = overlay.get();
        let _ = revision.get();
        cancel();
        let Some(u) = weak.upgrade() else {
            return;
        };
        if state != Overlay::Play || !u.computer() {
            return;
        }
        let g = u.game.borrow();
        let board = g.board.clone();
        let history = engine::prior_hashes(&g.previous);
        let seconds = if g.saved.pace == Pace::Untimed {
            f64::INFINITY
        } else {
            g.saved.remaining[Side::Black as usize]
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        let guard = cancelled.clone();
        let target = weak.clone();
        let task = day_core::task(async move {
            if seconds > 2. {
                day_core::sleep(200).await;
            }
            let best = engine::reply(board, history, seconds, guard).await;
            if let (Some(choice), Some(u)) = (best, target.upgrade())
                && u.overlay.get_untracked() == Overlay::Play
                && u.computer()
            {
                u.advance();
                if u.game.borrow().saved.end.is_some() {
                    u.finish();
                    return;
                }
                let mut g = u.game.borrow_mut();
                let claim = g.claim().or_else(|| g.intended(choice.mv));
                if let Some(r) = claim
                    && choice.score <= 0
                {
                    g.end(r, None);
                    drop(g);
                    u.finish();
                } else {
                    drop(g);
                    u.commit(choice.mv);
                }
            }
        });
        *pending.borrow_mut() = Some((task, cancelled));
    });
}
pub fn page(preset: Option<usize>) -> AnyPiece {
    gamekit::sounds(&[]);
    let key = match preset {
        Some(2) => "chess.local.v1",
        Some(_) => "chess.solo.v1",
        None => "chess.desktop.v1",
    };
    let saved = gamekit::restore::<Option<Saved>>(key)
        .flatten()
        .and_then(|s| {
            let n = preset.unwrap_or(s.players).clamp(1, 2);
            Game::restore(s, n)
        });
    let started = saved.is_some();
    let g = saved.unwrap_or_else(|| Game::new(preset.unwrap_or(1), Pace::Untimed));
    let initial = if g.saved.end.is_some() {
        Overlay::Result
    } else if started {
        Overlay::Pause
    } else {
        Overlay::Setup
    };
    let settings = gamekit::restore::<chrome::GameSettings>("chess.settings").unwrap_or_default();
    let ui = Rc::new(Ui {
        players: Signal::new(g.saved.players),
        game: RefCell::new(g),
        overlay: Signal::new(initial),
        back: Cell::new(initial),
        started: Cell::new(started),
        desktop: preset.is_none(),
        flip: Signal::new(gamekit::restore("chess.flip").unwrap_or(false)),
        sounds: Signal::new(settings.sounds),
        vibrations: Signal::new(settings.vibrations),
        repaint: Trigger::new(),
        revision: Signal::new(0),
        focus: Signal::new(false),
        size: Cell::new(Size::ZERO),
        selected: Cell::new(None),
        options: RefCell::new(Vec::new()),
        cursor: Cell::new(Square::E2),
        keyboard: Cell::new(false),
        contact: Cell::new(None),
        drag: Cell::new(None),
        motion: Cell::new(None),
        promotion: RefCell::new(Vec::new()),
        claiming: Cell::new(false),
        declined: Cell::new(false),
        stamp: Cell::new(now()),
    });
    let u = ui.clone();
    gamekit::autosave(key, move || {
        // Backgrounding settles the clock before saving. During unmount the
        // reactive signals may already be disposed; read only the match snapshot.
        u.started.get().then(|| u.game.borrow().saved.clone())
    });
    let u = ui.clone();
    gamekit::on_background(key, move || u.pause());
    let u = ui.clone();
    Effect::new(move || {
        gamekit::save(
            "chess.settings",
            &chrome::GameSettings {
                sounds: u.sounds.get(),
                vibrations: u.vibrations.get(),
                instructions_shown: true,
            },
        );
        gamekit::save("chess.flip", &u.flip.get());
    });
    opponent(&ui);
    let u = ui.clone();
    let header = chrome::game_header(res::str::nav_chess(), id!(chess_pause), move || u.pause());
    let u = ui.clone();
    let info = label(move || u.status())
        .font(Font::Headline)
        .color(BRASS)
        .align(TextAlign::Center)
        .id(id!(chess_status))
        .padding(5.)
        .any();
    let u = ui.clone();
    let footer = label(move || {
        u.repaint.track();
        if u.keyboard.get() {
            u.selection()
        } else {
            let g = u.game.borrow();
            if g.notation.is_empty() {
                String::new()
            } else {
                res::str::chess_last(g.notation.clone()).format()
            }
        }
    })
    .font(Font::Caption)
    .color(BRASS)
    .padding(6.)
    .any();
    let content = chrome::game_frame(header, Some(info), art::board(ui.clone()), Some(footer));
    let u = ui.clone();
    let v = ui.clone();
    let clock = gamekit::animation::clock(
        move || {
            u.repaint.track();
            matches!(u.overlay.get(), Overlay::Play | Overlay::Promotion)
                && (u.motion.get().is_some() || u.game.borrow().saved.pace != Pace::Untimed)
        },
        move |_| {
            let before = v.game.borrow().saved.remaining.map(f64::ceil);
            v.advance();
            if v.game.borrow().saved.end.is_some() {
                v.finish();
                return;
            }
            let moving = v.motion.get().is_some();
            if moving && v.motion.get().is_some_and(|m| now() - m.start >= 0.18) {
                v.motion.set(None);
            }
            if moving || before != v.game.borrow().saved.remaining.map(f64::ceil) {
                v.repaint.notify();
            }
        },
    );
    let o = ui.overlay;
    zstack((
        content,
        clock,
        when(move || o.get() != Overlay::Play, chrome::scrim),
        switch(
            move || o.get(),
            move |o| (*o != Overlay::Play).then(|| menu(ui.clone(), *o)),
        ),
    ))
    .background(SURFACE)
    .any()
}
fn menu(ui: Rc<Ui>, o: Overlay) -> AnyPiece {
    if o == Overlay::Help {
        return chrome::instructions_card(
            res::str::nav_chess(),
            vec![chrome::Help::Para(res::str::chess_help())],
            id!(chess_help_done),
            move || ui.show(ui.back.get()),
        );
    }
    let mut items = Vec::new();
    let go = |title, id, tint, next| {
        let u = ui.clone();
        chrome::menu_button(title, tint, id, move || {
            u.back.set(o);
            u.show(next);
        })
    };
    let action = |title, id, tint, f: fn(&Ui)| {
        let u = ui.clone();
        chrome::menu_button(title, tint, id, move || f(&u))
    };
    match o {
        Overlay::Setup => {
            items.push(chrome::card_title(res::str::chess_setup(), BRASS));
            if ui.desktop {
                items.push(
                    picker(
                        ui.players,
                        [
                            (1, res::str::party_computer()),
                            (2, res::str::fs_two_players()),
                        ],
                    )
                    .segmented()
                    .id(id!(chess_opponent))
                    .any(),
                );
            }
            for (p, t, id) in [
                (Pace::Untimed, res::str::chess_untimed(), id!(chess_untimed)),
                (Pace::Blitz, res::str::chess_blitz(), id!(chess_blitz)),
                (Pace::Rapid, res::str::chess_rapid(), id!(chess_rapid)),
            ] {
                let u = ui.clone();
                items.push(chrome::menu_button(t, JADE, id, move || u.start(p)));
            }
            items.push(go(
                res::str::gk_instructions(),
                id!(chess_help),
                chrome::SLATE,
                Overlay::Help,
            ));
            if ui.started.get() {
                items.push(action(
                    res::str::gk_cancel(),
                    id!(chess_cancel),
                    chrome::SLATE,
                    |u| {
                        u.show(if u.game.borrow().saved.end.is_some() {
                            Overlay::Result
                        } else {
                            Overlay::Pause
                        })
                    },
                ));
            } else {
                items.push(chrome::close_game_button(chrome::SLATE, id!(chess_quit)));
            }
        }
        Overlay::Pause => {
            items.push(chrome::card_title(res::str::gk_paused(), BRASS));
            items.push(action(
                res::str::gk_resume(),
                id!(chess_resume),
                JADE,
                Ui::resume,
            ));
            if !ui.computer() && ui.promotion.borrow().is_empty() {
                let g = ui.game.borrow();
                if g.claim().is_some() || legal(&g.board).iter().any(|&m| g.intended(m).is_some()) {
                    items.push(action(
                        res::str::chess_claim(),
                        id!(chess_claim),
                        JADE,
                        |u| {
                            let claim = u.game.borrow().claim();
                            if let Some(r) = claim {
                                u.game.borrow_mut().end(r, None);
                                u.finish();
                            } else {
                                u.claiming.set(true);
                                u.select(None);
                                u.resume();
                            }
                        },
                    ));
                }
                if g.saved.moves.len() >= 2 {
                    items.push(action(
                        res::str::chess_offer(),
                        id!(chess_offer),
                        chrome::SLATE,
                        |u| {
                            if u.game.borrow().saved.players == 2 {
                                u.show(Overlay::Offer);
                            } else if engine::evaluate(&u.game.borrow().board) >= 100 {
                                u.game.borrow_mut().end(Reason::Agreement, None);
                                u.finish();
                            } else {
                                u.declined.set(true);
                                u.resume();
                            }
                        },
                    ));
                }
                items.push(go(
                    res::str::chess_resign(),
                    id!(chess_resign),
                    chrome::SLATE,
                    Overlay::Resign,
                ));
            }
            items.push(go(
                res::str::gk_settings(),
                id!(chess_settings),
                chrome::SLATE,
                Overlay::Settings,
            ));
            items.push(go(
                res::str::gk_instructions(),
                id!(chess_help),
                chrome::SLATE,
                Overlay::Help,
            ));
            items.push(go(
                res::str::gk_new_game(),
                id!(chess_new),
                chrome::SLATE,
                Overlay::Setup,
            ));
            items.push(chrome::close_game_button(chrome::SLATE, id!(chess_quit)));
        }
        Overlay::Offer | Overlay::Resign => {
            let offer = o == Overlay::Offer;
            items.push(chrome::card_title(
                if offer {
                    res::str::chess_offer_question()
                } else {
                    res::str::chess_resign()
                },
                BRASS,
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                if offer {
                    res::str::chess_accept()
                } else {
                    res::str::chess_resign()
                },
                JADE,
                id!(chess_confirm),
                move || {
                    if offer {
                        u.game.borrow_mut().end(Reason::Agreement, None);
                    } else {
                        u.game.borrow_mut().lose(Reason::Resigned);
                    }
                    u.finish();
                },
            ));
            items.push(action(
                if offer {
                    res::str::chess_decline()
                } else {
                    res::str::gk_cancel()
                },
                id!(chess_cancel),
                chrome::SLATE,
                Ui::resume,
            ));
        }
        Overlay::Settings => {
            items.push(chrome::card_title(res::str::gk_settings(), BRASS));
            items.push(chrome::setting_row(
                res::str::gk_sounds(),
                toggle(ui.sounds).any(),
            ));
            items.push(chrome::setting_row(
                res::str::gk_vibrations(),
                toggle(ui.vibrations).any(),
            ));
            if ui.game.borrow().saved.players == 2 {
                items.push(chrome::setting_row(
                    res::str::chess_flip(),
                    toggle(ui.flip).id(id!(chess_flip_board)).any(),
                ));
            }
            items.push(go(
                res::str::gk_done(),
                id!(chess_done),
                JADE,
                Overlay::Pause,
            ));
        }
        Overlay::Promotion => {
            items.push(chrome::card_title(res::str::chess_promotion(), BRASS));
            for (p, id) in [
                (Man::Queen, id!(chess_queen)),
                (Man::Rook, id!(chess_rook)),
                (Man::Bishop, id!(chess_bishop)),
                (Man::Knight, id!(chess_knight)),
            ] {
                let u = ui.clone();
                items.push(chrome::menu_button(kind(p), JADE, id, move || {
                    let m = u
                        .promotion
                        .borrow()
                        .iter()
                        .copied()
                        .find(|m| m.promotion == Some(p));
                    if let Some(m) = m {
                        u.commit(m);
                    }
                }));
            }
        }
        Overlay::Result => {
            let e = ui.game.borrow().saved.end.expect("result overlay");
            items.push(chrome::card_title(outcome(e), BRASS));
            items.push(
                label(reason(e.reason))
                    .color(chrome::TEXT)
                    .font(Font::Headline)
                    .any(),
            );
            items.push(go(
                res::str::gk_play_again(),
                id!(chess_again),
                JADE,
                Overlay::Setup,
            ));
            items.push(chrome::close_game_button(chrome::SLATE, id!(chess_quit)));
        }
        _ => {}
    }
    gamekit::multiplayer::dialog(column(items).spacing(9.).align(HAlign::Center))
        .id(id!(chess_menu))
        .any()
}

#[cfg(test)]
mod tests;
