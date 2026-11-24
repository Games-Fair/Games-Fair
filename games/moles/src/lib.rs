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
    finish_age: Cell<f64>,
    hit_flash: Cell<[f64; 11]>,
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
        chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::cues::MEDIUM_BEAT);
    }
    fn changed(&self) {
        self.repaint.notify();
    }
    fn play(&self, i: usize) {
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        let accepted = self.game.borrow_mut().play(i);
        if accepted {
            self.game.borrow_mut().record_result();
            let mut flash = self.hit_flash.get();
            flash[i] = 1.0;
            self.hit_flash.set(flash);
            chrome::sound(
                self.sounds.get_untracked(),
                &res::assets::sounds::moles::hit_wav,
                0.8,
            );
            chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::cues::MEDIUM_BEAT);
            self.changed();
        } else if self.game.borrow().countdown <= 0.0 && !self.game.borrow().finished {
            chrome::sound(
                self.sounds.get_untracked(),
                &res::assets::sounds::moles::miss_wav,
                0.45,
            );
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
        self.hit_flash.set([0.0; 11]);
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
        None
    }
    fn status(&self) -> String {
        self.repaint.track();
        let g = self.game.borrow();
        if g.finished {
            self.outcome().format()
        } else if g.countdown > 0.0 {
            res::str::arena_countdown(g.countdown.ceil()).format()
        } else {
            res::str::party_status_detail(res::str::PartyStatusDetail {
                status: res::str::party_everyone_plays().format(),
                detail: res::str::party_moles_race().format(),
            })
            .format()
        }
    }
    fn selection(&self) -> String {
        self.repaint.track();
        let g = self.game.borrow();
        let i = self.cursor.get();
        let state = if !g.finished && g.countdown <= 0.0 && g.cells[i] != 0 {
            res::str::party_player(f64::from(g.cells[i]))
        } else {
            res::str::party_empty()
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
            Key::ArrowLeft => self.cursor.set((i + 11 - 1) % 11),
            Key::ArrowRight => self.cursor.set((i + 1) % 11),
            Key::ArrowUp => self.cursor.set((i + 11 - side) % 11),
            Key::ArrowDown => self.cursor.set((i + side) % 11),
            Key::Enter | Key::Character(' ') => self.play(i),
            Key::Escape | Key::Character('p' | 'P') => self.pause(),
            _ => return,
        }
        self.repaint.notify();
    }
}
pub fn moles_page(players: usize) -> AnyPiece {
    gamekit::sounds(SOUNDS);
    let settings = gamekit::restore::<chrome::GameSettings>("moles.settings").unwrap_or_default();
    let saved = gamekit::restore::<Match>("moles.v1")
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
        hit_flash: Cell::new([0.0; 11]),
        finish_age: Cell::new(0.0),
        keyboard: Cell::new(false),
    });
    gamekit::autosave("moles.v1", {
        let u = ui.clone();
        move || u.game.borrow().clone()
    });
    gamekit::on_background("moles.v1", {
        let u = ui.clone();
        move || u.pause()
    });
    Effect::new({
        let u = ui.clone();
        move || {
            gamekit::save(
                "moles.settings",
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
        res::str::nav_moles(),
        id!(moles_pause),
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
            .id(id!(moles_status)),
        scores(ui.clone()).id(id!(moles_scores)).any(),
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
                .id(id!(moles_selection))
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
                g.needs_clock() || g.finished
            }
        },
        move |dt| {
            let delta = dt.as_secs_f64().min(0.1);
            step.hit_flash
                .set(step.hit_flash.get().map(|f| (f - delta / 0.32).max(0.0)));
            let before = step.game.borrow().scores;
            let countdown = step.game.borrow().countdown.ceil();
            step.game.borrow_mut().tick(dt.as_secs_f64());
            step.game.borrow_mut().record_result();
            if countdown != step.game.borrow().countdown.ceil() {
                step.feedback(false);
            }
            if before != step.game.borrow().scores {
                step.feedback(true);
            }
            let g = step.game.borrow();
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
            (g.players, g.scores, g.finished, g.winners())
        },
        move |&(players, scores, finished, ref winners)| {
            let count = players.max(2);
            let cards = row((0..count)
                .map(|i| {
                    let state = if finished {
                        if winners.len() == 1 && winners[0] == i {
                            PlayerState::Winner
                        } else {
                            PlayerState::Finished
                        }
                    } else {
                        PlayerState::Playing
                    };
                    let value = res::str::party_count(scores[i] as f64);
                    party_ui::score_card(
                        ui.player(i),
                        value,
                        res::str::party_count(88.0),
                        i,
                        state,
                        Some(scores[i] as f64 / 30.0),
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
    let sprites = gamekit::sprites::load(res::assets::moles::moles_png);
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
            sprites.get().as_ref(),
            d.hit_flash.get(),
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
            if let Some(i) = (0..11).find(|&i| {
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
            } else if t.game.borrow().countdown <= 0.0 && !t.game.borrow().finished {
                chrome::sound(
                    t.sounds.get_untracked(),
                    &res::assets::sounds::moles::dirt_wav,
                    0.4,
                );
            }
        }
    })
    .on_key(move |e| k.key(e.key))
    .focused(ui.focus)
    .a11y(move |a11y| {
        a11y.label(res::str::party_board())
            .value(move || a.selection())
    })
    .id(id!(moles_board))
    .grow(Dimension::Both)
    .any()
}
fn cell_position(i: usize) -> (f64, f64) {
    [
        (0.6, 0.0),
        (2.8, 0.2),
        (1.7, 1.15),
        (0.25, 2.2),
        (2.6, 2.2),
        (1.6, 3.3),
        (0.1, 4.4),
        (2.7, 4.55),
        (1.75, 5.6),
        (0.45, 6.7),
        (2.9, 6.6),
    ][i]
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
fn draw_board(
    d: &mut Draw,
    size: Size,
    g: &Match,
    cursor: Option<usize>,
    sprites: Option<&day_core::Bitmap>,
    hits: [f64; 11],
) {
    let (ox, oy, c) = layout(size);
    if c < 4.0 {
        return;
    }
    d.fill(
        Shape::Rect(Rect::new(0.0, 0.0, size.width, size.height)),
        Color::hex(0x50342D),
    );
    for (x, y) in [(0.33, 1.17), (3.61, 2.68), (0.44, 6.11), (2.84, 7.63)] {
        for (dx, dy, radius) in [(0.0, 0.0, 0.12), (0.18, -0.13, 0.08), (-0.14, 0.1, 0.06)] {
            d.fill(
                Shape::Ellipse(Rect::new(
                    ox + (x + dx) * c,
                    oy + (y + dy) * c,
                    radius * 2.0 * c,
                    radius * 2.0 * c,
                )),
                Color::hex(0x483029),
            );
        }
    }
    for i in 0..11 {
        let (column, row) = cell_position(i);
        let (x, y) = (ox + column * c, oy + row * c);
        let center = Point::new(x + c * 0.5, y + c * 0.5);
        if let Some(sheet) = sprites {
            let tile = |index: u8| Rect::new(f64::from(index) * 192.0, 0.0, 192.0, 192.0);
            let centered = |side: f64| Rect::new(-side * 0.5, -side * 0.5, side, side);
            let angle = if column < 2.0 {
                -std::f64::consts::FRAC_PI_2
            } else {
                std::f64::consts::FRAC_PI_2
            };
            d.transformed(
                Affine::rotate(Angle::rad(angle)).then(Affine::translate(center.x, center.y)),
                |d| {
                    d.image_region(sheet, tile(0), centered(c * 0.73));
                    let owner = g.cells[i];
                    if owner > 0 && g.countdown <= 0.0 {
                        let age = g.mole_age[i];
                        let rise =
                            (age / 0.16).clamp(0.0, 1.0) * ((1.4 - age) / 0.2).clamp(0.0, 1.0);
                        let side = c * rise * 0.96;
                        if side > 0.0 {
                            d.image_region(sheet, tile(owner), centered(side));
                        }
                    }
                },
            );
        }
        if hits[i] > 0.0 && !day_core::reduce_motion() {
            let t = 1.0 - hits[i];
            for particle in 0..7 {
                let angle = particle as f64 * std::f64::consts::TAU / 7.0 + i as f64;
                let dist = c * (0.17 + 0.4 * t);
                let radius = c * (0.03 + 0.03 * hits[i]);
                let x = center.x + angle.cos() * dist;
                let y = center.y + angle.sin() * dist + c * 0.18 * t * t;
                d.fill(
                    Shape::Ellipse(Rect::new(
                        x - radius,
                        y - radius,
                        radius * 2.0,
                        radius * 2.0,
                    )),
                    Color::rgba(0.74, 0.54, 0.37, hits[i]),
                );
            }
        }
        if cursor == Some(i) {
            d.stroke(
                Shape::RoundedRect(Rect::new(x + 2.0, y + 2.0, c - 4.0, c - 4.0), 8.0),
                chrome::GOLD,
                2.0,
            );
        }
    }
    if g.countdown > 0.0 {
        d.text(
            &res::str::party_count(g.countdown.ceil()).format(),
            Point::new(size.width / 2.0, size.height / 2.0),
            TextStyle {
                size: 80.0,
                color: Color::WHITE.into(),
                anchor: TextAnchor::CENTERED,
                font: chrome::canvas_font(FontWeight::Bold),
            },
        );
    }
}
fn overlay_card(ui: Rc<Ui>, overlay: Overlay) -> AnyPiece {
    if overlay == Overlay::Help {
        return chrome::instructions_card(
            res::str::nav_moles(),
            vec![
                Help::Para(res::str::party_moles_rules()),
                Help::Para(res::str::party_controls()),
            ],
            id!(moles_help_done),
            move || ui.show(ui.back.get()),
        )
        .id(id!(moles_help))
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
                id!(moles_resume),
                move || u.show(Overlay::None),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(moles_new_game),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(action(
                res::str::gk_settings(),
                chrome::SLATE,
                id!(moles_settings),
                Overlay::Settings,
            ));
            items.push(action(
                res::str::gk_instructions(),
                chrome::INDIGO,
                id!(moles_instructions),
                Overlay::Help,
            ));
            items.push(chrome::close_game_button(chrome::RED, id!(moles_quit)));
        }
        Overlay::Settings => {
            items.push(chrome::card_title(res::str::gk_settings(), Color::WHITE));
            items.push(chrome::setting_row(
                res::str::gk_sounds(),
                toggle(ui.sounds).id(id!(moles_sounds)).any(),
            ));
            items.push(chrome::setting_row(
                res::str::gk_vibrations(),
                toggle(ui.vibrations).id(id!(moles_vibrations)).any(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_done(),
                chrome::GREEN,
                id!(moles_done),
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
                id!(moles_play_again),
                move || u.start(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(moles_change_players),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(chrome::close_game_button(
                chrome::SLATE,
                id!(moles_result_quit),
            ));
        }
        _ => {}
    }
    party_ui::dialog(column(items).spacing(10.0).align(HAlign::Center))
        .id(id!(moles_menu))
        .any()
}
pub fn moles_preview() -> AnyPiece {
    let banner = gamekit::sprites::load(res::assets::moles::banner_png);
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
    res::assets::sounds::moles::hit_wav,
    res::assets::sounds::moles::miss_wav,
    res::assets::sounds::moles::dirt_wav,
];
