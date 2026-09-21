// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception

use crate::res;
use day::prelude::*;

/// Reapply what the last run chose, before any piece is built. `DAY_THEME` and `DAY_LOCALE`
/// win when set, so a scripted run and a screenshot pass stay fixed.
pub(crate) fn apply_startup() {
    if std::env::var("DAY_LOCALE").is_err()
        && let Some(tag) = day::prefs::get(crate::LOCALE_KEY).filter(|t| !t.is_empty())
    {
        set_locale(&tag);
    }
    if std::env::var("DAY_THEME").is_err() && capability(Cap::Appearance) != Support::Unsupported {
        match day::prefs::get(crate::THEME_KEY).as_deref() {
            Some("light") => day::set_appearance(Some(false)),
            Some("dark") => day::set_appearance(Some(true)),
            _ => {}
        }
    }
}

/// Appearance and language: applied live and persisted through `day::prefs`
/// (https://daybrite.dev/docs/localization).
pub(crate) fn settings_body() -> impl Piece {
    form((section((appearance_row(), language_row())).title(res::str::section_general()),))
}

/// System / Light / Dark, where the backend honors a runtime override.
fn appearance_row() -> AnyPiece {
    if capability(Cap::Appearance) == Support::Unsupported {
        return column(()).any();
    }
    let choice = Signal::new(match day::prefs::get(crate::THEME_KEY).as_deref() {
        Some("light") => 1usize,
        Some("dark") => 2,
        _ => 0,
    });
    // The picker writes the index; this applies and persists what it wrote.
    watch(
        move || choice.get(),
        move |index, _| match index {
            1 => {
                day::prefs::set(crate::THEME_KEY, "light");
                day::set_appearance(Some(false));
            }
            2 => {
                day::prefs::set(crate::THEME_KEY, "dark");
                day::set_appearance(Some(true));
            }
            _ => {
                day::prefs::remove(crate::THEME_KEY);
                day::set_appearance(None);
            }
        },
    );
    labeled(
        res::str::settings_appearance(),
        picker(
            [
                res::str::appearance_system().format(),
                res::str::appearance_light().format(),
                res::str::appearance_dark().format(),
            ],
            choice,
        )
        .segmented()
        .id("theme-picker"),
    )
    .any()
}

/// The system language, then every locale under `resource/locales/` by its self-name.
fn language_row() -> AnyPiece {
    let stored = day::prefs::get(crate::LOCALE_KEY).unwrap_or_default();
    let start = res::locales::ALL
        .iter()
        .position(|(tag, _)| *tag == stored)
        .map(|i| i + 1)
        .unwrap_or(0);
    let choice = Signal::new(start);
    watch(
        move || choice.get(),
        move |index, _| match index.checked_sub(1).and_then(|i| res::locales::ALL.get(i)) {
            Some((tag, _)) => {
                day::prefs::set(crate::LOCALE_KEY, tag);
                set_locale(tag);
            }
            None => {
                day::prefs::remove(crate::LOCALE_KEY);
                set_locale(res::locales::DEFAULT);
            }
        },
    );
    let mut names = vec![res::str::appearance_system().format()];
    names.extend(res::locales::ALL.iter().map(|(_, name)| (*name).to_string()));
    labeled(
        res::str::settings_language(),
        picker(names, choice).id("language-picker"),
    )
    .any()
}

/// The same body as a section, for the platforms with no menu bar (see `has_menu_bar`).
pub(crate) fn settings_page() -> impl Piece {
    column((
        label(res::str::nav_settings())
            .font(Font::Title)
            .id("settings-title"),
        settings_body(),
    ))
    .spacing(12.0)
    .align(HAlign::Leading)
    .padding(16.0)
}
