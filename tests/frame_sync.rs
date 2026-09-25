// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Mount real game pages and deliver manual native frames: idle means no requested callbacks,
//! not merely no repaint. Input goes through the same event handlers as native/Dayscript input.
#![cfg(feature = "mock")]

use day::prelude::*;
use day::reactive::flush_sync;
use day_mock::MockToolkit;
use day_spec::{Event, FrameCallback, FrameStamp, KeyEvent};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

struct Frames {
    pending: Rc<RefCell<VecDeque<(Rc<Cell<bool>>, FrameCallback)>>>,
    time: Cell<f64>,
}
impl Frames {
    fn new() -> Self {
        let pending = Rc::new(RefCell::new(VecDeque::new()));
        let queue = pending.clone();
        day_core::install_frame_requester(move |_, cb| {
            let cancelled = Rc::new(Cell::new(false));
            queue.borrow_mut().push_back((cancelled.clone(), cb));
            Box::new(move || cancelled.set(true))
        });
        Self {
            pending,
            time: Cell::new(0.0),
        }
    }
    fn advance(&self, seconds: f64) {
        let frames = (seconds * 60.0).ceil() as usize;
        for _ in 0..frames {
            self.time.set(self.time.get() + 1.0 / 60.0);
            let pending = std::mem::take(&mut *self.pending.borrow_mut());
            for (cancelled, cb) in pending {
                if !cancelled.get() {
                    cb(FrameStamp::new(self.time.get()));
                }
            }
            flush_sync();
        }
    }
    fn idle(&self) {
        assert_eq!(day_core::frame_consumer_count(), 0);
        assert!(
            self.pending
                .borrow()
                .iter()
                .all(|(cancelled, _)| cancelled.get()),
            "idle game left a native frame request outstanding"
        );
    }
    fn awake(&self) {
        assert!(
            day_core::frame_consumer_count() > 0,
            "input did not wake the game"
        );
    }
}
fn emit(id: &str, event: Event) {
    let node = day_core::with_tree(|t| t.find_by_id(id)).unwrap_or_else(|| panic!("missing {id}"));
    day_core::enqueue_event(day_core::rnode_to_id(node), event);
    flush_sync();
}
fn tap(id: &str) {
    let node = day_core::with_tree(|t| t.find_by_id(id)).unwrap_or_else(|| panic!("missing {id}"));
    let node = day_core::rnode_to_id(node);
    day_core::enqueue_event(node, Event::Pressed);
    day_core::enqueue_event(node, Event::Tap(Point::new(10.0, 10.0)));
    flush_sync();
}
fn key(id: &str, key: &str) {
    emit(
        id,
        Event::Key(KeyEvent {
            key: key.into(),
            modifiers: 0,
        }),
    );
}

struct SavedPreferences {
    values: Vec<(String, Option<String>)>,
    selected: Signal<usize>,
}
impl SavedPreferences {
    fn new(selected: Signal<usize>) -> Self {
        let keys = [
            "pipes.v1",
            "pipes.settings",
            "pipes.records",
            "reversi.v1",
            "reversi.settings",
            "twentyfortyeight.v1",
            "twentyfortyeight.settings",
            "twentyfortyeight.best",
            "matchthree.v1",
            "matchthree.settings",
            "blockblast.v1",
            "blockblast.settings",
            "blockblast.best",
            "solitaire.v1",
            "solitaire.settings",
            "solitaire.stats",
        ];
        let values = keys
            .into_iter()
            .map(|key| {
                let key = format!("save.{key}");
                let previous = day::prefs::get(&key);
                day::prefs::remove(&key);
                (key, previous)
            })
            .collect();
        Self { values, selected }
    }
}
impl Drop for SavedPreferences {
    fn drop(&mut self) {
        // Closing first runs autosaves before restoring the pre-test preferences, even on a
        // failed assertion. File-backed preference stores can be shared with other binaries.
        self.selected.set(0);
        flush_sync();
        day_core::uninstall_tree();
        for (key, value) in &self.values {
            if let Some(value) = value {
                day::prefs::set(key, value);
            } else {
                day::prefs::remove(key);
            }
        }
    }
}

#[test]
fn real_boards_sleep_wake_and_cancel_on_cover_disposal() {
    let selected = Signal::new(0usize);
    let _preferences = SavedPreferences::new(selected);
    let (mock, _) = MockToolkit::new();
    gamekit::set_seed_override(15);
    day_core::launch_with(mock, dayapp::window(), move || {
        zstack((
            when(move || selected.get() == 1, pipes::pipes_page),
            when(move || selected.get() == 2, reversi::reversi_page),
            when(
                move || selected.get() == 3,
                twentyfortyeight::twentyfortyeight_page,
            ),
            when(move || selected.get() == 4, matchthree::matchthree_page),
            when(move || selected.get() == 5, blockblast::blockblast_page),
            when(move || selected.get() == 6, solitaire::solitaire_page),
        ))
        .any()
    });
    let frames = Frames::new();
    let open = |game, help| {
        eprintln!("opening {help}");
        selected.set(game);
        flush_sync();
        tap(help);
        frames.advance(4.0);
        frames.idle();
    };
    let close = || {
        selected.set(0);
        flush_sync();
        frames.idle();
    };

    open(1, "pp-help-done");
    for _ in 0..2 {
        key("pp-board", "Enter");
        frames.awake();
        frames.advance(1.0);
        frames.idle();
    }
    close();

    open(2, "rv-help-done");
    key("rv-board", "Enter"); // D3, followed by the computer's reply.
    frames.awake();
    frames.advance(3.0);
    frames.idle();
    close();

    open(3, "tf-help-done");
    for direction in ["ArrowRight", "ArrowLeft"] {
        key("tf-canvas", direction);
        frames.awake();
        frames.advance(1.0);
        frames.idle();
    }
    close();

    open(4, "mt-help-done");
    for _ in 0..2 {
        for k in ["Home", "Enter", "ArrowRight", "Enter"] {
            key("mt-board", k);
        }
        frames.awake();
        frames.advance(1.0); // Invalid swap returns to its original layout.
        frames.idle();
    }
    tap("mt-hint");
    frames.awake();
    key("mt-board", "Enter");
    close(); // Cancel while a cascade owns pending frames.

    open(5, "bb-help-done");
    key("bb-canvas", "1");
    frames.advance(1.0);
    frames.awake(); // A held piece's clear preview continues pulsing after pickup settles.
    tap("bb-pause");
    frames.idle();
    tap("bb-resume");
    frames.advance(1.0);
    frames.idle();
    close();

    open(6, "sol-help-done");
    tap("sol-hint"); // Before the first move, there is no elapsed clock to keep frames alive.
    frames.awake();
    frames.advance(5.0);
    frames.idle();
    close();
}
