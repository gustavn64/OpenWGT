//! The debugging tools' own windows (the console, the graph, the watch
//! list, the timeline): modeless like the help window, over the game's and
//! under its menus, each with a caption to drag it by, a close box and a
//! corner to size it. Dragged whole, as the enhancements drag windows.

use formats::fnt::Font;

use super::super::ui;
use crate::gfx::{IRect, Surface};

/// The frame's width round the caption and the inside.
pub const BORDER: i32 = 3;
/// The caption's height.
pub const CAPTION_H: i32 = 18;
/// The sizing corner's size.
const GRIP: i32 = 12;
/// The smallest a window gets.
pub const MIN_W: i32 = 160;
pub const MIN_H: i32 = 90;

/// Which window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Console,
    Graph,
    Watch,
    Timeline,
    /// The script editor (`edit`).
    Editor,
}

/// A window: what it is and where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Win {
    pub kind: Kind,
    pub rect: IRect,
}

/// The part of a window at a point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Close,
    Caption,
    Grip,
    Client,
}

/// A drag that began on a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drag {
    /// By its caption, `grab` from its top left.
    Move { kind: Kind, grab: (i32, i32) },
    /// By its corner, `grab` from its bottom right.
    Size { kind: Kind, grab: (i32, i32) },
    /// Inside it (a slider, a scroll), for its own handling.
    Client { kind: Kind },
}

#[must_use]
pub fn caption(r: IRect) -> IRect {
    IRect::new(r.x0 + BORDER, r.y0 + BORDER, r.x1 - BORDER, r.y0 + BORDER + CAPTION_H)
}

#[must_use]
pub fn close_box(r: IRect) -> IRect {
    let c = caption(r);
    IRect::sized(c.x1 - CAPTION_H + 2, c.y0 + 2, CAPTION_H - 4, CAPTION_H - 4)
}

/// What's inside the frame and under the caption.
#[must_use]
pub fn client(r: IRect) -> IRect {
    IRect::new(r.x0 + BORDER, r.y0 + BORDER + CAPTION_H, r.x1 - BORDER, r.y1 - BORDER)
}

fn grip(r: IRect) -> IRect {
    IRect::new(r.x1 - GRIP, r.y1 - GRIP, r.x1, r.y1)
}

/// What of the window at `r` is at `p`.
#[must_use]
pub fn hit(r: IRect, p: (i32, i32)) -> Option<Hit> {
    if !r.contains(p.0, p.1) {
        None
    } else if close_box(r).contains(p.0, p.1) {
        Some(Hit::Close)
    } else if caption(r).contains(p.0, p.1) {
        Some(Hit::Caption)
    } else if grip(r).contains(p.0, p.1) {
        Some(Hit::Grip)
    } else {
        Some(Hit::Client)
    }
}

/// `r` moved to stay in `area` with its caption below `top`, and no
/// smaller than the least.
#[must_use]
pub fn keep_in(r: IRect, area: IRect, top: i32) -> IRect {
    let w = r.width().clamp(MIN_W.min(area.width()), area.width().max(MIN_W));
    let h = r.height().clamp(MIN_H.min(area.height()), (area.y1 - top).max(MIN_H));
    let x = r.x0.clamp(area.x0, (area.x1 - w).max(area.x0));
    let y = r.y0.clamp(top, (area.y1 - h).max(top));
    IRect::sized(x, y, w, h)
}

/// The frame, the caption with `title` (blue while `active`), the close
/// box and the sizing corner; the inside is the window's to fill.
pub fn draw(s: &mut Surface, font: &Font, r: IRect, title: &str, active: bool) {
    s.fill(r, ui::FACE);
    ui::raised(s, r);
    let c = caption(r);
    ui::caption(s, c, active);
    let lh = i32::from(font.height);
    let old = s.set_clip(c.intersect(s.clip()));
    let fg = if active { ui::WHITE } else { ui::LIGHT };
    ui::label(s, font, c.x0 + 4, c.y0 + (c.height() - lh) / 2, title, fg);
    s.set_clip(old);
    // The close box: a small raised square with an ×.
    let b = close_box(r);
    s.fill(b, ui::FACE);
    ui::edge(s, b, ui::LIGHT, ui::DARK);
    for d in 0..2 {
        s.line((b.x0 + 3 + d, b.y0 + 3), (b.x1 - 4 + d - 1, b.y1 - 4), ui::BLACK);
        s.line((b.x0 + 3 + d, b.y1 - 4), (b.x1 - 4 + d - 1, b.y0 + 3), ui::BLACK);
    }
    // The sizing corner: three diagonal ridges.
    let g = grip(r);
    for k in [3, 6, 9] {
        s.line((g.x1 - 2 - k, g.y1 - 2), (g.x1 - 2, g.y1 - 2 - k), ui::LIGHT);
        s.line((g.x1 - 1 - k, g.y1 - 2), (g.x1 - 2, g.y1 - 1 - k), ui::SHADOW);
    }
}

/// The palette index nearest to `rgb`, for the windows' colours.
#[must_use]
pub fn nearest(palette: &[[u8; 3]; 256], rgb: [u8; 3]) -> u8 {
    let d = |c: &[u8; 3]| -> i32 {
        (0..3)
            .map(|i| {
                let e = i32::from(c[i]) - i32::from(rgb[i]);
                e * e
            })
            .sum()
    };
    let best = (0..256).min_by_key(|&i| d(&palette[i])).unwrap_or(0);
    u8::try_from(best).unwrap_or(0)
}

/// The colours the windows use, from the game's palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colours {
    pub red: u8,
    pub green: u8,
    pub orange: u8,
    pub blue: u8,
    pub purple: u8,
    pub teal: u8,
    pub maroon: u8,
    pub grey: u8,
    pub light_grey: u8,
    /// Selected text's background.
    pub selection: u8,
}

impl Colours {
    #[must_use]
    pub fn from(palette: &[[u8; 3]; 256]) -> Colours {
        let n = |rgb| nearest(palette, rgb);
        Colours {
            red: n([200, 0, 0]),
            green: n([0, 128, 0]),
            orange: n([200, 110, 0]),
            blue: n([0, 60, 200]),
            purple: n([140, 0, 160]),
            teal: n([0, 130, 130]),
            maroon: n([150, 20, 20]),
            grey: n([128, 128, 128]),
            light_grey: n([220, 220, 220]),
            selection: n([180, 205, 255]),
        }
    }

    /// Distinct colours for series, in turn.
    #[must_use]
    pub fn series(self, n: usize) -> u8 {
        let all = [self.blue, self.red, self.green, self.orange, self.purple, self.teal];
        all[n % all.len()]
    }
}
