//! The tools' popup menu: a right-click on a wire, a knot or a part
//! (while editing or held; Ctrl+right-click while the widget runs) offers
//! Show Graph, Add Probe, Watch, Break When Changes, Name This Part... and
//! Copy Reference for it.

use formats::fnt::Font;

use super::super::{Editor, ui};
use super::overlay::Target;
use super::window::Kind;
use crate::gfx::{IRect, Surface};
use crate::lang::tr;
use crate::script::{Cond, Host, KnotSel, PartSel, Selector, select};

const VK_RETURN: u16 = 0x0d;
const VK_ESCAPE: u16 = 0x1b;
const VK_UP: u16 = 0x26;
const VK_DOWN: u16 = 0x28;

/// What an item does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Graph,
    Probe,
    Watch,
    Break,
    Name,
    Copy,
}

impl Action {
    const ALL: [Action; 6] = [
        Action::Graph,
        Action::Probe,
        Action::Watch,
        Action::Break,
        Action::Name,
        Action::Copy,
    ];

    fn text(self) -> &'static str {
        tr(match self {
            Action::Graph => "Show &Graph",
            Action::Probe => "Add &Probe",
            Action::Watch => "&Watch",
            Action::Break => "&Break When Changes",
            Action::Name => "&Name This Part...",
            Action::Copy => "&Copy Reference",
        })
    }
}

/// The popup: where, for what, the item under the pointer.
#[derive(Debug, Clone, PartialEq)]
pub struct Popup {
    pub rect: IRect,
    /// What it's for (by id), and the part's id.
    pub sel: Selector,
    pub part: u32,
    /// Its heading: what it's for.
    pub title: String,
    pub hover: Option<usize>,
}

fn item_h(font: &Font) -> i32 {
    i32::from(font.height) + 5
}

/// Item `i`'s rectangle in `p`.
fn item(p: &Popup, font: &Font, i: usize) -> IRect {
    let h = item_h(font);
    let n = i32::try_from(i).unwrap_or(0) + 1;
    IRect::new(
        p.rect.x0 + 2,
        p.rect.y0 + 2 + n * h,
        p.rect.x1 - 2,
        p.rect.y0 + 2 + (n + 1) * h,
    )
}

/// Draws the popup.
pub fn draw(s: &mut Surface, font: &Font, p: &Popup) {
    s.fill(p.rect, ui::FACE);
    ui::raised(s, p.rect);
    let h = item_h(font);
    let head = IRect::new(p.rect.x0 + 2, p.rect.y0 + 2, p.rect.x1 - 2, p.rect.y0 + 2 + h);
    ui::caption(s, head, true);
    let old = s.set_clip(head.intersect(s.clip()));
    ui::label(s, font, head.x0 + 4, head.y0 + 2, &p.title, ui::WHITE);
    s.set_clip(old);
    for (i, a) in Action::ALL.iter().enumerate() {
        let r = item(p, font, i);
        let c = if p.hover == Some(i) {
            s.fill(r, ui::BLUE);
            ui::WHITE
        } else {
            ui::BLACK
        };
        ui::label(s, font, r.x0 + 8, r.y0 + 2, a.text(), c);
    }
}

impl Editor {
    /// What `t` stands for: a knot (a wire by the end that gives its
    /// value), else the part; and the part's id.
    fn target_sel(&self, t: Target) -> Option<(Selector, u32)> {
        let (g, k) = match t {
            Target::Part(g) => (g, None),
            Target::Knot(g, k) => (g, Some(k)),
            Target::Wire(w) => {
                let wire = self.doc.wires.get(w)?;
                let backwards = self.doc.gadgets[wire.from]
                    .core
                    .knots
                    .get(wire.from_knot)
                    .is_some_and(crate::gadget::Knot::is_input);
                if backwards {
                    (wire.to, Some(wire.to_knot))
                } else {
                    (wire.from, Some(wire.from_knot))
                }
            }
        };
        let id = self.doc.gadgets.get(g)?.core.id;
        let knot = k.and_then(|k| u8::try_from(k).ok()).map(KnotSel::Index);
        Some((
            Selector {
                part: PartSel::Id(id),
                knot,
            },
            id,
        ))
    }

    /// The popup for what's under `p` (frame coordinates); whether there's
    /// something there.
    pub(super) fn open_popup(&mut self, p: (i32, i32)) -> bool {
        let Some(font) = ui::font(&self.assets) else {
            return false;
        };
        let Some(t) = self.debug_target(Self::to_view(p)) else {
            return false;
        };
        let Some((sel, part)) = self.target_sel(t) else {
            return false;
        };
        let title = self.sel_name(&sel);
        let h = item_h(font);
        let w = Action::ALL
            .iter()
            .map(|a| ui::width(font, a.text()))
            .chain([ui::width(font, &title)])
            .max()
            .unwrap_or(0)
            + 24;
        let n = i32::try_from(Action::ALL.len()).unwrap_or(0) + 1;
        let area = self.surface.bounds();
        let r = IRect::sized(p.0, p.1, w, n * h + 4);
        let x = r.x0.min(area.x1 - r.width()).max(0);
        let y = if r.y1 > area.y1 {
            (p.1 - r.height()).max(0)
        } else {
            r.y0
        };
        self.debug.popup = Some(Popup {
            rect: IRect::sized(x, y, r.width(), r.height()),
            sel,
            part,
            title,
            hover: None,
        });
        true
    }

    /// The pointer over the popup: the item under it lit.
    pub(super) fn popup_hover(&mut self, p: (i32, i32)) {
        let Some(font) = ui::font(&self.assets) else { return };
        if let Some(pop) = &self.debug.popup {
            let hover = (0..Action::ALL.len()).find(|&i| item(pop, font, i).contains(p.0, p.1));
            if let Some(pop) = &mut self.debug.popup {
                pop.hover = hover;
            }
        }
    }

    /// A click with the popup up: its item done, or (elsewhere) it goes.
    pub(super) fn popup_click(&mut self, p: (i32, i32)) {
        self.popup_hover(p);
        let Some(pop) = self.debug.popup.take() else { return };
        if pop.rect.contains(p.0, p.1)
            && let Some(i) = pop.hover
        {
            self.popup_do(&pop, Action::ALL[i]);
        }
    }

    /// The popup's keys: Up, Down, Enter, Esc (and the rest taken).
    pub(super) fn popup_key(&mut self, vk: u16) {
        let n = Action::ALL.len();
        let Some(pop) = &mut self.debug.popup else { return };
        match vk {
            VK_DOWN => pop.hover = Some(pop.hover.map_or(0, |i| (i + 1) % n)),
            VK_UP => pop.hover = Some(pop.hover.map_or(n - 1, |i| (i + n - 1) % n)),
            VK_ESCAPE => self.debug.popup = None,
            VK_RETURN => {
                if let Some(pop) = self.debug.popup.take()
                    && let Some(i) = pop.hover
                {
                    self.popup_do(&pop, Action::ALL[i]);
                }
            }
            _ => {}
        }
    }

    fn popup_do(&mut self, pop: &Popup, a: Action) {
        let r = match a {
            Action::Graph => self.add_probe(&pop.sel, true),
            Action::Probe => self.add_probe(&pop.sel, false),
            Action::Watch => self.add_watch(&pop.sel),
            Action::Break => self.add_break(&Cond::Changed(pop.sel.clone())),
            Action::Name => {
                // The console, with the command to finish.
                self.open_window(Kind::Console);
                let c = &mut self.debug.console;
                c.input = format!("name #{} ", pop.part);
                c.caret = c.input.len();
                Ok(())
            }
            Action::Copy => {
                let text = self.reference(&pop.sel);
                if let Some(l) = &mut self.links {
                    let _ = (l.copy)(&text);
                }
                self.debug.console.add(super::LineKind::Info, &text);
                Ok(())
            }
        };
        if let Err(e) = r {
            self.debug.console.add(super::LineKind::Error, &e.message);
        }
    }

    /// How a script would say `sel`: by the part's name if it has one,
    /// else its id; the knot as `in2`, `out`.
    pub(super) fn reference(&self, sel: &Selector) -> String {
        let PartSel::Id(id) = sel.part else {
            return select::show(sel);
        };
        let part = match self.part_name(id) {
            Some(n) => PartSel::Text {
                text: n.to_owned(),
                nth: None,
            },
            None => PartSel::Id(id),
        };
        let knot = sel.knot.map(|k| match (k, self.part(crate::script::PartId(id))) {
            (KnotSel::Index(n), Ok(p)) => p.knots.get(usize::from(n)).map_or(k, |kn| {
                let nth = p.knots[..usize::from(n)].iter().filter(|o| o.input == kn.input).count();
                let nth = u8::try_from(nth).unwrap_or(0);
                if kn.input { KnotSel::In(nth) } else { KnotSel::Out(nth) }
            }),
            _ => k,
        });
        select::show(&Selector { part, knot })
    }
}
