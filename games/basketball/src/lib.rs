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
    rc::Rc,
};
mod art;
mod model;
use model::{Difficulty, Game, HEIGHT, Vec2, WIDTH};
pub const SURFACE: Color = Color::hex(0x34393F);
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
    sprite: Signal<Option<day_core::Bitmap>>,
    drag: Cell<Option<(u64, Vec2, Vec2, f64)>>,
    samples: RefCell<Vec<(Vec2, f64)>>,
    overlay: Signal<Overlay>,
    back: Cell<Overlay>,
    repaint: Trigger,
    focus: Signal<bool>,
    size: Cell<Size>,
    players: Signal<usize>,
    sounds: Signal<bool>,
    vibrations: Signal<bool>,
    difficulty: Signal<Difficulty>,
    finish_age: Cell<f64>,
    sound_wait: Cell<f64>,
}
impl Ui {
    fn reset_input(&self) {
        self.drag.set(None);
        self.samples.borrow_mut().clear();
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
        self.game
            .borrow_mut()
            .set_difficulty(self.difficulty.get_untracked());
        self.finish_age.set(0.0);
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
    fn status(&self) -> String {
        self.repaint.track();
        let g = self.game.borrow();
        if g.finished {
            self.outcome().format()
        } else if g.countdown > 0.0 {
            res::str::arena_countdown(g.countdown.ceil()).format()
        } else {
            res::str::party_turn(self.player(g.turn).format()).format()
        }
    }
    fn cue(&self) {
        chrome::sound(
            self.sounds.get_untracked(),
            &res::assets::sounds::basketball::throw_wav,
            0.7,
        );
        chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::cues::MEDIUM_BEAT);
    }
    fn touches(&self, touches: &[Touch]) {
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        let (ox, oy, scale) = art::layout(self.size.get());
        if scale <= 0.0 {
            return;
        }
        for t in touches {
            let g = self.game.borrow();
            let p = g.to_court(Vec2::new(
                (t.location.x - ox) / scale,
                (t.location.y - oy) / scale,
            ));
            let (ball, radius_scale) = model::project(g.ready_position());
            if t.phase == TouchPhase::Began
                && self.drag.get().is_none()
                && g.can_shoot()
                && p.distance(ball) < model::RADIUS * radius_scale + 18.0
            {
                let now = day_core::frame::now_secs();
                self.drag.set(Some((t.id, p, p, now)));
                *self.samples.borrow_mut() = vec![(p, now)];
                chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::cues::TICK_BEAT);
            }
            drop(g);
            if self.drag.get().is_some_and(|d| d.0 == t.id) {
                let now = day_core::frame::now_secs();
                let mut samples = self.samples.borrow_mut();
                samples.push((p, now));
                while samples.len() > 2 && samples[1].1 < now - 0.1 {
                    samples.remove(0);
                }
            }
            if let Some((id, start, _, when)) = self.drag.get()
                && id == t.id
            {
                match t.phase {
                    TouchPhase::Cancelled => self.drag.set(None),
                    TouchPhase::Ended => {
                        self.drag.set(None);
                        let now = day_core::frame::now_secs();
                        let samples = self.samples.borrow();
                        let (recent, at) = samples.first().copied().unwrap_or((start, when));
                        let speed = (recent.y - p.y) / (now - at).max(0.016);
                        // A hold is allowed; only the final flick determines power. Stopping
                        // before lifting drops the ball rather than turning a drag into a shot.
                        if speed > 100.0 {
                            let duration = (start.y - p.y).max(18.0) / speed;
                            self.game.borrow_mut().shoot(start, p, duration.max(0.016));
                        }
                    }
                    _ => self.drag.set(Some((id, start, p, when))),
                }
            }
        }
        self.repaint.notify();
    }
    fn key(&self, key: Key, down: bool) {
        if down && key == Key::Escape {
            if self.overlay.get_untracked() == Overlay::Pause {
                self.show(Overlay::None);
            } else {
                self.pause();
            }
        }
    }
}
pub fn basketball_page(players: usize) -> AnyPiece {
    gamekit::sounds(SOUNDS);
    let settings =
        gamekit::restore::<chrome::GameSettings>("basketball.settings").unwrap_or_default();
    let saved = gamekit::restore::<Game>("basketball.v1")
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
    let preferred = gamekit::restore::<Difficulty>("basketball.difficulty").unwrap_or_default();
    let mut game = saved.unwrap_or_else(|| {
        let mut g = Game::new(players);
        g.set_difficulty(preferred);
        g
    });
    let difficulty = game.difficulty;
    game.record_result();
    let ui = Rc::new(Ui {
        sprite: gamekit::sprites::load(res::assets::basketball::ball_png),
        drag: Cell::new(None),
        samples: RefCell::new(Vec::new()),
        players: Signal::new(game.humans),
        game: RefCell::new(game),
        overlay: Signal::new(initial),
        back: Cell::new(after_help),
        repaint: Trigger::new(),
        focus: Signal::new(false),
        size: Cell::new(Size::ZERO),
        sounds: Signal::new(settings.sounds),
        vibrations: Signal::new(settings.vibrations),
        difficulty: Signal::new(difficulty),
        finish_age: Cell::new(0.0),
        sound_wait: Cell::new(0.0),
    });
    gamekit::autosave("basketball.v1", {
        let u = ui.clone();
        move || u.game.borrow().clone()
    });
    gamekit::on_background("basketball.v1", {
        let u = ui.clone();
        move || u.pause()
    });
    Effect::new({
        let u = ui.clone();
        move || {
            let selected = u.difficulty.get();
            if u.game.borrow().difficulty != selected {
                u.reset_input();
                u.game.borrow_mut().set_difficulty(selected);
                u.repaint.notify();
            }
            gamekit::save("basketball.difficulty", &selected);
            gamekit::save(
                "basketball.settings",
                &chrome::GameSettings {
                    sounds: u.sounds.get(),
                    vibrations: u.vibrations.get(),
                    instructions_shown: true,
                },
            )
        }
    });
    let u = ui.clone();
    let content = board(ui.clone()).overlay_aligned(
        Alignment::Trailing,
        canvas(|d, size| {
            let c = Point::new(size.width * 0.5, size.height * 0.5);
            d.fill(
                Shape::Ellipse(Rect::new(c.x - 19.0, c.y - 19.0, 38.0, 38.0)),
                Color::WHITE,
            );
            chrome::draw_cross_glyph(d, c, 21.0, Color::hex(0x9DBCD0));
        })
        .on_tap(move || u.pause())
        .a11y(|a| a.label(res::str::gk_paused()).role(Role::Button))
        .id(id!(basketball_pause))
        .frame(44.0, 44.0)
        .padding(12.0),
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
            step.game.borrow_mut().tick(dt.as_secs_f64());
            step.sound_wait
                .set((step.sound_wait.get() - delta).max(0.0));
            let events = std::mem::take(&mut step.game.borrow_mut().events);
            for event in events {
                use model::Event;
                match event {
                    Event::Throw => {
                        chrome::sound(
                            step.sounds.get_untracked(),
                            &res::assets::sounds::basketball::throw_wav,
                            0.3,
                        );
                        chrome::sound_after(
                            step.sounds.get_untracked(),
                            &res::assets::sounds::basketball::spawn_wav,
                            0.18,
                            90,
                        );
                        chrome::haptic_pattern(
                            step.vibrations.get_untracked(),
                            chrome::cues::LIGHT_BEAT,
                        );
                    }
                    Event::Bounce(force) | Event::Rim(force) if step.sound_wait.get() <= 0.0 => {
                        let clips = [
                            res::assets::sounds::basketball::bounce_0_wav,
                            res::assets::sounds::basketball::bounce_1_wav,
                            res::assets::sounds::basketball::bounce_2_wav,
                            res::assets::sounds::basketball::bounce_3_wav,
                            res::assets::sounds::basketball::bounce_4_wav,
                            res::assets::sounds::basketball::bounce_5_wav,
                            res::assets::sounds::basketball::bounce_6_wav,
                        ];
                        let clip = if matches!(event, Event::Rim(_)) {
                            res::assets::sounds::basketball::rim_wav
                        } else {
                            clips[(step.game.borrow().elapsed * 31.0) as usize % clips.len()]
                                .clone()
                        };
                        chrome::sound(
                            step.sounds.get_untracked(),
                            &clip,
                            (force / 35.0).clamp(0.12, 0.85) as f32,
                        );
                        chrome::haptic_pattern(
                            step.vibrations.get_untracked(),
                            if force > 15.0 {
                                chrome::cues::MEDIUM_BEAT
                            } else {
                                chrome::cues::TICK_BEAT
                            },
                        );
                        step.sound_wait.set(0.075);
                    }
                    Event::Score => {
                        chrome::sound(
                            step.sounds.get_untracked(),
                            &res::assets::sounds::basketball::score_wav,
                            0.65,
                        );
                        chrome::haptic_pattern(
                            step.vibrations.get_untracked(),
                            chrome::cues::SUCCESS_BEAT,
                        );
                    }
                    Event::Turn | Event::End => {
                        step.drag.set(None);
                        chrome::sound(
                            step.sounds.get_untracked(),
                            &res::assets::sounds::basketball::turn_wav,
                            0.7,
                        );
                        chrome::haptic_pattern(
                            step.vibrations.get_untracked(),
                            chrome::cues::MEDIUM_BEAT,
                        );
                    }
                    _ => {}
                }
            }
            let g = step.game.borrow();
            if g.finished {
                let age = step.finish_age.get() + delta;
                step.finish_age.set(age);
                if age >= 1.3 {
                    chrome::sound(
                        step.sounds.get_untracked(),
                        &res::assets::sounds::basketball::finish_wav,
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
            (g.count, g.scores, g.turn, g.finished)
        },
        move |&(count, score, turn, finished)| {
            row((0..count)
                .map(|i| {
                    let state = if finished {
                        if ui.game.borrow().winners() == vec![i] {
                            PlayerState::Winner
                        } else {
                            PlayerState::Finished
                        }
                    } else if i == turn {
                        PlayerState::Turn
                    } else {
                        PlayerState::Waiting
                    };
                    let value = res::str::party_count(score[i] as f64);
                    party_ui::score_card(ui.player(i), value.clone(), value, i, state, None)
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
    let a = ui.clone();
    canvas(move |draw, size| {
        d.repaint.track();
        d.size.set(size);
        let sprite = d.sprite.get();
        art::draw(
            draw,
            size,
            &d.game.borrow(),
            sprite.as_ref(),
            d.drag.get().map(|(_, _, p, _)| p),
        );
        if d.game.borrow().finished {
            party_art::confetti(draw, size, d.finish_age.get());
        }
    })
    .on_touches(move |touches| t.touches(touches))
    .on_key(move |e| k.key(e.key, true))
    .focused(ui.focus)
    .a11y(move |a11y| {
        a11y.label(res::str::basketball_board())
            .value(move || a.status())
    })
    .id(id!(basketball_board))
    .grow(Dimension::Both)
    .any()
}
fn card(ui: Rc<Ui>, o: Overlay) -> AnyPiece {
    if o == Overlay::Help {
        return chrome::instructions_card(
            res::str::nav_basketball(),
            vec![Help::Para(res::str::basketball_rules())],
            id!(basketball_help_done),
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
                id!(basketball_resume),
                move || u.show(Overlay::None),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(basketball_new_game),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(action(
                res::str::gk_settings(),
                chrome::SLATE,
                id!(basketball_settings),
                Overlay::Settings,
            ));
            items.push(action(
                res::str::gk_instructions(),
                chrome::INDIGO,
                id!(basketball_instructions),
                Overlay::Help,
            ));
            items.push(chrome::close_game_button(chrome::RED, id!(basketball_quit)));
        }
        Overlay::Settings => {
            items.push(chrome::card_title(res::str::gk_settings(), Color::WHITE));
            items.push(chrome::setting_row(
                res::str::tf_difficulty(),
                picker(
                    ui.difficulty,
                    vec![
                        (Difficulty::Easy, res::str::fs_easy()),
                        (Difficulty::Medium, res::str::fs_medium()),
                        (Difficulty::Hard, res::str::fs_hard()),
                    ],
                )
                .id(id!(basketball_difficulty))
                .any(),
            ));
            items.push(chrome::setting_row(
                res::str::gk_sounds(),
                toggle(ui.sounds).id(id!(basketball_sounds)).any(),
            ));
            items.push(chrome::setting_row(
                res::str::gk_vibrations(),
                toggle(ui.vibrations).id(id!(basketball_vibrations)).any(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_done(),
                chrome::GREEN,
                id!(basketball_done),
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
                id!(basketball_play_again),
                move || u.start(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(basketball_change_players),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(chrome::close_game_button(
                chrome::SLATE,
                id!(basketball_result_quit),
            ));
        }
        _ => {}
    }
    party_ui::dialog(column(items).spacing(10.0).align(HAlign::Center))
        .id(id!(basketball_menu))
        .any()
}
pub fn basketball_preview() -> AnyPiece {
    let sprite = gamekit::sprites::load(res::assets::basketball::ball_png);
    let mut game = Game::new(2);
    game.ready_age = 1.0;
    canvas(move |d, size| {
        let s = size.width / 240.0;
        d.clipped(
            Shape::RoundedRect(Rect::new(0.0, 0.0, size.width, size.height), 12.0),
            |d| {
                d.transformed(
                    Affine::scale(s, s).then(Affine::translate(-60.0 * s, -120.0 * s)),
                    |d| {
                        art::draw(
                            d,
                            Size::new(WIDTH, HEIGHT),
                            &game,
                            sprite.get().as_ref(),
                            Some(Vec2::new(215.0, 220.0)),
                        )
                    },
                );
            },
        );
    })
    .grow(Dimension::Both)
    .any()
}

pub const SOUNDS: &[chrome::Sfx] = &[
    res::assets::sounds::basketball::bounce_0_wav,
    res::assets::sounds::basketball::bounce_1_wav,
    res::assets::sounds::basketball::bounce_2_wav,
    res::assets::sounds::basketball::bounce_3_wav,
    res::assets::sounds::basketball::bounce_4_wav,
    res::assets::sounds::basketball::bounce_5_wav,
    res::assets::sounds::basketball::bounce_6_wav,
    res::assets::sounds::basketball::rim_wav,
    res::assets::sounds::basketball::throw_wav,
    res::assets::sounds::basketball::spawn_wav,
    res::assets::sounds::basketball::score_wav,
    res::assets::sounds::basketball::turn_wav,
    res::assets::sounds::basketball::finish_wav,
];
