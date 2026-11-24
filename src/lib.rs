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
//! Settings and About are the shared surface (https://daybrite.dev/docs/settings): Language, the Sounds and Vibrations a game starts with, a reset for every game's records,
//! and the bundled fonts' licenses. A desktop reaches it from the app menu, the web from its menu
//! button, and the phones from the gear above the grid.

use std::f64::consts::TAU;

mod orientation;

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

/// Options for every window. The generated options carry the locale catalog, which `launch`
/// installs (https://daybrite.dev/docs/localization), and the version About shows when the app
/// runs outside `day build`; the title is the app metadata's, which flavor and target overrides
/// reach.
pub fn window() -> day::WindowOptions {
    res::window_options()
        .title_fn(|| res::str::app_title(APP_TITLE).format())
        // Desktop only; phones fill the screen. Tall enough for the Sudoku board and keypad.
        .size(720.0, 780.0)
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
        Flipside => "flipside",
        Pipes => "pipes",
        TicTacToe => "tictactoe",
        Memory => "memory",
        Moles => "moles",
        Snakes => "snakes",
        PaintFight => "paintfight",
        Pool => "pool",
        ButtonSoccer => "buttonsoccer",
        DotsBoxes => "dotsboxes",
        Impostor => "impostor",
        MiniGolf => "minigolf",
        FruitDuel => "fruitduel",
        FruitMerge => "fruitmerge",
        Basketball => "basketball",
        Sirtet => "sirtet",
        Solitaire => "solitaire",
        Sudoku => "sudoku",
        Game2048 => "twentyfortyeight",
    }
}

fn mobile_profile() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        capability(day_spec::Cap::ScrollKeyboardDismiss) != day_spec::Support::Unsupported
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        cfg!(any(
            target_os = "ios",
            target_os = "android",
            target_env = "ohos"
        ))
    }
}

/// Covers parse incoming URLs and navigation requests independently of tile taps.
#[derive(Clone, Copy, PartialEq)]
struct GameRoute(Section);
impl Route for GameRoute {
    fn key(&self) -> String {
        self.0.key()
    }
    fn from_key(key: &str) -> Option<Self> {
        let game = Section::from_key(key)?;
        available(game, mobile_profile()).then_some(Self(game))
    }
}
fn available(game: Section, mobile: bool) -> bool {
    mobile
        || !matches!(
            game,
            Section::Charades
                | Section::Impostor
                | Section::FruitDuel
                | Section::PaintFight
                | Section::Moles
                | Section::Snakes
                | Section::Basketball
        )
}

/// Home-screen palette: a deep night background with translucent tile cards.
const HOME_BG: Color = Color::hex(0x10_10_24);
/// The narrowest a tile's preview gets. The home grid stretches its columns across the window,
/// so previews grow from here. Leave room for two padded tiles on a 320pt Display Zoom phone.
const TILE_MIN: f64 = 104.0;

/// Each game's surface color, painted edge-to-edge behind the presented cover.
fn game_background(section: Section) -> Color {
    match section {
        Section::BlockBlast => blockblast::SURFACE,
        Section::Breakout => breakout::SURFACE,
        Section::Charades => charades::SURFACE,
        Section::MatchThree => matchthree::SURFACE,
        Section::Mines => mines::SURFACE,
        Section::Flipside => flipside::SURFACE,
        Section::Pipes => pipes::SURFACE,
        Section::TicTacToe => tictactoe::SURFACE,
        Section::Memory => memory::SURFACE,
        Section::Moles => moles::SURFACE,
        Section::Snakes => snakes::SURFACE,
        Section::PaintFight => paintfight::SURFACE,
        Section::Pool => pool::SURFACE,
        Section::ButtonSoccer => buttonsoccer::SURFACE,
        Section::DotsBoxes => dotsboxes::SURFACE,
        Section::Impostor => impostor::SURFACE,
        Section::MiniGolf => minigolf::SURFACE,
        Section::FruitDuel => fruitduel::SURFACE,
        Section::FruitMerge => fruitmerge::SURFACE,
        Section::Basketball => basketball::SURFACE,
        Section::Sirtet => sirtet::SURFACE,
        Section::Solitaire => solitaire::SURFACE,
        Section::Sudoku => sudoku::SURFACE,
        Section::Game2048 => twentyfortyeight::SURFACE,
    }
}

pub fn root() -> impl Piece {
    // These retired formats embedded level or deck data in the round itself.
    for key in [
        "save.buttonsoccer.v1",
        "save.fruitduel.v1",
        "save.fruitmerge.v1",
        "save.impostor.v1",
        "save.minigolf.v1",
    ] {
        if day::prefs::contains(key) {
            day::prefs::remove(key);
        }
    }

    // `day launch --env DAY_GAMES_SEED=<n>` pins every game's RNG, so a dayscript walkthrough
    // meets the same puzzle and layout on every run (dayscript/sudoku.yaml taps cells it knows
    // are empty for seed 15). Unset, each game seeds itself from the clock.
    if let Some(seed) = day::env("DAY_GAMES_SEED").and_then(|s| s.trim().parse::<u64>().ok()) {
        gamekit::set_seed_override(seed);
    }
    let prefs = preferences();
    // Game surfaces are deliberately dark, so native controls must use the same appearance.
    APPEARANCE.set(Appearance::Dark);
    day::set_appearance(Some(true));
    let open = Signal::new(None::<GameRoute>);
    let mobile = Signal::new(mobile_profile());
    #[cfg(target_arch = "wasm32")]
    day::on_lifecycle(Lifecycle::DidLaunch, move || mobile.set(mobile_profile()));
    let players = Signal::new(1usize);
    zstack((
        home_page(open, prefs.in_nav(), mobile, players),
        game_cover(open, players, mobile),
    ))
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
        .group(
            Group::new("general", res::str::settings_general())
                .icon(Symbol::Settings)
                .row(language_row(res::locales::ALL)),
        )
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
                        .id(id!(reset_records)),
                ),
        )
        .install()
}

/// Every game's records (best scores, best times, games won) wiped at once: each game clears
/// its own keys, and one that is open on a desktop resets the copy it holds in memory too
/// (`gamekit::on_reset_records`), so its next save keeps the reset. Flipside keeps no records,
/// and Match Three's stars are its level progress, which stays.
pub fn reset_all_records() {
    fruitmerge::reset_records();
    blockblast::reset_records();
    breakout::reset_records();
    charades::reset_records();
    mines::reset_records();
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
    .a11y(|a| a.label(day::tr("day-settings")).role(Role::Button))
    .id(id!(home_settings))
    .frame(gamekit::chrome::BUTTON, gamekit::chrome::BUTTON)
    .any()
}

/// Give the wrapping grid a compact preferred width, while measuring titles at the
/// actual column width once it is known. Long/localized titles wrap instead of making
/// every column as wide as their unwrapped text.
struct TileWidth;
impl day_core::Layout for TileWidth {
    fn measure(
        &self,
        cx: &mut dyn day_core::LayoutOps,
        children: &[day_core::RNode],
        p: day_spec::Proposal,
    ) -> Size {
        let width = p.width.unwrap_or(TILE_MIN);
        children.first().map_or(Size::new(width, 0.0), |&child| {
            let size = cx.measure_child(child, day_spec::Proposal::new(Some(width), p.height));
            Size::new(width, size.height)
        })
    }
    fn place(&self, cx: &mut dyn day_core::LayoutOps, children: &[day_core::RNode], bounds: Rect) {
        if let Some(&child) = children.first() {
            cx.place_child(child, Rect::from_size(bounds.size));
        }
    }
}

/// One home tile: the game's own preview over a translucent card; tapping presents the game.
fn tile(
    open: Signal<Option<GameRoute>>,
    section: Section,
    title: LocalizedText,
    preview: AnyPiece,
    id: ElementId,
) -> impl Piece + use<> {
    let a11y_title = title.clone();
    let content = column((
        // Square at whatever width the tile's column gets.
        preview
            .grow(Dimension::Width)
            .aspect_ratio(1.0)
            .corner_radius(16.0),
        label(title)
            .weight(FontWeight::Semibold)
            .color(Color::WHITE),
    ))
    .spacing(10.0);
    piece_fn(move |cx| {
        let node = cx.native(
            day_spec::kinds::CONTAINER,
            &day_spec::props::ContainerProps::default(),
            std::rc::Rc::new(TileWidth),
            day_core::Flex::default(),
            day_core::Boundary::No,
        );
        cx.under(node, |cx| content.build(cx));
        node
    })
    .padding(Space::M)
    .background(Color::rgba(1.0, 1.0, 1.0, 0.08))
    .corner_radius(20.0)
    .on_tap(move || open.set(Some(GameRoute(section))))
    .a11y(move |a| a.label(a11y_title))
    .id(id)
    // Last, so the home grid sees a growing cell and stretches its columns to the width.
    .grow(Dimension::Width)
}

/// Human player counts supported by each game. Computer opponents do not count as humans.
fn supports_players(section: Section, players: usize, mobile: bool) -> bool {
    if !available(section, mobile) || !(1..=4).contains(&players) || (!mobile && players != 1) {
        return false;
    }
    match section {
        Section::Impostor => players >= 3,
        Section::Charades => players >= 2,
        Section::TicTacToe | Section::Pool | Section::Flipside => players <= 2,
        Section::Memory
        | Section::Moles
        | Section::Snakes
        | Section::PaintFight
        | Section::Basketball
        | Section::ButtonSoccer
        | Section::DotsBoxes
        | Section::MiniGolf
        | Section::FruitDuel => true,
        _ => players == 1,
    }
}
const CATALOG: [Section; 24] = [
    Section::Breakout,
    Section::Sirtet,
    Section::Game2048,
    Section::Sudoku,
    Section::BlockBlast,
    Section::Solitaire,
    Section::Pipes,
    Section::Flipside,
    Section::Mines,
    Section::MatchThree,
    Section::Charades,
    Section::TicTacToe,
    Section::Memory,
    Section::Moles,
    Section::Snakes,
    Section::PaintFight,
    Section::Pool,
    Section::ButtonSoccer,
    Section::DotsBoxes,
    Section::Impostor,
    Section::MiniGolf,
    Section::FruitMerge,
    Section::FruitDuel,
    Section::Basketball,
];
fn catalog_tile(open: Signal<Option<GameRoute>>, section: Section) -> AnyPiece {
    match section {
        Section::Breakout => tile(
            open,
            section,
            res::str::nav_breakout(),
            breakout::breakout_preview(),
            id!(tile_breakout),
        )
        .any(),
        Section::Sirtet => tile(
            open,
            section,
            res::str::nav_sirtet(),
            sirtet::sirtet_preview(),
            id!(tile_sirtet),
        )
        .any(),
        Section::Game2048 => tile(
            open,
            section,
            res::str::nav_2048(),
            twentyfortyeight::twentyfortyeight_preview(),
            id!(tile_twentyfortyeight),
        )
        .any(),
        Section::Sudoku => tile(
            open,
            section,
            res::str::nav_sudoku(),
            sudoku::sudoku_preview(),
            id!(tile_sudoku),
        )
        .any(),
        Section::BlockBlast => tile(
            open,
            section,
            res::str::nav_blockblast(),
            blockblast::blockblast_preview(),
            id!(tile_blockblast),
        )
        .any(),
        Section::Solitaire => tile(
            open,
            section,
            res::str::nav_solitaire(),
            solitaire::solitaire_preview(),
            id!(tile_solitaire),
        )
        .any(),
        Section::Pipes => tile(
            open,
            section,
            res::str::nav_pipes(),
            pipes::pipes_preview(),
            id!(tile_pipes),
        )
        .any(),
        Section::Flipside => tile(
            open,
            section,
            res::str::nav_flipside(),
            flipside::flipside_preview(),
            id!(tile_flipside),
        )
        .any(),
        Section::Mines => tile(
            open,
            section,
            res::str::nav_mines(),
            mines::mines_preview(),
            id!(tile_mines),
        )
        .any(),
        Section::MatchThree => tile(
            open,
            section,
            res::str::nav_matchthree(),
            matchthree::matchthree_preview(),
            id!(tile_matchthree),
        )
        .any(),
        Section::Charades => tile(
            open,
            section,
            res::str::nav_charades(),
            charades::charades_preview(),
            id!(tile_charades),
        )
        .any(),
        Section::TicTacToe => tile(
            open,
            section,
            res::str::nav_tictactoe(),
            tictactoe::tictactoe_preview(),
            id!(tile_tictactoe),
        )
        .any(),
        Section::Memory => tile(
            open,
            section,
            res::str::nav_memory(),
            memory::memory_preview(),
            id!(tile_memory),
        )
        .any(),
        Section::Moles => tile(
            open,
            section,
            res::str::nav_moles(),
            moles::moles_preview(),
            id!(tile_moles),
        )
        .any(),
        Section::Snakes => tile(
            open,
            section,
            res::str::nav_snakes(),
            snakes::snakes_preview(),
            id!(tile_snakes),
        )
        .any(),
        Section::PaintFight => tile(
            open,
            section,
            res::str::nav_paintfight(),
            paintfight::paintfight_preview(),
            id!(tile_paintfight),
        )
        .any(),
        Section::ButtonSoccer => tile(
            open,
            section,
            res::str::nav_buttonsoccer(),
            buttonsoccer::buttonsoccer_preview(),
            id!(tile_buttonsoccer),
        )
        .any(),
        Section::DotsBoxes => tile(
            open,
            section,
            res::str::nav_dotsboxes(),
            dotsboxes::dotsboxes_preview(),
            id!(tile_dotsboxes),
        )
        .any(),
        Section::Impostor => tile(
            open,
            section,
            res::str::nav_impostor(),
            impostor::impostor_preview(),
            id!(tile_impostor),
        )
        .any(),
        Section::MiniGolf => tile(
            open,
            section,
            res::str::nav_minigolf(),
            minigolf::minigolf_preview(),
            id!(tile_minigolf),
        )
        .any(),
        Section::FruitDuel => tile(
            open,
            section,
            res::str::nav_fruitduel(),
            fruitduel::fruitduel_preview(),
            id!(tile_fruitduel),
        )
        .any(),
        Section::FruitMerge => tile(
            open,
            section,
            res::str::nav_fruitmerge(),
            fruitmerge::fruitmerge_preview(),
            id!(tile_fruitmerge),
        )
        .any(),
        Section::Basketball => tile(
            open,
            section,
            res::str::nav_basketball(),
            basketball::basketball_preview(),
            id!(tile_basketball),
        )
        .any(),
        Section::Pool => tile(
            open,
            section,
            res::str::nav_pool(),
            pool::pool_preview(),
            id!(tile_pool),
        )
        .any(),
    }
}
fn home_page(
    open: Signal<Option<GameRoute>>,
    settings_in_chrome: bool,
    mobile: Signal<bool>,
    players: Signal<usize>,
) -> impl Piece {
    let visibility = Signal::new(CATALOG.map(|s| {
        f64::from(supports_players(
            s,
            if mobile.get_untracked() {
                players.get_untracked()
            } else {
                1
            },
            mobile.get_untracked(),
        ))
    }));
    // Keep outgoing tiles mounted until their fade finishes; incoming tiles fade and scale in.
    // One frame clock is shared by the catalog and stops when every tile has settled.
    let animation = gamekit::animation::clock(
        move || {
            let values = visibility.get();
            CATALOG.iter().enumerate().any(|(i, &s)| {
                values[i]
                    != f64::from(supports_players(
                        s,
                        if mobile.get() { players.get() } else { 1 },
                        mobile.get(),
                    ))
            })
        },
        move |dt| {
            let mut values = visibility.get_untracked();
            for (i, &s) in CATALOG.iter().enumerate() {
                let goal = f64::from(supports_players(
                    s,
                    if mobile.get_untracked() {
                        players.get_untracked()
                    } else {
                        1
                    },
                    mobile.get_untracked(),
                ));
                values[i] = if day_core::reduce_motion() {
                    goal
                } else {
                    values[i]
                        + (goal - values[i])
                            .clamp(-dt.as_secs_f64() / 0.20, dt.as_secs_f64() / 0.20)
                };
            }
            visibility.set(values);
        },
    );
    let picker = move || {
        row((
            // The caption and segments share the same font metrics and canvas text anchor.
            gamekit::chrome::fitted_title(
                res::str::home_players_label(),
                16.0,
                FontWeight::Bold,
                Color::WHITE,
            )
            .width(|| {
                day_core::measure_text(
                    &res::str::home_players_label().format(),
                    16.0,
                    &gamekit::chrome::canvas_font(FontWeight::Bold),
                )
                .width
                .ceil()
            }),
            row((1..=4)
                .map(move |count| {
                    gamekit::chrome::fitted_title(
                        res::str::party_count(count as f64),
                        16.0,
                        FontWeight::Bold,
                        Color::WHITE,
                    )
                    .padding(10.0)
                    .min_height(44.0)
                    .grow(Dimension::Width)
                    .surface(move || {
                        Surface::new()
                            .fill(if players.get() == count {
                                Color::hex(0x46465F)
                            } else {
                                Color::hex(0x242436)
                            })
                            .corner_radius(9.0)
                    })
                    .on_tap(move || players.set(count))
                    .on_key(move |key| {
                        if matches!(key.key, day::Key::Enter | day::Key::Character(' ')) {
                            players.set(count);
                        }
                    })
                    .a11y(move |a| {
                        a.label(res::str::home_player_count(count as f64))
                            .role(Role::Button)
                            .selected(move || players.get() == count)
                    })
                    .id(match count {
                        1 => id!(home_players_1),
                        2 => id!(home_players_2),
                        3 => id!(home_players_3),
                        _ => id!(home_players_4),
                    })
                    .grow(Dimension::Width)
                    .any()
                })
                .collect::<Vec<_>>())
            .spacing(4.0)
            .grow(Dimension::Width),
        ))
        .align(VAlign::Center)
        .spacing(8.0)
        .padding(4.0)
        .surface(
            Surface::new()
                .fill(Color::hex(0x242436))
                .corner_radius(12.0),
        )
        .id(id!(home_player_filter))
        .grow(Dimension::Width)
    };
    let header = when(
        move || mobile.get() || settings_in_chrome,
        move || {
            row((
                when(move || mobile.get(), picker).grow(Dimension::Width),
                settings_in_chrome.then(settings_button),
            ))
            .spacing(6.0)
            .align(VAlign::Center)
            .grow(Dimension::Width)
        },
    );
    let tiles = row(CATALOG
        .into_iter()
        .enumerate()
        .map(move |(i, section)| {
            when(
                move || {
                    visibility.get()[i] > 0.0
                        || supports_players(
                            section,
                            if mobile.get() { players.get() } else { 1 },
                            mobile.get(),
                        )
                },
                move || {
                    catalog_tile(open, section)
                        .enabled(move || {
                            supports_players(
                                section,
                                if mobile.get() { players.get() } else { 1 },
                                mobile.get(),
                            )
                        })
                        .opacity(move || visibility.get()[i])
                        .scale(move || 0.94 + 0.06 * visibility.get()[i])
                        .grow(Dimension::Width)
                        .any()
                },
            )
            .any()
        })
        .collect::<Vec<_>>())
    .spacing(Space::L)
    .fit(RowFit::WrapColumns { run_spacing: 16.0 });
    zstack((
        scroll(
            column((header, tiles))
                .spacing(20.0)
                .align(HAlign::Leading)
                .padding(20.0),
        )
        .grow(Dimension::Both),
        animation,
    ))
    .grow(Dimension::Both)
    .background(HOME_BG)
    .id(id!(home))
}

/// The fullscreen game surface: the game page shielded from system edge gestures and
/// interactive dismissal, with a small X (top leading) as the one way out.
fn game_cover(
    open: Signal<Option<GameRoute>>,
    players: Signal<usize>,
    mobile: Signal<bool>,
) -> impl Piece {
    cover(open, move |route: &GameRoute| {
        let section = route.0;
        // The header draws in ink that contrasts with the game's surface: white on the dark
        // boards, near-black on 2048's cream one. Set before the page is built, because a
        // title's ink is resolved as its piece is made, not when it draws.
        gamekit::chrome::set_surface(game_background(section));
        // These games have real alternate layouts or intentionally support sideways play.
        let portrait = !matches!(
            section,
            Section::Solitaire | Section::Sudoku | Section::Mines | Section::Charades
        );
        orientation::portrait(portrait);
        Scope::current().on_cleanup(|| orientation::portrait(false));
        let game = match section {
            Section::BlockBlast => blockblast::blockblast_page(),
            Section::Breakout => breakout::breakout_page(),
            Section::Charades => charades::charades_page(),
            Section::MatchThree => matchthree::matchthree_page(),
            Section::Mines => mines::mines_page(),
            Section::Flipside => flipside::flipside_page_for_players(
                mobile.get_untracked().then(|| players.get_untracked()),
            ),
            Section::Pipes => pipes::pipes_page(),
            Section::TicTacToe => tictactoe::tictactoe_page(players.get_untracked()),
            Section::Memory => memory::memory_page(players.get_untracked()),
            Section::Moles => moles::moles_page(players.get_untracked()),
            Section::Snakes => snakes::snakes_page(players.get_untracked()),
            Section::PaintFight => paintfight::paintfight_page(players.get_untracked()),
            Section::Pool => pool::pool_page(players.get_untracked()),
            Section::ButtonSoccer => buttonsoccer::buttonsoccer_page(players.get_untracked()),
            Section::DotsBoxes => dotsboxes::dotsboxes_page(players.get_untracked()),
            Section::Impostor => impostor::impostor_page(players.get_untracked()),
            Section::MiniGolf => minigolf::minigolf_page(players.get_untracked()),
            Section::FruitDuel => fruitduel::fruitduel_page(players.get_untracked()),
            Section::FruitMerge => fruitmerge::fruitmerge_page(),
            Section::Basketball => basketball::basketball_page(players.get_untracked()),
            Section::Sirtet => sirtet::sirtet_page(),
            Section::Solitaire => solitaire::solitaire_page(),
            Section::Sudoku => sudoku::sudoku_page(),
            Section::Game2048 => twentyfortyeight::twentyfortyeight_page(),
        };
        // Leaving is the shell's to do, and the button that asks for it sits in the game's own
        // header row (gamekit::chrome::game_header), where it lines up with the title and the
        // pause button. Registered per open cover, and cleared with it.
        gamekit::on_close(move || open.set(None));
        game.grow(Dimension::Both)
            .defers_system_gestures(Edges::ALL)
            .interactive_dismiss_disabled()
            .any()
    })
    .background(|route| game_background(route.0))
}

#[cfg(test)]
mod catalog_tests {
    use super::*;
    #[test]
    fn player_counts_match_game_capabilities() {
        for p in 1..=4 {
            assert_eq!(supports_players(Section::Sudoku, p, true), p == 1);
            assert_eq!(supports_players(Section::Flipside, p, true), p <= 2);
            assert_eq!(supports_players(Section::TicTacToe, p, true), p <= 2);
            assert_eq!(supports_players(Section::Pool, p, true), p <= 2);
            assert_eq!(supports_players(Section::Charades, p, true), p >= 2);
            assert_eq!(supports_players(Section::FruitMerge, p, true), p == 1);
            assert_eq!(supports_players(Section::Impostor, p, true), p >= 3);
            for game in [
                Section::Memory,
                Section::Moles,
                Section::Snakes,
                Section::PaintFight,
                Section::Basketball,
                Section::ButtonSoccer,
                Section::DotsBoxes,
                Section::MiniGolf,
                Section::FruitDuel,
            ] {
                assert!(supports_players(game, p, true));
            }
        }
        for game in [
            Section::TicTacToe,
            Section::Memory,
            Section::Pool,
            Section::ButtonSoccer,
            Section::DotsBoxes,
            Section::MiniGolf,
            Section::FruitMerge,
        ] {
            assert!(supports_players(game, 1, false));
            assert!(!supports_players(game, 2, false));
        }
        for game in [
            Section::Moles,
            Section::Snakes,
            Section::PaintFight,
            Section::Basketball,
            Section::Charades,
            Section::Impostor,
            Section::FruitDuel,
        ] {
            assert!(!supports_players(game, 1, false));
            if !mobile_profile() {
                assert!(GameRoute::from_key(&game.key()).is_none());
            }
        }
    }
}

#[cfg(test)]
mod score_format_tests {
    #[test]
    fn territory_percent_always_keeps_one_decimal() {
        day::install_locales("en", crate::res::locales::CATALOG);
        for (value, expected) in [
            (0.0, "0.0%"),
            (9.9, "9.9%"),
            (15.0, "15.0%"),
            (15.4, "15.4%"),
            (100.0, "100.0%"),
        ] {
            assert_eq!(game_resources_for_test(value), expected);
        }
    }
    fn game_resources_for_test(value: f64) -> String {
        // Fluent surrounds numeric substitutions with bidi isolates; these do not occupy space.
        crate::res::str::party_area_percent(value)
            .format()
            .chars()
            .filter(|c| !matches!(c, '\u{2068}' | '\u{2069}'))
            .collect()
    }
}
