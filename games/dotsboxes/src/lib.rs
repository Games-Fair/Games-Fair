// SPDX-License-Identifier: GPL-2.0-or-later
mod model;
use day_pieces::prelude::*;
use day_spec::TouchPhase;
use game_resources::res;
use gamekit::board_game::{self as host, BoardGame, Feedback, circle, line, text};
use model::Game;
pub const SURFACE: Color = Color::hex(0xFF5F57);
const COLORS: [Color; 4] = [
    Color::hex(0xFF5F57),
    Color::hex(0x00C2F0),
    Color::hex(0x76CA3E),
    Color::hex(0xFFBF2F),
];
fn grid(s: Size) -> (f64, f64, f64) {
    let side = (s.width * 0.866).min((s.height - 200.0).max(80.0));
    ((s.width - side) * 0.5, (s.height - side) * 0.5, side / 5.0)
}
fn endpoints(e: usize) -> ((usize, usize), (usize, usize)) {
    if e < 30 {
        ((e % 5, e / 5), (e % 5 + 1, e / 5))
    } else {
        let e = e - 30;
        ((e % 6, e / 6), (e % 6, e / 6 + 1))
    }
}
impl BoardGame for Game {
    fn new(p: usize, s: u64) -> Self {
        Self::new(p, s)
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
        self.delay = (self.delay - dt).max(0.0);
        self.pulse = (self.pulse + dt).min(1.0);
        if self.humans == 1 && self.turn == 1 && !self.finished && self.delay == 0.0 {
            let e = self.ai();
            let n = self.play(e).unwrap_or(0);
            return vec![if n > 0 {
                Feedback::Score
            } else {
                Feedback::Tap
            }];
        }
        vec![]
    }
    fn touch(&mut self, phase: TouchPhase, p: Point, s: Size) -> Vec<Feedback> {
        if phase != TouchPhase::Ended || self.humans == 1 && self.turn == 1 {
            return vec![];
        }
        let (x, y, cell) = grid(s);
        let p = Point::new(p.x * s.width, p.y * s.height);
        let mut near = (usize::MAX, cell * 0.28);
        for (e, &owner) in self.edges.iter().enumerate() {
            if owner >= 0 {
                continue;
            }
            let (a, b) = endpoints(e);
            let a = Point::new(x + a.0 as f64 * cell, y + a.1 as f64 * cell);
            let b = Point::new(x + b.0 as f64 * cell, y + b.1 as f64 * cell);
            let px = p.x.clamp(a.x, b.x);
            let py = p.y.clamp(a.y, b.y);
            let dist = (p.x - px).hypot(p.y - py);
            if dist < near.1 {
                near = (e, dist);
            }
        }
        if let Some(n) = self.play(near.0) {
            vec![if n > 0 {
                Feedback::Score
            } else {
                Feedback::Tap
            }]
        } else {
            vec![]
        }
    }
    fn draw(&self, d: &mut Draw, s: Size, _: &[Option<day_core::Bitmap>]) {
        let bg = [0xFF5F57, 0x00C2F0, 0x76CA3E, 0xFFBF2F][self.turn];
        d.fill(
            Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)),
            Color::hex(bg),
        );
        let (x, y, c) = grid(s);
        d.fill(
            Shape::RoundedRect(Rect::new(0.0, y - c * 0.75, s.width, c * 6.5), c * 0.45),
            Color::hex([0xFFE7E6, 0xD9F6FC, 0xEAF7DE, 0xFFF5D8][self.turn]),
        );
        for b in 0..25 {
            if self.boxes[b] >= 0 {
                let color = COLORS[self.boxes[b] as usize];
                let pop = if self.last.is_some_and(|e| Game::boundaries(b).contains(&e)) {
                    (self.pulse / 0.22).min(1.0)
                } else {
                    1.0
                };
                let r = c * 0.5 * pop;
                d.fill(
                    Shape::Rect(Rect::new(
                        x + (b % 5) as f64 * c + c * 0.5 - r,
                        y + (b / 5) as f64 * c + c * 0.5 - r,
                        r * 2.0,
                        r * 2.0,
                    )),
                    color,
                );
            }
        }
        for e in 0..60 {
            let (a, b) = endpoints(e);
            let a = Point::new(x + a.0 as f64 * c, y + a.1 as f64 * c);
            let mut b = Point::new(x + b.0 as f64 * c, y + b.1 as f64 * c);
            let filled = self.edges[e] >= 0;
            let color = if filled {
                COLORS[self.edges[e] as usize]
            } else {
                Color::WHITE
            };
            if self.last == Some(e) {
                let t = (self.pulse / 0.18).min(1.0);
                b = Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            }
            line(d, a, b, c * 0.17, color);
        }
        for yy in 0..6 {
            for xx in 0..6 {
                circle(
                    d,
                    Point::new(x + xx as f64 * c, y + yy as f64 * c),
                    c * 0.16,
                    Color::BLACK,
                );
            }
        }
        host::scores(d, s, self);
        text(
            d,
            &self.status().format(),
            Point::new(s.width * 0.5, (y - c).max(100.0)),
            16.0,
            Color::hex(0x434343),
        );
    }
}
pub fn dotsboxes_page(players: usize) -> AnyPiece {
    host::page::<Game>(
        players,
        host::Config {
            key: "dotsboxes.v1",
            settings_key: "dotsboxes.settings",
            title: res::str::nav_dotsboxes,
            rules: res::str::dotsboxes_rules,
            board_id: id!(dotsboxes_board),
            pause_id: id!(dotsboxes_pause),
            help_id: id!(dotsboxes_help_done),
            background: SURFACE,
            sprites: vec![],
            sounds: SOUNDS,
        },
    )
}
pub fn dotsboxes_preview() -> AnyPiece {
    let mut g = Game::new(2, 1);
    for e in [0, 5, 30, 31, 1, 6, 32, 10, 36, 37] {
        g.delay = 0.0;
        g.play(e);
    }
    canvas(move |d, s| {
        d.transformed(
            Affine::scale(s.width / 360.0, s.width / 360.0)
                .then(Affine::translate(0.0, -190.0 * s.width / 360.0)),
            |d| g.draw(d, Size::new(360.0, 780.0), &[]),
        );
    })
    .grow(Dimension::Both)
    .any()
}

pub const SOUNDS: &[gamekit::chrome::Sfx] = &[
    res::assets::sounds::dotsboxes::line_wav,
    res::assets::sounds::dotsboxes::box_wav,
    res::assets::sounds::shared::tap_wav,
];
