// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! "Be Jewelled": faceted gemstones in deep jewel color, glitter and embers flying off every
//! move, bursts that flare and shake the board, starlight over a resting board, and a glass
//! marimba.
use super::{Board, Cues, Motion, Style};
use crate::fx::{self, Look, ellipse, glint, hash, mix, outline, tilted};
use crate::model::{Candy, Special};
use day_part_haptics::Haptic;
use day_pieces::prelude::*;
use gamekit::chrome::{self, cues};
use std::f64::consts::{PI, TAU};

pub static STYLE: Style = Style {
    name: "mt_theme_jewels",
    piece,
    tint: jewel,
    board: Board {
        frame: Color::hex(0x292738),
        edges: [0x8298B8, 0x719C9F, 0x99A8BB, 0xAC9B82, 0x9A91B1, 0x9D929E],
        tiles: [Color::hex(0x28223E), Color::hex(0x302742)],
        dot: Color::hex(0x9EBFE9),
        mark: Color::hex(0x87F4D1),
        cursor: Color::hex(0xB99ACF),
        ink: [Color::WHITE, chrome::GOLD, Color::hex(0x87F4D1)],
    },
    motion: Motion {
        finish: fx::Finish::Glitter,
        ambience: fx::Ambience::Starlight,
        shake: 1.0,
        flash: 1.0,
        twinkle: true,
        swell: 0.6,
        bloom: 0.85,
        bounce: 0.08,
        pulse: 0.35,
        pulse_rate: 4.0,
        pop: 0.45,
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

static SWAP: chrome::Cue = cues::with("sounds/matchthree/swap.wav", cues::LIGHT_BEAT);
static BLOOM: chrome::Cue = cues::with("sounds/matchthree/bloom.wav", cues::MEDIUM_BEAT);
static CASCADE: chrome::Cue = cues::with("sounds/matchthree/cascade.wav", chrome::CELEBRATE);
static MAGIC: chrome::Cue = cues::with("sounds/matchthree/magic.wav", chrome::THUD);
static VICTORY: chrome::Cue = cues::with("sounds/matchthree/victory.wav", chrome::BIG_CELEBRATE);
/// Three chains or more: the cascade clip over a phrase that keeps climbing.
static SURGE: chrome::Cue = cues::with(
    "sounds/matchthree/cascade.wav",
    &[
        (0, Haptic::Heavy),
        (70, Haptic::Medium),
        (140, Haptic::Medium),
        (230, Haptic::Success),
    ],
);

/// The six stones (ruby, sapphire, citrine, amethyst, emerald and fire opal), each as its shadow,
/// body and light tones. They keep the hues of the game's palette, so the chrome that uses the
/// palette still matches, but as deep, saturated jewel color.
const JEWELS: [[u32; 3]; 6] = [
    [0x3D0010, 0xD8002E, 0xFF5C7F],
    [0x021A52, 0x1463E0, 0x5CB8FF],
    [0x4F2600, 0xF09A00, 0xFFD740],
    [0x220646, 0x8230DE, 0xC48CFF],
    [0x01301B, 0x05A860, 0x45F0A0],
    [0x521000, 0xFF5A0A, 0xFFA257],
];

/// The body color of stone `i`: what its sparks, trails and wildcard wedge are tinted with.
fn jewel(i: u8) -> Color {
    Color::hex(JEWELS[i as usize % 6][1])
}

/// The flashes of spectral color a real stone throws as it turns (its "fire").
const FIRE: [u32; 5] = [0xFF5A6E, 0xFFD84A, 0x5CFF9A, 0x4AC8FF, 0xC07AFF];

/// A cut stone, lit from the upper left like a jewel under a lamp. A dark body with light
/// pooling in its lower right (what a real stone's pavilion sends back out), a crown of small
/// facets from shadow to white so the cut reads crisply, a bright table with its reflected
/// arrows, specular highlights, and flecks of spectral fire. `look` adds a halo and a glint.
fn gem(d: &mut Draw, cut: u8, x: f64, y: f64, r: f64, look: Look) {
    let [deep, body, light] = JEWELS[cut as usize % 6].map(Color::hex);
    let pts = outline(cut, x, y, r);
    let n = pts.len();
    // Every stone glows faintly in its own color; a held or chosen one blazes, and so does one
    // the resting board's starlight falls on.
    let glow = look.glow.max(0.5 * look.shine);
    let halo = r * (1.35 + 0.2 * glow);
    d.fill(
        ellipse(x, y, halo, halo),
        RadialGradient::centered(body.with_alpha(0.2 + 0.5 * glow), body.with_alpha(0.0)),
    );
    d.fill(
        Shape::Polygon(
            pts.iter()
                .map(|p| Point::new(p.x, p.y + r * 0.12))
                .collect(),
        ),
        Color::rgba(0.01, 0.0, 0.04, 0.5),
    );
    d.fill(Shape::Polygon(pts.clone()), deep);
    d.fill(
        Shape::Polygon(pts.clone()),
        RadialGradient::new(
            UnitPoint::new(0.62, 0.7),
            0.75,
            vec![
                (0.0, mix(light, Color::WHITE, 0.2)),
                (0.4, body),
                (1.0, body.with_alpha(0.0)),
            ],
        ),
    );
    let (cx, cy) = (x, y - r * 0.05);
    let inner: Vec<Point> = pts
        .iter()
        .map(|p| Point::new(cx + (p.x - x) * 0.54, cy + (p.y - y) * 0.54))
        .collect();
    let (lx, ly) = (-0.5, -0.866);
    // From shadow through body color to white: the crown's facets span the whole range, which is
    // what makes a stone look cut rather than molded.
    let shade = |s: f64| -> Color {
        if s < 0.0 {
            mix(body, deep, (-s).min(1.0) * 0.9)
        } else if s < 0.8 {
            mix(body, light, s / 0.8)
        } else {
            mix(light, Color::WHITE, ((s - 0.8) / 0.5).min(0.85))
        }
    };
    let mid = |a: Point, b: Point| Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b, ia, ib) = (pts[i], pts[j], inner[i], inner[j]);
        let m = mid(a, b);
        // Each side of the crown is three facets: two upper-girdle triangles and a star facet.
        for (k, tri) in [[a, m, ia], [m, b, ib], [m, ib, ia]]
            .into_iter()
            .enumerate()
        {
            let (tx, ty) = (
                (tri[0].x + tri[1].x + tri[2].x) / 3.0 - x,
                (tri[0].y + tri[1].y + tri[2].y) / 3.0 - y,
            );
            let len = (tx * tx + ty * ty).sqrt().max(1e-6);
            let slope = if k == 2 { 0.6 } else { 1.0 };
            let facet = (i * 3 + k) as u32 + cut as u32 * 97;
            let shimmer = if look.time > 0.0 {
                0.28 * (look.time * 1.2 + facet as f64 * 1.7 + look.seed * 2.3).sin()
            } else {
                0.0
            };
            let s = (tx * lx + ty * ly) / len * slope * 1.1 + 0.34 * hash(facet) + shimmer - 0.05;
            d.fill(Shape::Polygon(tri.to_vec()), shade(s));
        }
    }
    for i in 0..n {
        let j = (i + 1) % n;
        let m = mid(pts[i], pts[j]);
        d.stroke(
            Shape::Line(pts[i], inner[i]),
            Color::WHITE.with_alpha(0.22),
            0.6,
        );
        d.stroke(Shape::Line(m, inner[i]), Color::WHITE.with_alpha(0.14), 0.5);
        d.stroke(Shape::Line(m, inner[j]), Color::WHITE.with_alpha(0.14), 0.5);
    }
    // The table: a window into the stone, bright where it faces the light.
    d.fill(
        Shape::Polygon(inner.clone()),
        LinearGradient::new(
            UnitPoint::TOP_LEADING,
            UnitPoint::BOTTOM_TRAILING,
            vec![
                (0.0, mix(light, Color::WHITE, 0.15)),
                (0.45, body),
                (1.0, mix(body, deep, 0.5)),
            ],
        ),
    );
    // Its reflected arrows, and the smaller table mirrored in the pavilion below.
    for p in &inner {
        d.stroke(
            Shape::Line(Point::new(cx, cy), *p),
            deep.with_alpha(0.3),
            0.7,
        );
    }
    let echo: Vec<Point> = inner
        .iter()
        .map(|p| Point::new(cx + (p.x - cx) * 0.5, cy + (p.y - cy) * 0.5))
        .collect();
    d.fill(
        Shape::Polygon(echo.clone()),
        LinearGradient::new(
            UnitPoint::TOP_LEADING,
            UnitPoint::BOTTOM_TRAILING,
            vec![
                (0.0, mix(body, deep, 0.2).with_alpha(0.6)),
                (1.0, light.with_alpha(0.55)),
            ],
        ),
    );
    d.stroke(Shape::Polygon(echo), Color::WHITE.with_alpha(0.3), 0.6);
    d.stroke(Shape::Polygon(inner), Color::WHITE.with_alpha(0.6), 0.9);
    d.stroke(
        Shape::Polygon(pts),
        mix(light, Color::WHITE, 0.2).with_alpha(0.9),
        1.1,
    );
    // Light off the polished crown: a soft wash, a streak and a hot spot, and the colored glow
    // leaving the far side.
    d.fill(
        ellipse(x - r * 0.3, y - r * 0.42, r * 0.55, r * 0.45),
        RadialGradient::centered(Color::WHITE.with_alpha(0.4), Color::WHITE.with_alpha(0.0)),
    );
    d.fill(
        tilted(x - r * 0.38, y - r * 0.5, r * 0.36, r * 0.09, -0.62),
        Color::WHITE.with_alpha(0.85),
    );
    d.fill(
        ellipse(x - r * 0.62, y - r * 0.22, r * 0.07, r * 0.07),
        Color::WHITE,
    );
    d.fill(
        tilted(x + r * 0.4, y + r * 0.48, r * 0.26, r * 0.07, -0.62),
        mix(light, Color::WHITE, 0.4).with_alpha(0.7),
    );
    // Fire: now and then a fleck of spectral color flashes on the crown.
    if look.time > 0.0 {
        for k in 0..3 {
            let beat = look.time * 1.9 + k as f64 * 2.1 + look.seed * 1.3;
            let flash = beat.sin().max(0.0).powi(6);
            if flash < 0.02 {
                continue;
            }
            let a = look.seed * 2.4 + k as f64 * 2.2;
            let (fx, fy) = (x + a.cos() * r * 0.66, y + a.sin() * r * 0.66);
            let hue = Color::hex(FIRE[(look.seed as usize + k + (beat / TAU) as usize) % 5]);
            let size = r * 0.3 * flash;
            let rays: Vec<Point> = (0..8)
                .map(|i| {
                    let a = i as f64 * PI / 4.0 + PI / 4.0 * (k & 1) as f64;
                    let l = if i % 2 == 0 { size } else { size * 0.12 };
                    Point::new(fx + a.cos() * l, fy + a.sin() * l)
                })
                .collect();
            d.fill(Shape::Polygon(rays), hue.with_alpha(flash));
            let dot = size * 0.16;
            d.fill(ellipse(fx, fy, dot, dot), Color::WHITE.with_alpha(flash));
        }
    }
    if look.glint > 0.0 {
        glint(
            d,
            x + r * 0.32,
            y - r * 0.4,
            r * 0.6 * look.glint,
            look.time * 1.6,
            look.glint,
        );
    }
}

/// A faceted jewel for each shape (`gem`), with a special's mark worn over the stone.
fn piece(d: &mut Draw, c: Candy, x: f64, y: f64, r: f64, look: Look) {
    if c.special == Special::Rainbow {
        prism(d, x, y, r, look);
        return;
    }
    let color = jewel(c.color);
    gem(d, c.color, x, y, r, look);
    match c.special {
        Special::Row | Special::Column => {
            for offset in [-0.28, 0.0, 0.28] {
                let (a, b) = if c.special == Special::Row {
                    (
                        Point::new(x - r * 0.65, y + r * offset),
                        Point::new(x + r * 0.65, y + r * offset),
                    )
                } else {
                    (
                        Point::new(x + r * offset, y - r * 0.65),
                        Point::new(x + r * offset, y + r * 0.65),
                    )
                };
                d.stroke(Shape::Line(a, b), color.with_alpha(0.55), r * 0.26);
                d.stroke(Shape::Line(a, b), Color::WHITE.with_alpha(0.95), r * 0.11);
            }
        }
        Special::Wrapped => {
            let pulse = 0.65 + 0.35 * (look.time * 6.0).sin();
            let frame = Shape::RoundedRect(
                Rect::new(x - r * 0.62, y - r * 0.62, r * 1.24, r * 1.24),
                r * 0.22,
            );
            d.stroke(frame.clone(), color.with_alpha(0.5 * pulse), r * 0.22);
            d.stroke(frame, Color::WHITE, 2.0);
            glint(d, x, y, r * 0.55, look.time * 2.0, 1.0);
        }
        _ => {}
    }
}
/// The wildcard: turning wedges of every color under a glass dome.
fn prism(d: &mut Draw, x: f64, y: f64, r: f64, look: Look) {
    d.fill(
        ellipse(x, y, r * 1.5, r * 1.5),
        RadialGradient::centered(
            Color::WHITE.with_alpha(0.3 + 0.3 * look.glow),
            Color::WHITE.with_alpha(0.0),
        ),
    );
    d.fill(
        ellipse(x, y + r * 0.1, r, r),
        Color::rgba(0.02, 0.01, 0.06, 0.38),
    );
    let turn = look.time * 1.4;
    for k in 0..12 {
        let (a0, a1) = (
            turn + k as f64 * TAU / 12.0,
            turn + (k + 1) as f64 * TAU / 12.0,
        );
        d.fill(
            Shape::Polygon(vec![
                Point::new(x, y),
                Point::new(x + r * a0.cos(), y + r * a0.sin()),
                Point::new(x + r * a1.cos(), y + r * a1.sin()),
            ]),
            jewel((k % 6) as u8),
        );
    }
    d.fill(
        ellipse(x, y, r, r),
        RadialGradient::new(
            UnitPoint::new(0.35, 0.3),
            0.75,
            vec![
                (0.0, Color::WHITE.with_alpha(0.85)),
                (0.45, Color::WHITE.with_alpha(0.12)),
                (1.0, Color::BLACK.with_alpha(0.3)),
            ],
        ),
    );
    d.stroke(ellipse(x, y, r, r), Color::WHITE.with_alpha(0.8), 1.2);
    glint(d, x, y, r * 0.75, turn, 1.0);
}
