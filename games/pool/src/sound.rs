// SPDX-License-Identifier: GPL-2.0-or-later
//! Material-specific Pool Foley. Velocity changes timbre and gain, never playback pitch.
use game_resources::res::assets::sounds::pool as clips;
use gamekit::chrome;

pub const CLIPS: &[chrome::Sfx] = &[
    clips::ball_soft_wav,
    clips::ball_medium_wav,
    clips::ball_firm_wav,
    clips::cue_soft_wav,
    clips::cue_medium_wav,
    clips::cue_firm_wav,
    clips::cushion_soft_wav,
    clips::cushion_medium_wav,
    clips::cushion_firm_wav,
    clips::pocket_wav,
];
#[derive(Clone, Copy)]
pub enum Contact {
    Cue,
    Ball,
    Cushion,
}
fn response(speed: f64) -> (usize, f32) {
    let strength = (speed / 700.0).clamp(0.0, 1.0);
    let tier = if speed < 170.0 {
        0
    } else if speed < 420.0 {
        1
    } else {
        2
    };
    (tier, (0.16 + 0.68 * strength.powf(0.65)) as f32)
}
pub fn play(enabled: bool, kind: Contact, speed: f64) {
    if !enabled || !speed.is_finite() || speed <= 8.0 {
        return;
    }
    let (tier, gain) = response(speed);
    let offset = match kind {
        Contact::Ball => 0,
        Contact::Cue => 3,
        Contact::Cushion => 6,
    };
    chrome::sound(true, &CLIPS[offset + tier], gain);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strength_changes_material_excitation_and_gain_without_exceeding_headroom() {
        let mut previous = 0.0;
        for speed in 9..=1400 {
            let (_, gain) = response(speed as f64);
            assert!(gain >= previous && gain <= 0.85);
            previous = gain;
        }
        assert_eq!(response(70.0).0, 0);
        assert_eq!(response(250.0).0, 1);
        assert_eq!(response(600.0).0, 2);
        assert!(response(70.0).1 < response(600.0).1 * 0.5);
    }
}
