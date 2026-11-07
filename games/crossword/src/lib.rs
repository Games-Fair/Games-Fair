// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Crossword: a library of puzzles, and a solver for them in the manner of the dedicated
//! crossword apps. The grid is one canvas (a 21×21 Sunday is 441 squares, too many for a piece
//! each), tapped to place the cursor and tapped again to turn it; the current clue rides in a
//! bar over the grid, and the full Across and Down lists follow the cursor where the window is
//! wide enough to show them. Phones and tablets type on a keyboard drawn under the grid;
//! desktops and the web type on their own. Check, Reveal and Clear each act on a square, a word
//! or the whole puzzle, and every puzzle keeps its progress and its clock while you move
//! between them.
//!
//! The puzzles are read from Across Lite's `.puz` or from ipuz (`model.rs`), the two formats
//! the crossword world publishes in.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use day_localization_ui::{LocalizedText, tr};
use day_pieces::prelude::*;
use day_spec::{Cap, HeightClass, KeyEvent, Support, WidthClass};
use gamekit::chrome::{self, Cue, Feedback, Help, Sfx, cues};
use serde::{Deserialize, Serialize};

mod model;
use model::{Dir, Mark, Progress, Puzzle, Scope, Solve, fmt_time};

/// The prefs keys this game persists under (gamekit).
const SAVE_KEY: &str = "crossword.v1";
const SETTINGS_KEY: &str = "crossword.settings";
const RECORDS_KEY: &str = "crossword.records";

// The page is the games' night blue; the grid itself is paper and ink, as crosswords are.
const BG: Color = Color::rgb(0.06, 0.07, 0.12);
pub const SURFACE: Color = BG;
const PAPER: Color = Color::rgb(0.98, 0.97, 0.94);
const BLOCK: Color = Color::rgb(0.10, 0.11, 0.17);
const RULE: Color = Color::rgba(0.10, 0.11, 0.17, 0.35);
const WORD: Color = Color::rgb(0.76, 0.86, 1.0);
const CURSOR: Color = Color::rgb(1.0, 0.83, 0.33);
const INK: Color = Color::rgb(0.08, 0.10, 0.18);
const PENCIL: Color = Color::rgb(0.48, 0.50, 0.58);
const CHECKED: Color = Color::rgb(0.14, 0.38, 0.78);
const WRONG: Color = Color::rgb(0.86, 0.22, 0.22);
const NUMBER: Color = Color::rgba(0.08, 0.10, 0.18, 0.70);
const CIRCLE: Color = Color::rgba(0.08, 0.10, 0.18, 0.45);
const CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);
/// The current clue's row in the lists, and the clue bar: the grid's word color, dimmed for
/// the dark page.
const CLUE_ON: Color = Color::rgba(0.62, 0.76, 1.0, 0.26);
/// The crossing clue's row.
const CLUE_CROSS: Color = Color::rgba(1.0, 1.0, 1.0, 0.07);
const KEY: Color = Color::rgba(1.0, 1.0, 1.0, 0.13);
const KEY_ON: Color = Color::rgb(0.30, 0.55, 0.95);

/// The widest a grid's square gets, so a mini does not fill a desktop.
const SQUARE_MAX: f64 = 56.0;
/// The clue panels' width beside the grid on a wide window.
const LIST_W: f64 = 290.0;
/// The keyboard's key height and gaps.
const KEY_H: f64 = 46.0;
const KEY_GAP: f64 = 6.0;

/// Every clip this game plays besides the shared ones (gamekit preloads both).
pub const SOUNDS: &[Sfx] = &[];

/// One puzzle the library offers.
struct Entry {
    /// The key its progress saves under. Never reuse one for a different puzzle.
    id: &'static str,
    /// Where it comes from, for the library's byline.
    source: &'static str,
    data: Data,
}

enum Data {
    Ipuz(&'static str),
    #[allow(dead_code)] // The bundled puzzles are ipuz so far; `.puz` is what downloads bring.
    Puz(&'static [u8]),
}

impl Entry {
    fn load(&self) -> Option<Puzzle> {
        match self.data {
            Data::Ipuz(text) => Puzzle::from_ipuz(text).ok(),
            Data::Puz(bytes) => Puzzle::from_puz(bytes).ok(),
        }
    }
}

/// The puzzles shipped inside the app.
const BUNDLED: &[Entry] = &[Entry {
    id: "games-fair/warm-up",
    source: "Games Fair",
    data: Data::Ipuz(include_str!("../puzzles/warm-up.ipuz")),
}];

/// Every puzzle's progress, and which one is open.
#[derive(Clone, Default, Serialize, Deserialize)]
struct Saves {
    current: Option<String>,
    progress: BTreeMap<String, Progress>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Settings {
    sounds: bool,
    vibrations: bool,
    instructions_shown: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let shell = chrome::GameSettings::default();
        Settings {
            sounds: shell.sounds,
            vibrations: shell.vibrations,
            instructions_shown: false,
        }
    }
}

/// Puzzles finished, and how many of them without a check or a reveal.
#[derive(Clone, Default, Serialize, Deserialize)]
struct Records {
    solved: u32,
    clean: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Overlay {
    None,
    Pause,
    Library,
    Check,
    Reveal,
    Clear,
    Clues,
    Solved,
    Instructions,
}

/// Everything the page's closures share.
struct Ui {
    /// The open puzzle's solve, `None` before one is chosen.
    solve: RefCell<Option<Solve>>,
    /// The open puzzle's id: the page rebuilds its clue lists when it changes.
    open: Signal<Option<&'static str>>,
    saves: RefCell<Saves>,
    records: RefCell<Records>,
    /// Repaints the grid, the clue bar and the lists on every edit.
    board: Trigger,
    /// Ticks the clock readout once a second.
    clock: Trigger,
    overlay: Signal<Overlay>,
    return_to: Cell<Overlay>,
    sounds: Signal<bool>,
    vibrations: Signal<bool>,
    instructions_shown: Cell<bool>,
    board_focus: Signal<bool>,
    /// Programmatic scrolls for the Across and Down lists, to keep the current clue in view.
    across_scroll: Signal<Option<day_core::ScrollTarget>>,
    down_scroll: Signal<Option<day_core::ScrollTarget>>,
    /// The grid's and the keyboard's last sizes, for turning a tap into a square or a key.
    grid_size: Cell<Size>,
    keyboard_width: Cell<f64>,
}

impl Ui {
    fn feedback(&self) -> Feedback {
        Feedback {
            sounds: self.sounds.get_untracked(),
            vibrations: self.vibrations.get_untracked(),
        }
    }

    fn cue(&self, c: &Cue) {
        chrome::cue(self.feedback(), c);
    }

    /// Run an edit against the open solve, then react: a finished grid records itself and
    /// opens the solved card, the lists follow the cursor, and everything repaints.
    fn edit(&self, f: impl FnOnce(&mut Solve)) {
        let (was_solved, clue_before) = {
            let s = self.solve.borrow();
            let Some(s) = s.as_ref() else { return };
            (s.progress.solved, s.current_clue())
        };
        let (solved, assisted, clue) = {
            let mut s = self.solve.borrow_mut();
            let Some(s) = s.as_mut() else { return };
            f(s);
            (s.progress.solved, s.progress.assisted, s.current_clue())
        };
        if solved && !was_solved {
            {
                let mut r = self.records.borrow_mut();
                r.solved += 1;
                if !assisted {
                    r.clean += 1;
                }
            }
            gamekit::save(RECORDS_KEY, &*self.records.borrow());
            self.store();
            self.show(Overlay::Solved);
            self.cue(&cues::SUCCESS);
        }
        if clue != clue_before {
            self.follow_clue();
        }
        self.board.notify();
    }

    /// Scroll both lists to the clue under the cursor and its crossing.
    fn follow_clue(&self) {
        let s = self.solve.borrow();
        let Some(s) = s.as_ref() else { return };
        for k in [s.current_clue(), s.crossing_clue()].into_iter().flatten() {
            let target = Some(day_core::ScrollTarget::Id(
                id!(cw_clue, k as u64).as_str().to_string(),
            ));
            match s.puzzle.clues[k].dir {
                Dir::Across => self.across_scroll.set(target),
                Dir::Down => self.down_scroll.set(target),
            }
        }
    }

    /// Copy the open solve's progress into the saves and write them.
    fn store(&self) {
        let id = self.open.get_untracked();
        if let (Some(id), Some(s)) = (id, self.solve.borrow().as_ref()) {
            let mut saves = self.saves.borrow_mut();
            saves.current = Some(id.to_string());
            saves.progress.insert(id.to_string(), s.progress.clone());
        }
        gamekit::save(SAVE_KEY, &*self.saves.borrow());
    }

    /// Open the library's puzzle `id`, where its progress left off.
    fn open_puzzle(&self, id: &'static str) {
        self.store();
        let Some(puzzle) = BUNDLED.iter().find(|e| e.id == id).and_then(Entry::load) else {
            return;
        };
        let saved = self.saves.borrow().progress.get(id).cloned();
        *self.solve.borrow_mut() = Some(Solve::new(puzzle, saved));
        self.saves.borrow_mut().current = Some(id.to_string());
        self.open.set(Some(id));
        self.return_to.set(Overlay::None);
        self.show(Overlay::None);
        self.follow_clue();
        self.cue(&cues::START);
    }

    /// Present `kind`, or uncover the grid with `Overlay::None`. The clock stops while
    /// anything covers it, and uncovering it hands the keyboard back to the grid.
    fn show(&self, kind: Overlay) {
        if let Some(s) = self.solve.borrow_mut().as_mut() {
            s.paused = kind != Overlay::None;
        }
        self.overlay.set(kind);
        if kind == Overlay::None {
            self.board_focus.set(true);
        }
        self.board.notify();
    }

    fn push(&self, kind: Overlay) {
        self.return_to.set(self.overlay.get_untracked());
        self.show(kind);
    }

    fn pop(&self) {
        let back = self.return_to.replace(Overlay::None);
        // With no puzzle open there is nothing under a card but the library.
        if back == Overlay::None && self.solve.borrow().is_none() {
            self.show(Overlay::Library);
        } else {
            self.show(back);
        }
    }

    fn type_letter(&self, ch: char) {
        self.edit(|s| s.type_letter(ch));
        self.cue(&cues::TICK);
    }

    /// A hardware key, heard by the grid (docs/menus.md).
    fn key(&self, k: &KeyEvent) {
        if k.key == "Escape" {
            match self.overlay.get_untracked() {
                Overlay::None => self.show(Overlay::Pause),
                Overlay::Pause => self.show(Overlay::None),
                _ => {}
            }
            return;
        }
        if self.overlay.get_untracked() != Overlay::None {
            return;
        }
        match k.key.as_str() {
            "ArrowLeft" => self.edit(|s| s.arrow(0, -1)),
            "ArrowRight" => self.edit(|s| s.arrow(0, 1)),
            "ArrowUp" => self.edit(|s| s.arrow(-1, 0)),
            "ArrowDown" => self.edit(|s| s.arrow(1, 0)),
            "Delete" | "Backspace" => self.edit(|s| s.backspace()),
            " " => self.edit(|s| s.toggle_dir()),
            "Tab" => {
                let forward = k.modifiers & KeyEvent::SHIFT == 0;
                self.edit(|s| s.next_clue(forward));
            }
            "Enter" => self.edit(|s| s.finish_rebus()),
            key => {
                let mut chars = key.chars();
                if let (Some(ch), None) = (chars.next(), chars.next())
                    && ch.is_alphanumeric()
                {
                    self.type_letter(ch);
                }
            }
        }
    }

    fn settings(&self) -> Settings {
        Settings {
            sounds: self.sounds.get_untracked(),
            vibrations: self.vibrations.get_untracked(),
            instructions_shown: self.instructions_shown.get(),
        }
    }
}

pub fn reset_records() {
    gamekit::clear(RECORDS_KEY);
    gamekit::reset_live_records(RECORDS_KEY);
}

/// Whether to draw a keyboard under the grid: on the phones and tablets, which have no
/// keyboard of their own in reach. Those are the platforms without a menu bar, except the web,
/// which is mostly read on a computer.
fn touch_keyboard() -> bool {
    !cfg!(target_arch = "wasm32") && day_core::capability(Cap::AppMenu) == Support::Unsupported
}

/// The Crossword screen.
pub fn crossword_page() -> AnyPiece {
    let settings = gamekit::restore::<Settings>(SETTINGS_KEY).unwrap_or_default();
    let saves = gamekit::restore::<Saves>(SAVE_KEY).unwrap_or_default();
    let records = gamekit::restore::<Records>(RECORDS_KEY).unwrap_or_default();
    let ui = Rc::new(Ui {
        solve: RefCell::new(None),
        open: Signal::new(None),
        saves: RefCell::new(saves),
        records: RefCell::new(records),
        board: Trigger::new(),
        clock: Trigger::new(),
        overlay: Signal::new(Overlay::None),
        return_to: Cell::new(Overlay::None),
        sounds: Signal::new(settings.sounds),
        vibrations: Signal::new(settings.vibrations),
        instructions_shown: Cell::new(settings.instructions_shown),
        board_focus: Signal::new(true),
        across_scroll: Signal::new(None),
        down_scroll: Signal::new(None),
        grid_size: Cell::new(Size::new(0.0, 0.0)),
        keyboard_width: Cell::new(0.0),
    });
    gamekit::sounds(SOUNDS);
    gamekit::on_reset_records(RECORDS_KEY, {
        let u = ui.clone();
        move || *u.records.borrow_mut() = Records::default()
    });
    gamekit::autosave(SAVE_KEY, {
        let u = ui.clone();
        move || {
            u.store();
            u.saves.borrow().clone()
        }
    });
    Effect::new({
        let ui = ui.clone();
        move || {
            ui.sounds.track();
            ui.vibrations.track();
            gamekit::save(SETTINGS_KEY, &ui.settings());
        }
    });

    // Reopen the puzzle that was open, else start on the library.
    let current = ui.saves.borrow().current.clone();
    match current.and_then(|id| BUNDLED.iter().find(|e| e.id == id)) {
        Some(e) => ui.open_puzzle(e.id),
        None => ui.show(Overlay::Library),
    }
    if !ui.instructions_shown.get() {
        ui.instructions_shown.set(true);
        gamekit::save(SETTINGS_KEY, &ui.settings());
        ui.push(Overlay::Instructions);
    }

    gamekit::on_background(SAVE_KEY, {
        let ui = ui.clone();
        move || {
            if ui.overlay.get_untracked() == Overlay::None {
                ui.show(Overlay::Pause);
            }
        }
    });

    let demand = ui.clone();
    let ticker = gamekit::animation::clock(
        move || {
            demand.board.track();
            demand.overlay.get() == Overlay::None
                && demand
                    .solve
                    .borrow()
                    .as_ref()
                    .is_some_and(|s| !s.progress.solved)
        },
        {
            let ui = ui.clone();
            move |dt| {
                let mut s = ui.solve.borrow_mut();
                let Some(s) = s.as_mut() else { return };
                let before = s.progress.elapsed as u64;
                s.tick(dt.as_secs_f64());
                if s.progress.elapsed as u64 != before {
                    ui.clock.notify();
                }
            }
        },
    );

    let backdrop = canvas(|d, sz| {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, sz.width, sz.height)), BG);
    })
    .on_key(keys(&ui))
    .grow();

    // The page's shape follows the window: the clue lists beside the grid where there is room
    // for them, the grid over a keyboard on a phone, and the keyboard beside the grid on a
    // phone held sideways. A new puzzle rebuilds the page, since its lists are built per clue.
    let page = {
        let (u, b) = (ui.clone(), ui.clone());
        switch(
            move || (u.open.get(), layout()),
            move |&(open, shape)| match (open, shape) {
                (None, _) => column((header(&b),)).grow().any(),
                (Some(_), Layout::Wide) => wide_page(&b),
                (Some(_), Layout::Phone) => phone_page(&b),
                (Some(_), Layout::Sideways) => sideways_page(&b),
            },
        )
    };

    zstack((
        backdrop,
        zstack((page,)).grow().id(id!(cw_page)),
        overlays(ui),
        ticker,
    ))
    .any()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Layout {
    /// The clue lists beside the grid: a tablet, a desktop window, the web.
    Wide,
    /// The grid over the clue bar and a keyboard: a phone upright.
    Phone,
    /// The grid beside the clue bar and the keyboard: a phone on its side.
    Sideways,
}

fn layout() -> Layout {
    match day_core::size_class() {
        Some(c) if c.height == HeightClass::Compact => Layout::Sideways,
        Some(c) if c.width == WidthClass::Compact => Layout::Phone,
        _ => Layout::Wide,
    }
}

fn keys(ui: &Rc<Ui>) -> impl Fn(&KeyEvent) + 'static {
    let ui = ui.clone();
    move |k| ui.key(k)
}

fn header(ui: &Rc<Ui>) -> AnyPiece {
    let u = ui.clone();
    chrome::game_header(tr("nav_crossword"), id!(cw_pause), move || {
        if u.overlay.get_untracked() == Overlay::None {
            u.show(Overlay::Pause);
            u.cue(&cues::SELECT);
        }
    })
}

/// The puzzle's title and byline, and its clock.
fn byline(ui: &Rc<Ui>) -> AnyPiece {
    let (t, a, c) = (ui.clone(), ui.clone(), ui.clone());
    row((
        column((
            label(move || {
                t.board.track();
                t.solve
                    .borrow()
                    .as_ref()
                    .map(|s| s.puzzle.title.clone())
                    .unwrap_or_default()
            })
            .font(Font::Headline)
            .color(Color::WHITE)
            .max_lines(1)
            .id(id!(cw_title)),
            label(move || {
                a.board.track();
                a.solve
                    .borrow()
                    .as_ref()
                    .map(|s| s.puzzle.author.clone())
                    .unwrap_or_default()
            })
            .font(Font::Subheadline)
            .color(chrome::TEXT_DIM)
            .max_lines(1),
        ))
        .spacing(2.0)
        .align(HAlign::Leading)
        .grow_w(),
        label(move || {
            c.clock.track();
            c.board.track();
            c.solve
                .borrow()
                .as_ref()
                .map(|s| fmt_time(s.progress.elapsed as u64))
                .unwrap_or_default()
        })
        .font(Font::Title3)
        .tabular()
        .color(chrome::TEXT)
        .id(id!(cw_time)),
    ))
    .align(VAlign::Center)
    .padding(Insets::horizontal(16.0))
    .any()
}

/// A wide window: the byline, the clue bar over the grid, the tools under it, and the Across
/// and Down lists beside it (side by side on a wide window, stacked on a medium one).
fn wide_page(ui: &Rc<Ui>) -> AnyPiece {
    let two_lists = day_core::size_class().is_none_or(|c| c.width >= WidthClass::Expanded);
    let lists = if two_lists {
        row((clue_list(ui, Dir::Across), clue_list(ui, Dir::Down)))
            .spacing(Space::M)
            .align(VAlign::Top)
            .grow_h()
            .any()
    } else {
        column((clue_list(ui, Dir::Across), clue_list(ui, Dir::Down)))
            .spacing(Space::M)
            .grow_h()
            .any()
    };
    column((
        header(ui),
        byline(ui),
        row((
            column((
                clue_bar(ui),
                zstack((grid(ui),)).align(Alignment::Top).grow(),
                tools(ui),
            ))
            .spacing(Space::M)
            .grow(),
            lists,
        ))
        .spacing(Space::L)
        .align(VAlign::Top)
        .padding(Insets {
            bottom: 16.0,
            ..Insets::horizontal(16.0)
        })
        .grow(),
    ))
    .spacing(Space::S)
    .grow()
    .any()
}

/// A phone upright: the grid, the clue bar and the tools, and the keyboard at the foot.
fn phone_page(ui: &Rc<Ui>) -> AnyPiece {
    column((
        header(ui),
        byline(ui),
        zstack((grid(ui).padding(Insets::horizontal(chrome::EDGE)),))
            .align(Alignment::Top)
            .grow(),
        column((clue_bar(ui), tools(ui), keyboard(ui)))
            .spacing(Space::S)
            .padding(Insets {
                bottom: 8.0,
                ..Insets::horizontal(chrome::EDGE)
            }),
    ))
    .spacing(Space::S)
    .grow()
    .any()
}

/// A phone on its side: the grid on the leading half, the clue bar, the tools and the keyboard
/// on the trailing half.
fn sideways_page(ui: &Rc<Ui>) -> AnyPiece {
    column((
        header(ui),
        row((
            zstack((grid(ui),)).grow(),
            column((clue_bar(ui), tools(ui), spacer(), keyboard(ui)))
                .spacing(Space::S)
                .grow(),
        ))
        .spacing(Space::M)
        .padding(Insets {
            bottom: chrome::EDGE,
            ..Insets::horizontal(chrome::EDGE)
        })
        .grow(),
    ))
    .grow()
    .any()
}

// ---------------------------------------------------------------------------
// The grid
// ---------------------------------------------------------------------------

/// Where the squares sit in a canvas of `sz`: the square's side and the grid's origin.
fn grid_geometry(p: &Puzzle, sz: Size) -> (f64, f64, f64) {
    let side = (sz.width / p.width as f64)
        .min(sz.height / p.height as f64)
        .min(SQUARE_MAX)
        .floor()
        .max(4.0);
    let ox = ((sz.width - side * p.width as f64) / 2.0).floor();
    let oy = ((sz.height - side * p.height as f64) / 2.0).floor().max(0.0);
    (side, ox, oy)
}

fn draw_grid(d: &mut Draw, s: &Solve, sz: Size) {
    let p = &s.puzzle;
    let (side, ox, oy) = grid_geometry(p, sz);
    let word: Vec<usize> = s
        .current_clue()
        .map(|k| p.clues[k].cells.clone())
        .unwrap_or_default();
    let frame = Rect::new(ox, oy, side * p.width as f64, side * p.height as f64);
    d.fill(Shape::Rect(frame), PAPER);
    let number_font = chrome::canvas_font(FontWeight::Medium);
    let letter_font = chrome::canvas_font(FontWeight::Semibold);
    for i in 0..p.solution.len() {
        let (r, c) = (i / p.width, i % p.width);
        let cell = Rect::new(ox + c as f64 * side, oy + r as f64 * side, side, side);
        if p.is_block(i) {
            d.fill(Shape::Rect(cell), BLOCK);
            continue;
        }
        if i == s.cursor() {
            d.fill(Shape::Rect(cell), CURSOR);
        } else if word.contains(&i) {
            d.fill(Shape::Rect(cell), WORD);
        }
        if p.circled[i] {
            d.stroke(
                Shape::Ellipse(Rect::new(
                    cell.origin.x + 1.0,
                    cell.origin.y + 1.0,
                    side - 2.0,
                    side - 2.0,
                )),
                CIRCLE,
                1.0,
            );
        }
        if p.numbers[i] != 0 {
            d.text(
                &p.numbers[i].to_string(),
                Point::new(cell.origin.x + side * 0.06, cell.origin.y + side * 0.03),
                TextStyle {
                    size: (side * 0.28).max(6.0),
                    color: NUMBER,
                    anchor: TextAnchor::LEADING,
                    font: number_font.clone(),
                },
            );
        }
        let entry = &s.progress.entries[i];
        let mark = s.progress.marks[i];
        if !entry.is_empty() {
            let ink = if mark == Mark::Right {
                CHECKED
            } else if s.progress.pencil[i] {
                PENCIL
            } else {
                INK
            };
            let size = chrome::fit_text(entry, side * 0.58, &letter_font, side * 0.86);
            d.text(
                entry,
                Point::new(cell.origin.x + side / 2.0, cell.origin.y + side * 0.60),
                TextStyle {
                    size,
                    color: ink,
                    anchor: TextAnchor::CENTERED,
                    font: letter_font.clone(),
                },
            );
        }
        match mark {
            Mark::Wrong => d.stroke(
                Shape::Line(
                    Point::new(cell.origin.x + side * 0.15, cell.origin.y + side * 0.85),
                    Point::new(cell.origin.x + side * 0.85, cell.origin.y + side * 0.15),
                ),
                WRONG,
                (side * 0.05).max(1.0),
            ),
            Mark::Revealed => {
                let t = side * 0.26;
                let (x, y) = (cell.origin.x + side, cell.origin.y);
                d.fill(
                    Shape::Polygon(vec![
                        Point::new(x - t, y),
                        Point::new(x, y),
                        Point::new(x, y + t),
                    ]),
                    WRONG,
                );
            }
            _ => {}
        }
    }
    // The rules between squares, then the frame around them.
    for c in 1..p.width {
        let x = ox + c as f64 * side;
        d.stroke(
            Shape::Line(Point::new(x, oy), Point::new(x, oy + frame.size.height)),
            RULE,
            1.0,
        );
    }
    for r in 1..p.height {
        let y = oy + r as f64 * side;
        d.stroke(
            Shape::Line(Point::new(ox, y), Point::new(ox + frame.size.width, y)),
            RULE,
            1.0,
        );
    }
    d.stroke(Shape::Rect(frame), BLOCK, 2.0);
}

/// The grid: one canvas, tapped to place and turn the cursor, and the keyboard's home.
fn grid(ui: &Rc<Ui>) -> AnyPiece {
    let (du, tu, au) = (ui.clone(), ui.clone(), ui.clone());
    let (w, h) = ui
        .solve
        .borrow()
        .as_ref()
        .map(|s| (s.puzzle.width, s.puzzle.height))
        .unwrap_or((15, 15));
    canvas(move |d, sz| {
        du.board.track();
        du.grid_size.set(sz);
        if let Some(s) = du.solve.borrow().as_ref() {
            draw_grid(d, s, sz);
        }
    })
    .on_tap_at(move |pt| {
        let sz = tu.grid_size.get();
        let hit = tu.solve.borrow().as_ref().and_then(|s| {
            let (side, ox, oy) = grid_geometry(&s.puzzle, sz);
            let c = ((pt.x - ox) / side).floor();
            let r = ((pt.y - oy) / side).floor();
            let inside = c >= 0.0
                && r >= 0.0
                && (c as usize) < s.puzzle.width
                && (r as usize) < s.puzzle.height;
            inside.then(|| r as usize * s.puzzle.width + c as usize)
        });
        if let Some(i) = hit {
            tu.board_focus.set(true);
            tu.edit(|s| s.select(i));
        }
    })
    .on_key(keys(ui))
    .focused(ui.board_focus)
    .a11y(move |a| {
        let u = au.clone();
        a.label(tr("cw_grid")).value(move || {
            u.board.track();
            current_clue_text(&u)
        })
    })
    .id(id!(cw_grid))
    .aspect_ratio(w as f64 / h as f64)
    .max_width(SQUARE_MAX * w as f64)
    .any()
}

/// "12A Clue text", for the grid's screen-reader value.
fn current_clue_text(ui: &Ui) -> String {
    let s = ui.solve.borrow();
    let Some(s) = s.as_ref() else {
        return String::new();
    };
    s.current_clue()
        .map(|k| {
            let c = &s.puzzle.clues[k];
            format!("{}{} {}", c.number, dir_letter(c.dir), c.text)
        })
        .unwrap_or_default()
}

fn dir_letter(d: Dir) -> &'static str {
    match d {
        Dir::Across => "A",
        Dir::Down => "D",
    }
}

// ---------------------------------------------------------------------------
// Clues
// ---------------------------------------------------------------------------

/// The current clue over the grid, between buttons for the previous and next clue. Tapping it
/// turns the cursor to the crossing word.
fn clue_bar(ui: &Rc<Ui>) -> AnyPiece {
    let (nu, tu, xu, pu, fu) = (ui.clone(), ui.clone(), ui.clone(), ui.clone(), ui.clone());
    row((
        chevron(false, id!(cw_prev_clue), move || {
            pu.edit(|s| s.next_clue(false))
        }),
        row((
            label(move || {
                nu.board.track();
                let s = nu.solve.borrow();
                s.as_ref()
                    .and_then(|s| s.current_clue().map(|k| &s.puzzle.clues[k]))
                    .map(|c| format!("{}{}", c.number, dir_letter(c.dir)))
                    .unwrap_or_default()
            })
            .font(Font::Headline)
            .color(Color::WHITE)
            .tabular(),
            label(move || {
                tu.board.track();
                let s = tu.solve.borrow();
                match s.as_ref() {
                    // Every square has a letter but the grid is not right: say so, since there is
                    // no clue left to look at.
                    Some(s) if s.full_but_wrong() => tr("cw_not_quite").format(),
                    Some(s) => s
                        .current_clue()
                        .map(|k| s.puzzle.clues[k].text.clone())
                        .unwrap_or_default(),
                    None => String::new(),
                }
            })
            .font(Font::Body)
            .color(Color::WHITE)
            .max_lines(3)
            .grow_w()
            .id(id!(cw_clue_text)),
        ))
        .spacing(Space::S)
        .align(VAlign::Center)
        .grow_w()
        .on_tap(move || xu.edit(|s| s.toggle_dir())),
        chevron(true, id!(cw_next_clue), move || {
            fu.edit(|s| s.next_clue(true))
        }),
    ))
    .spacing(Space::XS)
    .align(VAlign::Center)
    .padding(Insets::vertical(4.0))
    .background(CLUE_ON)
    .corner_radius(12.0)
    .any()
}

/// A previous or next button: a chevron in a 44-point touch target.
fn chevron(forward: bool, id: ElementId, action: impl Fn() + 'static) -> AnyPiece {
    let a11y = if forward {
        tr("cw_next_clue")
    } else {
        tr("cw_prev_clue")
    };
    canvas(move |d, sz| {
        let c = Point::new(sz.width / 2.0, sz.height / 2.0);
        let dx = if forward { 4.0 } else { -4.0 };
        let tip = Point::new(c.x + dx, c.y);
        for dy in [-8.0, 8.0] {
            d.stroke_styled(
                Shape::Line(Point::new(c.x - dx, c.y + dy), tip),
                Color::rgba(1.0, 1.0, 1.0, 0.85),
                chrome::stroke_style(22.0),
            );
        }
    })
    .on_tap(action)
    .a11y(move |a| a.label(a11y).role(Role::Button))
    .id(id)
    .frame(chrome::BUTTON, chrome::BUTTON)
    .any()
}

/// One direction's clues under its heading, scrolling, with the current clue lit and the
/// crossing clue lit faintly. A clue whose word is full dims. Tapping a clue goes to its word.
fn clue_list(ui: &Rc<Ui>, dir: Dir) -> AnyPiece {
    let clues: Vec<(usize, u16, String)> = ui
        .solve
        .borrow()
        .as_ref()
        .map(|s| {
            s.puzzle
                .clues
                .iter()
                .enumerate()
                .filter(|(_, c)| c.dir == dir)
                .map(|(k, c)| (k, c.number, c.text.clone()))
                .collect()
        })
        .unwrap_or_default();
    let mut rows: Vec<AnyPiece> = Vec::with_capacity(clues.len());
    for (k, number, text) in clues {
        let (bu, cu, du, tu) = (ui.clone(), ui.clone(), ui.clone(), ui.clone());
        let full = move |u: &Ui| {
            u.board.track();
            u.solve.borrow().as_ref().is_some_and(|s| {
                s.puzzle.clues[k]
                    .cells
                    .iter()
                    .all(|&i| !s.progress.entries[i].is_empty())
            })
        };
        rows.push(
            row((
                label(number.to_string())
                    .font(Font::Body)
                    .bold()
                    .tabular()
                    .color(move || {
                        if full(&cu) {
                            chrome::TEXT_DIM
                        } else {
                            Color::WHITE
                        }
                    })
                    .align(TextAlign::Trailing)
                    .min_width(30.0),
                label(text)
                    .font(Font::Body)
                    .color(move || {
                        if full(&du) {
                            chrome::TEXT_DIM
                        } else {
                            chrome::TEXT
                        }
                    })
                    .grow_w(),
            ))
            .spacing(Space::S)
            .align(VAlign::FirstBaseline)
            .padding(Insets::new(6.0, 4.0, 6.0, 8.0))
            .background(move || {
                bu.board.track();
                let s = bu.solve.borrow();
                match s.as_ref() {
                    Some(s) if s.current_clue() == Some(k) => CLUE_ON,
                    Some(s) if s.crossing_clue() == Some(k) => CLUE_CROSS,
                    _ => CLEAR,
                }
            })
            .corner_radius(8.0)
            .on_tap(move || {
                tu.board_focus.set(true);
                tu.edit(|s| s.select_clue(k));
                if tu.overlay.get_untracked() == Overlay::Clues {
                    tu.show(Overlay::None);
                }
            })
            .id(id!(cw_clue, k as u64))
            .any(),
        );
    }
    let target = match dir {
        Dir::Across => ui.across_scroll,
        Dir::Down => ui.down_scroll,
    };
    column((
        chrome::section_heading(match dir {
            Dir::Across => tr("cw_across"),
            Dir::Down => tr("cw_down"),
        }),
        scroll(column(rows).spacing(2.0).grow_w())
            .scroll_target(target)
            .grow(),
    ))
    .spacing(Space::S)
    .align(HAlign::Leading)
    .width(LIST_W)
    .grow_h()
    .any()
}

// ---------------------------------------------------------------------------
// Tools and the keyboard
// ---------------------------------------------------------------------------

/// Pencil, Check, Reveal and Clear, and on a phone the full clue lists.
fn tools(ui: &Rc<Ui>) -> AnyPiece {
    let (pu, cu, ru, xu, lu) = (ui.clone(), ui.clone(), ui.clone(), ui.clone(), ui.clone());
    let pencil_on = {
        let u = ui.clone();
        move || {
            u.board.track();
            u.solve.borrow().as_ref().is_some_and(|s| s.pencil_mode)
        }
    };
    let list_button = (layout() != Layout::Wide).then(move || {
        tool_button(tr("cw_clues"), || false, id!(cw_show_clues), move || {
            lu.push(Overlay::Clues)
        })
    });
    row((
        tool_button(tr("cw_pencil"), pencil_on, id!(cw_pencil), move || {
            pu.edit(|s| s.pencil_mode = !s.pencil_mode)
        }),
        tool_button(tr("cw_check"), || false, id!(cw_check), move || {
            cu.push(Overlay::Check)
        }),
        tool_button(tr("cw_reveal"), || false, id!(cw_reveal), move || {
            ru.push(Overlay::Reveal)
        }),
        tool_button(tr("cw_clear"), || false, id!(cw_clear), move || {
            xu.push(Overlay::Clear)
        }),
        list_button,
    ))
    .spacing(Space::S)
    .any()
}

/// A tool: a caption in a rounded key, lit while its mode is on.
fn tool_button(
    title: LocalizedText,
    on: impl Fn() -> bool + 'static,
    id: ElementId,
    action: impl Fn() + 'static,
) -> AnyPiece {
    let spoken = title.clone();
    canvas(move |d, sz| {
        d.fill(
            Shape::RoundedRect(Rect::new(0.0, 0.0, sz.width, sz.height), 10.0),
            if on() { KEY_ON } else { KEY },
        );
        let text = title.format();
        let font = chrome::canvas_font(FontWeight::Semibold);
        d.text(
            &text,
            Point::new(sz.width / 2.0, sz.height / 2.0),
            TextStyle {
                size: chrome::fit_text(&text, 15.0, &font, sz.width - 8.0),
                color: Color::WHITE,
                anchor: TextAnchor::CENTERED,
                font,
            },
        );
    })
    .on_tap(action)
    .a11y(move |a| a.label(spoken).role(Role::Button))
    .id(id)
    .height(36.0)
    .grow_w()
    .max_width(110.0)
    .any()
}

/// The keyboard's letters, row by row; the last row also carries backspace.
const KEY_ROWS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];

/// Each key's rectangle and what it types (`None` for backspace), in a keyboard `width` wide:
/// ten keys to the top row, the second row centered under it, the third row's letters centered
/// in what the wide backspace key at its end leaves them.
fn key_layout(width: f64) -> Vec<(Rect, Option<char>)> {
    let unit = ((width - KEY_GAP * 9.0) / 10.0).max(10.0);
    let back = unit * 1.5;
    let mut keys = Vec::new();
    for (r, letters) in KEY_ROWS.iter().enumerate() {
        let y = r as f64 * (KEY_H + KEY_GAP);
        let n = letters.chars().count() as f64;
        let row_w = n * unit + (n - 1.0) * KEY_GAP;
        let room = if r == 2 { width - back - KEY_GAP } else { width };
        let x0 = (room - row_w) / 2.0;
        for (k, ch) in letters.chars().enumerate() {
            let x = x0 + k as f64 * (unit + KEY_GAP);
            keys.push((Rect::new(x, y, unit, KEY_H), Some(ch)));
        }
        if r == 2 {
            keys.push((Rect::new(width - back, y, back, KEY_H), None));
        }
    }
    keys
}

fn keyboard(ui: &Rc<Ui>) -> Option<AnyPiece> {
    if !touch_keyboard() {
        return None;
    }
    let (du, tu) = (ui.clone(), ui.clone());
    let height = KEY_H * 3.0 + KEY_GAP * 2.0;
    Some(
        canvas(move |d, sz| {
            du.keyboard_width.set(sz.width);
            let font = chrome::canvas_font(FontWeight::Medium);
            for (rect, ch) in key_layout(sz.width) {
                d.fill(Shape::RoundedRect(rect, 7.0), KEY);
                let c = Point::new(
                    rect.origin.x + rect.size.width / 2.0,
                    rect.origin.y + rect.size.height / 2.0,
                );
                match ch {
                    Some(ch) => d.text(
                        &ch.to_string(),
                        c,
                        TextStyle {
                            size: 20.0,
                            color: Color::WHITE,
                            anchor: TextAnchor::CENTERED,
                            font: font.clone(),
                        },
                    ),
                    None => draw_backspace(d, c),
                }
            }
        })
        .on_tap_at(move |pt| {
            let keys = key_layout(tu.keyboard_width.get());
            let half = KEY_GAP / 2.0;
            let hit = keys.iter().find(|(r, _)| {
                pt.x >= r.origin.x - half
                    && pt.x <= r.origin.x + r.size.width + half
                    && pt.y >= r.origin.y - half
                    && pt.y <= r.origin.y + r.size.height + half
            });
            match hit {
                Some((_, Some(ch))) => tu.type_letter(*ch),
                Some((_, None)) => tu.edit(|s| s.backspace()),
                None => {}
            }
        })
        .a11y(|a| a.label(tr("cw_keyboard")))
        .id(id!(cw_keyboard))
        .height(height)
        .grow_w()
        .max_width(560.0)
        .any(),
    )
}

/// The backspace key's glyph: a tag pointing left with a cross in it.
fn draw_backspace(d: &mut Draw, c: Point) {
    let (w, h) = (22.0, 16.0);
    let l = c.x - w / 2.0;
    let style = chrome::stroke_style(16.0);
    let outline = [
        Point::new(l, c.y),
        Point::new(l + 6.0, c.y - h / 2.0),
        Point::new(l + w, c.y - h / 2.0),
        Point::new(l + w, c.y + h / 2.0),
        Point::new(l + 6.0, c.y + h / 2.0),
        Point::new(l, c.y),
    ];
    for pair in outline.windows(2) {
        d.stroke_styled(
            Shape::Line(pair[0], pair[1]),
            Color::WHITE,
            style.clone(),
        );
    }
    let x = l + 13.5;
    for dy in [-3.5, 3.5] {
        d.stroke_styled(
            Shape::Line(Point::new(x - 3.5, c.y - dy), Point::new(x + 3.5, c.y + dy)),
            Color::WHITE,
            style.clone(),
        );
    }
}

// ---------------------------------------------------------------------------
// Overlays
// ---------------------------------------------------------------------------

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
            Overlay::Library => Some(library(ui.clone())),
            Overlay::Check | Overlay::Reveal | Overlay::Clear => {
                Some(scope_card(ui.clone(), *kind))
            }
            Overlay::Clues => Some(clues_card(ui.clone())),
            Overlay::Solved => Some(solved_card(ui.clone())),
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
            chrome::menu_button(tr("gk_resume"), chrome::GREEN, id!(cw_resume), move || {
                u1.show(Overlay::None)
            }),
            chrome::menu_button(tr("cw_puzzles"), chrome::BLUE, id!(cw_puzzles), move || {
                u2.push(Overlay::Library)
            }),
            chrome::menu_button(tr("cw_restart"), chrome::AMBER, id!(cw_restart), move || {
                u3.edit(|s| s.restart());
                u3.show(Overlay::None);
            }),
            chrome::menu_button(
                tr("gk_instructions"),
                chrome::INDIGO,
                id!(cw_instructions),
                move || u4.push(Overlay::Instructions),
            ),
            chrome::close_game_button(chrome::RED, id!(cw_quit)),
        ))
        .spacing(14.0)
        .align(HAlign::Center),
    )
    .id(id!(cw_pause_menu))
    .any()
}

/// Check, Reveal or Clear, asking how much: the square, the word, or the puzzle.
fn scope_card(ui: Rc<Ui>, kind: Overlay) -> AnyPiece {
    let (title, tint) = match kind {
        Overlay::Check => (tr("cw_check"), chrome::BLUE),
        Overlay::Reveal => (tr("cw_reveal"), chrome::AMBER),
        _ => (tr("cw_clear"), chrome::SLATE),
    };
    let act = move |ui: &Rc<Ui>, scope: Scope| {
        let ui = ui.clone();
        move || {
            ui.edit(|s| match kind {
                Overlay::Check => {
                    s.check(scope);
                }
                Overlay::Reveal => s.reveal(scope),
                _ => s.clear(scope),
            });
            // A reveal that finished the puzzle has already put the solved card up.
            if ui.overlay.get_untracked() == kind {
                ui.return_to.set(Overlay::None);
                ui.show(Overlay::None);
            }
        }
    };
    let cancel = ui.clone();
    chrome::card(
        column((
            label(title).font(Font::Title2).bold().color(Color::WHITE),
            chrome::menu_button(
                tr("cw_square"),
                tint,
                id!(cw_scope_square),
                act(&ui, Scope::Square),
            ),
            chrome::menu_button(tr("cw_word"), tint, id!(cw_scope_word), act(&ui, Scope::Word)),
            chrome::menu_button(
                tr("cw_puzzle"),
                tint,
                id!(cw_scope_puzzle),
                act(&ui, Scope::Puzzle),
            ),
            button(tr("gk_cancel"))
                .action(move || cancel.pop())
                .id(id!(cw_scope_cancel)),
        ))
        .spacing(14.0)
        .align(HAlign::Center),
    )
    .id(id!(cw_scope_card))
    .any()
}

/// A phone's full clue lists, as a card over the grid.
fn clues_card(ui: Rc<Ui>) -> AnyPiece {
    let done = ui.clone();
    chrome::card(
        column((
            column((clue_list(&ui, Dir::Across), clue_list(&ui, Dir::Down)))
                .spacing(Space::M)
                .max_height(520.0),
            button(tr("gk_done"))
                .prominent()
                .action(move || done.pop())
                .id(id!(cw_clues_done)),
        ))
        .spacing(Space::M)
        .align(HAlign::Center),
    )
    .id(id!(cw_clues_card))
    .any()
}

/// The library: every puzzle, with how far along it is.
fn library(ui: Rc<Ui>) -> AnyPiece {
    let mut rows: Vec<AnyPiece> = Vec::new();
    for (n, entry) in BUNDLED.iter().enumerate() {
        let Some(p) = entry.load() else { continue };
        let progress = ui.saves.borrow().progress.get(entry.id).cloned();
        let status = match &progress {
            Some(pr) if pr.solved => tr("cw_status_solved")
                .arg("time", fmt_time(pr.elapsed as u64))
                .format(),
            Some(pr) if pr.filled(&p) > 0.0 => tr("cw_status_started")
                .arg("percent", (pr.filled(&p) * 100.0).round())
                .format(),
            _ => tr("cw_status_new").format(),
        };
        let u = ui.clone();
        let id = entry.id;
        let title = p.title.clone();
        rows.push(
            column((
                label(p.title.clone())
                    .font(Font::Headline)
                    .color(Color::WHITE),
                label(format!(
                    "{} · {}×{} · {}",
                    entry.source, p.width, p.height, p.author
                ))
                .font(Font::Subheadline)
                .color(chrome::TEXT_DIM),
                label(status).font(Font::Footnote).color(chrome::TEXT),
            ))
            .spacing(2.0)
            .align(HAlign::Leading)
            .padding(Space::M)
            .grow_w()
            .background(Color::rgba(1.0, 1.0, 1.0, 0.07))
            .corner_radius(12.0)
            .on_tap(move || u.open_puzzle(id))
            .a11y(move |a| a.label(title).role(Role::Button))
            .id(id!(cw_entry, n as u64))
            .any(),
        );
    }
    let back = ui.clone();
    let has_open = ui.solve.borrow().is_some();
    chrome::card(
        column((
            label(tr("cw_puzzles"))
                .font(Font::Title2)
                .bold()
                .color(Color::WHITE),
            scroll(column(rows).spacing(Space::S).grow_w()).max_height(440.0),
            has_open.then(move || {
                button(tr("gk_done"))
                    .action(move || back.pop())
                    .id(id!(cw_library_done))
            }),
        ))
        .spacing(Space::M)
        .align(HAlign::Leading)
        .grow_w()
        .max_width(chrome::CARD_W),
    )
    .id(id!(cw_library))
    .any()
}

fn solved_card(ui: Rc<Ui>) -> AnyPiece {
    let (time, assisted) = ui
        .solve
        .borrow()
        .as_ref()
        .map(|s| (s.progress.elapsed as u64, s.progress.assisted))
        .unwrap_or_default();
    let (u1, u2) = (ui.clone(), ui.clone());
    chrome::card(
        column((
            chrome::card_title(tr("cw_solved"), chrome::GOLD),
            chrome::stat(
                tr("cw_time"),
                fmt_time(time),
                Font::Title,
                Color::WHITE,
                id!(cw_solved_time),
            ),
            label(if assisted {
                tr("cw_solved_assisted")
            } else {
                tr("cw_solved_clean")
            })
            .font(Font::Body)
            .color(chrome::TEXT)
            .align(TextAlign::Center),
            chrome::menu_button(
                tr("cw_puzzles"),
                chrome::BLUE,
                id!(cw_solved_puzzles),
                move || u1.push(Overlay::Library),
            ),
            chrome::menu_button(tr("cw_admire"), chrome::SLATE, id!(cw_admire), move || {
                u2.show(Overlay::None)
            }),
        ))
        .spacing(14.0)
        .align(HAlign::Center),
    )
    .id(id!(cw_solved_card))
    .any()
}

fn instructions_card(ui: Rc<Ui>) -> AnyPiece {
    chrome::instructions_card(
        tr("cw_help_title"),
        vec![
            Help::Para(tr("cw_help_goal")),
            Help::Heading(tr("cw_help_moving")),
            Help::Para(tr("cw_help_moving_1")),
            Help::Para(tr("cw_help_moving_2")),
            Help::Heading(tr("cw_help_typing")),
            Help::Para(tr("cw_help_typing_1")),
            Help::Para(tr("cw_help_typing_2")),
            Help::Heading(tr("cw_help_help")),
            Help::Para(tr("cw_help_help_1")),
        ],
        id!(cw_help_done),
        move || ui.pop(),
    )
}

// ---------------------------------------------------------------------------
// The home tile
// ---------------------------------------------------------------------------

/// The home tile: a small filled grid with the cursor in it and its word lit.
pub fn crossword_preview() -> AnyPiece {
    const FILL: [&str; 25] = [
        "#", "C", "L", "U", "E", //
        "A", "R", "E", "N", "A", //
        "S", "O", "", "", "", //
        "", "S", "", "", "", //
        "", "S", "", "", "#",
    ];
    canvas(|d, sz| {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, sz.width, sz.height)), BG);
        let n = 5.0;
        let side = (sz.width.min(sz.height) * 0.86 / n).floor();
        let ox = ((sz.width - side * n) / 2.0).floor();
        let oy = ((sz.height - side * n) / 2.0).floor();
        let font = chrome::canvas_font(FontWeight::Semibold);
        for (i, cell) in FILL.iter().enumerate() {
            let (r, c) = ((i / 5) as f64, (i % 5) as f64);
            let rect = Rect::new(ox + c * side, oy + r * side, side, side);
            let fill = if *cell == "#" {
                BLOCK
            } else if i == 12 {
                CURSOR
            } else if (10..15).contains(&i) {
                WORD
            } else {
                PAPER
            };
            d.fill(Shape::Rect(rect), fill);
            d.stroke(Shape::Rect(rect), RULE, 1.0);
            if *cell != "#" && !cell.is_empty() {
                d.text(
                    cell,
                    Point::new(rect.origin.x + side / 2.0, rect.origin.y + side * 0.6),
                    TextStyle {
                        size: side * 0.55,
                        color: INK,
                        anchor: TextAnchor::CENTERED,
                        font: font.clone(),
                    },
                );
            }
        }
        d.stroke(Shape::Rect(Rect::new(ox, oy, side * n, side * n)), BLOCK, 2.0);
    })
    .any()
}
