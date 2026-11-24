// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Original vector artwork, rendered at device resolution without external images.
use day_pieces::prelude::*;
pub const COLORS: [Color; 4] = [
    Color::hex(0xFF615D),
    Color::hex(0x05BBDC),
    Color::hex(0x80CB3D),
    Color::hex(0xFFC332),
];
pub fn confetti(d: &mut Draw, size: Size, time: f64) {
    for i in 0..60 {
        let phase = (time * 0.48 + i as f64 * 0.137) % 1.;
        let x = (i as f64 * 0.61803398875 % 1.) * size.width + (time * 2. + i as f64).sin() * 12.;
        let y = phase * size.height;
        d.transformed(
            Affine::rotate(Angle::deg(time * 100. + i as f64 * 37.)).then(Affine::translate(x, y)),
            |d| {
                d.fill(
                    Shape::RoundedRect(Rect::new(-2., -4., 4., 8.), 1.),
                    COLORS[i % 4],
                );
            },
        );
    }
}

/// Draw a shared-device thumb control in board coordinates. The inner ring follows
/// the finger within the fixed outer rim.
pub fn joystick(d: &mut Draw, center: Point, scale: f64, color: Color, offset: Point) {
    let disc = |d: &mut Draw, center: Point, radius: f64, color: Color| {
        let r = radius * scale;
        d.fill(
            Shape::Ellipse(Rect::new(center.x - r, center.y - r, r * 2.0, r * 2.0)),
            color,
        );
    };
    let length = offset.x.hypot(offset.y).max(1.0);
    let follow = length.min(13.0) / length * scale;
    let thumb = Point::new(center.x + offset.x * follow, center.y + offset.y * follow);
    disc(d, center, 44.0, Color::BLACK);
    disc(d, center, 39.0, color);
    disc(
        d,
        thumb,
        32.0,
        Color::rgb(color.r * 0.70, color.g * 0.70, color.b * 0.70),
    );
    disc(d, thumb, 27.0, color);
    disc(d, thumb, 21.0, Color::BLACK);
}
