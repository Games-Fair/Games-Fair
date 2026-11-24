// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
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
            res::assets::sounds::shared::success_wav
        } else {
            res::assets::sounds::shared::tap_wav
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
        let accepted = self.game.borrow_mut().play(i);
        if accepted {
            self.game.borrow_mut().record_result();
            self.feedback(self.game.borrow().scores != before);
            self.changed();
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
        self.finish_age.set(0.0);
        self.feedback(false);
        self.show(Overlay::None);
        self.repaint.notify();
    }
    fn player(&self, i: usize) -> LocalizedText {
        let g = self.game.borrow();
        if g.players == 1 && i == 1 {
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
        let state = match g.cells[i] {
            1 => res::str::party_cross(),
            2 => res::str::party_ring(),
            _ => res::str::party_empty(),
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
        let side = 3;
        let i = self.cursor.get();
        match key {
            Key::ArrowLeft => self.cursor.set((i + 9 - 1) % 9),
            Key::ArrowRight => self.cursor.set((i + 1) % 9),
            Key::ArrowUp => self.cursor.set((i + 9 - side) % 9),
            Key::ArrowDown => self.cursor.set((i + side) % 9),
            Key::Enter | Key::Character(' ') => self.play(i),
            Key::Escape | Key::Character('p' | 'P') => self.pause(),
            _ => return,
        }
        self.repaint.notify();
    }
}
pub fn tictactoe_page(players: usize) -> AnyPiece {
    gamekit::sounds(&[]);
    let settings =
        gamekit::restore::<chrome::GameSettings>("tictactoe.settings").unwrap_or_default();
    let saved = gamekit::restore::<Match>("tictactoe.v1")
        .filter(|g| g.valid() && g.players == players.clamp(1, 2));
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
        finish_age: Cell::new(0.0),
        keyboard: Cell::new(false),
    });
    gamekit::autosave("tictactoe.v1", {
        let u = ui.clone();
        move || u.game.borrow().clone()
    });
    gamekit::on_background("tictactoe.v1", {
        let u = ui.clone();
        move || u.pause()
    });
    Effect::new({
        let u = ui.clone();
        move || {
            gamekit::save(
                "tictactoe.settings",
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
        res::str::nav_tictactoe(),
        id!(tictactoe_pause),
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
            .id(id!(tictactoe_status)),
        scores(ui.clone()).id(id!(tictactoe_scores)).any(),
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
                .id(id!(tictactoe_selection))
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
                    || (0..9).any(|i| {
                        let target = f64::from(g.cells[i] != 0);
                        (u.faces.get()[i] - target).abs() > 0.001
                    })
            }
        },
        move |dt| {
            let delta = dt.as_secs_f64().min(0.1);
            let before = step.game.borrow().scores;
            step.game.borrow_mut().tick(dt.as_secs_f64());
            step.game.borrow_mut().record_result();
            if before != step.game.borrow().scores {
                step.feedback(true);
            }
            let g = step.game.borrow();
            let mut faces = step.faces.get();
            for i in 0..9 {
                let target = f64::from(g.cells[i] != 0);
                faces[i] += (target - faces[i]).clamp(-delta * 5.0, delta * 5.0);
            }
            step.faces.set(faces);
            if g.finished {
                let age = step.finish_age.get() + delta;
                step.finish_age.set(age);
                if age >= 1.1 {
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
            (g.players, g.turn, g.finished, g.round_wins, g.winners())
        },
        move |&(players, turn, finished, wins, ref winners)| {
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
                    let value = res::str::party_wins_count(wins[i] as f64);
                    party_ui::score_card(
                        ui.player(i),
                        value,
                        res::str::party_wins_count(999.0),
                        i,
                        state,
                        None,
                    )
                })
                .collect::<Vec<_>>())
            .spacing(6.0)
            .grow(Dimension::Width);
            column((cards,)).spacing(3.0).grow(Dimension::Width).any()
        },
    )
    .any()
}
fn board(ui: Rc<Ui>) -> AnyPiece {
    use gamekit::sprites::{Ink, VectorSprite};
    let marks = [
        res::assets::tictactoe::cross_svg,
        res::assets::tictactoe::ring_svg,
    ]
    .map(|asset| VectorSprite::new(asset, Ink::Original));
    let d = ui.clone();
    let t = ui.clone();
    let k = ui.clone();
    let a = ui.clone();
    canvas(move |draw, size| {
        d.repaint.track();
        d.size.set(size);
        draw_board(
            draw,
            size,
            &d.game.borrow(),
            d.keyboard.get().then(|| d.cursor.get()),
            d.faces.get(),
            &marks,
        );
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
            if let Some(i) = (0..9).find(|&i| {
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
    .id(id!(tictactoe_board))
    .grow(Dimension::Both)
    .any()
}
fn cell_position(i: usize) -> (f64, f64) {
    ((i % 3) as f64, (i / 3) as f64)
}
fn layout(size: Size) -> (f64, f64, f64) {
    let (columns, rows) = (3.0, 3.0);
    let cell = (size.width * 0.66 / columns)
        .min((size.height - 12.0) / rows)
        .clamp(0.0, 120.0);
    (
        (size.width - cell * columns) / 2.0,
        (size.height - cell * rows) / 2.0,
        cell,
    )
}
fn draw_board(
    d: &mut Draw,
    size: Size,
    g: &Match,
    cursor: Option<usize>,
    faces: [f64; 32],
    marks: &[gamekit::sprites::VectorSprite; 2],
) {
    let (ox, oy, c) = layout(size);
    if c < 4.0 {
        return;
    }
    let bg = if g.turn == 0 {
        Color::hex(0x855154)
    } else {
        Color::hex(0x2B748A)
    };
    d.fill(
        Shape::Rect(Rect::new(0.0, 0.0, size.width, size.height)),
        bg,
    );
    for i in 0..9 {
        let (column, row) = cell_position(i);
        let (x, y) = (ox + column * c, oy + row * c);
        let center = Point::new(x + c / 2.0, y + c / 2.0);
        let progress = faces[i];
        if g.cells[i] != 0
            && let Some(mark) = marks[usize::from(g.cells[i] - 1)].at_width(c * 0.8)
        {
            let side = c * 0.8 * progress;
            d.image(
                &mark,
                Rect::new(center.x - side * 0.5, center.y - side * 0.5, side, side),
            );
        }
        if cursor == Some(i) {
            d.stroke(
                Shape::RoundedRect(Rect::new(x + 2.0, y + 2.0, c - 4.0, c - 4.0), 8.0),
                chrome::GOLD,
                2.0,
            );
        }
    }
    for i in 1..3 {
        let offset = i as f64 * c;
        d.stroke(
            Shape::Line(
                Point::new(ox + offset, oy),
                Point::new(ox + offset, oy + 3.0 * c),
            ),
            Color::BLACK,
            c * 0.14,
        );
        d.stroke(
            Shape::Line(
                Point::new(ox, oy + offset),
                Point::new(ox + 3.0 * c, oy + offset),
            ),
            Color::BLACK,
            c * 0.14,
        );
    }
    if let Some([a, _, b]) = g.winning_line() {
        let at = |i: usize| {
            Point::new(
                ox + (i % 3) as f64 * c + c / 2.0,
                oy + (i / 3) as f64 * c + c / 2.0,
            )
        };
        d.stroke(Shape::Line(at(a), at(b)), Color::WHITE, c * 0.09);
    }
}
fn overlay_card(ui: Rc<Ui>, overlay: Overlay) -> AnyPiece {
    if overlay == Overlay::Help {
        return chrome::instructions_card(
            res::str::nav_tictactoe(),
            vec![
                Help::Para(res::str::party_ttt_rules()),
                Help::Para(res::str::party_controls()),
            ],
            id!(tictactoe_help_done),
            move || ui.show(ui.back.get()),
        )
        .id(id!(tictactoe_help))
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
                id!(tictactoe_resume),
                move || u.show(Overlay::None),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(tictactoe_new_game),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(action(
                res::str::gk_settings(),
                chrome::SLATE,
                id!(tictactoe_settings),
                Overlay::Settings,
            ));
            items.push(action(
                res::str::gk_instructions(),
                chrome::INDIGO,
                id!(tictactoe_instructions),
                Overlay::Help,
            ));
            items.push(chrome::close_game_button(chrome::RED, id!(tictactoe_quit)));
        }
        Overlay::Settings => {
            items.push(chrome::card_title(res::str::gk_settings(), Color::WHITE));
            items.push(chrome::setting_row(
                res::str::gk_sounds(),
                toggle(ui.sounds).id(id!(tictactoe_sounds)).any(),
            ));
            items.push(chrome::setting_row(
                res::str::gk_vibrations(),
                toggle(ui.vibrations).id(id!(tictactoe_vibrations)).any(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_done(),
                chrome::GREEN,
                id!(tictactoe_done),
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
                id!(tictactoe_play_again),
                move || u.start(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(tictactoe_change_players),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(chrome::close_game_button(
                chrome::SLATE,
                id!(tictactoe_result_quit),
            ));
        }
        _ => {}
    }
    party_ui::dialog(column(items).spacing(10.0).align(HAlign::Center))
        .id(id!(tictactoe_menu))
        .any()
}
pub fn tictactoe_preview() -> AnyPiece {
    let banner = gamekit::sprites::load(res::assets::tictactoe::banner_png);
    canvas(move |d, s| {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), SURFACE);
        if let Some(b) = banner.get() {
            d.image(&b, Rect::new(0.0, 0.0, s.width, s.height));
        }
    })
    .grow(Dimension::Both)
    .any()
}
