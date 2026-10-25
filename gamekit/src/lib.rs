// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! gamekit is the games' standard save-state mechanism. Each game serializes its durable state
//! (board, score, best) to JSON in `day-part-preferences` (docs/prefs.md) and restores it the next
//! time it opens. Two write triggers cover every exit path:
//!
//! - **game exit**: [`autosave`] hooks the page scope's cleanup, so closing the cover (the X
//!   button, system back, a route change) saves, on every backend, ArkUI included;
//! - **app backgrounding**: one process-wide set of lifecycle handlers
//!   (`DidEnterBackground` / `WillResignActive` / `WillTerminate`, where the backend delivers
//!   them; docs/lifecycle.md) saves every open game.
//!
//! [`on_background`] rides the same lifecycle handlers for a game that wants to react to
//! leaving the foreground (a timed game pauses its clock) before its state is saved.
//!
//! Records (best scores, best times, win counts) are each game's own keys, but the shell's
//! Settings › Records › Reset all records wipes every game's at once: each game crate exposes a
//! `reset_records()` that clears its keys, and a game that is open registers [`on_reset_records`]
//! so the bests it holds in memory follow (otherwise its next save would write them back).
//!
//! [`chrome`] is the shell every game shares: the pause menu, the results cards, the
//! settings and how-to-play sheets, the pause button, and the haptic gate.

pub mod animation;
pub mod chrome;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use day_reactive::Scope;
use day_spec::Lifecycle;
use serde::Serialize;
use serde::de::DeserializeOwned;

thread_local! {
    /// A fixed seed for scripted runs (see [`set_seed_override`]).
    static SEED_OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Make every later [`seed`] return `seed`, what the app installs from `DAY_GAMES_SEED` so a
/// dayscript walkthrough meets the same puzzle, brick layout, and piece order on every run
/// and every target. One value for all games: they are independent, so sharing it costs
/// nothing, and it keeps the answer independent of which game's preview drew first.
pub fn set_seed_override(seed: u64) {
    SEED_OVERRIDE.with(|s| s.set(Some(seed | 1)));
}

/// A fresh RNG seed per game start: the wall clock's nanoseconds where the platform has one,
/// browser entropy on the web (wasm32 has no `SystemTime`, and asking aborts the app). Always
/// odd, so an xorshift never seeds to zero.
pub fn seed() -> u64 {
    if let Some(fixed) = SEED_OVERRIDE.with(|s| s.get()) {
        return fixed;
    }
    #[cfg(not(target_arch = "wasm32"))]
    let raw = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    #[cfg(target_arch = "wasm32")]
    let raw = {
        let mut bytes = [0u8; 8];
        // A failed fill leaves zeros; the `| 1` below still yields a valid (if fixed) seed.
        let _ = getrandom::fill(&mut bytes);
        u64::from_le_bytes(bytes)
    };
    raw | 1
}

fn pref_key(key: &str) -> String {
    format!("save.{key}")
}

/// The saved state for `key`, if a readable one exists. An unparsable save (a schema change)
/// is discarded rather than propagated: the game starts fresh.
pub fn restore<T: DeserializeOwned>(key: &str) -> Option<T> {
    let raw = day_part_preferences::get(&pref_key(key))?;
    match serde_json::from_str(&raw) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!("gamekit: discarding unreadable save {key:?}: {e}");
            let _ = day_part_preferences::remove(&pref_key(key));
            None
        }
    }
}

/// Persist `state` under `key` now.
pub fn save<T: Serialize>(key: &str, state: &T) {
    match serde_json::to_string(state) {
        Ok(s) => {
            let _ = day_part_preferences::set(&pref_key(key), &s);
        }
        Err(e) => eprintln!("gamekit: failed to serialize save {key:?}: {e}"),
    }
}

/// Delete the saved state for `key`.
pub fn clear(key: &str) {
    let _ = day_part_preferences::remove(&pref_key(key));
}

thread_local! {
    /// Save closures for the games currently open (usually zero or one).
    static LIVE: RefCell<HashMap<String, Rc<dyn Fn()>>> = RefCell::new(HashMap::new());
    /// Background hooks for the games currently open, run before the saves.
    static BACKGROUND: RefCell<HashMap<String, Rc<dyn Fn()>>> = RefCell::new(HashMap::new());
    /// What the close button in a game's header does, registered by the shell ([`on_close`]).
    /// One at a time: the shell shows one game at a time.
    static CLOSE: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
    /// How each open game forgets the records it holds in memory ([`on_reset_records`]).
    static RESET_RECORDS: RefCell<HashMap<String, Rc<dyn Fn()>>> = RefCell::new(HashMap::new());
    static LIFECYCLE_HOOKED: Cell<bool> = const { Cell::new(false) };
}

fn save_all() {
    // Clone the closures out so a hook that touches the registry can't deadlock the borrow.
    let hooks: Vec<Rc<dyn Fn()>> = BACKGROUND.with(|m| m.borrow().values().cloned().collect());
    for f in hooks {
        f();
    }
    let snap: Vec<Rc<dyn Fn()>> = LIVE.with(|m| m.borrow().values().cloned().collect());
    for f in snap {
        f();
    }
}

/// One process-wide lifecycle registration; `lifecycle_supported` skips phases this backend
/// never delivers (docs/lifecycle.md); scope-cleanup saving still covers those platforms.
fn hook_lifecycle() {
    let first = LIFECYCLE_HOOKED.with(|h| !h.replace(true));
    if first {
        for phase in [
            Lifecycle::DidEnterBackground,
            Lifecycle::WillResignActive,
            Lifecycle::WillTerminate,
        ] {
            if day_core::lifecycle_supported(phase) {
                day_core::on_lifecycle(phase, save_all);
            }
        }
    }
}

/// Run `f` whenever the app leaves the foreground while the current scope (the game's page) is
/// alive, before that game's [`autosave`] snapshot is taken, so a clock it stops is saved
/// stopped. Desktop backends deliver no such phase; the hook is never called there.
pub fn on_background(key: &'static str, f: impl Fn() + 'static) {
    let hook: Rc<dyn Fn()> = Rc::new(f);
    BACKGROUND.with(|m| m.borrow_mut().insert(key.to_string(), hook));
    hook_lifecycle();
    Scope::current().on_cleanup(move || {
        BACKGROUND.with(|m| {
            m.borrow_mut().remove(key);
        });
    });
}

/// Keep `f` registered as the way `key`'s game forgets the records it holds in memory, for as
/// long as the current scope (the game's page) is alive. The game's own `reset_records()` runs
/// it through [`reset_live_records`] after clearing the stored keys, so a reset from the shell's
/// Settings while the game is open on a desktop lands on the live bests too, and the game's next
/// save writes the reset values rather than the old ones. Call once from the game's page builder.
pub fn on_reset_records(key: &'static str, f: impl Fn() + 'static) {
    let hook: Rc<dyn Fn()> = Rc::new(f);
    RESET_RECORDS.with(|m| m.borrow_mut().insert(key.to_string(), hook));
    Scope::current().on_cleanup(move || {
        RESET_RECORDS.with(|m| {
            m.borrow_mut().remove(key);
        });
    });
}

/// Run what `key`'s game registered with [`on_reset_records`], if it is open: true when it was.
/// A closed game has nothing in memory; its stored records are already gone by the time this is
/// asked.
pub fn reset_live_records(key: &str) -> bool {
    // Cloned out first: the hook may redraw or re-show an overlay, which must not find the
    // registry borrowed.
    let hook = RESET_RECORDS.with(|m| m.borrow().get(key).cloned());
    match hook {
        Some(f) => {
            f();
            true
        }
        None => false,
    }
}

/// Register what leaving a game does, for as long as the current scope (the shell's game cover)
/// is alive. Leaving belongs to the shell (it owns the cover) while the button that asks for it
/// sits in the game's header row ([`chrome::game_header`]), which is what keeps the close
/// button aligned with the title and the pause button on every game.
pub fn on_close(f: impl Fn() + 'static) {
    let hook: Rc<dyn Fn()> = Rc::new(f);
    CLOSE.with(|c| *c.borrow_mut() = Some(hook));
    Scope::current().on_cleanup(|| {
        CLOSE.with(|c| *c.borrow_mut() = None);
    });
}

/// Leave the open game, through whatever the shell registered with [`on_close`]. Inert when
/// nothing did: a game built outside the shell has no cover to close.
pub fn close() {
    // Cloned out of the cell first: the hook drops the cover, which clears this very registry.
    let hook = CLOSE.with(|c| c.borrow().clone());
    if let Some(f) = hook {
        f();
    }
}

/// Load the shell's shared clips and `clips` while the current scope (the game's page) is alive,
/// and release every one of them when it closes, which also stops any loop still sounding
/// ([`chrome::sound_loop`]). Call once from the game's page builder.
pub fn sounds(clips: &'static [chrome::Sfx]) {
    day_part_sound::preload(chrome::cues::SHARED);
    day_part_sound::preload(clips);
    Scope::current().on_cleanup(day_part_sound::unload_all);
}

/// Keep `snapshot` registered as `key`'s live state provider while the current scope (the
/// game's page) is alive: the state is saved when the scope is disposed (the game exited) and
/// whenever the app is backgrounded. Call once from the game's page builder.
pub fn autosave<T: Serialize>(key: &'static str, snapshot: impl Fn() -> T + 'static) {
    let saver: Rc<dyn Fn()> = Rc::new(move || save(key, &snapshot()));
    LIVE.with(|m| m.borrow_mut().insert(key.to_string(), saver.clone()));
    hook_lifecycle();

    Scope::current().on_cleanup(move || {
        saver();
        LIVE.with(|m| {
            m.borrow_mut().remove(key);
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game's live hook runs while its page scope is alive and is forgotten with it.
    #[test]
    fn reset_hook_lives_with_its_scope() {
        let ran = Rc::new(Cell::new(0));
        let page = Scope::detached();
        page.enter({
            let ran = ran.clone();
            move || on_reset_records("test.records", move || ran.set(ran.get() + 1))
        });
        assert!(reset_live_records("test.records"));
        assert_eq!(ran.get(), 1);
        assert!(!reset_live_records("other.records"));
        page.dispose();
        assert!(!reset_live_records("test.records"));
        assert_eq!(ran.get(), 1);
    }
}
