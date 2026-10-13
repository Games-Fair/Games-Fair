// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! The rules of both games, as the arcade ran them: tile-centered movement with Pac-Pac's
//! cornering, the four ghosts' targets and the scatter/chase schedule, the per-level speed,
//! fright and fruit tables, the ghost house's dot counters, Cruise Elroy, the pauses that
//! follow a dot, a ghost, a death and a cleared level, and Ms. Pac-Pac's four mazes with her
//! wandering fruit and the ghosts' random first strolls. Pixel units throughout: the maze is
//! 224 pixels wide, a tile is 8, and a character at 100% covers 75.76 pixels a second.

use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::art::{self, COLS, Fruit, Maze, ROWS, TILE, Tile};

/// The two games.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Mode {
    PacPac,
    MsPacPac,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Dir {
    Right,
    Left,
    Up,
    Down,
}

impl Dir {
    /// The order the arcade breaks ties in: up, left, down, right.
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Left, Dir::Down, Dir::Right];

    pub fn step(self) -> (i32, i32) {
        match self {
            Dir::Right => (1, 0),
            Dir::Left => (-1, 0),
            Dir::Up => (0, -1),
            Dir::Down => (0, 1),
        }
    }
    pub fn unit(self) -> (f64, f64) {
        let (x, y) = self.step();
        (x as f64, y as f64)
    }
    pub fn opposite(self) -> Dir {
        match self {
            Dir::Right => Dir::Left,
            Dir::Left => Dir::Right,
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
        }
    }
    /// The sprite tables' order.
    pub fn index(self) -> usize {
        match self {
            Dir::Right => 0,
            Dir::Left => 1,
            Dir::Up => 2,
            Dir::Down => 3,
        }
    }
    fn horizontal(self) -> bool {
        matches!(self, Dir::Left | Dir::Right)
    }
}

/// Pixels per second at 100%.
pub const FULL_SPEED: f64 = 75.757_575;
/// How far from a tile's center Pac-Pac may cut a corner.
const CORNER: f64 = 4.0;
/// Where the tunnel wraps: the center of the second hidden tile past each edge.
const WRAP_LEFT: f64 = -12.0;
const WRAP_RIGHT: f64 = COLS as f64 * TILE + 12.0;
/// The ghost house: its door's center and the resting spots inside.
const HOUSE_X: f64 = 112.0;
const HOUSE_Y: f64 = 116.0;
const DOOR_Y: f64 = 92.0;
/// The tile ghosts aim for on their way home, just left of the door.
const HOME_TILE: (i32, i32) = (13, 11);
/// Pac-Pac's start.
const PAC_START: (f64, f64) = (112.0, 188.0);
/// Where Pac-Pac's fruit waits.
const FRUIT_SPOT: (f64, f64) = (112.0, 140.0);
/// An extra life at this score, once.
const EXTRA_LIFE_AT: i64 = 10_000;
pub const START_LIVES: i32 = 3;
/// Ghost house exits by timer: no dot for this long frees the next ghost.
fn no_dot_limit(level: u32) -> f64 {
    if level >= 5 { 3.0 } else { 4.0 }
}

/// The per-level numbers from the arcade's tables. Speeds are fractions of [`FULL_SPEED`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelSpec {
    pub pac: f64,
    pub pac_fright: f64,
    pub ghost: f64,
    pub ghost_fright: f64,
    pub tunnel: f64,
    /// Dots left and Blinky's speed once Cruise Elroy kicks in, first and second stage.
    pub elroy1: (usize, f64),
    pub elroy2: (usize, f64),
    pub fright_secs: f64,
    pub flashes: u32,
}

pub fn level_spec(level: u32) -> LevelSpec {
    let (elroy, fright, flashes) = match level {
        1 => ((20, 10), 6.0, 5),
        2 => ((30, 15), 5.0, 5),
        3 => ((40, 20), 4.0, 5),
        4 => ((40, 20), 3.0, 5),
        5 => ((40, 20), 2.0, 5),
        6 => ((50, 25), 5.0, 5),
        7 | 8 => ((50, 25), 2.0, 5),
        9 => ((60, 30), 1.0, 3),
        10 => ((60, 30), 5.0, 5),
        11 => ((60, 30), 2.0, 5),
        12 | 13 => ((80, 40), 1.0, 3),
        14 => ((80, 40), 3.0, 5),
        15 | 16 => ((100, 50), 1.0, 3),
        17 => ((100, 50), 0.0, 0),
        18 => ((100, 50), 1.0, 3),
        _ => ((120, 60), 0.0, 0),
    };
    let (pac, pac_fright, ghost, ghost_fright, tunnel, e1, e2) = match level {
        1 => (0.80, 0.90, 0.75, 0.50, 0.40, 0.80, 0.85),
        2..=4 => (0.90, 0.95, 0.85, 0.55, 0.45, 0.90, 0.95),
        5..=20 => (1.00, 1.00, 0.95, 0.60, 0.50, 1.00, 1.05),
        _ => (0.90, 0.90, 0.95, 0.60, 0.50, 1.00, 1.05),
    };
    LevelSpec {
        pac,
        pac_fright,
        ghost,
        ghost_fright,
        tunnel,
        elroy1: (elroy.0, e1),
        elroy2: (elroy.1, e2),
        fright_secs: fright,
        flashes,
    }
}

/// The scatter/chase schedule: seven timed periods, then chase for good.
fn schedule(level: u32) -> [f64; 7] {
    match level {
        1 => [7.0, 20.0, 7.0, 20.0, 5.0, 20.0, 5.0],
        2..=4 => [7.0, 20.0, 7.0, 20.0, 5.0, 1033.0, 1.0 / 60.0],
        _ => [5.0, 20.0, 5.0, 20.0, 5.0, 1037.0, 1.0 / 60.0],
    }
}

/// The bonus item of `level`, and its points.
pub fn fruit_for(mode: Mode, level: u32, rng: &mut Rng) -> (Fruit, i64) {
    match mode {
        Mode::PacPac => match level {
            1 => (Fruit::Cherry, 100),
            2 => (Fruit::Strawberry, 300),
            3 | 4 => (Fruit::Orange, 500),
            5 | 6 => (Fruit::Apple, 700),
            7 | 8 => (Fruit::Melon, 1000),
            9 | 10 => (Fruit::Galaxian, 2000),
            11 | 12 => (Fruit::Bell, 3000),
            _ => (Fruit::Key, 5000),
        },
        Mode::MsPacPac => {
            const MS: [(Fruit, i64); 7] = [
                (Fruit::Cherry, 100),
                (Fruit::Strawberry, 200),
                (Fruit::Orange, 500),
                (Fruit::Pretzel, 700),
                (Fruit::Apple, 1000),
                (Fruit::Pear, 2000),
                (Fruit::Banana, 5000),
            ];
            if (1..=7).contains(&level) {
                MS[level as usize - 1]
            } else {
                MS[rng.below(7)]
            }
        }
    }
}

/// Which maze `level` plays in: Pac-Pac's always; Ms. Pac-Pac's first for two levels, her
/// second for three, her third for four, her fourth for four, then the last two in turn.
pub fn maze_index(mode: Mode, level: u32) -> usize {
    match mode {
        Mode::PacPac => 0,
        Mode::MsPacPac => match level {
            1 | 2 => 1,
            3..=5 => 2,
            6..=9 => 3,
            10..=13 => 4,
            n => {
                if (n - 14) / 4 % 2 == 0 {
                    3
                } else {
                    4
                }
            }
        },
    }
}

/// A tiny xorshift RNG: no `rand` dependency, deterministic per seed.
pub struct Rng(pub u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// A character on the grid: a position in maze pixels and a heading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Actor {
    pub x: f64,
    pub y: f64,
    pub dir: Dir,
}

impl Actor {
    pub fn tile(&self) -> (i32, i32) {
        (
            (self.x / TILE).floor() as i32,
            (self.y / TILE).floor() as i32,
        )
    }
    fn wrap(&mut self) {
        if self.x < WRAP_LEFT {
            self.x += WRAP_RIGHT - WRAP_LEFT;
        } else if self.x > WRAP_RIGHT {
            self.x -= WRAP_RIGHT - WRAP_LEFT;
        }
    }
}

pub fn tile_center(c: i32, r: i32) -> (f64, f64) {
    (c as f64 * TILE + TILE / 2.0, r as f64 * TILE + TILE / 2.0)
}

/// Move `a` forward by `dist`, pausing at every tile center to ask `decide` which way to go on
/// (`None` stops it there). Returns how far it went.
fn advance(
    maze: &Maze,
    a: &mut Actor,
    mut dist: f64,
    through_door: bool,
    decide: &mut dyn FnMut(&Actor) -> Option<Dir>,
) -> f64 {
    let mut moved = 0.0;
    let mut guard = 0;
    while dist > 1e-9 && guard < 64 {
        guard += 1;
        let (dx, dy) = a.dir.unit();
        // The next center at or ahead of the actor along its heading.
        let along = if a.dir.horizontal() {
            let c = ((a.x - TILE / 2.0) / TILE * dx).ceil() * dx;
            c * TILE + TILE / 2.0 - a.x
        } else {
            let r = ((a.y - TILE / 2.0) / TILE * dy).ceil() * dy;
            r * TILE + TILE / 2.0 - a.y
        } * if a.dir.horizontal() { dx } else { dy };
        let along = along.abs();
        if along > 1e-6 {
            let step = dist.min(along);
            a.x += dx * step;
            a.y += dy * step;
            a.wrap();
            dist -= step;
            moved += step;
            if step < along {
                break;
            }
        }
        // At a center: choose, then check the way is open.
        let (c, r) = a.tile();
        let (cx, cy) = tile_center(c, r);
        a.x = cx;
        a.y = cy;
        let Some(next) = decide(a) else {
            break;
        };
        a.dir = next;
        if !maze.open(c, r, a.dir, through_door) {
            break;
        }
        let (dx, dy) = a.dir.unit();
        let step = dist.min(TILE);
        a.x += dx * step;
        a.y += dy * step;
        a.wrap();
        dist -= step;
        moved += step;
    }
    moved
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GhostKind {
    Blinky,
    Pinky,
    Inky,
    Clyde,
}

impl GhostKind {
    pub fn index(self) -> usize {
        match self {
            GhostKind::Blinky => 0,
            GhostKind::Pinky => 1,
            GhostKind::Inky => 2,
            GhostKind::Clyde => 3,
        }
    }
    /// The corner each ghost retreats to in scatter mode, in tiles (off the maze on purpose).
    fn scatter_target(self) -> (i32, i32) {
        match self {
            GhostKind::Blinky => (25, -3),
            GhostKind::Pinky => (2, -3),
            GhostKind::Inky => (27, 32),
            GhostKind::Clyde => (0, 32),
        }
    }
    /// Where the ghost waits in the house.
    fn home_x(self) -> f64 {
        match self {
            GhostKind::Inky => HOUSE_X - 16.0,
            GhostKind::Clyde => HOUSE_X + 16.0,
            _ => HOUSE_X,
        }
    }
    /// Dots Pac-Pac must eat before this ghost leaves the house, per level.
    fn personal_limit(self, level: u32) -> u32 {
        match (self, level) {
            (GhostKind::Inky, 1) => 30,
            (GhostKind::Clyde, 1) => 60,
            (GhostKind::Clyde, 2) => 50,
            _ => 0,
        }
    }
    /// The same, counted globally after a life is lost.
    fn global_limit(self) -> u32 {
        match self {
            GhostKind::Pinky => 7,
            GhostKind::Inky => 17,
            GhostKind::Clyde => 32,
            GhostKind::Blinky => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GhostState {
    /// Bobbing in the house.
    Home,
    /// Sliding to the door and out.
    Leaving,
    /// Loose in the maze: scattering, chasing, or frightened.
    Roam,
    /// Eaten: a pair of eyes hurrying home.
    Eyes,
    /// Back at the door, dropping into the house.
    Entering,
}

#[derive(Clone, Copy, Debug)]
pub struct Ghost {
    pub kind: GhostKind,
    pub actor: Actor,
    pub state: GhostState,
    pub frightened: bool,
    /// A direction change ordered by a mode switch, taken at once.
    reverse: bool,
    dot_counter: u32,
}

impl Ghost {
    fn new(kind: GhostKind) -> Ghost {
        let (x, y, dir, state) = match kind {
            GhostKind::Blinky => (HOUSE_X, DOOR_Y, Dir::Left, GhostState::Roam),
            GhostKind::Pinky => (HOUSE_X, HOUSE_Y, Dir::Down, GhostState::Home),
            GhostKind::Inky => (kind.home_x(), HOUSE_Y, Dir::Up, GhostState::Home),
            GhostKind::Clyde => (kind.home_x(), HOUSE_Y, Dir::Up, GhostState::Home),
        };
        Ghost {
            kind,
            actor: Actor { x, y, dir },
            state,
            frightened: false,
            reverse: false,
            dot_counter: 0,
        }
    }
    /// Whether Pac-Pac can run into this ghost.
    pub fn in_play(&self) -> bool {
        self.state == GhostState::Roam
    }
}

/// A moment in the round the page reacts to with a sound, a haptic, or a card.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Happening {
    /// A new game's first round: the jingle.
    Intro,
    /// Play began (after READY!).
    Go,
    Chomp,
    Energizer,
    GhostEaten(i64),
    FruitEaten(i64),
    FruitAppeared,
    ExtraLife,
    Death,
    LevelClear,
    GameOver,
}

/// What the round is doing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    /// "READY!", counting down; `intro` is the longer first one of a game.
    Ready {
        t: f64,
        intro: bool,
    },
    Play,
    /// A ghost just got eaten: everything holds for a second showing the score.
    GhostEaten {
        t: f64,
    },
    /// Caught: everything holds, then Pac-Pac's death.
    DeathFreeze {
        t: f64,
    },
    Death {
        t: f64,
    },
    /// The last dot went: a hold, then the walls blink.
    LevelFreeze {
        t: f64,
    },
    LevelFlash {
        t: f64,
    },
    /// "GAME OVER" in the maze.
    GameOverText {
        t: f64,
    },
    GameOver,
}

const READY_SECS: f64 = 2.0;
const INTRO_SECS: f64 = 4.3;
/// How long the intro shows "PLAYER ONE" alone before the actors appear.
pub const INTRO_EMPTY_SECS: f64 = 2.2;
const GHOST_EATEN_SECS: f64 = 1.0;
const DEATH_FREEZE_SECS: f64 = 1.0;
pub const DEATH_FRAME_SECS: f64 = 0.13;
const LEVEL_FREEZE_SECS: f64 = 1.2;
pub const LEVEL_FLASH_SECS: f64 = 2.0;
const GAME_OVER_TEXT_SECS: f64 = 2.5;
const FRUIT_SECS: f64 = 9.5;
/// Pac-Pac's hesitation after a dot and after an energizer.
const DOT_PAUSE: f64 = 1.0 / 60.0;
const ENERGIZER_PAUSE: f64 = 3.0 / 60.0;
/// A ghost's eyes hurry home at this fraction of full speed.
const EYES_SPEED: f64 = 1.6;
/// Ghosts cross the house doorway at this fraction.
const HOUSE_SPEED: f64 = 0.4;
/// Ms. Pac-Pac's fruit wanders at this fraction.
const FRUIT_SPEED: f64 = 0.45;
/// A flash of a frightened ghost: white then blue.
pub const FRIGHT_FLASH_SECS: f64 = 0.4;

/// The bonus item on the board.
#[derive(Clone, Copy, Debug)]
pub struct FruitState {
    pub kind: Fruit,
    pub points: i64,
    pub actor: Actor,
    /// Seconds left on the board (Pac-Pac's waits; Ms. Pac-Pac's wanders until it leaves).
    pub t: f64,
    pub wander: bool,
    /// A wandering fruit heads for the ghost house first, then for a tunnel.
    leaving: bool,
    exit: (i32, i32),
}

/// A score shown where something was eaten.
#[derive(Clone, Copy, Debug)]
pub struct Popup {
    pub x: f64,
    pub y: f64,
    pub value: i64,
    pub ghost: bool,
    pub t: f64,
}

/// The durable snapshot: the game's progress and the dots left on this board.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct SaveState {
    pub mode: Mode,
    pub level: u32,
    pub score: i64,
    pub lives: i32,
    pub eaten: Vec<bool>,
    pub fruit_spawned: u8,
    pub extra_life_given: bool,
    pub game_over: bool,
    pub history: Vec<Fruit>,
}

pub struct Game {
    pub mode: Mode,
    pub maze: Rc<Maze>,
    pub maze_index: usize,
    pub level: u32,
    pub spec: LevelSpec,
    pub score: i64,
    pub best: i64,
    /// Lives left including the one in play; the arcade shows one fewer.
    pub lives: i32,
    pub eaten: Vec<bool>,
    pub dots_eaten: usize,
    pub pac: Actor,
    pub want: Option<Dir>,
    /// Mouth animation phase, advanced while moving.
    pub pac_anim: f64,
    pac_pause: f64,
    pub ghosts: [Ghost; 4],
    pub phase: Phase,
    /// Play seconds, for the skirts, the energizers and the 1UP blink.
    pub time: f64,
    mode_timer: f64,
    mode_index: usize,
    pub chase: bool,
    pub fright_t: f64,
    ghosts_eaten: u32,
    no_dot_t: f64,
    global_counter: Option<u32>,
    pub fruit: Option<FruitState>,
    fruit_spawned: u8,
    pub popups: Vec<Popup>,
    elroy_ok: bool,
    extra_life_given: bool,
    /// The bonus item of each level played, newest last, for the row at the foot of the screen.
    pub history: Vec<Fruit>,
    pub game_over: bool,
    pub rng: Rng,
    pub happenings: Vec<Happening>,
    /// Bumped whenever the dots or the walls change, so the maze layer re-records only then.
    pub maze_version: u32,
}

impl Game {
    pub fn new(mode: Mode, seed: u64) -> Game {
        let mut g = Game {
            mode,
            maze: art::maze(maze_index(mode, 1)),
            maze_index: maze_index(mode, 1),
            level: 1,
            spec: level_spec(1),
            score: 0,
            best: 0,
            lives: START_LIVES,
            eaten: Vec::new(),
            dots_eaten: 0,
            pac: Actor {
                x: PAC_START.0,
                y: PAC_START.1,
                dir: Dir::Left,
            },
            want: None,
            pac_anim: 0.0,
            pac_pause: 0.0,
            ghosts: [
                Ghost::new(GhostKind::Blinky),
                Ghost::new(GhostKind::Pinky),
                Ghost::new(GhostKind::Inky),
                Ghost::new(GhostKind::Clyde),
            ],
            phase: Phase::Ready {
                t: INTRO_SECS,
                intro: true,
            },
            time: 0.0,
            mode_timer: 0.0,
            mode_index: 0,
            chase: false,
            fright_t: 0.0,
            ghosts_eaten: 0,
            no_dot_t: 0.0,
            global_counter: None,
            fruit: None,
            fruit_spawned: 0,
            popups: Vec::new(),
            elroy_ok: true,
            extra_life_given: false,
            history: Vec::new(),
            game_over: false,
            rng: Rng(seed | 1),
            happenings: Vec::new(),
            maze_version: 0,
        };
        g.load_level(1);
        g.happenings.push(Happening::Intro);
        g
    }

    /// The board for `level`, with every dot in place and the actors at their starts.
    fn load_level(&mut self, level: u32) {
        self.level = level;
        self.spec = level_spec(level);
        self.maze_index = maze_index(self.mode, level);
        self.maze = art::maze(self.maze_index);
        self.eaten = vec![false; COLS * ROWS];
        self.dots_eaten = 0;
        self.fruit_spawned = 0;
        self.fruit = None;
        self.popups.clear();
        let (fruit, _) = fruit_for(self.mode, level, &mut self.rng);
        if self.history.len() < level as usize {
            self.history.push(fruit);
        }
        self.global_counter = None;
        for g in self.ghosts.iter_mut() {
            g.dot_counter = 0;
        }
        self.reset_actors();
        self.maze_version += 1;
    }

    /// Everyone back to their starts for a new round of the same board.
    fn reset_actors(&mut self) {
        self.pac = Actor {
            x: PAC_START.0,
            y: PAC_START.1,
            dir: Dir::Left,
        };
        self.want = None;
        self.pac_anim = 0.0;
        self.pac_pause = 0.0;
        for g in self.ghosts.iter_mut() {
            let fresh = Ghost::new(g.kind);
            g.actor = fresh.actor;
            g.state = fresh.state;
            g.frightened = false;
            g.reverse = false;
        }
        self.mode_timer = 0.0;
        self.mode_index = 0;
        self.chase = false;
        self.fright_t = 0.0;
        self.ghosts_eaten = 0;
        self.no_dot_t = 0.0;
        self.fruit = None;
        self.popups.clear();
        self.elroy_ok = self.global_counter.is_none();
    }

    pub fn save_state(&self) -> SaveState {
        SaveState {
            mode: self.mode,
            level: self.level,
            score: self.score,
            lives: self.lives,
            eaten: self.eaten.clone(),
            fruit_spawned: self.fruit_spawned,
            extra_life_given: self.extra_life_given,
            game_over: self.game_over,
            history: self.history.clone(),
        }
    }

    /// A game resumed from `s`: the same board with the same dots left, the actors back at
    /// their starts, and a fresh READY!.
    pub fn from_save(s: SaveState, seed: u64) -> Game {
        let mut g = Game::new(s.mode, seed);
        g.happenings.clear();
        g.history = s.history;
        g.load_level(s.level.max(1));
        g.score = s.score;
        g.lives = s.lives.clamp(1, 99);
        g.extra_life_given = s.extra_life_given;
        g.fruit_spawned = s.fruit_spawned;
        if s.eaten.len() == g.eaten.len() {
            g.eaten = s.eaten;
            g.dots_eaten = g
                .eaten
                .iter()
                .enumerate()
                .filter(|(i, e)| **e && matches!(g.maze.tiles[*i], Tile::Dot | Tile::Energizer))
                .count();
        }
        if s.game_over {
            g.game_over = true;
            g.lives = 0;
            g.phase = Phase::GameOver;
        } else {
            g.phase = Phase::Ready {
                t: READY_SECS,
                intro: false,
            };
        }
        g
    }

    pub fn dots_left(&self) -> usize {
        self.maze.dot_count.saturating_sub(self.dots_eaten)
    }

    /// Which siren plays, 1 (calm) to 5 (frantic), by how few dots remain.
    pub fn siren_level(&self) -> u8 {
        let left = self.dots_left();
        let total = self.maze.dot_count.max(1);
        let frac = left as f64 / total as f64;
        if frac > 0.70 {
            1
        } else if frac > 0.42 {
            2
        } else if frac > 0.20 {
            3
        } else if frac > 0.07 {
            4
        } else {
            5
        }
    }

    pub fn any_eyes(&self) -> bool {
        self.ghosts
            .iter()
            .any(|g| matches!(g.state, GhostState::Eyes | GhostState::Entering))
    }

    pub fn frightened(&self) -> bool {
        self.fright_t > 0.0 && self.ghosts.iter().any(|g| g.frightened)
    }

    /// Whether the frightened ghosts show white right now (the warning flashes).
    pub fn fright_flash_white(&self) -> bool {
        if self.fright_t <= 0.0 {
            return false;
        }
        let window = self.spec.flashes as f64 * FRIGHT_FLASH_SECS;
        if self.fright_t > window {
            return false;
        }
        ((self.fright_t / (FRIGHT_FLASH_SECS / 2.0)).floor() as i64) % 2 == 1
    }

    pub fn playing(&self) -> bool {
        matches!(self.phase, Phase::Play)
    }

    /// Whether the round is still live: something to animate when the clock runs.
    pub fn live(&self) -> bool {
        !matches!(self.phase, Phase::GameOver)
    }

    /// A direction asked for by a swipe or a key: taken at once when it is open or opposite,
    /// remembered until the next turn otherwise.
    pub fn steer(&mut self, dir: Dir) {
        self.want = Some(dir);
    }

    pub fn start_new_game(&mut self, mode: Mode) {
        let seed = self.rng.0;
        let best = self.best;
        *self = Game::new(mode, seed);
        self.best = best;
    }

    fn record_best(&mut self) {
        if self.score > self.best {
            self.best = self.score;
        }
    }

    // --- the tick ---------------------------------------------------------------------

    pub fn update(&mut self, dt: f64) {
        self.popups.retain_mut(|p| {
            if !p.ghost {
                p.t -= dt;
            }
            p.t > 0.0
        });
        match self.phase {
            Phase::Ready { t, intro } => {
                let t = t - dt;
                if t <= 0.0 {
                    self.phase = Phase::Play;
                    self.happenings.push(Happening::Go);
                    // Pinky never waits at the start of a round.
                    if self.global_counter.is_none() {
                        self.release(GhostKind::Pinky);
                    }
                } else {
                    self.phase = Phase::Ready { t, intro };
                }
            }
            Phase::Play => self.play(dt),
            Phase::GhostEaten { t } => {
                let t = t - dt;
                if t <= 0.0 {
                    self.popups.retain(|p| !p.ghost);
                    self.phase = Phase::Play;
                } else {
                    self.phase = Phase::GhostEaten { t };
                }
            }
            Phase::DeathFreeze { t } => {
                let t = t - dt;
                if t <= 0.0 {
                    self.phase = Phase::Death { t: 0.0 };
                    self.happenings.push(Happening::Death);
                } else {
                    self.phase = Phase::DeathFreeze { t };
                }
            }
            Phase::Death { t } => {
                let t = t + dt;
                if t >= DEATH_FRAME_SECS * 13.0 {
                    self.after_death();
                } else {
                    self.phase = Phase::Death { t };
                }
            }
            Phase::LevelFreeze { t } => {
                let t = t - dt;
                if t <= 0.0 {
                    self.phase = Phase::LevelFlash { t: 0.0 };
                    self.maze_version += 1;
                } else {
                    self.phase = Phase::LevelFreeze { t };
                }
            }
            Phase::LevelFlash { t } => {
                let before = ((t / 0.25).floor()) as i64;
                let t = t + dt;
                let after = ((t / 0.25).floor()) as i64;
                if before != after {
                    self.maze_version += 1;
                }
                if t >= LEVEL_FLASH_SECS {
                    let next = self.level + 1;
                    self.load_level(next);
                    self.phase = Phase::Ready {
                        t: READY_SECS,
                        intro: false,
                    };
                } else {
                    self.phase = Phase::LevelFlash { t };
                }
            }
            Phase::GameOverText { t } => {
                let t = t - dt;
                if t <= 0.0 {
                    self.phase = Phase::GameOver;
                    self.happenings.push(Happening::GameOver);
                } else {
                    self.phase = Phase::GameOverText { t };
                }
            }
            Phase::GameOver => {}
        }
    }

    /// The walls blink white during the level flash.
    pub fn walls_white(&self) -> bool {
        match self.phase {
            Phase::LevelFlash { t } => ((t / 0.25).floor() as i64) % 2 == 1,
            _ => false,
        }
    }

    fn after_death(&mut self) {
        self.lives -= 1;
        if self.lives <= 0 {
            self.lives = 0;
            self.game_over = true;
            self.record_best();
            self.phase = Phase::GameOverText {
                t: GAME_OVER_TEXT_SECS,
            };
        } else {
            self.global_counter = Some(0);
            self.reset_actors();
            self.phase = Phase::Ready {
                t: READY_SECS,
                intro: false,
            };
        }
    }

    fn play(&mut self, dt: f64) {
        self.time += dt;
        // Fright runs its own clock; the scatter/chase clock waits for it.
        if self.fright_t > 0.0 {
            self.fright_t -= dt;
            if self.fright_t <= 0.0 {
                self.fright_t = 0.0;
                for g in self.ghosts.iter_mut() {
                    g.frightened = false;
                }
            }
        } else {
            self.mode_timer += dt;
            let sched = schedule(self.level);
            if self.mode_index < sched.len() && self.mode_timer >= sched[self.mode_index] {
                self.mode_timer -= sched[self.mode_index];
                self.mode_index += 1;
                self.chase = !self.chase;
                for g in self.ghosts.iter_mut() {
                    if g.state == GhostState::Roam {
                        g.reverse = true;
                    }
                }
            }
        }

        // The house: a ghost leaves when no dot has been eaten for a while.
        self.no_dot_t += dt;
        if self.no_dot_t >= no_dot_limit(self.level) {
            self.no_dot_t = 0.0;
            if let Some(k) = self.next_waiting() {
                self.release(k);
            }
        }

        self.step_pac(dt);
        for i in 0..4 {
            self.step_ghost(i, dt);
        }
        self.step_fruit(dt);
        self.collide();
    }

    fn pac_speed(&self) -> f64 {
        let frac = if self.frightened() {
            self.spec.pac_fright
        } else {
            self.spec.pac
        };
        FULL_SPEED * frac
    }

    fn step_pac(&mut self, dt: f64) {
        if self.pac_pause > 0.0 {
            self.pac_pause -= dt;
            return;
        }
        if let Some(w) = self.want
            && w == self.pac.dir.opposite()
        {
            self.pac.dir = w;
            self.want = None;
        }
        // Cornering: a turn asked for near a tile's center is taken from the center.
        if let Some(w) = self.want
            && !w.horizontal() == self.pac.dir.horizontal()
        {
            let (c, r) = self.pac.tile();
            let (cx, cy) = tile_center(c, r);
            let off = if self.pac.dir.horizontal() {
                self.pac.x - cx
            } else {
                self.pac.y - cy
            };
            if off.abs() <= CORNER && self.maze.open(c, r, w, false) {
                self.pac.x = cx;
                self.pac.y = cy;
                self.pac.dir = w;
                self.want = None;
            }
        }
        let maze = self.maze.clone();
        let mut want = self.want;
        let dist = self.pac_speed() * dt;
        let moved = advance(&maze, &mut self.pac, dist, false, &mut |a| {
            let (c, r) = a.tile();
            if let Some(w) = want
                && maze.open(c, r, w, false)
            {
                want = None;
                return Some(w);
            }
            if maze.open(c, r, a.dir, false) {
                Some(a.dir)
            } else {
                None
            }
        });
        self.want = want;
        if moved > 0.0 {
            self.pac_anim += dt;
        }
        // Eat whatever is under Pac-Pac's center.
        let (c, r) = self.pac.tile();
        if c >= 0 && r >= 0 && c < COLS as i32 && r < ROWS as i32 {
            let i = r as usize * COLS + c as usize;
            if !self.eaten[i] {
                match self.maze.tiles[i] {
                    Tile::Dot => {
                        self.eaten[i] = true;
                        self.eat_dot(10, DOT_PAUSE);
                        self.happenings.push(Happening::Chomp);
                    }
                    Tile::Energizer => {
                        self.eaten[i] = true;
                        self.eat_dot(50, ENERGIZER_PAUSE);
                        self.frighten();
                        self.happenings.push(Happening::Energizer);
                    }
                    _ => {}
                }
            }
        }
    }

    fn eat_dot(&mut self, points: i64, pause: f64) {
        self.add_score(points);
        self.dots_eaten += 1;
        self.pac_pause = pause;
        self.no_dot_t = 0.0;
        self.maze_version += 1;
        self.count_dot();
        if self.fruit.is_none() && (self.dots_eaten == 70 || self.dots_eaten == 170) {
            self.spawn_fruit();
        }
        if self.dots_eaten >= self.maze.dot_count {
            self.record_best();
            self.phase = Phase::LevelFreeze {
                t: LEVEL_FREEZE_SECS,
            };
            self.happenings.push(Happening::LevelClear);
        }
    }

    fn add_score(&mut self, points: i64) {
        self.score += points;
        if self.score >= EXTRA_LIFE_AT && !self.extra_life_given {
            self.extra_life_given = true;
            self.lives += 1;
            self.happenings.push(Happening::ExtraLife);
        }
    }

    fn frighten(&mut self) {
        let secs = self.spec.fright_secs;
        self.ghosts_eaten = 0;
        for g in self.ghosts.iter_mut() {
            if g.state == GhostState::Roam {
                g.reverse = true;
            }
            if secs > 0.0 && !matches!(g.state, GhostState::Eyes | GhostState::Entering) {
                g.frightened = true;
            }
        }
        if secs > 0.0 {
            self.fright_t = secs;
        }
    }

    /// The ghost house's dot counters (the arcade's, the arcade's dossier's account of them).
    fn count_dot(&mut self) {
        if let Some(n) = self.global_counter {
            let n = n + 1;
            self.global_counter = Some(n);
            for k in [GhostKind::Pinky, GhostKind::Inky, GhostKind::Clyde] {
                if n == k.global_limit() {
                    let home = self.ghosts[k.index()].state == GhostState::Home;
                    if home {
                        self.release(k);
                    }
                    if k == GhostKind::Clyde {
                        // Clyde's limit resets the global counter whether he left or not,
                        // which is what makes the personal counters count again.
                        self.global_counter = None;
                    }
                }
            }
            return;
        }
        if let Some(k) = self.next_waiting() {
            let g = &mut self.ghosts[k.index()];
            g.dot_counter += 1;
            if g.dot_counter >= k.personal_limit(self.level) {
                self.release(k);
            }
        }
    }

    /// The first ghost still in the house, in the order they leave.
    fn next_waiting(&self) -> Option<GhostKind> {
        [GhostKind::Pinky, GhostKind::Inky, GhostKind::Clyde]
            .into_iter()
            .find(|k| self.ghosts[k.index()].state == GhostState::Home)
    }

    fn release(&mut self, k: GhostKind) {
        let g = &mut self.ghosts[k.index()];
        if g.state == GhostState::Home {
            g.state = GhostState::Leaving;
            if k == GhostKind::Clyde {
                self.elroy_ok = true;
            }
        }
    }

    fn ghost_speed(&self, g: &Ghost) -> f64 {
        let (c, r) = g.actor.tile();
        let frac = match g.state {
            GhostState::Eyes => EYES_SPEED,
            GhostState::Leaving | GhostState::Entering | GhostState::Home => HOUSE_SPEED,
            GhostState::Roam => {
                if self.maze.is_tunnel_slow(c, r) {
                    self.spec.tunnel
                } else if g.frightened {
                    self.spec.ghost_fright
                } else if g.kind == GhostKind::Blinky && self.elroy_ok {
                    let left = self.dots_left();
                    if left <= self.spec.elroy2.0 {
                        self.spec.elroy2.1
                    } else if left <= self.spec.elroy1.0 {
                        self.spec.elroy1.1
                    } else {
                        self.spec.ghost
                    }
                } else {
                    self.spec.ghost
                }
            }
        };
        FULL_SPEED * frac
    }

    /// Ms. Pac-Pac's ghosts wander at random through the first two scatter periods.
    fn random_scatter(&self) -> bool {
        self.mode == Mode::MsPacPac && !self.chase && self.mode_index < 3
    }

    /// The tile ghost `g` is heading for right now.
    fn target(&self, g: &Ghost) -> (i32, i32) {
        if g.state == GhostState::Eyes {
            return HOME_TILE;
        }
        if !self.chase {
            return g.kind.scatter_target();
        }
        let (pc, pr) = self.pac.tile();
        let (dx, dy) = self.pac.dir.step();
        // The arcade's up-bug: looking up, Pinky and Inky also look left.
        let bug = if self.pac.dir == Dir::Up { -1 } else { 0 };
        match g.kind {
            GhostKind::Blinky => (pc, pr),
            GhostKind::Pinky => (pc + 4 * dx + 4 * bug, pr + 4 * dy),
            GhostKind::Inky => {
                let (ax, ay) = (pc + 2 * dx + 2 * bug, pr + 2 * dy);
                let (bc, br) = self.ghosts[0].actor.tile();
                (2 * ax - bc, 2 * ay - br)
            }
            GhostKind::Clyde => {
                let (gc, gr) = g.actor.tile();
                let d2 = (gc - pc).pow(2) + (gr - pr).pow(2);
                if d2 > 64 {
                    (pc, pr)
                } else {
                    g.kind.scatter_target()
                }
            }
        }
    }

    fn step_ghost(&mut self, i: usize, dt: f64) {
        let mut g = self.ghosts[i];
        let speed = self.ghost_speed(&g);
        let dist = speed * dt;
        match g.state {
            GhostState::Home => {
                // Bobbing between three pixels above and below the resting line.
                let (_, dy) = g.actor.dir.unit();
                g.actor.y += dy * dist;
                if g.actor.y > HOUSE_Y + 3.0 {
                    g.actor.y = HOUSE_Y + 3.0;
                    g.actor.dir = Dir::Up;
                } else if g.actor.y < HOUSE_Y - 3.0 {
                    g.actor.y = HOUSE_Y - 3.0;
                    g.actor.dir = Dir::Down;
                }
            }
            GhostState::Leaving => {
                if (g.actor.x - HOUSE_X).abs() > 0.5 {
                    let s = (HOUSE_X - g.actor.x).signum();
                    g.actor.x += s * dist.min((HOUSE_X - g.actor.x).abs());
                    g.actor.dir = if s > 0.0 { Dir::Right } else { Dir::Left };
                } else {
                    g.actor.x = HOUSE_X;
                    g.actor.dir = Dir::Up;
                    g.actor.y -= dist;
                    if g.actor.y <= DOOR_Y {
                        g.actor.y = DOOR_Y;
                        g.state = GhostState::Roam;
                        g.actor.dir = Dir::Left;
                        g.reverse = false;
                    }
                }
            }
            GhostState::Entering => {
                if (g.actor.x - HOUSE_X).abs() > 0.5 && g.actor.y <= DOOR_Y + 0.5 {
                    let s = (HOUSE_X - g.actor.x).signum();
                    g.actor.x += s * dist.min((HOUSE_X - g.actor.x).abs());
                } else if g.actor.y < HOUSE_Y {
                    g.actor.x = HOUSE_X;
                    g.actor.dir = Dir::Down;
                    g.actor.y = (g.actor.y + dist).min(HOUSE_Y);
                } else {
                    let hx = g.kind.home_x();
                    if (g.actor.x - hx).abs() > 0.5 {
                        let s = (hx - g.actor.x).signum();
                        g.actor.x += s * dist.min((hx - g.actor.x).abs());
                        g.actor.dir = if s > 0.0 { Dir::Right } else { Dir::Left };
                    } else {
                        g.actor.x = hx;
                        g.state = GhostState::Leaving;
                        g.frightened = false;
                    }
                }
            }
            GhostState::Roam | GhostState::Eyes => {
                if g.reverse {
                    g.reverse = false;
                    g.actor.dir = g.actor.dir.opposite();
                }
                let eyes = g.state == GhostState::Eyes;
                let target = self.target(&g);
                let random = g.frightened && !eyes || (!eyes && self.random_scatter());
                let maze = self.maze.clone();
                let restrict_up = !random && !eyes;
                let rng = &mut self.rng;
                let snapshot = g;
                let mut arrived_home = false;
                advance(&maze, &mut g.actor, dist, false, &mut |a| {
                    let (c, r) = a.tile();
                    if eyes && (c, r) == HOME_TILE {
                        arrived_home = true;
                        return None;
                    }
                    Some(choose(
                        &maze,
                        a,
                        target,
                        random,
                        restrict_up,
                        rng,
                        snapshot.kind,
                    ))
                });
                if arrived_home {
                    g.state = GhostState::Entering;
                    g.actor.dir = Dir::Right;
                }
            }
        }
        self.ghosts[i] = g;
    }

    fn spawn_fruit(&mut self) {
        self.fruit_spawned += 1;
        let (kind, points) = fruit_for(self.mode, self.level, &mut self.rng);
        let wander = self.mode == Mode::MsPacPac && !self.maze.tunnels.is_empty();
        let actor = if wander {
            let row = self.maze.tunnels[self.rng.below(self.maze.tunnels.len())] as i32;
            let from_left = self.rng.below(2) == 0;
            let (_, cy) = tile_center(0, row);
            Actor {
                x: if from_left {
                    WRAP_LEFT + 1.0
                } else {
                    WRAP_RIGHT - 1.0
                },
                y: cy,
                dir: if from_left { Dir::Right } else { Dir::Left },
            }
        } else {
            Actor {
                x: FRUIT_SPOT.0,
                y: FRUIT_SPOT.1,
                dir: Dir::Left,
            }
        };
        let exit_row = self.maze.tunnels[self.rng.below(self.maze.tunnels.len().max(1))] as i32;
        let exit = if self.rng.below(2) == 0 {
            (-2, exit_row)
        } else {
            (COLS as i32 + 1, exit_row)
        };
        self.fruit = Some(FruitState {
            kind,
            points,
            actor,
            t: FRUIT_SECS + self.rng.unit(),
            wander,
            leaving: false,
            exit,
        });
        self.happenings.push(Happening::FruitAppeared);
    }

    fn step_fruit(&mut self, dt: f64) {
        let Some(mut f) = self.fruit else {
            return;
        };
        if !f.wander {
            f.t -= dt;
            self.fruit = if f.t > 0.0 { Some(f) } else { None };
            return;
        }
        let maze = self.maze.clone();
        let rng = &mut self.rng;
        let target = if f.leaving { f.exit } else { HOME_TILE };
        let mut reached = false;
        let x_before = f.actor.x;
        advance(
            &maze,
            &mut f.actor,
            FULL_SPEED * FRUIT_SPEED * dt,
            false,
            &mut |a| {
                let (c, r) = a.tile();
                if (c, r) == target || (!f.leaving && (c, r) == (HOME_TILE.0 + 1, HOME_TILE.1)) {
                    reached = true;
                }
                // Mostly purposeful, with a wobble so each visit takes a different route.
                let random = rng.below(4) == 0;
                Some(choose(
                    &maze,
                    a,
                    target,
                    random,
                    false,
                    rng,
                    GhostKind::Clyde,
                ))
            },
        );
        if reached && !f.leaving {
            f.leaving = true;
        }
        // Gone once it crosses into the tunnel's hidden tiles (the wrap would bring it back).
        let wrapped = (f.actor.x - x_before).abs() > COLS as f64 * TILE / 2.0;
        if f.leaving && (wrapped || f.actor.x <= WRAP_LEFT + 1.0 || f.actor.x >= WRAP_RIGHT - 1.0) {
            self.fruit = None;
        } else {
            self.fruit = Some(f);
        }
    }

    fn collide(&mut self) {
        if !self.playing() {
            return;
        }
        let pt = self.pac.tile();
        if let Some(f) = self.fruit
            && (f.actor.x - self.pac.x).abs() < TILE
            && (f.actor.y - self.pac.y).abs() < TILE
        {
            self.add_score(f.points);
            self.popups.push(Popup {
                x: f.actor.x,
                y: f.actor.y,
                value: f.points,
                ghost: false,
                t: 2.0,
            });
            self.fruit = None;
            self.happenings.push(Happening::FruitEaten(f.points));
        }
        for i in 0..4 {
            let g = self.ghosts[i];
            if !g.in_play() || g.actor.tile() != pt {
                continue;
            }
            if g.frightened {
                let points = 200 << self.ghosts_eaten.min(3);
                self.ghosts_eaten += 1;
                self.add_score(points);
                let gm = &mut self.ghosts[i];
                gm.state = GhostState::Eyes;
                gm.frightened = false;
                gm.reverse = false;
                self.popups.push(Popup {
                    x: g.actor.x,
                    y: g.actor.y,
                    value: points,
                    ghost: true,
                    t: GHOST_EATEN_SECS,
                });
                self.phase = Phase::GhostEaten {
                    t: GHOST_EATEN_SECS,
                };
                self.happenings.push(Happening::GhostEaten(points));
                return;
            }
            self.phase = Phase::DeathFreeze {
                t: DEATH_FREEZE_SECS,
            };
            return;
        }
    }
}

/// A ghost's choice at a tile center: the open way, not back, nearest `target`; at random when
/// frightened; never up through the arcade's four no-up tiles when `restrict_up`.
fn choose(
    maze: &Maze,
    a: &Actor,
    target: (i32, i32),
    random: bool,
    restrict_up: bool,
    rng: &mut Rng,
    _kind: GhostKind,
) -> Dir {
    let (c, r) = a.tile();
    let back = a.dir.opposite();
    let red_zone = (r == 11 || r == 23) && (12..=15).contains(&c);
    let open = |d: Dir| {
        d != back && maze.open(c, r, d, false) && !(restrict_up && red_zone && d == Dir::Up)
    };
    if random {
        let start = rng.below(4);
        for k in 0..4 {
            let d = Dir::ALL[(start + k) % 4];
            if open(d) {
                return d;
            }
        }
        return back;
    }
    let mut best: Option<(i64, Dir)> = None;
    for d in Dir::ALL {
        if !open(d) {
            continue;
        }
        let (dx, dy) = d.step();
        let (nc, nr) = (c + dx, r + dy);
        let dist = ((nc - target.0) as i64).pow(2) + ((nr - target.1) as i64).pow(2);
        if best.is_none_or(|(bd, _)| dist < bd) {
            best = Some((dist, d));
        }
    }
    best.map(|(_, d)| d).unwrap_or(back)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        let mut g = Game::new(Mode::PacPac, 0x5EED);
        g.happenings.clear();
        g.phase = Phase::Play;
        g.release(GhostKind::Pinky);
        g
    }

    fn drain(g: &mut Game) -> Vec<Happening> {
        std::mem::take(&mut g.happenings)
    }

    #[test]
    fn ready_counts_down_into_play_and_frees_pinky() {
        let mut g = Game::new(Mode::PacPac, 1);
        assert!(matches!(g.phase, Phase::Ready { intro: true, .. }));
        assert_eq!(drain(&mut g), vec![Happening::Intro]);
        g.update(INTRO_SECS + 0.1);
        assert!(g.playing());
        assert!(drain(&mut g).contains(&Happening::Go));
        assert_eq!(g.ghosts[1].state, GhostState::Leaving);
    }

    #[test]
    fn pac_pac_eats_left_from_the_start_and_scores() {
        let mut g = game();
        // From (112, 188) heading left the first dot is under tile 12 at x = 100.
        for _ in 0..30 {
            g.update(1.0 / 60.0);
        }
        assert!(g.pac.x < 104.0, "moved left to {}", g.pac.x);
        assert!(g.dots_eaten >= 1);
        assert_eq!(g.score, 10 * g.dots_eaten as i64);
        assert!(drain(&mut g).contains(&Happening::Chomp));
    }

    #[test]
    fn walls_stop_pac_pac_and_a_queued_turn_is_taken_when_open() {
        let mut g = game();
        g.steer(Dir::Up);
        // Up is a wall at the start; left stays the heading until a turn opens.
        g.update(0.05);
        assert_eq!(g.pac.dir, Dir::Left);
        assert_eq!(g.want, Some(Dir::Up));
        // Reverse is immediate.
        g.steer(Dir::Right);
        g.update(0.01);
        assert_eq!(g.pac.dir, Dir::Right);
        // Run right into the wall beside tile 16 and stop there.
        for _ in 0..200 {
            g.update(1.0 / 60.0);
        }
        let (c, _) = g.pac.tile();
        assert!(!g.maze.open(c, 23, Dir::Right, false));
        let x = g.pac.x;
        g.update(0.1);
        assert_eq!(g.pac.x, x, "stopped at the wall");
    }

    #[test]
    fn energizer_frightens_and_eating_ghosts_doubles() {
        let mut g = game();
        g.ghosts[0].actor = Actor {
            x: 20.0,
            y: 188.0,
            dir: Dir::Left,
        };
        g.frighten();
        assert!(g.ghosts[0].frightened);
        assert_eq!(g.fright_t, 6.0);
        assert!(g.ghosts[0].reverse);
        // Put Pac-Pac on Blinky.
        g.pac = g.ghosts[0].actor;
        g.collide();
        assert_eq!(g.ghosts[0].state, GhostState::Eyes);
        assert_eq!(g.score, 200);
        assert!(matches!(g.phase, Phase::GhostEaten { .. }));
        g.phase = Phase::Play;
        g.ghosts[1].state = GhostState::Roam;
        g.ghosts[1].frightened = true;
        g.ghosts[1].actor = g.pac;
        g.collide();
        assert_eq!(g.score, 600);
        assert!(drain(&mut g).contains(&Happening::GhostEaten(400)));
    }

    #[test]
    fn a_ghost_catches_pac_pac_and_the_round_restarts_with_one_life_fewer() {
        let mut g = game();
        g.ghosts[0].actor = g.pac;
        g.collide();
        assert!(matches!(g.phase, Phase::DeathFreeze { .. }));
        g.update(DEATH_FREEZE_SECS + 0.01);
        assert!(matches!(g.phase, Phase::Death { .. }));
        assert!(drain(&mut g).contains(&Happening::Death));
        g.update(DEATH_FRAME_SECS * 14.0);
        assert!(matches!(g.phase, Phase::Ready { intro: false, .. }));
        assert_eq!(g.lives, START_LIVES - 1);
        assert_eq!(g.global_counter, Some(0));
        assert_eq!(g.pac.x, PAC_START.0);
    }

    #[test]
    fn last_life_ends_the_game() {
        let mut g = game();
        g.lives = 1;
        g.score = 5000;
        g.ghosts[0].actor = g.pac;
        g.collide();
        g.update(DEATH_FREEZE_SECS + 0.01);
        g.update(DEATH_FRAME_SECS * 14.0);
        assert!(matches!(g.phase, Phase::GameOverText { .. }));
        assert!(g.game_over);
        assert_eq!(g.best, 5000);
        g.update(GAME_OVER_TEXT_SECS + 0.1);
        assert_eq!(g.phase, Phase::GameOver);
        assert!(drain(&mut g).contains(&Happening::GameOver));
    }

    #[test]
    fn clearing_the_board_flashes_and_loads_the_next_level() {
        let mut g = game();
        let total = g.maze.dot_count;
        // Every dot but the one just left of Pac-Pac's start, tile (12, 23).
        let last = 23 * COLS + 12;
        for (i, t) in g.maze.tiles.clone().iter().enumerate() {
            if matches!(t, Tile::Dot | Tile::Energizer) && i != last {
                g.eaten[i] = true;
            }
        }
        g.dots_eaten = total - 1;
        for _ in 0..30 {
            g.update(1.0 / 60.0);
            if !g.playing() {
                break;
            }
        }
        assert!(
            matches!(g.phase, Phase::LevelFreeze { .. }),
            "{:?}",
            g.phase
        );
        assert!(drain(&mut g).contains(&Happening::LevelClear));
        g.update(LEVEL_FREEZE_SECS + 0.01);
        assert!(matches!(g.phase, Phase::LevelFlash { .. }));
        g.update(LEVEL_FLASH_SECS + 0.01);
        assert_eq!(g.level, 2);
        assert!(matches!(g.phase, Phase::Ready { .. }));
        assert_eq!(g.dots_eaten, 0);
        assert_eq!(g.history, vec![Fruit::Cherry, Fruit::Strawberry]);
    }

    #[test]
    fn fruit_appears_at_seventy_dots_and_is_worth_its_level() {
        let mut g = game();
        g.dots_eaten = 69;
        g.eat_dot(10, 0.0);
        let f = g.fruit.expect("fruit");
        assert_eq!((f.kind, f.points), (Fruit::Cherry, 100));
        assert!(!f.wander);
        g.pac.x = f.actor.x;
        g.pac.y = f.actor.y;
        g.collide();
        assert!(g.fruit.is_none());
        assert!(drain(&mut g).contains(&Happening::FruitEaten(100)));
        assert_eq!(g.popups.len(), 1);
    }

    #[test]
    fn ms_pac_pac_fruit_wanders_in_from_a_tunnel() {
        let mut g = Game::new(Mode::MsPacPac, 7);
        g.phase = Phase::Play;
        g.dots_eaten = 69;
        g.eat_dot(10, 0.0);
        let f = g.fruit.expect("fruit");
        assert!(f.wander);
        assert!(g.maze.tunnels.contains(&(f.actor.tile().1 as usize)));
        for _ in 0..120 {
            g.step_fruit(1.0 / 60.0);
        }
        let f = g.fruit.expect("still wandering");
        assert!(
            f.actor.x > 0.0 && f.actor.x < 224.0,
            "inside at {}",
            f.actor.x
        );
    }

    #[test]
    fn ghosts_leave_the_house_on_the_dot_counters() {
        let mut g = game();
        assert_eq!(g.ghosts[2].state, GhostState::Home);
        for _ in 0..30 {
            g.count_dot();
        }
        assert_eq!(g.ghosts[2].state, GhostState::Leaving, "Inky at 30 dots");
        assert_eq!(g.ghosts[3].state, GhostState::Home);
        for _ in 0..60 {
            g.count_dot();
        }
        assert_eq!(g.ghosts[3].state, GhostState::Leaving, "Clyde at 60 more");
        // After a death the global counter frees them at 7, 17 and 32.
        g.reset_actors();
        g.global_counter = Some(0);
        for _ in 0..7 {
            g.count_dot();
        }
        assert_eq!(g.ghosts[1].state, GhostState::Leaving);
        for _ in 0..10 {
            g.count_dot();
        }
        assert_eq!(g.ghosts[2].state, GhostState::Leaving);
        for _ in 0..15 {
            g.count_dot();
        }
        assert_eq!(g.ghosts[3].state, GhostState::Leaving);
        assert_eq!(g.global_counter, None);
    }

    #[test]
    fn a_leaving_ghost_reaches_the_door_and_heads_left() {
        let mut g = game();
        for _ in 0..600 {
            g.step_ghost(1, 1.0 / 60.0);
            if g.ghosts[1].state == GhostState::Roam {
                break;
            }
        }
        assert_eq!(g.ghosts[1].state, GhostState::Roam);
        assert_eq!(g.ghosts[1].actor.dir, Dir::Left);
        assert_eq!(g.ghosts[1].actor.y, DOOR_Y);
    }

    #[test]
    fn eyes_go_home_and_come_back_out() {
        let mut g = game();
        g.ghosts[0].state = GhostState::Eyes;
        g.ghosts[0].actor = Actor {
            x: 100.0,
            y: 92.0,
            dir: Dir::Right,
        };
        let mut entered = false;
        for _ in 0..600 {
            g.step_ghost(0, 1.0 / 60.0);
            if g.ghosts[0].state == GhostState::Entering {
                entered = true;
            }
            if entered && g.ghosts[0].state == GhostState::Roam {
                break;
            }
        }
        assert!(entered, "went in");
        assert_eq!(g.ghosts[0].state, GhostState::Roam, "and came out");
    }

    #[test]
    fn scatter_and_chase_alternate_on_the_schedule_and_reverse_the_ghosts() {
        let mut g = game();
        assert!(!g.chase);
        g.ghosts[0].reverse = false;
        g.mode_timer = 6.99;
        g.play(0.02);
        assert!(g.chase);
        g.update(1.0 / 60.0);
        assert_eq!(g.mode_index, 1);
    }

    #[test]
    fn targets_follow_the_dossier() {
        let mut g = game();
        g.chase = true;
        g.pac = Actor {
            x: tile_center(10, 20).0,
            y: tile_center(10, 20).1,
            dir: Dir::Up,
        };
        g.ghosts[0].actor = Actor {
            x: tile_center(4, 20).0,
            y: tile_center(4, 20).1,
            dir: Dir::Left,
        };
        assert_eq!(g.target(&g.ghosts[0]), (10, 20));
        assert_eq!(g.target(&g.ghosts[1]), (6, 16), "Pinky's up-bug");
        assert_eq!(g.target(&g.ghosts[2]), (12, 16), "Inky doubles from Blinky");
        g.ghosts[3].actor = Actor {
            x: tile_center(1, 1).0,
            y: tile_center(1, 1).1,
            dir: Dir::Left,
        };
        assert_eq!(g.target(&g.ghosts[3]), (10, 20), "Clyde far away chases");
        g.ghosts[3].actor = Actor {
            x: tile_center(9, 20).0,
            y: tile_center(9, 20).1,
            dir: Dir::Left,
        };
        assert_eq!(g.target(&g.ghosts[3]), (0, 32), "Clyde up close scatters");
        g.chase = false;
        assert_eq!(g.target(&g.ghosts[0]), (25, -3));
    }

    #[test]
    fn level_tables_match_the_arcade() {
        let l1 = level_spec(1);
        assert_eq!(
            (l1.pac, l1.ghost, l1.fright_secs, l1.flashes),
            (0.80, 0.75, 6.0, 5)
        );
        assert_eq!(l1.elroy1, (20, 0.80));
        let l5 = level_spec(5);
        assert_eq!((l5.pac, l5.ghost, l5.fright_secs), (1.0, 0.95, 2.0));
        assert_eq!(level_spec(17).fright_secs, 0.0);
        assert_eq!(level_spec(21).pac, 0.90);
        assert_eq!(schedule(1)[0], 7.0);
        assert_eq!(schedule(5)[5], 1037.0);
        assert_eq!(maze_index(Mode::MsPacPac, 3), 2);
        assert_eq!(maze_index(Mode::MsPacPac, 14), 3);
        assert_eq!(maze_index(Mode::MsPacPac, 18), 4);
        let mut rng = Rng(3);
        assert_eq!(fruit_for(Mode::PacPac, 13, &mut rng), (Fruit::Key, 5000));
        assert_eq!(
            fruit_for(Mode::MsPacPac, 4, &mut rng),
            (Fruit::Pretzel, 700)
        );
    }

    #[test]
    fn the_tunnel_wraps_both_actors() {
        let mut g = game();
        g.pac = Actor {
            x: 4.0,
            y: 116.0,
            dir: Dir::Left,
        };
        for _ in 0..40 {
            g.step_pac(1.0 / 60.0);
        }
        assert!(g.pac.x > 200.0, "wrapped to {}", g.pac.x);
    }

    #[test]
    fn extra_life_once_at_ten_thousand() {
        let mut g = game();
        g.add_score(9_990);
        assert_eq!(g.lives, START_LIVES);
        g.add_score(10);
        assert_eq!(g.lives, START_LIVES + 1);
        g.add_score(10_000);
        assert_eq!(g.lives, START_LIVES + 1);
        assert!(drain(&mut g).contains(&Happening::ExtraLife));
    }

    #[test]
    fn save_and_restore_keep_the_board() {
        let mut g = Game::new(Mode::MsPacPac, 9);
        g.phase = Phase::Play;
        g.level = 1;
        g.eaten[COLS + 1] = true;
        g.dots_eaten = 1;
        g.score = 1234;
        g.lives = 2;
        let s = g.save_state();
        let json = serde_json::to_string(&s).unwrap();
        let back: SaveState = serde_json::from_str(&json).unwrap();
        let g2 = Game::from_save(back, 9);
        assert_eq!(g2.mode, Mode::MsPacPac);
        assert_eq!(g2.score, 1234);
        assert_eq!(g2.lives, 2);
        assert!(g2.eaten[COLS + 1]);
        assert_eq!(g2.dots_eaten, 1);
        assert!(matches!(g2.phase, Phase::Ready { intro: false, .. }));
        assert!(g2.happenings.is_empty());
    }

    #[test]
    fn sirens_climb_as_the_dots_go() {
        let mut g = game();
        assert_eq!(g.siren_level(), 1);
        g.dots_eaten = 240;
        assert_eq!(g.siren_level(), 5);
    }
}
