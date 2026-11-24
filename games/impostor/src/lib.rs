// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
mod model;
use day_pieces::prelude::*;
use day_spec::TouchPhase;
use game_resources::res;
use gamekit::{
    board_game::{circle, line, text},
    chrome::{self, Help},
    multiplayer,
};
use model::{Category, Game, Phase, Settings};
use std::{cell::RefCell, rc::Rc};
pub const SURFACE: Color = Color::hex(0x393F70);
const COLORS: [Color; 4] = [
    Color::hex(0xFF5F57),
    Color::hex(0x10AFD0),
    Color::hex(0x79C937),
    Color::hex(0xFFC332),
];
const KEY: &str = "impostor.v2";
const SETTINGS: &str = "impostor.settings";
pub const SOUNDS: &[chrome::Sfx] = &[
    res::assets::sounds::shared::tap_wav,
    res::assets::sounds::shared::success_wav,
];
fn decks(language: usize) -> Vec<Category> {
    let asset = [
        res::assets::impostor::english_json,
        res::assets::impostor::spanish_json,
        res::assets::impostor::french_json,
        res::assets::impostor::german_json,
        res::assets::impostor::italian_json,
        res::assets::impostor::portuguese_json,
    ][language.min(5)]
    .clone();
    day_spec::resource(asset)
        .and_then(|r| serde_json::from_slice(r.as_slice()).ok())
        .unwrap_or_default()
}
fn name(g: &Game, p: usize) -> String {
    if g.names[p].trim().is_empty() {
        res::str::party_player((p + 1) as f64).format()
    } else {
        g.names[p].clone()
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Overlay {
    None,
    Pause,
    Help,
    Settings,
}
struct Ui {
    game: RefCell<Game>,
    settings: RefCell<Settings>,
    repaint: Trigger,
    phase: Signal<Phase>,
    overlay: Signal<Overlay>,
    selected: Signal<Option<usize>>,
    sounds: Signal<bool>,
    haptics: Signal<bool>,
}
impl Ui {
    fn change(&self, f: impl FnOnce(&mut Game)) {
        f(&mut self.game.borrow_mut());
        self.phase.set(self.game.borrow().phase);
        self.repaint.notify();
    }
    fn sound(&self, success: bool) {
        chrome::sound(
            self.sounds.get_untracked(),
            &SOUNDS[usize::from(success)],
            0.55,
        );
        chrome::haptic_pattern(
            self.haptics.get_untracked(),
            if success {
                chrome::cues::SUCCESS_BEAT
            } else {
                chrome::cues::LIGHT_BEAT
            },
        );
    }
    fn show(&self, o: Overlay) {
        self.change(Game::hide);
        self.overlay.set(o);
    }
    fn restart(&self) {
        let settings = self.settings.borrow();
        let bank = decks(settings.language);
        self.change(|g| {
            g.start(&bank, &settings);
        });
        self.selected.set(None);
        self.overlay.set(Overlay::None);
        self.sound(false);
    }
}
pub fn impostor_page(players: usize) -> AnyPiece {
    gamekit::sounds(SOUNDS);
    let prefs = gamekit::restore::<chrome::GameSettings>("impostor.feedback").unwrap_or_default();
    let locale = day_localization_ui::locale().get_untracked();
    let default_lang = if locale.starts_with("es") {
        1
    } else if locale.starts_with("fr") {
        2
    } else if locale.starts_with("de") {
        3
    } else if locale.starts_with("it") {
        4
    } else if locale.starts_with("pt") {
        5
    } else {
        0
    };
    let mut settings = gamekit::restore::<Settings>(SETTINGS).unwrap_or(Settings {
        language: default_lang,
        ..Default::default()
    });
    settings.language = settings.language.min(5);
    settings.roles = settings.roles.clamp(1, 2);
    if settings.enabled.is_empty() {
        settings.enabled = (1..=12).collect();
    }
    let saved =
        gamekit::restore::<Game>(KEY).filter(|g| g.valid() && g.selected_players == players);
    let mut game = saved.clone().unwrap_or_else(|| {
        let mut g = Game::new(players, gamekit::seed());
        g.start(&decks(settings.language), &settings);
        g
    });
    game.hide();
    let phase = game.phase;
    let ui = Rc::new(Ui {
        game: RefCell::new(game),
        settings: RefCell::new(settings),
        repaint: Trigger::new(),
        phase: Signal::new(phase),
        overlay: Signal::new(if !prefs.instructions_shown {
            Overlay::Help
        } else if saved.is_some() {
            Overlay::Pause
        } else {
            Overlay::None
        }),
        selected: Signal::new(None),
        sounds: Signal::new(prefs.sounds),
        haptics: Signal::new(prefs.vibrations),
    });
    gamekit::autosave(KEY, {
        let u = ui.clone();
        move || {
            let mut g = u.game.borrow().clone();
            g.hide();
            g
        }
    });
    gamekit::autosave(SETTINGS, {
        let u = ui.clone();
        move || u.settings.borrow().clone()
    });
    gamekit::on_background(KEY, {
        let u = ui.clone();
        move || u.show(Overlay::Pause)
    });
    Effect::new({
        let u = ui.clone();
        move || {
            gamekit::save(
                "impostor.feedback",
                &chrome::GameSettings {
                    sounds: u.sounds.get(),
                    vibrations: u.haptics.get(),
                    instructions_shown: true,
                },
            )
        }
    });
    let clock = gamekit::animation::clock(
        {
            let u = ui.clone();
            move || u.overlay.get() == Overlay::None
        },
        {
            let u = ui.clone();
            move |dt| {
                let ready = u.game.borrow_mut().tick(dt.as_secs_f64().min(0.1));
                if ready {
                    u.sound(false);
                }
                u.repaint.notify();
            }
        },
    );
    let u = ui.clone();
    let v = ui.clone();
    let w = ui.clone();
    let m = ui.clone();
    zstack((
        canvas(move |d, s| {
            u.repaint.track();
            let g = u.game.borrow();
            let c = if g.phase == Phase::Reveal {
                COLORS[g.turn % 4]
            } else {
                SURFACE
            };
            d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), c);
            if g.phase != Phase::Reveal {
                for i in 0..12 {
                    text(
                        d,
                        "?",
                        Point::new(
                            (i % 3) as f64 * s.width * 0.5,
                            (i / 3) as f64 * s.height * 0.3,
                        ),
                        s.width * 0.3,
                        Color::rgba(0.4, 0.5, 0.8, 0.12),
                    );
                }
            }
        })
        .grow(Dimension::Both),
        column((
            row((
                button(res::str::gk_pause())
                    .action(move || v.show(Overlay::Pause))
                    .id(id!(impostor_pause)),
                spacer(),
                label(res::str::nav_impostor()).bold().color(Color::WHITE),
            ))
            .padding(12.0),
            switch(move || w.phase.get(), {
                let u = ui.clone();
                move |phase| screen(u.clone(), *phase)
            })
            .grow(Dimension::Both),
        ))
        .grow(Dimension::Both),
        clock,
        when(move || m.overlay.get() != Overlay::None, chrome::scrim),
        switch(
            {
                let u = ui.clone();
                move || u.overlay.get()
            },
            move |o| (*o != Overlay::None).then(|| menu(ui.clone(), *o)),
        ),
    ))
    .background(SURFACE)
    .any()
}
fn screen(ui: Rc<Ui>, phase: Phase) -> AnyPiece {
    match phase {
        Phase::Reveal => {
            let u = ui.clone();
            let a = ui.clone();
            let b = ui.clone();
            let c = ui.clone();
            let e = ui.clone();
            let f = ui.clone();
            let card = canvas(move |d, s| {
                u.repaint.track();
                let g = u.game.borrow();
                d.fill(
                    Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)),
                    Color::hex(0x292C2F),
                );
                let visible = g.held && g.flip >= 0.5;
                if visible {
                    d.fill(
                        Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)),
                        Color::hex([0x35180F, 0x10333B, 0x233B14, 0x3B3210][g.turn % 4]),
                    );
                    if g.impostors.contains(&g.turn) {
                        spy(
                            d,
                            Point::new(s.width * 0.5, s.height * 0.36),
                            s.height.min(s.width) * 0.25,
                        );
                        wrapped(
                            d,
                            &res::str::impostor_role().format(),
                            s.width,
                            s.height * 0.68,
                            25.0,
                            Color::WHITE,
                        );
                        if u.settings.borrow().hint {
                            wrapped(d, &g.category, s.width, s.height * 0.88, 17.0, Color::WHITE);
                        }
                    } else {
                        wrapped(d, &g.word, s.width, s.height * 0.5, 36.0, Color::WHITE);
                    }
                } else {
                    for i in -20..40 {
                        let x = i as f64 * 24.0;
                        line(
                            d,
                            Point::new(x, 0.0),
                            Point::new(x - s.height, s.height),
                            12.0,
                            Color::rgba(1.0, 1.0, 1.0, 0.025),
                        );
                    }
                    text(
                        d,
                        "?",
                        Point::new(s.width * 0.5, s.height * 0.34),
                        72.0,
                        Color::hex(0x838383),
                    );
                    wrapped(
                        d,
                        &res::str::impostor_hold().format(),
                        s.width,
                        s.height * 0.68,
                        24.0,
                        Color::WHITE,
                    );
                }
            })
            .on_touches(move |ts| {
                if a.overlay.get_untracked() != Overlay::None {
                    return;
                }
                for t in ts {
                    a.change(|g| match t.phase {
                        TouchPhase::Began => {
                            g.held = true;
                            g.hold_age = 0.0;
                        }
                        TouchPhase::Ended | TouchPhase::Cancelled => g.hide(),
                        _ => {}
                    });
                }
            })
            .a11y(move |a11y| {
                a11y.label(res::str::impostor_hold()).value(move || {
                    b.repaint.track();
                    let g = b.game.borrow();
                    if g.held && g.flip >= 0.5 {
                        if g.impostors.contains(&g.turn) {
                            res::str::impostor_role()
                        } else {
                            res::str::impostor_content(g.word.clone())
                        }
                    } else {
                        res::str::impostor_word_for(name(&g, g.turn))
                    }
                })
            })
            .id(id!(impostor_reveal))
            .grow(Dimension::Both);
            column((
                label(move || {
                    c.repaint.track();
                    let g = c.game.borrow();
                    res::str::impostor_word_for(name(&g, g.turn))
                })
                .font(Font::LargeTitle)
                .bold()
                .align(TextAlign::Center)
                .color(Color::hex(0x191B1C))
                .padding(20.0),
                card,
                label(move || {
                    e.repaint.track();
                    let g = e.game.borrow();
                    if g.turn + 1 < g.count {
                        res::str::impostor_pass(name(&g, g.turn + 1))
                    } else {
                        res::str::impostor_discuss()
                    }
                })
                .bold()
                .align(TextAlign::Center)
                .color(Color::hex(0x191B1C))
                .padding(16.0),
                button(res::str::gk_done())
                    .prominent()
                    .tint(Color::hex(0x191B1C))
                    .enabled(move || {
                        f.repaint.track();
                        let g = f.game.borrow();
                        g.seen && !g.held
                    })
                    .action(move || {
                        if ui.game.borrow_mut().next() {
                            ui.phase.set(ui.game.borrow().phase);
                            ui.repaint.notify();
                            ui.sound(false);
                        }
                    })
                    .id(id!(impostor_next))
                    .padding(20.0),
            ))
            .spacing(8.0)
            .grow(Dimension::Both)
            .any()
        }
        Phase::Discussion => {
            let u = ui.clone();
            column((
                spacer(),
                chrome::card_title(res::str::impostor_discuss(), chrome::BLUE),
                label(res::str::impostor_clues())
                    .font(Font::Title2)
                    .align(TextAlign::Center)
                    .color(Color::WHITE)
                    .padding(22.0),
                spacer(),
                chrome::menu_button(
                    res::str::gk_done(),
                    Color::hex(0x191B1C),
                    id!(impostor_discussed),
                    move || {
                        u.change(|g| g.phase = Phase::Voting);
                        u.sound(false);
                    },
                ),
            ))
            .align(HAlign::Center)
            .padding(20.0)
            .grow(Dimension::Both)
            .any()
        }
        Phase::Voting => {
            let g = ui.game.borrow();
            let mut rows = vec![
                chrome::card_title(res::str::impostor_vote(), chrome::BLUE),
                label(res::str::impostor_choose())
                    .align(TextAlign::Center)
                    .color(Color::WHITE)
                    .any(),
            ];
            for p in 0..g.count {
                let s = ui.selected;
                let u = ui.clone();
                rows.push(
                    label(name(&g, p))
                        .font(Font::Title)
                        .bold()
                        .color(Color::BLACK)
                        .align(TextAlign::Center)
                        .padding(24.0)
                        .grow(Dimension::Width)
                        .background(COLORS[p % 4])
                        .corner_radius(18.0)
                        .overlay(
                            canvas(move |d, size| {
                                if s.get() == Some(p) {
                                    d.stroke(
                                        Shape::RoundedRect(
                                            Rect::new(
                                                3.0,
                                                3.0,
                                                size.width - 6.0,
                                                size.height - 6.0,
                                            ),
                                            16.0,
                                        ),
                                        Color::WHITE,
                                        5.0,
                                    );
                                }
                            })
                            .grow(Dimension::Both),
                        )
                        .on_tap(move || {
                            u.selected.set(Some(p));
                            u.sound(false);
                        })
                        .a11y(|a| a.role(Role::Button))
                        .id(id!(impostor_vote, p))
                        .any(),
                );
            }
            drop(g);
            let u = ui.clone();
            let v = ui.clone();
            rows.push(
                button(move || {
                    u.selected
                        .get()
                        .map(|p| res::str::impostor_confirm(name(&u.game.borrow(), p)))
                        .unwrap_or_else(res::str::impostor_vote)
                })
                .prominent()
                .tint(Color::hex(0x191B1C))
                .enabled(move || v.selected.get().is_some())
                .action(move || {
                    if let Some(p) = ui.selected.get_untracked() {
                        ui.change(|g| {
                            g.vote(p);
                        });
                        ui.sound(true);
                    }
                })
                .id(id!(impostor_confirm))
                .any(),
            );
            scroll(column(rows).spacing(14.0).padding(16.0))
                .grow(Dimension::Both)
                .any()
        }
        Phase::Result => {
            let g = ui.game.borrow();
            let mut rows = vec![
                chrome::card_title(
                    if g.caught {
                        res::str::impostor_caught()
                    } else {
                        res::str::impostor_escaped()
                    },
                    if g.caught { chrome::GREEN } else { chrome::RED },
                ),
                canvas(|d, s| {
                    spy(
                        d,
                        Point::new(s.width / 2.0, s.height / 2.0),
                        s.height * 0.42,
                    )
                })
                .height(110.0)
                .grow(Dimension::Width)
                .any(),
                label(res::str::impostor_roles()).color(Color::WHITE).any(),
                label(
                    g.impostors
                        .iter()
                        .map(|&p| name(&g, p))
                        .collect::<Vec<_>>()
                        .join(", "),
                )
                .font(Font::Title2)
                .bold()
                .color(chrome::RED)
                .any(),
                label(res::str::impostor_secret()).color(Color::WHITE).any(),
                label(g.word.clone())
                    .font(Font::Title)
                    .bold()
                    .color(Color::WHITE)
                    .any(),
            ];
            for p in 0..g.count {
                rows.push(
                    row((
                        label(name(&g, p)).bold().color(COLORS[p % 4]),
                        spacer(),
                        label(res::str::party_count(g.scores[p] as f64))
                            .font(Font::Title2)
                            .bold()
                            .tabular()
                            .color(COLORS[p % 4]),
                    ))
                    .padding(8.0)
                    .any(),
                );
            }
            drop(g);
            let u = ui.clone();
            rows.push(chrome::menu_button(
                res::str::impostor_next(),
                chrome::BLUE,
                id!(impostor_next_round),
                move || u.restart(),
            ));
            rows.push(chrome::close_game_button(chrome::RED, id!(impostor_quit)));
            scroll(
                column(rows)
                    .spacing(12.0)
                    .align(HAlign::Center)
                    .padding(18.0),
            )
            .grow(Dimension::Both)
            .any()
        }
    }
}
fn menu(ui: Rc<Ui>, overlay: Overlay) -> AnyPiece {
    if overlay == Overlay::Help {
        return chrome::instructions_card(
            res::str::nav_impostor(),
            vec![Help::Para(res::str::impostor_rules())],
            id!(impostor_help_done),
            move || ui.show(Overlay::None),
        )
        .any();
    }
    let mut rows = vec![chrome::card_title(
        if overlay == Overlay::Settings {
            res::str::gk_settings()
        } else {
            res::str::gk_paused()
        },
        Color::WHITE,
    )];
    if overlay == Overlay::Settings {
        rows.push(chrome::setting_row(
            res::str::gk_sounds(),
            toggle(ui.sounds).any(),
        ));
        rows.push(chrome::setting_row(
            res::str::gk_vibrations(),
            toggle(ui.haptics).any(),
        ));
        let roles = Signal::new(ui.settings.borrow().roles);
        let hint = Signal::new(ui.settings.borrow().hint);
        let language = Signal::new(ui.settings.borrow().language);
        rows.push(chrome::setting_row(
            res::str::impostor_roles(),
            picker(
                roles,
                vec![
                    (1, res::str::party_count(1.0)),
                    (2, res::str::party_count(2.0)),
                ],
            )
            .any(),
        ));
        rows.push(chrome::setting_row(
            res::str::impostor_hint(),
            toggle(hint).any(),
        ));
        rows.push(chrome::setting_row(
            res::str::impostor_language(),
            picker(
                language,
                [
                    res::str::impostor_deck_english(),
                    res::str::impostor_deck_spanish(),
                    res::str::impostor_deck_french(),
                    res::str::impostor_deck_german(),
                    res::str::impostor_deck_italian(),
                    res::str::impostor_deck_portuguese(),
                ]
                .into_iter()
                .enumerate()
                .collect::<Vec<_>>(),
            )
            .any(),
        ));
        Effect::new({
            let u = ui.clone();
            move || {
                let mut s = u.settings.borrow_mut();
                s.roles = roles.get();
                s.hint = hint.get();
                s.language = language.get();
                gamekit::save(SETTINGS, &*s);
            }
        });
        rows.push(chrome::section_heading(res::str::impostor_categories()));
        let u = ui.clone();
        rows.push(
            switch(
                move || language.get(),
                move |l| {
                    let bank = decks(*l);
                    column(
                        bank.into_iter()
                            .map(|c| {
                                let enabled =
                                    Signal::new(u.settings.borrow().enabled.contains(&c.id));
                                Effect::new({
                                    let u = u.clone();
                                    move || {
                                        let on = enabled.get();
                                        let mut s = u.settings.borrow_mut();
                                        if on && !s.enabled.contains(&c.id) {
                                            s.enabled.push(c.id);
                                        } else if !on && s.enabled.len() > 1 {
                                            s.enabled.retain(|&id| id != c.id);
                                        } else if !on {
                                            enabled.set(true);
                                        }
                                        gamekit::save(SETTINGS, &*s);
                                    }
                                });
                                chrome::setting_row(
                                    res::str::impostor_content(c.title),
                                    toggle(enabled).any(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    )
                    .spacing(8.0)
                    .any()
                },
            )
            .any(),
        );
        let count = Signal::new(ui.game.borrow().count);
        rows.push(chrome::setting_row(
            res::str::home_players_label(),
            picker(
                count,
                (3..=10)
                    .map(|n| (n, res::str::party_count(n as f64)))
                    .collect::<Vec<_>>(),
            )
            .any(),
        ));
        Effect::new({
            let u = ui.clone();
            move || {
                let n = count.get();
                if n != u.game.borrow().count {
                    u.change(|g| {
                        g.count = n;
                        g.names.resize(n, String::new());
                        g.scores.resize(n, 0);
                        let settings = u.settings.borrow();
                        g.start(&decks(settings.language), &settings);
                    });
                    u.selected.set(None);
                }
            }
        });
        let u = ui.clone();
        rows.push(
            switch(
                move || count.get(),
                move |count| {
                    column(
                        (0..*count)
                            .map(|p| {
                                let n = Signal::new(u.game.borrow().names[p].clone());
                                Effect::new({
                                    let u = u.clone();
                                    move || {
                                        u.game.borrow_mut().names[p] = n.get();
                                        u.repaint.notify();
                                    }
                                });
                                chrome::setting_row(
                                    res::str::party_player((p + 1) as f64),
                                    text_field(n)
                                        .placeholder(res::str::party_player((p + 1) as f64))
                                        .width(150.0)
                                        .any(),
                                )
                            })
                            .collect::<Vec<_>>(),
                    )
                    .spacing(8.0)
                    .any()
                },
            )
            .any(),
        );
        let u = ui.clone();
        rows.push(chrome::menu_button(
            res::str::gk_done(),
            chrome::GREEN,
            id!(impostor_settings_done),
            move || u.show(Overlay::Pause),
        ));
    } else {
        let u = ui.clone();
        rows.push(chrome::menu_button(
            res::str::gk_resume(),
            chrome::GREEN,
            id!(impostor_resume),
            move || u.show(Overlay::None),
        ));
        let u = ui.clone();
        rows.push(chrome::menu_button(
            res::str::impostor_next(),
            chrome::BLUE,
            id!(impostor_restart),
            move || u.restart(),
        ));
        let u = ui.clone();
        rows.push(chrome::menu_button(
            res::str::gk_settings(),
            chrome::SLATE,
            id!(impostor_settings),
            move || u.show(Overlay::Settings),
        ));
        let u = ui.clone();
        rows.push(chrome::menu_button(
            res::str::gk_instructions(),
            chrome::INDIGO,
            id!(impostor_help),
            move || u.show(Overlay::Help),
        ));
        rows.push(chrome::close_game_button(chrome::RED, id!(impostor_quit)));
    }
    multiplayer::dialog(column(rows).spacing(12.0).align(HAlign::Center)).any()
}
fn wrapped(d: &mut Draw, value: &str, width: f64, y: f64, size: f64, color: Color) {
    let max = (width - 28.0) / (size * 0.6);
    let mut lines = vec![String::new()];
    for word in value.split_whitespace() {
        let last = lines.last_mut().expect("validated game invariant");
        if !last.is_empty() && last.chars().count() + word.chars().count() + 1 > max as usize {
            lines.push(word.to_owned());
        } else {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(word);
        }
    }
    let n = lines.len();
    for (i, l) in lines.iter().enumerate() {
        let size = size.min((width - 28.0) / (l.chars().count().max(1) as f64 * 0.62));
        text(
            d,
            l,
            Point::new(
                width / 2.0,
                y + (i as f64 - (n - 1) as f64 / 2.0) * size * 1.2,
            ),
            size,
            color,
        );
    }
}
fn spy(d: &mut Draw, p: Point, r: f64) {
    let ink = Color::hex(0x202332);
    d.fill(
        Shape::RoundedRect(
            Rect::new(p.x - r * 0.8, p.y - r * 0.2, r * 1.6, r * 1.1),
            r * 0.2,
        ),
        ink,
    );
    d.fill(
        PathBuilder::new()
            .move_to(Point::new(p.x - r * 0.9, p.y - r * 0.7))
            .line_to(Point::new(p.x + r * 0.9, p.y - r * 0.7))
            .line_to(Point::new(p.x + r * 0.48, p.y - r * 0.9))
            .line_to(Point::new(p.x + r * 0.35, p.y - r * 1.3))
            .line_to(Point::new(p.x - r * 0.35, p.y - r * 1.3))
            .line_to(Point::new(p.x - r * 0.48, p.y - r * 0.9))
            .close()
            .build(),
        ink,
    );
    for x in [-0.3, 0.3] {
        circle(
            d,
            Point::new(p.x + x * r, p.y - r * 0.44),
            r * 0.24,
            Color::hex(0xC93825),
        );
    }
    for sign in [-1.0, 1.0] {
        d.fill(
            PathBuilder::new()
                .move_to(Point::new(p.x + sign * r * 0.45, p.y))
                .line_to(Point::new(p.x, p.y + r * 0.7))
                .line_to(Point::new(p.x + sign * r * 0.12, p.y + r * 0.12))
                .close()
                .build(),
            Color::WHITE,
        );
    }
}
pub fn impostor_preview() -> AnyPiece {
    canvas(move |d, s| {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), SURFACE);
        spy(
            d,
            Point::new(s.width / 2.0, s.height * 0.58),
            s.width.min(s.height) * 0.46,
        );
    })
    .grow(Dimension::Both)
    .any()
}
