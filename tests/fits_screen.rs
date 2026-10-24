// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Every game fits the screen it is given: nothing drawn past the window's edges, and no page
//! that has to scroll to show its own controls. The narrowest iPhone layout is 320 points wide
//! (an iPhone 13 mini or SE with Display Zoom set to Larger Text), and a game that assumed the
//! 375 points of the default zoom clipped its board on both sides and scrolled its keypad.
//!
//! Each game opens fresh, so its first-run how-to-play card comes up by itself: the card is
//! checked first, then the page under it once the card is dismissed.
#![cfg(feature = "mock")]

use day::prelude::*;
use day::reactive::flush_sync;
use day_mock::{MockProbe, MockState, MockToolkit};
use day_spec::{Event, SizeClass};
use std::collections::HashMap;

/// What a pixel of rounding is allowed to stick out by.
const SLACK: f64 = 0.5;

/// Page-sized scrolls that are meant to scroll: a list longer than any screen, not a page that
/// failed to fit its controls.
const LISTS: &[&str] = &["ch_deck_list"];

/// Every key a game saves, so each opens as it would the first time, put back afterwards.
const KEYS: &[&str] = &[
    "blockblast.best",
    "blockblast.settings",
    "blockblast.v1",
    "breakout.best",
    "breakout.settings",
    "breakout.v2",
    "charades.records",
    "charades.rotation",
    "charades.settings",
    "flipside.settings",
    "flipside.v1",
    "matchthree.settings",
    "matchthree.v1",
    "mines.records",
    "mines.settings",
    "mines.v1",
    "pacpac.best.mspacpac",
    "pacpac.best.pacpac",
    "pacpac.settings",
    "pacpac.v1",
    "pipes.records",
    "pipes.settings",
    "pipes.v1",
    "sirtet.best",
    "sirtet.settings",
    "sirtet.v1",
    "solitaire.settings",
    "solitaire.stats",
    "solitaire.v1",
    "sudoku.records",
    "sudoku.settings",
    "sudoku.v2",
    "twentyfortyeight.best",
    "twentyfortyeight.settings",
    "twentyfortyeight.v1",
];

/// The keys' values before the test, put back on drop even when an assertion fails. The prefs
/// store is the machine's own and other test binaries share it.
struct Saved {
    values: Vec<(String, Option<String>)>,
    selected: Signal<usize>,
}

impl Saved {
    fn take(selected: Signal<usize>) -> Self {
        let values = KEYS
            .iter()
            .map(|k| {
                let key = format!("save.{k}");
                let previous = day::prefs::get(&key);
                day::prefs::remove(&key);
                (key, previous)
            })
            .collect();
        Saved { values, selected }
    }

    /// Forget what the last game saved, so the next one also opens as new.
    fn clear(&self) {
        for (key, _) in &self.values {
            day::prefs::remove(key);
        }
    }
}

impl Drop for Saved {
    fn drop(&mut self) {
        // Closing first runs the autosaves, so they land before the old values go back.
        self.selected.set(0);
        flush_sync();
        day_core::uninstall_tree();
        for (key, value) in &self.values {
            match value {
                Some(v) => {
                    day::prefs::set(key, v);
                }
                None => {
                    day::prefs::remove(key);
                }
            }
        }
    }
}

type Page = fn() -> AnyPiece;

/// Each game: its page, and the button that dismisses its first-run how-to-play card.
const GAMES: &[(&str, Page, &str)] = &[
    ("blockblast", blockblast::blockblast_page, "bb_help_done"),
    ("breakout", breakout::breakout_page, "bk_help_done"),
    ("charades", charades::charades_page, "ch_help_done"),
    ("flipside", flipside::flipside_page, "fs_help_done"),
    ("matchthree", matchthree::matchthree_page, "mt_help_done"),
    ("mines", mines::mines_page, "mi_help_done"),
    ("pacpac", pacpac::pacpac_page, "pm_help_done"),
    ("pipes", pipes::pipes_page, "pp_help_done"),
    ("sirtet", sirtet::sirtet_page, "st_help_done"),
    ("solitaire", solitaire::solitaire_page, "sol_help_done"),
    ("sudoku", sudoku::sudoku_page, "su_help_done"),
    (
        "twentyfortyeight",
        twentyfortyeight::twentyfortyeight_page,
        "tf_help_done",
    ),
];

fn tap(id: &str) -> bool {
    let Some(node) = day_core::with_tree(|t| t.find_by_id(id)) else {
        return false;
    };
    let node = day_core::rnode_to_id(node);
    day_core::enqueue_event(node, Event::Pressed);
    day_core::enqueue_event(node, Event::Tap(Point::new(10.0, 10.0)));
    flush_sync();
    true
}

/// Each widget's frame in window coordinates, and whether a scroll holds it.
fn window_frames(s: &MockState) -> HashMap<u64, (Rect, bool)> {
    let mut parent: HashMap<u64, u64> = HashMap::new();
    for (&id, w) in &s.widgets {
        for &c in &w.children {
            parent.insert(c, id);
        }
    }
    s.widgets
        .iter()
        .map(|(&id, w)| {
            let mut origin = w.frame.origin;
            let mut scrolled = false;
            let mut at = id;
            while let Some(&p) = parent.get(&at) {
                let pw = &s.widgets[&p];
                if pw.kind == "day.scroll" {
                    scrolled = true;
                }
                origin.x += pw.frame.origin.x;
                origin.y += pw.frame.origin.y;
                at = p;
            }
            (
                id,
                (
                    Rect::new(origin.x, origin.y, w.frame.size.width, w.frame.size.height),
                    scrolled,
                ),
            )
        })
        .collect()
}

/// The window frame of the widget with accessibility identifier `ident`.
fn frame_of(s: &MockState, ident: &str) -> Rect {
    let frames = window_frames(s);
    s.widgets
        .iter()
        .find(|(_, w)| w.a11y.identifier.as_deref() == Some(ident))
        .map(|(id, _)| frames[id].0)
        .unwrap_or_else(|| panic!("no widget {ident}"))
}

/// Every widget that spills past a `window`-sized screen, and every scroll the size of the
/// screen whose content does not fit in it (a page that scrolls to reach its own controls).
/// Content inside a smaller scroll is that scroll's business: a long help text is meant to.
fn overflows(s: &MockState, window: Size) -> Vec<String> {
    let frames = window_frames(s);
    let mut found = Vec::new();
    for (&id, w) in &s.widgets {
        if w.frame.size.width <= 0.0 || w.frame.size.height <= 0.0 {
            continue;
        }
        let (f, scrolled) = frames[&id];
        let name = match &w.a11y.identifier {
            Some(ident) => format!("{} {ident}", w.kind),
            None => format!("{} #{id}", w.kind),
        };
        if !scrolled
            && (f.origin.x < -SLACK
                || f.origin.y < -SLACK
                || f.origin.x + f.size.width > window.width + SLACK
                || f.origin.y + f.size.height > window.height + SLACK)
        {
            found.push(format!(
                "{name} at ({:.0}, {:.0}) {:.0}×{:.0}",
                f.origin.x, f.origin.y, f.size.width, f.size.height
            ));
        }
        let page_sized =
            w.frame.size.width >= window.width * 0.9 && w.frame.size.height >= window.height * 0.6;
        let listed = w
            .a11y
            .identifier
            .as_deref()
            .is_some_and(|i| LISTS.contains(&i));
        if w.kind == "day.scroll"
            && page_sized
            && !listed
            && (w.scroll_content.height > w.frame.size.height + SLACK
                || w.scroll_content.width > w.frame.size.width + SLACK)
        {
            found.push(format!(
                "{name} scrolls: content {:.0}×{:.0} in {:.0}×{:.0}",
                w.scroll_content.width,
                w.scroll_content.height,
                w.frame.size.width,
                w.frame.size.height
            ));
        }
    }
    found.sort();
    found
}

/// Mount every game in a `width`×`height` window, reporting each window size class the way a
/// phone's backend would, and run `each` on every game with its help card up and then down.
fn with_games(width: f64, height: f64, mut each: impl FnMut(&str, &str, &MockProbe)) {
    let selected = Signal::new(0usize);
    let saved = Saved::take(selected);
    let (mock, probe): (MockToolkit, MockProbe) = MockToolkit::new();
    gamekit::set_seed_override(15);
    day_core::launch_with(mock, dayapp::window().size(width, height), move || {
        let pages: Vec<AnyPiece> = GAMES
            .iter()
            .enumerate()
            .map(|(i, &(_, page, _))| when(move || selected.get() == i + 1, page).any())
            .collect();
        zstack(pages).any()
    });
    // The mock reports no size class of its own; a phone's backend reports this one.
    day_core::override_size_class(SizeClass::from_size(width, height));
    flush_sync();
    for (i, &(name, _, help)) in GAMES.iter().enumerate() {
        saved.clear();
        selected.set(i + 1);
        flush_sync();
        each(name, "help card", &probe);
        assert!(tap(help), "{name}: no first-run help card ({help})");
        each(name, "page", &probe);
        selected.set(0);
        flush_sync();
    }
    drop(saved);
}

/// Open every game in a `width`×`height` window and collect what does not fit.
fn check(width: f64, height: f64) -> Vec<String> {
    let window = Size::new(width, height);
    let mut problems = Vec::new();
    with_games(width, height, |name, stage, s| {
        for o in overflows(&s.state.borrow(), window) {
            problems.push(format!("{width}×{height} {name} ({stage}): {o}"));
        }
    });
    problems
}

/// An iPhone 13 mini with Display Zoom: 320×693 points, less the status bar and home indicator.
#[test]
fn every_game_fits_a_zoomed_mini() {
    let problems = check(320.0, 621.0);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// An iPhone SE with Display Zoom, the shortest portrait screen: 320×568, less the status bar.
#[test]
fn every_game_fits_a_zoomed_se() {
    let problems = check(320.0, 548.0);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// An iPhone 13 mini with Display Zoom held sideways: 693×320, less the notch and home
/// indicator insets. The size class says compact height, as the phone's backend reports.
#[test]
fn every_game_fits_a_sideways_zoomed_mini() {
    let problems = check(607.0, 299.0);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Sudoku's board and keypad: their full size where the screen has room, and shrunk to fit
/// where it does not, never past a size a finger can still hit.
#[test]
fn sudoku_board_and_keypad_size_to_the_screen() {
    // (window, the board's cells' span, a digit key's side): the default zoom keeps today's
    // sizes, a 350-point board whose rim leaves its nine cells 342 points.
    let cases = [
        ((375.0, 734.0), 342.0..=342.0, 64.0..=64.0),
        ((320.0, 621.0), 290.0..=320.0, 50.0..=64.0),
        ((320.0, 548.0), 220.0..=320.0, 50.0..=64.0),
        // A 13 mini held sideways with Display Zoom: the keypad moves beside the board.
        ((607.0, 299.0), 200.0..=300.0, 40.0..=64.0),
    ];
    for ((width, height), board, key) in cases {
        let mut seen = false;
        with_games(width, height, |name, stage, s| {
            if name != "sudoku" || stage != "page" {
                return;
            }
            seen = true;
            let state = s.state.borrow();
            let b = frame_of(&state, "su_board");
            let k = frame_of(&state, "su_key:5");
            let at = format!("{width}×{height}");
            assert!(
                (b.size.width - b.size.height).abs() < 1.0,
                "{at}: board {b:?} is not square"
            );
            assert!(
                board.contains(&b.size.width.round()),
                "{at}: board {b:?}, wanted {board:?}"
            );
            assert!(
                key.contains(&k.size.width.round()),
                "{at}: key {k:?}, wanted {key:?}"
            );
            assert!(
                (k.size.width - k.size.height).abs() < 1.0,
                "{at}: key {k:?} is not square"
            );
        });
        assert!(seen, "sudoku never opened");
    }
}

/// The text of the element with id `id`, as `assert_text` reads it.
fn text_of(id: &str) -> String {
    day_core::with_tree(|t| {
        let node = t.find_by_id(id).unwrap_or_else(|| panic!("missing {id}"));
        t.node_probe(node).map(|p| p.text).unwrap_or_default()
    })
}

/// The readouts are drawn, not labels, yet a script and a screen reader still follow them: the
/// value is the canvas's accessibility value, re-read as the game changes.
#[test]
fn a_drawn_readout_follows_the_game() {
    let mut checked = false;
    with_games(375.0, 734.0, |name, stage, _| {
        if name != "pipes" || stage != "page" {
            return;
        }
        checked = true;
        assert_eq!(text_of("pp_moves"), "0");
        let board = day_core::with_tree(|t| t.find_by_id("pp_board")).expect("pp_board");
        day_core::enqueue_event(
            day_core::rnode_to_id(board),
            Event::Key(day_spec::KeyEvent {
                key: "Enter".into(),
                modifiers: 0,
            }),
        );
        flush_sync();
        assert_eq!(text_of("pp_moves"), "1");
    });
    assert!(checked, "pipes never opened");
}
