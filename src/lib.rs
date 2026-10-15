// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Games Fair is a [Day](https://daybrite.dev) app of small arcade/puzzle games, one crate each.
//! `root()` is the whole UI, shared by every platform: the phones, the desktops, and the web.
//!
//! The home screen is a wrapping grid of game tiles stretched to the window's width, whose
//! previews are drawn by each game's crate with the same rendering code as gameplay. Tapping a
//! tile presents the game in a fullscreen `cover` (https://daybrite.dev/docs/cover) with an X in
//! the top-leading corner to exit; the games defer the
//! system's edge gestures and disable interactive dismissal so an edge swipe mid-game doesn't
//! leave the game. Each game saves its state through `gamekit` when the cover closes or the
//! app is backgrounded, and restores it the next time it opens.
//!
//! Settings and About are the shared surface (https://daybrite.dev/docs/settings): Appearance
//! and Language, the Sounds and Vibrations a game starts with, a reset for every game's records,
//! and the bundled fonts' licenses. A desktop reaches it from the app menu, the web from its menu
//! button, and the phones from the gear above the grid.

use std::f64::consts::TAU;

use day::prelude::*;
use day_piece_settings::prelude::*;

// Resolved from the app metadata, including flavor and target overrides when built with Day.
include!(concat!(env!("OUT_DIR"), "/app_title.rs"));
/// The bundled fonts' license texts, one after another (build.rs gathers them from
/// resource/font-licenses/), for the About window's Licenses page.
const FONT_LICENSES: &str = include_str!(concat!(env!("OUT_DIR"), "/font_licenses.txt"));
/// The project's website and source repository, linked from About.
const WEBSITE: &str = "https://games-fair.github.io/Games-Fair/";
const SOURCE: &str = "https://github.com/Games-Fair/Games-Fair";

// Entry point for the mobile, macOS, and web hosts; a desktop binary enters through src/main.rs.
day::day_start!(options: window(), root);

/// Options for every window. The locale catalog and title go to `launch`, which installs them
/// (https://daybrite.dev/docs/localization).
pub fn window() -> day::WindowOptions {
    day::WindowOptions {
        locales: Some((res::locales::DEFAULT, res::locales::CATALOG)),
        title_fn: Some(|| res::str::app_title(APP_TITLE).format()),
        // Desktop only; phones fill the screen. Tall enough for the Sudoku board and keypad.
        size: day::prelude::Size::new(720.0, 780.0),
        // What About shows when the app runs outside `day build`, which exports the manifest's.
        version: Some(env!("CARGO_PKG_VERSION").into()),
        ..Default::default()
    }
}

// Typed names for everything under `resource/` (https://daybrite.dev/docs/resources).
day::resources!();

day::routes! {
    /// The app's games, typed: each variant's key is what deep links, dayscript, and
    /// `current_route()` speak.
    pub(crate) enum Section {
        BlockBlast => "blockblast",
        Breakout => "breakout",
        Charades => "charades",
        MatchThree => "matchthree",
        Mines => "mines",
        PacPac => "pacpac",
        Flipside => "flipside",
        Pipes => "pipes",
        Sirtet => "sirtet",
        Solitaire => "solitaire",
        Sudoku => "sudoku",
        Game2048 => "twentyfortyeight",
    }
}

/// Home-screen palette: a deep night background with translucent tile cards.
const HOME_BG: Color = Color::hex(0x10_10_24);
/// The narrowest a tile's preview gets. The home grid stretches its columns across the window,
/// so previews grow from here; this floor still fits two tiles to a line on a 360pt phone.
const TILE_MIN: f64 = 126.0;

/// Each game's surface color, painted edge-to-edge behind the presented cover.
fn game_background(section: Section) -> Color {
    match section {
        Section::BlockBlast => blockblast::SURFACE,
        Section::Breakout => breakout::SURFACE,
        Section::Charades => charades::SURFACE,
        Section::MatchThree => matchthree::SURFACE,
        Section::Mines => mines::SURFACE,
        Section::PacPac => pacpac::SURFACE,
        Section::Flipside => flipside::SURFACE,
        Section::Pipes => pipes::SURFACE,
        Section::Sirtet => sirtet::SURFACE,
        Section::Solitaire => solitaire::SURFACE,
        Section::Sudoku => sudoku::SURFACE,
        Section::Game2048 => twentyfortyeight::SURFACE,
    }
}

pub fn root() -> impl Piece {
    // `day launch --env DAY_GAMES_SEED=<n>` pins every game's RNG, so a dayscript walkthrough
    // meets the same puzzle and layout on every run (dayscript/sudoku.yaml taps cells it knows
    // are empty for seed 15). Unset, each game seeds itself from the clock.
    if let Some(seed) = day::env("DAY_GAMES_SEED").and_then(|s| s.trim().parse::<u64>().ok()) {
        gamekit::set_seed_override(seed);
    }
    let prefs = preferences();
    let open = Signal::new(None::<Section>);
    zstack((home_page(open, prefs.in_nav()), game_cover(open)))
}

/// Settings and About (https://daybrite.dev/docs/settings). On a desktop `install()` adds the
/// app menu's Settings… and About items and their windows. The web's menu button carries the
/// same items, and its Settings… presents the groups as a cover. On a phone and HarmonyOS, which
/// have no app menu, the home grid's gear ([`settings_button`]) presents that cover.
///
/// Sounds and Vibrations here are the app-wide defaults: what a game's own switches start at
/// the first time it opens. A game that has saved its settings keeps them, so the switch in its
/// pause menu still sets that one game apart.
fn preferences() -> Prefs {
    Preferences::new()
        .about(
            About::from_manifest()
                .link(day::tr("day-about-website"), WEBSITE)
                .link(day::tr("day-about-source"), SOURCE)
                .licenses(FONT_LICENSES),
        )
        .group(general_group(res::locales::ALL))
        .group(
            Group::new("sound", res::str::settings_sound())
                .icon(res::vectors::prefs_sound)
                .section(
                    day_piece_settings::Section::new()
                        .row(rows::toggle(
                            &gamekit::chrome::SOUNDS,
                            res::str::gk_sounds(),
                        ))
                        .row(rows::toggle(
                            &gamekit::chrome::VIBRATIONS,
                            res::str::gk_vibrations(),
                        ))
                        .footer(res::str::settings_sound_footer()),
                ),
        )
        .group(
            Group::new("records", res::str::settings_records())
                .icon(res::vectors::prefs_records)
                .row(
                    rows::action(res::str::settings_reset_records(), reset_all_records)
                        .caption(res::str::settings_reset_records_caption())
                        .destructive()
                        .confirm(res::str::settings_reset_records_confirm())
                        .id("reset-records"),
                ),
        )
        .install()
}

/// Every game's records (best scores, best times, games won) wiped at once: each game clears
/// its own keys, and one that is open on a desktop resets the copy it holds in memory too
/// (`gamekit::on_reset_records`), so its next save keeps the reset. Flipside keeps no records,
/// and Match Three's stars are its level progress, which stays.
pub fn reset_all_records() {
    blockblast::reset_records();
    breakout::reset_records();
    charades::reset_records();
    mines::reset_records();
    pacpac::reset_records();
    pipes::reset_records();
    sirtet::reset_records();
    solitaire::reset_records();
    sudoku::reset_records();
    twentyfortyeight::reset_records();
}

/// The gear that opens Settings where the platform gives no app menu to reach it from (the
/// phones and HarmonyOS; `Prefs::in_nav`).
/// Drawn like the games' pause button: a stroke glyph in the home page's ink, in a 44-point
/// touch target.
fn settings_button() -> AnyPiece {
    canvas(|d, sz| {
        let c = Point::new(sz.width / 2.0, sz.height / 2.0);
        let ink = Color::rgba(1.0, 1.0, 1.0, 0.8);
        let style = gamekit::chrome::stroke_style(22.0);
        // Eight teeth around a rim, and the hub.
        let (rim, tip, hub) = (7.5, 11.0, 2.5);
        for i in 0..8 {
            let (sin, cos) = (f64::from(i) * TAU / 8.0).sin_cos();
            d.stroke_styled(
                Shape::Line(
                    Point::new(c.x + cos * rim, c.y + sin * rim),
                    Point::new(c.x + cos * tip, c.y + sin * tip),
                ),
                ink,
                style.clone(),
            );
        }
        d.stroke_styled(
            Shape::Ellipse(Rect::new(c.x - rim, c.y - rim, 2.0 * rim, 2.0 * rim)),
            ink,
            style.clone(),
        );
        d.stroke_styled(
            Shape::Ellipse(Rect::new(c.x - hub, c.y - hub, 2.0 * hub, 2.0 * hub)),
            ink,
            style,
        );
    })
    .on_tap(|| {
        day_piece_settings::open();
    })
    .a11y(|a| a.label(day::tr("day-settings").format()).role(Role::Button))
    .id("home-settings")
    .frame(gamekit::chrome::BUTTON, gamekit::chrome::BUTTON)
    .any()
}

/// One home tile: the game's own preview over a translucent card; tapping presents the game.
fn tile(
    open: Signal<Option<Section>>,
    section: Section,
    title: LocalizedText,
    preview: AnyPiece,
    id: &'static str,
) -> impl Piece + use<> {
    let a11y_title = title.format();
    column((
        // Square at whatever width the tile's column gets.
        preview.grow_w().aspect_ratio(1.0).corner_radius(16.0),
        label(title)
            .weight(FontWeight::Semibold)
            .color(Color::WHITE),
    ))
    .spacing(10.0)
    .min_width(TILE_MIN)
    .padding(12.0)
    .background(Color::rgba(1.0, 1.0, 1.0, 0.08))
    .corner_radius(20.0)
    .on_tap(move || open.set(Some(section)))
    .a11y(move |a| a.label(a11y_title))
    .id(id)
    // Last, so the home grid sees a growing cell and stretches its columns to the width.
    .grow_w()
}

fn home_page(open: Signal<Option<Section>>, settings_in_chrome: bool) -> impl Piece {
    // The title, and the Settings gear beside it where no menu bar offers the window.
    let mut header: Vec<AnyPiece> = vec![
        label(res::str::app_title(APP_TITLE))
            .font(Font::Title)
            .bold()
            .color(Color::WHITE)
            .id("app-title")
            .grow_w()
            .any(),
    ];
    if settings_in_chrome {
        header.push(settings_button());
    }
    scroll(
        column((
            row(PieceVec(header)).align(VAlign::Center).grow_w(),
            // As many columns as the narrowest tile allows, stretched to the window's width and
            // re-flowed as it resizes, so a new game is one more tile and never a layout change.
            row((
                tile(
                    open,
                    Section::Breakout,
                    res::str::nav_breakout(),
                    breakout::breakout_preview(),
                    "tile-breakout",
                ),
                tile(
                    open,
                    Section::Sirtet,
                    res::str::nav_sirtet(),
                    sirtet::sirtet_preview(),
                    "tile-sirtet",
                ),
                tile(
                    open,
                    Section::Game2048,
                    res::str::nav_2048(),
                    twentyfortyeight::twentyfortyeight_preview(),
                    "tile-twentyfortyeight",
                ),
                tile(
                    open,
                    Section::Sudoku,
                    res::str::nav_sudoku(),
                    sudoku::sudoku_preview(),
                    "tile-sudoku",
                ),
                tile(
                    open,
                    Section::BlockBlast,
                    res::str::nav_blockblast(),
                    blockblast::blockblast_preview(),
                    "tile-blockblast",
                ),
                tile(
                    open,
                    Section::Solitaire,
                    res::str::nav_solitaire(),
                    solitaire::solitaire_preview(),
                    "tile-solitaire",
                ),
                tile(
                    open,
                    Section::Charades,
                    res::str::nav_charades(),
                    charades::charades_preview(),
                    "tile-charades",
                ),
                tile(
                    open,
                    Section::Pipes,
                    res::str::nav_pipes(),
                    pipes::pipes_preview(),
                    "tile-pipes",
                ),
                tile(
                    open,
                    Section::Flipside,
                    res::str::nav_flipside(),
                    flipside::flipside_preview(),
                    "tile-flipside",
                ),
                tile(
                    open,
                    Section::Mines,
                    res::str::nav_mines(),
                    mines::mines_preview(),
                    "tile-mines",
                ),
                tile(
                    open,
                    Section::MatchThree,
                    res::str::nav_matchthree(),
                    matchthree::matchthree_preview(),
                    "tile-matchthree",
                ),
                tile(
                    open,
                    Section::PacPac,
                    res::str::nav_pacpac(),
                    pacpac::pacpac_preview(),
                    "tile-pacpac",
                ),
            ))
            .spacing(16.0)
            .fit(RowFit::WrapColumns { run_spacing: 16.0 }),
        ))
        .spacing(20.0)
        .align(HAlign::Leading)
        .padding(20.0),
    )
    .grow()
    .background(HOME_BG)
    .id("home")
}

/// The fullscreen game surface: the game page shielded from system edge gestures and
/// interactive dismissal, with a small X (top leading) as the one way out.
fn game_cover(open: Signal<Option<Section>>) -> impl Piece {
    cover(open, move |section: &Section| {
        let section = *section;
        // The header draws in ink that contrasts with the game's surface: white on the dark
        // boards, near-black on 2048's cream one. Set before the page is built, because a
        // title's ink is resolved as its piece is made, not when it draws.
        gamekit::chrome::set_surface(game_background(section));
        let game = match section {
            Section::BlockBlast => blockblast::blockblast_page(),
            Section::Breakout => breakout::breakout_page(),
            Section::Charades => charades::charades_page(),
            Section::MatchThree => matchthree::matchthree_page(),
            Section::Mines => mines::mines_page(),
            Section::PacPac => pacpac::pacpac_page(),
            Section::Flipside => flipside::flipside_page(),
            Section::Pipes => pipes::pipes_page(),
            Section::Sirtet => sirtet::sirtet_page(),
            Section::Solitaire => solitaire::solitaire_page(),
            Section::Sudoku => sudoku::sudoku_page(),
            Section::Game2048 => twentyfortyeight::twentyfortyeight_page(),
        };
        // Leaving is the shell's to do, and the button that asks for it sits in the game's own
        // header row (gamekit::chrome::game_header), where it lines up with the title and the
        // pause button. Registered per open cover, and cleared with it.
        gamekit::on_close(move || open.set(None));
        game.grow()
            .defers_system_gestures(Edges::ALL)
            .interactive_dismiss_disabled()
            .any()
    })
    .background(|section| game_background(*section))
}
