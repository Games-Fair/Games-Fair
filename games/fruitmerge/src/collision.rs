// SPDX-License-Identifier: GPL-2.0-or-later
//! Convex contact outlines sampled from the illustrated fruit skins. Two round fruits use
//! circles. Angular impulses let uneven fruit rock.
use crate::model::{Fruit, SIZES};
use gamekit::multiplayer::Vec2;
pub fn outline(f: &Fruit, shapes: &[Vec<Vec2>]) -> Vec<Vec2> {
    let Some(shape) = shapes.get(f.kind).filter(|s| !s.is_empty()) else {
        return vec![];
    };
    let (sin, cos) = f.angle.sin_cos();
    shape
        .iter()
        .map(|v| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos) * SIZES[f.kind] + f.p)
        .collect()
}
fn projection(points: &[Vec2], f: &Fruit, n: Vec2) -> (f64, f64) {
    if points.is_empty() {
        let c = f.p.dot(n);
        return (c - f.radius(), c + f.radius());
    }
    points
        .iter()
        .map(|p| p.dot(n))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
            (a.min(v), b.max(v))
        })
}
fn support(points: &[Vec2], f: &Fruit, n: Vec2) -> Vec2 {
    if points.is_empty() {
        return f.p + n * f.radius();
    }
    let max = points
        .iter()
        .map(|p| p.dot(n))
        .fold(f64::NEG_INFINITY, f64::max);
    let mut sum = Vec2::default();
    let mut count = 0.0_f64;
    for p in points {
        if max - p.dot(n) < 0.8 {
            sum = sum + *p;
            count += 1.0;
        }
    }
    sum / count.max(1.0)
}
pub fn contact(a: &Fruit, b: &Fruit, ap: &[Vec2], bp: &[Vec2]) -> Option<(Vec2, f64, Vec2)> {
    let delta = b.p - a.p;
    if delta.length() > (a.radius() + b.radius()) * 1.42 {
        return None;
    }
    if ap.is_empty() && bp.is_empty() {
        let dist = delta.length();
        let depth = a.radius() + b.radius() - dist;
        return (depth > 0.0).then(|| {
            let n = if dist > 1e-6 {
                delta / dist
            } else {
                Vec2::new(1.0, 0.0)
            };
            (n, depth, a.p + n * a.radius())
        });
    }
    let mut best = (f64::INFINITY, Vec2::new(1.0, 0.0));
    let mut axis = |n: Vec2| {
        let n = n.normalized();
        let (amin, amax) = projection(ap, a, n);
        let (bmin, bmax) = projection(bp, b, n);
        let depth = amax.min(bmax) - amin.max(bmin);
        if depth <= 0.0 {
            return false;
        }
        if depth < best.0 {
            best = (depth, n);
        }
        true
    };
    for poly in [ap, bp] {
        for i in 0..poly.len() {
            let edge = poly[(i + 1) % poly.len()] - poly[i];
            if !axis(Vec2::new(-edge.y, edge.x)) {
                return None;
            }
        }
    }
    for (center, poly) in [(a.p, bp), (b.p, ap)] {
        if let Some(v) = poly
            .iter()
            .min_by(|x, y| x.distance(center).total_cmp(&y.distance(center)))
            && !axis(*v - center)
        {
            return None;
        }
    }
    let n = if best.1.dot(delta) < 0.0 {
        best.1 * -1.0
    } else {
        best.1
    };
    let point = (support(ap, a, n) + support(bp, b, n * -1.0)) * 0.5;
    Some((n, best.0, point))
}
fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}
fn angular(v: Vec2, w: f64) -> Vec2 {
    Vec2::new(-v.y * w, v.x * w)
}
pub fn resolve(a: &mut Fruit, b: &mut Fruit, n: Vec2, depth: f64, point: Vec2) -> f64 {
    let ma = a.mass();
    let mb = b.mass();
    let ia = ma * a.radius().powi(2) * 0.5;
    let ib = mb * b.radius().powi(2) * 0.5;
    let ra = point - a.p;
    let rb = point - b.p;
    let pen = (depth - 0.05).max(0.0) * 0.65 / (1.0 / ma + 1.0 / mb);
    a.p = a.p - n * (pen / ma);
    b.p = b.p + n * (pen / mb);
    let relative = (b.v + angular(rb, b.spin)) - (a.v + angular(ra, a.spin));
    let vn = relative.dot(n);
    if vn >= 0.0 {
        return 0.0;
    }
    let inv = 1.0 / ma + 1.0 / mb + cross(ra, n).powi(2) / ia + cross(rb, n).powi(2) / ib;
    let impulse = -vn * 1.06 / inv;
    let tangent = Vec2::new(-n.y, n.x);
    let t_inv =
        1.0 / ma + 1.0 / mb + cross(ra, tangent).powi(2) / ia + cross(rb, tangent).powi(2) / ib;
    let friction = (-relative.dot(tangent) / t_inv).clamp(-impulse * 0.35, impulse * 0.35);
    let force = n * impulse + tangent * friction;
    a.v = a.v - force / ma;
    b.v = b.v + force / mb;
    a.spin = (a.spin - cross(ra, force) / ia).clamp(-25.0, 25.0);
    b.spin = (b.spin + cross(rb, force) / ib).clamp(-25.0, 25.0);
    vn.abs()
}
pub fn walls(f: &mut Fruit, points: &[Vec2], width: f64, floor: f64) -> f64 {
    let mut impact = 0.0_f64;
    for (n, limit) in [
        (Vec2::new(-1.0, 0.0), 0.0),
        (Vec2::new(1.0, 0.0), width),
        (Vec2::new(0.0, 1.0), floor),
    ] {
        let point = support(points, f, n);
        let penetration = point.dot(n) - limit;
        if penetration <= 0.0 {
            continue;
        }
        f.p = f.p - n * penetration;
        let r = point - f.p;
        let v = f.v + angular(r, f.spin);
        let vn = v.dot(n);
        if vn <= 0.0 {
            continue;
        }
        impact = impact.max(vn);
        let mass = f.mass();
        let inertia = mass * f.radius().powi(2) * 0.5;
        let impulse = vn * 1.06 / (1.0 / mass + cross(r, n).powi(2) / inertia);
        f.v = f.v - n * (impulse / mass);
        f.spin = (f.spin - cross(r, n * impulse) / inertia).clamp(-25.0, 25.0);
        if n.y > 0.0 {
            f.v.x *= 0.99;
            f.spin *= 0.995;
        }
    }
    impact
}
