//! What the debugging tools draw over the worksheet: the note of values
//! under a pointer held still (a wire's, a knot's, a part's outputs), and
//! the run's status (`tick 1234 · paused · ¼×`).

use super::super::{Editor, ui, worksheet};
use super::Debugger;
use formats::fnt::Font;

use crate::assets::{Assets, Module};
use crate::gadgets::format_number;
use crate::gfx::{IRect, Pictures, Surface};
use crate::lang::{tr, trf};
use crate::script::{PartInfo, Val};

/// How long the pointer stays still before the note shows, in ms.
pub const HOVER_MS: u64 = 400;
/// A knot is under the pointer within this squared distance (nearer than
/// the editor's own, so a wire next to it can be pointed at).
const KNOT_D2: i32 = 37;
/// The note's padding, and its distance from the pointer.
const PAD: i32 = 3;
const BELOW: (i32, i32) = (12, 20);
/// A picture's thumbnail size.
const THUMB: i32 = 32;

/// What the pointer is over: a part, a knot of one, a wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Part(usize),
    Knot(usize, usize),
    Wire(usize),
}

/// The note: its lines, a picture to show (a sheet), and the pointer it
/// is for (worksheet coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct Tip {
    pub lines: Vec<String>,
    pub picture: Option<u32>,
    pub at: (i32, i32),
}

/// The pointer's stillness and the note it has.
#[derive(Debug, Clone, Default)]
pub struct Hover {
    /// Where the pointer was last seen (frame coordinates), and since when.
    mouse: (i32, i32),
    since: u64,
    pub tip: Option<Tip>,
    /// The status line (with the tools, while the widget runs).
    pub status: Option<String>,
    /// Where the probes are on the worksheet, and their colours' turns.
    pub markers: Vec<((i32, i32), usize)>,
}

/// `v` as the note shows it: on/off, a number as the displays show it,
/// `picture N`, `sound 1.3 s`, `—` for nothing.
#[must_use]
pub fn show(v: &Val) -> String {
    match v {
        Val::None => "—".into(),
        Val::Bool(b) => (if *b { "on" } else { "off" }).into(),
        Val::Number(n) => format_number(*n, &mut 0).trim().to_owned(),
        Val::Picture(p) => format!("picture {p}"),
        Val::Sound { rate, len } => {
            let secs = if *rate == 0 {
                0.0
            } else {
                f64::from(*len) / f64::from(*rate)
            };
            format!("sound {secs:.1} s")
        }
        Val::NoObject => "no object".into(),
    }
}

/// A part for the note: its name or label, else its kind and id.
pub(super) fn who(p: &PartInfo) -> String {
    match p.name.as_ref().or(p.label.as_ref()) {
        Some(n) => format!("\"{n}\""),
        None => format!("{} {}", p.kind_words(), p.id),
    }
}

/// Knot `k` of `p` as a selector ends: `in`, `in2`, `out`, `out3`.
pub(super) fn knot_name(p: &PartInfo, k: usize) -> String {
    let Some(knot) = p.knots.get(k) else {
        return "?".into();
    };
    let n = p.knots[..k].iter().filter(|o| o.input == knot.input).count();
    let what = if knot.input { "in" } else { "out" };
    if n == 0 {
        what.into()
    } else {
        format!("{what}{}", n + 1)
    }
}

impl Editor {
    /// What's under `p` (worksheet coordinates): a knot, a joint's or a
    /// wire's wire, a part; the topmost of each.
    pub(super) fn debug_target(&self, p: (i32, i32)) -> Option<Target> {
        let near = self.doc.gadgets.iter().enumerate().rev().find_map(|(g, gadget)| {
            worksheet::nearest_knot(&gadget.core, p)
                .filter(|&(_, d)| d < KNOT_D2)
                .map(|(k, _)| Target::Knot(g, k))
        });
        near.or_else(|| worksheet::joint_at(&self.doc, p).map(|(w, _)| Target::Wire(w)))
            .or_else(|| worksheet::wire_at(&self.doc, p).map(|(w, _)| Target::Wire(w)))
            .or_else(|| worksheet::gadget_at(&self.doc, p).map(Target::Part))
    }

    /// The note's lines for `t`, and the first picture in them.
    pub(super) fn tip_lines(&self, t: Target) -> (Vec<String>, Option<u32>) {
        let mut picture = None;
        let mut val = |v: &Val| {
            if let Val::Picture(n) = v {
                picture.get_or_insert(*n);
            }
            show(v)
        };
        let lines = match t {
            Target::Wire(w) => {
                let Some(wire) = self.doc.wires.get(w) else {
                    return (Vec::new(), None);
                };
                // The output's end, as `Io::source` reads a wire drawn
                // backwards.
                let backwards = self.doc.gadgets[wire.from]
                    .core
                    .knots
                    .get(wire.from_knot)
                    .is_some_and(crate::gadget::Knot::is_input);
                let (src, sk, dst, dk) = if backwards {
                    (wire.to, wire.to_knot, wire.from, wire.from_knot)
                } else {
                    (wire.from, wire.from_knot, wire.to, wire.to_knot)
                };
                let (a, b) = (self.info(src), self.info(dst));
                let mut value = val(&self.out_value(src, sk));
                if wire.changed {
                    value = format!("{value}  ({})", tr("changed, unread"));
                }
                vec![
                    format!("{}.{} -> {}.{}", who(&a), knot_name(&a, sk), who(&b), knot_name(&b, dk)),
                    value,
                ]
            }
            Target::Knot(g, k) => {
                let p = self.info(g);
                let v = p.knots.get(k).map_or(Val::None, |k| k.value.clone());
                vec![format!("{}.{}: {}", who(&p), knot_name(&p, k), val(&v))]
            }
            Target::Part(g) => {
                let p = self.info(g);
                let mut lines = vec![p.describe()];
                // Its outputs; for a part with none, what comes in.
                let outputs = p.knots.iter().any(|k| !k.input);
                for (k, knot) in p.knots.iter().enumerate() {
                    if knot.input != outputs {
                        lines.push(format!("{}: {}", knot_name(&p, k), val(&knot.value)));
                    }
                }
                lines
            }
        };
        (lines, picture)
    }

    /// The pointer moved to `(x, y)` (frame coordinates): the note waits
    /// for it to be still again. Whether the tools took the move.
    pub(in crate::editor) fn debug_mouse_move(&mut self, x: i32, y: i32) -> bool {
        let h = &mut self.debug.hover;
        if h.mouse != (x, y) {
            h.mouse = (x, y);
            h.since = self.now;
            h.tip = None;
        }
        if self.debug.popup.is_some() {
            self.popup_hover((x, y));
            return true;
        }
        self.window_drag_move(x, y)
    }

    /// Before drawing: the note under a pointer still for
    /// [`HOVER_MS`] over the worksheet (with nothing held or open), and the
    /// status while the widget runs; none without the tools.
    pub(in crate::editor) fn debug_prepare(&mut self) {
        self.windows_prepare();
        let h = &mut self.debug.hover;
        if h.mouse != self.mouse {
            h.mouse = self.mouse;
            h.since = self.now;
        }
        let still = self.now.saturating_sub(self.debug.hover.since) >= HOVER_MS;
        if !self.debugging() {
            self.debug.hover.tip = None;
            self.debug.hover.status = None;
            self.debug.hover.markers.clear();
            return;
        }
        let tip = (still && self.drag.is_none() && !self.menu.is_open() && self.over_view(self.mouse))
            .then(|| {
                let at = Self::to_view(self.mouse);
                let t = self.debug_target(at)?;
                let (lines, picture) = self.tip_lines(t);
                (!lines.is_empty()).then_some(Tip { lines, picture, at })
            })
            .flatten();
        self.debug.hover.tip = tip;
        self.debug.hover.markers = self.probe_markers();
        self.debug.hover.status = self.running.then(|| self.debug_status());
    }

    /// Where each probe's knot (or part) is.
    fn probe_markers(&self) -> Vec<((i32, i32), usize)> {
        let mut v = Vec::new();
        for p in &self.debug.observed.probes {
            let crate::script::PartSel::Id(id) = p.sel.part else {
                continue;
            };
            let Some(g) = self.doc.gadgets.iter().find(|g| g.core.id == id) else {
                continue;
            };
            let at = match p.sel.knot {
                Some(crate::script::KnotSel::Index(k)) => g.core.knot_at(usize::from(k)),
                _ => None,
            };
            let r = g.core.rect();
            v.push((at.unwrap_or((r.x1 - 3, r.y0 + 3)), p.colour));
        }
        v
    }

    /// `tick 1234 · paused · ¼×`.
    pub(super) fn debug_status(&self) -> String {
        let end = self.debug.timeline.end;
        let mut s = if self.env.ticks < end {
            trf("tick {} / {}", &[&self.env.ticks.to_string(), &end.to_string()])
        } else {
            trf("tick {}", &[&self.env.ticks.to_string()])
        };
        if self.debug.run.paused {
            s.push_str(" · ");
            s.push_str(tr("paused"));
        }
        if let Some(l) = self.debug.run.speed.label() {
            s.push_str(" · ");
            s.push_str(l);
        }
        s
    }
}

/// The note's rectangle on the worksheet `view` (its own coordinates):
/// below and right of the pointer, moved left and above it where the
/// worksheet ends, and kept inside it.
pub(super) fn tip_rect(tip: &Tip, font: &Font, thumb: bool, view: IRect) -> IRect {
    let lh = i32::from(font.height);
    let text_w = tip.lines.iter().map(|l| ui::width(font, l)).max().unwrap_or(0);
    let lines = i32::try_from(tip.lines.len()).unwrap_or(0);
    let thumb_w = if thumb { THUMB + PAD } else { 0 };
    let w = text_w + thumb_w + 2 * PAD + 4;
    let h = (lines * lh).max(if thumb { THUMB } else { 0 }) + 2 * PAD + 4;
    let mut x = tip.at.0 + BELOW.0;
    let mut y = tip.at.1 + BELOW.1;
    if x + w > view.x1 {
        x = view.x1 - w;
    }
    if y + h > view.y1 {
        y = tip.at.1 - h - 4;
    }
    IRect::sized(x.max(view.x0), y.max(view.y0), w, h)
}

/// The status's rectangle: in the worksheet's bottom right corner.
pub(super) fn status_rect(text: &str, font: &Font, view: IRect) -> IRect {
    let w = ui::width(font, text) + 2 * PAD + 4;
    let h = i32::from(font.height) + 2 * PAD + 4;
    IRect::sized(view.x1 - w - 4, view.y1 - h - 4, w, h)
}

impl Debugger {
    /// The note and the status, on the worksheet (`area`, in the frame;
    /// `s` drawing in its coordinates, clipped to it).
    pub fn draw(&self, s: &mut Surface, assets: &Assets, pictures: &Pictures, area: IRect) {
        let Some(font) = ui::font(assets) else { return };
        let view = IRect::new(0, 0, area.width(), area.height());
        let lh = i32::from(font.height);
        // The probes: a square in their colour on what they record.
        if let Some(col) = self.colours {
            for &((x, y), c) in &self.hover.markers {
                let r = IRect::sized(x - 4, y - 4, 9, 9);
                s.fill(r, ui::BLACK);
                s.fill(r.inset(1), col.series(c));
                s.fill(IRect::sized(x - 1, y - 1, 3, 3), ui::WHITE);
            }
        }
        if let Some(st) = &self.hover.status {
            let r = status_rect(st, font, view);
            s.fill(r, ui::FACE);
            ui::raised(s, r);
            ui::label(s, font, r.x0 + PAD + 2, r.y0 + PAD + 2, st, ui::BLACK);
        }
        let Some(tip) = &self.hover.tip else { return };
        let thumb = tip
            .picture
            .and_then(|n| pictures.frame(assets, Module::Gadget, n, 1))
            .filter(|i| i.width > 0 && i.height > 0);
        let r = tip_rect(tip, font, thumb.is_some(), view);
        s.fill(r, ui::FACE);
        ui::raised(s, r);
        let mut ty = r.y0 + PAD + 2;
        for l in &tip.lines {
            ui::label(s, font, r.x0 + PAD + 2, ty, l, ui::BLACK);
            ty += lh;
        }
        if let Some(img) = thumb {
            // Fitted in the square, its shape kept.
            let (iw, ih) = (i32::from(img.width), i32::from(img.height));
            let k = f64::from(THUMB) / f64::from(iw.max(ih));
            #[allow(clippy::cast_possible_truncation)]
            let (tw, th) = (
                ((f64::from(iw) * k).round() as i32).max(1),
                ((f64::from(ih) * k).round() as i32).max(1),
            );
            let x0 = r.x1 - PAD - 2 - THUMB + (THUMB - tw) / 2;
            let y0 = r.y0 + PAD + 2 + (THUMB - th) / 2;
            s.fill(
                IRect::sized(r.x1 - PAD - 2 - THUMB, r.y0 + PAD + 2, THUMB, THUMB),
                ui::WHITE,
            );
            s.blit_scaled(&img, IRect::sized(x0, y0, tw, th));
        }
    }

    /// The note shown now (for tests and the program).
    #[must_use]
    pub fn tip(&self) -> Option<&Tip> {
        self.hover.tip.as_ref()
    }

    /// The status shown now.
    #[must_use]
    pub fn status(&self) -> Option<&str> {
        self.hover.status.as_deref()
    }
}
