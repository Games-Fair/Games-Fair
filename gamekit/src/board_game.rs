// SPDX-License-Identifier: GPL-2.0-or-later
//! Shared lifecycle and menus for canvas games; mechanics and artwork stay in each game crate.
use crate::{
    art::COLORS,
    chrome::{self, Help},
    multiplayer,
};
use day_core::Bitmap;
use day_localization_ui::LocalizedText;
use day_pieces::prelude::*;
use day_spec::{AssetName, Touch, TouchPhase};
use day_spec::{LineCap, StrokeStyle};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy, Debug)]
pub enum Feedback {
    Tap,
    Flick(f64),
    Impact(f64),
    Score,
    OwnGoal,
    Turn,
    Wall(f64),
    Disc(f64),
    Sweep,
    Miss,
}
pub struct GameOption {
    pub title: LocalizedText,
    pub choices: Vec<LocalizedText>,
    pub selected: usize,
}
pub trait BoardGame: Clone + Serialize + DeserializeOwned + 'static {
    fn new(players: usize, seed: u64) -> Self;
    fn restart(&self, players: usize, seed: u64) -> Self {
        Self::new(players, seed)
    }
    fn options(&self) -> Vec<GameOption> {
        vec![]
    }
    fn set_option(&mut self, _index: usize, _value: usize) {}
    fn commands(&self) -> Vec<(LocalizedText, bool)> {
        vec![]
    }
    fn command(&mut self, _index: usize) -> Vec<Feedback> {
        vec![]
    }
    /// Opt in for games where several people press independent controls at once.
    fn simultaneous_touches() -> bool {
        false
    }
    fn pause_alignment() -> Alignment {
        Alignment::Trailing
    }
    fn valid(&self) -> bool;
    fn players(&self) -> usize;
    fn humans(&self) -> usize {
        self.players()
    }
    fn turn(&self) -> usize;
    fn score(&self, player: usize) -> u32;
    fn finished(&self) -> bool;
    fn tick(&mut self, dt: f64) -> Vec<Feedback>;
    fn touch(&mut self, phase: TouchPhase, point: Point, size: Size) -> Vec<Feedback>;
    fn haptic(_event: Feedback, default: chrome::Pattern) -> chrome::Pattern {
        default
    }
    fn cancel(&mut self) {}
    fn clear_records(&mut self) {}
    fn draw(&self, draw: &mut Draw, size: Size, sprites: &[Option<Bitmap>]);
    fn status(&self) -> LocalizedText {
        res::str::party_turn(res::str::party_player((self.turn() + 1) as f64).format())
    }
    fn result_title(&self) -> LocalizedText {
        let w = self.winners();
        if self.players() == 1 {
            res::str::gk_game_over()
        } else if w.len() == 1 {
            res::str::party_wins(res::str::party_player((w[0] + 1) as f64).format())
        } else {
            res::str::party_draw()
        }
    }
    fn winners(&self) -> Vec<usize> {
        let best = (0..self.players())
            .map(|p| self.score(p))
            .max()
            .unwrap_or(0);
        (0..self.players())
            .filter(|&p| self.score(p) == best)
            .collect()
    }
}
use game_resources::res;
pub struct Config {
    pub key: &'static str,
    pub settings_key: &'static str,
    pub title: fn() -> LocalizedText,
    pub rules: fn() -> LocalizedText,
    pub board_id: ElementId,
    pub pause_id: ElementId,
    pub help_id: ElementId,
    pub background: Color,
    pub sprites: Vec<AssetName>,
    pub sounds: &'static [chrome::Sfx],
}
#[derive(Clone, Copy, PartialEq)]
enum Overlay {
    None,
    Pause,
    Help,
    Settings,
    Result,
}
struct Ui<G: BoardGame> {
    game: RefCell<G>,
    config: Config,
    overlay: Signal<Overlay>,
    back: Cell<Overlay>,
    repaint: Trigger,
    sounds: Signal<bool>,
    haptics: Signal<bool>,
    sprites: Vec<Signal<Option<Bitmap>>>,
    size: Cell<Size>,
    pointer: Cell<Option<u64>>,
    end_age: Cell<f64>,
    sound_wait: Cell<f64>,
    players: usize,
}
impl<G: BoardGame> Ui<G> {
    fn show(&self, next: Overlay) {
        self.game.borrow_mut().cancel();
        self.pointer.set(None);
        self.overlay.set(next);
        self.repaint.notify();
    }
    fn pause(&self) {
        if self.overlay.get_untracked() == Overlay::None {
            self.show(Overlay::Pause);
        }
    }
    fn new_game(&self) {
        let game = self.game.borrow().restart(self.players, crate::seed());
        *self.game.borrow_mut() = game;
        self.end_age.set(0.0);
        self.show(Overlay::None);
    }
    fn feedback(&self, events: Vec<Feedback>) {
        for e in events {
            if matches!(
                e,
                Feedback::Impact(_) | Feedback::Wall(_) | Feedback::Disc(_)
            ) {
                if self.sound_wait.get() > 0.0 {
                    continue;
                }
                self.sound_wait.set(0.065);
            }
            let (index, volume, cue) = match e {
                Feedback::Flick(v) => (6, (0.22 + 0.6 * v) as f32, chrome::cues::LIGHT_BEAT),
                Feedback::Tap => (0, 0.6, chrome::cues::LIGHT_BEAT),
                Feedback::Impact(v) => (0, v.clamp(0.1, 0.9) as f32, chrome::cues::TICK_BEAT),
                Feedback::OwnGoal => (5, 0.7, chrome::cues::WARNING_BEAT),
                Feedback::Score => (1, 0.7, chrome::cues::SUCCESS_BEAT),
                Feedback::Wall(v) => (3, v.clamp(0.1, 0.8) as f32, chrome::cues::TICK_BEAT),
                Feedback::Disc(v) => (4, v.clamp(0.1, 0.8) as f32, chrome::cues::TICK_BEAT),
                Feedback::Sweep => (5, 0.65, chrome::cues::SUCCESS_BEAT),
                Feedback::Miss => (3, 0.75, chrome::cues::WARNING_BEAT),
                Feedback::Turn => (2, 0.5, chrome::cues::MEDIUM_BEAT),
            };
            chrome::sound(
                self.sounds.get_untracked(),
                self.config
                    .sounds
                    .get(index)
                    .unwrap_or(&self.config.sounds[0]),
                volume,
            );
            chrome::haptic_pattern(self.haptics.get_untracked(), G::haptic(e, cue));
        }
    }
    fn touches(&self, events: &[Touch]) {
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        for t in events {
            if t.phase == TouchPhase::Began && self.pointer.get().is_none() {
                self.pointer.set(Some(t.id));
            }
            if !G::simultaneous_touches() && self.pointer.get() != Some(t.id) {
                continue;
            }
            let s = self.size.get();
            if s.width <= 0.0 || s.height <= 0.0 {
                continue;
            }
            let events = self.game.borrow_mut().touch(
                t.phase,
                Point::new(t.location.x / s.width, t.location.y / s.height),
                s,
            );
            self.feedback(events);
            if matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
                self.pointer.set(None);
            }
        }
        self.repaint.notify();
    }
}
pub fn page<G: BoardGame>(players: usize, config: Config) -> AnyPiece {
    page_with_art::<G>(players, config, |_, _| {})
}

pub fn page_with_art<G: BoardGame>(
    players: usize,
    config: Config,
    add_sprites: impl Fn(Size, &mut Vec<Option<Bitmap>>) + 'static,
) -> AnyPiece {
    crate::sounds(config.sounds);
    let settings = crate::restore::<chrome::GameSettings>(config.settings_key).unwrap_or_default();
    let saved = crate::restore::<G>(config.key).filter(|g| g.valid() && g.humans() == players);
    let initial = if !settings.instructions_shown {
        Overlay::Help
    } else if saved.as_ref().is_some_and(|g| g.finished()) {
        Overlay::Result
    } else if saved.is_some() {
        Overlay::Pause
    } else {
        Overlay::None
    };
    let sprites = config
        .sprites
        .iter()
        .cloned()
        .map(crate::sprites::load)
        .collect();
    let ui = Rc::new(Ui {
        game: RefCell::new(saved.unwrap_or_else(|| G::new(players, crate::seed()))),
        config,
        overlay: Signal::new(initial),
        back: Cell::new(Overlay::None),
        repaint: Trigger::new(),
        sounds: Signal::new(settings.sounds),
        haptics: Signal::new(settings.vibrations),
        sprites,
        size: Cell::new(Size::ZERO),
        pointer: Cell::new(None),
        end_age: Cell::new(0.0),
        sound_wait: Cell::new(0.0),
        players,
    });
    crate::autosave(ui.config.key, {
        let u = ui.clone();
        move || {
            let mut g = u.game.borrow().clone();
            g.cancel();
            g
        }
    });
    crate::on_reset_records(ui.config.key, {
        let u = ui.clone();
        move || {
            u.game.borrow_mut().clear_records();
            u.repaint.notify();
        }
    });
    crate::on_background(ui.config.key, {
        let u = ui.clone();
        move || u.pause()
    });
    Effect::new({
        let u = ui.clone();
        move || {
            crate::save(
                u.config.settings_key,
                &chrome::GameSettings {
                    sounds: u.sounds.get(),
                    vibrations: u.haptics.get(),
                    instructions_shown: true,
                },
            )
        }
    });
    let d = ui.clone();
    let t = ui.clone();
    let a = ui.clone();
    let p = ui.clone();
    let content = canvas(move |draw, size| {
        d.repaint.track();
        d.size.set(size);
        let mut sprites = d.sprites.iter().map(|s| s.get()).collect::<Vec<_>>();
        add_sprites(size, &mut sprites);
        d.game.borrow().draw(draw, size, &sprites);
    })
    .on_touches(move |ts| t.touches(ts))
    .a11y(move |a11y| {
        a11y.label((a.config.title)()).value({
            let a = a.clone();
            move || {
                a.repaint.track();
                a.game.borrow().status()
            }
        })
    })
    .id(ui.config.board_id.clone())
    .grow(Dimension::Both)
    .overlay_aligned(
        G::pause_alignment(),
        canvas(|d, s| {
            let c = Point::new(s.width / 2.0, s.height / 2.0);
            d.fill(
                Shape::Ellipse(Rect::new(c.x - 18.0, c.y - 18.0, 36.0, 36.0)),
                Color::WHITE,
            );
            chrome::draw_cross_glyph(d, c, 20.0, Color::hex(0x9DBCD0));
        })
        .frame(44.0, 44.0)
        .on_tap(move || p.pause())
        .a11y(|a| a.label(res::str::gk_pause()).role(Role::Button))
        .id(ui.config.pause_id.clone())
        .padding(6.0),
    );
    let u = ui.clone();
    let t = ui.clone();
    let clock = crate::animation::clock(
        move || u.overlay.get() == Overlay::None,
        move |dt| {
            let delta = dt.as_secs_f64().min(0.1);
            t.sound_wait.set((t.sound_wait.get() - delta).max(0.0));
            let events = t.game.borrow_mut().tick(delta);
            t.feedback(events);
            if t.game.borrow().finished() {
                t.end_age.set(t.end_age.get() + delta);
                if t.end_age.get() > 1.5 {
                    t.show(Overlay::Result);
                }
            }
            t.repaint.notify();
        },
    );
    let o = ui.clone();
    let m = ui.clone();
    let bg = ui.config.background;
    zstack((
        content,
        clock,
        when(move || o.overlay.get() != Overlay::None, chrome::scrim),
        switch(
            move || m.overlay.get(),
            move |o| (*o != Overlay::None).then(|| menu(ui.clone(), *o)),
        ),
    ))
    .background(bg)
    .any()
}
fn menu<G: BoardGame>(ui: Rc<Ui<G>>, overlay: Overlay) -> AnyPiece {
    if overlay == Overlay::Help {
        let id = ui.config.help_id.clone();
        return chrome::instructions_card(
            (ui.config.title)(),
            vec![Help::Para((ui.config.rules)())],
            id,
            move || ui.show(ui.back.get()),
        )
        .any();
    }
    let mut items = Vec::new();
    if overlay == Overlay::Settings {
        items.push(chrome::card_title(res::str::gk_settings(), Color::WHITE));
        items.push(chrome::setting_row(
            res::str::gk_sounds(),
            toggle(ui.sounds).any(),
        ));
        items.push(chrome::setting_row(
            res::str::gk_vibrations(),
            toggle(ui.haptics).any(),
        ));
        let options = ui.game.borrow().options();
        for (i, option) in options.into_iter().enumerate() {
            let selected = Signal::new(option.selected);
            let previous = std::cell::Cell::new(option.selected);
            let u = ui.clone();
            Effect::new(move || {
                let value = selected.get();
                if value != previous.replace(value) {
                    u.game.borrow_mut().set_option(i, value);
                    u.repaint.notify();
                }
            });
            items.push(chrome::setting_row(
                option.title,
                picker(
                    selected,
                    option.choices.into_iter().enumerate().collect::<Vec<_>>(),
                )
                .any(),
            ));
        }
        let u = ui.clone();
        items.push(chrome::menu_button(
            res::str::gk_done(),
            chrome::GREEN,
            id!(board_settings_done),
            move || u.show(Overlay::Pause),
        ));
    } else {
        let result = overlay == Overlay::Result;
        let title = if result {
            ui.game.borrow().result_title()
        } else {
            res::str::gk_paused()
        };
        items.push(chrome::card_title(title, Color::WHITE));
        if result {
            let g = ui.game.borrow();
            items.push(
                row((0..g.players())
                    .map(|p| {
                        multiplayer::score_card(
                            res::str::party_player((p + 1) as f64),
                            res::str::party_count(g.score(p) as f64),
                            res::str::party_count(g.score(p) as f64),
                            p,
                            multiplayer::PlayerState::Finished,
                            None,
                        )
                    })
                    .collect::<Vec<_>>())
                .spacing(6.0)
                .any(),
            );
        } else {
            let u = ui.clone();
            items.push(chrome::menu_button(
                res::str::gk_resume(),
                chrome::GREEN,
                id!(board_resume),
                move || u.show(Overlay::None),
            ));
        }
        for (i, (title, enabled)) in ui.game.borrow().commands().into_iter().enumerate() {
            let u = ui.clone();
            items.push(
                button(title)
                    .prominent()
                    .tint(chrome::INDIGO)
                    .enabled(enabled)
                    .action(move || {
                        let events = u.game.borrow_mut().command(i);
                        u.feedback(events);
                        u.end_age.set(0.0);
                        u.show(Overlay::None);
                    })
                    .id(id!(board_command, i))
                    .width(chrome::MENU_W)
                    .any(),
            );
        }
        let u = ui.clone();
        items.push(chrome::menu_button(
            if result {
                res::str::gk_play_again()
            } else {
                res::str::gk_new_game()
            },
            chrome::BLUE,
            id!(board_new_game),
            move || u.new_game(),
        ));
        let u = ui.clone();
        items.push(chrome::menu_button(
            res::str::gk_settings(),
            chrome::SLATE,
            id!(board_settings),
            move || u.show(Overlay::Settings),
        ));
        let u = ui.clone();
        items.push(chrome::menu_button(
            res::str::gk_instructions(),
            chrome::INDIGO,
            id!(board_help),
            move || {
                u.back.set(overlay);
                u.show(Overlay::Help);
            },
        ));
        items.push(chrome::close_game_button(chrome::RED, id!(board_quit)));
    }
    multiplayer::dialog(column(items).spacing(10.0).align(HAlign::Center)).any()
}
/// Shared drawing helpers, with fixed-width score badges and original player colors.
pub fn text(d: &mut Draw, value: &str, p: Point, size: f64, color: Color) {
    d.text(
        value,
        p,
        TextStyle {
            size,
            color: color.into(),
            anchor: TextAnchor::CENTERED,
            font: chrome::canvas_font(FontWeight::Bold),
        },
    );
}
pub fn circle(d: &mut Draw, p: Point, r: f64, c: Color) {
    d.fill(
        Shape::Ellipse(Rect::new(p.x - r, p.y - r, 2.0 * r, 2.0 * r)),
        c,
    );
}
pub fn line(d: &mut Draw, a: Point, b: Point, width: f64, c: Color) {
    d.stroke_styled(
        Shape::Line(a, b),
        c,
        StrokeStyle {
            width,
            cap: LineCap::Round,
            ..Default::default()
        },
    );
}
pub fn scores<G: BoardGame>(d: &mut Draw, s: Size, g: &G) {
    let w = (s.width * 0.28).min(116.0);
    let h = 56.0;
    for (p, &color) in COLORS.iter().enumerate().take(g.players()) {
        let top = p == 1 || p == 2;
        let right = p == 0 || p == 2;
        let x = if right { s.width - w + 5.0 } else { -5.0 };
        let y = if top { 40.0 } else { s.height - h - 40.0 };
        let r = Rect::new(x, y, w, h);
        d.fill(Shape::RoundedRect(r, h * 0.5), Color::BLACK);
        let inner = Rect::new(x + 7.0, y + 7.0, w - 14.0, h - 14.0);
        d.fill(Shape::RoundedRect(inner, h * 0.5), Color::hex(0x2F333E));
        if p == g.turn() {
            d.stroke(Shape::RoundedRect(inner, h * 0.5), color, 2.0);
        }
        text(
            d,
            &res::str::party_player((p + 1) as f64).format(),
            Point::new(x + w * 0.5, y + 17.0),
            10.0,
            color,
        );
        text(
            d,
            &res::str::party_count(g.score(p) as f64).format(),
            Point::new(x + w * 0.5, y + 36.0),
            28.0,
            color,
        );
    }
}
