// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! The effects layer every theme shares: piece outlines, a small particle system that trails
//! moving pieces and bursts where matches clear, and the resting board's ambience. Particle
//! positions are in board cells (the piece in column 2, row 3 sits at (2.5, 3.5)), so a resize
//! never strands a spark. Each theme picks a particle `Finish` and an idle `Ambience`.
use day_pieces::prelude::*;
use std::f64::consts::{PI, TAU};

/// How a piece is lit this frame: `glint` flashes a star on it, `glow` haloes it (held or
/// selected), `shine` is the resting board's ambience on it (starlight on a jewel, the wash of a
/// wave on a stone), and `time` animates it.
#[derive(Clone, Copy, Default)]
pub struct Look {
    pub glint: f64,
    pub glow: f64,
    pub shine: f64,
    pub time: f64,
    /// Sets each piece shimmering on its own beat, and varies its grain.
    pub seed: f64,
}

/// A fixed pseudo-random value in -1..1 for `n`, so a piece is drawn the same way every frame
/// without shading in a smooth ramp.
pub fn hash(n: u32) -> f64 {
    let mut h = n.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA77);
    h ^= h >> 13;
    (h & 0xFFFF) as f64 / 65535.0 * 2.0 - 1.0
}

/// What the particles look like: star glints and embers flying fast, or foam bubbles and
/// ripples drifting slowly.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Finish {
    #[default]
    Glitter,
    Foam,
}

/// What a resting board does: a few pieces at a time glow under slow turning stars, or waves
/// wash gently up over the pieces and leave them wet.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Ambience {
    #[default]
    Starlight,
    Tide,
}

pub fn mix(a: Color, b: Color, t: f64) -> Color {
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

pub fn ellipse(x: f64, y: f64, rx: f64, ry: f64) -> Shape {
    Shape::Ellipse(Rect::new(x - rx, y - ry, rx * 2.0, ry * 2.0))
}

/// An ellipse turned by `angle`, as a polygon (a highlight laid along a facet).
pub fn tilted(x: f64, y: f64, rx: f64, ry: f64, angle: f64) -> Shape {
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
    Bubble,
    Ripple,
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

/// The live sparks. Bounded, and every spark dies within a second and a half, so the board goes
/// back to requesting no frames soon after the last move.
pub struct Sparks {
    list: Vec<Spark>,
    rng: u64,
    pub finish: Finish,
}

const MAX_SPARKS: usize = 260;

impl Sparks {
    pub fn new() -> Self {
        Self {
            list: Vec::new(),
            // Its own generator: sparkle never draws from the game's seeded RNG.
            rng: 0x9E37_79B9_7F4A_7C15,
            finish: Finish::Glitter,
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
        // Foam drifts on: it barely slows, and wobbles as it goes.
        let float = (1.0 - 0.6 * dt).max(0.0);
        for s in &mut self.list {
            s.age += dt;
            if s.age < 0.0 {
                continue;
            }
            s.x += s.vx * dt;
            s.y += s.vy * dt;
            if s.kind == Kind::Bubble {
                s.vx *= float;
                s.vy *= float;
                s.x += (s.spin * 1.3).sin() * 0.04 * dt;
            } else {
                s.vx *= drag;
                s.vy *= drag;
            }
            if s.kind == Kind::Ember {
                s.vy += 3.2 * dt;
            }
            s.spin += dt * 4.0;
        }
        self.list.retain(|s| s.age < s.life);
    }
    /// A moving gem's wake: `amount` sparks on average, glints and embers shed around it.
    pub fn trail(&mut self, x: f64, y: f64, color: Color, amount: f64) {
        if self.finish == Finish::Foam {
            return self.froth(x, y, color, amount * 0.3);
        }
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
        if self.finish == Finish::Foam {
            // A stone lifting out of still water: a slow ripple and a little foam.
            self.push(Spark {
                x,
                y,
                vx: 0.0,
                vy: 0.0,
                age: 0.0,
                life: 1.1,
                size: 0.5 * p.sqrt(),
                spin: 0.0,
                color: mix(color, Color::WHITE, 0.6),
                kind: Kind::Ripple,
            });
            let n = 3 + p as usize;
            for k in 0..n {
                let a = k as f64 / n as f64 * TAU + self.rand();
                let s = Spark {
                    x: x + a.cos() * 0.2,
                    y: y + a.sin() * 0.2,
                    vx: a.cos() * 0.3,
                    vy: a.sin() * 0.3 - 0.1,
                    age: 0.0,
                    life: 1.0 + self.rand() * 0.4,
                    size: 0.04 + self.rand() * 0.04,
                    spin: self.rand() * TAU,
                    color: mix(color, Color::WHITE, 0.75),
                    kind: Kind::Bubble,
                };
                self.push(s);
            }
            return;
        }
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
        if self.finish == Finish::Foam {
            // A line of foam spreading along the row or column, like a wave's edge.
            for k in -4..=4 {
                let (dx, dy) = if horizontal {
                    (k as f64, (self.rand() - 0.5) * 0.3)
                } else {
                    ((self.rand() - 0.5) * 0.3, k as f64)
                };
                let s = Spark {
                    x: x + dx,
                    y: y + dy,
                    vx: dx * 0.08,
                    vy: dy * 0.08 - 0.05,
                    age: -(k as f64).abs() * 0.06,
                    life: 1.1,
                    size: 0.05 + self.rand() * 0.03,
                    spin: self.rand() * TAU,
                    color: mix(color, Color::WHITE, 0.75),
                    kind: Kind::Bubble,
                };
                self.push(s);
            }
            return;
        }
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
        if self.finish == Finish::Foam {
            return;
        }
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
    /// A moving stone's wake: a few foam bubbles left drifting behind it.
    fn froth(&mut self, x: f64, y: f64, color: Color, amount: f64) {
        let mut left = amount;
        while left > 0.0 {
            if left < 1.0 && self.rand() > left {
                break;
            }
            left -= 1.0;
            let s = Spark {
                x: x + (self.rand() - 0.5) * 0.5,
                y: y + (self.rand() - 0.5) * 0.5,
                vx: (self.rand() - 0.5) * 0.12,
                vy: -0.08 - self.rand() * 0.08,
                age: 0.0,
                life: 0.9 + self.rand() * 0.4,
                size: 0.035 + self.rand() * 0.03,
                spin: self.rand() * TAU,
                color: mix(color, Color::WHITE, 0.75),
                kind: Kind::Bubble,
            };
            self.push(s);
        }
    }
    /// Paint every spark over a board whose origin is (ox, oy) and whose cells are `c` wide.
    pub fn draw(&self, d: &mut Draw, ox: f64, oy: f64, c: f64) {
        for s in self.list.iter().filter(|s| s.age >= 0.0) {
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
                Kind::Bubble => {
                    let a = (1.0 - t) * (t * 8.0).min(1.0);
                    let r = s.size * c * (0.8 + 0.4 * t);
                    d.fill(ellipse(x, y, r, r), s.color.with_alpha(0.14 * a));
                    d.stroke(ellipse(x, y, r, r), Color::WHITE.with_alpha(0.5 * a), 0.8);
                    d.fill(
                        ellipse(x - r * 0.35, y - r * 0.35, r * 0.25, r * 0.25),
                        Color::WHITE.with_alpha(0.7 * a),
                    );
                }
                Kind::Ripple => {
                    let ease = 1.0 - (1.0 - t).powi(2);
                    let r = (0.3 + ease * 0.9) * s.size * c * 1.6;
                    let a = (1.0 - t).powf(1.5);
                    d.stroke(
                        ellipse(x, y, r, r * 0.82),
                        s.color.with_alpha(0.45 * a),
                        c * 0.025 + 0.5,
                    );
                    d.stroke(
                        ellipse(x, y, r * 0.66, r * 0.66 * 0.82),
                        s.color.with_alpha(0.25 * a),
                        c * 0.018 + 0.5,
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

/// A wave washing up the board from the bottom edge: it runs up to `reach` cells, then drains
/// back a little and fades over `dur` seconds.
struct Wave {
    age: f64,
    dur: f64,
    reach: f64,
    phase: f64,
}

impl Wave {
    /// How far up the board, in cells from the bottom, the water reaches at column `x`.
    fn height(&self, x: f64) -> f64 {
        let t = self.age / self.dur;
        let run = if t < 0.5 {
            1.0 - (1.0 - t / 0.5).powi(3)
        } else {
            1.0 - 0.45 * ((t - 0.5) / 0.5).powi(2)
        };
        self.reach * run + 0.22 * (x * 1.1 + self.phase + self.age * 0.7).sin() - 0.3
    }
    /// How strongly it shows: full while it runs up, fading as it drains away.
    fn alpha(&self) -> f64 {
        let t = self.age / self.dur;
        (t / 0.08).min(1.0) * if t < 0.5 { 1.0 } else { 1.0 - (t - 0.5) / 0.5 }
    }
}

/// The resting board's ambience (see [`Ambience`]). Starlight: a few gems at a time slowly
/// brighten under a turning star and fade as others take over, while motes rise from them like
/// dust in a beam. Tide: overlapping waves wash up the board, leave the stones they cover wet,
/// and scatter foam. Either fades in over a couple of seconds and out as soon as play resumes.
pub struct Idle {
    pub ambience: Ambience,
    stars: Vec<Star>,
    motes: Vec<Mote>,
    waves: Vec<Wave>,
    wet: Vec<f64>,
    next_wave: f64,
    side: f64,
    fade: f64,
    rng: u64,
}

impl Idle {
    pub fn new() -> Self {
        Self {
            ambience: Ambience::Starlight,
            stars: Vec::new(),
            motes: Vec::new(),
            waves: Vec::new(),
            wet: Vec::new(),
            next_wave: 0.0,
            side: 0.0,
            fade: 0.0,
            rng: 0xD1B5_4A32_D192_ED03,
        }
    }
    /// Switch to `ambience`, starting over if it changed.
    pub fn set_ambience(&mut self, ambience: Ambience) {
        if self.ambience != ambience {
            *self = Self {
                ambience,
                rng: self.rng,
                ..Self::new()
            };
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
    /// How strongly the piece in `cell` shows the ambience now, 0..1: how brightly a gem glows,
    /// or how wet a stone is.
    pub fn light(&self, cell: usize) -> f64 {
        if self.ambience == Ambience::Tide {
            return self.wet.get(cell).copied().unwrap_or(0.0) * self.fade;
        }
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
        for w in &mut self.waves {
            w.age += dt;
        }
        self.waves.retain(|w| w.age < w.dur);
        self.side = side;
        let Some(gems) = rest else {
            self.fade = (self.fade - dt * 3.0).max(0.0);
            if self.fade == 0.0 {
                self.stars.clear();
                self.motes.clear();
                self.waves.clear();
                self.wet.clear();
                self.next_wave = 0.0;
            }
            return;
        };
        self.fade = (self.fade + dt / 2.5).min(1.0);
        if self.ambience == Ambience::Tide {
            return self.tide(dt, gems, side);
        }
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
    fn tide(&mut self, dt: f64, gems: &[Resting], side: f64) {
        // Waves overlap, so the water is never quite still.
        self.next_wave -= dt;
        if self.next_wave <= 0.0 {
            let wave = Wave {
                age: 0.0,
                dur: 4.8 + self.rand() * 1.6,
                reach: side * (0.5 + self.rand() * 0.55),
                phase: self.rand() * TAU,
            };
            self.waves.push(wave);
            self.next_wave = 3.2 + self.rand() * 2.2;
        }
        // Stones under the water wet quickly and dry slowly.
        if let Some(last) = gems.iter().map(|g| g.cell).max() {
            self.wet.resize(self.wet.len().max(last + 1), 0.0);
        }
        for g in gems {
            let covered = self
                .waves
                .iter()
                .any(|w| w.alpha() > 0.3 && side - g.y < w.height(g.x));
            let wet = &mut self.wet[g.cell];
            *wet = if covered {
                (*wet + dt * 2.5).min(1.0)
            } else {
                (*wet - dt / 4.0).max(0.0)
            };
        }
        // Foam fizzes along each running wave's edge.
        let edges: Vec<(f64, f64, f64, f64)> = self
            .waves
            .iter()
            .filter(|w| w.age < w.dur * 0.55)
            .map(|w| (w.reach, w.phase, w.age, w.dur))
            .collect();
        for (reach, phase, age, dur) in edges {
            if self.rand() < 3.0 * dt && self.motes.len() < MAX_MOTES {
                let x = self.rand() * side;
                let h = Wave {
                    age,
                    dur,
                    reach,
                    phase,
                }
                .height(x);
                let m = Mote {
                    x,
                    y: side - h,
                    vx: (self.rand() - 0.5) * 0.1,
                    vy: 0.03,
                    age: 0.0,
                    life: 1.6 + self.rand() * 1.0,
                    size: 0.02 + self.rand() * 0.02,
                    phase: self.rand() * TAU,
                    color: Color::WHITE,
                };
                self.motes.push(m);
            }
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
    /// Paint what the ambience draws over the pieces (the starlight's motes and stars) on a
    /// board whose origin is (ox, oy) and whose cells are `c` wide. The tide draws behind them,
    /// in [`Idle::draw_back`].
    pub fn draw(&self, d: &mut Draw, ox: f64, oy: f64, c: f64) {
        if self.fade <= 0.0 {
            return;
        }
        if self.ambience == Ambience::Tide {
            return;
        }
        self.draw_motes(d, ox, oy, c, 1.0);
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
    /// The drifting motes (dust in the starlight, foam on the tide), at `strength` of full.
    fn draw_motes(&self, d: &mut Draw, ox: f64, oy: f64, c: f64, strength: f64) {
        for m in &self.motes {
            let t = m.age / m.life;
            let a = strength
                * self.fade
                * (t * PI).sin()
                * (0.55 + 0.45 * (m.phase + m.age * 4.5).sin());
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
    }
    /// What the ambience draws behind the pieces: the tide, as a quiet background over the
    /// board's cells, so the stones sit in the water rather than under a sheet of it.
    pub fn draw_back(&self, d: &mut Draw, ox: f64, oy: f64, c: f64) {
        if self.fade <= 0.0 || self.ambience != Ambience::Tide {
            return;
        }
        self.draw_waves(d, ox, oy, c);
        self.draw_motes(d, ox, oy, c, 0.5);
    }
    /// Each wave as a sheet of clear water over the lower board, brightest at its foamy edge.
    fn draw_waves(&self, d: &mut Draw, ox: f64, oy: f64, c: f64) {
        let side = self.side;
        let board = Shape::RoundedRect(Rect::new(ox, oy, side * c, side * c), 12.0);
        d.clipped(board, |d| {
            for w in &self.waves {
                let a = w.alpha() * self.fade;
                if a <= 0.01 {
                    continue;
                }
                let edge: Vec<Point> = (0..=28)
                    .map(|k| {
                        let x = side * k as f64 / 28.0;
                        let h = w.height(x).max(0.0);
                        Point::new(ox + x * c, oy + (side - h) * c)
                    })
                    .collect();
                let mut sheet = vec![Point::new(ox, oy + side * c)];
                sheet.extend(edge.iter().copied());
                sheet.push(Point::new(ox + side * c, oy + side * c));
                d.fill(
                    Shape::Polygon(sheet),
                    LinearGradient::vertical(
                        Color::hex(0xD8F2F6).with_alpha(0.07 * a),
                        Color::hex(0x9FD3DE).with_alpha(0.015 * a),
                    ),
                );
                // The foam line and a fainter one just below, each a single ribbon so no joints
                // show.
                let ribbon = |drop: f64, width: f64| -> Shape {
                    let mut band: Vec<Point> = edge
                        .iter()
                        .map(|p| Point::new(p.x, p.y + drop - width / 2.0))
                        .collect();
                    band.extend(
                        edge.iter()
                            .rev()
                            .map(|p| Point::new(p.x, p.y + drop + width / 2.0)),
                    );
                    Shape::Polygon(band)
                };
                d.fill(
                    ribbon(0.0, c * 0.035 + 0.5),
                    Color::WHITE.with_alpha(0.2 * a),
                );
                d.fill(
                    ribbon(c * 0.1, c * 0.025 + 0.5),
                    Color::WHITE.with_alpha(0.08 * a),
                );
            }
        });
    }
}
