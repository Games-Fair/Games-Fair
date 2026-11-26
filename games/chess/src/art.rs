// SPDX-License-Identifier: GPL-2.0-or-later
use super::*;
use cozy_chess::{File, Rank};
use day_spec::TouchPhase;
use gamekit::sprites::{Ink, VectorSprite};
const LIGHT: Color = Color::hex(0xe3d6b9);
const DARK: Color = Color::hex(0x779181);
const INKS: [u32; 2] = [0xfff3d7, 0x273630];

#[derive(Clone, Copy)]
struct Grid {
    x: f64,
    y: f64,
    cell: f64,
    band: f64,
    reverse: bool,
}
impl Grid {
    fn new(s: Size, reverse: bool) -> Self {
        let band = (s.height * 0.09).clamp(20., 45.);
        let edge = s.width.min((s.height - 2. * band).max(0.)).min(720.);
        Self {
            x: (s.width - edge) / 2.,
            y: (s.height - edge) / 2.,
            cell: edge / 8.,
            band,
            reverse,
        }
    }
    fn center(self, s: Square) -> Point {
        let (mut col, mut row) = (s.file() as usize, 7 - s.rank() as usize);
        if self.reverse {
            col = 7 - col;
            row = 7 - row;
        }
        Point::new(
            self.x + (col as f64 + 0.5) * self.cell,
            self.y + (row as f64 + 0.5) * self.cell,
        )
    }
    fn square(self, p: Point) -> Option<Square> {
        let (c, r) = (
            ((p.x - self.x) / self.cell).floor(),
            ((p.y - self.y) / self.cell).floor(),
        );
        if !(0.0..8.).contains(&c) || !(0.0..8.).contains(&r) {
            return None;
        }
        let (f, r) = if self.reverse {
            (7 - c as usize, r as usize)
        } else {
            (c as usize, 7 - r as usize)
        };
        Some(Square::new(File::index(f), Rank::index(r)))
    }
}
fn rect(d: &mut Draw, x: f64, y: f64, w: f64, h: f64, c: Color) {
    d.fill(Shape::Rect(Rect::new(x, y, w, h)), c);
}
fn circle(p: Point, r: f64) -> Shape {
    Shape::Ellipse(Rect::new(p.x - r, p.y - r, 2. * r, 2. * r))
}
fn text(d: &mut Draw, t: &str, x: f64, y: f64, size: f64, c: Color) {
    d.text(
        t,
        Point::new(x, y),
        TextStyle {
            size,
            color: c.into(),
            anchor: TextAnchor::CENTERED,
            font: chrome::canvas_font(FontWeight::Semibold),
        },
    );
}
fn bands(d: &mut Draw, grid: Grid, g: &Game, images: &[[Option<day_core::Bitmap>; 6]; 2]) {
    let edge = grid.cell * 8.;
    for s in Side::ALL {
        let below = (s == Side::White) != grid.reverse;
        let y = if below {
            grid.y + edge
        } else {
            grid.y - grid.band
        };
        let mid = y + grid.band / 2.;
        let active = s == g.board.side_to_move() && g.saved.end.is_none();
        rect(
            d,
            grid.x,
            y,
            edge,
            grid.band,
            if active {
                Color::hex(0x2b453c)
            } else {
                Color::hex(0x1e342d)
            },
        );
        if active {
            rect(
                d,
                grid.x,
                if below { y } else { y + grid.band - 2. },
                edge,
                2.,
                BRASS,
            );
        }
        let mark = Point::new(grid.x + edge * 0.065, mid);
        let r = (grid.band * 0.15).min(7.);
        d.fill(circle(mark, r), Color::hex(INKS[s as usize]));
        d.stroke(circle(mark, r), BRASS, 1.1);
        let name = if g.saved.players == 1 && s == Side::Black {
            res::str::party_computer()
        } else {
            side(s)
        };
        text(
            d,
            &name.format(),
            grid.x + edge * 0.26,
            mid,
            grid.band * 0.34,
            Color::hex(0xf3e9d4),
        );
        let tray = &g.taken[s as usize];
        let w = grid.band * 0.50;
        let room = edge
            * if g.saved.pace == Pace::Untimed {
                0.55
            } else {
                0.27
            };
        let spacing = ((room - w) / (tray.len().saturating_sub(1).max(1) as f64)).min(w * 0.62);
        if !tray.is_empty() {
            let width = w + spacing * (tray.len() - 1) as f64;
            d.fill(
                Shape::RoundedRect(
                    Rect::new(
                        grid.x + edge * 0.44 - 2.,
                        mid - w / 2. - 1.,
                        width + 4.,
                        w + 2.,
                    ),
                    w * 0.3,
                ),
                if s == Side::White {
                    Color::hex(0xb6bc9f)
                } else {
                    SURFACE
                },
            );
        }
        for (i, p) in tray.iter().enumerate() {
            if let Some(img) = &images[(!s) as usize][*p as usize] {
                d.image(
                    img,
                    Rect::new(
                        grid.x + edge * 0.44 + i as f64 * spacing,
                        mid - w / 2.,
                        w,
                        w,
                    ),
                );
            }
        }
        if g.saved.pace != Pace::Untimed {
            let sec = g.saved.remaining[s as usize].ceil() as u32;
            let time = res::str::chess_clock(res::str::ChessClock {
                minutes: (sec / 60).to_string(),
                digits: format!("{:02}", sec % 60),
            })
            .format();
            let w = edge * 0.24;
            let h = grid.band * 0.76;
            let cx = grid.x + edge * 0.85;
            d.fill(
                Shape::RoundedRect(Rect::new(cx - w / 2., mid - h / 2., w, h), h * 0.24),
                if active { BRASS } else { Color::hex(0x344b41) },
            );
            text(
                d,
                &time,
                cx,
                mid,
                h * 0.58,
                if active { SURFACE } else { LIGHT },
            );
        }
    }
}
pub(super) fn board(ui: Rc<Ui>) -> AnyPiece {
    let assets = [
        res::assets::chess::pawn_svg,
        res::assets::chess::knight_svg,
        res::assets::chess::bishop_svg,
        res::assets::chess::rook_svg,
        res::assets::chess::queen_svg,
        res::assets::chess::king_svg,
    ];
    let sprites = INKS.map(|ink| assets.clone().map(|a| VectorSprite::new(a, Ink::Team(ink))));
    let u = ui.clone();
    let t = ui.clone();
    let k = ui.clone();
    let a = ui.clone();
    canvas(move |d, size| {
        u.repaint.track();
        u.size.set(size);
        let grid = Grid::new(size, u.reversed());
        let c = grid.cell;
        if c < 1. {
            return;
        }
        let images = sprites
            .each_ref()
            .map(|row| row.each_ref().map(|s| s.at_width(c)));
        let g = u.game.borrow();
        bands(d, grid, &g, &images);
        let checked =
            (!g.board.checkers().is_empty()).then(|| g.board.king(g.board.side_to_move()));
        let options = u.options.borrow();
        let motion = u.motion.get();
        let drag = u.drag.get();
        let face_to_face = g.saved.players == 2 && !u.flip.get();
        let piece_image = |d: &mut Draw, side: Side, kind: Man, p: Point, scale: f64| {
            if let Some(img) = &images[side as usize][kind as usize] {
                let w = c * scale;
                let bounds = Rect::new(p.x - w / 2., p.y - w / 2., w, w);
                if face_to_face && side == Side::Black {
                    let turn = Affine::translate(-p.x, -p.y)
                        .then(Affine::rotate(Angle::deg(180.)))
                        .then(Affine::translate(p.x, p.y));
                    d.transformed(turn, |d| d.image(img, bounds));
                } else {
                    d.image(img, bounds);
                }
            }
        };
        let draw_piece = |d: &mut Draw, s: Square, p: Point, scale: f64| {
            if let (Some(kind), Some(side)) = (g.board.piece_on(s), g.board.color_on(s)) {
                piece_image(d, side, kind, p, scale);
            }
        };
        for sq in Square::ALL {
            let p = grid.center(sq);
            let (x, y) = (p.x - c / 2., p.y - c / 2.);
            let dark = (sq.file() as usize + sq.rank() as usize).is_multiple_of(2);
            let fill = if Some(sq) == checked {
                Color::hex(0xd38c70)
            } else if Some(sq) == u.selected.get() {
                Color::hex(0xd2b572)
            } else if g.last.is_some_and(|(a, b)| a == sq || b == sq) {
                if dark {
                    Color::hex(0xa3a478)
                } else {
                    Color::hex(0xe8d7a4)
                }
            } else if dark {
                DARK
            } else {
                LIGHT
            };
            rect(d, x, y, c, c, fill);
            if p.y > grid.y + 7. * c {
                text(
                    d,
                    &sq.file().to_string(),
                    x + c * 0.89,
                    y + c * 0.88,
                    (c * 0.18).min(12.),
                    SURFACE,
                );
            }
            if p.x < grid.x + c {
                text(
                    d,
                    &sq.rank().to_string(),
                    x + c * 0.10,
                    y + c * 0.13,
                    (c * 0.18).min(12.),
                    SURFACE,
                );
            }
            if options.iter().any(|&m| destination(&g.board, m) == sq) {
                if g.board.piece_on(sq).is_some() {
                    d.stroke(circle(p, c * 0.445), JADE, c * 0.064);
                } else {
                    d.fill(circle(p, c * 0.105), JADE);
                }
            }
            if u.keyboard.get() && u.cursor.get() == sq {
                d.stroke(
                    Shape::Rect(Rect::new(x + 2., y + 2., c - 4., c - 4.)),
                    SURFACE,
                    2.,
                );
            }
        }
        for sq in g.board.occupied() {
            if motion.is_some_and(|m| m.to == sq || m.rook.is_some_and(|(_, to)| to == sq))
                || (drag.is_some() && u.selected.get() == Some(sq))
            {
                continue;
            }
            draw_piece(d, sq, grid.center(sq), 0.93);
        }
        if let Some(m) = motion {
            let t = ((now() - m.start) / 0.18).clamp(0., 1.);
            let ease = t * t * (3. - 2. * t);
            if let Some((piece, sq)) = m.taken {
                piece_image(
                    d,
                    g.board.side_to_move(),
                    piece,
                    grid.center(sq),
                    0.93 * (1. - t.powi(3)),
                );
            }
            for (from, to) in [Some((m.from, m.to)), m.rook].into_iter().flatten() {
                let (a, b) = (grid.center(from), grid.center(to));
                piece_image(
                    d,
                    !g.board.side_to_move(),
                    if to == m.to { m.moving } else { Man::Rook },
                    Point::new(a.x + (b.x - a.x) * ease, a.y + (b.y - a.y) * ease),
                    0.93,
                );
            }
        }
        if let (Some(p), Some(sq)) = (drag, u.selected.get()) {
            draw_piece(d, sq, p, 1.06);
        }
    })
    .on_touches(move |events| {
        if t.overlay.get_untracked() != Overlay::Play
            || t.computer()
            || (t.flip.get_untracked() && t.motion.get().is_some())
        {
            return;
        }
        let grid = Grid::new(t.size.get(), t.reversed());
        for e in events {
            let sq = grid.square(e.location);
            if e.phase == TouchPhase::Began
                && t.contact.get().is_none()
                && let Some(s) = sq
            {
                t.keyboard.set(false);
                t.contact.set(Some((e.id, s, e.location)));
                continue;
            }
            let Some((id, start, origin)) = t.contact.get() else {
                continue;
            };
            if id != e.id {
                continue;
            }
            match e.phase {
                TouchPhase::Moved
                    if (origin.x - e.location.x).hypot(origin.y - e.location.y) > 4. =>
                {
                    let own = {
                        let g = t.game.borrow();
                        g.board.color_on(start) == Some(g.board.side_to_move())
                    };
                    if own {
                        if t.selected.get() != Some(start) {
                            t.select(Some(start));
                        }
                        t.drag.set(Some(e.location));
                        t.repaint.notify();
                    }
                }
                TouchPhase::Ended => {
                    t.contact.set(None);
                    let dragged = t.drag.take().is_some();
                    if let Some(s) = sq {
                        if start != s && !dragged {
                            let own = {
                                let g = t.game.borrow();
                                g.board.color_on(start) == Some(g.board.side_to_move())
                            };
                            if own {
                                t.select(Some(start));
                            }
                        }
                        if !dragged || s != start {
                            t.tap(s);
                        }
                    }
                    t.repaint.notify();
                }
                TouchPhase::Cancelled => {
                    t.contact.set(None);
                    t.drag.set(None);
                    t.repaint.notify();
                }
                _ => {}
            }
        }
    })
    .on_key(move |e| k.key(e.key))
    .focused(ui.focus)
    .a11y(move |v| {
        v.label(res::str::party_board())
            .value(move || a.selection())
    })
    .id(id!(chess_board))
    .grow(Dimension::Both)
    .any()
}
pub fn preview() -> AnyPiece {
    let art = VectorSprite::new(res::assets::chess::knight_svg, Ink::Team(INKS[0]));
    canvas(move |d, s| {
        let c = (s.width / 6.).max(s.height / 3.).max(1.);
        for row in 0..3 {
            for col in 0..6 {
                rect(
                    d,
                    col as f64 * c,
                    row as f64 * c,
                    c,
                    c,
                    if (row + col) % 2 == 0 {
                        Color::hex(0x355248)
                    } else {
                        Color::hex(0x44675a)
                    },
                );
            }
        }
        let w = s.height.min(s.width) * 0.69;
        if let Some(img) = art.at_width(w) {
            d.image(
                &img,
                Rect::new((s.width - w) / 2., (s.height - w) / 2., w, w),
            );
        }
    })
    .grow(Dimension::Both)
    .any()
}
