# Games Fair

Solitaire, Block Blast, Breakout, falling blocks, Sudoku, 2048, Mines, Charades, Flipside,
Pipes, Match Three, and thirteen additional mobile games in one app. Built with
[Day](https://daybrite.dev) in one Rust codebase and rendered with the platform's own widgets on
iPhone, Android, HarmonyOS, macOS, Windows, Linux, and the web. Every game runs entirely on the
device, and your progress is saved when you leave a game and restored when you come back.

## Run it in one command

Install the `day` CLI, then let it clone, build, and launch the app on your desktop:

```sh
cargo install day-cli
day launch --git https://github.com/Games-Fair/Games-Fair.git
```

With no `-p`, the CLI picks the host's own toolkit (`macos-appkit`, `windows-winui`, `linux-gtk`).
Name a target to run elsewhere: `-p ios-uikit` for a booted Simulator, `-p android-material` for a
running emulator or device, `-p harmony-arkui` for HarmonyOS, `-p web-dom` to serve the browser
build. `day doctor` lists what each toolkit needs and prints the install command for anything
missing. The launch prints where it put the checkout, so you can open the code and change it.

To rename a fork, change `[app].title` in `Day.toml`: the home header and the window title read
it. `[app].id` there is the app's own id, `io.github.GamesFair`. The App Store and Play records
the App Fair publishes this app under are separate, and live in `Day-appfair.toml`, which
`day --flavor appfair` merges over the manifest.

## The games

- **Solitaire.** Klondike, turning the stock one card at a time or three. With Winnable Deals
  Only on, a solver plays every deal through to a win before it reaches the table, and Hint
  shows a move on a winning line. Drag or tap cards; once every card is face up, the rest fly
  home by themselves, and a win sends the whole deck bouncing off the table.
- **Block Blast.** Drag pieces from a tray of three onto an 8×8 board and fill rows and columns
  to clear them. A preview lights up the lines a drop would clear, clears on consecutive
  placements build a combo worth up to four times the points, emptying the board pays a
  5,000-point bonus, and three difficulties decide how reliably the tray deals pieces that fit.
- **Breakout.** Clear the bricks with a paddle that glides under your finger and can smash the
  ball up or down. Quick successive breaks multiply the score, armored bricks arrive on later
  levels, and five power-ups drop from marked bricks: a wider paddle, a slower ball, a ball
  that smashes straight through, extra balls, and an extra life.
- **Falling blocks.** The one you already know how to play, with the next piece shown, lines
  clearing in a flash, and the pieces coming quicker as the level climbs.
- **Sudoku.** Four difficulties, pencil marks, unlimited undo and redo, a checkpoint you can
  commit or revert, hints where the difficulty allows them, and a best time per difficulty.
- **2048.** Slide the tiles, watch them merge, and chase your best score. Reaching 2048 is not
  the end unless you want it to be.
- **Mines.** The sweeper you know, with the sharp edges filed off: your first tap is always safe
  and opens a clearing, holding a square plants a flag (or turn on Flag Mode and tap), and tapping
  a number that already has its flags opens the rest around it. Three board sizes, each turning
  with the window so the squares stay big enough to hit on a phone, and a best time for each.
- **Pipes.** Rotate tiles to connect every branch to the gold source. Choose a 5×5, 7×7,
  or 9×9 generated puzzle; lock tiles to protect them, and watch a wave of light cross the
  completed network. Every puzzle is solvable. Saves include rotations and locks, and each
  board size keeps its fewest-rotation record. Arrows select, Space/Enter rotates, and L locks.
- **Flipside.** Trap and flip opposing discs on an 8×8 board. Play Black against three
  computer difficulties or share the board in pass-and-play. Legal moves are marked, flips
  animate, and a side with no move passes automatically. Use arrows and Space/Enter on a
  keyboard. The game ends when neither side can move; the larger disc count wins.
- **Charades.** The party game for a phone on your forehead: your friends give clues, you nod
  when you guess right and tip your head back to pass, and the phone reads the tilt. Decks run
  from animals and movies to sayings and things to act out, no word repeats until its deck is
  played through, and the screen stays on for the whole round. Without a motion sensor, rounds
  use Correct and Pass buttons. The word lists live in `games/charades/words/`, one folder per
  language; its README explains the format and how another language gets its own lists.

- **Match Three.** Six levels of matching faceted gems, with sparkle trails behind moving pieces,
  bursts where matches clear, and haptics timed to picks, landings, and cascades.
  Hold and drag to preview neighboring swaps; lift to commit or drag home to cancel. You can
  also tap neighbors. Match four for stripes, T/L shapes for area bursts, and five for a
  wildcard. Combine specials, collect red circles, and clear single/double layers on boards
  with different layouts and falling lanes. Earn up to three stars per level; the current
  board and progress save automatically. Original sound cues accompany haptic patterns,
  with separate sound, vibration, and reduced-motion settings.

On phones, tablets, and touch-device browser profiles, **Players: 1 / 2 / 3 / 4** filters the
catalog by the number of people playing. Desktop players can play Tic-Tac-Toe, Memory, Pool,
Button Soccer, Dots and Boxes, Mini Golf, and Fruit Merge alongside the classic solo games.
Charades, Impostor, Fruit Duel, Paint Fight, Whack-a-Mole, Snakes, and Basketball Hoops use
the mobile catalog. Flipside also supports pass-and-play on a shared device.

- **Tic-Tac-Toe.** Two players or an unbeatable computer; animated marks, winning lines, draws,
  and rematches with a running win tally.
- **Memory.** Thirty illustrated cards and fifteen pairs, with animated reveals and pair removal.
  One to four players; matching keeps the turn, a mismatch passes it. Solo play counts attempts.
- **Whack-a-Mole.** Simultaneous colored targets in eleven holes. Tap your color; the first to
  thirty wins. One to four humans, with a computer opponent when playing alone.
- **Snakes.** Continuous steering with independent corner joysticks for up to four players.
  Illustrated snake heads and coiled body segments, accelerating body growth, and opposite-edge wrapping. Avoid other
  snakes; the last survivor wins. Solo play adds a computer opponent.
- **Paint Fight.** Hold your joystick to roll, repaint rival territory, and collect speed boosts and paint bursts. Own the most area after 45 seconds.
  Independent touch controls, live percentages, a time bar, and solo computer play.
- **Pool.** Pocket the seven solids or stripes in your group before the black eight. Pull back from the cue ball,
  preview the contact point and power, then release. Ball collisions, cushions, pockets, scratches,
  retained turns, early-eight losses, computer shots, and rematches are implemented.

- **Basketball Hoops.** Swipe shots with spinning basketballs, rim and backboard rebounds, net
  swishes, bounce audio and haptics. Thirty-second turns for two to four players, 45 seconds solo;
  the court flips for the players across the device.

- **Button Soccer.** Flick star discs at a soccer ball, using six formations, rebound shots,
  animated goals and a first-to-three match. One to four players, including solo AI.
- **Dots and Boxes.** Claim the fourth side of a box to score and keep your turn. One to four
  players, including an opponent that avoids handing over easy boxes.
- **Impostor.** Privately reveal roles, discuss clues and vote together. Three to ten players,
  one or two impostors, category filters, optional hints and six word-deck languages.
- **Mini Golf.** Seven rounds of hole-in-one attempts across nine courses, including moving
  obstacles and wind. Alternating ends, rebounds, hole effects and solo AI.
- **Fruit Duel.** Race to slice fruit, watch for decoys, and be the first to ten points.
- **Fruit Merge.** Drop and stack eleven kinds of fruit, merge matching pairs, undo a drop,
  sweep small fruit, and chase a saved best score at three difficulties.

The new games use the catalog’s selected player count, sound and vibration controls, pause/resume,
saved rounds, and result celebrations. The arcade illustrations are drawn in SVG and code. Their
sound effects are synthesized locally. See the [artwork and audio build notes](docs/party-games.md).

Every game has the same shell: a pause button, a menu to resume, start over, open the
settings, or reread the rules, and a results card with your score or the outcome. The rules open by
themselves the first time you play a game. Each game plays sound effects timed to its haptics,
and its settings have a Sounds switch and a Vibrations switch. With a mouse or trackpad the Breakout paddle follows the pointer across the field
and the cursor hides while it does; the arrow keys move the paddle, the piece, the tiles, or
the Sudoku selection. In Block Blast, 1 to 3 pick up a piece, the arrows move it, and the same
number drops it. In Solitaire, 1 to 7 pick a column and another of those digits moves it there,
8 picks the waste, 9 sends a card home, and 0 turns the stock. In Charades, ↓ or Return scores
a card and ↑ passes it. In Mines, the arrows move the pointer, Space or Return uncovers, and F
plants a flag.

The app uses a consistent dark appearance for its native controls. Its Settings hold what applies to every game: Language (any of the 13
the app ships), the Sounds and Vibrations a game starts with the first time it opens (a game
you have changed keeps its own), and Reset all records, which clears every game's best scores,
best times, and games won at once. About shows the version, links to this site and the source,
and the licenses of the five bundled fonts. On a desktop, Settings… in the app menu (⌘, on a
Mac) and About open them as windows. In the browser, the same two items are in the menu button
at the top right. On a phone, the gear beside the home title presents them. The surface is Day's shared one
([day-piece-settings](https://daybrite.dev/docs/settings)).

The home screen is a grid of tiles that fills the window's width and adds a column whenever
the window has room for another tile. Each preview is drawn by the game's own crate with the same code that renders gameplay. Tapping a tile presents the game in a fullscreen cover with an X
to exit. On the phones the games defer the system's edge gestures and disable interactive
dismissal, so an edge swipe mid-game stays in the game.

Everything runs on the device with nothing to sign in to and nothing shown but the game. Each
game is a self-contained crate with physics on the frame clock and a serde save state.

## Build from a clone

Day compiles one toolkit backend per binary, so name a target when you build or launch. Every
target the app ships is listed in `Day.toml`.

```sh
day doctor                       # toolchains present and missing, with fixes
day launch -p macos-appkit       # the Mac's own toolkit (macos-gtk and macos-qt run here too)
day launch -p ios-uikit          # needs a booted Simulator
day launch -p android-material        # needs a JDK and a running emulator or device
day launch -p harmony-arkui      # needs the OpenHarmony SDK and an emulator
day launch -p web-dom            # builds the wasm bundle and serves it to your browser
day build  -p windows-winui      # build only (Windows builds on a Windows host)
```

To build from plain cargo, pass the backend feature yourself, for example
`cargo build --features appkit`; a bare `cargo build` enables no backend and will not link.

[Dayscripts](https://daybrite.dev/docs/dayscript) drive the app: `smoke.yaml` opens each
game, `sudoku.yaml` walks every Sudoku surface, `blockblast.yaml` places Block Blast pieces
from the keyboard and walks its menus, `flipside.yaml` checks captures, solo play, menus, and save restoration;
`pipes.yaml` checks locks, saves, all board sizes, and a complete seeded solution;
`matchthree.yaml` checks invalid swaps, a win, progression, settings, and save restoration;
`matchthree-levels.yaml` completes all six levels (both use `--env DAY_GAMES_SEED=15 --locale en`);
`solitaire.yaml` plays a proven-winnable deal from the
keyboard, and `bk.yaml` and `games.yaml` sweep gameplay for screenshots; `games.yaml` ends in
the Settings window:

```sh
day launch -p ios-uikit --script dayscript/games.yaml
```

## Inside the code

- `src/lib.rs` is `root()`: the home grid and the fullscreen cover each game opens in, with typed
  routes so deep links and dayscript can open a game by name, and the Settings and About
  declaration: General, Sound & haptics, Records, and the font licenses `build.rs` gathers from
  `resource/font-licenses/`.
- `games/blockblast`, `games/breakout`, `games/sirtet`, `games/solitaire`, `games/sudoku`, `games/flipside`, `games/pipes`, and
  `games/twentyfortyeight` are one crate per game: canvas or grid-layout UI, physics on the
  frame clock, and a serde save state.
- `gamekit/src/animation.rs` adapts Day's native display callbacks to game demand. Each mounted
  loop tracks a predicate, sleeps when its work settles, wakes on input, and cancels on scope
  disposal. No game owns a frame timer. Elapsed clocks use raw frame deltas; physics and
  cosmetic integration explicitly cap catch-up at 100 ms. First/resumed frames have zero delta.
  Match Three (including gesture settling and tap suppression), 2048, Block Blast, Pipes, and
  Flipside stop between moves; AI turns, held-piece pulses, pending feedback, and result delays
  keep the relevant loop awake, as do Match Three's sparkles until the last one fades. Arcade
  play, live game clocks, and animated celebrations keep frames while needed. Pause/help screens
  and closed covers release demand. Mines only repaints its board for effects/input, even while
  its elapsed-time readout is running.
- `gamekit/` is also the shared persistence layer: each game's state is saved when its cover closes
  or the app is backgrounded, and restored the next time it opens. A game that keeps a clock
  can also hook the backgrounding itself, which is how Sudoku pauses. Its `GameSettings`
  default reads the app-wide Sounds and Vibrations prefs, which is how the Settings switches
  seed a game that has never saved its own, and `on_reset_records` is how an open game's
  bests follow Reset all records.
- `resource/locales/en/app.ftl` carries every user-facing string.
- `platform/` holds the thin native host projects the Apple, Android, and HarmonyOS targets
  build through.

`dayscript/frame-sync.yaml` exercises repeated moves after idle periods, pause/resume, and
closing/reopening animated covers. Run with `--env DAY_GAMES_SEED=15 --locale en`.
`cargo test --workspace` includes a manual-frame regression for zero work while idle, wakeup,
settling without an extra frame, raw deltas, pause/resume baselines, and late delivery after
scope cancellation. `tests/frame_sync.rs` mounts six real game pages and verifies native
request counts across input, idle, AI responses, pause, held-piece pulses, hint searches, and
scope disposal; it preserves and restores the preferences it exercises. `tests/records.rs` seeds
every game's records and checks that Reset all records clears them, the same way.
`tests/fits_screen.rs` opens every game, with its how-to-play card and without, on the narrowest
phone screens (320 points wide, upright and sideways) and fails if anything reaches past the
window's edges.

`day lint` checks routes, element ids, and locale coverage.

For a native Match Three held-drag check, launch an Android emulator with
`--env DAY_GAMES_SEED=15 --locale en`, then run
`python3 scripts/matchthree-touch-test.py SERIAL`. This checks reversible previews while a
finger is held, commit on release, cancellation, and ordinary taps.

For Android cover hit testing, launch on a 360×640dp emulator with
`day launch -p android-material --android-device SERIAL --env DAY_GAMES_SEED=15 --keep-alive`,
then run `python3 scripts/android-cover-touch-test.py SERIAL` with `adb` on PATH.
This uses native screen taps to check that passive game headers cannot activate the hidden
home grid, while pause, close, and the home tiles still respond. Ordinary dayscript taps
address a node directly and cannot detect this form of touch-through.

## License

Games Fair is free software under the GNU Affero General Public License, version 3 only
([`LICENSE.txt`](LICENSE.txt)), with the App Fair Distribution Exception
([`LICENSE-EXCEPTIONS.txt`](LICENSE-EXCEPTIONS.txt)), which permits distribution through an app
storefront notwithstanding terms that would otherwise conflict with the AGPL. Every source file
says so:

```
// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
```

## Publishing through the App Fair

The App Fair Project builds, signs, and submits the apps in its catalog. To publish a new
version, tag a release and open a pull request against
[`appfair/appfair-apps`](https://github.com/appfair/appfair-apps) naming the tag and its commit.

## Website theme

`website/site.toml` selects [App Fair's appsite theme](https://github.com/appfair/appsite)
at `main`. The shared Day workflow fetches the theme and uses its Astro configuration,
App Fair header/footer, and journal pages automatically on qualifying website deployments.
The app keeps its own URL, accent color, platform choices, localized listings, screenshots,
and downloads. Project customization can be added in `website/daysite.config.mjs`;
`website/theme.css` is applied after the theme's styles.

This requires the published customization-capable daysite renderer, the theme repository,
and the updated `daybrite/actions` `v1` workflow. No separate website deployment workflow is
needed. For local previews, clone and install daysite, clone appsite, and set `DAYSITE_THEME`
to the appsite checkout's absolute path before installing customization dependencies or
running the preview. See Day's [app website guide](https://daybrite.dev/docs/websites)
for setup, customization, GitHub Pages, and custom domains.
