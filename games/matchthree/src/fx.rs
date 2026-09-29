// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! The jewel finish and the sparkle layer: faceted gem drawing, and a small particle system that
//! trails moving gems and bursts where matches clear. Particle positions are in board cells (the
//! gem in column 2, row 3 sits at (2.5, 3.5)), so a resize never strands a spark.
use day_pieces::prelude::*;
use std::f64::consts::{PI, TAU};

/// How a gem is lit this frame: `glint` flashes a star on its crown, `glow` haloes it (held or
/// selected), and `time` turns the glint's rays.
#[derive(Clone, Copy, Default)]
pub struct Look {
    pub glint: f64,
    pub glow: f64,
    pub time: f64,
    /// Sets each stone's facets shimmering and its fire flashing on its own beat.
    pub seed: f64,
}

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
pub fn jewel(i: u8) -> Color {
    Color::hex(JEWELS[i as usize % 6][1])
}

/// The flashes of spectral color a real stone throws as it turns (its "fire").
const FIRE: [u32; 5] = [0xFF5A6E, 0xFFD84A, 0x5CFF9A, 0x4AC8FF, 0xC07AFF];

/// A fixed pseudo-random value in -1..1 for facet `n`, so every stone is cut the same way each
/// frame but its facets don't shade in a smooth ramp.
fn hash(n: u32) -> f64 {
    let mut h = n.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA77);
    h ^= h >> 13;
    (h & 0xFFFF) as f64 / 65535.0 * 2.0 - 1.0
}

pub fn mix(a: Color, b: Color, t: f64) -> Color {
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn ellipse(x: f64, y: f64, rx: f64, ry: f64) -> Shape {
    Shape::Ellipse(Rect::new(x - rx, y - ry, rx * 2.0, ry * 2.0))
}

/// An ellipse turned by `angle`, as a polygon (a highlight laid along a facet).
fn tilted(x: f64, y: f64, rx: f64, ry: f64, angle: f64) -> Shape {
    let (s, c) = angle.sin_cos();
    Shape::Polygon(
        (0..14)
            .map(|i| {
                let a = i as f64 * TAU / 14.0;
                let (u, v) = (rx * a.cos(), ry * a.sin());
                Point::new(x + u * c - v * s, y + u * s + v * c)
            })
            .collect(),
    )
}

/// The outline of each of the six cuts around (x, y), radius `r`. Round and oval become
/// brilliants, the rounded rectangle an emerald cut; every silhouette stays the one the rules
/// and the color-blind can tell apart.
pub fn outline(cut: u8, x: f64, y: f64, r: f64) -> Vec<Point> {
    let ring = |n: usize, rx: f64, ry: f64, phase: f64| -> Vec<Point> {
        (0..n)
            .map(|i| {
                let a = phase + i as f64 * TAU / n as f64;
                Point::new(x + rx * a.cos(), y + ry * a.sin())
            })
            .collect()
    };
    match cut {
        0 => ring(16, r * 0.9, r * 0.9, -PI / 2.0),
        1 => {
            // A rhombus, with its edge midpoints as facet corners.
            let v = [
                Point::new(x, y - r),
                Point::new(x + r * 0.85, y),
                Point::new(x, y + r),
                Point::new(x - r * 0.85, y),
            ];
            (0..4)
                .flat_map(|i| {
                    let (a, b) = (v[i], v[(i + 1) % 4]);
                    [a, Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)]
                })
                .collect()
        }
        2 => (0..10)
            .map(|i| {
                let a = i as f64 * PI / 5.0 - PI / 2.0;
                let rr = if i % 2 == 0 { r * 1.08 } else { r * 0.52 };
                Point::new(x + a.cos() * rr, y + a.sin() * rr)
            })
            .collect(),
        3 => {
            let (w, h, k) = (r * 0.85, r, r * 0.34);
            vec![
                Point::new(x - w + k, y - h),
                Point::new(x + w - k, y - h),
                Point::new(x + w, y - h + k),
                Point::new(x + w, y + h - k),
                Point::new(x + w - k, y + h),
                Point::new(x - w + k, y + h),
                Point::new(x - w, y + h - k),
                Point::new(x - w, y - h + k),
            ]
        }
        4 => ring(16, r, r * 0.72, -PI / 2.0),
        _ => ring(6, r, r, 0.0),
    }
}

/// A cut stone, lit from the upper left like a jewel under a lamp. A dark body with light
/// pooling in its lower right (what a real stone's pavilion sends back out), a crown of small
/// facets from shadow to white so the cut reads crisply, a bright table with its reflected
/// arrows, specular highlights, and flecks of spectral fire. `look` adds a halo and a glint.
pub fn gem(d: &mut Draw, cut: u8, _base: Color, x: f64, y: f64, r: f64, look: Look) {
    let [deep, body, light] = JEWELS[cut as usize % 6].map(Color::hex);
    let pts = outline(cut, x, y, r);
    let n = pts.len();
    // Every stone glows faintly in its own color; a held or chosen one blazes.
    let halo = r * (1.35 + 0.2 * look.glow);
    d.fill(
        ellipse(x, y, halo, halo),
        RadialGradient::centered(body.with_alpha(0.2 + 0.5 * look.glow), body.with_alpha(0.0)),
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

/// A four-rayed star of light with a soft bloom, turned by `rot`.
pub fn glint(d: &mut Draw, x: f64, y: f64, size: f64, rot: f64, alpha: f64) {
    if size < 0.5 || alpha <= 0.01 {
        return;
    }
    d.fill(
        ellipse(x, y, size * 0.7, size * 0.7),
        RadialGradient::centered(
            Color::WHITE.with_alpha(0.55 * alpha),
            Color::WHITE.with_alpha(0.0),
        ),
    );
    d.fill(
        Shape::Polygon(
            (0..8)
                .map(|i| {
                    let a = rot + i as f64 * PI / 4.0;
                    let l = if i % 2 == 0 { size } else { size * 0.16 };
                    Point::new(x + a.cos() * l, y + a.sin() * l)
                })
                .collect(),
        ),
        Color::WHITE.with_alpha(alpha.min(1.0)),
    );
}

/// A gem's occasional flash while the board is moving: each gem gets its own beat, so the
/// glints ripple across the board instead of blinking together.
pub fn twinkle(time: f64, seed: usize) -> f64 {
    let phase = (time * 0.45 + (seed as f64 * 0.618_034).fract()).fract();
    if phase < 0.1 {
        (phase / 0.1 * PI).sin()
    } else {
        0.0
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Glint,
    Ember,
    Ring,
    Streak,
}

#[derive(Clone, Copy)]
struct Spark {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    age: f64,
    life: f64,
    size: f64,
    spin: f64,
    color: Color,
    kind: Kind,
}

/// The live sparks. Bounded, and every spark dies within a second, so the board goes back to
/// requesting no frames soon after the last move.
pub struct Sparks {
    list: Vec<Spark>,
    rng: u64,
}

const MAX_SPARKS: usize = 260;

impl Sparks {
    pub fn new() -> Self {
        Self {
            list: Vec::new(),
            // Its own generator: sparkle never draws from the game's seeded RNG.
            rng: 0x9E37_79B9_7F4A_7C15,
        }
    }
    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }
    fn push(&mut self, s: Spark) {
        if self.list.len() >= MAX_SPARKS {
            self.list.remove(0);
        }
        self.list.push(s);
    }
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
    pub fn clear(&mut self) {
        self.list.clear();
    }
    pub fn step(&mut self, dt: f64) {
        let drag = (1.0 - 3.0 * dt).max(0.0);
        for s in &mut self.list {
            s.age += dt;
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            s.vx *= drag;
            s.vy *= drag;
            if s.kind == Kind::Ember {
                s.vy += 3.2 * dt;
            }
            s.spin += dt * 4.0;
        }
        self.list.retain(|s| s.age < s.life);
    }
    /// A moving gem's wake: `amount` sparks on average, glints and embers shed around it.
    pub fn trail(&mut self, x: f64, y: f64, color: Color, amount: f64) {
        let mut left = amount;
        while left > 0.0 {
            if left < 1.0 && self.rand() > left {
                break;
            }
            left -= 1.0;
            let a = self.rand() * TAU;
            let speed = 0.25 + self.rand() * 0.45;
            let is_glint = self.rand() < 0.45;
            let s = Spark {
                x: x + (self.rand() - 0.5) * 0.5,
                y: y + (self.rand() - 0.5) * 0.5,
                vx: a.cos() * speed,
                vy: a.sin() * speed - 0.2,
                age: 0.0,
                life: 0.3 + self.rand() * 0.25,
                size: if is_glint {
                    0.16 + self.rand() * 0.12
                } else {
                    0.05 + self.rand() * 0.05
                },
                spin: self.rand() * TAU,
                color: if is_glint {
                    Color::WHITE
                } else {
                    mix(color, Color::WHITE, 0.35)
                },
                kind: if is_glint { Kind::Glint } else { Kind::Ember },
            };
            self.push(s);
        }
    }
    /// A cleared gem: a shockwave ring, a fan of embers, and a few big glints. `power` grows with
    /// the cascade and with specials.
    pub fn burst(&mut self, x: f64, y: f64, color: Color, power: f64) {
        let p = power.max(0.5);
        self.push(Spark {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            age: 0.0,
            life: 0.4,
            size: 0.55 * p.sqrt(),
            spin: 0.0,
            color: mix(color, Color::WHITE, 0.4),
            kind: Kind::Ring,
        });
        let embers = (8.0 * p).round() as usize;
        for k in 0..embers {
            let a = k as f64 / embers as f64 * TAU + self.rand() * 0.5;
            let speed = (1.6 + self.rand() * 1.8) * p.sqrt();
            let s = Spark {
                x,
                y,
                vx: a.cos() * speed,
                vy: a.sin() * speed - 0.6,
                age: 0.0,
                life: 0.45 + self.rand() * 0.25,
                size: 0.05 + self.rand() * 0.06,
                spin: 0.0,
                color: mix(color, Color::WHITE, self.rand() * 0.5),
                kind: Kind::Ember,
            };
            self.push(s);
        }
        for _ in 0..(2 + p as usize) {
            let a = self.rand() * TAU;
            let s = Spark {
                x: x + a.cos() * 0.3,
                y: y + a.sin() * 0.3,
                vx: a.cos() * 0.6,
                vy: a.sin() * 0.6 - 0.4,
                age: 0.0,
                life: 0.5,
                size: 0.25 + self.rand() * 0.15,
                spin: self.rand() * TAU,
                color: Color::WHITE,
                kind: Kind::Glint,
            };
            self.push(s);
        }
    }
    /// A striped gem's beam: streaks racing both ways along its row or column.
    pub fn beam(&mut self, x: f64, y: f64, horizontal: bool, color: Color) {
        for k in 0..14 {
            let dir = if k % 2 == 0 { 1.0 } else { -1.0 };
            let speed = (4.0 + self.rand() * 5.0) * dir;
            let spread = (self.rand() - 0.5) * 0.3;
            let s = Spark {
                x: x + if horizontal { 0.0 } else { spread },
                y: y + if horizontal { spread } else { 0.0 },
                vx: if horizontal { speed } else { 0.0 },
                vy: if horizontal { 0.0 } else { speed },
                age: 0.0,
                life: 0.35,
                size: 0.06,
                spin: 0.0,
                color: mix(color, Color::WHITE, 0.5),
                kind: Kind::Streak,
            };
            self.push(s);
        }
    }
    /// A gem settling after a fall: one small twinkle at its foot.
    pub fn land(&mut self, x: f64, y: f64) {
        let s = Spark {
            x: x + (self.rand() - 0.5) * 0.6,
            y: y + 0.32,
            vx: 0.0,
            vy: -0.3,
            age: 0.0,
            life: 0.3,
            size: 0.14,
            spin: self.rand() * TAU,
            color: Color::WHITE,
            kind: Kind::Glint,
        };
        self.push(s);
    }
    /// Paint every spark over a board whose origin is (ox, oy) and whose cells are `c` wide.
    pub fn draw(&self, d: &mut Draw, ox: f64, oy: f64, c: f64) {
        for s in &self.list {
            let t = (s.age / s.life).clamp(0.0, 1.0);
            let (x, y) = (ox + s.x * c, oy + s.y * c);
            match s.kind {
                Kind::Glint => {
                    let bloom = (t * PI).sin();
                    glint(d, x, y, s.size * c * bloom, s.spin, bloom);
                }
                Kind::Ember => {
                    let r = s.size * c * (1.0 - t * 0.5);
                    let a = 1.0 - t;
                    d.fill(
                        ellipse(x, y, r * 3.0, r * 3.0),
                        RadialGradient::centered(
                            s.color.with_alpha(0.45 * a),
                            s.color.with_alpha(0.0),
                        ),
                    );
                    d.fill(
                        ellipse(x, y, r, r),
                        mix(s.color, Color::WHITE, 0.5).with_alpha(a),
                    );
                }
                Kind::Ring => {
                    let r = (0.25 + t * 1.1) * s.size * c * 2.0;
                    d.stroke(
                        ellipse(x, y, r, r),
                        s.color.with_alpha(0.8 * (1.0 - t)),
                        c * 0.08 * (1.0 - t) + 0.5,
                    );
                }
                Kind::Streak => {
                    let tail = Point::new(x - s.vx * c * 0.05, y - s.vy * c * 0.05);
                    let w = s.size * c * (1.0 - t) + 0.5;
                    d.stroke(
                        Shape::Line(tail, Point::new(x, y)),
                        s.color.with_alpha(1.0 - t),
                        w,
                    );
                    d.stroke(
                        Shape::Line(tail, Point::new(x, y)),
                        Color::WHITE.with_alpha(1.0 - t),
                        w * 0.4,
                    );
                }
            }
        }
    }
}

/// A gem on a resting board: its cell, its center in board cells, and its color.
pub struct Resting {
    pub cell: usize,
    pub x: f64,
    pub y: f64,
    pub color: Color,
}

/// A gem slowly catching the light. It waits while `age` is negative, then brightens and dims
/// over `life` while its star turns by `turn` radians a second.
struct Star {
    cell: usize,
    x: f64,
    y: f64,
    color: Color,
    age: f64,
    life: f64,
    spin: f64,
    turn: f64,
}

impl Star {
    fn brightness(&self) -> f64 {
        if self.age <= 0.0 {
            0.0
        } else {
            (self.age / self.life * PI).sin().powi(2)
        }
    }
}

/// A mote of light drifting up over the board, twinkling as it goes.
struct Mote {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    age: f64,
    life: f64,
    size: f64,
    phase: f64,
    color: Color,
}

/// How many gems glow at once, and the most motes afloat.
const LIT: usize = 4;
const MAX_MOTES: usize = 140;

/// The resting board's starlight. A few gems at a time slowly brighten under a turning star and
/// fade as others take over, while motes rise from them and drift across the board like dust in
/// a beam. It fades in over a couple of seconds and out as soon as play resumes.
pub struct Idle {
    stars: Vec<Star>,
    motes: Vec<Mote>,
    fade: f64,
    rng: u64,
}

impl Idle {
    pub fn new() -> Self {
        Self {
            stars: Vec::new(),
            motes: Vec::new(),
            fade: 0.0,
            rng: 0xD1B5_4A32_D192_ED03,
        }
    }
    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Whether anything still shows, so the board keeps drawing frames.
    pub fn is_live(&self) -> bool {
        self.fade > 0.0
    }
    /// How far the starlight has faded in, 0..1.
    pub fn fade(&self) -> f64 {
        self.fade
    }
    /// How brightly the gem in `cell` glows now, 0..1.
    pub fn light(&self, cell: usize) -> f64 {
        self.stars
            .iter()
            .filter(|s| s.cell == cell)
            .map(Star::brightness)
            .fold(0.0, f64::max)
            * self.fade
    }
    /// Advance by `dt`: `rest` holds the board's gems while it rests, or `None` once play
    /// resumes; `side` is the board's width in cells.
    pub fn step(&mut self, dt: f64, rest: Option<&[Resting]>, side: f64) {
        for s in &mut self.stars {
            s.age += dt;
            s.spin += s.turn * dt;
        }
        for m in &mut self.motes {
            m.age += dt;
            m.x += m.vx * dt;
            m.y += m.vy * dt;
        }
        self.motes.retain(|m| m.age < m.life);
        let Some(gems) = rest else {
            self.fade = (self.fade - dt * 3.0).max(0.0);
            if self.fade == 0.0 {
                self.stars.clear();
                self.motes.clear();
            }
            return;
        };
        self.fade = (self.fade + dt / 2.5).min(1.0);
        self.stars
            .retain(|s| s.age < s.life && gems.iter().any(|g| g.cell == s.cell));
        // A fresh start staggers its stars so they never swell together; afterwards each one
        // that sets is replaced by another somewhere else after a short breath.
        let fresh = self.stars.is_empty();
        while self.stars.len() < LIT.min(gems.len()) {
            let free: Vec<&Resting> = gems
                .iter()
                .filter(|g| self.stars.iter().all(|s| s.cell != g.cell))
                .collect();
            let g = free[((self.rand() * free.len() as f64) as usize).min(free.len() - 1)];
            let (cell, x, y, color) = (g.cell, g.x, g.y, g.color);
            let delay = if fresh {
                self.stars.len() as f64 * 1.2 + self.rand() * 0.4
            } else {
                0.3 + self.rand() * 0.9
            };
            let star = Star {
                cell,
                x,
                y,
                color,
                age: -delay,
                life: 4.0 + self.rand() * 2.5,
                spin: self.rand() * TAU,
                turn: (0.25 + self.rand() * 0.2) * if self.rand() < 0.5 { 1.0 } else { -1.0 },
            };
            self.stars.push(star);
        }
        // Motes rise from the glowing gems, and a few more wander in anywhere.
        let lit: Vec<(f64, f64, Color, f64)> = self
            .stars
            .iter()
            .map(|s| (s.x, s.y, s.color, s.brightness()))
            .collect();
        for (x, y, color, b) in lit {
            if self.rand() < 1.4 * b * dt {
                self.mote(x, y, 0.35, mix(color, Color::WHITE, 0.55));
            }
        }
        if self.rand() < 0.8 * dt {
            let (x, y) = (self.rand() * side, self.rand() * side);
            self.mote(x, y, 0.0, Color::WHITE);
        }
    }
    fn mote(&mut self, x: f64, y: f64, spread: f64, color: Color) {
        if self.motes.len() >= MAX_MOTES {
            return;
        }
        let m = Mote {
            x: x + (self.rand() - 0.5) * spread * 2.0,
            y: y + (self.rand() - 0.5) * spread * 1.4,
            vx: (self.rand() - 0.5) * 0.14,
            vy: -(0.08 + self.rand() * 0.12),
            age: 0.0,
            life: 2.8 + self.rand() * 2.4,
            size: 0.022 + self.rand() * 0.03,
            phase: self.rand() * TAU,
            color,
        };
        self.motes.push(m);
    }
    /// Paint the motes and stars over a board whose origin is (ox, oy) and whose cells are `c`
    /// wide.
    pub fn draw(&self, d: &mut Draw, ox: f64, oy: f64, c: f64) {
        if self.fade <= 0.0 {
            return;
        }
        for m in &self.motes {
            let t = m.age / m.life;
            let a = self.fade * (t * PI).sin() * (0.55 + 0.45 * (m.phase + m.age * 4.5).sin());
            if a <= 0.01 {
                continue;
            }
            let (x, y, r) = (ox + m.x * c, oy + m.y * c, m.size * c);
            d.fill(
                ellipse(x, y, r * 3.5, r * 3.5),
                RadialGradient::centered(m.color.with_alpha(0.4 * a), m.color.with_alpha(0.0)),
            );
            d.fill(ellipse(x, y, r * 0.8, r * 0.8), Color::WHITE.with_alpha(a));
        }
        for s in &self.stars {
            let b = s.brightness() * self.fade;
            if b <= 0.01 {
                continue;
            }
            let (x, y) = (ox + (s.x + 0.2) * c, oy + (s.y - 0.22) * c);
            d.fill(
                ellipse(x, y, c * 0.55 * b, c * 0.55 * b),
                RadialGradient::centered(
                    mix(s.color, Color::WHITE, 0.7).with_alpha(0.3 * b),
                    s.color.with_alpha(0.0),
                ),
            );
            // Long rays and short diagonals: an eight-pointed star that slowly turns.
            glint(d, x, y, c * 0.62 * b, s.spin, b);
            glint(d, x, y, c * 0.3 * b, s.spin + PI / 4.0, 0.8 * b);
        }
    }
}
