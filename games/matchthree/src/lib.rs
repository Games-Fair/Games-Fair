// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Match Three: six levels of shape matching with native vector artwork and animated cascades.
use day_fluent::{LocalizedText, tr};
use day_pieces::prelude::*;
use gamekit::chrome::{self, Feedback, Help, Sfx, cues, sfx};
use serde::{Deserialize, Serialize};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};
mod gesture;
mod model;
use gesture::{Hold, Preview};
use model::*;
pub const SURFACE: Color = Color::hex(0x18132E);
const SAVE: &str = "matchthree.v1";
const SETTINGS: &str = "matchthree.settings";
const ACCENT: Color = Color::hex(0x9EBFE9);
const MINT: Color = Color::hex(0x87F4D1);
const PALETTE: [u32; 6] = [0xEB7889, 0x60B9D7, 0xE5BE69, 0xA594D6, 0x74BD98, 0xD89571];
pub const SOUNDS: &[Sfx] = &[
    sfx("sounds/matchthree/swap.wav"),
    sfx("sounds/matchthree/bloom.wav"),
    sfx("sounds/matchthree/cascade.wav"),
    sfx("sounds/matchthree/magic.wav"),
    sfx("sounds/matchthree/victory.wav"),
];
static SWAP: chrome::Cue = cues::with("sounds/matchthree/swap.wav", cues::LIGHT_BEAT);
static BLOOM: chrome::Cue = cues::with("sounds/matchthree/bloom.wav", cues::MEDIUM_BEAT);
static CASCADE: chrome::Cue = cues::with("sounds/matchthree/cascade.wav", chrome::CELEBRATE);
static MAGIC: chrome::Cue = cues::with("sounds/matchthree/magic.wav", chrome::THUD);
static VICTORY: chrome::Cue = cues::with("sounds/matchthree/victory.wav", chrome::BIG_CELEBRATE);
#[derive(Clone, Default, Serialize, Deserialize)]
struct Settings {
    shell: chrome::GameSettings,
    #[serde(default)]
    reduced: bool,
}
#[derive(Clone, Copy, PartialEq)]
enum Overlay {
    None,
    Pause,
    Map,
    Help,
    Settings,
    Result,
}
struct Ui {
    save: RefCell<Save>,
    frames: RefCell<VecDeque<Frame>>,
    elapsed: Cell<f64>,
    time: Cell<f64>,
    selected: Cell<Option<usize>>,
    cursor: Cell<usize>,
    hint: Cell<Option<(usize, usize)>>,
    size: Cell<Size>,
    drag: Cell<Option<Hold>>,
    settling: Cell<Option<Preview>>,
    suppress_tap_until: Cell<f64>,
    overlay: Signal<Overlay>,
    back: Cell<Overlay>,
    repaint: Trigger,
    focus: Signal<bool>,
    sounds: Signal<bool>,
    vibrations: Signal<bool>,
    reduced: Signal<bool>,
}
impl Ui {
    fn cue(&self, c: &chrome::Cue) {
        chrome::cue(
            Feedback {
                sounds: self.sounds.get_untracked(),
                vibrations: self.vibrations.get_untracked(),
            },
            c,
        );
    }
    fn show(&self, o: Overlay) {
        if o != Overlay::None {
            self.cancel_hold();
        }
        self.overlay.set(o);
        self.focus.set(o == Overlay::None);
        self.cue(&cues::SELECT);
    }
    fn push(&self, o: Overlay) {
        self.back.set(self.overlay.get_untracked());
        self.show(o);
    }
    fn busy(&self) -> bool {
        self.overlay.get_untracked() != Overlay::None
            || !self.frames.borrow().is_empty()
            || self.save.borrow().game.over()
            || self.drag.get().is_some()
            || self.settling.get().is_some()
    }
    fn start(&self, level: usize) {
        if !self.save.borrow().unlocked(level) {
            return;
        }
        self.save.borrow_mut().game = Game::new(level, gamekit::seed());
        self.frames.borrow_mut().clear();
        self.drag.set(None);
        self.settling.set(None);
        self.selected.set(None);
        self.hint.set(None);
        self.cursor
            .set((0..LEN).find(|&i| active(level, i)).unwrap());
        self.elapsed.set(0.0);
        self.show(Overlay::None);
        self.repaint.notify();
        self.cue(&cues::START);
        self.persist();
    }
    fn persist(&self) {
        gamekit::save(SAVE, &*self.save.borrow());
    }
    fn swap(&self, a: usize, b: usize) {
        if self.busy() {
            return;
        }
        let turn = self.save.borrow_mut().game.swap(a, b);
        self.selected.set(None);
        self.hint.set(None);
        self.elapsed.set(0.0);
        *self.frames.borrow_mut() = turn.frames.into();
        self.cue(if turn.valid { &SWAP } else { &cues::WARNING });
        self.save.borrow_mut().record();
        self.persist();
        self.repaint.notify();
    }
    fn choose(&self, i: usize) {
        if self.busy() || self.save.borrow().game.grid[i].is_none() {
            return;
        }
        self.cursor.set(i);
        if let Some(a) = self.selected.get() {
            if a == i {
                self.selected.set(None);
            } else if Game::adjacent(a, i) {
                self.swap(a, i);
                return;
            } else {
                self.selected.set(Some(i));
            }
        } else {
            self.selected.set(Some(i));
        }
        self.cue(&cues::TICK);
        self.repaint.notify();
    }
    fn hint(&self) {
        if self.busy() {
            return;
        }
        let pair = self.save.borrow().game.hint();
        self.hint.set(pair);
        if let Some((a, b)) = pair {
            self.selected.set(Some(a));
            self.cursor.set(b);
        }
        self.cue(&cues::HINT);
        self.repaint.notify();
    }
    fn key(&self, key: &str) {
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        if self.drag.get().is_some() {
            if matches!(key, "Escape" | "p" | "P") {
                self.show(Overlay::Pause);
            }
            return;
        }
        let i = self.cursor.get();
        let next = match key {
            "Home" => Some(0),
            "ArrowLeft" => Some(i / N * N + (i % N + N - 1) % N),
            "ArrowRight" => Some(i / N * N + (i % N + 1) % N),
            "ArrowUp" => Some((i + LEN - N) % LEN),
            "ArrowDown" => Some((i + N) % LEN),
            " " | "Enter" | "Return" => {
                self.choose(i);
                None
            }
            "h" | "H" => {
                self.hint();
                None
            }
            "Escape" | "p" | "P" => {
                self.show(Overlay::Pause);
                None
            }
            _ => None,
        };
        if let Some(j) = next {
            self.cursor.set(j);
            self.repaint.notify();
        }
    }
    fn needs_frame(&self) -> bool {
        !self.frames.borrow().is_empty()
            || self.settling.get().is_some()
            || self.drag.get().is_some()
            || self.selected.get().is_some()
            || self.hint.get().is_some()
            // Keep the gesture's duplicate-tap guard advancing even after a reduced-motion
            // return has already settled; otherwise an idle board could suppress taps forever.
            || self.time.get() < self.suppress_tap_until.get()
            || self.save.borrow().game.over()
    }
    fn tick(&self, dt: f64) {
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        self.time.set(self.time.get() + dt);
        let duration = self.frames.borrow().front().map(|f| match f.phase {
            Phase::Swap | Phase::Return => 0.18,
            Phase::Burst => 0.28,
            Phase::Fall => 0.32,
            Phase::Shuffle => 0.4,
        });
        if let Some(duration) = duration {
            let t = self.elapsed.get() + dt;
            self.elapsed.set(t);
            if t >= if self.reduced.get_untracked() {
                0.04
            } else {
                duration
            } {
                self.frames.borrow_mut().pop_front();
                self.settling.set(None);
                self.elapsed.set(0.0);
                let cue = self.frames.borrow().front().and_then(|f| {
                    if f.phase == Phase::Burst {
                        Some(
                            if f.cleared
                                .iter()
                                .any(|&i| f.grid[i].is_some_and(|c| c.special != Special::Plain))
                            {
                                &MAGIC
                            } else if f.chain > 1 {
                                &CASCADE
                            } else {
                                &BLOOM
                            },
                        )
                    } else {
                        None
                    }
                });
                if let Some(c) = cue {
                    self.cue(c);
                }
            }
        }
        let returning = duration.is_none() && self.settling.get().is_some();
        if returning {
            self.elapsed.set(self.elapsed.get() + dt);
            if self.elapsed.get() >= 0.16 || self.reduced.get_untracked() {
                self.settling.set(None);
            }
        }
        if self.frames.borrow().is_empty() && self.save.borrow().game.over() {
            self.show(Overlay::Result);
            self.cue(if self.save.borrow().game.won() {
                &VICTORY
            } else {
                &cues::LETDOWN
            });
        }
        if duration.is_some()
            || returning
            || self.drag.get().is_some()
            || self.selected.get().is_some()
            || self.hint.get().is_some()
        {
            self.repaint.notify();
        }
    }
    fn cancel_hold(&self) {
        if self.drag.take().is_some() {
            self.selected.set(None);
            self.hint.set(None);
            self.repaint.notify();
        }
        // A partially animated committed turn still owns its start positions.
        if self.frames.borrow().is_empty() {
            self.settling.set(None);
        }
    }
    fn preview(&self) -> Option<Preview> {
        if let Some(hold) = self.drag.get() {
            return Some(hold.preview);
        }
        if self.frames.borrow().is_empty() {
            return self
                .settling
                .get()
                .map(|p| p.scaled((1.0 - self.elapsed.get() / 0.16).clamp(0.0, 1.0).powi(3)));
        }
        None
    }
    fn gesture(&self, e: Drag) {
        if e.phase == DragPhase::Began {
            if self.busy() {
                return;
            }
            // Native pan recognizers report Began after touch slop; recover the actual press.
            let start = Point::new(
                e.location.x - e.translation.x,
                e.location.y - e.translation.y,
            );
            let Some(from) = self.at(start) else {
                return;
            };
            if self.save.borrow().game.grid[from].is_none() {
                return;
            }
            self.drag.set(Some(Hold::new(from, start)));
            self.hint.set(None);
            self.cursor.set(from);
        }
        let Some(mut hold) = self.drag.get() else {
            return;
        };
        let old = hold.preview.to;
        let (_, _, cell) = layout(self.size.get());
        hold.update(
            &self.save.borrow().game,
            e.location,
            cell,
            self.at(e.location).is_some(),
        );
        if hold.moved {
            self.selected.set(None);
        }
        if e.phase == DragPhase::Ended {
            self.drag.set(None);
            if hold.moved {
                // Some recognizers also report a tap after a pan that returns to its origin.
                self.suppress_tap_until.set(self.time.get() + 0.15);
                self.elapsed.set(0.0);
                if let Some((a, b)) = hold.release() {
                    self.swap(a, b);
                }
                self.settling.set(Some(hold.preview));
            }
        } else {
            self.drag.set(Some(hold));
            if hold.preview.to.is_some() && hold.preview.to != old {
                self.cue(&cues::TICK);
            }
        }
        self.repaint.notify();
    }
    fn at(&self, p: Point) -> Option<usize> {
        let (x, y, c) = layout(self.size.get());
        if c <= 0.0 || p.x < x || p.y < y || p.x >= x + c * N as f64 || p.y >= y + c * N as f64 {
            None
        } else {
            Some(((p.y - y) / c) as usize * N + ((p.x - x) / c) as usize)
        }
    }
}
pub fn matchthree_page() -> AnyPiece {
    let settings = gamekit::restore::<Settings>(SETTINGS).unwrap_or_default();
    let save = gamekit::restore::<Save>(SAVE)
        .and_then(Save::valid)
        .unwrap_or_else(|| Save::new(gamekit::seed()));
    let ui = Rc::new(Ui {
        save: RefCell::new(save),
        frames: RefCell::new(VecDeque::new()),
        elapsed: Cell::new(0.0),
        time: Cell::new(0.0),
        selected: Cell::new(None),
        cursor: Cell::new(0),
        hint: Cell::new(None),
        size: Cell::new(Size::ZERO),
        drag: Cell::new(None),
        settling: Cell::new(None),
        suppress_tap_until: Cell::new(0.0),
        overlay: Signal::new(Overlay::None),
        back: Cell::new(Overlay::None),
        repaint: Trigger::new(),
        focus: Signal::new(true),
        sounds: Signal::new(settings.shell.sounds),
        vibrations: Signal::new(settings.shell.vibrations),
        reduced: Signal::new(settings.reduced),
    });
    gamekit::sounds(SOUNDS);
    let u = ui.clone();
    gamekit::autosave(SAVE, move || u.save.borrow().clone());
    let u = ui.clone();
    gamekit::on_background(SAVE, move || {
        if u.overlay.get_untracked() == Overlay::None {
            u.show(Overlay::Pause);
        }
    });
    let u = ui.clone();
    Effect::new(move || {
        gamekit::save(
            SETTINGS,
            &Settings {
                shell: chrome::GameSettings {
                    sounds: u.sounds.get(),
                    vibrations: u.vibrations.get(),
                    instructions_shown: true,
                },
                reduced: u.reduced.get(),
            },
        )
    });
    if !settings.shell.instructions_shown {
        ui.push(Overlay::Help);
    }
    let u = ui.clone();
    let header = chrome::game_header(tr("nav_matchthree"), "mt-pause", move || {
        u.show(Overlay::Pause)
    });
    let mut stats = vec![];
    for (key, id, kind, color) in [
        ("gk_score", "mt-score", 0, ACCENT),
        ("mt_moves", "mt-moves", 1, Color::WHITE),
        ("mt_goal", "mt-goal", 2, MINT),
    ] {
        let u = ui.clone();
        stats.push(chrome::info_stat(
            tr(key),
            move || {
                u.repaint.track();
                let s = u.save.borrow();
                let g = &s.game;
                match kind {
                    0 => g.score.to_string(),
                    1 => g.moves.to_string(),
                    _ => STAGES[g.level].target.to_string(),
                }
            },
            color,
            id,
        ));
    }
    let u = ui.clone();
    let v = ui.clone();
    let progress_ui = ui.clone();
    let progress = canvas(move |d, size| {
        progress_ui.repaint.track();
        let save = progress_ui.save.borrow();
        let g = &save.game;
        let progress = (g.score as f64 / STAGES[g.level].target as f64).min(1.0);
        d.fill(
            Shape::RoundedRect(Rect::new(0.0, 2.0, size.width, 6.0), 3.0),
            Color::hex(0x392E50),
        );
        if progress > 0.0 {
            d.fill(
                Shape::RoundedRect(Rect::new(0.0, 2.0, size.width * progress, 6.0), 3.0),
                LinearGradient::horizontal(ACCENT, MINT),
            );
        }
    })
    .width(240.0)
    .height(10.0);
    let info = column((
        label(move || {
            u.repaint.track();
            let g = &u.save.borrow().game;
            format!(
                "{:02}  /  {}",
                g.level + 1,
                tr(STAGES[g.level].name).format()
            )
        })
        .font(Font::Subheadline)
        .bold()
        .color(MINT)
        .id("mt-level"),
        chrome::info_row(stats),
        progress,
        label(move || {
            v.repaint.track();
            let g = &v.save.borrow().game;
            objective(g)
        })
        .font(Font::Caption)
        .color(Color::hex(0xD4C8E9))
        .align(TextAlign::Center)
        .id("mt-objective"),
    ))
    .spacing(6.0)
    .any();
    let (h, m, s) = (ui.clone(), ui.clone(), ui.clone());
    let footer = column((
        label(move || {
            s.repaint.track();
            if let Some(f) = s.frames.borrow().front() {
                if f.phase == Phase::Shuffle {
                    return tr("mt_shuffling").format();
                }
                if f.chain > 1 {
                    return tr("mt_cascade").arg("n", f.chain as f64).format();
                }
                return tr("mt_matching").format();
            }
            tr("mt_swap_hint").format()
        })
        .font(Font::Caption)
        .color(Color::hex(0xC2B3D5))
        .id("mt-status"),
        row((
            footer_button(tr("mt_hint"), "mt-hint", move || h.hint()),
            footer_button(tr("mt_levels"), "mt-map", move || m.push(Overlay::Map)),
        ))
        .spacing(24.0),
    ))
    .spacing(8.0)
    .padding(12.0)
    .any();
    let content = chrome::game_frame(header, Some(info), board(ui.clone()), Some(footer));
    let (u, v) = (ui.clone(), ui.clone());
    let clock = when(
        move || u.overlay.get() == Overlay::None,
        move || {
            let u = v.clone();
            let step = u.clone();
            gamekit::animation::clock(
                move || {
                    u.repaint.track();
                    u.overlay.get() == Overlay::None && u.needs_frame()
                },
                move |dt| step.tick(dt.as_secs_f64().min(0.1)),
            )
        },
    );
    zstack((content, overlays(ui), clock))
        .background(SURFACE)
        .any()
}
fn objective(g: &Game) -> String {
    let level = STAGES[g.level];
    tr(level.detail)
        .arg("berries", g.collected.min(level.collect) as f64)
        .arg("target", level.collect as f64)
        .arg("frost", g.frosting() as f64)
        .format()
}
fn board(ui: Rc<Ui>) -> AnyPiece {
    let (d, t, g, k) = (ui.clone(), ui.clone(), ui.clone(), ui.clone());
    canvas(move |draw, size| {
        d.repaint.track();
        d.size.set(size);
        let save = d.save.borrow();
        let frames = d.frames.borrow();
        render(
            draw,
            size,
            &save.game,
            frames.front(),
            d.elapsed.get(),
            d.time.get(),
            d.selected.get(),
            Some(d.cursor.get()),
            d.hint.get(),
            d.reduced.get_untracked(),
            d.preview(),
            d.settling.get(),
        );
    })
    .on_tap_at(move |p| {
        if t.time.get() < t.suppress_tap_until.get() {
            return;
        }
        if let Some(i) = t.at(p) {
            t.choose(i);
        }
    })
    .on_drag(move |e| g.gesture(e))
    .on_key(move |e| k.key(&e.key))
    .focused(ui.focus)
    .a11y(|a| a.label(tr("mt_board_a11y").format()))
    .id("mt-board")
    .grow()
    .any()
}
fn layout(s: Size) -> (f64, f64, f64) {
    let side = (s.width.min(s.height) - 24.0).clamp(0.0, 560.0);
    (
        (s.width - side) / 2.0,
        (s.height - side) / 2.0,
        side / N as f64,
    )
}
fn ellipse(x: f64, y: f64, w: f64, h: f64) -> Shape {
    Shape::Ellipse(Rect::new(x, y, w, h))
}
fn star(x: f64, y: f64, r: f64, points: usize) -> Shape {
    Shape::Polygon(
        (0..points * 2)
            .map(|i| {
                let a =
                    i as f64 * std::f64::consts::PI / points as f64 - std::f64::consts::FRAC_PI_2;
                let r = if i % 2 == 0 { r } else { r * 0.48 };
                Point::new(x + a.cos() * r, y + a.sin() * r)
            })
            .collect(),
    )
}
/// Subtle bevels and highlights give each geometric gem depth without a candy finish.
fn draw_piece(d: &mut Draw, c: Candy, x: f64, y: f64, s: f64, scale: f64) {
    let r = s * 0.36 * scale;
    let color = Color::hex(PALETTE[c.color as usize]);
    let shape = |x: f64, y: f64| match c.color {
        0 => ellipse(x - r * 0.88, y - r * 0.88, r * 1.76, r * 1.76),
        1 => Shape::Polygon(vec![
            Point::new(x, y - r),
            Point::new(x + r * 0.85, y),
            Point::new(x, y + r),
            Point::new(x - r * 0.85, y),
        ]),
        2 => star(x, y, r * 1.08, 5),
        3 => Shape::RoundedRect(Rect::new(x - r * 0.85, y - r, r * 1.7, r * 2.0), r * 0.4),
        4 => ellipse(x - r, y - r * 0.72, r * 2.0, r * 1.44),
        _ => Shape::Polygon(
            (0..6)
                .map(|i| {
                    let a = i as f64 * std::f64::consts::TAU / 6.0;
                    Point::new(x + r * a.cos(), y + r * a.sin())
                })
                .collect(),
        ),
    };
    d.fill(shape(x, y + 2.0), Color::rgba(0.02, 0.01, 0.06, 0.32));
    if c.special == Special::Rainbow {
        d.fill(
            ellipse(x - r, y - r, r * 2.0, r * 2.0),
            Color::hex(0x596377),
        );
        for i in 0..12 {
            let a = i as f64 * std::f64::consts::TAU / 12.0;
            d.fill(
                ellipse(
                    x + a.cos() * r * 0.66 - r * 0.15,
                    y + a.sin() * r * 0.66 - r * 0.15,
                    r * 0.3,
                    r * 0.3,
                ),
                Color::hex(PALETTE[i % 6]),
            );
        }
        d.fill(star(x, y, r * 0.5, 4), Color::WHITE);
        return;
    }
    let light = Color::rgb(
        color.r * 0.88 + 0.12,
        color.g * 0.88 + 0.12,
        color.b * 0.88 + 0.12,
    );
    let shade = Color::rgb(color.r * 0.76, color.g * 0.76, color.b * 0.76);
    d.fill(shape(x, y), LinearGradient::vertical(light, shade));
    d.stroke(shape(x, y), color.with_alpha(0.8), 1.0);
    d.clipped(shape(x, y), |d| {
        d.fill(
            ellipse(x - r * 0.52, y - r * 0.64, r * 0.76, r * 0.24),
            Color::WHITE.with_alpha(0.23),
        );
    });
    match c.special {
        Special::Row | Special::Column => {
            for offset in [-0.28, 0.0, 0.28] {
                let (a, b) = if c.special == Special::Row {
                    (
                        Point::new(x - r * 0.65, y + r * offset),
                        Point::new(x + r * 0.65, y + r * offset),
                    )
                } else {
                    (
                        Point::new(x + r * offset, y - r * 0.65),
                        Point::new(x + r * offset, y + r * 0.65),
                    )
                };
                d.stroke(Shape::Line(a, b), Color::WHITE.with_alpha(0.9), r * 0.12);
            }
        }
        Special::Wrapped => {
            d.stroke(
                Shape::RoundedRect(
                    Rect::new(x - r * 0.58, y - r * 0.58, r * 1.16, r * 1.16),
                    r * 0.2,
                ),
                Color::WHITE,
                2.0,
            );
            d.fill(star(x, y, r * 0.36, 4), Color::WHITE);
        }
        _ => {}
    }
}
#[allow(clippy::too_many_arguments)]
fn render(
    d: &mut Draw,
    size: Size,
    g: &Game,
    frame: Option<&Frame>,
    elapsed: f64,
    time: f64,
    selected: Option<usize>,
    cursor: Option<usize>,
    hint: Option<(usize, usize)>,
    reduced: bool,
    preview: Option<Preview>,
    settling: Option<Preview>,
) {
    let (ox, oy, c) = layout(size);
    if c < 2.0 {
        return;
    }
    let side = c * N as f64;
    // A quiet board frame keeps attention on the moving pieces.
    let accent = Color::hex([0x8298B8, 0x719C9F, 0x99A8BB, 0xAC9B82, 0x9A91B1, 0x9D929E][g.level]);
    d.fill(
        Shape::RoundedRect(
            Rect::new(ox - 8.0, oy - 8.0, side + 16.0, side + 16.0),
            20.0,
        ),
        Color::hex(0x292738),
    );
    d.stroke(
        Shape::RoundedRect(
            Rect::new(ox - 8.0, oy - 8.0, side + 16.0, side + 16.0),
            20.0,
        ),
        accent.with_alpha(0.45),
        1.0,
    );
    let (grid, frost) = frame.map_or((&g.grid, &g.frost), |f| (&f.grid, &f.frost));
    for i in 0..LEN {
        let (x, y) = (ox + (i % N) as f64 * c, oy + (i / N) as f64 * c);
        if !active(g.level, i) {
            d.fill(
                ellipse(x + c * 0.47, y + c * 0.47, c * 0.06, c * 0.06),
                ACCENT.with_alpha(0.2),
            );
            continue;
        }
        let tile = Shape::RoundedRect(Rect::new(x + 1.5, y + 1.5, c - 3.0, c - 3.0), c * 0.16);
        d.fill(
            tile.clone(),
            if (i / N + i % N) % 2 == 0 {
                Color::hex(0x28223E)
            } else {
                Color::hex(0x302742)
            },
        );
        if frost[i] > 0 {
            d.fill(
                tile.clone(),
                LinearGradient::vertical(
                    Color::hex(0xBDDDF3).with_alpha(0.48),
                    Color::hex(0x8A6DBD).with_alpha(0.48),
                ),
            );
            d.stroke(
                tile.clone(),
                Color::hex(0xDCEBFF).with_alpha(0.7),
                if frost[i] == 2 { 3.0 } else { 1.0 },
            );
            for k in 0..frost[i] {
                d.fill(
                    star(x + c * (0.2 + k as f64 * 0.6), y + c * 0.8, c * 0.09, 6),
                    Color::WHITE.with_alpha(0.6),
                );
            }
        }
        if selected == Some(i)
            || hint.is_some_and(|(a, b)| i == a || i == b)
            || preview.is_some_and(|p| p.from == i || p.to == Some(i))
        {
            d.stroke(
                tile.clone(),
                MINT,
                if reduced {
                    2.5
                } else {
                    2.5 + 0.35 * (time * 4.0).sin()
                },
            );
            d.fill(tile.clone(), MINT.with_alpha(0.12));
        } else if cursor == Some(i) {
            d.stroke(tile, Color::hex(0xB99ACF).with_alpha(0.6), 1.0);
        }
    }
    // The held gem paints last so it follows the finger above every neighbor.
    let held = preview.map(|p| p.from);
    for i in (0..LEN).filter(|&i| held != Some(i)).chain(held) {
        let Some(cv) = grid[i].as_ref() else { continue };
        let (mut x, mut y) = (
            ox + (i % N) as f64 * c + c / 2.0,
            oy + (i / N) as f64 * c + c / 2.0,
        );
        let mut scale = 1.0;
        if !reduced {
            if let Some(f) = frame {
                match f.phase {
                    Phase::Swap | Phase::Return | Phase::Fall => {
                        let dur = if f.phase == Phase::Fall { 0.32 } else { 0.18 };
                        let t = (elapsed / dur).clamp(0.0, 1.0);
                        let p = 1.0 - (1.0 - t).powi(3);
                        if let Some(&(from, _)) = f.motion.iter().find(|&&(_, to)| to == i) {
                            let (mut fx, mut fy) = if from >= LEN {
                                (x, y - c * (from - LEN) as f64)
                            } else {
                                (
                                    ox + (from % N) as f64 * c + c / 2.0,
                                    oy + (from / N) as f64 * c + c / 2.0,
                                )
                            };
                            if f.phase == Phase::Swap {
                                if let Some(preview) = settling {
                                    let offset = preview.offset(from);
                                    fx += offset.x * c;
                                    fy += offset.y * c;
                                }
                            }
                            x = fx + (x - fx) * p;
                            y = fy + (y - fy) * p;
                            if f.phase == Phase::Fall {
                                scale = 1.0 + 0.08 * (t * std::f64::consts::PI * 2.0).sin();
                            }
                        }
                    }
                    Phase::Burst => {
                        if f.cleared.contains(&i) {
                            let t = (elapsed / 0.28).clamp(0.0, 1.0);
                            scale = (1.0 - t) * (1.0 + 0.5 * (t * std::f64::consts::PI).sin());
                            for k in 0..7 {
                                let a = k as f64 * std::f64::consts::TAU / 7.0 + i as f64;
                                let r = c * t * 0.85;
                                d.fill(
                                    star(x + r * a.cos(), y + r * a.sin(), c * 0.07 * (1.0 - t), 4),
                                    Color::hex(PALETTE[cv.color as usize]).with_alpha(1.0 - t),
                                );
                            }
                            if cv.special == Special::Row || cv.special == Special::Column {
                                let (a, b) = if cv.special == Special::Row {
                                    (Point::new(ox, y), Point::new(ox + side, y))
                                } else {
                                    (Point::new(x, oy), Point::new(x, oy + side))
                                };
                                d.stroke(
                                    Shape::Line(a, b),
                                    Color::WHITE.with_alpha(1.0 - t),
                                    c * 0.12 * (1.0 - t),
                                );
                            }
                        }
                    }
                    Phase::Shuffle => scale = (elapsed / 0.4).clamp(0.1, 1.0),
                }
            }
        }
        if let Some(preview) = preview {
            let offset = preview.offset(i);
            x += offset.x * c;
            y += offset.y * c;
            if i == preview.from {
                scale = 1.04;
            }
        }
        d.clipped(
            Shape::RoundedRect(Rect::new(ox, oy, side, side), 12.0),
            |d| draw_piece(d, *cv, x, y, c, scale),
        );
    }
    if let Some(f) = frame {
        if f.phase == Phase::Burst {
            let at = Point::new(size.width / 2.0, oy + side * 0.42 - elapsed * 20.0);
            let text = format!("+{}", f.points);
            d.text(
                &text,
                Point::new(at.x + 1.0, at.y + 2.0),
                TextStyle {
                    size: c * 0.65,
                    color: SURFACE,
                    anchor: TextAnchor::CENTERED,
                    font: chrome::canvas_font(FontWeight::Black),
                },
            );
            d.text(
                &text,
                at,
                TextStyle {
                    size: c * 0.65,
                    color: Color::WHITE,
                    anchor: TextAnchor::CENTERED,
                    font: chrome::canvas_font(FontWeight::Black),
                },
            );
        }
    }
}
fn overlays(ui: Rc<Ui>) -> AnyPiece {
    let u = ui.clone();
    let mut layers = vec![when(move || u.overlay.get() != Overlay::None, chrome::scrim).any()];
    for kind in [
        Overlay::Pause,
        Overlay::Map,
        Overlay::Help,
        Overlay::Settings,
        Overlay::Result,
    ] {
        let (u, v) = (ui.clone(), ui.clone());
        layers.push(
            when(
                move || u.overlay.get() == kind,
                move || card(v.clone(), kind),
            )
            .any(),
        );
    }
    zstack(PieceVec(layers)).any()
}
fn footer_button(title: LocalizedText, id: &'static str, action: impl Fn() + 'static) -> AnyPiece {
    let name = title.format();
    label(title)
        .font(Font::Headline)
        .color(Color::WHITE)
        .align(TextAlign::Center)
        .padding(10.0)
        .min_width(90.0)
        .background(Color::hex(0x343047))
        .corner_radius(10.0)
        .on_tap(action)
        .a11y(move |a| a.label(name).role(Role::Button))
        .id(id)
        .any()
}

/// Explicit control colors keep their contrast in native light and dark appearances.
fn menu_button(
    title: LocalizedText,
    tint: Color,
    id: &'static str,
    action: impl Fn() + 'static,
) -> AnyPiece {
    let name = title.format();
    label(title)
        .font(Font::Headline)
        .bold()
        .color(Color::WHITE)
        .align(TextAlign::Center)
        .padding(12.0)
        .width(260.0)
        .background(tint.with_alpha(0.3))
        .corner_radius(14.0)
        .on_tap(action)
        .a11y(move |a| a.label(name).role(Role::Button))
        .id(id)
        .any()
}
fn card(ui: Rc<Ui>, kind: Overlay) -> AnyPiece {
    if kind == Overlay::Help {
        return chrome::instructions_card(
            tr("nav_matchthree"),
            vec![
                Help::Para(tr("mt_help_swap")),
                Help::Heading(tr("mt_help_special_title")),
                Help::Para(tr("mt_help_special")),
                Help::Para(tr("mt_help_combo")),
                Help::Heading(tr("mt_levels")),
                Help::Para(tr("mt_help_goals")),
                Help::Para(tr("mt_help_keys")),
            ],
            "mt-help-done",
            move || ui.show(ui.back.get()),
        )
        .id("mt-help")
        .any();
    }
    let mut items = vec![];
    let nav = |key: &'static str, id: &'static str, o: Overlay| {
        let u = ui.clone();
        menu_button(tr(key), chrome::INDIGO, id, move || u.push(o))
    };
    match kind {
        Overlay::Pause => {
            items.push(chrome::card_title(tr("nav_matchthree"), ACCENT));
            let u = ui.clone();
            items.push(menu_button(
                tr("gk_resume"),
                chrome::GREEN,
                "mt-resume",
                move || u.show(Overlay::None),
            ));
            items.push(nav("mt_levels", "mt-levels", Overlay::Map));
            let u = ui.clone();
            items.push(menu_button(
                tr("mt_retry"),
                chrome::AMBER,
                "mt-restart",
                move || {
                    let level = u.save.borrow().game.level;
                    u.start(level);
                },
            ));
            items.push(nav("gk_settings", "mt-settings", Overlay::Settings));
            items.push(nav("gk_instructions", "mt-instructions", Overlay::Help));
        }
        Overlay::Map => {
            items.push(chrome::card_title(tr("mt_levels"), ACCENT));
            items.push(
                label(tr("mt_map_intro"))
                    .font(Font::Caption)
                    .color(chrome::TEXT_DIM)
                    .any(),
            );
            for (i, stage) in STAGES.iter().enumerate() {
                let s = ui.save.borrow();
                let unlocked = s.unlocked(i);
                let stars = s.stars[i];
                drop(s);
                let title = format!("{:02}  {}", i + 1, tr(stage.name).format());
                let detail = if unlocked {
                    if stars == 0 {
                        "◇  ◇  ◇".into()
                    } else {
                        "★".repeat(stars as usize)
                    }
                } else {
                    tr("mt_locked").format()
                };
                let u = ui.clone();
                items.push(
                    row((
                        label(title)
                            .bold()
                            .color(if unlocked {
                                Color::WHITE
                            } else {
                                chrome::TEXT_DIM
                            })
                            .grow_w(),
                        label(detail).font(Font::Caption).color(if unlocked {
                            chrome::GOLD
                        } else {
                            chrome::TEXT_DIM
                        }),
                    ))
                    .spacing(8.0)
                    .padding(10.0)
                    .background(Color::hex(PALETTE[i]).with_alpha(if unlocked {
                        0.18
                    } else {
                        0.06
                    }))
                    .corner_radius(12.0)
                    .on_tap(move || u.start(i))
                    .a11y(move |a| a.label(tr(stage.name).format()).role(Role::Button))
                    .id(format!("mt-stage-{}", i + 1))
                    .width(260.0)
                    .any(),
                );
            }
            let u = ui.clone();
            items.push(menu_button(
                tr("gk_done"),
                chrome::GREEN,
                "mt-map-done",
                move || u.show(u.back.get()),
            ));
        }
        Overlay::Settings => {
            items.push(chrome::card_title(tr("gk_settings"), Color::WHITE));
            items.push(chrome::setting_row(
                tr("gk_sounds"),
                toggle(ui.sounds).id("mt-sounds").any(),
            ));
            items.push(chrome::setting_row(
                tr("gk_vibrations"),
                toggle(ui.vibrations).id("mt-vibrations").any(),
            ));
            items.push(chrome::setting_row(
                tr("mt_reduced"),
                toggle(ui.reduced).id("mt-reduced").any(),
            ));
            let u = ui.clone();
            items.push(menu_button(
                tr("gk_done"),
                chrome::GREEN,
                "mt-settings-done",
                move || u.show(u.back.get()),
            ));
        }
        Overlay::Result => {
            let s = ui.save.borrow();
            let g = &s.game;
            let won = g.won();
            let level = g.level;
            let stars = g.stars();
            items.push(chrome::card_title(
                tr(if won {
                    if level == LEVELS - 1 {
                        "mt_complete"
                    } else {
                        "mt_win"
                    }
                } else {
                    "mt_out"
                }),
                if won { ACCENT } else { Color::WHITE },
            ));
            if won {
                items.push(celebration(ui.reduced.get_untracked()));
            }
            items.push(
                label(if won {
                    "★".repeat(stars as usize)
                } else {
                    "◇  ◇  ◇".to_string()
                })
                .font(Font::LargeTitle)
                .color(chrome::GOLD)
                .id("mt-stars")
                .any(),
            );
            items.push(chrome::stat(
                tr("gk_score"),
                g.score.to_string(),
                Font::LargeTitle,
                MINT,
                "mt-final-score",
            ));
            items.push(
                label(tr(if won {
                    if level == LEVELS - 1 {
                        "mt_complete_detail"
                    } else {
                        "mt_win_detail"
                    }
                } else {
                    "mt_retry_detail"
                }))
                .color(chrome::TEXT_DIM)
                .align(TextAlign::Center)
                .any(),
            );
            if won && level + 1 < LEVELS {
                let u = ui.clone();
                items.push(menu_button(
                    tr("mt_next"),
                    chrome::GREEN,
                    "mt-next",
                    move || u.start(level + 1),
                ));
            }
            let u = ui.clone();
            items.push(menu_button(
                tr("mt_retry"),
                chrome::INDIGO,
                "mt-play-again",
                move || u.start(level),
            ));
            items.push(nav("mt_levels", "mt-result-map", Overlay::Map));
        }
        _ => {}
    }
    let id = match kind {
        Overlay::Pause => "mt-pause-card",
        Overlay::Map => "mt-levels-card",
        Overlay::Settings => "mt-settings-card",
        _ => "mt-result",
    };
    chrome::card(column(PieceVec(items)).spacing(12.0).align(HAlign::Center))
        .id(id)
        .any()
}
fn celebration(reduced: bool) -> AnyPiece {
    let time = Signal::new(0.0_f64);
    let art = canvas(move |d, size| {
        let t = time.get();
        let x = size.width / 2.0;
        let y = size.height / 2.0;
        d.fill(
            ellipse(x - 54.0, y - 54.0, 108.0, 108.0),
            RadialGradient::centered(chrome::GOLD.with_alpha(0.4), SURFACE.with_alpha(0.0)),
        );
        for i in 0..30 {
            let k = i as f64;
            let px = (k * 61.7) % size.width;
            let py = if reduced {
                (k * 37.3) % size.height
            } else {
                (k * 37.3 + t * (15.0 + (i % 5) as f64 * 8.0)) % size.height
            };
            d.fill(
                star(px, py, 2.5 + (i % 3) as f64, 4),
                Color::hex(PALETTE[i % 6]).with_alpha(0.65),
            );
        }
        d.fill(star(x, y + 3.0, 43.0, 5), Color::hex(0xBD651E));
        d.fill(
            star(x, y, 43.0, 5),
            LinearGradient::vertical(Color::hex(0xFFF4B8), chrome::GOLD),
        );
        d.fill(
            star(x - 9.0, y - 12.0, 8.0, 4),
            Color::WHITE.with_alpha(0.8),
        );
    })
    .width(270.0)
    .height(108.0);
    zstack((
        art,
        when(
            move || !reduced,
            move || {
                gamekit::animation::clock(
                    || true,
                    move |dt| time.set(time.get_untracked() + dt.as_secs_f64().min(0.1)),
                )
            },
        ),
    ))
    .any()
}
pub fn matchthree_preview() -> AnyPiece {
    let mut g = Game::new(0, 15);
    g.grid[16].as_mut().unwrap().special = Special::Row;
    g.grid[24].as_mut().unwrap().special = Special::Rainbow;
    g.grid[32].as_mut().unwrap().special = Special::Wrapped;
    canvas(move |d, size| {
        render(
            d, size, &g, None, 0.0, 0.0, None, None, None, true, None, None,
        )
    })
    .background(SURFACE)
    .any()
}
