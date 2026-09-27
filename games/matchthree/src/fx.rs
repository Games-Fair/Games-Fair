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
}

pub fn mix(a: Color, b: Color, t: f64) -> Color {
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// `c` pushed away from its own gray by `k`: the palette's soft tones become gemstone color.
fn vivid(c: Color, k: f64) -> Color {
    let l = 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
    let ch = |v: f64| (l + (v - l) * k).clamp(0.0, 1.0);
    Color::rgba(ch(c.r), ch(c.g), ch(c.b), c.a)
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

/// A cut stone: a lit body, crown facets shaded against a light from the upper left, a bright
/// table, a specular streak, and (when `look` asks) a halo and a glint.
pub fn gem(d: &mut Draw, cut: u8, base: Color, x: f64, y: f64, r: f64, look: Look) {
    let base = vivid(base, 1.5);
    let pts = outline(cut, x, y, r);
    if look.glow > 0.0 {
        let g = r * (1.5 + 0.15 * look.glow);
        d.fill(
            ellipse(x, y, g, g),
            RadialGradient::centered(base.with_alpha(0.55 * look.glow), base.with_alpha(0.0)),
        );
    }
    d.fill(
        Shape::Polygon(pts.iter().map(|p| Point::new(p.x, p.y + r * 0.1)).collect()),
        Color::rgba(0.02, 0.01, 0.06, 0.38),
    );
    let light = mix(base, Color::WHITE, 0.6);
    let deep = mix(base, Color::BLACK, 0.6);
    d.fill(
        Shape::Polygon(pts.clone()),
        RadialGradient::new(
            UnitPoint::new(0.36, 0.3),
            0.85,
            vec![(0.0, light), (0.45, base), (1.0, deep)],
        ),
    );
    let (cx, cy) = (x, y - r * 0.06);
    let inner: Vec<Point> = pts
        .iter()
        .map(|p| Point::new(cx + (p.x - x) * 0.5, cy + (p.y - y) * 0.5))
        .collect();
    let n = pts.len();
    let (lx, ly) = (-0.52, -0.85);
    for i in 0..n {
        let j = (i + 1) % n;
        let (a, b) = (pts[i], pts[j]);
        let (mx, my) = ((a.x + b.x) / 2.0 - x, (a.y + b.y) / 2.0 - y);
        let len = (mx * mx + my * my).sqrt().max(1e-6);
        // Alternate facets catch the light a little differently, which is what reads as a cut.
        let lit = (mx * lx + my * ly) / len + if i % 2 == 0 { 0.12 } else { -0.06 };
        let facet = Shape::Polygon(vec![a, b, inner[j], inner[i]]);
        if lit >= 0.0 {
            d.fill(facet, Color::WHITE.with_alpha(0.05 + 0.32 * lit.min(1.0)));
        } else {
            d.fill(facet, Color::BLACK.with_alpha(0.28 * (-lit).min(1.0)));
        }
    }
    d.fill(
        Shape::Polygon(inner.clone()),
        LinearGradient::new(
            UnitPoint::TOP_LEADING,
            UnitPoint::BOTTOM_TRAILING,
            vec![
                (0.0, mix(base, Color::WHITE, 0.5).with_alpha(0.95)),
                (1.0, base.with_alpha(0.85)),
            ],
        ),
    );
    for (p, q) in pts.iter().zip(&inner) {
        d.stroke(Shape::Line(*p, *q), Color::WHITE.with_alpha(0.16), 0.7);
    }
    d.stroke(Shape::Polygon(inner), Color::WHITE.with_alpha(0.42), 0.8);
    d.stroke(
        Shape::Polygon(pts),
        mix(base, Color::WHITE, 0.6).with_alpha(0.9),
        1.2,
    );
    d.fill(
        tilted(x - r * 0.4, y - r * 0.5, r * 0.34, r * 0.1, -0.62),
        Color::WHITE.with_alpha(0.6),
    );
    d.fill(
        ellipse(x - r * 0.62, y - r * 0.24, r * 0.06, r * 0.06),
        Color::WHITE.with_alpha(0.75),
    );
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
