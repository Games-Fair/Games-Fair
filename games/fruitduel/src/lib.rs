// SPDX-License-Identifier: GPL-2.0-or-later
mod decoy;
mod effects;
mod model;
use day_core::Bitmap;
use day_localization_ui::LocalizedText;
use day_pieces::prelude::*;
use day_spec::{AssetName, TouchPhase};
use game_resources::res;
use gamekit::{
    board_game::{self as host, BoardGame, Feedback, GameOption, circle, text},
    multiplayer::Vec2,
};
use model::{FRUIT, Game, SCIMITAR, world};
pub const SURFACE: Color = Color::hex(0x935B98);
const COLORS: [Color; 4] = [
    Color::hex(0xFF5F57),
    Color::hex(0x00C2F0),
    Color::hex(0x76CA3E),
    Color::hex(0xFFBF2F),
];
fn assets() -> Vec<AssetName> {
    use res::assets::fruitduel::*;
    vec![
        orange_png,
        pomegranate_png,
        watermelon_png,
        present_png,
        flower_png,
        ball_png,
        orange_cut_png,
        pomegranate_cut_png,
        watermelon_cut_png,
        fruit_shadow_png,
        pomegranate_shadow_png,
        orange_cut_shadow_png,
        pomegranate_cut_shadow_png,
        watermelon_cut_shadow_png,
        star_png,
    ]
}
/// Corner controls retain comfortable touch targets at narrow sizes.
fn control(p: usize, s: Size) -> (Point, f64) {
    let r = (s.width * 0.13).min(s.height * 0.065).max(32.0);
    let x = if p == 0 || p == 2 {
        s.width - r * 1.12
    } else {
        r * 1.12
    };
    let y = if p == 0 || p == 3 {
        s.height - r * 1.4
    } else {
        r * 1.4
    };
    (Point::new(x, y), r)
}
#[allow(clippy::too_many_arguments)] // A sprite draw has independent source, transform, and opacity.
fn sprite(
    d: &mut Draw,
    images: &[Option<Bitmap>],
    i: usize,
    meta: &model::Sprite,
    at: Vec2,
    scale: f64,
    angle: f64,
    opacity: f64,
) {
    let Some(Some(im)) = images.get(i) else {
        return;
    };
    let [w, h] = [meta.size[0] * scale, meta.size[1] * scale];
    // World y is upward; bitmap coordinates are downward.
    let m = Affine::scale(1.0, -1.0)
        .then(Affine::rotate(Angle::rad(angle)))
        .then(Affine::translate(at.x, at.y));
    d.transformed(m, |d| {
        d.image_with_opacity(
            im,
            Rect::new(-w * meta.pivot[0], -h * (1.0 - meta.pivot[1]), w, h),
            opacity,
        )
    });
}
fn board_scale(s: Size) -> f64 {
    (s.height / 11.9).min(s.width / 5.8)
}
fn sword_transform(p: usize, angle: f64) -> Affine {
    let a = world(p, angle, Vec2::default());
    let x = world(p, angle, Vec2::new(1.0, 0.0)) - a;
    let y = world(p, angle, Vec2::new(0.0, 1.0)) - a;
    Affine {
        a: x.x,
        b: x.y,
        c: y.x,
        d: y.y,
        tx: a.x,
        ty: a.y,
    }
}
impl Game {
    fn paint(&self, d: &mut Draw, s: Size, images: &[Option<Bitmap>]) {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), SURFACE);
        // Keep the full arc between the scoreboards, including on narrow phones.
        let k = board_scale(s);
        let m = Affine::scale(k, -k).then(Affine::translate(s.width * 0.5, s.height * 0.5));
        d.transformed(m, |d| {
            // Render real sprite shadows before the solid pieces, including the detached fruit lid.
            for (p, sword) in self.swords.iter().take(self.count).enumerate() {
                let transform =
                    sword_transform(p, sword.angle()).then(Affine::translate(0.13, -0.16));
                d.transformed(transform, |d| {
                    sprite(d, images, 19, &SCIMITAR, Vec2::default(), 1.0, 0.0, 1.0);
                });
            }
            let t = (self.split_age * 2.5).clamp(0.0, 1.0);
            let t = 2.0 * t - t * t;
            let offset = self.split_offset * t;
            let angle = self.item_angle.to_radians();
            let top_angle = angle + self.split_rotation.to_radians();
            let top_shadow = if self.item == 1 { 10 } else { 9 };
            if self.split_age >= 0.0 {
                let bottom_shadow = if self.item == 0 { 9 } else { 11 + self.item };
                sprite(
                    d,
                    images,
                    bottom_shadow,
                    &FRUIT,
                    Vec2::new(0.31, -0.29),
                    0.5,
                    angle,
                    1.0,
                );
                sprite(
                    d,
                    images,
                    top_shadow,
                    &FRUIT,
                    offset + Vec2::new(0.31, -0.29),
                    0.5,
                    top_angle,
                    1.0,
                );
                sprite(
                    d,
                    images,
                    6 + self.item,
                    &FRUIT,
                    Vec2::default(),
                    0.5,
                    angle,
                    1.0,
                );
            } else {
                sprite(
                    d,
                    images,
                    top_shadow,
                    &FRUIT,
                    Vec2::new(0.31, -0.29),
                    0.5,
                    angle,
                    1.0,
                );
            }
            sprite(d, images, self.item, &FRUIT, offset, 0.5, top_angle, 1.0);
        });
        self.swing_trails(d, m);
        d.transformed(m, |d| {
            self.effects(d, images);
            for (p, sword) in self.swords.iter().take(self.count).enumerate() {
                d.transformed(sword_transform(p, sword.angle()), |d| {
                    sprite(d, images, 15 + p, &SCIMITAR, Vec2::default(), 1.0, 0.0, 1.0);
                });
            }
        });
        for p in 0..self.count {
            let (center, r) = control(p, s);
            let right = p == 0 || p == 2;
            let bottom = p == 0 || p == 3;
            let c = COLORS[p];
            let unit = r / 36.0;
            let sx = center.x + if right { -60.0 * unit } else { 60.0 * unit };
            let dark = Color::hex([0xD34035, 0x00A1C8, 0x59A52D, 0xD49B1B][p]);
            let rotation = match p {
                0 => -45.0,
                1 => 135.0,
                2 => -135.0,
                _ => 45.0,
            };
            let rect = Rect::new(
                sx.min(center.x) - 20.0 * unit,
                center.y - 20.0 * unit,
                (sx - center.x).abs() + 40.0 * unit,
                40.0 * unit,
            );
            d.fill(Shape::RoundedRect(rect, 20.0 * unit), Color::BLACK);
            d.fill(
                Shape::RoundedRect(
                    Rect::new(
                        rect.origin.x + 5.0 * unit,
                        rect.origin.y + 5.0 * unit,
                        rect.size.width - 10.0 * unit,
                        rect.size.height - 10.0 * unit,
                    ),
                    15.0 * unit,
                ),
                Color::hex(0x2F323C),
            );
            d.transformed(
                Affine::scale(unit, unit)
                    .then(Affine::rotate(Angle::deg(rotation)))
                    .then(Affine::translate(center.x, center.y)),
                |d| {
                    d.fill(
                        Shape::RoundedRect(Rect::new(-40.0, -40.0, 80.0, 132.0), 40.0),
                        Color::BLACK,
                    );
                    d.fill(
                        Shape::RoundedRect(Rect::new(-36.0, -36.0, 72.0, 123.0), 36.0),
                        dark,
                    );
                },
            );
            circle(d, center, 32.5 * unit, Color::BLACK);
            let disabled = self.swords[p].active || self.played[p] || self.finished;
            circle(
                d,
                center,
                28.0 * unit,
                if disabled {
                    Color::hex(0x5A626C)
                } else {
                    Color::hex([0xFF8A84, 0x6BE3FF, 0xAFE385, 0xFFDA83][p])
                },
            );
            circle(
                d,
                center,
                25.0 * unit,
                if disabled { Color::hex(0x424952) } else { c },
            );
            let label = if self.humans == 1 && p == 1 {
                res::str::fs_solo()
            } else {
                res::str::party_player((p + 1) as f64)
            };
            let face = if bottom {
                Affine::IDENTITY
            } else {
                Affine::translate(-sx, -center.y)
                    .then(Affine::rotate(Angle::HALF))
                    .then(Affine::translate(sx, center.y))
            };
            d.transformed(face, |d| {
                text(
                    d,
                    &label.format(),
                    Point::new(sx, center.y - r * 0.30),
                    r * 0.19,
                    c,
                );
                text(
                    d,
                    &res::str::party_count(self.scores[p] as f64).format(),
                    Point::new(sx, center.y + r * 0.10),
                    r * 0.64,
                    c,
                );
            });
        }
    }
}
impl BoardGame for Game {
    fn new(p: usize, s: u64) -> Self {
        Self::new(p, s)
    }
    fn restart(&self, p: usize, s: u64) -> Self {
        let mut g = Self::new(p, s);
        g.difficulty = self.difficulty;
        g.next_item();
        g
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
        self.first.unwrap_or(0)
    }
    fn score(&self, p: usize) -> u32 {
        self.scores[p]
    }
    fn finished(&self) -> bool {
        self.finished
    }
    fn haptic(event: Feedback, default: gamekit::chrome::Pattern) -> gamekit::chrome::Pattern {
        if matches!(event, Feedback::Miss) {
            gamekit::chrome::THUD
        } else {
            default
        }
    }
    fn simultaneous_touches() -> bool {
        true
    }
    fn status(&self) -> LocalizedText {
        res::str::party_everyone_plays()
    }
    fn tick(&mut self, dt: f64) -> Vec<Feedback> {
        self.tick(dt)
    }
    fn touch(&mut self, phase: TouchPhase, p: Point, s: Size) -> Vec<Feedback> {
        if phase != TouchPhase::Began {
            return vec![];
        }
        let p = Point::new(p.x * s.width, p.y * s.height);
        for player in 0..self.humans {
            let (c, r) = control(player, s);
            if (p.x - c.x).hypot(p.y - c.y) <= r * 1.18 {
                return self.press(player);
            }
        }
        vec![]
    }
    fn draw(&self, d: &mut Draw, s: Size, images: &[Option<Bitmap>]) {
        self.paint(d, s, images)
    }
    fn options(&self) -> Vec<GameOption> {
        if self.humans != 1 {
            return vec![];
        }
        vec![GameOption {
            title: res::str::fs_difficulty(),
            choices: vec![
                res::str::fs_easy(),
                res::str::fs_medium(),
                res::str::fs_hard(),
            ],
            selected: self.difficulty,
        }]
    }
    fn set_option(&mut self, index: usize, value: usize) {
        if index == 0 && value < 3 {
            self.difficulty = value;
        }
    }
}
pub fn fruitduel_page(players: usize) -> AnyPiece {
    use gamekit::sprites::{Ink, TEAM_INKS, VectorSprite};
    let sword = res::assets::fruitduel::scimitar_svg;
    let colors = TEAM_INKS.map(|rgb| VectorSprite::new(sword.clone(), Ink::Team(rgb)));
    let shadow = VectorSprite::new(sword, Ink::Shadow);
    host::page_with_art::<Game>(
        players,
        host::Config {
            key: "fruitduel.v2",
            settings_key: "fruitduel.settings",
            title: res::str::nav_fruitduel,
            rules: res::str::fruitduel_rules,
            board_id: id!(fruitduel_board),
            pause_id: id!(fruitduel_pause),
            help_id: id!(fruitduel_help_done),
            background: SURFACE,
            sprites: assets(),
            sounds: SOUNDS,
        },
        move |size, sprites| {
            let width = board_scale(size) * SCIMITAR.size[0];
            sprites.extend(colors.iter().enumerate().map(|(i, ink)| {
                if i < players.max(2) {
                    ink.at_width(width)
                } else {
                    None
                }
            }));
            sprites.push(shadow.at_width(width));
        },
    )
}
pub fn fruitduel_preview() -> AnyPiece {
    let banner = gamekit::sprites::load(res::assets::fruitduel::banner_png);
    canvas(move |d, s| {
        d.fill(Shape::Rect(Rect::new(0.0, 0.0, s.width, s.height)), SURFACE);
        if let Some(b) = banner.get() {
            d.image(&b, Rect::new(0.0, 0.0, s.width, s.height));
        }
    })
    .grow(Dimension::Both)
    .any()
}
pub const SOUNDS: &[gamekit::chrome::Sfx] = &[
    res::assets::sounds::fruitduel::swoosh_wav,
    res::assets::sounds::fruitduel::cut_wav,
    res::assets::sounds::shared::tap_wav,
    res::assets::sounds::fruitduel::clang_wav,
];
