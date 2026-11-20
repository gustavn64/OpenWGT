//! The watch list (Debug ▸ Watch List): the probes (with how often their
//! value changed), the values watched as they are now, and the
//! breakpoints (with how often they held the run), each with a box to
//! take it away.

use formats::fnt::Font;

use super::super::{Editor, ui};
use super::probes::Observed;
use super::window::{self, Colours};
use crate::gfx::{IRect, Surface};
use crate::lang::{tr, trf};

/// What a row of the list is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Heading(&'static str),
    Probe(usize),
    Watch(usize),
    Break(usize),
}

/// The rows in the window at `r`: where, the remove box, what.
#[must_use]
pub fn rows(r: IRect, font: &Font, o: &Observed) -> Vec<(IRect, IRect, Item)> {
    let c = window::client(r).inset(4);
    let h = i32::from(font.height) + 3;
    let mut items = Vec::new();
    let mut section = |heading: &'static str, n: usize, f: fn(usize) -> Item| {
        if n > 0 {
            items.push(Item::Heading(heading));
            items.extend((0..n).map(f));
        }
    };
    section("Probes", o.probes.len(), Item::Probe);
    section("Watches", o.watches.len(), Item::Watch);
    section("Breakpoints", o.breaks.len(), Item::Break);
    let mut y = c.y0;
    items
        .into_iter()
        .map(|it| {
            let row = IRect::sized(c.x0, y, c.width(), h);
            y += h;
            let x = IRect::sized(row.x1 - h + 2, row.y0 + 1, h - 3, h - 3);
            (row, x, it)
        })
        .collect()
}

/// A row's text.
fn text(o: &Observed, it: Item) -> String {
    match it {
        Item::Heading(h) => tr(h).to_owned(),
        Item::Probe(i) => {
            let p = &o.probes[i];
            let n = p.changes.len().to_string();
            format!("{}  ({})", p.name, trf("{} changes", &[&n]))
        }
        Item::Watch(i) => {
            let w = &o.watches[i];
            format!("{} = {}", w.name, w.value)
        }
        Item::Break(i) => {
            let b = &o.breaks[i];
            let n = b.hits.to_string();
            format!("{} {}  ({})", tr("when"), b.text, trf("held {}", &[&n]))
        }
    }
}

/// Draws the list in its window `r`.
pub fn draw(s: &mut Surface, font: &Font, r: IRect, col: Colours, o: &Observed) {
    let c = window::client(r).inset(2);
    ui::sunken(s, c);
    s.fill(c.inset(2), ui::WHITE);
    let rows = rows(r, font, o);
    if rows.is_empty() {
        let lines = ui::wrap(
            font,
            tr("Right-click a wire, a knot or a part to watch it, or to hold the run when it changes."),
            c.width() - 12,
        );
        let mut y = c.y0 + 4;
        for line in lines {
            ui::label(s, font, c.x0 + 6, y, &line, col.grey);
            y += i32::from(font.height);
        }
        return;
    }
    for (row, x, it) in rows {
        let colour = match it {
            Item::Heading(_) => col.grey,
            Item::Probe(i) => col.series(o.probes[i].colour),
            Item::Watch(_) => ui::BLACK,
            Item::Break(_) => col.red,
        };
        let indent = if matches!(it, Item::Heading(_)) { 0 } else { 8 };
        let old = s.set_clip(IRect::new(row.x0, row.y0, x.x0 - 2, row.y1).intersect(s.clip()));
        ui::label(s, font, row.x0 + 2 + indent, row.y0 + 1, &text(o, it), colour);
        s.set_clip(old);
        if !matches!(it, Item::Heading(_)) {
            s.fill(x, ui::FACE);
            ui::edge(s, x, ui::LIGHT, ui::DARK);
            s.line((x.x0 + 3, x.y0 + 3), (x.x1 - 4, x.y1 - 4), ui::BLACK);
            s.line((x.x0 + 3, x.y1 - 4), (x.x1 - 4, x.y0 + 3), ui::BLACK);
        }
    }
}

impl Editor {
    /// A click in the watch list `r` at `p`: a remove box takes its row
    /// away.
    pub(super) fn watch_click(&mut self, r: IRect, p: (i32, i32)) {
        let Some(font) = ui::font(&self.assets) else { return };
        let hit = rows(r, font, &self.debug.observed)
            .into_iter()
            .find(|(_, x, _)| x.contains(p.0, p.1))
            .map(|(_, _, it)| it);
        let o = &mut self.debug.observed;
        match hit {
            Some(Item::Probe(i)) => {
                o.probes.remove(i);
            }
            Some(Item::Watch(i)) => {
                o.watches.remove(i);
            }
            Some(Item::Break(i)) => {
                o.breaks.remove(i);
            }
            Some(Item::Heading(_)) | None => {}
        }
    }
}
