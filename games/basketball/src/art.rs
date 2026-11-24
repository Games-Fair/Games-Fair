// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
use crate::model::{Game, HEIGHT, RADIUS, V3, Vec2, WIDTH, project};
use day_core::Bitmap;
use day_pieces::prelude::*;
use game_resources::res;
use gamekit::chrome;
pub const COLORS: [Color; 4] = [
    Color::hex(0xFF5F57),
    Color::hex(0x00C2F0),
    Color::hex(0x76CA3E),
    Color::hex(0xFFBF2F),
];
fn circle(d: &mut Draw, x: f64, y: f64, r: f64, c: Color) {
    d.fill(Shape::Ellipse(Rect::new(x - r, y - r, r * 2.0, r * 2.0)), c);
}
fn line(d: &mut Draw, a: (f64, f64), b: (f64, f64), c: Color, w: f64) {
    d.stroke(
        PathBuilder::new()
            .move_to(Point::new(a.0, a.1))
            .line_to(Point::new(b.0, b.1))
            .build(),
        c,
        w,
    );
}
fn rounded(d: &mut Draw, r: Rect, rad: f64, c: Color) {
    d.fill(Shape::RoundedRect(r, rad), c);
}
pub fn layout(size: Size) -> (f64, f64, f64) {
    let s = (size.height / HEIGHT).max(0.0);
    (
        (size.width - WIDTH * s) * 0.5,
        (size.height - HEIGHT * s) * 0.5,
        s,
    )
}
pub fn draw(d: &mut Draw, size: Size, g: &Game, sheet: Option<&Bitmap>, held: Option<Vec2>) {
    let (ox, oy, s) = layout(size);
    if s <= 0.0 {
        return;
    }
    d.clipped(
        Shape::Rect(Rect::new(0.0, 0.0, size.width, size.height)),
        |d| {
            d.transformed(
                Affine::translate(-WIDTH * 0.5, -HEIGHT * 0.5)
                    .then(Affine::rotate(Angle::rad(g.rotation)))
                    .then(Affine::translate(WIDTH * 0.5, HEIGHT * 0.5))
                    .then(Affine::scale(s, s))
                    .then(Affine::translate(ox, oy)),
                |d| court(d, g, sheet, held, size.width / s),
            );
        },
    );
}
fn court(d: &mut Draw, g: &Game, sheet: Option<&Bitmap>, held: Option<Vec2>, viewport_width: f64) {
    let left = (WIDTH - viewport_width) * 0.5;
    let right = left + viewport_width;
    let color = COLORS[g.turn];
    let pale = [
        Color::hex(0xFF8788),
        Color::hex(0x5ADFFF),
        Color::hex(0xA7DF82),
        Color::hex(0xFFDA88),
    ][g.turn];
    d.fill(
        Shape::Rect(Rect::new(
            left - 800.0,
            -800.0,
            viewport_width + 1600.0,
            HEIGHT + 1600.0,
        )),
        Color::hex(0x9CE9FB),
    );
    for (offset, y) in [(0.0, 165.0), (210.0, 285.0)] {
        let x = (g.elapsed * 3.5 + offset).rem_euclid(WIDTH + 100.0) - 50.0;
        rounded(
            d,
            Rect::new(x - 35.0, y, 86.0, 40.0),
            20.0,
            Color::hex(0xB2F2FC),
        );
        circle(d, x + 6.0, y, 24.0, Color::hex(0xB2F2FC));
    }
    d.fill(
        Shape::Rect(Rect::new(
            left - 800.0,
            406.0,
            viewport_width + 1600.0,
            70.0,
        )),
        Color::hex(0x76CA3E),
    );
    d.fill(
        Shape::Rect(Rect::new(
            left - 800.0,
            476.0,
            viewport_width + 1600.0,
            1104.0,
        )),
        Color::hex(0x34393F),
    );
    let fence = Color::hex(0x7C858F);
    d.clipped(
        Shape::Rect(Rect::new(left, 236.0, viewport_width, 231.0)),
        |d| {
            for i in (left / 37.0) as i32 - 8..=(right / 37.0) as i32 + 8 {
                let x = i as f64 * 37.0;
                line(d, (x, 235.0), (x + 240.0, 475.0), fence, 4.5);
                line(d, (x, 235.0), (x - 240.0, 475.0), fence, 4.5);
            }
            for y in (254..468).step_by(39) {
                for x in ((left / 37.0).floor() as i32 * 37 + 15..right as i32).step_by(37) {
                    circle(d, x as f64, y as f64, 3.4, fence);
                }
            }
        },
    );
    line(d, (left, 254.0), (right, 254.0), fence, 9.0);
    line(d, (left, 467.0), (right, 467.0), fence, 5.0);
    d.fill(
        Shape::Rect(Rect::new(174.0, 260.0, 12.0, 259.0)),
        Color::hex(0x484D53),
    );
    d.fill(
        Shape::Rect(Rect::new(176.0, 260.0, 3.0, 259.0)),
        Color::hex(0x60666A),
    );
    line(d, (left, 519.0), (right, 519.0), color, 4.0);
    line(d, (63.0, 519.0), (-95.0, 710.0), color, 9.0);
    line(d, (297.0, 519.0), (455.0, 710.0), color, 9.0);
    line(d, (29.0, 562.0), (331.0, 562.0), pale, 3.5);
    // Paint the backboard, inset panel and target rectangle behind the rim.
    rounded(d, Rect::new(102.0, 162.0, 156.0, 109.0), 10.0, Color::BLACK);
    rounded(d, Rect::new(104.5, 164.5, 151.0, 104.0), 8.0, pale);
    rounded(d, Rect::new(109.0, 169.0, 142.0, 95.0), 4.0, color);
    d.stroke(
        Shape::RoundedRect(Rect::new(141.0, 201.5, 78.0, 53.0), 6.0),
        pale,
        4.0,
    );
    rounded(d, Rect::new(156.5, 141.0, 47.0, 26.0), 7.0, Color::BLACK);
    rounded(
        d,
        Rect::new(159.0, 143.5, 42.0, 21.0),
        4.5,
        Color::hex(0x252628),
    );
    d.text(
        &res::str::party_count(g.remaining.ceil()).format(),
        Point::new(180.0, 154.5),
        TextStyle {
            size: 15.0,
            color: color.into(),
            anchor: TextAnchor::CENTERED,
            font: chrome::canvas_font(FontWeight::Bold),
        },
    );
    let mut balls = g.balls.iter().collect::<Vec<_>>();
    balls.sort_by(|a, b| b.p.z.total_cmp(&a.p.z));
    for ball in &balls {
        let (p, scale) = project(V3::new(ball.p.x, -12.25, ball.p.z));
        let r = RADIUS * scale;
        d.fill(
            Shape::Ellipse(Rect::new(p.x - r, p.y - r * 0.27, r * 2.0, r * 0.54)),
            Color::rgba(0.0, 0.0, 0.0, 0.18 * (1.0 - ball.fade)),
        );
    }
    // Back half of the hoop and net, then balls, then the front cords for a convincing swish.
    net(d, g, false);
    for ball in &balls {
        if ball.p.z >= 22.5 {
            draw_ball(d, sheet, ball.p, ball.spin, 1.0 - ball.fade);
        }
    }
    rim(d, g);
    net(d, g, true);
    for ball in &balls {
        if ball.p.z < 22.5 {
            draw_ball(d, sheet, ball.p, ball.spin, 1.0 - ball.fade);
        }
    }
    if g.remaining > 0.0 && !g.finished {
        let p = g.ready_position();
        let (mut point, scale) = project(p);
        let r = RADIUS * scale;
        let pop = (g.ready_age / 0.5).min(1.0);
        let bounce = (pop * std::f64::consts::PI * 2.0).sin().abs() * (1.0 - pop) * 25.0;
        d.fill(
            Shape::Ellipse(Rect::new(
                point.x - r,
                point.y + r * 0.72,
                r * 2.0,
                r * 0.55,
            )),
            Color::hex(0x272C31),
        );
        if let Some(held) = held {
            point = held;
        } else {
            point.y -= bounce;
        }
        ball_at(d, sheet, point, r, 0.0, 1.0);
    }
    if g.net_age < 0.8 {
        let t = g.net_age / 0.8;
        for i in 0..12 {
            let a = i as f64 * std::f64::consts::TAU / 12.0;
            circle(
                d,
                180.0 + a.cos() * t * 60.0,
                250.0 + a.sin() * t * 45.0 + 30.0 * t * t,
                (1.0 - t) * 2.2,
                Color::hex(0xFFE874),
            );
        }
        d.text(
            &res::str::basketball_point().format(),
            Point::new(180.0, 190.0 - t * 35.0),
            TextStyle {
                size: 24.0,
                color: Color::WHITE.into(),
                anchor: TextAnchor::CENTERED,
                font: chrome::canvas_font(FontWeight::Bold),
            },
        );
    }
    // The compact edge scores leave the whole court available for play.
    for i in 0..g.count {
        let on_right = i % 2 == 0;
        let y = 40.0 + (i / 2) as f64 * 58.0;
        let x = if on_right { right - 77.0 } else { left - 8.0 };
        rounded(d, Rect::new(x, y, 85.0, 52.0), 26.0, Color::BLACK);
        rounded(
            d,
            Rect::new(x + 6.0, y + 6.0, 73.0, 40.0),
            20.0,
            Color::hex(0x2F333E),
        );
        if i == g.turn {
            d.stroke(
                Shape::RoundedRect(Rect::new(x + 6.0, y + 6.0, 73.0, 40.0), 20.0),
                COLORS[i],
                2.0,
            );
        }
        d.text(
            &res::str::party_player((i + 1) as f64).format(),
            Point::new(x + 42.5, y + 14.0),
            TextStyle {
                size: 9.0,
                color: COLORS[i].into(),
                anchor: TextAnchor::CENTERED,
                font: chrome::canvas_font(FontWeight::Bold),
            },
        );
        d.text(
            &res::str::party_count(g.scores[i] as f64).format(),
            Point::new(x + 42.5, y + 31.0),
            TextStyle {
                size: 27.0,
                color: COLORS[i].into(),
                anchor: TextAnchor::CENTERED,
                font: chrome::canvas_font(FontWeight::Bold),
            },
        );
    }
    if !g.running && g.transition <= 0.0 && !g.finished {
        d.text(
            &res::str::basketball_swipe().format(),
            Point::new(180.0, 750.0),
            TextStyle {
                size: 12.0,
                color: Color::hex(0xDFE8F0).into(),
                anchor: TextAnchor::CENTERED,
                font: chrome::canvas_font(FontWeight::Semibold),
            },
        );
    }
}
fn rim(d: &mut Draw, g: &Game) {
    let pulse = if g.rim_age < 0.25 {
        (g.rim_age * 60.0).sin() * (1.0 - g.rim_age / 0.25) * 1.4
    } else {
        0.0
    };
    let scale = g.difficulty.hoop_scale();
    let r = Rect::new(
        180.0 - 30.0 * scale,
        245.0 - 8.0 * scale + pulse,
        60.0 * scale,
        16.0 * scale,
    );
    d.stroke(Shape::Ellipse(r), Color::BLACK, 5.0);
    d.stroke(Shape::Ellipse(r), Color::hex(0xFFD545), 2.7);
}
fn net(d: &mut Draw, g: &Game, front: bool) {
    let sway = if g.net_age < 0.7 {
        (g.net_age * 24.0).sin() * 4.0 * (1.0 - g.net_age / 0.7)
    } else {
        0.0
    };
    let stretch = if g.net_age < 0.6 {
        (g.net_age / 0.6 * std::f64::consts::PI).sin() * 8.0
    } else {
        0.0
    };
    let color = if front {
        Color::WHITE
    } else {
        Color::hex(0xD6E9EC)
    };
    let start = if front { 0.0 } else { std::f64::consts::PI };
    for ring in 0..=7 {
        let t = ring as f64 / 7.0;
        let radius = (27.0 - t * 12.0) * g.difficulty.hoop_scale();
        let y = 245.0 + t * (55.0 * g.difficulty.hoop_scale() + stretch);
        let mut p = PathBuilder::new();
        for j in 0..=24 {
            let a = start + j as f64 / 24.0 * std::f64::consts::PI;
            let q = Point::new(
                180.0 + sway * t + a.cos() * radius,
                y + a.sin() * radius * 0.23,
            );
            p = if j == 0 { p.move_to(q) } else { p.line_to(q) };
        }
        d.stroke(p.build(), color, 0.95);
    }
    for i in 0..=6 {
        let a = start + i as f64 / 6.0 * std::f64::consts::PI;
        let mut p = PathBuilder::new();
        for j in 0..=12 {
            let t = j as f64 / 12.0;
            let radius = (27.0 - t * 12.0) * g.difficulty.hoop_scale();
            let q = Point::new(
                180.0 + sway * t + a.cos() * radius,
                245.0 + t * (55.0 * g.difficulty.hoop_scale() + stretch) + a.sin() * radius * 0.23,
            );
            p = if j == 0 { p.move_to(q) } else { p.line_to(q) };
        }
        d.stroke(p.build(), color, 1.1);
    }
}
fn draw_ball(d: &mut Draw, sheet: Option<&Bitmap>, p: V3, spin: f64, alpha: f64) {
    let (point, scale) = project(p);
    ball_at(d, sheet, point, RADIUS * scale, spin, alpha);
}
fn ball_at(d: &mut Draw, sheet: Option<&Bitmap>, p: Vec2, r: f64, spin: f64, alpha: f64) {
    if let Some(sheet) = sheet {
        let frame = ((spin / std::f64::consts::TAU * 24.0) as usize) % 24;
        d.image_region_with_opacity(
            sheet,
            Rect::new(
                (frame % 8) as f64 * 192.0,
                (frame / 8) as f64 * 192.0,
                192.0,
                192.0,
            ),
            Rect::new(p.x - r, p.y - r, r * 2.0, r * 2.0),
            alpha,
        );
    }
}
