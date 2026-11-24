// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use day_localization_ui::LocalizedText;
use day_pieces::prelude::*;
use day_spec::Key;
use day_spec::{Touch, TouchPhase};
use game_resources::res;
use gamekit::chrome::{self, Help};
use gamekit::multiplayer::{self as party_ui, PlayerState};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
mod art;
mod model;
use model::{Game, HEIGHT, Vec2, WIDTH};
pub const SOUNDS: &[chrome::Sfx] = &[
    res::assets::sounds::snakes::hiss_wav,
    res::assets::sounds::snakes::growth_wav,
];
pub const SURFACE: Color = Color::hex(0x292139);
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
    game: RefCell<Game>,
    sprites: art::Sprites,
    overlay: Signal<Overlay>,
    back: Cell<Overlay>,
    repaint: Trigger,
    focus: Signal<bool>,
    size: Cell<Size>,
    players: Signal<usize>,
    sounds: Signal<bool>,
    vibrations: Signal<bool>,
    controls: Cell<[Option<Vec2>; 4]>,
    contacts: RefCell<HashMap<u64, usize>>,
    keys: Cell<[[bool; 4]; 4]>,
    aim: Cell<Option<(u64, Vec2, Vec2)>>,
    finish_age: Cell<f64>,
    sound_wait: Cell<f64>,
    eliminated_age: Cell<[f64; 4]>,
}
impl Ui {
    fn reset_input(&self) {
        self.controls.set([None; 4]);
        self.contacts.borrow_mut().clear();
        self.keys.set([[false; 4]; 4]);
        self.aim.set(None);
    }
    fn show(&self, overlay: Overlay) {
        self.reset_input();
        self.overlay.set(overlay);
        self.focus.set(overlay == Overlay::None);
        self.repaint.notify();
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
    fn start(&self) {
        let wins = if self.overlay.get_untracked() == Overlay::Result {
            self.game.borrow().round_wins
        } else {
            [0; 4]
        };
        *self.game.borrow_mut() = Game::new(self.players.get_untracked());
        self.game.borrow_mut().round_wins = wins;
        self.finish_age.set(0.0);
        self.eliminated_age.set([0.0; 4]);
        self.cue();
        self.show(Overlay::None);
    }
    fn player(&self, i: usize) -> LocalizedText {
        if i >= self.game.borrow().humans {
            res::str::party_computer()
        } else {
            res::str::party_player((i + 1) as f64)
        }
    }
    fn outcome(&self) -> LocalizedText {
        let winners = self.game.borrow().winners();
        if winners.len() == 1 {
            res::str::party_wins(self.player(winners[0]).format())
        } else {
            res::str::party_draw()
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
            res::str::party_everyone_plays().format()
        }
    }
    fn cue(&self) {
        chrome::sound(
            self.sounds.get_untracked(),
            &res::assets::sounds::shared::tap_wav,
            0.7,
        );
        chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::cues::MEDIUM_BEAT);
    }
    fn touches(&self, touches: &[Touch]) {
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        let (ox, oy, scale) = layout(self.size.get());
        if scale <= 0.0 {
            return;
        }
        let humans = self.game.borrow().humans;
        let pads = pad_centers();
        let mut controls = self.controls.get();
        for t in touches {
            let p = Vec2::new((t.location.x - ox) / scale, (t.location.y - oy) / scale);
            if t.phase == TouchPhase::Began
                && let Some(i) = (0..humans).find(|&i| {
                    p.distance(pads[i]) < 64.0 && !self.contacts.borrow().values().any(|&v| v == i)
                })
            {
                self.contacts.borrow_mut().insert(t.id, i);
                chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::cues::TICK_BEAT);
            }
            let player = self.contacts.borrow().get(&t.id).copied();
            if let Some(i) = player {
                if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                    self.contacts.borrow_mut().remove(&t.id);
                    controls[i] = None;
                } else {
                    controls[i] = Some(Vec2::new(p.x - pads[i].x, p.y - pads[i].y));
                }
            }
        }
        self.controls.set(controls);
        self.repaint.notify();
    }
    fn key(&self, key: Key, down: bool) {
        if down && key == Key::Escape {
            if self.overlay.get_untracked() == Overlay::Pause {
                self.show(Overlay::None);
            } else {
                self.pause();
            }
            return;
        }
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        let mapped = match key {
            Key::Character('w' | 'W') => (0, 0),
            Key::Character('s' | 'S') => (0, 1),
            Key::Character('a' | 'A') => (0, 2),
            Key::Character('d' | 'D') => (0, 3),
            Key::ArrowUp => (1, 0),
            Key::ArrowDown => (1, 1),
            Key::ArrowLeft => (1, 2),
            Key::ArrowRight => (1, 3),
            Key::Character('i' | 'I') => (2, 0),
            Key::Character('k' | 'K') => (2, 1),
            Key::Character('j' | 'J') => (2, 2),
            Key::Character('l' | 'L') => (2, 3),
            Key::Character('t' | 'T') => (3, 0),
            Key::Character('g' | 'G') => (3, 1),
            Key::Character('f' | 'F') => (3, 2),
            Key::Character('h' | 'H') => (3, 3),
            _ => return,
        };
        let (i, d) = mapped;
        if i >= self.game.borrow().humans {
            return;
        }
        let mut keys = self.keys.get();
        keys[i][d] = down;
        self.keys.set(keys);
        let mut controls = self.controls.get();
        let x = i32::from(keys[i][3]) - i32::from(keys[i][2]);
        let y = i32::from(keys[i][1]) - i32::from(keys[i][0]);
        controls[i] = if x == 0 && y == 0 {
            None
        } else {
            Some(Vec2::new(x as f64, y as f64))
        };
        self.controls.set(controls);
        self.repaint.notify();
    }
}
pub fn snakes_page(players: usize) -> AnyPiece {
    gamekit::sounds(SOUNDS);
    let settings = gamekit::restore::<chrome::GameSettings>("snakes.settings").unwrap_or_default();
    let saved = gamekit::restore::<Game>("snakes.v1")
        .filter(|g| g.valid() && g.humans == players.clamp(1, 4));
    let initial = if !settings.instructions_shown {
        Overlay::Help
    } else if saved.as_ref().is_some_and(|g| g.finished) {
        Overlay::Result
    } else {
        Overlay::Pause
    };
    let fresh = saved.is_none();
    let initial = if fresh && settings.instructions_shown {
        Overlay::None
    } else {
        initial
    };
    let after_help = if fresh {
        Overlay::None
    } else if saved.as_ref().is_some_and(|g| g.finished) {
        Overlay::Result
    } else {
        Overlay::Pause
    };
    let mut game = saved.unwrap_or_else(|| Game::new(players));
    game.record_result();
    let ui = Rc::new(Ui {
        sprites: art::Sprites::load(),
        players: Signal::new(game.humans),
        game: RefCell::new(game),
        overlay: Signal::new(initial),
        back: Cell::new(after_help),
        repaint: Trigger::new(),
        focus: Signal::new(false),
        size: Cell::new(Size::ZERO),
        sounds: Signal::new(settings.sounds),
        vibrations: Signal::new(settings.vibrations),
        controls: Cell::new([None; 4]),
        contacts: RefCell::new(HashMap::new()),
        keys: Cell::new([[false; 4]; 4]),
        aim: Cell::new(None),
        finish_age: Cell::new(0.0),
        sound_wait: Cell::new(0.0),
        eliminated_age: Cell::new([0.0; 4]),
    });
    gamekit::autosave("snakes.v1", {
        let u = ui.clone();
        move || u.game.borrow().clone()
    });
    gamekit::on_background("snakes.v1", {
        let u = ui.clone();
        move || u.pause()
    });
    Effect::new({
        let u = ui.clone();
        move || {
            gamekit::save(
                "snakes.settings",
                &chrome::GameSettings {
                    sounds: u.sounds.get(),
                    vibrations: u.vibrations.get(),
                    instructions_shown: true,
                },
            )
        }
    });
    let u = ui.clone();
    let s = ui.clone();
    let title_tint = ui.clone();
    let status_tint = ui.clone();
    let content = chrome::game_frame(
        party_ui::header(
            res::str::nav_snakes(),
            id!(snakes_pause),
            move || u.pause(),
            move || title_tint.active_player(),
        ),
        Some(
            column((
                label(move || s.status())
                    .font(Font::Subheadline)
                    .bold()
                    .color(move || {
                        status_tint
                            .active_player()
                            .map_or(Color::WHITE, |i| COLORS[i])
                    })
                    .align(TextAlign::Center)
                    .id(id!(snakes_status)),
                scores(ui.clone()),
            ))
            .spacing(4.0)
            .padding(8.0)
            .grow(Dimension::Width)
            .any(),
        ),
        board(ui.clone()),
        None,
    );
    let u = ui.clone();
    let step = ui.clone();
    let clock = gamekit::animation::clock(
        move || {
            u.repaint.track();
            u.overlay.get() == Overlay::None
                && (u.game.borrow().needs_clock() || u.game.borrow().finished)
        },
        move |dt| {
            let delta = dt.as_secs_f64().min(0.1);
            let countdown = step.game.borrow().countdown.ceil();
            let growth = step.game.borrow().growth;
            let alive = step.game.borrow().riders.iter().filter(|r| r.alive).count();
            step.game
                .borrow_mut()
                .tick(dt.as_secs_f64(), step.controls.get());
            step.game.borrow_mut().record_result();
            step.sound_wait
                .set((step.sound_wait.get() - delta).max(0.0));
            let g = step.game.borrow();
            if alive != g.riders.iter().filter(|r| r.alive).count() {
                chrome::haptic_pattern(step.vibrations.get_untracked(), chrome::LETDOWN);
            }
            if growth != g.growth {
                chrome::haptic_pattern(step.vibrations.get_untracked(), chrome::cues::TICK_BEAT);
            }
            if countdown != g.countdown.ceil() {
                chrome::haptic_pattern(step.vibrations.get_untracked(), chrome::cues::LIGHT_BEAT);
            }
            let clip = if alive != g.riders.iter().filter(|r| r.alive).count() {
                Some(res::assets::sounds::snakes::hiss_wav)
            } else if countdown != g.countdown.ceil() {
                Some(res::assets::sounds::shared::tick_wav)
            } else if growth != g.growth && step.sound_wait.get() <= 0.0 {
                Some(res::assets::sounds::snakes::growth_wav)
            } else {
                None
            };
            if let Some(clip) = clip {
                chrome::sound(step.sounds.get_untracked(), &clip, 0.55);
                step.sound_wait.set(0.09);
            }
            let mut ages = step.eliminated_age.get();
            for (i, r) in g.riders.iter().enumerate() {
                if !r.alive {
                    ages[i] = (ages[i] + delta).min(1.0);
                }
            }
            step.eliminated_age.set(ages);
            if g.finished {
                let age = step.finish_age.get() + delta;
                step.finish_age.set(age);
                if age >= 1.3 {
                    chrome::sound(
                        step.sounds.get_untracked(),
                        &res::assets::sounds::shared::success_wav,
                        0.75,
                    );
                    chrome::haptic_pattern(
                        step.vibrations.get_untracked(),
                        chrome::cues::SUCCESS_BEAT,
                    );
                    step.show(Overlay::Result);
                }
            }
            drop(g);
            step.repaint.notify();
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
            move |o| (*o != Overlay::None).then(|| card(ui.clone(), *o)),
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
            (
                g.count,
                g.scores,
                g.riders.iter().map(|r| r.alive).collect::<Vec<_>>(),
                g.finished,
                g.winners(),
            )
        },
        move |&(count, _score, ref alive, finished, ref winners)| {
            row((0..count)
                .map(|i| {
                    let state = if finished {
                        if winners.len() == 1 && winners[0] == i {
                            PlayerState::Winner
                        } else {
                            PlayerState::Finished
                        }
                    } else if alive[i] {
                        PlayerState::Playing
                    } else {
                        PlayerState::Out
                    };
                    let value = if alive[i] {
                        res::str::arena_alive()
                    } else {
                        res::str::arena_out()
                    };
                    party_ui::score_card(ui.player(i), value.clone(), value.clone(), i, state, None)
                })
                .collect::<Vec<_>>())
            .spacing(6.0)
            .grow(Dimension::Width)
            .any()
        },
    )
    .any()
}
fn board(ui: Rc<Ui>) -> AnyPiece {
    let d = ui.clone();
    let t = ui.clone();
    let k = ui.clone();
    let up = ui.clone();
    let a = ui.clone();
    canvas(move |draw, size| {
        d.repaint.track();
        d.size.set(size);
        draw_game(
            draw,
            size,
            &d.game.borrow(),
            &d.sprites,
            d.controls.get(),
            d.aim.get().map(|(_, s, p)| Vec2::new(s.x - p.x, s.y - p.y)),
        );
        let g = d.game.borrow();
        let (ox, oy, scale) = layout(size);
        for (i, r) in g.riders.iter().enumerate() {
            let age = d.eliminated_age.get()[i];
            if !r.alive && age < 1.0 {
                for j in 0..10 {
                    let a = j as f64 * std::f64::consts::TAU / 10.0;
                    let p = Point::new(
                        ox + (r.pos.x + a.cos() * age * 60.0) * scale,
                        oy + (r.pos.y + a.sin() * age * 60.0) * scale,
                    );
                    let radius = (1.0 - age) * 4.0 * scale;
                    draw.fill(
                        Shape::Ellipse(Rect::new(
                            p.x - radius,
                            p.y - radius,
                            radius * 2.0,
                            radius * 2.0,
                        )),
                        COLORS[i],
                    );
                }
            }
        }
        if g.finished {
            party_art::confetti(draw, size, d.finish_age.get());
        }
    })
    .on_touches(move |touches| t.touches(touches))
    .on_key(move |e| k.key(e.key, true))
    .on_key_up(move |e| up.key(e.key, false))
    .focused(ui.focus)
    .a11y(move |a11y| {
        a11y.label(res::str::arena_board())
            .value(move || a.status())
    })
    .id(id!(snakes_board))
    .grow(Dimension::Both)
    .any()
}
fn layout(size: Size) -> (f64, f64, f64) {
    let s = (size.width / WIDTH).min(size.height / HEIGHT).max(0.0);
    (
        (size.width - WIDTH * s) / 2.0,
        (size.height - HEIGHT * s) / 2.0,
        s,
    )
}
fn pad_centers() -> [Vec2; 4] {
    [
        Vec2::new(306.0, 506.0),
        Vec2::new(54.0, 54.0),
        Vec2::new(306.0, 54.0),
        Vec2::new(54.0, 506.0),
    ]
}
fn draw_game(
    d: &mut Draw,
    size: Size,
    g: &Game,
    sprites: &art::Sprites,
    controls: [Option<Vec2>; 4],
    _aim: Option<Vec2>,
) {
    let (ox, oy, s) = layout(size);
    if s <= 0.0 {
        return;
    }
    let point = |p: Vec2| Point::new(ox + p.x * s, oy + p.y * s);
    let rect = |x: f64, y: f64, w: f64, h: f64| Rect::new(ox + x * s, oy + y * s, w * s, h * s);
    let bg = Color::hex(0x594266);
    d.fill(
        Shape::RoundedRect(rect(0.0, 0.0, WIDTH, HEIGHT), 12.0 * s),
        bg,
    );
    d.clipped(Shape::Rect(rect(0.0, 0.0, WIDTH, HEIGHT)), |d| {
        d.transformed(Affine::scale(s, s).then(Affine::translate(ox, oy)), |d| {
            art::snakes(d, g, sprites, s);
        });
    });
    for (i, p) in pad_centers().iter().enumerate().take(g.count) {
        let color = if g.riders[i].alive {
            COLORS[i]
        } else {
            Color::hex(0x707780)
        };
        let offset = controls[i].unwrap_or_default();
        party_art::joystick(d, point(*p), s, color, Point::new(offset.x, offset.y));
        let text = if i < g.humans {
            res::str::party_count((i + 1) as f64).format()
        } else {
            res::str::arena_ai().format()
        };
        d.text(
            &text,
            point(*p),
            TextStyle {
                size: 14.0 * s,
                color: color.into(),
                anchor: TextAnchor::CENTERED,
                font: chrome::canvas_font(FontWeight::Bold),
            },
        );
    }
    if g.countdown > 0.0 {
        let pulse = 1.0 + (g.countdown.fract()) * 0.15;
        d.text(
            &res::str::party_count(g.countdown.ceil()).format(),
            point(Vec2::new(WIDTH / 2.0, HEIGHT / 2.0)),
            TextStyle {
                size: 78.0 * s * pulse,
                color: Color::WHITE.into(),
                anchor: TextAnchor::CENTERED,
                font: chrome::canvas_font(FontWeight::Bold),
            },
        );
    }
}
fn card(ui: Rc<Ui>, o: Overlay) -> AnyPiece {
    if o == Overlay::Help {
        return chrome::instructions_card(
            res::str::nav_snakes(),
            vec![
                Help::Para(res::str::arena_snakes_rules()),
                Help::Para(res::str::arena_controls()),
            ],
            id!(snakes_help_done),
            move || ui.show(ui.back.get()),
        )
        .any();
    }
    let mut items = Vec::new();
    let action = |title, tint, id, next| {
        let u = ui.clone();
        chrome::menu_button(title, tint, id, move || u.push(next))
    };
    match o {
        Overlay::Pause => {
            items.push(chrome::card_title(res::str::gk_paused(), Color::WHITE));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_resume(),
                chrome::GREEN,
                id!(snakes_resume),
                move || u.show(Overlay::None),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(snakes_new_game),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(action(
                res::str::gk_settings(),
                chrome::SLATE,
                id!(snakes_settings),
                Overlay::Settings,
            ));
            items.push(action(
                res::str::gk_instructions(),
                chrome::INDIGO,
                id!(snakes_instructions),
                Overlay::Help,
            ));
            items.push(chrome::close_game_button(chrome::RED, id!(snakes_quit)));
        }
        Overlay::Settings => {
            items.push(chrome::card_title(res::str::gk_settings(), Color::WHITE));
            items.push(chrome::setting_row(
                res::str::gk_sounds(),
                toggle(ui.sounds).id(id!(snakes_sounds)).any(),
            ));
            items.push(chrome::setting_row(
                res::str::gk_vibrations(),
                toggle(ui.vibrations).id(id!(snakes_vibrations)).any(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_done(),
                chrome::GREEN,
                id!(snakes_done),
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
                row((0..ui.game.borrow().count)
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
                id!(snakes_play_again),
                move || u.start(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(snakes_change_players),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(chrome::close_game_button(
                chrome::SLATE,
                id!(snakes_result_quit),
            ));
        }
        _ => {}
    }
    party_ui::dialog(column(items).spacing(10.0).align(HAlign::Center))
        .id(id!(snakes_menu))
        .any()
}
pub fn snakes_preview() -> AnyPiece {
    let sprites = art::Sprites::load();
    let mut g = Game::new(2);
    g.countdown = 0.0;
    for (i, r) in g.riders.iter_mut().enumerate() {
        let side = if i == 0 { 1.0 } else { -1.0 };
        r.pos = Vec2::new(180.0 + side * 32.0, 280.0 - side * 35.0);
        r.angle = if i == 0 { std::f64::consts::PI } else { 0.0 };
        r.body = (0..45)
            .map(|j| {
                let t = j as f64 / 44.0;
                Vec2::new(
                    r.pos.x + side * (t * 70.0),
                    r.pos.y + side * (t * 4.0).sin() * 30.0,
                )
            })
            .collect();
    }
    canvas(move |d, size| {
        let width = size.width * 1.8;
        let full = Size::new(width, width * HEIGHT / WIDTH);
        d.clipped(
            Shape::RoundedRect(Rect::new(0.0, 0.0, size.width, size.height), 12.0),
            |d| {
                d.transformed(
                    Affine::translate(
                        (size.width - full.width) / 2.0,
                        (size.height - full.height) / 2.0,
                    ),
                    |d| draw_game(d, full, &g, &sprites, [None; 4], None),
                );
            },
        );
    })
    .grow(Dimension::Both)
    .any()
}
#[cfg(test)]
mod input_tests {
    use super::*;
    fn ui(humans: usize) -> Ui {
        Ui {
            game: RefCell::new(Game::new(humans)),
            sprites: art::Sprites::load(),
            overlay: Signal::new(Overlay::None),
            back: Cell::new(Overlay::None),
            repaint: Trigger::new(),
            focus: Signal::new(true),
            size: Cell::new(Size::new(WIDTH, HEIGHT)),
            players: Signal::new(humans),
            sounds: Signal::new(false),
            vibrations: Signal::new(false),
            controls: Cell::new([None; 4]),
            contacts: RefCell::new(HashMap::new()),
            keys: Cell::new([[false; 4]; 4]),
            aim: Cell::new(None),
            finish_age: Cell::new(0.0),
            sound_wait: Cell::new(0.0),
            eliminated_age: Cell::new([0.0; 4]),
        }
    }
    fn contact(id: u64, phase: TouchPhase, x: f64, y: f64) -> Touch {
        Touch::new(id, phase, Point::new(x, y))
    }
    #[test]
    fn independent_fingers_release_cancel_and_pause() {
        let scope = Scope::detached();
        scope.enter(|| {
            let u = ui(4);
            u.touches(&[
                contact(11, TouchPhase::Began, 306.0, 506.0),
                contact(22, TouchPhase::Began, 54.0, 54.0),
            ]);
            u.touches(&[
                contact(11, TouchPhase::Moved, 275.0, 506.0),
                contact(22, TouchPhase::Moved, 54.0, 85.0),
            ]);
            assert_eq!(u.controls.get()[0].expect("first player steers").x, -31.0);
            assert_eq!(u.controls.get()[1].expect("second player steers").y, 31.0);
            u.touches(&[
                contact(11, TouchPhase::Ended, 275.0, 506.0),
                contact(22, TouchPhase::Stationary, 54.0, 85.0),
            ]);
            assert!(u.controls.get()[0].is_none());
            assert!(u.controls.get()[1].is_some());
            u.touches(&[contact(22, TouchPhase::Cancelled, 54.0, 85.0)]);
            assert!(u.controls.get().iter().all(Option::is_none));
            u.key(Key::Character('d'), true);
            assert!(u.controls.get()[0].is_some());
            u.pause();
            assert!(u.controls.get().iter().all(Option::is_none));
            u.touches(&[contact(11, TouchPhase::Began, 306.0, 506.0)]);
            assert!(u.contacts.borrow().is_empty());
            u.show(Overlay::None);
            assert!(u.controls.get().iter().all(Option::is_none));
        });
        scope.dispose();
    }
}
