// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
mod collision;
mod model;
use day_pieces::prelude::*;
use day_spec::TouchPhase;
use game_resources::res;
use gamekit::board_game::{self as host, BoardGame, Feedback, circle, line, text};
use model::{FLOOR, Game, SIZES, WIDTH};
pub const SURFACE: Color = Color::hex(0xC4DFC3);
const COLORS: [Color; 11] = [
    Color::hex(0xF45055),
    Color::hex(0xF48FA4),
    Color::hex(0xAD7ED0),
    Color::hex(0xFFB34C),
    Color::hex(0xF47747),
    Color::hex(0xE45B55),
    Color::hex(0xFFE15D),
    Color::hex(0xFFD586),
    Color::hex(0xEAD898),
    Color::hex(0xBBDE72),
    Color::hex(0x80B84F),
];
fn layout(s: Size) -> (f64, f64, f64) {
    let k = (s.width * 0.909 / WIDTH).min(s.height * 0.525 / FLOOR);
    ((s.width - WIDTH * k) * 0.5, s.height * 0.329, k)
}
fn fruit(
    d: &mut Draw,
    sheet: Option<&day_core::Bitmap>,
    kind: usize,
    p: Point,
    r: f64,
    angle: f64,
    alpha: f64,
) {
    if let Some(sheet) = sheet {
        d.transformed(
            Affine::translate(-p.x, -p.y)
                .then(Affine::rotate(Angle::rad(angle)))
                .then(Affine::translate(p.x, p.y)),
            |d| {
                d.image_region_with_opacity(
                    sheet,
                    Rect::new(
                        (kind % 4) as f64 * 256.0,
                        (kind / 4) as f64 * 256.0,
                        256.0,
                        256.0,
                    ),
                    Rect::new(p.x - r, p.y - r, 2.0 * r, 2.0 * r),
                    alpha,
                )
            },
        );
    }
}
impl BoardGame for Game {
    fn new(_: usize, seed: u64) -> Self {
        let mut g = Self::new(seed);
        g.shapes = day_spec::resource(res::assets::fruitmerge::colliders_json)
            .and_then(|r| serde_json::from_slice(r.as_slice()).ok())
            .unwrap_or_default();
        g.best = gamekit::restore::<u32>("fruitmerge.best").unwrap_or(0);
        g
    }
    fn restart(&self, _: usize, seed: u64) -> Self {
        let mut g = <Self as BoardGame>::new(1, seed);
        g.difficulty = self.difficulty;
        g
    }
    fn pause_alignment() -> Alignment {
        Alignment::TopLeading
    }
    fn options(&self) -> Vec<host::GameOption> {
        vec![host::GameOption {
            title: res::str::su_difficulty(),
            choices: vec![
                res::str::su_easy(),
                res::str::su_medium(),
                res::str::su_hard(),
            ],
            selected: self.difficulty,
        }]
    }
    fn set_option(&mut self, _: usize, v: usize) {
        self.difficulty = v.min(2);
    }
    fn commands(&self) -> Vec<(day_localization_ui::LocalizedText, bool)> {
        vec![
            (res::str::sol_undo(), !self.history.is_empty()),
            (res::str::fruitmerge_sweep(), !self.swept),
        ]
    }
    fn command(&mut self, i: usize) -> Vec<Feedback> {
        if i == 0 && self.undo() {
            vec![Feedback::Turn]
        } else if i == 1 && self.sweep() {
            vec![Feedback::Sweep]
        } else {
            vec![]
        }
    }
    fn valid(&self) -> bool {
        self.valid()
    }
    fn players(&self) -> usize {
        1
    }
    fn turn(&self) -> usize {
        0
    }
    fn score(&self, _: usize) -> u32 {
        self.score
    }
    fn finished(&self) -> bool {
        self.finished
    }
    fn tick(&mut self, dt: f64) -> Vec<Feedback> {
        let best = self.best;
        let events = self.tick(dt);
        if self.best > best {
            gamekit::save("fruitmerge.best", &self.best);
        }
        events
    }
    fn clear_records(&mut self) {
        self.best = self.score;
    }
    fn cancel(&mut self) {
        self.held = false;
    }
    fn status(&self) -> day_localization_ui::LocalizedText {
        res::str::party_count(self.score as f64)
    }
    fn touch(&mut self, phase: TouchPhase, p: Point, s: Size) -> Vec<Feedback> {
        let (x, y, k) = layout(s);
        let px = p.x * s.width;
        let py = p.y * s.height;
        let cy = y * 0.62;
        let r = (s.width * 0.075).min(32.0);
        if phase == TouchPhase::Ended && (py - cy).abs() < r * 1.2 {
            if (px - s.width * 0.11).abs() < r * 1.2 {
                return self.command(1);
            }
            if (px - s.width * 0.34).abs() < r * 1.2 {
                return self.command(0);
            }
        }
        match phase {
            TouchPhase::Began if py > y - 90.0 * k && py < y + FLOOR * k => {
                self.held = true;
                self.aim = ((px - x) / k)
                    .clamp(SIZES[self.next[0]] * 0.5, WIDTH - SIZES[self.next[0]] * 0.5);
            }
            TouchPhase::Moved if self.held => self.aim = ((px - x) / k).clamp(0.0, WIDTH),
            TouchPhase::Ended if self.held => {
                self.held = false;
                if py > y - 90.0 * k && self.drop() {
                    return vec![Feedback::Tap];
                }
            }
            TouchPhase::Cancelled => self.held = false,
            _ => {}
        }
        vec![]
    }
    fn draw(&self, d: &mut Draw, s: Size, sprites: &[Option<day_core::Bitmap>]) {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), SURFACE);
        let sheet = sprites.first().and_then(|s| s.as_ref());
        let (x, y, k) = layout(s);
        let p = |a, b| Point::new(x + a * k, y + b * k);
        let fw = WIDTH * k;
        let bottom = y + FLOOR * k;
        let wall = (s.width * 0.045).min(25.0);
        let top = y - 38.0 * k;
        let frame = Rect::new(
            x - wall,
            top - 24.0 * k,
            fw + 2.0 * wall,
            bottom - top + wall + 24.0 * k,
        );
        d.fill(
            Shape::Rect(Rect::new(x, top, fw, bottom - top)),
            Color::hex(0xFFE9B0),
        );
        d.clipped(Shape::Rect(Rect::new(x, top, fw, bottom - top)), |d| {
            let cell = fw / 16.0;
            for row in 0..40 {
                for col in 0..16 {
                    if (row + col) % 2 == 0 {
                        d.fill(
                            Shape::Rect(Rect::new(
                                x + col as f64 * cell,
                                top + row as f64 * cell,
                                cell,
                                cell,
                            )),
                            Color::hex(0xFFE0A0),
                        );
                    }
                }
            }
        });
        d.fill(
            Shape::Rect(Rect::new(x, top, 8.0 * k, bottom - top)),
            Color::hex(0xF9D198),
        );
        d.fill(
            Shape::Rect(Rect::new(x, bottom - 20.0 * k, fw, 20.0 * k)),
            Color::hex(0xF9D198),
        );
        let terracotta = Color::hex(0xC6633F);
        line(
            d,
            Point::new(x - wall * 0.5, top - 20.0 * k),
            Point::new(x - wall * 0.5, bottom + wall * 0.5),
            wall,
            terracotta,
        );
        line(
            d,
            Point::new(x + fw + wall * 0.5, top - 20.0 * k),
            Point::new(x + fw + wall * 0.5, bottom + wall * 0.5),
            wall,
            terracotta,
        );
        d.fill(
            Shape::Rect(Rect::new(x - wall, bottom, fw + wall * 2.0, wall)),
            terracotta,
        );
        let danger = self
            .fruits
            .iter()
            .map(|f| f.overflow)
            .fold(0.0_f64, f64::max);
        for i in 0..13 {
            let a = i as f64 * WIDTH / 13.0;
            line(
                d,
                p(a + 12.0, 0.0),
                p(a + 42.0, 0.0),
                6.0 * k,
                Color::hex(0xE5825C),
            );
        }
        let header_y = (y * 0.22).max(34.0);
        let bw = s.width * 0.245;
        for (i, title, value, color) in [
            (
                0,
                [
                    res::str::su_easy(),
                    res::str::su_medium(),
                    res::str::su_hard(),
                ][self.difficulty]
                    .clone(),
                self.score,
                Color::WHITE,
            ),
            (1, res::str::gk_best(), self.best, Color::hex(0x2EDDB7)),
        ] {
            let xx = s.width * (0.235 + i as f64 * 0.29);
            d.fill(
                Shape::RoundedRect(Rect::new(xx, header_y, bw, 43.0), 8.0),
                Color::hex(0x485747),
            );
            text(
                d,
                &title.format(),
                Point::new(xx + bw * 0.5, header_y + 12.0),
                12.0,
                color,
            );
            text(
                d,
                &res::str::party_count(value as f64).format(),
                Point::new(xx + bw * 0.5, header_y + 31.0),
                24.0,
                color,
            );
        }
        let controls_y = y * 0.62;
        let rr = (s.width * 0.075).min(32.0);
        for (i, enabled) in [(0, !self.swept), (1, !self.history.is_empty())] {
            let at = Point::new(s.width * (0.11 + i as f64 * 0.23), controls_y);
            circle(
                d,
                at,
                rr,
                if enabled {
                    Color::hex(0xDA61A2)
                } else {
                    Color::hex(0x2D323B)
                },
            );
            let ink = if enabled {
                Color::WHITE
            } else {
                Color::hex(0x6B7580)
            };
            if i == 1 {
                let mut path =
                    PathBuilder::new().move_to(Point::new(at.x - rr * 0.45, at.y - rr * 0.3));
                for j in 0..20 {
                    let a = -std::f64::consts::FRAC_PI_2 + j as f64 * std::f64::consts::PI / 19.0;
                    path = path.line_to(Point::new(
                        at.x + rr * 0.3 * a.cos(),
                        at.y + rr * 0.3 + rr * 0.55 * a.sin(),
                    ));
                }
                d.stroke(path.build(), ink, 4.0);
                line(
                    d,
                    Point::new(at.x - rr * 0.6, at.y - rr * 0.25),
                    Point::new(at.x - rr * 0.2, at.y - rr * 0.6),
                    4.0,
                    ink,
                );
                line(
                    d,
                    Point::new(at.x - rr * 0.6, at.y - rr * 0.25),
                    Point::new(at.x - rr * 0.2, at.y + rr * 0.1),
                    4.0,
                    ink,
                );
            } else {
                line(
                    d,
                    Point::new(at.x + rr * 0.5, at.y - rr * 0.65),
                    Point::new(at.x - rr * 0.1, at.y + rr * 0.1),
                    4.0,
                    ink,
                );
                d.stroke(
                    PathBuilder::new()
                        .move_to(Point::new(at.x - rr * 0.1, at.y))
                        .line_to(Point::new(at.x - rr * 0.65, at.y + rr * 0.3))
                        .line_to(Point::new(at.x - rr * 0.1, at.y + rr * 0.7))
                        .line_to(Point::new(at.x + rr * 0.2, at.y + rr * 0.2))
                        .close()
                        .build(),
                    ink,
                    3.0,
                );
            }
            circle(
                d,
                Point::new(at.x + rr * 0.65, at.y - rr * 0.75),
                rr * 0.34,
                Color::WHITE,
            );
            text(
                d,
                &res::str::party_count(if enabled { 1.0 } else { 0.0 }).format(),
                Point::new(at.x + rr * 0.65, at.y - rr * 0.75),
                rr * 0.65,
                Color::hex(0x444C55),
            );
        }
        let tray = Rect::new(
            s.width * 0.59,
            controls_y - rr * 1.12,
            s.width * 0.38,
            rr * 2.24,
        );
        glass(d, tray);
        for i in 0..3 {
            let at = Point::new(
                tray.origin.x + tray.size.width * (i as f64 + 0.5) / 3.0,
                controls_y,
            );
            fruit(
                d,
                sheet,
                self.next[i + 1],
                at,
                (SIZES[self.next[i + 1]] * k * 0.5).min(rr * 0.7),
                0.0,
                1.0,
            );
        }
        if !self.finished {
            let r = SIZES[self.next[0]] * 0.5;
            let aim = self.aim.clamp(r, WIDTH - r);
            let at = p(aim, -46.0);
            for i in 0..24 {
                circle(
                    d,
                    p(aim, 40.0 + i as f64 * (FLOOR - 40.0) / 24.0),
                    3.2 * k,
                    Color::hex(0xDD5540),
                );
            }
            fruit(
                d,
                sheet,
                self.next[0],
                at,
                r * k,
                0.0,
                if self.delay > 0.0 { 0.65 } else { 1.0 },
            );
        }
        for f in &self.fruits {
            let at = p(f.p.x, f.p.y);
            let pop = if f.merge_age < 0.15 {
                0.8 + 0.2 * (f.merge_age / 0.15)
            } else {
                1.0
            };
            circle(
                d,
                Point::new(at.x + 3.0 * k, at.y + 5.0 * k),
                f.radius() * k,
                Color::rgba(0.4, 0.3, 0.15, 0.12),
            );
            fruit(d, sheet, f.kind, at, f.radius() * k * pop, f.angle, 1.0);
        }
        for b in &self.bursts {
            let t = b.age / 0.5;
            for j in 0..12 {
                let a = j as f64 * std::f64::consts::TAU / 12.0;
                circle(
                    d,
                    p(
                        b.p.x + a.cos() * SIZES[b.kind] * t,
                        b.p.y + a.sin() * SIZES[b.kind] * t + 70.0 * t * t,
                    ),
                    (1.0 - t) * 4.0,
                    COLORS[b.kind],
                );
            }
        }
        let evolution = Rect::new(
            s.width * 0.03,
            bottom + wall + 14.0,
            s.width * 0.94,
            40.0_f64.min(s.height * 0.07),
        );
        glass(d, evolution);
        for i in 0..11 {
            let xx = evolution.origin.x + evolution.size.width * (i as f64 + 0.5) / 11.0;
            fruit(
                d,
                sheet,
                i,
                Point::new(xx, evolution.origin.y + evolution.size.height * 0.5),
                (s.width / 28.0 + i as f64 * 0.18).min(evolution.size.height * 0.42),
                0.0,
                1.0,
            );
        }
        if danger > 0.0 {
            d.stroke(
                Shape::RoundedRect(frame, wall * 0.5),
                Color::rgba(1.0, 0.2, 0.2, 0.4 + 0.3 * (self.elapsed * 9.0).sin()),
                3.0,
            );
        }
    }
}
fn glass(d: &mut Draw, r: Rect) {
    d.fill(
        Shape::RoundedRect(r, r.size.height * 0.5),
        Color::hex(0xCEE5CA),
    );
    d.stroke(
        Shape::RoundedRect(r, r.size.height * 0.5),
        Color::hex(0xB1D5AF),
        3.0,
    );
    d.stroke(
        Shape::RoundedRect(
            Rect::new(
                r.origin.x + 5.0,
                r.origin.y + 5.0,
                r.size.width - 10.0,
                r.size.height - 10.0,
            ),
            r.size.height * 0.5 - 5.0,
        ),
        Color::rgba(1.0, 1.0, 1.0, 0.45),
        1.8,
    );
}

pub const SOUNDS: &[gamekit::chrome::Sfx] = &[
    res::assets::sounds::fruitmerge::drop_wav,
    res::assets::sounds::fruitmerge::merge_wav,
    res::assets::sounds::fruitmerge::undo_wav,
    res::assets::sounds::fruitmerge::collision_wav,
    res::assets::sounds::fruitmerge::collision_wav,
    res::assets::sounds::fruitmerge::sweep_wav,
];
pub fn fruitmerge_page() -> AnyPiece {
    host::page::<Game>(
        1,
        host::Config {
            key: "fruitmerge.v2",
            settings_key: "fruitmerge.settings",
            title: res::str::nav_fruitmerge,
            rules: res::str::fruitmerge_rules,
            board_id: id!(fruitmerge_board),
            pause_id: id!(fruitmerge_pause),
            help_id: id!(fruitmerge_help_done),
            background: SURFACE,
            sprites: vec![res::assets::fruitmerge::fruits_png],
            sounds: SOUNDS,
        },
    )
}
pub fn fruitmerge_preview() -> AnyPiece {
    let sheet = gamekit::sprites::load(res::assets::fruitmerge::fruits_png);
    canvas(move |d, s| {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), SURFACE);
        for (i, x, y, r) in [
            (10, 0.55, 0.58, 0.34),
            (5, 0.2, 0.3, 0.19),
            (3, 0.78, 0.2, 0.12),
            (2, 0.19, 0.77, 0.13),
        ] {
            fruit(
                d,
                sheet.get().as_ref(),
                i,
                Point::new(s.width * x, s.height * y),
                s.width * r,
                0.0,
                1.0,
            );
        }
    })
    .grow(Dimension::Both)
    .any()
}

pub fn reset_records() {
    gamekit::clear("fruitmerge.best");
    if !gamekit::reset_live_records("fruitmerge.v2")
        && let Some(mut game) = gamekit::restore::<Game>("fruitmerge.v2")
    {
        game.best = game.score;
        gamekit::save("fruitmerge.v2", &game);
    }
}
