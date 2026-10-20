// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Pac-Pac: Pac-Pac and Ms. Pac-Pac, picked when a game starts, on an immediate-mode canvas
//! that draws the arcade's 224×288 screen pixel for pixel and scales it to whatever the window
//! gives it. The rules live in `model` (the arcade's dossier's tables and timings), the pixels in
//! `art`. Swipe anywhere on the screen or use the arrow keys to steer; the sirens, the chomps,
//! the ghosts' retreat and the deaths all sound as they did, with haptics beside them. A
//! composite Day piece: pure composition over `day_pieces` plus the shared `gamekit` chrome, so it
//! renders the same on every backend.

mod art;
mod model;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use day_fluent::tr;
use day_part_haptics::Haptic;
use day_pieces::prelude::*;
use gamekit::chrome::cues::{self, with};
use gamekit::chrome::{self, Cue, Feedback, Help, Sfx, sfx};

use art::{MAZE_TOP, SCREEN_H, SCREEN_W, TILE};
use day_spec::{LineCap, LineJoin, StrokeStyle, WidthClass};
use model::{Dir, Game, GhostState, Happening, Mode, Phase, SaveState};

/// The prefs keys this game persists under (gamekit; bump the game key on schema change).
const SAVE_KEY: &str = "pacpac.v1";
const SETTINGS_KEY: &str = "pacpac.settings";
const BEST_PACPAC_KEY: &str = "pacpac.best.pacpac";
const BEST_MSPACPAC_KEY: &str = "pacpac.best.mspacpac";

/// The game's cover surface color (edge-to-edge behind the safe area): the arcade's black.
pub const SURFACE: Color = Color::rgb(0.0, 0.0, 0.0);

/// A swipe this long steers; the finger can keep going and steer again.
const SWIPE: f64 = 18.0;
/// The joypad under the maze on a phone: its canvas, its ring, and the thumb's reach.
const PAD_W: f64 = 180.0;
const PAD_H: f64 = 150.0;
const PAD_RING: f64 = 62.0;
const PAD_REACH: f64 = 44.0;
/// A thumb this far from the pad's center steers.
const PAD_DEAD: f64 = 10.0;

// Sounds, each with the haptic it plays beside (gamekit::chrome::Cue). The clips are
// synthesized by scripts/pacpac-sounds.py in the arcade's waveforms.
static INTRO: Cue = with("sounds/pacpac/intro.wav", cues::MEDIUM_BEAT);
static MS_INTRO: Cue = with("sounds/pacpac/ms_intro.wav", cues::MEDIUM_BEAT);
static EAT_GHOST: Cue = with("sounds/pacpac/eat_ghost.wav", chrome::CELEBRATE);
static EAT_FRUIT: Cue = with("sounds/pacpac/eat_fruit.wav", cues::SUCCESS_BEAT);
static EXTRA_LIFE: Cue = with("sounds/pacpac/extra_life.wav", chrome::BIG_CELEBRATE);
static DEATH: Cue = with("sounds/pacpac/death.wav", chrome::LETDOWN);
static MS_DEATH: Cue = with("sounds/pacpac/ms_death.wav", chrome::LETDOWN);
static CHOMP_A: Sfx = sfx("sounds/pacpac/chomp_a.wav");
static CHOMP_B: Sfx = sfx("sounds/pacpac/chomp_b.wav");
static MS_CHOMP: Sfx = sfx("sounds/pacpac/ms_chomp.wav");
static SIRENS: [Sfx; 5] = [
    sfx("sounds/pacpac/siren_1.wav"),
    sfx("sounds/pacpac/siren_2.wav"),
    sfx("sounds/pacpac/siren_3.wav"),
    sfx("sounds/pacpac/siren_4.wav"),
    sfx("sounds/pacpac/siren_5.wav"),
];
static MS_SIRENS: [Sfx; 5] = [
    sfx("sounds/pacpac/ms_siren_1.wav"),
    sfx("sounds/pacpac/ms_siren_2.wav"),
    sfx("sounds/pacpac/ms_siren_3.wav"),
    sfx("sounds/pacpac/ms_siren_4.wav"),
    sfx("sounds/pacpac/ms_siren_5.wav"),
];
static FRIGHT_LOOP: Sfx = sfx("sounds/pacpac/fright.wav");
static MS_FRIGHT_LOOP: Sfx = sfx("sounds/pacpac/ms_fright.wav");
static EYES_LOOP: Sfx = sfx("sounds/pacpac/eyes.wav");

/// Every clip this game plays besides the shared ones (gamekit preloads both).
pub const SOUNDS: &[Sfx] = &[
    sfx("sounds/pacpac/intro.wav"),
    sfx("sounds/pacpac/ms_intro.wav"),
    sfx("sounds/pacpac/eat_ghost.wav"),
    sfx("sounds/pacpac/eat_fruit.wav"),
    sfx("sounds/pacpac/extra_life.wav"),
    sfx("sounds/pacpac/death.wav"),
    sfx("sounds/pacpac/ms_death.wav"),
    sfx("sounds/pacpac/chomp_a.wav"),
    sfx("sounds/pacpac/chomp_b.wav"),
    sfx("sounds/pacpac/ms_chomp.wav"),
    sfx("sounds/pacpac/siren_1.wav"),
    sfx("sounds/pacpac/siren_2.wav"),
    sfx("sounds/pacpac/siren_3.wav"),
    sfx("sounds/pacpac/siren_4.wav"),
    sfx("sounds/pacpac/siren_5.wav"),
    sfx("sounds/pacpac/ms_siren_1.wav"),
    sfx("sounds/pacpac/ms_siren_2.wav"),
    sfx("sounds/pacpac/ms_siren_3.wav"),
    sfx("sounds/pacpac/ms_siren_4.wav"),
    sfx("sounds/pacpac/ms_siren_5.wav"),
    sfx("sounds/pacpac/fright.wav"),
    sfx("sounds/pacpac/ms_fright.wav"),
    sfx("sounds/pacpac/eyes.wav"),
];

/// The clips played as loops, which must each close on themselves (the app's sound test checks).
pub const LOOPS: &[Sfx] = &[
    sfx("sounds/pacpac/siren_1.wav"),
    sfx("sounds/pacpac/siren_2.wav"),
    sfx("sounds/pacpac/siren_3.wav"),
    sfx("sounds/pacpac/siren_4.wav"),
    sfx("sounds/pacpac/siren_5.wav"),
    sfx("sounds/pacpac/ms_siren_1.wav"),
    sfx("sounds/pacpac/ms_siren_2.wav"),
    sfx("sounds/pacpac/ms_siren_3.wav"),
    sfx("sounds/pacpac/ms_siren_4.wav"),
    sfx("sounds/pacpac/ms_siren_5.wav"),
    sfx("sounds/pacpac/fright.wav"),
    sfx("sounds/pacpac/ms_fright.wav"),
    sfx("sounds/pacpac/eyes.wav"),
];

/// The siren, the fright warble and the eyes' retreat are loops: one second of whole cycles
/// each, repeated by the sound engine until the round has something else to say. They sit well
/// under the effects.
const LOOP_VOLUME: f32 = 0.35;

// ---------------------------------------------------------------------------
// Drawing the screen
// ---------------------------------------------------------------------------

/// Where the 224×288 screen sits in a canvas of `sz`: scaled to fit, snapped to half-points so
/// the pixels stay whole on 1× and 2× displays, and centered.
fn placement(sz: Size) -> Affine {
    let raw = (sz.width / SCREEN_W).min(sz.height / SCREEN_H);
    let s = if raw >= 1.0 {
        (raw * 2.0).floor() / 2.0
    } else {
        ((raw * 16.0).floor() / 16.0).max(0.125)
    };
    let ox = ((sz.width - SCREEN_W * s) / 2.0 * 2.0).floor() / 2.0;
    let oy = ((sz.height - SCREEN_H * s) / 2.0 * 2.0).floor() / 2.0;
    Affine::scale(s, s).then(Affine::translate(ox, oy))
}

/// The maze layer: the walls (blinking white when a level is cleared) and the dots left.
fn draw_maze_layer(g: &Game, d: &mut Draw) {
    d.fill(
        Shape::Rect(Rect::new(0.0, 0.0, SCREEN_W, SCREEN_H)),
        art::BLACK,
    );
    d.transformed(Affine::translate(0.0, MAZE_TOP), |d| {
        g.maze.draw_walls(d, g.walls_white());
        g.maze.draw_dots(d, &g.eaten);
    });
}

/// The play layer: the score rows, the energizers, the fruit, the actors, the score popups, the
/// messages, the lives and the level fruit.
fn draw_play_layer(g: &Game, d: &mut Draw) {
    let sheet = art::sheet();
    let ms = g.mode == Mode::MsPacPac;
    let blink_on = ((g.time / 0.2).floor() as i64) % 2 == 0;

    // The score rows.
    let one_up_on = !g.playing() || ((g.time / 0.25).floor() as i64) % 2 == 0;
    if one_up_on {
        sheet.text(d, "1UP", 24.0, 0.0, art::TEXT_WHITE);
    }
    sheet.text(d, "HIGH SCORE", 72.0, 0.0, art::TEXT_WHITE);
    let score = if g.score == 0 {
        "00".to_string()
    } else {
        g.score.to_string()
    };
    sheet.text_right(d, &score, 56.0, 8.0, art::TEXT_WHITE);
    if g.best > 0 {
        sheet.text_right(d, &g.best.to_string(), 136.0, 8.0, art::TEXT_WHITE);
    }

    let intro_empty = match g.phase {
        Phase::Ready { t, intro: true } => t > model::INTRO_EMPTY_SECS,
        _ => false,
    };

    d.transformed(Affine::translate(0.0, MAZE_TOP), |d| {
        // Energizers blink in play and sit still otherwise.
        if blink_on || !g.playing() {
            for (c, r) in g.maze.energizers(&g.eaten) {
                sheet
                    .energizer
                    .draw_tinted(d, c as f64 * TILE, r as f64 * TILE, Some(g.maze.dot));
            }
        }
        // The fruit, hopping if it wanders.
        if let Some(f) = &g.fruit {
            let hop = if f.wander {
                (2.0 * (g.time * 10.0).sin().abs()).round()
            } else {
                0.0
            };
            sheet.fruit(f.kind).draw(
                d,
                (f.actor.x - 8.0).round(),
                (f.actor.y - 8.0 - hop).round(),
            );
        }

        let ghost_popups: Vec<(f64, f64)> = g
            .popups
            .iter()
            .filter(|p| p.ghost)
            .map(|p| (p.x, p.y))
            .collect();
        let (show_ghosts, show_pac) = match g.phase {
            Phase::Ready { .. } => (!intro_empty, !intro_empty),
            Phase::Play | Phase::GhostEaten { .. } | Phase::DeathFreeze { .. } => (true, true),
            Phase::LevelFreeze { .. } => (true, true),
            Phase::Death { .. } => (false, true),
            Phase::LevelFlash { .. } => (false, true),
            Phase::GameOverText { .. } | Phase::GameOver => (false, false),
        };

        if show_ghosts {
            let frame = ((g.time / 0.13).floor() as usize) % 2;
            let white = g.fright_flash_white();
            for gh in &g.ghosts {
                let a = gh.actor;
                let hidden = matches!(g.phase, Phase::GhostEaten { .. })
                    && ghost_popups
                        .iter()
                        .any(|(x, y)| (*x - a.x).abs() < 0.5 && (*y - a.y).abs() < 0.5);
                if hidden {
                    continue;
                }
                let (x, y) = ((a.x - 8.0).round(), (a.y - 8.0).round());
                match gh.state {
                    GhostState::Eyes | GhostState::Entering => {
                        sheet.eyes[a.dir.index()].draw(d, x, y);
                    }
                    _ if gh.frightened => {
                        if white {
                            sheet.flash[frame].draw(d, x, y);
                            sheet.flash_face.draw(d, x, y);
                        } else {
                            sheet.fright[frame].draw(d, x, y);
                            sheet.fright_face.draw(d, x, y);
                        }
                    }
                    _ => {
                        sheet.ghost[gh.kind.index()][frame].draw(d, x, y);
                        sheet.eyes[a.dir.index()].draw(d, x, y);
                    }
                }
            }
        }

        if show_pac {
            let (x, y) = ((g.pac.x - 8.0).round(), (g.pac.y - 8.0).round());
            let hidden = matches!(g.phase, Phase::GhostEaten { .. });
            match g.phase {
                Phase::Death { t } => {
                    let frames = if ms {
                        &sheet.ms_pac_death
                    } else {
                        &sheet.pac_death
                    };
                    let i = ((t / model::DEATH_FRAME_SECS).floor() as usize).min(frames.len() - 1);
                    frames[i].draw(d, x, y);
                }
                Phase::LevelFlash { .. } | Phase::Ready { .. } => {
                    let faces = if ms { &sheet.ms_pac } else { &sheet.pac };
                    faces[g.pac.dir.index()][0].draw(d, x, y);
                }
                _ if !hidden => {
                    let faces = if ms { &sheet.ms_pac } else { &sheet.pac };
                    let cycle = [0usize, 1, 2, 1];
                    let frame = cycle[((g.pac_anim / 0.033).floor() as usize) % 4];
                    faces[g.pac.dir.index()][frame].draw(d, x, y);
                }
                _ => {}
            }
        }

        for p in &g.popups {
            let color = if p.ghost {
                art::SCORE_CYAN
            } else {
                art::SCORE_PINK
            };
            sheet.mini_number(d, p.value, p.x, (p.y - 2.0).round(), color);
        }

        // The messages, on the arcade's rows.
        match g.phase {
            Phase::Ready { .. } => {
                if intro_empty {
                    sheet.text(d, "PLAYER ONE", 72.0, 112.0, art::TEXT_CYAN);
                }
                sheet.text(d, "READY!", 88.0, 160.0, art::YELLOW);
            }
            Phase::GameOverText { .. } | Phase::GameOver => {
                sheet.text(d, "GAME  OVER", 72.0, 160.0, art::TEXT_RED);
            }
            _ => {}
        }
    });

    // Lives and the level fruit along the bottom row.
    let bottom = MAZE_TOP + art::MAZE_H;
    let spare = (g.lives - 1).clamp(0, 6);
    let life = if ms { &sheet.ms_life } else { &sheet.life };
    for i in 0..spare {
        life.draw(d, 16.0 + i as f64 * 16.0, bottom);
    }
    let recent = g.history.iter().rev().take(7);
    for (i, fruit) in recent.enumerate() {
        sheet.fruit(*fruit).draw(d, 176.0 - i as f64 * 16.0, bottom);
    }
}

/// The home-grid tile preview: the renderer itself over a mid-level moment of Pac-Pac's maze.
pub fn pacpac_preview() -> AnyPiece {
    canvas(|d, sz| {
        if sz.width < 4.0 || sz.height < 4.0 {
            return;
        }
        let mut g = Game::new(Mode::PacPac, 0x5EED_0002);
        g.happenings.clear();
        g.phase = Phase::Play;
        g.time = 0.1;
        g.score = 1480;
        for (i, t) in g.maze.tiles.clone().iter().enumerate() {
            let (c, r) = (i % art::COLS, i / art::COLS);
            let gone = r <= 8 && c >= 6 || (r >= 20 && c < 13);
            if gone && matches!(t, art::Tile::Dot | art::Tile::Energizer) {
                g.eaten[i] = true;
                g.dots_eaten += 1;
            }
        }
        g.pac = model::Actor {
            x: 76.0,
            y: 188.0,
            dir: Dir::Right,
        };
        g.pac_anim = 0.07;
        g.ghosts[0].actor = model::Actor {
            x: 148.0,
            y: 188.0,
            dir: Dir::Left,
        };
        g.ghosts[1].state = GhostState::Roam;
        g.ghosts[1].actor = model::Actor {
            x: 52.0,
            y: 44.0,
            dir: Dir::Down,
        };
        g.ghosts[2].state = GhostState::Roam;
        g.ghosts[2].frightened = true;
        g.fright_t = 4.0;
        g.ghosts[2].actor = model::Actor {
            x: 172.0,
            y: 68.0,
            dir: Dir::Left,
        };
        g.ghosts[3].state = GhostState::Roam;
        g.ghosts[3].actor = model::Actor {
            x: 212.0,
            y: 236.0,
            dir: Dir::Up,
        };
        // The maze alone fills the square tile.
        let maze = Size::new(SCREEN_W, art::MAZE_H);
        let s = (sz.width / maze.width).min(sz.height / maze.height);
        let ox = (sz.width - maze.width * s) / 2.0;
        let oy = (sz.height - maze.height * s) / 2.0 - MAZE_TOP * s;
        d.fill(
            Shape::Rect(Rect::new(0.0, 0.0, sz.width, sz.height)),
            art::BLACK,
        );
        d.transformed(Affine::scale(s, s).then(Affine::translate(ox, oy)), |d| {
            d.transformed(Affine::translate(0.0, MAZE_TOP), |d| {
                g.maze.draw_walls(d, false);
                g.maze.draw_dots(d, &g.eaten);
            });
            draw_play_layer(&g, d);
        });
    })
    .any()
}

// ---------------------------------------------------------------------------
// The page
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Overlay {
    None,
    Pause,
    ModePicker,
    GameOver,
    Settings,
    Instructions,
}

struct Ui {
    game: Rc<RefCell<Game>>,
    /// The play layer: every frame while the round runs.
    repaint: Trigger,
    /// The maze layer: when a dot goes or the walls blink.
    maze: Trigger,
    seen_maze: Cell<u32>,
    overlay: Signal<Overlay>,
    /// Where Cancel on the mode picker goes back to.
    return_to: Cell<Overlay>,
    sounds: Signal<bool>,
    vibrations: Signal<bool>,
    /// Where the finger last steered from, while it is down.
    swipe_from: Cell<Option<Point>>,
    /// The loop the round wants now, and the handle that keeps it sounding (none while the
    /// Sounds setting is off). Dropping the handle stops the loop.
    voice: RefCell<Option<(&'static Sfx, Option<chrome::SfxLoop>)>>,
    chomp_b: Cell<bool>,
    /// No game had been saved when the page opened, so the rules lead to the mode picker.
    fresh: Cell<bool>,
    /// A pointer has hovered the screen, so a mouse or trackpad is at hand.
    pointer_seen: Signal<bool>,
    /// A key has been pressed, so a keyboard is at hand.
    keyboard_seen: Signal<bool>,
    /// Where the thumb is on the pad while a finger is down, in pad coordinates.
    thumb: Cell<Option<Point>>,
    pad_repaint: Trigger,
}

impl Ui {
    fn haptic(&self, h: Haptic) {
        chrome::haptic(self.vibrations.get_untracked(), h);
    }
    fn feedback(&self) -> Feedback {
        Feedback {
            sounds: self.sounds.get_untracked(),
            vibrations: self.vibrations.get_untracked(),
        }
    }
    fn cue(&self, c: &Cue) {
        chrome::cue(self.feedback(), c);
    }
    fn sound(&self, clip: &Sfx, volume: f32) {
        chrome::sound(self.sounds.get_untracked(), clip, volume);
    }

    fn sync_maze(&self) {
        let v = self.game.borrow().maze_version;
        if self.seen_maze.replace(v) != v {
            self.maze.notify();
        }
    }

    fn show(&self, kind: Overlay) {
        // The clock that drives the loops stops behind a card, so the card silences them; the
        // first frame after it closes starts the right one again.
        if kind != Overlay::None {
            self.voice.replace(None);
        }
        self.overlay.set(kind);
        self.repaint.notify();
        self.sync_maze();
    }

    /// The pause menu, when there is a round to pause.
    fn pause(&self) {
        if self.overlay.get_untracked() == Overlay::None && self.game.borrow().live() {
            self.show(Overlay::Pause);
        }
    }

    /// Escape on a keyboard: pause the round in play, or close the pause menu to resume it.
    fn escape(&self) {
        match self.overlay.get_untracked() {
            Overlay::None => {
                self.pause();
                self.cue(&cues::SELECT);
            }
            Overlay::Pause => self.show(Overlay::None),
            _ => {}
        }
    }

    fn pick_mode(&self, from: Overlay) {
        self.return_to.set(from);
        self.show(Overlay::ModePicker);
    }

    fn new_game(&self, mode: Mode) {
        {
            let mut g = self.game.borrow_mut();
            g.start_new_game(mode);
            g.best = best_for(mode);
        }
        gamekit::clear(SAVE_KEY);
        self.voice.replace(None);
        self.fresh.set(false);
        self.show(Overlay::None);
    }

    fn steer(&self, dir: Dir) {
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        self.game.borrow_mut().steer(dir);
        self.pad_repaint.notify();
    }

    /// Whether the joypad belongs under the maze right now. Tracked, so the pad comes and
    /// goes with the window: a phone always has it until a keyboard or pointer shows up; a
    /// desktop never does; the web has it in a window the width of a phone or a tablet, and
    /// loses it the moment a key is pressed or a pointer hovers.
    fn pad_visible(&self) -> bool {
        match pad_override().as_deref() {
            Some("1") => return true,
            Some("0") => return false,
            _ => {}
        }
        if self.pointer_seen.get() || self.keyboard_seen.get() {
            return false;
        }
        if cfg!(any(
            target_os = "ios",
            target_os = "android",
            target_env = "ohos"
        )) {
            return true;
        }
        if cfg!(not(target_arch = "wasm32")) {
            return false;
        }
        day_core::size_class().is_some_and(|c| c.width <= WidthClass::Medium)
    }

    /// The pad's thumb moved to `at`: steer by where it is relative to the pad's center.
    fn pad_touch(&self, at: Option<Point>) {
        let center = Point::new(PAD_W / 2.0, PAD_H / 2.0);
        match at {
            Some(p) => {
                let (dx, dy) = (p.x - center.x, p.y - center.y);
                let len = (dx * dx + dy * dy).sqrt();
                let k = if len > PAD_REACH {
                    PAD_REACH / len
                } else {
                    1.0
                };
                self.thumb
                    .set(Some(Point::new(center.x + dx * k, center.y + dy * k)));
                if let Some(dir) = pad_direction(dx, dy) {
                    self.steer(dir);
                }
            }
            None => self.thumb.set(None),
        }
        self.pad_repaint.notify();
    }

    /// Keep the siren, the fright warble or the eyes' retreat sounding while the round plays:
    /// start the one the round wants when it changes, and stop it the moment play does.
    fn drive_loops(&self) {
        let want: Option<&'static Sfx> = {
            let g = self.game.borrow();
            if g.playing() {
                let ms = g.mode == Mode::MsPacPac;
                if g.any_eyes() {
                    Some(&EYES_LOOP)
                } else if g.frightened() {
                    Some(if ms { &MS_FRIGHT_LOOP } else { &FRIGHT_LOOP })
                } else {
                    let i = (g.siren_level() as usize - 1).min(4);
                    Some(if ms { &MS_SIRENS[i] } else { &SIRENS[i] })
                }
            } else {
                None
            }
        };
        let mut voice = self.voice.borrow_mut();
        let same = match (want, voice.as_ref()) {
            (Some(a), Some((b, _))) => a.as_str() == b.as_str(),
            (None, None) => true,
            _ => false,
        };
        if !same {
            // The new loop starts before the old handle is dropped, so one hands over to the
            // next without a gap.
            let sounds = self.sounds.get_untracked();
            *voice = want.map(|clip| (clip, chrome::sound_loop(sounds, clip, LOOP_VOLUME)));
        }
    }

    fn react(&self, h: Happening, ms: bool) {
        match h {
            Happening::Intro => self.cue(if ms { &MS_INTRO } else { &INTRO }),
            Happening::Go => {}
            Happening::Chomp => {
                self.haptic(Haptic::Selection);
                if ms {
                    self.sound(&MS_CHOMP, 0.7);
                } else {
                    let b = self.chomp_b.replace(!self.chomp_b.get());
                    self.sound(if b { &CHOMP_B } else { &CHOMP_A }, 0.7);
                }
            }
            Happening::Energizer => self.haptic(Haptic::Medium),
            Happening::GhostEaten(_) => self.cue(&EAT_GHOST),
            Happening::FruitEaten(_) => self.cue(&EAT_FRUIT),
            Happening::FruitAppeared => {}
            Happening::ExtraLife => self.cue(&EXTRA_LIFE),
            Happening::Death => self.cue(if ms { &MS_DEATH } else { &DEATH }),
            Happening::LevelClear => {
                chrome::haptic_pattern(self.vibrations.get_untracked(), chrome::BIG_CELEBRATE)
            }
            Happening::GameOver => {
                self.cue(&cues::OVER_ARCADE);
                self.show(Overlay::GameOver);
            }
        }
    }
}

/// The direction a thumb `(dx, dy)` from the pad's center asks for, past the dead zone.
fn pad_direction(dx: f64, dy: f64) -> Option<Dir> {
    if dx.abs().max(dy.abs()) < PAD_DEAD {
        return None;
    }
    Some(if dx.abs() > dy.abs() {
        if dx < 0.0 { Dir::Left } else { Dir::Right }
    } else if dy < 0.0 {
        Dir::Up
    } else {
        Dir::Down
    })
}

/// `DAY_PACPAC_PAD`, the desktop walkthrough's way of forcing the joypad on (`1`) or off
/// (`0`). A browser has no process environment, and needs no override: there the window's
/// width decides.
fn pad_override() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("DAY_PACPAC_PAD").ok()
    }
}

/// The best score saved for `mode`.
fn best_for(mode: Mode) -> i64 {
    let key = match mode {
        Mode::PacPac => BEST_PACPAC_KEY,
        Mode::MsPacPac => BEST_MSPACPAC_KEY,
    };
    gamekit::restore::<i64>(key).unwrap_or(0)
}

fn save_best(mode: Mode, best: i64) {
    let key = match mode {
        Mode::PacPac => BEST_PACPAC_KEY,
        Mode::MsPacPac => BEST_MSPACPAC_KEY,
    };
    gamekit::save(key, &best);
}

/// Forget both games' high scores (a save carries neither). An open game's live best follows
/// through [`gamekit::on_reset_records`]. The settings card's button and the shell's Reset all
/// records both come here.
pub fn reset_records() {
    gamekit::clear(BEST_PACPAC_KEY);
    gamekit::clear(BEST_MSPACPAC_KEY);
    gamekit::reset_live_records(SAVE_KEY);
}

/// The Pac-Pac screen.
pub fn pacpac_page() -> AnyPiece {
    let settings = gamekit::restore::<chrome::GameSettings>(SETTINGS_KEY).unwrap_or_default();
    let saved = gamekit::restore::<SaveState>(SAVE_KEY);
    let fresh = saved.is_none();
    let mut game = match saved {
        Some(s) => Game::from_save(s, gamekit::seed()),
        None => Game::new(Mode::PacPac, gamekit::seed()),
    };
    game.best = best_for(game.mode);
    let game_over = game.game_over;
    let ui = Rc::new(Ui {
        game: Rc::new(RefCell::new(game)),
        repaint: Trigger::new(),
        maze: Trigger::new(),
        seen_maze: Cell::new(u32::MAX),
        overlay: Signal::new(Overlay::None),
        return_to: Cell::new(Overlay::None),
        sounds: Signal::new(settings.sounds),
        vibrations: Signal::new(settings.vibrations),
        swipe_from: Cell::new(None),
        voice: RefCell::new(None),
        chomp_b: Cell::new(false),
        fresh: Cell::new(fresh),
        pointer_seen: Signal::new(false),
        keyboard_seen: Signal::new(false),
        thumb: Cell::new(None),
        pad_repaint: Trigger::new(),
    });
    gamekit::autosave(SAVE_KEY, {
        let game = ui.game.clone();
        move || game.borrow().save_state()
    });
    gamekit::sounds(SOUNDS);
    gamekit::on_reset_records(SAVE_KEY, {
        let u = ui.clone();
        move || {
            u.game.borrow_mut().best = 0;
            u.repaint.notify();
        }
    });
    Effect::new({
        let ui = ui.clone();
        move || {
            gamekit::save(
                SETTINGS_KEY,
                &chrome::GameSettings {
                    sounds: ui.sounds.get(),
                    vibrations: ui.vibrations.get(),
                    instructions_shown: true,
                },
            );
        }
    });
    // The first open explains the game and then asks which one to play; a restored round
    // waits behind the pause menu; a finished one shows its card.
    if !settings.instructions_shown {
        ui.show(Overlay::Instructions);
    } else if game_over {
        ui.show(Overlay::GameOver);
    } else if fresh {
        ui.pick_mode(Overlay::None);
    } else {
        ui.show(Overlay::Pause);
    }
    gamekit::on_background(SAVE_KEY, {
        let ui = ui.clone();
        move || ui.pause()
    });

    // Two layers over the frame's play box: the maze (walls and dots) under the play layer.
    let maze_layer = {
        let mu = ui.clone();
        canvas(move |d, sz| {
            mu.maze.track();
            let g = mu.game.borrow();
            d.transformed(placement(sz), |d| draw_maze_layer(&g, d));
        })
        .grow()
    };
    let play_layer = {
        let (du, dr, tu, ku, hu) = (ui.clone(), ui.clone(), ui.clone(), ui.clone(), ui.clone());
        canvas(move |d, sz| {
            du.repaint.track();
            let g = du.game.borrow();
            d.transformed(placement(sz), |d| draw_play_layer(&g, d));
        })
        // A pointer hovering means a mouse or trackpad is at hand: the joypad steps aside.
        .on_hover(move |at| {
            if at.is_some() && !hu.pointer_seen.get_untracked() {
                hu.pointer_seen.set(true);
            }
        })
        // A swipe steers; a finger that keeps moving steers again every SWIPE points.
        .on_drag(move |dg| match dg.phase {
            DragPhase::Began => dr.swipe_from.set(Some(dg.location)),
            DragPhase::Ended => dr.swipe_from.set(None),
            _ => {
                let Some(from) = dr.swipe_from.get() else {
                    dr.swipe_from.set(Some(dg.location));
                    return;
                };
                let (dx, dy) = (dg.location.x - from.x, dg.location.y - from.y);
                if dx.abs().max(dy.abs()) < SWIPE {
                    return;
                }
                let dir = if dx.abs() > dy.abs() {
                    if dx < 0.0 { Dir::Left } else { Dir::Right }
                } else if dy < 0.0 {
                    Dir::Up
                } else {
                    Dir::Down
                };
                dr.swipe_from.set(Some(dg.location));
                dr.steer(dir);
            }
        })
        .on_tap(move || {
            // A tap on a finished game opens its card again.
            if tu.overlay.get_untracked() == Overlay::None && !tu.game.borrow().live() {
                tu.show(Overlay::GameOver);
            }
        })
        // The arrows steer, as do WASD and vi's HJKL; Escape and P pause.
        .on_key(move |k| {
            if !ku.keyboard_seen.get_untracked() {
                ku.keyboard_seen.set(true);
            }
            let dir = match k.key.as_str() {
                "Escape" | "p" | "P" => return ku.escape(),
                "ArrowLeft" | "a" | "A" | "h" | "H" => Dir::Left,
                "ArrowRight" | "d" | "D" | "l" | "L" => Dir::Right,
                "ArrowUp" | "w" | "W" | "k" | "K" => Dir::Up,
                "ArrowDown" | "s" | "S" | "j" | "J" => Dir::Down,
                _ => return,
            };
            ku.steer(dir);
        })
        .id(id!(pm_canvas))
        .grow()
    };

    // Mounted only while the round runs, so the display link goes idle behind a card.
    let clock = {
        let (cu, bu) = (ui.clone(), ui.clone());
        when(
            move || cu.overlay.get() == Overlay::None,
            move || pacpac_clock(bu.clone()),
        )
    };

    let pu = ui.clone();
    let header = chrome::game_header(tr("nav_pacpac"), id!(pm_pause), move || {
        pu.pause();
        pu.cue(&cues::SELECT);
    });
    let play = zstack((maze_layer, play_layer)).grow().any();
    let pad = {
        let (vu, bu) = (ui.clone(), ui.clone());
        when(move || vu.pad_visible(), move || joypad(bu.clone()))
    };
    zstack((
        chrome::game_frame(header, None, play, Some(pad.any())),
        overlays(ui),
        clock,
    ))
    .background(SURFACE)
    .any()
}

/// The joypad under the maze: a ring with four arrows, the one Pac-Pac is heading for (or
/// has been asked for) lit, and the thumb shown wherever the finger holds it. Dragging on it
/// steers like a stick; tapping an arrow steers too.
fn joypad(ui: Rc<Ui>) -> impl Piece {
    let (du, dr, tu) = (ui.clone(), ui.clone(), ui);
    canvas(move |d, sz| {
        du.pad_repaint.track();
        du.repaint.track();
        let (want, heading, playing) = {
            let g = du.game.borrow();
            (g.want, g.pac.dir, g.playing())
        };
        let c = Point::new(sz.width / 2.0, sz.height / 2.0);
        let ring = Rect::new(
            c.x - PAD_RING,
            c.y - PAD_RING,
            PAD_RING * 2.0,
            PAD_RING * 2.0,
        );
        d.fill(Shape::Ellipse(ring), Color::rgba(1.0, 1.0, 1.0, 0.07));
        d.stroke(Shape::Ellipse(ring), Color::rgba(1.0, 1.0, 1.0, 0.18), 1.5);
        let lit = want.or(if playing { Some(heading) } else { None });
        for dir in [Dir::Up, Dir::Left, Dir::Down, Dir::Right] {
            let (ux, uy) = dir.unit();
            let tip = Point::new(c.x + ux * 48.0, c.y + uy * 48.0);
            let base = Point::new(c.x + ux * 30.0, c.y + uy * 30.0);
            let (px, py) = (-uy * 11.0, ux * 11.0);
            let color = if lit == Some(dir) {
                art::YELLOW
            } else {
                Color::rgba(1.0, 1.0, 1.0, 0.35)
            };
            d.fill(
                Shape::Polygon(vec![
                    tip,
                    Point::new(base.x + px, base.y + py),
                    Point::new(base.x - px, base.y - py),
                ]),
                color,
            );
        }
        let thumb = du.thumb.get().unwrap_or(c);
        let r = if du.thumb.get().is_some() { 20.0 } else { 9.0 };
        d.fill(
            Shape::Ellipse(Rect::new(thumb.x - r, thumb.y - r, r * 2.0, r * 2.0)),
            Color::rgba(
                1.0,
                1.0,
                0.4,
                if du.thumb.get().is_some() { 0.55 } else { 0.35 },
            ),
        );
        d.stroke_styled(
            Shape::Ellipse(Rect::new(thumb.x - r, thumb.y - r, r * 2.0, r * 2.0)),
            art::YELLOW,
            StrokeStyle {
                width: 1.5,
                cap: LineCap::Round,
                join: LineJoin::Round,
                ..Default::default()
            },
        );
    })
    .on_drag(move |dg| match dg.phase {
        DragPhase::Ended => dr.pad_touch(None),
        _ => dr.pad_touch(Some(dg.location)),
    })
    .on_tap_at(move |p| {
        tu.pad_touch(Some(p));
        tu.pad_touch(None);
    })
    .a11y(|a| a.label(tr("pm_pad")))
    .id(id!(pm_pad))
    .frame(PAD_W, PAD_H)
}

/// The round's frame consumer: the step, then the sounds and haptics it earned, the loops, and
/// the layers that changed.
fn pacpac_clock(ui: Rc<Ui>) -> impl Piece {
    let demand = ui.clone();
    gamekit::animation::clock(
        move || demand.overlay.get() == Overlay::None && demand.game.borrow().live(),
        move |dt| {
            let dt = dt.as_secs_f64().min(0.1);
            let (happenings, ms, best, mode) = {
                let mut g = ui.game.borrow_mut();
                g.update(dt);
                (
                    std::mem::take(&mut g.happenings),
                    g.mode == Mode::MsPacPac,
                    g.best,
                    g.mode,
                )
            };
            for h in happenings {
                ui.react(h, ms);
                if matches!(
                    h,
                    Happening::Death | Happening::LevelClear | Happening::GameOver
                ) {
                    save_best(mode, best);
                }
            }
            ui.drive_loops();
            ui.repaint.notify();
            ui.sync_maze();
        },
    )
}

fn overlays(ui: Rc<Ui>) -> impl Piece {
    let scrim = {
        let u = ui.clone();
        when(move || u.overlay.get() != Overlay::None, chrome::scrim)
    };
    let u = ui.clone();
    let card = switch(
        move || u.overlay.get(),
        move |kind| match kind {
            Overlay::None => None,
            Overlay::Pause => Some(pause_menu(ui.clone())),
            Overlay::ModePicker => Some(mode_picker(ui.clone())),
            Overlay::GameOver => Some(game_over_card(ui.clone())),
            Overlay::Settings => Some(settings_card(ui.clone())),
            Overlay::Instructions => Some(instructions_card(ui.clone())),
        },
    );
    zstack((scrim, card))
}

fn pause_menu(ui: Rc<Ui>) -> AnyPiece {
    let (u1, u2, u3, u4) = (ui.clone(), ui.clone(), ui.clone(), ui.clone());
    chrome::card(
        column((
            chrome::card_title(tr("gk_paused"), Color::WHITE),
            chrome::menu_button(tr("gk_resume"), chrome::GREEN, id!(pm_resume), move || {
                u1.show(Overlay::None)
            }),
            chrome::menu_button(
                tr("gk_new_game"),
                chrome::BLUE,
                id!(pm_new_game),
                move || u2.pick_mode(Overlay::Pause),
            ),
            chrome::menu_button(
                tr("gk_settings"),
                chrome::SLATE,
                id!(pm_settings),
                move || u3.show(Overlay::Settings),
            ),
            chrome::menu_button(
                tr("gk_instructions"),
                chrome::INDIGO,
                id!(pm_instructions),
                move || u4.show(Overlay::Instructions),
            ),
            chrome::close_game_button(chrome::RED, id!(pm_quit)),
        ))
        .spacing(14.0)
        .align(HAlign::Center),
    )
    .id(id!(pm_pause_menu))
    .any()
}

/// New Game: Pac-Pac or Ms. Pac-Pac.
fn mode_picker(ui: Rc<Ui>) -> AnyPiece {
    let current = ui.game.borrow().mode;
    let choices = [
        (
            Mode::PacPac,
            tr("pm_mode_pacpac"),
            tr("pm_mode_pacpac_detail"),
            art::YELLOW,
            id!(pm_mode_pacpac),
        ),
        (
            Mode::MsPacPac,
            tr("pm_mode_mspacpac"),
            tr("pm_mode_mspacpac_detail"),
            art::PINKY,
            id!(pm_mode_mspacpac),
        ),
    ];
    let mut cards = Vec::new();
    for (mode, title, detail, tint, id) in choices {
        let u = ui.clone();
        let check = when(
            move || mode == current,
            move || {
                canvas(move |dr, sz| {
                    chrome::draw_check_glyph(
                        dr,
                        Point::new(sz.width / 2.0, sz.height / 2.0),
                        18.0,
                        tint,
                    );
                })
                .frame(24.0, 24.0)
            },
        );
        let spoken = title.clone();
        cards.push(
            row((
                column((
                    label(title).font(Font::Title3).bold().color(Color::WHITE),
                    label(detail).font(Font::Caption).color(chrome::TEXT_DIM),
                ))
                .spacing(Space::XS)
                .align(HAlign::Leading)
                .grow_w(),
                check,
            ))
            .align(VAlign::Center)
            .padding(14.0)
            .background(tint.with_alpha(0.18))
            .corner_radius(14.0)
            .on_tap(move || {
                u.new_game(mode);
                u.cue(&cues::START);
            })
            .a11y(move |a| a.label(spoken.clone()).role(Role::Button))
            .id(id)
            .width(300.0)
            .any(),
        );
    }
    let u = ui;
    chrome::card(
        column((
            label(tr("pm_mode_title"))
                .font(Font::Title2)
                .bold()
                .color(Color::WHITE),
            column(cards).spacing(Space::M),
            button(tr("gk_cancel"))
                .action(move || {
                    let back = u.return_to.replace(Overlay::None);
                    u.show(back);
                })
                .id(id!(pm_cancel)),
        ))
        .spacing(Space::L)
        .align(HAlign::Center),
    )
    .id(id!(pm_mode_picker))
    .any()
}

fn game_over_card(ui: Rc<Ui>) -> AnyPiece {
    let (score, level, best, mode) = {
        let g = ui.game.borrow();
        (g.score, g.level, g.best, g.mode)
    };
    let record = when(
        move || score >= best && score > 0,
        || {
            label(tr("gk_new_high_score"))
                .font(Font::Title3)
                .bold()
                .color(chrome::GOLD)
        },
    );
    let mode_name = match mode {
        Mode::PacPac => tr("pm_mode_pacpac"),
        Mode::MsPacPac => tr("pm_mode_mspacpac"),
    };
    let u = ui;
    chrome::card(
        column((
            chrome::card_title(tr("gk_game_over"), Color::WHITE),
            label(mode_name)
                .font(Font::Headline)
                .color(chrome::TEXT_DIM),
            chrome::stat(
                tr("gk_score"),
                score.to_string(),
                Font::LargeTitle,
                chrome::GOLD,
                id!(pm_final_score),
            ),
            row((
                chrome::stat(
                    tr("pm_level_caption"),
                    level.to_string(),
                    Font::Title3,
                    Color::WHITE,
                    id!(pm_final_level),
                ),
                chrome::stat(
                    tr("gk_best"),
                    best.to_string(),
                    Font::Title3,
                    Color::WHITE,
                    id!(pm_best),
                ),
            ))
            .spacing(Space::XL),
            record,
            chrome::menu_button(
                tr("gk_play_again"),
                chrome::BLUE,
                id!(pm_play_again),
                move || u.pick_mode(Overlay::GameOver),
            ),
            chrome::close_game_button(chrome::RED, id!(pm_quit)),
        ))
        .spacing(14.0)
        .align(HAlign::Center),
    )
    .id(id!(pm_game_over))
    .any()
}

fn settings_card(ui: Rc<Ui>) -> AnyPiece {
    let reset = button(tr("gk_reset_high_score"))
        .tint(chrome::RED)
        .action(|| {
            day_core::task(async {
                let sure = Alert::new(tr("gk_reset_high_score_title"))
                    .message(tr("gk_reset_high_score_message"))
                    .destructive(tr("gk_reset_confirm"), true)
                    .cancel(tr("gk_cancel"))
                    .present()
                    .await;
                if sure == Some(true) {
                    reset_records();
                }
            });
        })
        .id(id!(pm_reset_high_score));
    let done = ui.clone();
    chrome::card(
        column((
            label(tr("gk_settings"))
                .font(Font::Title2)
                .bold()
                .color(Color::WHITE),
            chrome::section_heading(tr("nav_pacpac")),
            chrome::setting_row(tr("gk_sounds"), toggle(ui.sounds).id(id!(pm_sounds)).any()),
            chrome::setting_row(
                tr("gk_vibrations"),
                toggle(ui.vibrations).id(id!(pm_vibrations)).any(),
            ),
            chrome::section_heading(tr("gk_data")),
            reset,
            button(tr("gk_done"))
                .prominent()
                .action(move || done.show(Overlay::Pause))
                .id(id!(pm_done)),
        ))
        .spacing(Space::M)
        .align(HAlign::Center),
    )
    .id(id!(pm_settings_card))
    .any()
}

fn instructions_card(ui: Rc<Ui>) -> AnyPiece {
    // Done goes on to the mode picker on a first open, back to the pause menu when the game is
    // paused, and to the board otherwise.
    let paused_before = ui.return_to.get() == Overlay::Pause
        || ui.overlay.get_untracked() == Overlay::Instructions && !ui.fresh.get();
    chrome::instructions_card(
        tr("nav_pacpac"),
        vec![
            Help::Para(tr("pm_help_intro")),
            Help::Heading(tr("pm_help_play")),
            Help::Para(tr("pm_help_play_1")),
            Help::Para(tr("pm_help_play_2")),
            Help::Para(tr("pm_help_play_3")),
            Help::Heading(tr("pm_help_ghosts")),
            Help::Para(tr("pm_help_ghosts_1")),
            Help::Para(tr("pm_help_ghosts_2")),
            Help::Heading(tr("pm_help_games")),
            Help::Para(tr("pm_help_games_1")),
            Help::Para(tr("pm_help_games_2")),
            Help::Heading(tr("pm_help_scoring")),
            Help::Para(tr("pm_help_scoring_1")),
        ],
        id!(pm_help_done),
        move || {
            if ui.fresh.get() {
                ui.pick_mode(Overlay::None);
            } else if paused_before || ui.game.borrow().live() {
                ui.show(Overlay::Pause);
            } else {
                ui.show(Overlay::GameOver);
            }
        },
    )
    .id(id!(pm_instructions_card))
    .any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_snaps_to_half_points_and_centers() {
        let m = placement(Size::new(448.0, 600.0));
        let p = m.apply(Point::new(0.0, 0.0));
        assert_eq!(p, Point::new(0.0, 12.0));
        let q = m.apply(Point::new(SCREEN_W, SCREEN_H));
        assert_eq!(q, Point::new(448.0, 588.0));
        let m = placement(Size::new(300.0, 1000.0));
        let q = m.apply(Point::new(SCREEN_W, 0.0));
        assert!(q.x <= 300.0 && q.x > 200.0, "fits the width: {}", q.x);
        let m = placement(Size::new(200.0, 200.0));
        let q = m.apply(Point::new(SCREEN_W, SCREEN_H));
        assert!(q.x <= 200.0 && q.y <= 200.0, "fits a small tile: {q:?}");
    }

    #[test]
    fn both_layers_record_the_screen() {
        let g = Game::new(Mode::PacPac, 1);
        let mut d = Draw::new();
        draw_maze_layer(&g, &mut d);
        assert!(d.ops().len() > 20, "walls and dots: {} ops", d.ops().len());
        let mut d = Draw::new();
        draw_play_layer(&g, &mut d);
        let fills = d
            .ops()
            .iter()
            .filter(|op| matches!(op, day_spec::DrawOp::Fill(..)))
            .count();
        assert!(fills > 10, "HUD text and READY!: {fills} fills");
    }

    #[test]
    fn preview_draws() {
        let _ = pacpac_preview();
    }

    #[test]
    fn pad_reads_the_thumb_past_a_dead_zone() {
        assert_eq!(pad_direction(3.0, -4.0), None);
        assert_eq!(pad_direction(30.0, 10.0), Some(Dir::Right));
        assert_eq!(pad_direction(-30.0, 10.0), Some(Dir::Left));
        assert_eq!(pad_direction(5.0, -30.0), Some(Dir::Up));
        assert_eq!(pad_direction(5.0, 30.0), Some(Dir::Down));
    }
}
