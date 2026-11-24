// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use day_pieces::prelude::*;
const INK: Color = Color::hex(0x252334);
fn oval(d: &mut Draw, x: f64, y: f64, w: f64, h: f64, color: Color) {
    d.fill(Shape::Ellipse(Rect::new(x, y, w, h)), color);
}
fn polygon(d: &mut Draw, points: &[(f64, f64)], color: Color) {
    let mut p = PathBuilder::new();
    p = p.move_to(Point::new(points[0].0, points[0].1));
    for &(x, y) in &points[1..] {
        p = p.line_to(Point::new(x, y));
    }
    d.fill(p.close().build(), color);
}
pub fn paint_pickup(d: &mut Draw, center: Point, scale: f64, pulse: f64, burst: bool) {
    d.transformed(
        Affine::scale(scale, scale).then(Affine::translate(center.x, center.y)),
        |d| {
            let r = 20. + pulse * 1.5;
            oval(
                d,
                -r + 2.,
                -r + 3.,
                r * 2.,
                r * 2.,
                Color::rgba(0., 0., 0., 0.12),
            );
            oval(d, -r, -r, r * 2., r * 2., Color::WHITE);
            if burst {
                let points: Vec<_> = (0..16)
                    .map(|i| {
                        let angle = i as f64 * std::f64::consts::TAU / 16.;
                        let r = if i % 2 == 0 { 15. } else { 8. };
                        (angle.cos() * r, angle.sin() * r)
                    })
                    .collect();
                polygon(d, &points, INK);
            } else {
                polygon(
                    d,
                    &[
                        (-1., -15.),
                        (12., -15.),
                        (2., -3.),
                        (12., -3.),
                        (-8., 16.),
                        (-3., 3.),
                        (-13., 3.),
                    ],
                    INK,
                );
            }
        },
    );
}
