// SPDX-License-Identifier: GPL-2.0-or-later
use crate::model::{Game, HEIGHT, WIDTH};
use day_pieces::prelude::*;
use game_resources::res;
use gamekit::sprites::{Ink, TEAM_INKS, VectorSprite};
pub struct Sprites {
    heads: [VectorSprite; 4],
    bodies: [VectorSprite; 4],
}
impl Sprites {
    pub fn load() -> Self {
        Self {
            heads: TEAM_INKS
                .map(|rgb| VectorSprite::new(res::assets::snakes::head_svg, Ink::Team(rgb))),
            bodies: TEAM_INKS
                .map(|rgb| VectorSprite::new(res::assets::snakes::body_svg, Ink::Team(rgb))),
        }
    }
}
pub fn snakes(d: &mut Draw, game: &Game, sprites: &Sprites, scale: f64) {
    for (i, r) in game.riders.iter().enumerate() {
        if !r.alive {
            continue;
        }
        let head = sprites.heads[i].at_width(35. * scale);
        let body = sprites.bodies[i].at_width(23. * scale);
        // Draw neighboring copies only at the seams, preserving the continuous wrapping body.
        for dx in [-WIDTH, 0.0, WIDTH] {
            for dy in [-HEIGHT, 0.0, HEIGHT] {
                for (j, p) in r.body.iter().enumerate().rev().step_by(2) {
                    let radius = (11.5 * ((r.body.len() - j) as f64 / 9.0).min(1.0)).max(5.5);
                    let x = p.x + dx;
                    let y = p.y + dy;
                    if x < -15.0 || x > WIDTH + 15.0 || y < -15.0 || y > HEIGHT + 15.0 {
                        continue;
                    }
                    if let Some(body) = &body {
                        d.image(
                            body,
                            Rect::new(x - radius, y - radius, radius * 2.0, radius * 2.0),
                        );
                    }
                }
                let x = r.pos.x + dx;
                let y = r.pos.y + dy;
                if x < -20.0 || x > WIDTH + 20.0 || y < -20.0 || y > HEIGHT + 20.0 {
                    continue;
                }
                if let Some(head) = &head {
                    d.transformed(
                        Affine::rotate(Angle::rad(r.angle - std::f64::consts::FRAC_PI_2))
                            .then(Affine::translate(x, y)),
                        |d| {
                            d.image(head, Rect::new(-13.5, -22.0, 27.0, 35.0));
                        },
                    );
                }
            }
        }
    }
}
