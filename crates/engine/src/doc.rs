//! A widget: its gadgets and wires (`CWidgetDoc`), read from and written
//! to saved widgets (`0x42503e` reads them), run a tick at a time
//! (`0x42498f`) and drawn.

use std::fmt;

use formats::wgt::{self, Chunk, Form};

use crate::assets::Assets;
use crate::gadget::{Core, DrawCx, Gadget, Settings, Tag, Unknown, knots};
use crate::gadgets;
use crate::sim::{Env, Io, Wire};

/// The size of a saved gadget's base part.
pub const BASE_SIZE: usize = 0x150;
/// The format version this writes (`fver`).
pub const VERSION: u16 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Format(wgt::Error),
    /// "This file is from a newer version" (versions above 4, or 0).
    Version(u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Format(e) => write!(f, "{e}"),
            Error::Version(v) => write!(f, "unsupported widget format version {v}"),
        }
    }
}

impl std::error::Error for Error {}

/// A widget.
#[derive(Default)]
pub struct Document {
    /// In list order: the order they're drawn and ticked in.
    pub gadgets: Vec<Gadget>,
    pub wires: Vec<Wire>,
    /// `back`.
    pub background: u16,
    /// `numG`: how many of each part the tray still has, where limited
    /// (a puzzle's; -1 is unlimited, 0 greys the part out).
    pub tray_limits: Vec<(Tag, i16)>,
    /// The version it was read as.
    pub version: u16,
    next_id: u32,
}

impl Document {
    #[must_use]
    pub fn new() -> Document {
        Document {
            version: VERSION,
            next_id: 1,
            ..Document::default()
        }
    }

    /// Reads a saved widget.
    ///
    /// # Errors
    /// If it isn't one, or of a version the game doesn't read.
    pub fn load(assets: &Assets, bytes: &[u8]) -> Result<Document, Error> {
        let mut form = Form::parse(bytes).map_err(Error::Format)?;
        crate::net::wgt::reveal(&mut form);
        let version = form.version().unwrap_or(1);
        if version == 0 || version > VERSION {
            return Err(Error::Version(version));
        }
        let mut doc = Document {
            version,
            next_id: 1,
            ..Document::default()
        };
        let mut conns = Vec::new();
        for c in &form.chunks {
            match &c.tag {
                b"fver" => {}
                b"back" => doc.background = wgt::be16(&c.data, 0).unwrap_or(0),
                b"numG" => doc.tray_limits = read_tray_limits(&c.data),
                b"conn" => {
                    if let Ok(conn) = wgt::Connection::parse(&c.data, version) {
                        conns.push(conn);
                    }
                }
                _ => doc.gadgets.push(load_gadget(assets, c)),
            }
        }
        doc.next_id = doc.gadgets.iter().map(|g| g.core.id).max().unwrap_or(0).wrapping_add(1);
        for c in conns {
            let find = |id: u32| doc.gadgets.iter().position(|g| g.core.id == id);
            let (Some(from), Some(to)) = (find(c.from), find(c.to)) else {
                continue;
            };
            doc.connect(
                from,
                usize::from(c.from_knot),
                to,
                usize::from(c.to_knot),
                c.points.iter().map(|&(v, h)| (i32::from(h), i32::from(v))).collect(),
                c.flag,
            );
        }
        Ok(doc)
    }

    /// The widget as a saved widget file.
    #[must_use]
    pub fn save(&self) -> Vec<u8> {
        let mut form = Form::default();
        form.chunks.push(Chunk {
            tag: *b"fver",
            data: VERSION.to_be_bytes().to_vec(),
        });
        form.chunks.push(Chunk {
            tag: *b"back",
            data: self.background.to_be_bytes().to_vec(),
        });
        for g in &self.gadgets {
            form.chunks.push(save_gadget(g));
        }
        for w in &self.wires {
            let to16 = |i: i32| i16::try_from(i).unwrap_or(0);
            let conn = wgt::Connection {
                from: self.gadgets[w.from].core.id,
                to: self.gadgets[w.to].core.id,
                from_knot: u16::try_from(w.from_knot).unwrap_or(0),
                to_knot: u16::try_from(w.to_knot).unwrap_or(0),
                flag: w.flag,
                points: w.points.iter().map(|&(h, v)| (to16(v), to16(h))).collect(),
            };
            form.chunks.push(Chunk {
                tag: *b"conn",
                data: conn.write(),
            });
        }
        // The tray limits come last, after the wires (`0x424d0a`).
        if self.tray_limits.iter().any(|&(_, n)| n != -1) {
            form.chunks.push(Chunk {
                tag: *b"numG",
                data: write_tray_limits(&self.tray_limits),
            });
        }
        crate::net::wgt::hide(&mut form);
        form.write()
    }

    /// A fresh id for a new gadget.
    pub fn new_id(&mut self) -> u32 {
        let id = self.next_id.max(1);
        self.next_id = id.wrapping_add(1);
        id
    }

    /// Wires output `from_knot` of gadget `from` to input `to_knot` of
    /// gadget `to`, replacing what either knot had. Returns the wire.
    pub fn connect(
        &mut self,
        from: usize,
        from_knot: usize,
        to: usize,
        to_knot: usize,
        points: Vec<(i32, i32)>,
        flag: u8,
    ) -> Option<usize> {
        if from >= self.gadgets.len()
            || to >= self.gadgets.len()
            || from_knot >= self.gadgets[from].core.knots.len()
            || to_knot >= self.gadgets[to].core.knots.len()
        {
            return None;
        }
        for (g, k) in [(from, from_knot), (to, to_knot)] {
            if let Some(w) = self.gadgets[g].core.knots[k].wire {
                self.remove_wire(w);
            }
        }
        let i = self.wires.len();
        self.wires.push(Wire {
            from,
            from_knot,
            to,
            to_knot,
            flag,
            points,
            changed: false,
        });
        self.gadgets[from].core.knots[from_knot].wire = Some(i);
        self.gadgets[to].core.knots[to_knot].wire = Some(i);
        Some(i)
    }

    /// Removes wire `w`, renumbering the ones after it.
    pub fn remove_wire(&mut self, w: usize) {
        if w >= self.wires.len() {
            return;
        }
        self.wires.remove(w);
        for g in &mut self.gadgets {
            for k in &mut g.core.knots {
                k.wire = match k.wire {
                    Some(x) if x == w => None,
                    Some(x) if x > w => Some(x - 1),
                    x => x,
                };
            }
        }
    }

    /// Removes gadget `i` and its wires.
    pub fn remove_gadget(&mut self, i: usize) -> Gadget {
        let mut ws: Vec<usize> = self.gadgets[i].wires().collect();
        ws.sort_unstable();
        for w in ws.into_iter().rev() {
            self.remove_wire(w);
        }
        let g = self.gadgets.remove(i);
        for w in &mut self.wires {
            if w.from > i {
                w.from -= 1;
            }
            if w.to > i {
                w.to -= 1;
            }
        }
        g
    }

    /// Runs `f` on gadget `i` with an [`Io`] over the rest.
    pub fn with_gadget<R>(
        &mut self,
        i: usize,
        assets: &Assets,
        env: &mut Env,
        f: impl FnOnce(&mut Gadget, &mut Io<'_>) -> R,
    ) -> R {
        // A part wired to itself reads its own outputs through the
        // placeholder, as the original reads its own fields: they're
        // taken before it runs (`Outputs`). Only such a wire reaches the
        // placeholder: an output takes no `Io`, so nothing follows wires
        // through it. The part's own knots name its wires.
        let own_wire = self.gadgets[i]
            .core
            .knots
            .iter()
            .filter_map(|k| k.wire)
            .any(|w| self.wires.get(w).is_some_and(|w| w.from == i && w.to == i));
        let behavior: Box<dyn crate::gadget::Behavior> = if own_wire {
            Box::new(Outputs::of(&self.gadgets[i]))
        } else {
            Box::new(Unknown::default())
        };
        let placeholder = Gadget {
            core: self.gadgets[i].core.clone(),
            behavior,
        };
        let mut g = std::mem::replace(&mut self.gadgets[i], placeholder);
        let r = {
            let mut io = Io {
                assets,
                gadgets: &self.gadgets,
                wires: &mut self.wires,
                env,
            };
            f(&mut g, &mut io)
        };
        self.gadgets[i] = g;
        r
    }

    /// Starts running (`0x424a12`): every gadget's slot 0x20, head to
    /// tail (the end of [`Document::gadgets`] first), wires unchanged.
    pub fn start(&mut self, assets: &Assets, env: &mut Env) {
        for w in &mut self.wires {
            w.changed = false;
        }
        for i in (0..self.gadgets.len()).rev() {
            self.with_gadget(i, assets, env, |g, io| g.behavior.start(&mut g.core, io));
        }
    }

    /// Stops running (`0x424a94`): every gadget's slot 0x1c.
    pub fn stop(&mut self) {
        for w in &mut self.wires {
            w.changed = false;
        }
        for g in &mut self.gadgets {
            g.behavior.reset(&mut g.core);
        }
    }

    /// One tick: every gadget's slot 0x44, head to tail, so the newest
    /// first (`0x42498f`, docs/editor-interaction.md 1.2); only [`Env::exclusive`] while one is set, and none after a gadget
    /// ends the run.
    pub fn tick(&mut self, assets: &Assets, env: &mut Env) {
        for i in (0..self.gadgets.len()).rev() {
            if env.end_run.is_some() {
                break;
            }
            if env.exclusive.is_some_and(|id| id != self.gadgets[i].core.id) {
                continue;
            }
            self.with_gadget(i, assets, env, |g, io| g.behavior.tick(&mut g.core, io));
        }
    }

    /// The topmost gadget at `x`, `y` on the worksheet.
    #[must_use]
    pub fn gadget_at(&self, x: i32, y: i32) -> Option<usize> {
        self.gadgets.iter().rposition(|g| g.core.rect().contains(x, y))
    }

    /// Draws every gadget in list order.
    pub fn draw_gadgets(&self, cx: &mut DrawCx<'_>) {
        for g in &self.gadgets {
            g.behavior.draw(&g.core, cx);
        }
    }

    /// The bounding box of everything on the worksheet.
    #[must_use]
    pub fn extent(&self) -> crate::gfx::IRect {
        self.gadgets
            .iter()
            .map(|g| g.core.rect())
            .fold(crate::gfx::IRect::default(), crate::gfx::IRect::union)
    }
}

/// What a running part's outputs gave as it began its step, standing in
/// for it while it runs ([`Document::with_gadget`]) so that a wire from
/// one of its outputs to one of its own inputs reads them.
#[derive(Debug, Default)]
struct Outputs {
    /// Per knot: (has, value) for the boolean, number and object.
    knots: Vec<OutputKnot>,
}

#[derive(Debug, Default, Clone)]
struct OutputKnot {
    bool: Option<bool>,
    number: Option<f64>,
    /// Whether it gives an object, and which (`None` a silent part's).
    gives_object: bool,
    object: Option<crate::gadget::Object>,
}

impl Outputs {
    fn of(g: &Gadget) -> Outputs {
        let (b, c) = (&g.behavior, &g.core);
        let knots = (0..c.knots.len())
            .map(|k| OutputKnot {
                bool: b.has_bool(c, k).then(|| b.bool_out(c, k)),
                number: b.has_number(c, k).then(|| b.number_out(c, k)),
                gives_object: b.has_object(c, k),
                object: if b.has_object(c, k) { b.object_out(c, k) } else { None },
            })
            .collect();
        Outputs { knots }
    }

    fn knot(&self, k: usize) -> Option<&OutputKnot> {
        self.knots.get(k)
    }
}

impl crate::gadget::Behavior for Outputs {
    fn has_bool(&self, _core: &Core, k: usize) -> bool {
        self.knot(k).is_some_and(|o| o.bool.is_some())
    }

    fn bool_out(&self, _core: &Core, k: usize) -> bool {
        self.knot(k).and_then(|o| o.bool).unwrap_or(false)
    }

    fn has_number(&self, _core: &Core, k: usize) -> bool {
        self.knot(k).is_some_and(|o| o.number.is_some())
    }

    fn number_out(&self, _core: &Core, k: usize) -> f64 {
        self.knot(k).and_then(|o| o.number).unwrap_or(0.0)
    }

    fn has_object(&self, _core: &Core, k: usize) -> bool {
        self.knot(k).is_some_and(|o| o.gives_object)
    }

    fn object_out(&self, _core: &Core, k: usize) -> Option<crate::gadget::Object> {
        self.knot(k).and_then(|o| o.object.clone())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The string with a fresh Billboard's text, in OpenWDATA (the original
/// has its text in the code).
const BILLBOARD_TEXT: u32 = 20000;

/// A new gadget of class `tag` (a fresh one, as dragged from the tray), at
/// `h`, `v`.
#[must_use]
pub fn new_gadget(assets: &Assets, tag: Tag, id: u32, h: i32, v: i32) -> Gadget {
    let (core, behavior) = make(assets, tag);
    let mut g = Gadget { core, behavior };
    g.core.id = id;
    g.core.h = h;
    g.core.v = v;
    g.behavior.init(&mut g.core);
    // The Billboard's first words: OpenWDATA's own (string 20000), else
    // the original's (`TextBox::init`).
    if tag == Tag(*b"TxBx")
        && let Some(t) = assets.string(BILLBOARD_TEXT)
    {
        g.core
            .settings
            .set_long_text(&crate::text::encode(&t.replace('\n', "\r\n")));
    }
    g.behavior.reset(&mut g.core);
    g.behavior.settings_changed(&mut g.core);
    g.behavior.layout(&mut g.core, assets);
    g
}

fn make(assets: &Assets, tag: Tag) -> (Core, Box<dyn crate::gadget::Behavior>) {
    let class = gadgets::class(tag);
    let (gid, sheet, knot_table) = class.map_or((0, 0, 0), |c| (c.gadget_id, c.sheet, c.knots));
    let mut core = Core::new(tag, gid, sheet);
    core.knots = knots(assets, knot_table);
    let behavior = class.map_or_else(|| Box::new(Unknown::default()) as Box<_>, |c| (c.make)());
    core.bounds = crate::gfx::Pictures::default()
        .rect(assets, crate::assets::Module::Gadget, sheet, behavior.bounds_frame())
        .unwrap_or_default();
    (core, behavior)
}

#[allow(clippy::cast_possible_wrap)]
fn load_gadget(assets: &Assets, c: &Chunk) -> Gadget {
    let tag = Tag(c.tag);
    let (mut core, behavior) = make(assets, tag);
    let d = &c.data;
    let base = &d[..d.len().min(BASE_SIZE)];
    core.id = wgt::be32(base, 0).unwrap_or(0);
    core.v = i32::from(wgt::be16(base, 4).unwrap_or(0) as i16);
    core.h = i32::from(wgt::be16(base, 6).unwrap_or(0) as i16);
    core.settings = Settings::from_be(base.get(8..).unwrap_or_default());
    core.locked = base.get(0x110).is_some_and(|&b| b != 0);
    core.raw_base = base.to_vec();
    let mut g = Gadget { core, behavior };
    g.behavior.load(&mut g.core, d.get(BASE_SIZE..).unwrap_or_default());
    g.behavior.reset(&mut g.core);
    g.behavior.settings_changed(&mut g.core);
    g.behavior.layout(&mut g.core, assets);
    g
}

fn save_gadget(g: &Gadget) -> Chunk {
    let mut base = g.core.raw_base.clone();
    base.resize(BASE_SIZE, 0);
    base[0..4].copy_from_slice(&g.core.id.to_be_bytes());
    base[4..6].copy_from_slice(&i16::try_from(g.core.v).unwrap_or(0).to_be_bytes());
    base[6..8].copy_from_slice(&i16::try_from(g.core.h).unwrap_or(0).to_be_bytes());
    let settings = g.behavior.saved_settings(&g.core);
    let settings = settings.as_ref().unwrap_or(&g.core.settings);
    base[8..8 + crate::gadget::SETTINGS_SIZE].copy_from_slice(&settings.to_be());
    base[0x110] = u8::from(g.core.locked);
    base.extend_from_slice(&g.behavior.save(&g.core));
    Chunk {
        tag: g.core.tag.0,
        data: base,
    }
}

/// A `numG` chunk's entries (`0x432b3c`).
fn read_tray_limits(d: &[u8]) -> Vec<(Tag, i16)> {
    let n = usize::from(wgt::be16(d, 0).unwrap_or(0));
    (0..n)
        .map_while(|i| {
            let at = 2 + 6 * i;
            let tag: [u8; 4] = d.get(at..at + 4)?.try_into().ok()?;
            Some((Tag(tag), i16::from_be_bytes([*d.get(at + 4)?, *d.get(at + 5)?])))
        })
        .collect()
}

/// The `numG` chunk for the limited parts (`0x432aa5`).
fn write_tray_limits(limits: &[(Tag, i16)]) -> Vec<u8> {
    let limited: Vec<_> = limits.iter().filter(|&&(_, n)| n != -1).collect();
    let mut out = u16::try_from(limited.len()).unwrap_or(0).to_be_bytes().to_vec();
    for (tag, n) in limited {
        out.extend_from_slice(&tag.0);
        out.extend_from_slice(&n.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gadget::Value;

    /// A Switcher whose output is wired to its own selector reads its own
    /// output (a false boolean), not nothing: it stays on input 1, as the
    /// original does, instead of jumping to input 2.
    #[test]
    fn a_part_reads_its_own_output() {
        // The Switcher's knots (type 1014/1036): selector, inputs 1 and 2,
        // output, as (v, h, kind).
        let mut knots = vec![4, 0];
        for (v, h, k) in [(4u16, 52u16, 0u16), (29, 4, 0), (59, 4, 0), (45, 82, 1)] {
            for n in [v, h, k] {
                knots.extend_from_slice(&n.to_le_bytes());
            }
        }
        let files: Vec<(&'static str, &'static [u8])> =
            vec![("WWGADGET/1014/1036.bin", Box::leak(knots.into_boxed_slice()))];
        let tables = crate::assets::Embedded::new(Box::leak(files.into_boxed_slice()));
        let assets = Assets::load(None, Some(&tables)).unwrap();
        let mut doc = Document::new();
        doc.gadgets.push(new_gadget(&assets, Tag(*b"Mplx"), 1, 0, 0));
        doc.connect(0, 3, 0, 0, vec![], 0).unwrap();
        let mut env = Env::default();
        doc.start(&assets, &mut env);
        let seen = doc.with_gadget(0, &assets, &mut env, |g, io| io.input(&g.core, 0));
        assert_eq!(seen, Value::Bool(false));
        for t in 0..20 {
            env.ticks = t * 5;
            doc.tick(&assets, &mut env);
        }
        assert!(env.sounds.is_empty(), "it switched: {:?}", env.sounds);
    }
}
