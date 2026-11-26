// SPDX-License-Identifier: GPL-2.0-or-later
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
mod model;
mod presentation;
mod rolling;
mod sound;
use model::{BALL_R, Game, HEIGHT, Vec2, WIDTH};
pub use sound::CLIPS as SOUNDS;
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
    sound_wait: Cell<[f64; 2]>,
    drops: RefCell<Vec<presentation::Drop>>,
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
        self.drops.borrow_mut().clear();
        self.sound_wait.set([0.0; 2]);
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
        let g = self.game.borrow();
        (!g.finished).then_some(g.turn)
    }
    fn status(&self) -> String {
        self.repaint.track();
        let g = self.game.borrow();
        if g.finished {
            self.outcome().format()
        } else if g.countdown > 0.0 {
            res::str::arena_countdown(g.countdown.ceil()).format()
        } else if g.moving || !self.drops.borrow().is_empty() {
            res::str::party_status_detail(res::str::PartyStatusDetail {
                status: res::str::party_turn(self.player(g.turn).format()).format(),
                detail: res::str::arena_rolling().format(),
            })
            .format()
        } else {
            res::str::party_turn(self.player(g.turn).format()).format()
        }
    }
    fn cue(&self, speed: f64) {
        sound::play(self.sounds.get_untracked(), sound::Contact::Cue, speed);
        chrome::haptic_pattern(
            self.vibrations.get_untracked(),
            party_ui::impact_pattern(speed),
        );
    }
    fn touches(&self, touches: &[Touch]) {
        if self.overlay.get_untracked() != Overlay::None || !self.drops.borrow().is_empty() {
            return;
        }
        let (ox, oy, scale) = layout(self.size.get());
        if scale <= 0.0 {
            return;
        }
        if self.game.borrow().moving || self.game.borrow().computer_turn() {
            self.aim.set(None);
            return;
        }
        for t in touches {
            let p = Vec2::new((t.location.x - ox) / scale, (t.location.y - oy) / scale);
            match t.phase {
                TouchPhase::Began
                    if self.aim.get().is_none()
                        && p.distance(self.game.borrow().balls[0].pos) < 65.0 =>
                {
                    self.aim.set(Some((t.id, p, p)))
                }
                TouchPhase::Moved | TouchPhase::Stationary => {
                    if let Some((id, start, _)) = self.aim.get()
                        && id == t.id
                    {
                        self.aim.set(Some((id, start, p)));
                    }
                }
                TouchPhase::Ended => {
                    if let Some((id, start, _)) = self.aim.get()
                        && id == t.id
                    {
                        self.aim.set(None);
                        if self
                            .game
                            .borrow_mut()
                            .shoot(Vec2::new(start.x - p.x, start.y - p.y))
                        {
                            self.cue(self.game.borrow().cue_impact);
                        }
                    }
                }
                TouchPhase::Cancelled if self.aim.get().is_some_and(|(id, _, _)| id == t.id) => {
                    self.aim.set(None);
                }
                _ => {}
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
pub fn pool_page(players: usize) -> AnyPiece {
    gamekit::sounds(sound::CLIPS);
    let settings = gamekit::restore::<chrome::GameSettings>("pool.settings").unwrap_or_default();
    let saved = gamekit::restore::<Game>("pool.v1")
        .filter(|g| g.valid() && g.humans == players.clamp(1, 2));
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
        sound_wait: Cell::new([0.0; 2]),
        drops: RefCell::new(Vec::new()),
    });
    gamekit::autosave("pool.v1", {
        let u = ui.clone();
        move || u.game.borrow().clone()
    });
    gamekit::on_background("pool.v1", {
        let u = ui.clone();
        move || u.pause()
    });
    Effect::new({
        let u = ui.clone();
        move || {
            gamekit::save(
                "pool.settings",
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
            res::str::nav_pool(),
            id!(pool_pause),
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
                    .id(id!(pool_status)),
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
                && (u.game.borrow().needs_clock()
                    || u.game.borrow().finished
                    || !u.drops.borrow().is_empty())
        },
        move |dt| {
            let delta = dt.as_secs_f64().min(0.1);
            // Pause freezes both the physics and the drop; landing feedback fires once on resume.
            let mut landed = None;
            step.drops.borrow_mut().retain_mut(|drop| {
                let old = drop.age;
                drop.age += delta;
                if old < presentation::LAND_AT
                    && drop.age >= presentation::LAND_AT
                    && (drop.event.ball == 0 || landed.is_none())
                {
                    landed = Some(if drop.event.ball == 0 {
                        chrome::cues::WARNING_BEAT
                    } else {
                        chrome::cues::MEDIUM_BEAT
                    });
                }
                drop.alive()
            });
            if let Some(pulse) = landed {
                chrome::haptic_pattern(step.vibrations.get_untracked(), pulse);
            }
            step.game.borrow_mut().tick(delta, step.controls.get());
            step.game.borrow_mut().record_result();
            let events = std::mem::take(&mut step.game.borrow_mut().pocket_events);
            if !events.is_empty() {
                // A shared drop sound keeps a simultaneous multi-pot from clipping the mix.
                chrome::sound(
                    step.sounds.get_untracked(),
                    &res::assets::sounds::pool::pocket_wav,
                    0.75,
                );
                step.drops
                    .borrow_mut()
                    .extend(events.into_iter().map(presentation::Drop::new));
            }
            let g = step.game.borrow();
            if g.cue_impact > 0.0 {
                step.cue(g.cue_impact);
            }
            let mut waits = step.sound_wait.get().map(|t| (t - delta).max(0.0));
            let mut pulse = 0.0_f64;
            for (i, (kind, speed)) in [
                (sound::Contact::Ball, g.ball_impact),
                (sound::Contact::Cushion, g.cushion_impact),
            ]
            .into_iter()
            .enumerate()
            {
                if speed > 8.0 && waits[i] <= 0.0 {
                    sound::play(step.sounds.get_untracked(), kind, speed);
                    pulse = pulse.max(speed);
                    waits[i] = if i == 0 { 0.035 } else { 0.055 };
                }
            }
            step.sound_wait.set(waits);
            if pulse > 30.0 && landed.is_none() {
                chrome::haptic_pattern(
                    step.vibrations.get_untracked(),
                    party_ui::impact_pattern(pulse),
                );
            }
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
            (g.count, g.scores, g.turn, g.finished, g.winners())
        },
        move |&(count, score, turn, finished, ref winners)| {
            row((0..count)
                .map(|i| {
                    let state = if finished {
                        if winners.len() == 1 && winners[0] == i {
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
                    column((
                        party_ui::score_card(
                            ui.player(i),
                            value,
                            res::str::party_count(88.0),
                            i,
                            state,
                            Some(score[i] as f64 / 7.0),
                        ),
                        label(if i == 0 {
                            res::str::arena_pool_solids()
                        } else {
                            res::str::arena_pool_stripes()
                        })
                        .font(Font::Caption)
                        .bold()
                        .color(Color::WHITE)
                        .align(TextAlign::Center),
                    ))
                    .spacing(3.0)
                    .grow(Dimension::Width)
                    .any()
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
            &d.drops.borrow(),
            d.aim.get().map(|(_, s, p)| Vec2::new(s.x - p.x, s.y - p.y)),
        );
        let g = d.game.borrow();
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
    .id(id!(pool_board))
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
fn draw_game(d: &mut Draw, size: Size, g: &Game, drops: &[presentation::Drop], aim: Option<Vec2>) {
    let (ox, oy, s) = layout(size);
    if s <= 0.0 {
        return;
    }
    let point = |p: Vec2| Point::new(ox + p.x * s, oy + p.y * s);
    let rect = |x: f64, y: f64, w: f64, h: f64| Rect::new(ox + x * s, oy + y * s, w * s, h * s);
    let bg = Color::hex(0x1A786C);
    d.fill(
        Shape::RoundedRect(rect(0.0, 0.0, WIDTH, HEIGHT), 12.0 * s),
        bg,
    );
    d.stroke(
        Shape::RoundedRect(rect(8.0, 8.0, WIDTH - 16.0, HEIGHT - 16.0), 12.0 * s),
        Color::hex(0x704D32),
        16.0 * s,
    );
    for y in [HEIGHT * 0.25, HEIGHT * 0.75] {
        for x in [8.0, WIDTH - 8.0] {
            d.fill(
                Shape::Ellipse(rect(x - 2.0, y - 2.0, 4.0, 4.0)),
                Color::hex(0xE6CDA1),
            );
        }
    }
    for player in 0..2 {
        for i in 0..7 {
            let x = 126.0 + i as f64 * 18.0;
            let y = if player == 0 { HEIGHT - 8.0 } else { 8.0 };
            d.fill(
                Shape::Ellipse(rect(x - 5.0, y - 5.0, 10.0, 10.0)),
                if i < g.scores[player] {
                    COLORS[player]
                } else {
                    Color::hex(0x3D302B)
                },
            );
        }
    }
    let reduced_motion = day_core::reduce_motion();
    d.clipped(
        Shape::RoundedRect(rect(0.0, 0.0, WIDTH, HEIGHT), 12.0 * s),
        |d| {
            d.transformed(Affine::translate(ox, oy), |d| {
                d.transformed(Affine::scale(s, s), |d| {
                    for i in 0..6 {
                        presentation::pocket(d, i, if reduced_motion { &[] } else { drops });
                    }
                })
            });
        },
    );
    if let Some(v) = aim {
        let cue = g.balls[0].pos;
        let len = v.x.hypot(v.y).max(1.0);
        let end = g.shot_guide(v);
        let dx = v.x / len;
        let dy = v.y / len;
        let pull = len.min(110.0) * 0.35 + 14.0;
        let tip = Vec2::new(cue.x - dx * pull, cue.y - dy * pull);
        let butt = Vec2::new(cue.x - dx * (pull + 86.0), cue.y - dy * (pull + 86.0));
        d.stroke(
            Shape::Line(point(tip), point(butt)),
            Color::hex(0x28202A),
            7.0 * s,
        );
        d.stroke(
            Shape::Line(point(tip), point(butt)),
            Color::hex(0xDFC492),
            4.5 * s,
        );
        d.stroke(
            Shape::Ellipse(rect(
                end.x - BALL_R,
                end.y - BALL_R,
                BALL_R * 2.0,
                BALL_R * 2.0,
            )),
            Color::rgba(1.0, 1.0, 1.0, 0.55),
            1.5 * s,
        );
        d.stroke(
            Shape::Line(point(cue), point(end)),
            Color::rgba(1.0, 1.0, 1.0, 0.65),
            2.0 * s,
        );
        let pull = (len / 150.0).min(1.0);
        d.fill(
            Shape::RoundedRect(rect(120.0, HEIGHT - 10.0, 120.0 * pull, 5.0), 2.0 * s),
            chrome::GOLD,
        );
    }
    d.clipped(
        Shape::RoundedRect(rect(0.0, 0.0, WIDTH, HEIGHT), 12.0 * s),
        |d| {
            d.transformed(Affine::translate(ox, oy), |d| {
                d.transformed(Affine::scale(s, s), |d| {
                    for (i, b) in g.balls.iter().enumerate() {
                        if !b.pocketed && !drops.iter().any(|drop| drop.event.ball == i) {
                            presentation::ball(
                                d,
                                b.pos,
                                BALL_R,
                                i,
                                if reduced_motion {
                                    rolling::Rotation::default()
                                } else {
                                    b.orientation
                                },
                                0.0,
                                true,
                            );
                        }
                    }
                    for drop in drops {
                        presentation::drop(d, drop, reduced_motion);
                    }
                })
            });
        },
    );
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
            res::str::nav_pool(),
            vec![
                Help::Para(res::str::arena_pool_rules()),
                Help::Para(res::str::arena_pool_controls()),
            ],
            id!(pool_help_done),
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
                id!(pool_resume),
                move || u.show(Overlay::None),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(pool_new_game),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(action(
                res::str::gk_settings(),
                chrome::SLATE,
                id!(pool_settings),
                Overlay::Settings,
            ));
            items.push(action(
                res::str::gk_instructions(),
                chrome::INDIGO,
                id!(pool_instructions),
                Overlay::Help,
            ));
            items.push(chrome::close_game_button(chrome::RED, id!(pool_quit)));
        }
        Overlay::Settings => {
            items.push(chrome::card_title(res::str::gk_settings(), Color::WHITE));
            items.push(chrome::setting_row(
                res::str::gk_sounds(),
                toggle(ui.sounds).id(id!(pool_sounds)).any(),
            ));
            items.push(chrome::setting_row(
                res::str::gk_vibrations(),
                toggle(ui.vibrations).id(id!(pool_vibrations)).any(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_done(),
                chrome::GREEN,
                id!(pool_done),
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
                id!(pool_play_again),
                move || u.start(),
            ));
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_new_game(),
                chrome::BLUE,
                id!(pool_change_players),
                move || {
                    u.show(Overlay::None);
                    u.start();
                },
            ));
            items.push(chrome::close_game_button(
                chrome::SLATE,
                id!(pool_result_quit),
            ));
        }
        _ => {}
    }
    party_ui::dialog(column(items).spacing(10.0).align(HAlign::Center))
        .id(id!(pool_menu))
        .any()
}
pub fn pool_preview() -> AnyPiece {
    let mut g = Game::new(2);
    g.countdown = 0.0;
    for b in &mut g.balls {
        b.pos.y += 70.0;
    }
    g.balls[0].pos = Vec2::new(145.0, 335.0);
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
                    |d| draw_game(d, full, &g, &[], None),
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
            sound_wait: Cell::new([0.0; 2]),
            drops: RefCell::new(Vec::new()),
        }
    }
    fn contact(id: u64, phase: TouchPhase, x: f64, y: f64) -> Touch {
        Touch::new(id, phase, Point::new(x, y))
    }
    #[test]
    fn pool_cancel_does_not_shoot_and_release_shoots_once() {
        let scope = Scope::detached();
        scope.enter(|| {
            let u = ui(2);
            u.touches(&[
                contact(1, TouchPhase::Began, 180.0, 410.0),
                contact(2, TouchPhase::Began, 180.0, 410.0),
            ]);
            assert_eq!(u.aim.get().expect("first contact owns aim").0, 1);
            u.touches(&[contact(1, TouchPhase::Moved, 180.0, 490.0)]);
            u.touches(&[contact(1, TouchPhase::Cancelled, 180.0, 490.0)]);
            assert!(!u.game.borrow().moving);
            u.touches(&[contact(3, TouchPhase::Began, 180.0, 410.0)]);
            u.touches(&[contact(3, TouchPhase::Ended, 180.0, 490.0)]);
            assert!(u.game.borrow().moving);
            assert!(u.game.borrow().balls[0].velocity.y < 0.0);
            assert!(u.aim.get().is_none());
        });
        scope.dispose();
    }
    #[test]
    fn scratch_drop_blocks_touch_until_respotted_ball_is_visible() {
        let scope = Scope::detached();
        scope.enter(|| {
            let u = ui(2);
            u.drops
                .borrow_mut()
                .push(presentation::Drop::new(model::PocketEvent {
                    ball: 0,
                    pocket: 0,
                    entry: model::pockets()[0],
                    orientation: rolling::Rotation::default(),
                    velocity: Vec2::default(),
                }));
            u.touches(&[contact(1, TouchPhase::Began, 180.0, 410.0)]);
            assert!(u.aim.get().is_none());
            u.drops.borrow_mut().clear();
            u.touches(&[contact(2, TouchPhase::Began, 180.0, 410.0)]);
            assert!(u.aim.get().is_some());
        });
        scope.dispose();
    }
}
