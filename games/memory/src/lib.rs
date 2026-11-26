// SPDX-License-Identifier: GPL-2.0-or-later
use day_localization_ui::LocalizedText;
use day_pieces::prelude::*;
use day_spec::Key;
use game_resources::res;
use gamekit::chrome::{self, Help};
use gamekit::multiplayer::{self as party_ui, PlayerState};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
const MATCH_HOLD: f64 = 1.0;
const MATCH_TRAVEL: f64 = 0.8;
const MATCH_DURATION: f64 = MATCH_HOLD + 2.0 * MATCH_TRAVEL;
mod model;
use model::Match;
pub const SURFACE: Color = Color::hex(0x151C35);
use gamekit::art::{self as party_art, COLORS};
#[derive(Clone, Copy, PartialEq)]
enum Overlay {
    None,
    Pause,
    Help,
    Settings,
    Result,
}
struct Ui {
    game: RefCell<Match>,
    overlay: Signal<Overlay>,
    back: Cell<Overlay>,
    repaint: Trigger,
    cursor: Cell<usize>,
    size: Cell<Size>,
    focus: Signal<bool>,
    players: Signal<usize>,
    sounds: Signal<bool>,
    vibrations: Signal<bool>,
    faces: Cell<[f64; 32]>,
    matched_by: Cell<[usize; 30]>,
    matched_age: Cell<[f64; 32]>,
    finish_age: Cell<f64>,
    keyboard: Cell<bool>,
}
impl Ui {
    fn show(&self, overlay: Overlay) {
        self.overlay.set(overlay);
        self.focus.set(overlay == Overlay::None);
    }
    fn push(&self, overlay: Overlay) {
        self.back.set(self.overlay.get_untracked());
        self.show(overlay);
    }
    fn pause(&self) {
        if self.overlay.get_untracked() == Overlay::None {
            self.show(Overlay::Pause);
        }
    }
    fn feedback(&self, success: bool) {
        let clip = if success {
            res::assets::sounds::memory::pair_wav
        } else {
            res::assets::sounds::memory::flip_wav
        };
        chrome::sound(self.sounds.get_untracked(), &clip, 0.7);
        chrome::haptic_pattern(
            self.vibrations.get_untracked(),
            if success {
                chrome::cues::SUCCESS_BEAT
            } else {
                chrome::cues::MEDIUM_BEAT
            },
        );
    }
    fn changed(&self) {
        self.repaint.notify();
    }
    fn play(&self, i: usize) {
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        let before = self.game.borrow().scores;
        let first = self.game.borrow().first;
        let accepted = self.game.borrow_mut().play(i);
        if accepted {
            self.revealed(i, first, before);
            self.changed();
        }
    }
    fn revealed(&self, i: usize, first: Option<usize>, before: [u32; 4]) {
        if let Some(first) = first
            && self.game.borrow().matched[i]
        {
            let mut owners = self.matched_by.get();
            owners[i] = self.game.borrow().turn;
            owners[first] = owners[i];
            self.matched_by.set(owners);
        }
        self.game.borrow_mut().record_result();
        if self.game.borrow().second.is_some() {
            chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::cues::WARNING_BEAT);
            chrome::sound(
                self.sounds.get_untracked(),
                &res::assets::sounds::memory::flip_wav,
                0.7,
            );
        } else {
            self.feedback(self.game.borrow().scores != before);
        }
    }
    fn start(&self) {
        let wins = if self.overlay.get_untracked() == Overlay::Result {
            self.game.borrow().round_wins
        } else {
            [0; 4]
        };
        *self.game.borrow_mut() = Match::new(self.players.get_untracked(), gamekit::seed());
        self.game.borrow_mut().round_wins = wins;
        self.cursor.set(0);
        self.faces.set([0.0; 32]);
        self.matched_age.set([0.0; 32]);
        self.finish_age.set(0.0);
        self.feedback(false);
        self.show(Overlay::None);
        self.repaint.notify();
    }
    fn player(&self, i: usize) -> LocalizedText {
        if self.players.get_untracked() == 1 && i == 1 {
            res::str::party_computer()
        } else {
            res::str::party_player((i + 1) as f64)
        }
    }
    fn outcome(&self) -> LocalizedText {
        let g = self.game.borrow();
        let winners = g.winners();
        if winners.len() != 1 {
            res::str::party_draw()
        } else {
            res::str::party_wins(self.player(winners[0]).format())
        }
    }
    fn active_player(&self) -> Option<usize> {
        self.repaint.track();
        let g = self.game.borrow();
        (!g.finished).then_some(g.turn)
    }
    fn status(&self) -> String {
        self.repaint.track();
        let g = self.game.borrow();
        if g.finished {
            self.outcome().format()
        } else {
            res::str::party_turn(self.player(g.turn).format()).format()
        }
    }
    fn selection(&self) -> String {
        self.repaint.track();
        let g = self.game.borrow();
        let i = self.cursor.get();
        let state = if g.matched[i] {
            res::str::party_matched()
        } else if g.first == Some(i) || g.second == Some(i) {
            res::str::party_card_value(f64::from(g.cells[i] + 1))
        } else {
            res::str::party_hidden()
        };
        res::str::party_selection(res::str::PartySelection {
            row: cell_position(i).1 + 1.0,
            column: cell_position(i).0.floor() + 1.0,
            state: state.format(),
        })
        .format()
    }
    fn key(&self, key: Key) {
        if key == Key::Escape && self.overlay.get_untracked() == Overlay::Pause {
            self.show(Overlay::None);
            return;
        }
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        self.keyboard.set(true);
        let side = 4;
        let i = self.cursor.get();
        match key {
            Key::ArrowLeft => self.cursor.set((i + 30 - 1) % 30),
            Key::ArrowRight => self.cursor.set((i + 1) % 30),
            Key::ArrowUp => self.cursor.set((i + 30 - side) % 30),
            Key::ArrowDown => self.cursor.set((i + side) % 30),
            Key::Enter | Key::Character(' ') => self.play(i),
            Key::Escape | Key::Character('p' | 'P') => self.pause(),
            _ => return,
        }
        self.repaint.notify();
    }
}
pub fn memory_page(players: usize) -> AnyPiece {
    gamekit::sounds(SOUNDS);
    let settings = gamekit::restore::<chrome::GameSettings>("memory.settings").unwrap_or_default();
    let saved = gamekit::restore::<Match>("memory.v1")
        .filter(|g| g.valid() && g.players == players.clamp(1, 4));
    let initial = if !settings.instructions_shown {
        Overlay::Help
    } else if saved.as_ref().is_some_and(|g| g.finished) {
        Overlay::Result
    } else if saved.is_some() {
        Overlay::Pause
    } else {
        Overlay::None
    };
    let fresh = saved.is_none();
    let initial = if fresh && settings.instructions_shown {
        Overlay::None
    } else {
        initial
    };
    let mut game = saved.unwrap_or_else(|| Match::new(players, gamekit::seed()));
    game.record_result();
    let after_help = if game.finished {
        Overlay::Result
    } else if fresh {
        Overlay::None
    } else {
        Overlay::Pause
    };
    let matched_age = std::array::from_fn(|i| {
        if game.matched.get(i) == Some(&true) {
            MATCH_DURATION
        } else {
            0.0
        }
    });
    let ui = Rc::new(Ui {
        players: Signal::new(game.players),
        game: RefCell::new(game),
        overlay: Signal::new(initial),
        back: Cell::new(after_help),
        repaint: Trigger::new(),
        cursor: Cell::new(0),
        size: Cell::new(Size::ZERO),
        focus: Signal::new(initial == Overlay::None),
        sounds: Signal::new(settings.sounds),
        vibrations: Signal::new(settings.vibrations),
        faces: Cell::new([0.0; 32]),
        matched_by: Cell::new([0; 30]),
        matched_age: Cell::new(matched_age),
        finish_age: Cell::new(0.0),
        keyboard: Cell::new(false),
    });
    gamekit::autosave("memory.v1", {
        let u = ui.clone();
        move || u.game.borrow().clone()
    });
    gamekit::on_background("memory.v1", {
        let u = ui.clone();
        move || u.pause()
    });
    Effect::new({
        let u = ui.clone();
        move || {
            gamekit::save(
                "memory.settings",
                &chrome::GameSettings {
                    sounds: u.sounds.get(),
                    vibrations: u.vibrations.get(),
                    instructions_shown: true,
                },
            )
        }
    });
    let u = ui.clone();
    let tint = ui.clone();
    let header = party_ui::header(
        res::str::nav_memory(),
        id!(memory_pause),
        move || u.pause(),
        move || tint.active_player(),
    );
    let u = ui.clone();
    let tint = ui.clone();
    let info = column((
        label(move || u.status())
            .font(Font::Subheadline)
            .bold()
            .color(move || tint.active_player().map_or(Color::WHITE, |i| COLORS[i]))
            .align(TextAlign::Center)
            .id(id!(memory_status)),
        scores(ui.clone()).id(id!(memory_scores)).any(),
    ))
    .spacing(4.0)
    .padding(8.0)
    .grow(Dimension::Width)
    .any();
    let u = ui.clone();
    let r = ui.clone();
    let footer = when(
        move || {
            r.repaint.track();
            r.keyboard.get()
        },
        move || {
            let u = u.clone();
            label(move || u.selection())
                .font(Font::Caption)
                .color(chrome::TEXT)
                .align(TextAlign::Center)
                .id(id!(memory_selection))
                .padding(8.0)
        },
    )
    .any();
    let content = chrome::game_frame(header, Some(info), board(ui.clone()), Some(footer));
    let u = ui.clone();
    let step = ui.clone();
    let clock = gamekit::animation::clock(
        move || {
            u.repaint.track();
            u.overlay.get() == Overlay::None && {
                let g = u.game.borrow();
                g.needs_clock()
                    || g.finished
                    || (0..30).any(|i| {
                        let target =
                            f64::from(g.matched[i] || g.first == Some(i) || g.second == Some(i));
                        (u.faces.get()[i] - target).abs() > 0.001
                            || g.matched[i] && u.matched_age.get()[i] < MATCH_DURATION
                    })
            }
        },
        move |dt| {
            let delta = dt.as_secs_f64().min(0.1);
            let before = step.game.borrow().scores;
            let first = step.game.borrow().first;
            let revealed = step.game.borrow_mut().tick(dt.as_secs_f64());
            if let Some(i) = revealed {
                step.revealed(i, first, before);
            }
            let g = step.game.borrow();
            let mut faces = step.faces.get();
            let mut ages = step.matched_age.get();
            let mut collecting = false;
            for i in 0..30 {
                let target = f64::from(g.matched[i] || g.first == Some(i) || g.second == Some(i));
                faces[i] += (target - faces[i]).clamp(-delta * 2.0, delta * 2.0);
                if g.matched[i] {
                    collecting |= ages[i] < MATCH_HOLD && ages[i] + delta >= MATCH_HOLD;
                    ages[i] = (ages[i] + delta).min(MATCH_DURATION);
                }
            }
            if collecting {
                chrome::sound(
                    step.sounds.get_untracked(),
                    &res::assets::sounds::memory::collect_wav,
                    0.5,
                );
            }
            step.faces.set(faces);
            step.matched_age.set(ages);
            if g.finished {
                let age = step.finish_age.get() + delta;
                step.finish_age.set(age);
                if age >= MATCH_DURATION {
                    step.feedback(true);
                    step.show(Overlay::Result);
                }
            }
            drop(g);
            step.changed();
        },
    );
    let u = ui.clone();
    let v = ui.clone();
    zstack((
        content,
        clock,
        when(move || u.overlay.get() != Overlay::None, chrome::scrim),
        switch(
            move || v.overlay.get(),
            move |overlay| (*overlay != Overlay::None).then(|| overlay_card(ui.clone(), *overlay)),
        ),
    ))
    .background(SURFACE)
    .any()
}
fn scores(ui: Rc<Ui>) -> AnyPiece {
    let u = ui.clone();
    switch(
        move || {
            u.repaint.track();
            let g = u.game.borrow();
            (g.players, g.scores, g.turn, g.finished, g.winners())
        },
        move |&(players, scores, turn, finished, ref winners)| {
            let count = players.max(2);
            let cards = row((0..count)
                .map(|i| {
                    let state = if finished {
                        if winners.len() == 1 && winners[0] == i {
                            PlayerState::Winner
                        } else {
                            PlayerState::Finished
                        }
                    } else if turn == i {
                        PlayerState::Turn
                    } else {
                        PlayerState::Waiting
                    };
                    let value = res::str::party_count(scores[i] as f64);
                    party_ui::score_card(
                        ui.player(i),
                        value,
                        res::str::party_count(88.0),
                        i,
                        state,
                        Some(scores[i] as f64 / 15.0),
                    )
                })
                .collect::<Vec<_>>())
            .spacing(6.0)
            .grow(Dimension::Width);
            cards.any()
        },
    )
    .any()
}
fn board(ui: Rc<Ui>) -> AnyPiece {
    let faces = gamekit::sprites::load(res::assets::memory::faces_png);
    let back = gamekit::sprites::load(res::assets::memory::back_png);
    let d = ui.clone();
    let t = ui.clone();
    let k = ui.clone();
    let a = ui.clone();
    canvas(move |draw, size| {
        d.repaint.track();
        d.size.set(size);
        draw_board(draw, size, &d, [faces.get().as_ref(), back.get().as_ref()]);
        if d.game.borrow().finished {
            party_art::confetti(draw, size, d.finish_age.get());
        }
    })
    .on_touches(move |touches| {
        for contact in touches
            .iter()
            .filter(|t| t.phase == day_spec::TouchPhase::Began)
        {
            let p = contact.location;
            let (x, y, cell) = layout(t.size.get());
            if cell <= 0.0 {
                continue;
            }
            if let Some(i) = (0..30).find(|&i| {
                let (column, row) = cell_position(i);
                p.x >= x + column * cell
                    && p.x < x + (column + 1.0) * cell
                    && p.y >= y + row * cell
                    && p.y < y + (row + 1.0) * cell
            }) {
                t.keyboard.set(false);
                t.cursor.set(i);
                t.play(i);
                t.repaint.notify();
            }
        }
    })
    .on_key(move |e| k.key(e.key))
    .focused(ui.focus)
    .a11y(move |a11y| {
        a11y.label(res::str::party_board())
            .value(move || a.selection())
    })
    .id(id!(memory_board))
    .grow(Dimension::Both)
    .any()
}
fn cell_position(i: usize) -> (f64, f64) {
    if i < 3 {
        (i as f64 + 0.5, 0.0)
    } else if i >= 27 {
        ((i - 27) as f64 + 0.5, 7.0)
    } else {
        (((i - 3) % 4) as f64, ((i - 3) / 4 + 1) as f64)
    }
}
fn layout(size: Size) -> (f64, f64, f64) {
    let (columns, rows) = (4.0, 8.0);
    let cell = ((size.width - 12.0) / columns)
        .min((size.height - 12.0) / rows)
        .clamp(0.0, 120.0);
    (
        (size.width - cell * columns) / 2.0,
        (size.height - cell * rows) / 2.0,
        cell,
    )
}
fn draw_board(d: &mut Draw, size: Size, ui: &Ui, art: [Option<&day_core::Bitmap>; 2]) {
    let g = ui.game.borrow();
    let faces = ui.faces.get();
    let ages = ui.matched_age.get();
    let cursor = ui.keyboard.get().then(|| ui.cursor.get());
    let (ox, oy, c) = layout(size);
    if c < 4.0 {
        return;
    }
    let surround = [0xF79877, 0x63BBE2, 0x9ED571, 0xFFCF70][g.turn];
    d.fill(
        Shape::Rect(Rect::new(0.0, 0.0, size.width, size.height)),
        Color::hex(surround),
    );
    d.fill(
        Shape::Rect(Rect::new(ox, oy, c * 4.0, c * 8.0)),
        Color::hex(0x523F42),
    );
    // Lift revealed and travelling cards above the rest of the deck.
    for i in (0..30)
        .filter(|&i| faces[i] == 0.0)
        .chain((0..30).filter(|&i| faces[i] > 0.0))
    {
        let (column, row) = cell_position(i);
        let (x, y) = (ox + column * c, oy + row * c);
        let age = ages[i];
        if age >= MATCH_DURATION {
            continue;
        }
        let mid = ((age - MATCH_HOLD) / MATCH_TRAVEL).clamp(0.0, 1.0);
        let fly = ((age - MATCH_HOLD - MATCH_TRAVEL) / MATCH_TRAVEL).clamp(0.0, 1.0);
        let lift = (faces[i] * std::f64::consts::PI).sin();
        let scale = 1.0 + 0.3 * lift + 0.8 * mid;
        let w = c * 0.83 * scale * (faces[i] * std::f64::consts::PI).cos().abs().max(0.02);
        let h = c * 0.83 * scale;
        let direction = if matches!(ui.matched_by.get()[i], 0 | 3) {
            1.0
        } else {
            -1.0
        };
        let cx = (x + c * 0.5) * (1.0 - mid) + (size.width * 0.5 + (column - 1.5) * c * 0.35) * mid;
        let cy = (y + c * 0.5) * (1.0 - mid)
            + size.height * 0.5 * mid
            + direction * (size.height * 0.5 + h) * fly * fly;
        let tilt = ((i * 31 + usize::from(g.cells[i]) * 17) % 17) as f64 - 8.0;
        d.transformed(
            Affine::rotate(Angle::rad(tilt.to_radians())).then(Affine::translate(cx, cy)),
            |d| {
                let rect = Rect::new(-w * 0.5, -h * 0.5, w, h);
                d.fill(
                    Shape::RoundedRect(
                        Rect::new(
                            -w * 0.5 + c * 0.035,
                            -h * 0.5 + c * (0.05 + 0.06 * lift),
                            w,
                            h,
                        ),
                        c * 0.07,
                    ),
                    Color::rgba(0.0, 0.0, 0.0, 0.28),
                );
                if faces[i] >= 0.5 {
                    if let Some(sheet) = art[0] {
                        let tile = g.cells[i];
                        d.image_region(
                            sheet,
                            Rect::new(
                                f64::from(tile % 4) * 128.0,
                                f64::from(tile / 4) * 128.0,
                                128.0,
                                128.0,
                            ),
                            rect,
                        );
                    }
                } else if let Some(back) = art[1] {
                    d.image(back, rect);
                }
            },
        );
        if cursor == Some(i) {
            d.stroke(
                Shape::RoundedRect(Rect::new(x + 2.0, y + 2.0, c - 4.0, c - 4.0), 8.0),
                chrome::GOLD,
                2.0,
            );
        }
    }
}
fn overlay_card(ui: Rc<Ui>, overlay: Overlay) -> AnyPiece {
    if overlay == Overlay::Help {
        return chrome::instructions_card(
            res::str::nav_memory(),
            vec![
                Help::Para(res::str::party_memory_rules()),
                Help::Para(res::str::party_controls()),
            ],
            id!(memory_help_done),
            move || ui.show(ui.back.get()),
        )
        .id(id!(memory_help))
        .any();
    }
    let mut items = Vec::new();
    let action = |title, tint, id, next| {
        let u = ui.clone();
        chrome::menu_button(title, tint, id, move || u.push(next))
    };
    match overlay {
        Overlay::Pause => {
            items.push(chrome::card_title(res::str::gk_paused(), Color::WHITE));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_resume(),
                chrome::GREEN,
                id!(memory_resume),
                move || u.show(Overlay::None),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(memory_new_game),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(action(
                res::str::gk_settings(),
                chrome::SLATE,
                id!(memory_settings),
                Overlay::Settings,
            ));
            items.push(action(
                res::str::gk_instructions(),
                chrome::INDIGO,
                id!(memory_instructions),
                Overlay::Help,
            ));
            items.push(chrome::close_game_button(chrome::RED, id!(memory_quit)));
        }
        Overlay::Settings => {
            items.push(chrome::card_title(res::str::gk_settings(), Color::WHITE));
            items.push(chrome::setting_row(
                res::str::gk_sounds(),
                toggle(ui.sounds).id(id!(memory_sounds)).any(),
            ));
            items.push(chrome::setting_row(
                res::str::gk_vibrations(),
                toggle(ui.vibrations).id(id!(memory_vibrations)).any(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_done(),
                chrome::GREEN,
                id!(memory_done),
                move || u.show(u.back.get()),
            ));
        }
        Overlay::Result => {
            items.push(chrome::card_title(ui.outcome(), chrome::GOLD));
            items.push(scores(ui.clone()));
            items.push(
                label(res::str::party_round_wins())
                    .color(chrome::TEXT)
                    .font(Font::Caption)
                    .any(),
            );
            let wins = ui.game.borrow().round_wins;
            items.push(
                row((0..ui.game.borrow().players.max(2))
                    .map(|i| {
                        label(res::str::party_series_score(res::str::PartySeriesScore {
                            player: ui.player(i).format(),
                            wins: wins[i] as f64,
                        }))
                        .font(Font::Caption)
                        .color(COLORS[i])
                        .any()
                    })
                    .collect::<Vec<_>>())
                .spacing(10.0)
                .fit(RowFit::WrapColumns { run_spacing: 3.0 })
                .any(),
            );
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_play_again(),
                chrome::GREEN,
                id!(memory_play_again),
                move || u.start(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(memory_change_players),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(chrome::close_game_button(
                chrome::SLATE,
                id!(memory_result_quit),
            ));
        }
        _ => {}
    }
    party_ui::dialog(column(items).spacing(10.0).align(HAlign::Center))
        .id(id!(memory_menu))
        .any()
}
pub fn memory_preview() -> AnyPiece {
    let banner = gamekit::sprites::load(res::assets::memory::banner_png);
    canvas(move |d, s| {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), SURFACE);
        if let Some(b) = banner.get() {
            d.image(&b, Rect::new(0.0, 0.0, s.width, s.height));
        }
    })
    .grow(Dimension::Both)
    .any()
}

pub const SOUNDS: &[chrome::Sfx] = &[
    res::assets::sounds::memory::flip_wav,
    res::assets::sounds::memory::pair_wav,
    res::assets::sounds::memory::collect_wav,
];
