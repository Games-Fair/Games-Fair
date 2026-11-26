// SPDX-License-Identifier: GPL-2.0-or-later
//! Pocket hardware and transient ball-drop presentation, independent of scoring and physics.
use crate::model::{BALL_R, HEIGHT, PocketEvent, Vec2, WIDTH, pockets};
use crate::rolling::Rotation;
use day_pieces::prelude::*;
use day_spec::UnitPoint;
use game_resources::res;
use gamekit::chrome;
use std::f64::consts::{PI, TAU};

pub const LAND_AT: f64 = 0.28;
pub const DROP_TIME: f64 = 0.66;
#[derive(Clone)]
pub struct Drop {
    pub event: PocketEvent,
    pub age: f64,
}
impl Drop {
    pub fn new(event: PocketEvent) -> Self {
        Self { event, age: 0.0 }
    }
    pub fn alive(&self) -> bool {
        self.age < DROP_TIME
    }
    /// The ball reaches the throat before shrinking into the dark, keeping the entry continuous.
    pub fn pose(&self) -> (Vec2, f64, f64, Rotation) {
        let target = pockets()[self.event.pocket];
        // Cross the cloth edge at full size. Only fall once the ball is centered in the mouth.
        let travel = (self.age / 0.19).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - travel).powi(3);
        let depth = ((self.age - 0.19) / 0.37).clamp(0.0, 1.0);
        let position = Vec2::new(
            self.event.entry.x + (target.x - self.event.entry.x) * ease,
            self.event.entry.y + (target.y - self.event.entry.y) * ease + 3.0 * depth,
        );
        let mut orientation = self.event.orientation;
        orientation.roll(
            position.x - self.event.entry.x,
            position.y - self.event.entry.y,
        );
        // Retain a little angular momentum during the fall, without resetting the ball's markings.
        let falling = (self.age - 0.19).clamp(0.0, 0.37);
        orientation.roll(
            self.event.velocity.x * falling * 0.22,
            self.event.velocity.y * falling * 0.22,
        );
        (
            position,
            BALL_R * (1.0 - depth).powf(1.3),
            depth * 0.93,
            orientation,
        )
    }
}
fn circle(d: &mut Draw, p: Vec2, r: f64, color: Color) {
    d.fill(
        Shape::Ellipse(Rect::new(p.x - r, p.y - r, r * 2.0, r * 2.0)),
        color,
    );
}
fn arc(d: &mut Draw, p: Vec2, r: f64, start: f64, end: f64, color: Color, width: f64) {
    let mut path =
        PathBuilder::new().move_to(Point::new(p.x + r * start.cos(), p.y + r * start.sin()));
    for i in 1..=24 {
        let a = start + (end - start) * i as f64 / 24.0;
        path = path.line_to(Point::new(p.x + r * a.cos(), p.y + r * a.sin()));
    }
    d.stroke(path.build(), color, width);
}
/// The rail leather is flush; the bevel, shadows and net are entirely inside the cutout.
pub fn pocket(d: &mut Draw, index: usize, drops: &[Drop]) {
    let p = pockets()[index];
    let inward = (HEIGHT / 2.0 - p.y).atan2(WIDTH / 2.0 - p.x);
    let net_motion = drops
        .iter()
        .filter(|x| x.event.pocket == index && x.age >= LAND_AT)
        .map(|x| {
            let t = x.age - LAND_AT;
            (t * 32.0).sin() * (-t * 12.0).exp()
        })
        .sum::<f64>()
        .clamp(-1.0, 1.0);
    // Leather and stitching only on the rail side: no ring or cast shadow on the playing cloth.
    arc(
        d,
        p,
        25.8,
        inward + PI - 1.45,
        inward + PI + 1.45,
        Color::hex(0x594330),
        3.0,
    );
    for j in 0..17 {
        let a = inward + PI - 1.4 + j as f64 * 2.8 / 16.0;
        let start = Point::new(p.x + 26.0 * a.cos(), p.y + 26.0 * a.sin());
        let end = Point::new(
            p.x + 26.0 * (a + 0.026).cos(),
            p.y + 26.0 * (a + 0.026).sin(),
        );
        d.stroke(
            Shape::Line(start, end),
            Color::rgba(0.78, 0.63, 0.42, 0.6),
            0.65,
        );
    }
    let opening = Shape::Ellipse(Rect::new(p.x - 25.0, p.y - 25.0, 50.0, 50.0));
    d.fill(
        opening.clone(),
        LinearGradient::vertical(Color::hex(0x071A19), Color::hex(0x406356)),
    );
    // The dark upper wall and softly lit lower wall establish a recessed opening.
    circle(d, Vec2::new(p.x, p.y + 1.5), 22.8, Color::hex(0x07100F));
    d.clipped(
        Shape::Ellipse(Rect::new(p.x - 21.5, p.y - 20.0, 43.0, 43.0)),
        |d| {
            for k in -4..=4 {
                for direction in [-1.0, 1.0] {
                    d.stroke(
                        Shape::Line(
                            Point::new(p.x + k as f64 * 6.0 - 22.0 * direction, p.y - 6.0),
                            Point::new(
                                p.x + k as f64 * 6.0 + 22.0 * direction,
                                p.y + 30.0 + net_motion * 2.0,
                            ),
                        ),
                        Color::rgba(0.46, 0.43, 0.33, 0.16),
                        0.7,
                    );
                }
            }
            d.fill(
                Shape::Ellipse(Rect::new(p.x - 23.0, p.y - 23.0, 46.0, 46.0)),
                LinearGradient::vertical(
                    Color::rgba(0.0, 0.015, 0.01, 0.90),
                    Color::rgba(0.0, 0.0, 0.0, 0.0),
                ),
            );
        },
    );
    arc(
        d,
        p,
        24.7,
        PI + 0.12,
        TAU - 0.12,
        Color::rgba(0.0, 0.0, 0.0, 0.35),
        0.65,
    );
    arc(
        d,
        p,
        24.8,
        0.15,
        PI - 0.15,
        Color::rgba(0.62, 0.80, 0.67, 0.20),
        0.55,
    );
}

/// Project a spherical cap, clipping at the sphere's horizon (not a flat, rotating sticker).
fn cap_outline(n: [f64; 3], u: [f64; 3], v: [f64; 3], cosine: f64) -> Vec<[f64; 3]> {
    let sine = (1.0 - cosine * cosine).sqrt();
    if n[2] < -sine {
        return Vec::new();
    }
    let ring: Vec<_> = (0..48)
        .map(|i| {
            let (s, c) = (i as f64 * TAU / 48.0).sin_cos();
            std::array::from_fn(|j| n[j] * cosine + sine * (u[j] * c + v[j] * s))
        })
        .collect();
    let mut clipped = Vec::new();
    let mut previous = ring[47];
    for current in ring {
        if (previous[2] >= 0.0) != (current[2] >= 0.0) {
            let t = previous[2] / (previous[2] - current[2]);
            let mut edge: [f64; 3] =
                std::array::from_fn(|j| previous[j] + t * (current[j] - previous[j]));
            let length = edge[0].hypot(edge[1]);
            edge = [edge[0] / length, edge[1] / length, 0.0];
            clipped.push(edge);
        }
        if current[2] >= 0.0 {
            clipped.push(current);
        }
        previous = current;
    }
    if clipped.is_empty() {
        return clipped;
    }
    let mut outline = Vec::new();
    for i in 0..clipped.len() {
        let a = clipped[i];
        let b = clipped[(i + 1) % clipped.len()];
        outline.push(a);
        if a[2].abs() < 1e-9 && b[2].abs() < 1e-9 {
            let start = a[1].atan2(a[0]);
            let end = b[1].atan2(b[0]);
            let angle = (end - start + PI).rem_euclid(TAU) - PI;
            for j in 1..12 {
                let t = start + angle * j as f64 / 12.0;
                outline.push([t.cos(), t.sin(), 0.0]);
            }
        }
    }
    outline
}
fn cap(p: Vec2, r: f64, n: [f64; 3], u: [f64; 3], v: [f64; 3], cosine: f64) -> Option<Shape> {
    let points = cap_outline(n, u, v, cosine);
    let first = points.first()?;
    let project = |v: &[f64; 3]| Point::new(p.x + r * v[0], p.y + r * v[1]);
    let mut path = PathBuilder::new().move_to(project(first));
    for point in &points[1..] {
        path = path.line_to(project(point));
    }
    Some(path.line_to(project(first)).build())
}
/// Conventional 8-ball colors: stripes 9–15 repeat the solid 1–7 palette.
fn ball_color(number: usize) -> Color {
    const PALETTE: [u32; 7] = [
        0xF4C52B, 0x2464C4, 0xD83935, 0x7042A0, 0xED8024, 0x258454, 0x842D40,
    ];
    Color::hex(PALETTE[(number - 1) % 8])
}
/// Surface decoration rotates in 3D; illumination and the specular reflection stay in world space.
pub fn ball(
    d: &mut Draw,
    p: Vec2,
    r: f64,
    index: usize,
    orientation: Rotation,
    darkness: f64,
    shadow: bool,
) {
    if r < 0.25 {
        return;
    }
    let ivory = Color::hex(0xF6F3E6);
    let color = if index == 0 {
        ivory
    } else if index == 8 {
        Color::hex(0x20232B)
    } else {
        ball_color(index)
    };
    if shadow {
        d.fill(
            Shape::Ellipse(Rect::new(p.x - r + 2.5, p.y - r + 3.0, r * 2.0, r * 2.0)),
            RadialGradient::centered(
                Color::rgba(0.0, 0.0, 0.0, 0.30),
                Color::rgba(0.0, 0.0, 0.0, 0.0),
            ),
        );
    }
    let disk = Shape::Ellipse(Rect::new(p.x - r, p.y - r, r * 2.0, r * 2.0));
    d.fill(disk.clone(), color);
    d.clipped(disk.clone(), |d| {
        if index > 8 {
            // A genuine band around the sphere makes the striped group readable from every angle.
            for sign in [-1.0, 1.0] {
                if let Some(shape) = cap(
                    p,
                    r,
                    orientation.apply([0.0, sign, 0.0]),
                    orientation.apply([1.0, 0.0, 0.0]),
                    orientation.apply([0.0, 0.0, sign]),
                    0.58,
                ) {
                    d.fill(shape, ivory);
                }
            }
        }
        if index > 0 {
            for sign in [-1.0, 1.0] {
                let n = orientation.apply([0.0, 0.0, sign]);
                let u = orientation.apply([sign, 0.0, 0.0]);
                let v = orientation.apply([0.0, 1.0, 0.0]);
                if let Some(patch) = cap(p, r, n, u, v, 0.89) {
                    d.fill(patch.clone(), ivory);
                    if n[2] > 0.035 && r > 2.0 {
                        d.clipped(patch, |d| {
                            d.transformed(
                                Affine {
                                    a: u[0],
                                    b: u[1],
                                    c: v[0],
                                    d: v[1],
                                    tx: p.x + r * 0.89 * n[0],
                                    ty: p.y + r * 0.89 * n[1],
                                },
                                |d| {
                                    d.text(
                                        &res::str::party_count(index as f64).format(),
                                        Point::new(0.0, 0.0),
                                        TextStyle {
                                            size: r * if index >= 10 { 0.59 } else { 0.72 },
                                            color: Color::hex(0x191C23).into(),
                                            anchor: TextAnchor::CENTERED,
                                            font: chrome::canvas_font(FontWeight::Bold),
                                        },
                                    )
                                },
                            )
                        });
                    }
                }
            }
        }
        d.fill(
            disk.clone(),
            RadialGradient::new(
                UnitPoint::new(0.30, 0.24),
                0.80,
                [
                    (0.0, Color::rgba(1.0, 1.0, 1.0, 0.16)),
                    (0.40, Color::rgba(0.0, 0.0, 0.0, 0.0)),
                    (0.73, Color::rgba(0.0, 0.0, 0.0, 0.07)),
                    (1.0, Color::rgba(0.0, 0.0, 0.0, 0.44)),
                ],
            ),
        );
    });
    d.stroke(disk.clone(), Color::rgba(0.03, 0.08, 0.08, 0.65), r * 0.045);
    d.fill(
        Shape::Ellipse(Rect::new(
            p.x - r * 0.48,
            p.y - r * 0.56,
            r * 0.30,
            r * 0.18,
        )),
        Color::rgba(1.0, 1.0, 1.0, 0.65),
    );
    if darkness > 0.0 {
        d.fill(disk, Color::rgba(0.01, 0.02, 0.03, darkness));
    }
}
pub fn drop(d: &mut Draw, drop: &Drop, reduced_motion: bool) {
    let (mut position, mut radius, mut darkness, mut orientation) = drop.pose();
    if reduced_motion {
        position = pockets()[drop.event.pocket];
        radius = BALL_R * 0.75;
        darkness = (drop.age / 0.22).min(1.0);
        orientation = Rotation::default();
    }
    if darkness >= 1.0 {
        return;
    }
    // No foreground rim or circular mask: the whole ball crosses the edge before sinking.
    ball(
        d,
        position,
        radius,
        drop.event.ball,
        orientation,
        darkness,
        false,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ball_enters_continuously_then_sinks_below_the_lip_and_expires() {
        let mut drop = Drop::new(PocketEvent {
            ball: 1,
            pocket: 0,
            entry: Vec2::new(33.0, 33.0),
            orientation: Rotation::default(),
            velocity: Vec2::new(-100.0, -140.0),
        });
        let (start, r, shade, _) = drop.pose();
        assert_eq!(start.distance(drop.event.entry), 0.0);
        assert_eq!(r, BALL_R);
        assert_eq!(shade, 0.0);
        drop.age = LAND_AT;
        let (landing, r, shade, _) = drop.pose();
        assert!(landing.distance(pockets()[0]) < 5.0);
        assert!(r < BALL_R && r > 0.0);
        assert!(shade > 0.0);
        drop.age = DROP_TIME;
        assert!(!drop.alive());
        assert_eq!(drop.pose().1, 0.0);
    }
    #[test]
    fn surface_patches_disappear_behind_the_ball_and_follow_its_curved_horizon() {
        assert!(cap_outline([0.0, 0.0, -1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.89).is_empty());
        let edge = cap_outline([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0], 0.89);
        assert!(!edge.is_empty());
        for p in &edge {
            assert!(p[2] >= -1e-9);
            assert!(p[0] >= 0.89 - 1e-9);
            assert!(p[0] * p[0] + p[1] * p[1] <= 1.0 + 1e-9);
        }
        assert!(edge.iter().filter(|p| p[2] == 0.0).count() > 8);
    }
    #[test]
    fn pocket_entry_keeps_full_ball_size_until_it_clears_the_edge() {
        let drop = Drop {
            event: PocketEvent {
                ball: 12,
                pocket: 2,
                entry: Vec2::new(40.0, 280.0),
                velocity: Vec2::new(-150.0, 0.0),
                orientation: Rotation::default(),
            },
            age: 0.18,
        };
        let (position, r, darkness, _) = drop.pose();
        assert_eq!(r, BALL_R);
        assert_eq!(darkness, 0.0);
        assert!(position.distance(pockets()[2]) + r < 22.8);
    }
}
