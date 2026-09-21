// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Games Fair, an App Fair app built with [Day](https://daybrite.dev). `root()` runs once and
//! opens the first window; each section lives under `pages/`.

use day::prelude::*;

mod pages;
use crate::pages::*;

// Entry point for the mobile hosts; a desktop build enters through src/main.rs.
day::day_start!(options: window(), root);

/// Options for every window. The catalog and title go to `launch`, which installs them
/// (https://daybrite.dev/docs/localization).
pub fn window() -> day::WindowOptions {
    day::WindowOptions {
        locales: Some((res::locales::DEFAULT, res::locales::CATALOG)),
        title_fn: Some(|| res::str::app_title().format()),
        // Desktop only; phones fill the screen.
        size: day::prelude::Size::new(960.0, 640.0),
        ..Default::default()
    }
}

// Typed names for everything under `resource/` (https://daybrite.dev/docs/resources).
day::resources!();

/// The `day::prefs` keys the Settings page writes and startup reads.
pub(crate) const THEME_KEY: &str = "app.theme";
pub(crate) const LOCALE_KEY: &str = "app.locale";

day::routes! {
    /// The app's sections, as typed routes (https://daybrite.dev/docs/navigation).
    pub(crate) enum Section {
        Welcome => "welcome",
        Navigate => "navigate",
        Settings => "settings",
    }
}

/// True where there is a menu bar, so Settings lives in the App menu instead of the nav.
pub(crate) fn has_menu_bar() -> bool {
    capability(Cap::AppMenu) != Support::Unsupported
}

/// One-time app setup, then the first window's content.
pub fn root() -> impl Piece {
    // Day installs a logger at launch, so `info!` works as is.
    info!("Games Fair starting");
    // Reapply the saved appearance and language before anything is built.
    pages::apply_startup();

    // A Settings window and App ▸ Settings… on desktop; a fullscreen cover elsewhere.
    day::register_preferences(settings_body);
    // File ▸ New Window builds the same shell again.
    day::register_new_window(|| window_shell(false));
    app_menu(menus());

    window_shell(true)
}

/// One window's UI, for the first window and every File ▸ New Window.
///
/// Tabs on a phone, a rail on a tablet, a sidebar on a desktop
/// (https://daybrite.dev/docs/navigation).
fn window_shell(primary: bool) -> impl Piece {
    let section = Signal::new(Section::Welcome);
    let nav = nav(section)
        .title(res::str::app_title())
        .item_icon(
            Section::Welcome,
            res::str::nav_welcome(),
            res::vectors::tab_welcome,
            welcome_page,
        )
        // One tint per section, so the icons read apart.
        .icon_tint(Color::hex(0xF59E0B))
        .item_icon(
            Section::Navigate,
            res::str::nav_navigate(),
            res::vectors::tab_navigate,
            navigate_page,
        )
        .icon_tint(Color::hex(0x3B82F6))
        // Settings is a nav row only where there is no menu bar.
        .items(
            move || {
                if has_menu_bar() {
                    Vec::new()
                } else {
                    vec![Section::Settings]
                }
            },
            |s: &Section| {
                item(*s, res::str::nav_settings())
                    .icon(res::vectors::tab_settings)
                    .icon_tint(Color::hex(0x10B981))
            },
        )
        .destination(|_: &Section| settings_page())
        .id("nav");
    // Only the first window joins the route namespace: two routed navs make a deep link
    // ambiguous.
    if primary {
        nav.restore("app.section")
    } else {
        nav.local()
    }
}

/// The desktop menu bar; the mobile toolkits ignore it (https://daybrite.dev/docs/guide-desktop).
fn menus() -> Vec<MenuEntry> {
    vec![sub_menu(
        res::str::menu_file().format(),
        vec![
            // The platform's New Window item and ⌘N.
            menu_role(MenuRole::NewWindow),
            menu_separator(),
            menu_item(res::str::cmd_appfair().format())
                .action(|| open_link("https://appfair.org")),
            menu_separator(),
            menu_role(MenuRole::CloseWindow),
        ],
    )]
}
