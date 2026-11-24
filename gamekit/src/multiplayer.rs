// SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
//! Shared, legible player cards and turn-colored headings for the local multiplayer games.
use crate::art::COLORS;
use crate::chrome;
use day_geometry::Proposal;
use day_localization_ui::LocalizedText;
use day_pieces::prelude::*;
use game_resources::res;

#[derive(Clone, Copy, PartialEq)]
pub enum PlayerState {
    Turn,
    Playing,
    Waiting,
    Out,
    Winner,
    Finished,
}

pub fn header(
    title: LocalizedText,
    pause: ElementId,
    on_pause: impl Fn() + 'static,
    active: impl Fn() -> Option<usize> + 'static,
) -> AnyPiece {
    row((
        chrome::close_button(),
        switch(active, move |player| {
            chrome::fitted_title(
                title.clone(),
                chrome::TITLE,
                FontWeight::Heavy,
                player.map_or(Color::WHITE, |i| COLORS[i]),
            )
        })
        .grow(Dimension::Width),
        chrome::pause_button(res::str::gk_pause(), pause, on_pause),
    ))
    .align(VAlign::Center)
    .padding(chrome::EDGE)
    .any()
}

pub fn score_card(
    player: LocalizedText,
    value: LocalizedText,
    reservation: LocalizedText,
    index: usize,
    state: PlayerState,
    progress: Option<f64>,
) -> AnyPiece {
    let color = COLORS[index];
    let highlighted = matches!(state, PlayerState::Turn | PlayerState::Winner);
    let active = highlighted || state == PlayerState::Playing;
    let badge = match state {
        PlayerState::Turn => res::str::party_turn_badge(),
        PlayerState::Playing => res::str::party_playing_badge(),
        PlayerState::Waiting => res::str::party_waiting_badge(),
        PlayerState::Out => res::str::arena_out(),
        PlayerState::Winner => res::str::party_winner_badge(),
        PlayerState::Finished => res::str::party_finished_badge(),
    };
    let ink = if active {
        Color::WHITE
    } else {
        Color::hex(0xA8A8BC)
    };
    column((
        chrome::fitted_title(player, 13.0, FontWeight::Semibold, ink).grow(Dimension::Width),
        score_readout(value, reservation, if active { color } else { ink }).grow(Dimension::Width),
        chrome::fitted_title(
            badge,
            11.0,
            FontWeight::Bold,
            if active { color } else { ink },
        )
        .grow(Dimension::Width),
        progress.map(move |amount| {
            canvas(move |d, size| {
                let bounds = Rect::new(0.0, 0.0, size.width, 4.0);
                d.fill(
                    Shape::RoundedRect(bounds, 2.0),
                    Color::rgba(1.0, 1.0, 1.0, 0.10),
                );
                if amount > 0.0 {
                    d.fill(
                        Shape::RoundedRect(
                            Rect::new(0.0, 0.0, size.width * amount.clamp(0.0, 1.0), 4.0),
                            2.0,
                        ),
                        color,
                    );
                }
            })
            .height(4.0)
            .grow(Dimension::Width)
        }),
    ))
    .spacing(0.0)
    .padding(6.0)
    .surface(
        Surface::new()
            .fill(if highlighted {
                Color::rgba(
                    color.r * 0.22 + 0.08,
                    color.g * 0.22 + 0.08,
                    color.b * 0.22 + 0.10,
                    1.0,
                )
            } else {
                Color::hex(0x242438)
            })
            .corner_radius(12.0)
            .border(
                if active { color } else { Color::hex(0x424255) },
                if highlighted { 2.5 } else { 1.0 },
            ),
    )
    .grow(Dimension::Width)
    .any()
}

/// Size a scroll viewport to its content, bounded only by the available screen height.
/// Scroll itself is greedy; this wrapper asks for its natural height before clamping.
pub fn dialog(content: impl Piece) -> AnyPiece {
    let content = scroll(content);
    let fitting = day_core::piece_fn(move |cx: &mut day_core::BuildCx| {
        let node = cx.layout_only(
            std::rc::Rc::new(DialogLayout),
            day_core::Flex {
                grow_w: true,
                ..Default::default()
            },
            day_core::Boundary::No,
        );
        cx.under(node, |cx| {
            content.build(cx);
        });
        node
    });
    chrome::card(fitting)
}
struct DialogLayout;
impl day_core::Layout for DialogLayout {
    fn measure(
        &self,
        cx: &mut dyn day_core::LayoutOps,
        children: &[day_core::RNode],
        proposal: Proposal,
    ) -> Size {
        let natural = children.first().map_or(Size::ZERO, |&c| {
            cx.measure_child(c, Proposal::new(proposal.width, None))
        });
        Size::new(
            natural.width,
            proposal
                .height
                .map_or(natural.height, |h| natural.height.min(h)),
        )
    }
    fn place(&self, cx: &mut dyn day_core::LayoutOps, children: &[day_core::RNode], bounds: Rect) {
        if let Some(&child) = children.first() {
            cx.place_child(
                child,
                Rect::new(0.0, 0.0, bounds.size.width, bounds.size.height),
            );
        }
    }
}

#[derive(Clone, Copy, Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}
impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, b: Self) -> f64 {
        (self.x - b.x).hypot(self.y - b.y)
    }
    pub fn valid(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.x.abs() < 10000.0 && self.y.abs() < 10000.0
    }
}

/// Numeric slots have fixed advances and a reserved width. The font size depends only on the
/// reservation and available space, never the current score. Right alignment keeps decimal
/// points and units anchored when crossing 9.9 / 10.0 or 99.9 / 100.0.
fn score_readout(value: LocalizedText, reservation: LocalizedText, color: Color) -> AnyPiece {
    let spoken = value.clone();
    canvas(move |d, size| {
        let text = value.format();
        let sample = reservation.format();
        let font = chrome::canvas_font(FontWeight::Heavy);
        let digits = display_chars(&text);
        let template = display_chars(&sample);
        if digits.iter().any(|c| c.is_alphabetic()) {
            let fit = chrome::fit_text(&sample, 30.0, &font, size.width).min(30.0);
            d.text(
                &text,
                Point::new(size.width / 2.0, size.height / 2.0),
                TextStyle {
                    size: fit,
                    color: color.into(),
                    anchor: TextAnchor::CENTERED,
                    font,
                },
            );
        } else {
            let digit_width = template
                .iter()
                .filter(|c| c.is_numeric())
                .map(|c| day_core::measure_text(&c.to_string(), 30.0, &font).width)
                .fold(1.0_f64, f64::max);
            let advance = |c: &char| {
                if c.is_numeric() {
                    digit_width
                } else {
                    day_core::measure_text(&c.to_string(), 30.0, &font).width
                }
            };
            let reserved = template.iter().map(advance).sum::<f64>().max(1.0);
            let scale = (size.width / reserved).min(1.0);
            let mut right = (size.width + reserved * scale) / 2.0;
            for c in digits.iter().rev() {
                let step = advance(c) * scale;
                d.text(
                    &c.to_string(),
                    Point::new(right - step / 2.0, size.height / 2.0),
                    TextStyle {
                        size: 30.0 * scale,
                        color: color.into(),
                        anchor: TextAnchor::CENTERED,
                        font: font.clone(),
                    },
                );
                right -= step;
            }
        }
    })
    .height(45.0)
    .a11y(move |a| a.label(spoken))
    .any()
}
fn display_chars(text: &str) -> Vec<char> {
    text.chars()
        .filter(|c| !c.is_control() && !matches!(c, '\u{2068}' | '\u{2069}'))
        .collect()
}
/// Stronger physical impacts get stronger pulses, without a vibration on every physics step.
pub fn impact_pattern(speed: f64) -> chrome::Pattern {
    if speed >= 240.0 {
        chrome::cues::HEAVY_BEAT
    } else if speed >= 80.0 {
        chrome::cues::MEDIUM_BEAT
    } else {
        chrome::cues::LIGHT_BEAT
    }
}

impl Vec2 {
    pub fn length(self) -> f64 {
        self.x.hypot(self.y)
    }
    pub fn normalized(self) -> Self {
        let l = self.length();
        if l > 1e-9 { self / l } else { Self::default() }
    }
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }
}
impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y)
    }
}
impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.x - b.x, self.y - b.y)
    }
}
impl std::ops::Mul<f64> for Vec2 {
    type Output = Self;
    fn mul(self, b: f64) -> Self {
        Self::new(self.x * b, self.y * b)
    }
}
impl std::ops::Div<f64> for Vec2 {
    type Output = Self;
    fn div(self, b: f64) -> Self {
        Self::new(self.x / b, self.y / b)
    }
}
