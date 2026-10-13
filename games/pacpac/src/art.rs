// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! The arcade's pixels: the 224×288 screen, its 8-pixel tiles, the sprites (Pac-Pac's mouths,
//! the ghosts' skirts and eyes, the fruit, the death burst), the 7×7 capital font, the 3×5 score
//! digits, the mazes, and the wall renderer that turns a tile map into the double-lined walls
//! the arcade drew. Everything is authored on the pixel grid and drawn as filled paths of merged
//! pixel runs, so it scales crisply on every toolkit without an image API.
//!
//! Walls: the arcade painted its maze from 8×8 tiles whose lines run through the middle of each
//! wall tile beside a corridor, with the maze's outer edge and the ghost house carrying a second
//! line at their far side. [`Maze::build`] recovers the same picture from the map alone: the
//! inner line is the boundary of the corridors dilated by half a tile (so it passes through wall
//! tile centers), the outer line is the boundary of the void outside the maze, and every corner
//! of both is rounded. Ms. Pac-Pac's mazes fill the walls between the two lines.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use day_pieces::prelude::*;
use day_spec::FillRule;

use crate::model::Dir;

/// One tile is 8 pixels square.
pub const TILE: f64 = 8.0;
pub const COLS: usize = 28;
pub const ROWS: usize = 31;
/// The arcade screen in pixels: three text rows, the maze, two rows for lives and fruit.
pub const SCREEN_W: f64 = 224.0;
pub const SCREEN_H: f64 = 288.0;
/// Where the maze starts: under the three score rows.
pub const MAZE_TOP: f64 = 24.0;
pub const MAZE_H: f64 = ROWS as f64 * TILE;

// --- palette ------------------------------------------------------------------------------
pub const BLACK: Color = Color::rgb(0.0, 0.0, 0.0);
pub const YELLOW: Color = Color::hex(0xFF_FF_00);
pub const BLINKY: Color = Color::hex(0xFF_00_00);
pub const PINKY: Color = Color::hex(0xFF_B8_FF);
pub const INKY: Color = Color::hex(0x00_FF_FF);
pub const CLYDE: Color = Color::hex(0xFF_B8_51);
pub const FRIGHT: Color = Color::hex(0x21_21_FF);
pub const FRIGHT_FACE: Color = Color::hex(0xFF_B8_FF);
pub const FLASH_BODY: Color = Color::hex(0xDE_DE_FF);
pub const FLASH_FACE: Color = Color::hex(0xFF_00_00);
pub const EYE_WHITE: Color = Color::hex(0xDE_DE_FF);
pub const PUPIL: Color = Color::hex(0x21_21_FF);
pub const WALL_BLUE: Color = Color::hex(0x21_21_DE);
pub const WALL_FLASH: Color = Color::hex(0xDE_DE_FF);
pub const DOT_PINK: Color = Color::hex(0xFF_B8_AE);
pub const DOOR_PINK: Color = Color::hex(0xFF_B8_FF);
pub const TEXT_WHITE: Color = Color::hex(0xDE_DE_FF);
pub const TEXT_CYAN: Color = Color::hex(0x00_FF_FF);
pub const TEXT_RED: Color = Color::hex(0xFF_00_00);
pub const SCORE_CYAN: Color = Color::hex(0x00_FF_FF);
pub const SCORE_PINK: Color = Color::hex(0xFF_B8_FF);
const RED: Color = Color::hex(0xFF_00_00);
const GREEN: Color = Color::hex(0x00_C8_00);
const DARK_GREEN: Color = Color::hex(0x00_80_00);
const WHITE: Color = Color::hex(0xFF_FF_FF);
const BROWN: Color = Color::hex(0xB0_60_20);
const TAN: Color = Color::hex(0xDE_9C_51);
const ORANGE: Color = Color::hex(0xFF_B8_51);
const BLUE: Color = Color::hex(0x21_21_FF);
const BOW: Color = Color::hex(0xFF_00_00);

// ---------------------------------------------------------------------------
// Sprites
// ---------------------------------------------------------------------------

/// A pixel sprite: a palette and, per color, one path of the merged horizontal runs.
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    px: Vec<u8>,
    colors: Vec<Color>,
    paths: Vec<(Color, Shape)>,
}

impl Sprite {
    /// A transparent `w`×`h` sprite.
    pub fn blank(w: usize, h: usize) -> Sprite {
        Sprite {
            w,
            h,
            px: vec![0; w * h],
            colors: Vec::new(),
            paths: Vec::new(),
        }
    }

    /// Parse `rows` with `palette` mapping each character to a color; `.` is transparent, and a
    /// character missing from the palette is too.
    pub fn parse(rows: &[&str], palette: &[(char, Color)]) -> Sprite {
        let h = rows.len();
        let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        let mut s = Sprite::blank(w, h);
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if let Some((_, c)) = palette.iter().find(|(p, _)| *p == ch) {
                    s.set(x, y, *c);
                }
            }
        }
        s
    }

    fn index(&mut self, color: Color) -> u8 {
        if let Some(i) = self.colors.iter().position(|c| *c == color) {
            return i as u8 + 1;
        }
        self.colors.push(color);
        self.colors.len() as u8
    }

    pub fn set(&mut self, x: usize, y: usize, color: Color) {
        if x < self.w && y < self.h {
            let i = self.index(color);
            self.px[y * self.w + x] = i;
        }
    }

    pub fn filled(&self, x: usize, y: usize) -> bool {
        x < self.w && y < self.h && self.px[y * self.w + x] != 0
    }

    /// Paint `other` over this sprite with its top-left at (`x`, `y`).
    pub fn overlay(&mut self, other: &Sprite, x: usize, y: usize) {
        for oy in 0..other.h {
            for ox in 0..other.w {
                let i = other.px[oy * other.w + ox];
                if i != 0 {
                    let c = other.colors[i as usize - 1];
                    self.set(x + ox, y + oy, c);
                }
            }
        }
    }

    /// Build the per-color paths. Call once the pixels are final; drawing an unfinished sprite
    /// draws nothing.
    pub fn finish(mut self) -> Sprite {
        let mut paths = Vec::new();
        for (ci, color) in self.colors.iter().enumerate() {
            let want = ci as u8 + 1;
            let mut pb = PathBuilder::new();
            let mut any = false;
            for y in 0..self.h {
                let mut x = 0;
                while x < self.w {
                    if self.px[y * self.w + x] != want {
                        x += 1;
                        continue;
                    }
                    let start = x;
                    while x < self.w && self.px[y * self.w + x] == want {
                        x += 1;
                    }
                    let (x0, x1, y0, y1) = (start as f64, x as f64, y as f64, y as f64 + 1.0);
                    pb = pb
                        .move_to(Point::new(x0, y0))
                        .line_to(Point::new(x1, y0))
                        .line_to(Point::new(x1, y1))
                        .line_to(Point::new(x0, y1))
                        .close();
                    any = true;
                }
            }
            if any {
                paths.push((*color, pb.build()));
            }
        }
        self.paths = paths;
        self
    }

    /// Draw with the top-left corner at (`x`, `y`) in pixel units.
    pub fn draw(&self, d: &mut Draw, x: f64, y: f64) {
        self.draw_tinted(d, x, y, None);
    }

    /// Draw with every color replaced by `tint` when one is given.
    pub fn draw_tinted(&self, d: &mut Draw, x: f64, y: f64, tint: Option<Color>) {
        d.transformed(Affine::translate(x, y), |d| {
            for (color, shape) in &self.paths {
                d.fill(shape.clone(), tint.unwrap_or(*color));
            }
        });
    }
}

/// Pac-Pac's 13-pixel body, the arcade's circle.
const PAC_BODY: [&str; 13] = [
    "....XXXXX....",
    "..XXXXXXXXX..",
    ".XXXXXXXXXXX.",
    ".XXXXXXXXXXX.",
    "XXXXXXXXXXXXX",
    "XXXXXXXXXXXXX",
    "XXXXXXXXXXXXX",
    "XXXXXXXXXXXXX",
    "XXXXXXXXXXXXX",
    ".XXXXXXXXXXX.",
    ".XXXXXXXXXXX.",
    "..XXXXXXXXX..",
    "....XXXXX....",
];

/// The half-angles of Pac-Pac's three mouth frames: shut, half open, wide open.
const MOUTH: [f64; 3] = [0.0, 30.0, 55.0];

/// Pac-Pac facing `dir` with a mouth of `half_angle` degrees, in a 16×16 cell centered on
/// pixel (8, 8). `ms` adds Ms. Pac-Pac's bow, eye, beauty mark, and lipstick.
fn pacpac_sprite(dir: Dir, half_angle: f64, ms: bool) -> Sprite {
    let mut s = Sprite::blank(16, 16);
    let (fx, fy) = dir.unit();
    // The pixels the mouth took out of the body, for the lipstick along its lower edge.
    let mut mouth: Vec<(usize, usize)> = Vec::new();
    for (y, row) in PAC_BODY.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if ch != 'X' {
                continue;
            }
            let (dx, dy) = (x as f64 - 6.0, y as f64 - 6.0);
            let open = if half_angle > 0.0 {
                if dx == 0.0 && dy == 0.0 {
                    true
                } else {
                    let len = (dx * dx + dy * dy).sqrt();
                    let cos = (dx * fx + dy * fy) / len;
                    cos.clamp(-1.0, 1.0).acos().to_degrees() < half_angle
                }
            } else {
                false
            };
            if open {
                mouth.push((x + 2, y + 2));
            } else {
                s.set(x + 2, y + 2, YELLOW);
            }
        }
    }
    if ms {
        ms_features(&mut s, dir, &mouth);
    }
    s.finish()
}

/// Ms. Pac-Pac's bow (on the back of her head, turning with it), eye, beauty mark and
/// lipstick (the two body pixels at the tip of the lower lip).
fn ms_features(s: &mut Sprite, dir: Dir, mouth: &[(usize, usize)]) {
    let bow = [
        (0, 0),
        (1, 0),
        (3, 0),
        (4, 0),
        (0, 1),
        (4, 1),
        (0, 2),
        (1, 2),
        (3, 2),
        (4, 2),
    ];
    let knot = [(1, 1), (2, 1), (3, 1)];
    // The bow's origin, the eye, the mark, and which side of the mouth the lower lip is.
    let (bx, by) = match dir {
        Dir::Right => (1, 0),
        Dir::Left => (10, 0),
        Dir::Up => (0, 11),
        Dir::Down => (10, 0),
    };
    let (eye, mark) = match dir {
        Dir::Right => ((9, 5), (11, 10)),
        Dir::Left => ((6, 5), (4, 10)),
        Dir::Up => ((10, 7), (3, 6)),
        Dir::Down => ((10, 8), (3, 9)),
    };
    let lip: (i32, i32) = match dir {
        Dir::Right | Dir::Left => (0, 1),
        Dir::Up => (-1, 0),
        Dir::Down => (1, 0),
    };
    for (x, y) in bow {
        s.set(bx + x, by + y, BOW);
    }
    for (x, y) in knot {
        s.set(bx + x, by + y, BLUE);
    }
    if s.filled(eye.0, eye.1) {
        s.set(eye.0, eye.1, BLUE);
        if eye.1 > 0 && s.filled(eye.0, eye.1 - 1) {
            s.set(eye.0, eye.1 - 1, BLACK);
        }
    }
    if s.filled(mark.0, mark.1) {
        s.set(mark.0, mark.1, BLACK);
    }
    // Lipstick: of the body pixels just past the mouth's lower edge, the two nearest the
    // front of the face.
    let (fx, fy) = dir.step();
    let mut lips: Vec<(i32, usize, usize)> = Vec::new();
    for &(mx, my) in mouth {
        let (x, y) = (mx as i32 + lip.0, my as i32 + lip.1);
        if x < 0 || y < 0 {
            continue;
        }
        let (x, y) = (x as usize, y as usize);
        if s.filled(x, y) && !lips.iter().any(|l| l.1 == x && l.2 == y) {
            lips.push((x as i32 * fx + y as i32 * fy, x, y));
        }
    }
    lips.sort_by_key(|l| std::cmp::Reverse(l.0));
    for (_, x, y) in lips.into_iter().take(2) {
        s.set(x, y, RED);
    }
}

/// The ghost body, 14 pixels wide, two skirt frames.
const GHOST_TOP: [&str; 12] = [
    ".....XXXX.....",
    "...XXXXXXXX...",
    "..XXXXXXXXXX..",
    ".XXXXXXXXXXXX.",
    ".XXXXXXXXXXXX.",
    "XXXXXXXXXXXXXX",
    "XXXXXXXXXXXXXX",
    "XXXXXXXXXXXXXX",
    "XXXXXXXXXXXXXX",
    "XXXXXXXXXXXXXX",
    "XXXXXXXXXXXXXX",
    "XXXXXXXXXXXXXX",
];
const GHOST_SKIRT: [[&str; 2]; 2] = [
    ["XXX..XXXX..XXX", "XX....XX....XX"],
    ["X.XX.XXXX.XX.X", "X..X..XX..X..X"],
];

fn ghost_body(color: Color, frame: usize) -> Sprite {
    let mut rows: Vec<&str> = GHOST_TOP.to_vec();
    rows.extend_from_slice(&GHOST_SKIRT[frame]);
    let body = Sprite::parse(&rows, &[('X', color)]);
    let mut s = Sprite::blank(16, 16);
    s.overlay(&body, 1, 1);
    s
}

/// The eyes of a ghost looking `dir`: whites and pupils, in the 16×16 cell.
fn ghost_eyes(dir: Dir) -> Sprite {
    let white = Sprite::parse(
        &[".XX.", "XXXX", "XXXX", "XXXX", ".XX."],
        &[('X', EYE_WHITE)],
    );
    let (shift_x, shift_y, px, py): (i32, i32, usize, usize) = match dir {
        Dir::Right => (1, 0, 2, 2),
        Dir::Left => (-1, 0, 0, 2),
        Dir::Up => (0, -2, 1, 0),
        Dir::Down => (0, 1, 1, 3),
    };
    let mut s = Sprite::blank(16, 16);
    for ex in [3i32, 9] {
        let x = (ex + shift_x) as usize;
        let y = (5 + shift_y) as usize;
        s.overlay(&white, x, y);
        for dy in 0..2 {
            for dx in 0..2 {
                s.set(x + px + dx, y + py + dy, PUPIL);
            }
        }
    }
    s.finish()
}

/// The frightened face: two eyes and a wavy mouth, in `face`.
fn fright_face(face: Color) -> Sprite {
    let rows = [
        "..............",
        "..............",
        "..............",
        "..............",
        "...XX....XX...",
        "...XX....XX...",
        "..............",
        "..............",
        "..............",
        ".X.XX.XX.XX.X.",
        "X.X..X..X..X.X",
    ];
    let f = Sprite::parse(&rows, &[('X', face)]);
    let mut s = Sprite::blank(16, 16);
    s.overlay(&f, 1, 1);
    s.finish()
}

/// Pac-Pac's death: the mouth opens past the top until nothing is left, then a burst.
fn death_frames(ms: bool) -> Vec<Sprite> {
    let mut v: Vec<Sprite> = (0..11)
        .map(|i| pacpac_sprite(Dir::Up, 30.0 + i as f64 * 15.0, false))
        .collect();
    if ms {
        // Ms. Pac-Pac spins instead: her half-open face turning through the four directions.
        v = [Dir::Up, Dir::Left, Dir::Down, Dir::Right]
            .iter()
            .cycle()
            .take(12)
            .map(|d| pacpac_sprite(*d, 30.0, true))
            .collect();
    }
    let burst = [
        ".......X.......",
        "......X.X......",
        "....X.....X....",
        "...............",
        "..X.........X..",
        "...............",
        ".X...........X.",
        "X.............X",
        ".X...........X.",
        "...............",
        "..X.........X..",
        "...............",
        "....X.....X....",
        "......X.X......",
        ".......X.......",
    ];
    let b = Sprite::parse(&burst, &[('X', YELLOW)]);
    let mut s = Sprite::blank(16, 16);
    s.overlay(&b, 1, 1);
    v.push(s.finish());
    v
}

/// The fruit, 12×12 to 14×14 pixels in the 16×16 cell.
pub fn fruit_sprite(kind: Fruit) -> Sprite {
    let (rows, palette): (&[&str], &[(char, Color)]) = match kind {
        Fruit::Cherry => (
            &[
                "..........GG",
                ".........GG.",
                "........GG..",
                ".......G....",
                "..GG..G.....",
                ".RRRRG......",
                "RRRRRR.RRR..",
                "RWRRRRRRRRR.",
                "RWRRRRRWRRRR",
                ".RRRR.RWRRRR",
                "......RRRRRR",
                ".......RRRR.",
            ],
            &[('G', DARK_GREEN), ('R', RED), ('W', WHITE)],
        ),
        Fruit::Strawberry => (
            &[
                ".....GG.....",
                "..GGGGGGGG..",
                ".RRRGGGGRRR.",
                "RRRWRRRRWRRR",
                "RRRRRRRRRRRR",
                "RWRRRWRRRWRR",
                "RRRRRRRRRRRR",
                ".RRWRRRWRRR.",
                ".RRRRRRRRRR.",
                "..RRRWRRRR..",
                "...RRRRRR...",
                ".....RR.....",
            ],
            &[('G', GREEN), ('R', RED), ('W', WHITE)],
        ),
        Fruit::Orange => (
            &[
                "......GG....",
                ".....GG.....",
                "....BB......",
                "..OOOBOOO...",
                ".OOOOOOOOO..",
                "OOOOOOOOOOO.",
                "OOOOOOOOOOO.",
                "OOOOOOOOOOO.",
                "OOOOOOOOOOO.",
                ".OOOOOOOOO..",
                "..OOOOOOO...",
                "....OOO.....",
            ],
            &[('G', GREEN), ('B', BROWN), ('O', ORANGE)],
        ),
        Fruit::Apple => (
            &[
                ".....BB.....",
                "....GBB.....",
                "..RRRBRRR...",
                ".RRRRRRRRR..",
                "RRRRRRRRRRR.",
                "RWRRRRRRRRR.",
                "RWRRRRRRRRR.",
                "RRRRRRRRRRR.",
                "RRRRRRRRRRR.",
                ".RRRRRRRRR..",
                ".RRRR.RRRR..",
                "..RR...RR...",
            ],
            &[('B', BROWN), ('G', GREEN), ('R', RED), ('W', WHITE)],
        ),
        Fruit::Melon => (
            &[
                "......BB....",
                ".....BB.....",
                "..GGGDGGG...",
                ".GGDGGGDGG..",
                "GGGDGGGDGGG.",
                "GDGGGDGGGDG.",
                "GDGGGDGGGDG.",
                "GGGDGGGDGGG.",
                "GGGDGGGDGGG.",
                ".GGGGGGGGG..",
                "..GGGGGGG...",
                "....GGG.....",
            ],
            &[('B', BROWN), ('G', GREEN), ('D', DARK_GREEN)],
        ),
        Fruit::Galaxian => (
            &[
                "......R.....",
                ".....RRR....",
                ".....RRR....",
                "Y...RRRRR..Y",
                "YY..RRRRR.YY",
                "YYY.BRRRB.YY",
                "YYYYBBRBBYYY",
                ".YYYBBBBBYY.",
                "..YYBBBBBY..",
                "...YBBBBB...",
                "....B.B.B...",
                ".....B.B....",
            ],
            &[('R', RED), ('Y', YELLOW), ('B', BLUE)],
        ),
        Fruit::Bell => (
            &[
                ".....YY.....",
                "....YYYY....",
                "...YYYYYY...",
                "...YYYYYY...",
                "..YYYYYYYY..",
                "..YYYYYYYY..",
                ".YYYYYYYYYY.",
                ".YYYYYYYYYY.",
                "YYYYYYYYYYYY",
                "WWWWWWWWWWWW",
                ".....BB.....",
                "....BBBB....",
            ],
            &[('Y', YELLOW), ('W', WHITE), ('B', BLUE)],
        ),
        Fruit::Key => (
            &[
                "....CCCC....",
                "...CC..CC...",
                "...CC..CC...",
                "....CCCC....",
                ".....CC.....",
                ".....CC.....",
                ".....CCCC...",
                ".....CC.....",
                ".....CCCC...",
                ".....CC.....",
                ".....CCC....",
                ".....CC.....",
            ],
            &[('C', INKY)],
        ),
        Fruit::Pretzel => (
            &[
                "..BBB..BBB..",
                ".BTTTB.BTTB.",
                ".BT.BBBB.TB.",
                ".BT.BTTB.TB.",
                "..BBTBBTBB..",
                "..BTTBBTTB..",
                ".BTBB..BBTB.",
                "BTTB....BTTB",
                "BTTBB..BBTTB",
                ".BTTBBBBTTB.",
                "..BBTTTTBB..",
                "....BBBB....",
            ],
            &[('B', BROWN), ('T', TAN)],
        ),
        Fruit::Pear => (
            &[
                ".....BB.....",
                ".....BB.....",
                "....GGGG....",
                "....GGGG....",
                "...GGGGGG...",
                "...GGGGGG...",
                "..GGGGGGGG..",
                ".GYGGGGGGGG.",
                ".GYGGGGGGGG.",
                ".GGGGGGGGGG.",
                "..GGGGGGGG..",
                "...GGGGGG...",
            ],
            &[('B', BROWN), ('G', GREEN), ('Y', YELLOW)],
        ),
        Fruit::Banana => (
            &[
                "..........BB",
                ".........YYB",
                "........YYY.",
                ".......YYYY.",
                "......YYYY..",
                ".....YYYY...",
                "....YYYY....",
                "...YYYY.....",
                "..YYYY......",
                "BYYYY.......",
                "BYYY........",
                ".BB.........",
            ],
            &[('B', BROWN), ('Y', YELLOW)],
        ),
    };
    let f = Sprite::parse(rows, palette);
    let mut s = Sprite::blank(16, 16);
    s.overlay(&f, 2, 2);
    s.finish()
}

/// The bonus items of both games.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Fruit {
    Cherry,
    Strawberry,
    Orange,
    Apple,
    Melon,
    Galaxian,
    Bell,
    Key,
    Pretzel,
    Pear,
    Banana,
}

impl Fruit {
    pub const ALL: [Fruit; 11] = [
        Fruit::Cherry,
        Fruit::Strawberry,
        Fruit::Orange,
        Fruit::Apple,
        Fruit::Melon,
        Fruit::Galaxian,
        Fruit::Bell,
        Fruit::Key,
        Fruit::Pretzel,
        Fruit::Pear,
        Fruit::Banana,
    ];
    fn index(self) -> usize {
        Fruit::ALL.iter().position(|f| *f == self).unwrap_or(0)
    }
}

const ENERGIZER: [&str; 8] = [
    "..XXXX..", ".XXXXXX.", "XXXXXXXX", "XXXXXXXX", "XXXXXXXX", "XXXXXXXX", ".XXXXXX.", "..XXXX..",
];

/// The 7×7 capitals, digits and punctuation of the arcade's font, in 8-pixel cells.
const FONT: &[(char, [&str; 7])] = &[
    (
        'A',
        [
            ".#####.", "##...##", "##...##", "#######", "##...##", "##...##", "##...##",
        ],
    ),
    (
        'B',
        [
            "######.", "##...##", "##...##", "######.", "##...##", "##...##", "######.",
        ],
    ),
    (
        'C',
        [
            ".#####.", "##...##", "##.....", "##.....", "##.....", "##...##", ".#####.",
        ],
    ),
    (
        'D',
        [
            "#####..", "##..##.", "##...##", "##...##", "##...##", "##..##.", "#####..",
        ],
    ),
    (
        'E',
        [
            "#######", "##.....", "##.....", "#####..", "##.....", "##.....", "#######",
        ],
    ),
    (
        'F',
        [
            "#######", "##.....", "##.....", "#####..", "##.....", "##.....", "##.....",
        ],
    ),
    (
        'G',
        [
            ".#####.", "##...##", "##.....", "##.####", "##...##", "##...##", ".#####.",
        ],
    ),
    (
        'H',
        [
            "##...##", "##...##", "##...##", "#######", "##...##", "##...##", "##...##",
        ],
    ),
    (
        'I',
        [
            ".#####.", "..##...", "..##...", "..##...", "..##...", "..##...", ".#####.",
        ],
    ),
    (
        'J',
        [
            "..#####", "....##.", "....##.", "....##.", "##..##.", "##..##.", ".####..",
        ],
    ),
    (
        'K',
        [
            "##...##", "##..##.", "##.##..", "####...", "##.##..", "##..##.", "##...##",
        ],
    ),
    (
        'L',
        [
            "##.....", "##.....", "##.....", "##.....", "##.....", "##.....", "#######",
        ],
    ),
    (
        'M',
        [
            "##...##", "###.###", "#######", "##.#.##", "##...##", "##...##", "##...##",
        ],
    ),
    (
        'N',
        [
            "##...##", "###..##", "####.##", "##.####", "##..###", "##...##", "##...##",
        ],
    ),
    (
        'O',
        [
            ".#####.", "##...##", "##...##", "##...##", "##...##", "##...##", ".#####.",
        ],
    ),
    (
        'P',
        [
            "######.", "##...##", "##...##", "######.", "##.....", "##.....", "##.....",
        ],
    ),
    (
        'Q',
        [
            ".#####.", "##...##", "##...##", "##...##", "##.#.##", "##..##.", ".####.#",
        ],
    ),
    (
        'R',
        [
            "######.", "##...##", "##...##", "######.", "##.##..", "##..##.", "##...##",
        ],
    ),
    (
        'S',
        [
            ".#####.", "##...##", "##.....", ".#####.", ".....##", "##...##", ".#####.",
        ],
    ),
    (
        'T',
        [
            "#######", "..##...", "..##...", "..##...", "..##...", "..##...", "..##...",
        ],
    ),
    (
        'U',
        [
            "##...##", "##...##", "##...##", "##...##", "##...##", "##...##", ".#####.",
        ],
    ),
    (
        'V',
        [
            "##...##", "##...##", "##...##", "##...##", ".##.##.", ".##.##.", "..###..",
        ],
    ),
    (
        'W',
        [
            "##...##", "##...##", "##...##", "##.#.##", "#######", "###.###", "##...##",
        ],
    ),
    (
        'X',
        [
            "##...##", "##...##", ".##.##.", "..###..", ".##.##.", "##...##", "##...##",
        ],
    ),
    (
        'Y',
        [
            "##...##", "##...##", ".##.##.", "..###..", "..##...", "..##...", "..##...",
        ],
    ),
    (
        'Z',
        [
            "#######", ".....##", "....##.", "..###..", ".##....", "##.....", "#######",
        ],
    ),
    (
        '0',
        [
            ".#####.", "##...##", "##..###", "##.#.##", "###..##", "##...##", ".#####.",
        ],
    ),
    (
        '1',
        [
            "..##...", ".###...", "..##...", "..##...", "..##...", "..##...", "######.",
        ],
    ),
    (
        '2',
        [
            ".#####.", "##...##", ".....##", "..####.", ".##....", "##.....", "#######",
        ],
    ),
    (
        '3',
        [
            ".#####.", "##...##", ".....##", "..####.", ".....##", "##...##", ".#####.",
        ],
    ),
    (
        '4',
        [
            "....##.", "...###.", "..####.", ".##.##.", "#######", "....##.", "....##.",
        ],
    ),
    (
        '5',
        [
            "#######", "##.....", "##.....", "######.", ".....##", "##...##", ".#####.",
        ],
    ),
    (
        '6',
        [
            ".#####.", "##...##", "##.....", "######.", "##...##", "##...##", ".#####.",
        ],
    ),
    (
        '7',
        [
            "#######", ".....##", "....##.", "...##..", "..##...", "..##...", "..##...",
        ],
    ),
    (
        '8',
        [
            ".#####.", "##...##", "##...##", ".#####.", "##...##", "##...##", ".#####.",
        ],
    ),
    (
        '9',
        [
            ".#####.", "##...##", "##...##", ".######", ".....##", "##...##", ".#####.",
        ],
    ),
    (
        '!',
        [
            "..##...", "..##...", "..##...", "..##...", "..##...", ".......", "..##...",
        ],
    ),
    (
        '-',
        [
            ".......", ".......", ".......", "#######", ".......", ".......", ".......",
        ],
    ),
    (
        '.',
        [
            ".......", ".......", ".......", ".......", ".......", "..##...", "..##...",
        ],
    ),
    (
        '/',
        [
            ".....##", "....##.", "...##..", "..##...", ".##....", "##.....", ".......",
        ],
    ),
];

/// The 3×5 digits of the score sprites.
const MINI: [[&str; 5]; 10] = [
    ["###", "#.#", "#.#", "#.#", "###"],
    [".#.", "##.", ".#.", ".#.", "###"],
    ["###", "..#", "###", "#..", "###"],
    ["###", "..#", "###", "..#", "###"],
    ["#.#", "#.#", "###", "..#", "..#"],
    ["###", "#..", "###", "..#", "###"],
    ["###", "#..", "###", "#.#", "###"],
    ["###", "..#", "..#", "..#", "..#"],
    ["###", "#.#", "###", "#.#", "###"],
    ["###", "#.#", "###", "..#", "###"],
];

/// Every sprite the game draws, built once per process.
pub struct Sheet {
    /// `pac[dir][frame]`: frame 0 shut, 1 half open, 2 wide open.
    pub pac: Vec<Vec<Sprite>>,
    pub ms_pac: Vec<Vec<Sprite>>,
    pub pac_death: Vec<Sprite>,
    pub ms_pac_death: Vec<Sprite>,
    /// `ghost[kind][frame]`: the body in the kind's color.
    pub ghost: Vec<Vec<Sprite>>,
    pub eyes: Vec<Sprite>,
    pub fright: Vec<Sprite>,
    pub fright_face: Sprite,
    pub flash: Vec<Sprite>,
    pub flash_face: Sprite,
    pub fruit: Vec<Sprite>,
    pub energizer: Sprite,
    /// The life counter's Pac-Pac, facing left with its mouth half open.
    pub life: Sprite,
    pub ms_life: Sprite,
    glyphs: BTreeMap<char, Shape>,
    mini: Vec<Shape>,
}

thread_local! {
    static SHEET: RefCell<Option<Rc<Sheet>>> = const { RefCell::new(None) };
}

/// The process-wide sprite sheet.
pub fn sheet() -> Rc<Sheet> {
    SHEET.with(|s| {
        s.borrow_mut()
            .get_or_insert_with(|| Rc::new(Sheet::build()))
            .clone()
    })
}

impl Sheet {
    fn build() -> Sheet {
        let dirs = [Dir::Right, Dir::Left, Dir::Up, Dir::Down];
        let faces = |ms: bool| -> Vec<Vec<Sprite>> {
            dirs.iter()
                .map(|d| MOUTH.iter().map(|a| pacpac_sprite(*d, *a, ms)).collect())
                .collect()
        };
        let ghost = [BLINKY, PINKY, INKY, CLYDE]
            .iter()
            .map(|c| (0..2).map(|f| ghost_body(*c, f).finish()).collect())
            .collect();
        let mut glyphs = BTreeMap::new();
        for (ch, rows) in FONT {
            let s = Sprite::parse(rows, &[('#', WHITE)]).finish();
            if let Some((_, shape)) = s.paths.into_iter().next() {
                glyphs.insert(*ch, shape);
            }
        }
        let mini = MINI
            .iter()
            .map(|rows| {
                let s = Sprite::parse(rows, &[('#', WHITE)]).finish();
                s.paths
                    .into_iter()
                    .next()
                    .map(|(_, shape)| shape)
                    .unwrap_or_else(|| PathBuilder::new().build())
            })
            .collect();
        Sheet {
            pac: faces(false),
            ms_pac: faces(true),
            pac_death: death_frames(false),
            ms_pac_death: death_frames(true),
            ghost,
            eyes: dirs.iter().map(|d| ghost_eyes(*d)).collect(),
            fright: (0..2).map(|f| ghost_body(FRIGHT, f).finish()).collect(),
            fright_face: fright_face(FRIGHT_FACE),
            flash: (0..2).map(|f| ghost_body(FLASH_BODY, f).finish()).collect(),
            flash_face: fright_face(FLASH_FACE),
            fruit: Fruit::ALL.iter().map(|f| fruit_sprite(*f)).collect(),
            energizer: Sprite::parse(&ENERGIZER, &[('X', DOT_PINK)]).finish(),
            life: pacpac_sprite(Dir::Left, 30.0, false),
            ms_life: pacpac_sprite(Dir::Left, 30.0, true),
            glyphs,
            mini,
        }
    }

    pub fn fruit(&self, kind: Fruit) -> &Sprite {
        &self.fruit[kind.index()]
    }

    /// Draw `text` in the arcade font, its first cell's top-left at (`x`, `y`). Lowercase is
    /// drawn as capitals; a character the font lacks is a space.
    pub fn text(&self, d: &mut Draw, text: &str, x: f64, y: f64, color: Color) {
        let mut cx = x;
        for ch in text.chars() {
            if let Some(shape) = self.glyphs.get(&ch.to_ascii_uppercase()) {
                d.transformed(Affine::translate(cx, y), |d| d.fill(shape.clone(), color));
            }
            cx += TILE;
        }
    }

    /// `text` right-aligned so its last cell ends at `right`.
    pub fn text_right(&self, d: &mut Draw, text: &str, right: f64, y: f64, color: Color) {
        let w = text.chars().count() as f64 * TILE;
        self.text(d, text, right - w, y, color);
    }

    /// A score popup in the 3×5 digits, centered on `cx` with its top at `y`.
    pub fn mini_number(&self, d: &mut Draw, n: i64, cx: f64, y: f64, color: Color) {
        let digits: Vec<usize> = n
            .to_string()
            .bytes()
            .filter(|b| b.is_ascii_digit())
            .map(|b| (b - b'0') as usize)
            .collect();
        let w = digits.len() as f64 * 4.0 - 1.0;
        let mut x = (cx - w / 2.0).round();
        for dgt in digits {
            d.transformed(Affine::translate(x, y), |d| {
                d.fill(self.mini[dgt].clone(), color)
            });
            x += 4.0;
        }
    }
}

// ---------------------------------------------------------------------------
// Mazes
// ---------------------------------------------------------------------------

/// What a map character means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Wall,
    Dot,
    Energizer,
    /// A corridor with nothing to eat: the tunnels, the ring around the ghost house.
    Path,
    /// Outside the maze.
    Void,
    /// The ghost house door.
    Door,
    /// Inside the ghost house.
    House,
}

impl Tile {
    fn from_char(c: char) -> Tile {
        match c {
            '#' => Tile::Wall,
            '.' => Tile::Dot,
            'o' => Tile::Energizer,
            ' ' => Tile::Path,
            '=' => Tile::Door,
            'H' => Tile::House,
            _ => Tile::Void,
        }
    }
    pub fn walkable(self) -> bool {
        matches!(self, Tile::Dot | Tile::Energizer | Tile::Path)
    }
    fn void(self) -> bool {
        matches!(self, Tile::Void | Tile::House)
    }
}

/// A maze as authored: its map and its colors.
pub struct MazeDef {
    pub rows: [&'static str; ROWS],
    /// The wall lines.
    pub wall: Color,
    /// Ms. Pac-Pac's mazes fill the walls between their lines; Pac-Pac's are lines alone.
    pub fill: Option<Color>,
    pub dot: Color,
    /// How far into the maze a tunnel's slow stretch reaches, in tiles from the edge.
    pub tunnel_slow: usize,
}

/// Pac-Pac's maze, the one everyone knows.
pub const PACPAC: MazeDef = MazeDef {
    rows: [
        "############################",
        "#............##............#",
        "#.####.#####.##.#####.####.#",
        "#o####.#####.##.#####.####o#",
        "#.####.#####.##.#####.####.#",
        "#..........................#",
        "#.####.##.########.##.####.#",
        "#.####.##.########.##.####.#",
        "#......##....##....##......#",
        "######.##### ## #####.######",
        "~~~~~#.##### ## #####.#~~~~~",
        "~~~~~#.##          ##.#~~~~~",
        "~~~~~#.## ###==### ##.#~~~~~",
        "######.## #HHHHHH# ##.######",
        "      .   #HHHHHH#   .      ",
        "######.## #HHHHHH# ##.######",
        "~~~~~#.## ######## ##.#~~~~~",
        "~~~~~#.##          ##.#~~~~~",
        "~~~~~#.## ######## ##.#~~~~~",
        "######.## ######## ##.######",
        "#............##............#",
        "#.####.#####.##.#####.####.#",
        "#.####.#####.##.#####.####.#",
        "#o..##.......  .......##..o#",
        "###.##.##.########.##.##.###",
        "###.##.##.########.##.##.###",
        "#......##....##....##......#",
        "#.##########.##.##########.#",
        "#.##########.##.##########.#",
        "#..........................#",
        "############################",
    ],
    wall: WALL_BLUE,
    fill: None,
    dot: DOT_PINK,
    tunnel_slow: 6,
};

/// Ms. Pac-Pac's four mazes, in the order the levels visit them. Their layouts are original,
/// in the arcade's idiom: the same ghost house and start, tunnels on new rows, four energizers.
pub const MS_MAZES: [MazeDef; 4] = [
    MazeDef {
        rows: [
            "############################",
            "#......##..........##......#",
            "#o####.##.########.##.####o#",
            "#.####.##.########.##.####.#",
            "#..........................#",
            "#.##.####.##.##.##.####.##.#",
            "#.##.####.##.##.##.####.##.#",
            "#.##......##.##.##......##.#",
            "#.####.#####.##.#####.####.#",
            " .####.##### ## #####.####. ",
            "#.####.##### ## #####.####.#",
            "#.####.##          ##.####.#",
            "#.####.## ###==### ##.####.#",
            "#......## #HHHHHH# ##......#",
            "#.####.   #HHHHHH#   .####.#",
            "#.####.## #HHHHHH# ##.####.#",
            "#.####.## ######## ##.####.#",
            " ......##          ##...... ",
            "#.####.## ######## ##.####.#",
            "#.####.## ######## ##.####.#",
            "#......##....##....##......#",
            "#.####.##.##.##.##.##.####.#",
            "#.####.##.##.##.##.##.####.#",
            "#o........##.  .##........o#",
            "###.###.############.###.###",
            "###.###.############.###.###",
            "#............##............#",
            "#.##########.##.##########.#",
            "#.##########.##.##########.#",
            "#..........................#",
            "############################",
        ],
        wall: Color::hex(0xFF_B8_DE),
        fill: Some(Color::hex(0x7E_28_55)),
        dot: Color::hex(0xDE_DE_FF),
        tunnel_slow: 3,
    },
    MazeDef {
        rows: [
            "############################",
            "#............##............#",
            "#.####.#####.##.#####.####.#",
            "#o####.#####.##.#####.####o#",
            "#......##..........##......#",
            "#.####.##.########.##.####.#",
            " .####.##.########.##.####. ",
            "#.####.##.##....##.##.####.#",
            "#......##.##.##.##.##......#",
            "#.####.##.##.##.##.##.####.#",
            "#.####.##.##.##.##.##.####.#",
            "#......##          ##......#",
            "###.##.## ###==### ##.##.###",
            "###.##.## #HHHHHH# ##.##.###",
            "#...##.   #HHHHHH#   .##...#",
            "#.####.## #HHHHHH# ##.####.#",
            "#.####.## ######## ##.####.#",
            "#......##          ##......#",
            "#.####.## ######## ##.####.#",
            "#.####.## ######## ##.####.#",
            "#......##....##....##......#",
            "#.####.##.##.##.##.##.####.#",
            "#.####.##.##.##.##.##.####.#",
            "#o........##.  .##........o#",
            "#.####.##############.####.#",
            " .####.##############.####. ",
            "#............##............#",
            "#.##########.##.##########.#",
            "#.##########.##.##########.#",
            "#..........................#",
            "############################",
        ],
        wall: Color::hex(0x47_B8_FF),
        fill: Some(Color::hex(0x1C_3A_6E)),
        dot: Color::hex(0xFF_FF_00),
        tunnel_slow: 3,
    },
    MazeDef {
        rows: [
            "############################",
            "#............##............#",
            "#.##.#######.##.#######.##.#",
            "#o##.#######.##.#######.##o#",
            "#.##.#######.##.#######.##.#",
            "#..........................#",
            "#.####.##.########.##.####.#",
            "#.####.##.########.##.####.#",
            "#......##....##....##......#",
            "######.##### ## #####.######",
            "~~~~~#.##### ## #####.#~~~~~",
            "~~~~~#.##          ##.#~~~~~",
            "~~~~~#.## ###==### ##.#~~~~~",
            "######.## #HHHHHH# ##.######",
            "      .   #HHHHHH#   .      ",
            "######.## #HHHHHH# ##.######",
            "~~~~~#.## ######## ##.#~~~~~",
            "~~~~~#.##          ##.#~~~~~",
            "~~~~~#.## ######## ##.#~~~~~",
            "######.## ######## ##.######",
            "#............##............#",
            "#.####.#####.##.#####.####.#",
            "#.####.#####.##.#####.####.#",
            "#o...........  ...........o#",
            "###.####.##########.####.###",
            "###.####.##########.####.###",
            "#............##............#",
            "#.##########.##.##########.#",
            "#.##########.##.##########.#",
            "#..........................#",
            "############################",
        ],
        wall: Color::hex(0xDE_97_51),
        fill: Some(Color::hex(0x6E_3C_14)),
        dot: Color::hex(0xFF_00_00),
        tunnel_slow: 6,
    },
    MazeDef {
        rows: [
            "############################",
            "#......##..........##......#",
            "#o####.##.########.##.####o#",
            "#.####.##.########.##.####.#",
            "#..........................#",
            "#.##.###.###.##.###.###.##.#",
            "#.##.###.###.##.###.###.##.#",
            "#.##.....###.##.###.....##.#",
            "#.####.#####.##.#####.####.#",
            " .####.##### ## #####.####. ",
            "#.####.##### ## #####.####.#",
            "#.####.##          ##.####.#",
            "#.####.## ###==### ##.####.#",
            "#......## #HHHHHH# ##......#",
            "#.####.   #HHHHHH#   .####.#",
            "#.####.## #HHHHHH# ##.####.#",
            "#.####.## ######## ##.####.#",
            " ......##          ##...... ",
            "#.####.## ######## ##.####.#",
            "#.####.## ######## ##.####.#",
            "#......##....##....##......#",
            "#.####.##.##.##.##.##.####.#",
            "#.####.##.##.##.##.##.####.#",
            "#o........##.  .##........o#",
            "###.####.##########.####.###",
            "###.####.##########.####.###",
            "#............##............#",
            "#.##########.##.##########.#",
            "#.##########.##.##########.#",
            "#..........................#",
            "############################",
        ],
        wall: Color::hex(0x21_21_FF),
        fill: Some(Color::hex(0x10_10_60)),
        dot: Color::hex(0xDE_DE_FF),
        tunnel_slow: 3,
    },
];

/// A built maze: its tiles, where things are, and the wall picture.
pub struct Maze {
    pub tiles: Vec<Tile>,
    pub wall: Color,
    pub fill: Option<Color>,
    pub dot: Color,
    pub tunnel_slow: usize,
    /// The rows a tunnel runs across.
    pub tunnels: Vec<usize>,
    /// How many dots and energizers there are to eat.
    pub dot_count: usize,
    /// The inner wall lines, one closed path per contour.
    inner: Vec<Shape>,
    /// The outer lines: the maze's edge and the ghost house, each a closed path.
    outer: Vec<Shape>,
    /// The corridors as one even-odd region, for filled mazes.
    corridor_fill: Shape,
    /// The void as one even-odd region, for filled mazes.
    void_fill: Shape,
    door: Rect,
}

impl Maze {
    pub fn build(def: &MazeDef) -> Maze {
        let mut tiles = Vec::with_capacity(COLS * ROWS);
        for row in def.rows {
            let mut n = 0;
            for ch in row.chars().take(COLS) {
                tiles.push(Tile::from_char(ch));
                n += 1;
            }
            while n < COLS {
                tiles.push(Tile::Void);
                n += 1;
            }
        }
        let tunnels = (0..ROWS)
            .filter(|&r| tiles[r * COLS].walkable() || tiles[r * COLS + COLS - 1].walkable())
            .collect();
        let dot_count = tiles
            .iter()
            .filter(|t| matches!(t, Tile::Dot | Tile::Energizer))
            .count();
        let mut door = Rect::new(0.0, 0.0, 0.0, 0.0);
        for (i, t) in tiles.iter().enumerate() {
            if *t == Tile::Door {
                let (c, r) = ((i % COLS) as f64, (i / COLS) as f64);
                let cell = Rect::new(c * TILE, r * TILE, TILE, TILE);
                door = if door.size.width == 0.0 {
                    cell
                } else {
                    let x0 = door.min_x().min(cell.min_x());
                    let y0 = door.origin.y.min(cell.origin.y);
                    let x1 = door.max_x().max(cell.max_x());
                    let y1 =
                        (door.origin.y + door.size.height).max(cell.origin.y + cell.size.height);
                    Rect::new(x0, y0, x1 - x0, y1 - y0)
                };
            }
        }
        let mut m = Maze {
            tiles,
            wall: def.wall,
            fill: def.fill,
            dot: def.dot,
            tunnel_slow: def.tunnel_slow,
            tunnels,
            dot_count,
            inner: Vec::new(),
            outer: Vec::new(),
            corridor_fill: PathBuilder::new().build(),
            void_fill: PathBuilder::new().build(),
            door,
        };
        m.trace_walls();
        m
    }

    pub fn tile(&self, c: i32, r: i32) -> Tile {
        if c < 0 || r < 0 || c >= COLS as i32 || r >= ROWS as i32 {
            return self.margin(c, r);
        }
        self.tiles[r as usize * COLS + c as usize]
    }

    /// The tile at `(c, r)` beyond the map: a tunnel runs on into the margin, the walls lining
    /// it run on too, and everything else out there is void.
    fn margin(&self, c: i32, r: i32) -> Tile {
        if r < 0 || r >= ROWS as i32 {
            return Tile::Void;
        }
        let edge = |rr: i32| {
            if rr < 0 || rr >= ROWS as i32 {
                return Tile::Void;
            }
            let cc = if c < 0 { 0 } else { COLS - 1 };
            self.tiles[rr as usize * COLS + cc]
        };
        let here = edge(r);
        if here.walkable() {
            Tile::Path
        } else if here == Tile::Wall && (edge(r - 1).walkable() || edge(r + 1).walkable()) {
            Tile::Wall
        } else {
            Tile::Void
        }
    }

    /// Whether an actor may step from tile `(c, r)` toward `dir`. The door opens only when
    /// `through_door` (a ghost leaving or returning).
    pub fn open(&self, c: i32, r: i32, dir: Dir, through_door: bool) -> bool {
        let (dx, dy) = dir.step();
        let t = self.tile(c + dx, r + dy);
        t.walkable() || (through_door && matches!(t, Tile::Door | Tile::House))
    }

    pub fn is_tunnel_slow(&self, c: i32, r: i32) -> bool {
        self.tunnels.contains(&(r as usize))
            && (c < self.tunnel_slow as i32 || c >= COLS as i32 - self.tunnel_slow as i32)
    }

    /// Build the wall picture (see the module doc).
    fn trace_walls(&mut self) {
        // Inner lines: the corridors dilated by half a tile, on the grid of tile centers. Cell
        // (i, j) is the square whose corners are the centers of tiles (i, j) and (i+1, j+1).
        let dilated = |i: i32, j: i32| {
            if i < -3 || j < -3 || i > COLS as i32 + 1 || j > ROWS as i32 + 1 {
                return false;
            }
            [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)]
                .iter()
                .any(|(c, r)| self.tile(*c, *r).walkable())
        };
        let inner_loops = trace_loops(&dilated, -3, -3, COLS as i32 + 2, ROWS as i32 + 2);
        let to_px = |p: (i32, i32)| Point::new(p.0 as f64 * TILE + 4.5, p.1 as f64 * TILE + 4.5);
        let mut corridor = PathBuilder::new().rule(FillRule::EvenOdd);
        for lp in &inner_loops {
            let pts: Vec<Point> = lp.iter().map(|p| to_px(*p)).collect();
            let shape = rounded_loop(&pts, TILE / 2.0);
            corridor = append(corridor, &shape);
            self.inner.push(shape);
        }
        self.corridor_fill = corridor.build();

        // Outer lines: the void's boundary, half a pixel into the wall beside it.
        let void = |c: i32, r: i32| {
            if c < -2 || r < -2 || c >= COLS as i32 + 2 || r >= ROWS as i32 + 2 {
                return false;
            }
            self.tile(c, r).void()
        };
        let void_loops = trace_loops(&void, -2, -2, COLS as i32 + 2, ROWS as i32 + 2);
        let mut void_fill = PathBuilder::new().rule(FillRule::EvenOdd);
        for lp in &void_loops {
            let n = lp.len();
            let mut pts = Vec::with_capacity(n);
            for i in 0..n {
                let prev = lp[(i + n - 1) % n];
                let cur = lp[i];
                let next = lp[(i + 1) % n];
                let din = (cur.0 - prev.0).signum() as f64;
                let din_y = (cur.1 - prev.1).signum() as f64;
                let dout = (next.0 - cur.0).signum() as f64;
                let dout_y = (next.1 - cur.1).signum() as f64;
                // The region is on the right of travel; the wall, where the line goes, on the
                // left: the left normal of (x, y) is (y, -x).
                let off_x = (din_y + dout_y) * 0.5;
                let off_y = -(din + dout) * 0.5;
                pts.push(Point::new(
                    cur.0 as f64 * TILE + off_x,
                    cur.1 as f64 * TILE + off_y,
                ));
            }
            let shape = rounded_loop(&pts, TILE - 0.5);
            void_fill = append(void_fill, &shape);
            self.outer.push(shape);
        }
        self.void_fill = void_fill.build();
    }

    /// Draw the walls and the door in maze pixel coordinates (the maze's top-left at the
    /// origin). `flash` swaps the wall color for white, the level-clear blink.
    pub fn draw_walls(&self, d: &mut Draw, flash: bool) {
        // The lines run on into the margin (the tunnels, their walls); the maze's rectangle
        // is where they show.
        d.clipped(
            Shape::Rect(Rect::new(0.0, 0.0, COLS as f64 * TILE, ROWS as f64 * TILE)),
            |d| self.draw_walls_unclipped(d, flash),
        );
    }

    fn draw_walls_unclipped(&self, d: &mut Draw, flash: bool) {
        let line = if flash { WALL_FLASH } else { self.wall };
        if let Some(fill) = self.fill {
            let fill = if flash { WALL_FLASH } else { fill };
            d.fill(
                Shape::Rect(Rect::new(0.0, 0.0, COLS as f64 * TILE, ROWS as f64 * TILE)),
                fill,
            );
            d.fill(self.corridor_fill.clone(), BLACK);
            d.fill(self.void_fill.clone(), BLACK);
        }
        for shape in self.inner.iter().chain(self.outer.iter()) {
            d.stroke(shape.clone(), line, 1.0);
        }
        // The door: the lines stop short of it, and a pink bar closes the gap.
        if self.door.size.width > 0.0 {
            d.fill(Shape::Rect(self.door), BLACK);
            d.fill(
                Shape::Rect(Rect::new(
                    self.door.origin.x,
                    self.door.origin.y + 3.0,
                    self.door.size.width,
                    2.0,
                )),
                DOOR_PINK,
            );
        }
    }

    /// Draw the dots not yet eaten (`eaten` indexed like `tiles`), as one stamp.
    pub fn draw_dots(&self, d: &mut Draw, eaten: &[bool]) {
        let mut at = Vec::new();
        for (i, t) in self.tiles.iter().enumerate() {
            if *t == Tile::Dot && !eaten.get(i).copied().unwrap_or(false) {
                let (c, r) = ((i % COLS) as f64, (i / COLS) as f64);
                at.push(Point::new(c * TILE + 3.0, r * TILE + 3.0));
            }
        }
        if !at.is_empty() {
            d.stamp(Shape::Rect(Rect::new(0.0, 0.0, 2.0, 2.0)), at, self.dot);
        }
    }

    /// The tiles holding an uneaten energizer.
    pub fn energizers(&self, eaten: &[bool]) -> Vec<(usize, usize)> {
        self.tiles
            .iter()
            .enumerate()
            .filter(|(i, t)| **t == Tile::Energizer && !eaten.get(*i).copied().unwrap_or(false))
            .map(|(i, _)| (i % COLS, i / COLS))
            .collect()
    }
}

/// Copy `shape`'s contours onto `pb`.
fn append(mut pb: PathBuilder, shape: &Shape) -> PathBuilder {
    if let Shape::Path(p) = shape {
        for seg in &p.segs {
            pb = match seg {
                day_spec::PathSeg::Move(p) => pb.move_to(*p),
                day_spec::PathSeg::Line(p) => pb.line_to(*p),
                day_spec::PathSeg::Quad(c, p) => pb.quad_to(*c, *p),
                day_spec::PathSeg::Cubic(a, b, p) => pb.cubic_to(*a, *b, *p),
                day_spec::PathSeg::Close => pb.close(),
            };
        }
    }
    pb
}

/// The closed boundaries of the cells where `inside` holds, over `x0..x1` × `y0..y1`, as
/// loops of corner vertices with the region on the right of travel.
fn trace_loops(
    inside: &dyn Fn(i32, i32) -> bool,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> Vec<Vec<(i32, i32)>> {
    // Directed edges, clockwise around each cell on a y-down screen: top, right, bottom, left.
    let mut edges: BTreeMap<(i32, i32), Vec<(i32, i32)>> = BTreeMap::new();
    let mut add = |a: (i32, i32), b: (i32, i32)| edges.entry(a).or_default().push(b);
    for j in y0..y1 {
        for i in x0..x1 {
            if !inside(i, j) {
                continue;
            }
            if !inside(i, j - 1) {
                add((i, j), (i + 1, j));
            }
            if !inside(i + 1, j) {
                add((i + 1, j), (i + 1, j + 1));
            }
            if !inside(i, j + 1) {
                add((i + 1, j + 1), (i, j + 1));
            }
            if !inside(i - 1, j) {
                add((i, j + 1), (i, j));
            }
        }
    }
    let mut loops = Vec::new();
    while let Some((&start, _)) = edges.iter().find(|(_, outs)| !outs.is_empty()) {
        let mut path = vec![start];
        let mut cur = start;
        let mut dir: Option<(i32, i32)> = None;
        loop {
            let outs = match edges.get_mut(&cur) {
                Some(o) if !o.is_empty() => o,
                _ => break,
            };
            // At a pinch (two regions touching at a corner) take the right turn, which keeps
            // each loop hugging its own region.
            let pick = match dir {
                Some((dx, dy)) => {
                    let right = (-dy, dx);
                    let straight = (dx, dy);
                    let score = |to: &(i32, i32)| {
                        let d = ((to.0 - cur.0).signum(), (to.1 - cur.1).signum());
                        if d == right {
                            0
                        } else if d == straight {
                            1
                        } else {
                            2
                        }
                    };
                    let mut best = 0;
                    for (k, to) in outs.iter().enumerate() {
                        if score(to) < score(&outs[best]) {
                            best = k;
                        }
                    }
                    best
                }
                None => 0,
            };
            let next = outs.swap_remove(pick);
            dir = Some(((next.0 - cur.0).signum(), (next.1 - cur.1).signum()));
            if next == start {
                break;
            }
            path.push(next);
            cur = next;
        }
        if path.len() >= 4 {
            loops.push(simplify(path));
        }
    }
    loops
}

/// Drop the vertices along straight runs, keeping corners.
fn simplify(path: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let n = path.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let prev = path[(i + n - 1) % n];
        let cur = path[i];
        let next = path[(i + 1) % n];
        let a = ((cur.0 - prev.0).signum(), (cur.1 - prev.1).signum());
        let b = ((next.0 - cur.0).signum(), (next.1 - cur.1).signum());
        if a != b {
            out.push(cur);
        }
    }
    out
}

/// A closed path through `pts` (all corners) with each corner rounded by up to `radius`.
fn rounded_loop(pts: &[Point], radius: f64) -> Shape {
    let n = pts.len();
    if n < 3 {
        return PathBuilder::new().build();
    }
    let corner = |i: usize| {
        let prev = pts[(i + n - 1) % n];
        let cur = pts[i];
        let next = pts[(i + 1) % n];
        let len_in = ((cur.x - prev.x).powi(2) + (cur.y - prev.y).powi(2)).sqrt();
        let len_out = ((next.x - cur.x).powi(2) + (next.y - cur.y).powi(2)).sqrt();
        let r = radius.min(len_in / 2.0).min(len_out / 2.0);
        let din = Point::new((cur.x - prev.x) / len_in, (cur.y - prev.y) / len_in);
        let dout = Point::new((next.x - cur.x) / len_out, (next.y - cur.y) / len_out);
        (
            Point::new(cur.x - din.x * r, cur.y - din.y * r),
            cur,
            Point::new(cur.x + dout.x * r, cur.y + dout.y * r),
        )
    };
    let (_, _, first_out) = corner(0);
    let mut pb = PathBuilder::new().move_to(first_out);
    for i in 1..=n {
        let (p_in, v, p_out) = corner(i % n);
        pb = pb.line_to(p_in).quad_to(v, p_out);
    }
    pb.close().build()
}

thread_local! {
    static MAZES: RefCell<BTreeMap<usize, Rc<Maze>>> = const { RefCell::new(BTreeMap::new()) };
}

/// Maze `index` (0 = Pac-Pac's, 1..=4 = Ms. Pac-Pac's), built once per process.
pub fn maze(index: usize) -> Rc<Maze> {
    MAZES.with(|m| {
        m.borrow_mut()
            .entry(index)
            .or_insert_with(|| {
                let def = if index == 0 {
                    &PACPAC
                } else {
                    &MS_MAZES[(index - 1).min(3)]
                };
                Rc::new(Maze::build(def))
            })
            .clone()
    })
}

/// Every map, for the checks below and the sweep that builds them all.
#[cfg(test)]
pub fn all_defs() -> Vec<&'static MazeDef> {
    let mut v = vec![&PACPAC];
    v.extend(MS_MAZES.iter());
    v
}

/// The problems with a map, if any: the arcade's rules every maze here must keep.
#[cfg(test)]
pub fn lint(def: &MazeDef) -> Vec<String> {
    use std::collections::{BTreeSet, VecDeque};
    let mut problems = Vec::new();
    let m = Maze::build(def);
    for (r, row) in def.rows.iter().enumerate() {
        if row.chars().count() != COLS {
            problems.push(format!("row {r} is {} wide", row.chars().count()));
        }
        for c in 0..COLS / 2 {
            let a = m.tile(c as i32, r as i32);
            let b = m.tile((COLS - 1 - c) as i32, r as i32);
            if a != b {
                problems.push(format!("row {r} is not symmetric at column {c}"));
            }
        }
    }
    let walk: Vec<(i32, i32)> = (0..ROWS as i32)
        .flat_map(|r| (0..COLS as i32).map(move |c| (c, r)))
        .filter(|(c, r)| m.tile(*c, *r).walkable())
        .collect();
    let set: BTreeSet<(i32, i32)> = walk.iter().copied().collect();
    // Every corridor tile has at least two ways on (an edge tile's tunnel counts).
    for &(c, r) in &walk {
        let mut ways = 0;
        for d in Dir::ALL {
            let (dx, dy) = d.step();
            let (nc, nr) = (c + dx, r + dy);
            if set.contains(&(nc, nr)) || (nc < 0 || nc >= COLS as i32) && m.tile(nc, nr).walkable()
            {
                ways += 1;
            }
        }
        if ways < 2 {
            problems.push(format!("dead end at ({c}, {r})"));
        }
        // No 2×2 open block: corridors are one tile wide.
        if set.contains(&(c + 1, r)) && set.contains(&(c, r + 1)) && set.contains(&(c + 1, r + 1)) {
            problems.push(format!("open 2x2 block at ({c}, {r})"));
        }
    }
    // No wall is one tile thick between two corridors: it would draw as nothing.
    for r in 0..ROWS as i32 {
        for c in 0..COLS as i32 {
            if m.tile(c, r) != Tile::Wall {
                continue;
            }
            let lr = set.contains(&(c - 1, r)) && set.contains(&(c + 1, r));
            let ud = set.contains(&(c, r - 1)) && set.contains(&(c, r + 1));
            if lr || ud {
                problems.push(format!("thin wall at ({c}, {r})"));
            }
        }
    }
    // No wall sits in a block smaller than three tiles long: a lone 2×2 draws as a circle.
    for r in 0..ROWS as i32 {
        for c in 0..COLS as i32 {
            if m.tile(c, r) != Tile::Wall {
                continue;
            }
            let run = |dc: i32, dr: i32| {
                let mut n = 1;
                for k in 1..3 {
                    if m.tile(c + dc * k, r + dr * k) == Tile::Wall {
                        n += 1;
                    } else {
                        break;
                    }
                }
                for k in 1..3 {
                    if m.tile(c - dc * k, r - dr * k) == Tile::Wall {
                        n += 1;
                    } else {
                        break;
                    }
                }
                n
            };
            if run(1, 0) < 3 && run(0, 1) < 3 {
                problems.push(format!("stubby wall at ({c}, {r})"));
            }
        }
    }
    // Everything connects.
    if let Some(&first) = walk.first() {
        let mut seen = BTreeSet::new();
        let mut q = VecDeque::from([first]);
        seen.insert(first);
        while let Some((c, r)) = q.pop_front() {
            for d in Dir::ALL {
                let (dx, dy) = d.step();
                let mut n = (c + dx, r + dy);
                if n.0 < 0 {
                    n.0 = COLS as i32 - 1;
                } else if n.0 >= COLS as i32 {
                    n.0 = 0;
                }
                if set.contains(&n) && seen.insert(n) {
                    q.push_back(n);
                }
            }
        }
        if seen.len() != set.len() {
            problems.push(format!(
                "{} corridor tiles unreachable",
                set.len() - seen.len()
            ));
        }
    }
    let energizers = m.tiles.iter().filter(|t| **t == Tile::Energizer).count();
    if energizers != 4 {
        problems.push(format!("{energizers} energizers"));
    }
    if !(200..=320).contains(&m.dot_count) {
        problems.push(format!("{} dots", m.dot_count));
    }
    if m.tunnels.is_empty() {
        problems.push("no tunnel".to_string());
    }
    for r in &m.tunnels {
        let r = *r as i32;
        if !(m.tile(0, r - 1) == Tile::Wall && m.tile(0, r + 1) == Tile::Wall) {
            problems.push(format!("tunnel on row {r} is not walled"));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_maze_keeps_the_arcade_rules() {
        let mut report = Vec::new();
        for (i, def) in all_defs().iter().enumerate() {
            let problems = lint(def);
            if !problems.is_empty() {
                report.push(format!("maze {i}: {problems:?}"));
            }
        }
        assert!(report.is_empty(), "{}", report.join("\n"));
    }

    #[test]
    fn pacpac_maze_has_the_arcade_counts() {
        let m = Maze::build(&PACPAC);
        assert_eq!(m.dot_count, 244);
        assert_eq!(m.tunnels, vec![14]);
        assert_eq!(m.door, Rect::new(104.0, 96.0, 16.0, 8.0));
    }

    #[test]
    fn walls_are_closed_rounded_loops() {
        let m = Maze::build(&PACPAC);
        assert!(m.inner.len() >= 20, "{} inner contours", m.inner.len());
        assert!(m.outer.len() >= 2, "{} outer contours", m.outer.len());
        for shape in m.inner.iter().chain(m.outer.iter()) {
            let Shape::Path(p) = shape else {
                panic!("not a path")
            };
            assert!(matches!(p.segs.last(), Some(day_spec::PathSeg::Close)));
        }
    }

    #[test]
    fn margin_extends_tunnels_and_their_walls() {
        let m = Maze::build(&PACPAC);
        assert_eq!(m.tile(-1, 14), Tile::Path);
        assert_eq!(m.tile(-2, 13), Tile::Wall);
        assert_eq!(m.tile(-1, 15), Tile::Wall);
        assert_eq!(m.tile(-1, 0), Tile::Void);
        assert_eq!(m.tile(30, 10), Tile::Void);
        assert!(m.is_tunnel_slow(2, 14));
        assert!(!m.is_tunnel_slow(10, 14));
    }

    #[test]
    fn sprites_have_pixels() {
        let s = sheet();
        assert_eq!(s.pac.len(), 4);
        assert!(s.pac[0][0].filled(8, 8), "shut mouth has a center");
        assert!(
            !s.pac[0][2].filled(13, 8),
            "wide mouth is open to the right"
        );
        assert!(
            s.pac[1][2].filled(13, 8),
            "facing left, the right edge is body"
        );
        assert!(s.ms_pac[0][1].paths.len() >= 3, "Ms. Pac-Pac wears a bow");
        assert_eq!(s.pac_death.len(), 12);
        assert_eq!(s.fruit.len(), Fruit::ALL.len());
        assert!(s.glyphs.contains_key(&'A') && s.glyphs.contains_key(&'!'));
    }

    #[test]
    fn text_draws_one_fill_per_known_glyph() {
        let s = sheet();
        let mut d = Draw::new();
        s.text(&mut d, "READY!", 0.0, 0.0, YELLOW);
        let fills = d
            .ops()
            .iter()
            .filter(|op| matches!(op, day_spec::DrawOp::Fill(..)))
            .count();
        assert_eq!(fills, 6);
    }
}
