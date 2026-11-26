// SPDX-License-Identifier: GPL-2.0-or-later
mod model;
use day_pieces::prelude::*;
use day_spec::TouchPhase;
use game_resources::res;
use gamekit::{
    board_game::{self as host, BoardGame, Feedback, circle, line, text},
    multiplayer::Vec2,
};
use model::Game;
pub const SURFACE: Color = Color::hex(0xF89875);
const COLORS: [Color; 4] = [
    Color::hex(0xFF5F57),
    Color::hex(0x00C2F0),
    Color::hex(0x76CA3E),
    Color::hex(0xFFBF2F),
];
fn scale(s: Size) -> f64 {
    (s.width / 21.5).min(s.height / 47.5)
}
impl BoardGame for Game {
    fn new(p: usize, s: u64) -> Self {
        let mut g = Self::new(p, s);
        g.formations = day_spec::resource(res::assets::buttonsoccer::formations_json)
            .and_then(|r| serde_json::from_slice(r.as_slice()).ok())
            .unwrap_or_default();
        g.formation = (s as usize) % g.formations.len().max(1);
        g.reset();
        g.reset_progress = 1.0;
        g
    }
    fn pause_alignment() -> Alignment {
        Alignment::Bottom
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
    fn haptic(event: Feedback, default: gamekit::chrome::Pattern) -> gamekit::chrome::Pattern {
        use gamekit::chrome::cues;
        match event {
            Feedback::Flick(v) => {
                if v >= 0.65 {
                    cues::MEDIUM_BEAT
                } else {
                    cues::LIGHT_BEAT
                }
            }
            Feedback::Impact(v) => gamekit::multiplayer::impact_pattern(v * 340.),
            Feedback::Wall(v) | Feedback::Disc(v) if v >= 0.35 => cues::LIGHT_BEAT,
            Feedback::Score => gamekit::chrome::CELEBRATE,
            Feedback::OwnGoal => gamekit::chrome::LETDOWN,
            Feedback::Turn => cues::TICK_BEAT,
            _ => default,
        }
    }
    fn cancel(&mut self) {
        self.selected = None;
        self.drag = None;
    }
    fn touch(&mut self, phase: TouchPhase, p: Point, s: Size) -> Vec<Feedback> {
        if self.humans == 1 && self.turn == 1 || self.finished {
            return vec![];
        }
        let k = scale(s);
        let p = Vec2::new((p.x - 0.5) * s.width / k, (0.5 - p.y) * s.height / k);
        match phase {
            TouchPhase::Began if !self.moving && self.delay == 0.0 && self.goal_age == 0.0 => {
                self.selected = (0..10)
                    .filter(|i| i / 5 == Game::side(self.turn))
                    .find(|&i| self.bodies[i].p.distance(p) < 1.5);
                self.drag = Some(p);
            }
            TouchPhase::Moved => self.drag = Some(p),
            TouchPhase::Ended => {
                if let Some(i) = self.selected.take() {
                    let pull = self.bodies[i].p - p;
                    self.drag = None;
                    if self.shoot(i, pull) {
                        return vec![self.flick_feedback(i)];
                    }
                }
            }
            TouchPhase::Cancelled => self.cancel(),
            _ => {}
        }
        vec![]
    }
    fn draw(&self, d: &mut Draw, s: Size, sprites: &[Option<day_core::Bitmap>]) {
        let bg = Color::hex([0xF89875, 0xA5D0DF, 0xB9DD8F, 0xFFE096][self.turn]);
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), bg);
        let k = scale(s);
        let p = |x, y| Point::new(s.width * 0.5 + x * k, s.height * 0.5 - y * k);
        let rect =
            |x, y, w, h| Rect::new(s.width * 0.5 + x * k, s.height * 0.5 - y * k, w * k, h * k);
        d.fill(Shape::Rect(rect(-10.22, 16.84, 20.44, 33.68)), Color::WHITE);
        for i in 0..10 {
            d.fill(
                Shape::Rect(rect(-9.22, 15.84 - i as f64 * 3.168, 18.44, 3.168)),
                Color::hex(if i % 2 == 0 { 0x50AF3F } else { 0x4CA23D }),
            );
        }
        line(d, p(-9.22, 0.0), p(9.22, 0.0), 0.55 * k, Color::WHITE);
        d.stroke(
            Shape::Ellipse(rect(-1.8, 1.8, 3.6, 3.6)),
            Color::WHITE,
            0.50 * k,
        );
        for side in 0..2 {
            let sign = if side == 0 { -1.0 } else { 1.0 };
            let player = if side == Game::side(self.turn) {
                self.turn
            } else {
                (0..self.count)
                    .find(|&i| Game::side(i) == side)
                    .expect("validated game invariant")
            };
            let color = COLORS[player];
            let y = if side == 1 { 18.6 } else { -15.84 };
            let goal = rect(-3.45, y, 6.9, 2.76);
            d.fill(Shape::Rect(goal), bg);
            d.clipped(Shape::Rect(goal), |d| {
                for j in -5..20 {
                    let x = -3.45 + j as f64 * 0.85;
                    line(
                        d,
                        p(x, sign * 15.84),
                        p(x + 2.76, sign * 18.6),
                        0.28 * k,
                        Color::rgba(1.0, 1.0, 1.0, 0.45),
                    );
                    line(
                        d,
                        p(x, sign * 15.84),
                        p(x - 2.76, sign * 18.6),
                        0.28 * k,
                        Color::rgba(1.0, 1.0, 1.0, 0.45),
                    );
                }
            });
            d.stroke(Shape::Rect(goal), color, 0.38 * k);
            d.stroke(
                Shape::Rect(rect(
                    -4.94,
                    if side == 1 { 16.15 } else { -12.98 },
                    9.88,
                    3.17,
                )),
                Color::WHITE,
                0.55 * k,
            );
        }
        if let Some(i) = self.selected
            && let Some(at) = self.drag
        {
            let b = &self.bodies[i];
            let delta = b.p - at;
            let dir = delta.normalized();
            let power = (delta.length() - 1.0).clamp(0.0, 3.0);
            let tip = b.p + dir * (power * 1.5 + 1.2);
            let normal = Vec2::new(-dir.y, dir.x) * 0.55;
            d.fill(
                PathBuilder::new()
                    .move_to(p(tip.x, tip.y))
                    .line_to(p(b.p.x + normal.x, b.p.y + normal.y))
                    .line_to(p(b.p.x - normal.x, b.p.y - normal.y))
                    .close()
                    .build(),
                Color::rgba(1.0, 1.0, 1.0, 0.65),
            );
        }
        for &(at, age) in &self.trail {
            circle(
                d,
                p(at.x, at.y),
                (1.0 - age / 0.22) * 0.7 * k,
                Color::rgba(1.0, 1.0, 1.0, (1.0 - age / 0.22) * 0.2),
            );
        }
        for (i, b) in self.bodies.iter().enumerate() {
            let radius = model::radius(i) * k;
            let t = self.reset_progress;
            let t = 1.0 - (1.0 - t).powi(3);
            let at = self
                .reset_from
                .get(i)
                .copied()
                .map(|old| old + (b.p - old) * t)
                .unwrap_or(b.p);
            let point = p(at.x, at.y);
            circle(
                d,
                Point::new(point.x + 0.25 * k, point.y + 0.25 * k),
                radius,
                Color::rgba(0.0, 0.0, 0.0, 0.26),
            );
            if i == 10 {
                circle(d, point, radius * 1.7, Color::rgba(0.8, 1.0, 0.1, 0.2));
                circle(d, point, radius, Color::BLACK);
                circle(d, point, radius * 0.84, Color::WHITE);
                d.clipped(
                    Shape::Ellipse(Rect::new(
                        point.x - radius * 0.84,
                        point.y - radius * 0.84,
                        radius * 1.68,
                        radius * 1.68,
                    )),
                    |d| {
                        for j in 0..6 {
                            let a = b.spin * 0.4 + j as f64 * std::f64::consts::TAU / 5.0;
                            let center = if j == 5 {
                                point
                            } else {
                                Point::new(
                                    point.x + a.cos() * radius * 0.8,
                                    point.y + a.sin() * radius * 0.8,
                                )
                            };
                            let mut path = PathBuilder::new();
                            for v in 0..5 {
                                let a = a + v as f64 * std::f64::consts::TAU / 5.0;
                                let q = Point::new(
                                    center.x + a.cos() * radius * 0.36,
                                    center.y + a.sin() * radius * 0.36,
                                );
                                path = if v == 0 {
                                    path.move_to(q)
                                } else {
                                    path.line_to(q)
                                };
                            }
                            d.fill(path.close().build(), Color::BLACK);
                        }
                    },
                );
            } else {
                let side = i / 5;
                let player = if side == Game::side(self.turn) {
                    self.turn
                } else {
                    (0..self.count)
                        .find(|&i| Game::side(i) == side)
                        .expect("validated game invariant")
                };
                circle(d, point, radius, Color::BLACK);
                circle(d, point, radius * 0.75, COLORS[player]);
                if let Some(star) = sprites.first().and_then(|b| b.as_ref()) {
                    d.transformed(
                        Affine::translate(-point.x, -point.y)
                            .then(Affine::rotate(Angle::rad(b.spin * 0.12)))
                            .then(Affine::translate(point.x, point.y)),
                        |d| {
                            d.image(
                                star,
                                Rect::new(
                                    point.x - 0.53 * k,
                                    point.y - 0.53 * k,
                                    1.06 * k,
                                    1.06 * k,
                                ),
                            )
                        },
                    );
                }
                if side == Game::side(self.turn)
                    && !self.moving
                    && let Some(ring) = sprites.get(1).and_then(|b| b.as_ref())
                {
                    d.transformed(
                        Affine::translate(-point.x, -point.y)
                            .then(Affine::rotate(Angle::rad(self.elapsed * 0.4)))
                            .then(Affine::translate(point.x, point.y)),
                        |d| {
                            d.image(
                                ring,
                                Rect::new(
                                    point.x - 1.38 * k,
                                    point.y - 1.38 * k,
                                    2.76 * k,
                                    2.76 * k,
                                ),
                            )
                        },
                    );
                }
            }
        }
        host::scores(d, s, self);
        text(
            d,
            &self.status().format(),
            Point::new(s.width * 0.5, 28.0),
            14.0,
            Color::hex(0x273A34),
        );
        if self.goal_age > 0.0 {
            let scorer = self.last_scorer;
            d.fill(
                Shape::RoundedRect(
                    Rect::new(
                        s.width * 0.15,
                        s.height * 0.43,
                        s.width * 0.7,
                        s.height * 0.14,
                    ),
                    22.0,
                ),
                COLORS[scorer],
            );
            text(
                d,
                &res::str::party_player((scorer + 1) as f64).format(),
                Point::new(s.width * 0.5, s.height * 0.47),
                20.0,
                Color::WHITE,
            );
            text(
                d,
                &res::str::party_count(self.scores[scorer] as f64).format(),
                Point::new(s.width * 0.5, s.height * 0.53),
                44.0,
                Color::WHITE,
            );
            gamekit::art::confetti(d, s, 2.0 - self.goal_age);
        }
    }
}

pub const SOUNDS: &[gamekit::chrome::Sfx] = &[
    res::assets::sounds::buttonsoccer::kick_wav,
    res::assets::sounds::buttonsoccer::goal_wav,
    res::assets::sounds::shared::tap_wav,
    res::assets::sounds::buttonsoccer::wall_wav,
    res::assets::sounds::buttonsoccer::disc_wav,
    res::assets::sounds::buttonsoccer::own_goal_wav,
    res::assets::sounds::buttonsoccer::flick_wav,
];
pub fn buttonsoccer_page(p: usize) -> AnyPiece {
    host::page::<Game>(
        p,
        host::Config {
            key: "buttonsoccer.v2",
            settings_key: "buttonsoccer.settings",
            title: res::str::nav_buttonsoccer,
            rules: res::str::buttonsoccer_rules,
            board_id: id!(buttonsoccer_board),
            pause_id: id!(buttonsoccer_pause),
            help_id: id!(buttonsoccer_help_done),
            background: SURFACE,
            sprites: vec![
                res::assets::buttonsoccer::star_png,
                res::assets::buttonsoccer::selection_png,
            ],
            sounds: SOUNDS,
        },
    )
}
pub fn buttonsoccer_preview() -> AnyPiece {
    let g = Game::new(2, 1);
    canvas(move |d, s| g.draw(d, s, &[]))
        .grow(Dimension::Both)
        .any()
}
