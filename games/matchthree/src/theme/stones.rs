// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! "Sea Stones": sea-worn pebbles in quiet natural colors on a tide-pool board. Matches lift
//! away with a slow ripple and a little foam, waves wash up over a resting board and leave the
//! stones glistening, and every sound is pebbles clicking together in the surf.
use super::{Board, Cues, Motion, Style};
use crate::fx::{self, Look, ellipse, hash, mix, outline, tilted};
use crate::model::{Candy, Special};
use day_part_haptics::Haptic;
use day_pieces::prelude::*;
use gamekit::chrome::{self, cues};
use std::f64::consts::PI;

pub static STYLE: Style = Style {
    name: "mt_theme_stones",
    piece,
    tint: stone,
    board: Board {
        frame: Color::hex(0x26323A),
        edges: [0x7C8F96, 0x8C9A88, 0x9A938A, 0xA39680, 0x8E8C9C, 0x8FA0A0],
        tiles: [Color::hex(0x2B3940), Color::hex(0x314149)],
        dot: Color::hex(0xC9BFA8),
        mark: Color::hex(0xEFE3C8),
        cursor: Color::hex(0xA9B8BD),
        ink: [
            Color::hex(0xF4EFE6),
            Color::hex(0xE8D6A8),
            Color::hex(0xBFE3DD),
        ],
    },
    motion: Motion {
        finish: fx::Finish::Foam,
        ambience: fx::Ambience::Tide,
        shake: 0.0,
        flash: 0.25,
        twinkle: false,
        swell: 0.0,
        bloom: 0.3,
        bounce: 0.0,
        pulse: 0.2,
        pulse_rate: 1.6,
        pop: 0.1,
    },
    cues: Cues {
        swap: &SWAP,
        bloom: &BLOOM,
        cascade: &CASCADE,
        surge: &SURGE,
        magic: &MAGIC,
        victory: &VICTORY,
    },
};

// Soft haptics to match: taps and light beats, never a thud.
static SWAP: chrome::Cue = cues::with("sounds/matchthree/stones/swap.wav", cues::TICK_BEAT);
static BLOOM: chrome::Cue = cues::with("sounds/matchthree/stones/bloom.wav", cues::LIGHT_BEAT);
static CASCADE: chrome::Cue = cues::with(
    "sounds/matchthree/stones/cascade.wav",
    &[(0, Haptic::Light), (110, Haptic::Light)],
);
static SURGE: chrome::Cue = cues::with(
    "sounds/matchthree/stones/cascade.wav",
    &[
        (0, Haptic::Light),
        (110, Haptic::Light),
        (220, Haptic::Medium),
    ],
);
static MAGIC: chrome::Cue = cues::with("sounds/matchthree/stones/magic.wav", cues::MEDIUM_BEAT);
static VICTORY: chrome::Cue = cues::with("sounds/matchthree/stones/victory.wav", chrome::CELEBRATE);

/// The six pebbles (red jasper, slate, sandstone, lilac granite, sage jade and quartz), each as
/// shadow, body and light tones. Muted, but still six hues far enough apart to tell at a glance,
/// and each keeps the silhouette of its jewel so color-blind players lose nothing.
const STONES: [[u32; 3]; 6] = [
    [0x5A342C, 0xA8675A, 0xD8A597],
    [0x2C3B47, 0x5E7C91, 0xA9C0CF],
    [0x5A4827, 0xB7985A, 0xE6D3A4],
    [0x39324A, 0x7B6F92, 0xBDB3CB],
    [0x2B3F31, 0x6A8B71, 0xB2C8B5],
    [0x69635A, 0xCFC6B8, 0xF4EFE6],
];

fn stone(i: u8) -> Color {
    Color::hex(STONES[i as usize % 6][1])
}

/// One round of corner cutting: every edge keeps its middle half, so corners round off.
fn chaikin(p: &[Point]) -> Vec<Point> {
    let n = p.len();
    (0..n)
        .flat_map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            [
                Point::new(a.x * 0.75 + b.x * 0.25, a.y * 0.75 + b.y * 0.25),
                Point::new(a.x * 0.25 + b.x * 0.75, a.y * 0.25 + b.y * 0.75),
            ]
        })
        .collect()
}

/// The jewel silhouette of `cut`, worn smooth like a pebble.
fn pebble(cut: u8, x: f64, y: f64, r: f64) -> Vec<Point> {
    (0..3).fold(outline(cut, x, y, r * 0.96), |p, _| chaikin(&p))
}

fn shifted(p: &[Point], dx: f64, dy: f64, grow: f64, x: f64, y: f64) -> Vec<Point> {
    p.iter()
        .map(|q| Point::new(x + (q.x - x) * grow + dx, y + (q.y - y) * grow + dy))
        .collect()
}

/// A smooth stone, softly lit from the upper left: a blurred shadow, a matte body with its
/// grain and a pale vein, light bouncing up into its underside, and a broad soft highlight.
/// `look.shine` wets it: its color deepens and a glossy film catches the light.
fn pebble_stone(d: &mut Draw, cut: u8, x: f64, y: f64, r: f64, look: Look, colors: [Color; 3]) {
    let [shadow, body, light] = colors;
    let wet = look.shine.clamp(0.0, 1.0);
    let body = mix(body, shadow, 0.22 * wet);
    let light = mix(light, body, 0.25 * wet);
    let pts = pebble(cut, x, y, r);
    if look.glow > 0.0 {
        let g = r * 1.4;
        d.fill(
            ellipse(x, y, g, g),
            RadialGradient::centered(
                Color::hex(0xF3E9D2).with_alpha(0.35 * look.glow),
                Color::hex(0xF3E9D2).with_alpha(0.0),
            ),
        );
    }
    for k in 0..3 {
        let k = k as f64;
        d.fill(
            Shape::Polygon(shifted(
                &pts,
                0.0,
                r * (0.1 + 0.03 * k),
                1.0 + 0.05 * k,
                x,
                y,
            )),
            Color::rgba(0.0, 0.01, 0.02, 0.13),
        );
    }
    d.fill(
        Shape::Polygon(pts.clone()),
        RadialGradient::new(
            UnitPoint::new(0.36, 0.3),
            0.95,
            vec![(0.0, light), (0.5, body), (1.0, shadow)],
        ),
    );
    let seed = look.seed as u32 * 31 + cut as u32 * 7;
    d.clipped(Shape::Polygon(pts.clone()), |d| {
        // A pale vein of quartz across the stone, at its own angle.
        let a = hash(seed) * PI;
        let off = hash(seed + 1) * r * 0.3;
        let (s, c) = a.sin_cos();
        let (nx, ny) = (-s, c);
        let at = |t: f64, o: f64| Point::new(x + c * t + nx * o, y + s * t + ny * o);
        d.stroke(
            Shape::Line(at(-r * 1.2, off), at(r * 1.2, off)),
            light.with_alpha(0.22),
            r * 0.07,
        );
        d.stroke(
            Shape::Line(at(-r * 1.2, off + r * 0.12), at(r * 1.2, off + r * 0.12)),
            shadow.with_alpha(0.12),
            r * 0.04,
        );
        // Grain: a few darker and lighter specks.
        for k in 0..9u32 {
            let (px, py) = (
                x + hash(seed + 10 + k * 2) * r * 0.7,
                y + hash(seed + 11 + k * 2) * r * 0.7,
            );
            let size = r * (0.02 + 0.015 * hash(seed + 40 + k).abs());
            let fleck = if k % 2 == 0 {
                shadow.with_alpha(0.3)
            } else {
                light.with_alpha(0.35)
            };
            d.fill(ellipse(px, py, size, size), fleck);
        }
        d.fill(
            ellipse(x + r * 0.1, y + r * 0.58, r * 0.55, r * 0.25),
            RadialGradient::centered(light.with_alpha(0.2), light.with_alpha(0.0)),
        );
    });
    let soft = 0.22 + 0.2 * wet + 0.3 * look.glint;
    d.fill(
        ellipse(x - r * 0.3, y - r * 0.38, r * 0.45, r * 0.3),
        RadialGradient::centered(Color::WHITE.with_alpha(soft), Color::WHITE.with_alpha(0.0)),
    );
    if wet > 0.01 {
        d.fill(
            tilted(x - r * 0.32, y - r * 0.44, r * 0.3, r * 0.075, -0.5),
            Color::WHITE.with_alpha(0.45 * wet),
        );
        d.fill(
            ellipse(x + r * 0.36, y - r * 0.08, r * 0.05, r * 0.05),
            Color::WHITE.with_alpha(0.5 * wet),
        );
    }
    d.stroke(Shape::Polygon(pts), shadow.with_alpha(0.45), 1.0);
}

/// A groove cut into the stone: a dark channel with a pale lip below it.
fn groove(d: &mut Draw, a: Point, b: Point, width: f64, shadow: Color, light: Color) {
    d.stroke(Shape::Line(a, b), shadow.with_alpha(0.65), width);
    let lip = |p: Point| Point::new(p.x + width * 0.25, p.y + width * 0.45);
    d.stroke(
        Shape::Line(lip(a), lip(b)),
        light.with_alpha(0.4),
        width * 0.4,
    );
}

/// A pebble for each shape, with a special's mark carved into it; the wildcard is a banded
/// agate.
fn piece(d: &mut Draw, c: Candy, x: f64, y: f64, r: f64, look: Look) {
    if c.special == Special::Rainbow {
        return agate(d, x, y, r, look);
    }
    let colors = STONES[c.color as usize % 6].map(Color::hex);
    pebble_stone(d, c.color, x, y, r, look, colors);
    let [shadow, _, light] = colors;
    match c.special {
        Special::Row | Special::Column => {
            for offset in [-0.28, 0.0, 0.28] {
                let (a, b) = if c.special == Special::Row {
                    (
                        Point::new(x - r * 0.55, y + r * offset),
                        Point::new(x + r * 0.55, y + r * offset),
                    )
                } else {
                    (
                        Point::new(x + r * offset, y - r * 0.55),
                        Point::new(x + r * offset, y + r * 0.55),
                    )
                };
                groove(d, a, b, r * 0.11, shadow, light);
            }
        }
        Special::Wrapped => {
            let breathe = 0.18 + 0.08 * (look.time * 1.5).sin();
            d.fill(
                ellipse(x, y, r * 0.7, r * 0.7),
                RadialGradient::centered(light.with_alpha(breathe), light.with_alpha(0.0)),
            );
            let h = r * 0.55;
            let corners = [
                Point::new(x - h, y - h),
                Point::new(x + h, y - h),
                Point::new(x + h, y + h),
                Point::new(x - h, y + h),
            ];
            for i in 0..4 {
                groove(d, corners[i], corners[(i + 1) % 4], r * 0.1, shadow, light);
            }
        }
        _ => {}
    }
}

/// The wildcard: a round agate whose bands hold every stone's color.
fn agate(d: &mut Draw, x: f64, y: f64, r: f64, look: Look) {
    let pts = pebble(0, x, y, r);
    if look.glow > 0.0 {
        d.fill(
            ellipse(x, y, r * 1.4, r * 1.4),
            RadialGradient::centered(
                Color::hex(0xF3E9D2).with_alpha(0.35 * look.glow),
                Color::hex(0xF3E9D2).with_alpha(0.0),
            ),
        );
    }
    d.fill(
        Shape::Polygon(shifted(&pts, 0.0, r * 0.12, 1.04, x, y)),
        Color::rgba(0.0, 0.01, 0.02, 0.25),
    );
    let drift = look.time * 0.2;
    d.clipped(Shape::Polygon(pts.clone()), |d| {
        for k in 0..9 {
            let f = 1.05 - k as f64 * 0.11;
            let (dx, dy) = (
                drift.cos() * r * 0.04 * k as f64,
                drift.sin() * r * 0.04 * k as f64,
            );
            let [_, body, light] = STONES[k % 6].map(Color::hex);
            d.fill(
                ellipse(x + dx, y + dy, r * f, r * f * 0.9),
                mix(body, light, 0.25),
            );
        }
    });
    d.fill(
        Shape::Polygon(pts.clone()),
        RadialGradient::new(
            UnitPoint::new(0.36, 0.3),
            0.95,
            vec![
                (0.0, Color::WHITE.with_alpha(0.35)),
                (0.5, Color::WHITE.with_alpha(0.0)),
                (1.0, Color::BLACK.with_alpha(0.3)),
            ],
        ),
    );
    let wet = look.shine.clamp(0.0, 1.0);
    if wet > 0.01 {
        d.fill(
            tilted(x - r * 0.32, y - r * 0.44, r * 0.3, r * 0.075, -0.5),
            Color::WHITE.with_alpha(0.45 * wet),
        );
    }
    d.stroke(
        Shape::Polygon(pts),
        Color::hex(0x3B3530).with_alpha(0.5),
        1.0,
    );
}
