// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Impact particles and a tapered saber trail.
use crate::{
    model::{FOLLOW_THROUGH, Game, WINDUP, blade_point, world},
    sprite,
};
use day_core::Bitmap;
use day_pieces::prelude::*;
use gamekit::{
    board_game::{circle, line},
    multiplayer::Vec2,
};
fn soft(d: &mut Draw, p: Point, r: f64, rgb: [f64; 3], alpha: f64) {
    // Layer translucent discs to soften the particle edge.
    for i in (1..=12).rev() {
        circle(
            d,
            p,
            r * i as f64 / 12.0,
            Color::rgba(rgb[0], rgb[1], rgb[2], alpha / 6.0),
        );
    }
}
impl Game {
    pub(crate) fn swing_trails(&self, d: &mut Draw, board: Affine) {
        const LIFETIME: f64 = 0.108;
        for (player, sword) in self.swords[..self.count].iter().enumerate() {
            let stop = if sword.failed && sword.hit {
                sword.hit_age
            } else {
                FOLLOW_THROUGH
            };
            if !sword.active || !(WINDUP..stop + LIFETIME).contains(&sword.age) {
                continue;
            }
            // Each strip follows a different point on the curved blade. Offset the
            // narrower streaks in time so their tapered ends don't all line up.
            let edge = &crate::model::EDGE;
            for (anchor, width, lag, span, opacity) in [
                (edge[6], 0.46, 0.000, 0.095, 0.30),
                (edge[8], 0.036, 0.007, 0.087, 0.88),
                (edge[6], 0.042, 0.015, 0.092, 0.72),
                (edge[4], 0.028, 0.009, 0.063, 0.56),
            ] {
                let head = (sword.age - lag).min(stop);
                let tail = (sword.age - lag - span).max(WINDUP);
                if tail >= head {
                    continue;
                }
                let anchor = blade_point(anchor);
                let radial = anchor.normalized();
                let mut outside = [Point::new(0., 0.); 15];
                let mut path = PathBuilder::new();
                let mut brightness: f64 = 0.;
                for (i, outer) in outside.iter_mut().enumerate() {
                    let t = tail + (head - tail) * i as f64 / 14.;
                    let life = (1.0 - (sword.age - lag - t) / span).clamp(0., 1.);
                    let phase = (t - WINDUP) / (FOLLOW_THROUGH - WINDUP);
                    let weight = life * 4.0 * phase * (1.0 - phase);
                    brightness = brightness.max(weight);
                    let offset = radial * (width * weight);
                    let angle = sword.angle_at(t);
                    let point = |v| {
                        let v = world(player, angle, v);
                        board.apply(Point::new(v.x, v.y))
                    };
                    let inner = point(anchor - offset);
                    path = if i == 0 {
                        path.move_to(inner)
                    } else {
                        path.line_to(inner)
                    };
                    *outer = point(anchor + offset);
                }
                for point in outside.into_iter().rev() {
                    path = path.line_to(point);
                }
                d.fill(
                    path.close().build(),
                    Color::rgba(0.87, 0.98, 1.0, opacity * brightness),
                );
            }
        }
    }
    pub(crate) fn effects(&self, d: &mut Draw, images: &[Option<Bitmap>]) {
        for sword in &self.swords[..self.count] {
            if sword.failed && sword.hit {
                let t = (sword.age - sword.hit_age) / 0.24;
                if (0.0..1.0).contains(&t) {
                    let direction = sword.contact.y.atan2(sword.contact.x);
                    for ray in 0..3 {
                        let angle = direction + (ray as f64 - 1.) * 0.83;
                        let point = |r| {
                            Point::new(
                                sword.contact.x + r * angle.cos(),
                                sword.contact.y + r * angle.sin(),
                            )
                        };
                        line(
                            d,
                            point(0.06 + 0.55 * t),
                            point(0.36 + 0.65 * t),
                            0.07,
                            Color::rgba(1.0, 0.85, 0.58, 1.0 - t),
                        );
                    }
                }
            }
        }

        let age = self.split_age;
        if !(0.0..0.8).contains(&age) {
            return;
        }
        let color = [[1.0, 0.66, 0.26], [0.90, 0.31, 0.46], [0.99, 0.53, 0.44]][self.item.min(2)];
        let flash = (1.0 - age / 0.22).max(0.0);
        if flash > 0.0 {
            soft(
                d,
                Point::new(0.0, 0.0),
                1.75 - flash,
                [1.0, 0.94, 0.79],
                flash * 0.48,
            );
        }
        for i in 0..9 {
            // The lattice staggers size and flight time without advancing the game RNG.
            let phase = ((i * 5) % 9) as f64 / 9.0;
            let t = (age / (0.45 + 0.35 * phase)).min(1.0);
            let angle = self.item_angle.to_radians() + i as f64 * std::f64::consts::TAU / 9.0;
            let travel = 0.55 + (2.1 + phase) * t;
            let at = Vec2::new(angle.cos() * travel, angle.sin() * travel - t * t * 0.9);
            let opacity = (1.0 - t) * (t * 16.0).min(1.0);
            if i % 3 == 0 {
                let side = 0.30 + 0.2 * phase;
                let star = crate::model::Sprite {
                    size: [side, side],
                    pivot: [0.5, 0.5],
                };
                sprite(d, images, 14, &star, at, 1.0, angle + t, opacity);
            } else {
                let radius = (0.10 + phase * 0.08) * (1.0 - t * 0.5);
                d.transformed(
                    Affine::scale(1.7, 1.0)
                        .then(Affine::rotate(Angle::rad(angle)))
                        .then(Affine::translate(at.x, at.y)),
                    |d| soft(d, Point::new(0.0, 0.0), radius, color, opacity),
                );
            }
        }
    }
}
