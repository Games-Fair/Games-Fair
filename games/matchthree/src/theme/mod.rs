// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Themes: the same game in different dress. A theme only changes what is seen, heard and
//! felt; the rules, the levels and the timing of every move stay the same, so a theme can be
//! changed in the middle of a level.
//!
//! To add one: write a module with its [`Style`] (see `jewels` and `stones`), list it in
//! [`Theme`], [`THEMES`] and [`Theme::style`], add its clips to [`SOUNDS`], and give its name
//! (`mt_theme_<key>`) to every locale.
use crate::fx::{self, Look};
use crate::model::Candy;
use day_pieces::prelude::*;
use gamekit::chrome::{self, Sfx, sfx};
use serde::{Deserialize, Serialize};

mod jewels;
mod stones;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// "Be Jewelled": faceted gemstones, glitter and starlight.
    #[default]
    Jewels,
    /// "Sea Stones": pebbles at the water's edge, foam and gentle waves.
    Stones,
}

/// Every theme, in the order Settings lists them.
pub const THEMES: [Theme; 2] = [Theme::Jewels, Theme::Stones];

impl Theme {
    pub fn style(self) -> &'static Style {
        match self {
            Theme::Jewels => &jewels::STYLE,
            Theme::Stones => &stones::STYLE,
        }
    }
    pub fn index(self) -> usize {
        THEMES.iter().position(|&t| t == self).unwrap_or(0)
    }
    pub fn from_index(i: usize) -> Self {
        THEMES.get(i).copied().unwrap_or_default()
    }
}

/// Every clip any theme plays, loaded up front so switching themes never waits on a sound.
pub const SOUNDS: &[Sfx] = &[
    sfx("sounds/matchthree/swap.wav"),
    sfx("sounds/matchthree/bloom.wav"),
    sfx("sounds/matchthree/cascade.wav"),
    sfx("sounds/matchthree/magic.wav"),
    sfx("sounds/matchthree/victory.wav"),
    sfx("sounds/matchthree/stones/swap.wav"),
    sfx("sounds/matchthree/stones/bloom.wav"),
    sfx("sounds/matchthree/stones/cascade.wav"),
    sfx("sounds/matchthree/stones/magic.wav"),
    sfx("sounds/matchthree/stones/victory.wav"),
];

/// Everything a theme decides.
pub struct Style {
    /// The Fluent key of the theme's name in Settings.
    pub name: &'static str,
    /// Draw one piece, plain or special, centered at (x, y) with radius `r`.
    pub piece: fn(&mut Draw, Candy, f64, f64, f64, Look),
    /// The color of piece `i`'s particles and flashes.
    pub tint: fn(u8) -> Color,
    pub board: Board,
    pub motion: Motion,
    pub cues: Cues,
}

/// The board under the pieces.
pub struct Board {
    /// The frame around the board, and its edge for each level.
    pub frame: Color,
    pub edges: [u32; 6],
    /// The checkerboard of cells, and the dot marking a cell outside the level's shape.
    pub tiles: [Color; 2],
    pub dot: Color,
    /// The chosen, hinted or held cell, and the keyboard cursor.
    pub mark: Color,
    pub cursor: Color,
    /// The points that pop up after a match: a single match, a second chain, and more.
    pub ink: [Color; 3],
}

/// How lively the effects are. Scales are relative to the jewels' 1.0.
pub struct Motion {
    pub finish: fx::Finish,
    pub ambience: fx::Ambience,
    /// Board shake and full-board flash on big matches.
    pub shake: f64,
    pub flash: f64,
    /// Whether pieces glint on their own while the board is moving.
    pub twinkle: bool,
    /// How far a clearing piece swells before it vanishes, and how bright its bloom is.
    pub swell: f64,
    pub bloom: f64,
    /// How much a falling piece bounces as it lands.
    pub bounce: f64,
    /// How strongly the chosen cell's outline breathes, and how fast.
    pub pulse: f64,
    pub pulse_rate: f64,
    /// How much the points pop in larger than they settle.
    pub pop: f64,
}

/// The sounds, with their haptics, of the moments that differ between themes.
pub struct Cues {
    pub swap: &'static chrome::Cue,
    pub bloom: &'static chrome::Cue,
    pub cascade: &'static chrome::Cue,
    /// Three chains or more.
    pub surge: &'static chrome::Cue,
    /// A special piece going off.
    pub magic: &'static chrome::Cue,
    pub victory: &'static chrome::Cue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_round_trips_through_its_index_and_its_saved_name() {
        for (i, &theme) in THEMES.iter().enumerate() {
            assert_eq!(theme.index(), i);
            assert_eq!(Theme::from_index(i), theme);
            let saved = serde_json::to_string(&theme).unwrap();
            assert_eq!(serde_json::from_str::<Theme>(&saved).unwrap(), theme);
        }
        assert_eq!(Theme::from_index(99), Theme::Jewels);
    }

    #[test]
    fn every_theme_clip_is_loaded_up_front() {
        for theme in THEMES {
            let c = &theme.style().cues;
            for cue in [c.swap, c.bloom, c.cascade, c.surge, c.magic, c.victory] {
                let sound = cue.sound.clone().expect("each theme cue has a sound");
                assert!(
                    SOUNDS.contains(&sound),
                    "a {} clip is missing from SOUNDS",
                    theme.style().name
                );
            }
        }
    }
}
