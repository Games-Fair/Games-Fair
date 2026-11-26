// SPDX-License-Identifier: GPL-2.0-or-later
//! Contact against the painted gift, flower and beach ball in their rotated artwork coordinates.
use super::model::{EDGE, blade_point, world};
use gamekit::multiplayer::Vec2;

fn ellipse(x: f64, y: f64, rx: f64, ry: f64) -> bool {
    (x / rx).powi(2) + (y / ry).powi(2) <= 1.
}

fn rounded_box(x: f64, y: f64, w: f64, h: f64, corner: f64) -> bool {
    let dx = (x.abs() - (w - corner)).max(0.);
    let dy = (y.abs() - (h - corner)).max(0.);
    dx * dx + dy * dy <= corner * corner
}

fn contains(item: usize, x: f64, y: f64) -> bool {
    // Origin at the center of the 100-unit SVG, positive y towards its bottom.
    match item {
        3 => {
            rounded_box(x, y - 11.5, 37., 29., 7.5)
                || rounded_box(x, y + 12.5, 42.5, 11., 5.5)
                || ellipse(x.abs() - 13., y + 30., 13., 10.5)
        }
        4 => {
            // Reflect the five identical petals into the top petal's sector.
            let sector = std::f64::consts::TAU / 5.;
            let theta = (x.atan2(-y) + sector * 0.5).rem_euclid(sector) - sector * 0.5;
            let r = x.hypot(y);
            r <= 19.5 || ellipse(r * theta.sin(), r * theta.cos() - 23., 14.5, 23.5)
        }
        5 => x * x + y * y <= 46_f64.powi(2),
        _ => false,
    }
}

/// Sample the two flanks of the blade more finely than one unit of the object illustration.
pub(super) fn contact(
    item: usize,
    rotation: (f64, f64),
    player: usize,
    angle: f64,
) -> Option<Vec2> {
    const SPINE: [[f64; 2]; 6] = [
        [92., 5.],
        [73., 43.],
        [60., 78.],
        [51., 108.],
        [47., 143.],
        [47., 181.],
    ];
    let transform = |pt| world(player, angle, blade_point(pt));
    let (sin, cos) = rotation;
    for flank in [&EDGE[..], &SPINE[..]] {
        let mut start = transform(flank[0]);
        for &end in &flank[1..] {
            let end = transform(end);
            let delta = end - start;
            let t = (-start.dot(delta) / delta.dot(delta)).clamp(0., 1.);
            if (start + delta * t).length() <= 0.94 {
                let steps = (delta.length() / 0.016).ceil() as usize;
                for i in 0..=steps {
                    let q = start + delta * (i as f64 / steps as f64);
                    let x = (cos * q.x + sin * q.y) / 0.016;
                    let y = (sin * q.x - cos * q.y) / 0.016;
                    if contains(item, x, y) {
                        return Some(q);
                    }
                }
            }
            start = end;
        }
    }
    None
}
