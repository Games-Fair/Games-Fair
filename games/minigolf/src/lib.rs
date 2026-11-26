// SPDX-License-Identifier: GPL-2.0-or-later
mod model;
use day_pieces::prelude::*;
use day_spec::TouchPhase;
use game_resources::res;
use gamekit::{
    board_game::{self as host, BoardGame, Feedback, circle, line, text},
    multiplayer::Vec2,
};
use model::{Course, Game};
pub const SURFACE: Color = Color::hex(0x279936);
fn courses() -> Vec<Course> {
    day_spec::resource(res::assets::minigolf::courses_json)
        .and_then(|b| serde_json::from_slice(b.as_slice()).ok())
        .unwrap_or_else(|| vec![Course::default(); 9])
}
fn scale(s: Size) -> f64 {
    (s.width / 9.75).min(s.height / 21.2)
}
impl BoardGame for Game {
    fn new(p: usize, s: u64) -> Self {
        Self::new(p, s, courses())
    }
    fn valid(&self) -> bool {
        self.valid()
    }
    fn players(&self) -> usize {
        self.count
    }
    fn humans(&self) -> usize {
        self.humans
    }
    fn turn(&self) -> usize {
        self.turn
    }
    fn score(&self, p: usize) -> u32 {
        self.scores[p]
    }
    fn finished(&self) -> bool {
        self.finished
    }
    fn tick(&mut self, dt: f64) -> Vec<Feedback> {
        self.tick(dt)
    }
    fn cancel(&mut self) {
        self.drag = None;
    }
    fn touch(&mut self, phase: TouchPhase, p: Point, s: Size) -> Vec<Feedback> {
        if self.humans == 1 && self.turn == 1 {
            return vec![];
        }
        let k = scale(s);
        let p = Vec2::new((p.x - 0.5) * s.width / k, (0.5 - p.y) * s.height / k);
        match phase {
            TouchPhase::Began
                if !self.moving && self.delay == 0.0 && self.ball.distance(p) < 1.0 =>
            {
                self.drag = Some(p)
            }
            TouchPhase::Moved if self.drag.is_some() => self.drag = Some(p),
            TouchPhase::Ended if self.drag.is_some() => {
                self.drag = None;
                if self.shoot(self.ball - p) {
                    return vec![Feedback::Tap];
                }
            }
            TouchPhase::Cancelled => self.drag = None,
            _ => {}
        }
        vec![]
    }
    fn draw(&self, d: &mut Draw, s: Size, _: &[Option<day_core::Bitmap>]) {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), SURFACE);
        let k = scale(s);
        let p = |x, y| Point::new(s.width * 0.5 + x * k, s.height * 0.5 - y * k);
        let rect =
            |x, y, w, h| Rect::new(s.width * 0.5 + x * k, s.height * 0.5 - y * k, w * k, h * k);
        d.fill(
            Shape::Rect(rect(-3.3, 8.45, 7.95, 16.9)),
            Color::rgba(0.0, 0.0, 0.0, 0.22),
        );
        d.fill(Shape::Rect(rect(-4.0, 8.45, 8.0, 16.9)), Color::BLACK);
        d.fill(Shape::Rect(rect(-3.55, 8.0, 7.1, 16.0)), Color::WHITE);
        d.fill(Shape::Rect(rect(-3.05, 7.5, 6.1, 15.0)), Color::BLACK);
        d.fill(
            Shape::Rect(rect(-2.58, 7.03, 5.16, 14.06)),
            Color::hex([0xF89975, 0xA5D0DF, 0xB8DF8A, 0xFFE499][self.turn]),
        );
        d.fill(
            Shape::Rect(rect(-2.58, 7.03, 0.45, 14.06)),
            Color::rgba(0.0, 0.0, 0.0, 0.2),
        );
        d.clipped(Shape::Rect(rect(-2.7, 7.1, 5.4, 14.2)), |d| {
            for poly in self.polygons() {
                let mut path = PathBuilder::new();
                for (i, v) in poly.iter().enumerate() {
                    let q = p(v.x, v.y);
                    path = if i == 0 {
                        path.move_to(q)
                    } else {
                        path.line_to(q)
                    };
                }
                let shape = path.close().build();
                d.fill(shape.clone(), Color::hex(0xFBF7F4));
                d.stroke(shape, Color::BLACK, 0.18 * k);
            }
            for c in &self.course().circles {
                circle(d, p(c.p[0], c.p[1]), c.r * k, Color::BLACK);
                circle(d, p(c.p[0], c.p[1]), c.r * k * 0.65, Color::hex(0xFBF7F4));
            }
            if !self.course().wind.is_empty() {
                for i in 0..16 {
                    let x = (self.elapsed * 5.0 + i as f64 * 0.7).rem_euclid(6.0) - 3.0;
                    let y = (i % 5) as f64 * 0.8 - 1.6;
                    line(
                        d,
                        p(x, y),
                        p(x - 0.35, y + 0.1),
                        0.04 * k,
                        Color::rgba(1.0, 1.0, 1.0, 0.5),
                    );
                }
            }
            let hole = self.hole();
            circle(d, p(hole.x, hole.y), 0.61 * k, Color::BLACK);
            for (i, t) in self.trail.iter().enumerate() {
                circle(
                    d,
                    p(t.x, t.y),
                    0.26 * k * i as f64 / 16.0,
                    Color::rgba(1.0, 1.0, 1.0, 0.15 * i as f64 / 16.0),
                );
            }
            if let Some(at) = self.drag {
                let pull = self.ball - at;
                let aim = self.ball + pull.normalized() * pull.length().min(5.0);
                let perpendicular = Vec2::new(-pull.y, pull.x).normalized() * 0.27;
                d.fill(
                    PathBuilder::new()
                        .move_to(p(aim.x, aim.y))
                        .line_to(p(
                            self.ball.x + perpendicular.x,
                            self.ball.y + perpendicular.y,
                        ))
                        .line_to(p(
                            self.ball.x - perpendicular.x,
                            self.ball.y - perpendicular.y,
                        ))
                        .close()
                        .build(),
                    Color::rgba(1.0, 1.0, 1.0, 0.55),
                );
            }
            let r = if self.sink > 0.0 {
                0.7 * (1.0 - self.sink / 0.6).max(0.0)
            } else {
                0.7
            };
            circle(d, p(self.ball.x, self.ball.y), r * k, Color::BLACK);
            circle(d, p(self.ball.x, self.ball.y), r * k * 0.54, Color::WHITE);
        });
        host::scores(d, s, self);
        text(
            d,
            &self.status().format(),
            Point::new(s.width * 0.5, 92.0),
            15.0,
            Color::WHITE,
        );
    }
}
pub const SOUNDS: &[gamekit::chrome::Sfx] = &[
    res::assets::sounds::buttonsoccer::wall_wav,
    res::assets::sounds::minigolf::hole_wav,
    res::assets::sounds::shared::tap_wav,
];
pub fn minigolf_page(p: usize) -> AnyPiece {
    host::page::<Game>(
        p,
        host::Config {
            key: "minigolf.v2",
            settings_key: "minigolf.settings",
            title: res::str::nav_minigolf,
            rules: res::str::minigolf_rules,
            board_id: id!(minigolf_board),
            pause_id: id!(minigolf_pause),
            help_id: id!(minigolf_help_done),
            background: SURFACE,
            sprites: vec![],
            sounds: SOUNDS,
        },
    )
}
pub fn minigolf_preview() -> AnyPiece {
    let mut g = Game::new(2, 1, courses());
    g.order[0] = 2;
    canvas(move |d, s| {
        d.transformed(
            Affine::scale(1.7, 1.7).then(Affine::translate(-s.width * 0.35, -s.height * 0.35)),
            |d| g.draw(d, s, &[]),
        );
    })
    .grow(Dimension::Both)
    .any()
}
