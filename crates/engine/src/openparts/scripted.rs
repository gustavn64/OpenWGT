//! The scripted part (`Scrp`): a part whose behaviour is a script's
//! (`script::parts`), for making parts of one's own. Its script calls
//! `definePart({name, inputs, outputs, settings, start, tick})`; the part
//! shows the name on its plate and has the inputs (left) and outputs
//! (right) it names, up to four each, at the knots of table 1905, in
//! order. Each tick it hands its inputs' values to the script's `tick`
//! and gives what that returns, an output keeping its value until the
//! script gives another.
//!
//! A script that throws or runs too long breaks its part (the game's
//! broken picture) until the run stops. A part whose script the player
//! hasn't allowed to run is inert, drawn "script not allowed" (frame 2):
//! it gives nothing.
//!
//! Its chunk keeps, after the base, the script and what it defined (so
//! the part has its knots and name before its script ever runs):
//!
//! ```text
//! 01                     this layout
//! u8 length, the name (UTF-8)
//! u8 inputs, then each: u8 length, its name
//! u8 outputs, then each: u8 length, its name
//! the script (UTF-8), to the chunk's end
//! ```

use crate::assets::Assets;
use crate::gadget::{Behavior, Core, DrawCx, Knot, Tag};
use crate::script::parts::{DEFAULT_CODE, MAX_KNOTS, PartDef};
use crate::script::runtime::ScriptError;
use crate::script::trust::Digest;
use crate::script::{Level, Val};
use crate::sim::Io;
use formats::tables::KnotKind;

pub const SCRIPTED: Tag = Tag::new(b"Scrp");
/// Its sheet: 1 the part, 2 its script not allowed, 3 an input's stalk,
/// 4 an output's; and its knot table (four inputs, then four outputs).
pub const SHEET: u32 = 1905;
/// The part's size.
pub const SIZE: (i32, i32) = (128, 84);
/// The name plate (x, y, width, height), where the name is written.
pub const PLATE: (i32, i32, i32, i32) = (17, 5, 94, 13);
/// The knots' names start this far in from the part's sides.
const NAME_IN: i32 = 14;
const FACE: &str = "Small Fonts";
const FACE_SIZE: u16 = 9;
const NAME_COLOUR: [u8; 3] = [150, 255, 150];
const KNOT_COLOUR: [u8; 3] = [240, 240, 232];
/// The saved layout.
const LAYOUT: u8 = 1;

/// A scripted part.
#[derive(Debug)]
pub struct Scripted {
    code: String,
    digest: Digest,
    def: PartDef,
    /// What each output gives.
    outs: Vec<Val>,
    /// Whether its script may run, as last seen ([`Scripted::set_allowed`],
    /// and at GO).
    allowed: bool,
    /// Its script runs this run (allowed, and it started).
    ready: bool,
}

impl Default for Scripted {
    fn default() -> Scripted {
        Scripted {
            code: DEFAULT_CODE.to_owned(),
            digest: Digest::of(DEFAULT_CODE),
            def: PartDef::default_part(),
            outs: Vec::new(),
            allowed: false,
            ready: false,
        }
    }
}

/// `def` with at most [`MAX_KNOTS`] inputs and outputs, and names that
/// fit their bytes.
fn clamp(mut def: PartDef) -> PartDef {
    def.inputs.truncate(MAX_KNOTS);
    def.outputs.truncate(MAX_KNOTS);
    def
}

fn put(out: &mut Vec<u8>, s: &str) {
    let mut end = s.len().min(usize::from(u8::MAX));
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    out.push(u8::try_from(end).unwrap_or(0));
    out.extend_from_slice(&s.as_bytes()[..end]);
}

fn get(d: &[u8], at: &mut usize) -> Option<String> {
    let n = usize::from(*d.get(*at)?);
    let s = std::str::from_utf8(d.get(*at + 1..*at + 1 + n)?).ok()?.to_owned();
    *at += 1 + n;
    Some(s)
}

fn names(d: &[u8], at: &mut usize) -> Option<Vec<String>> {
    let n = *d.get(*at)?;
    *at += 1;
    (0..n).map(|_| get(d, at)).collect()
}

impl Scripted {
    /// Its script.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Its script's digest, which the player's consent is kept by.
    #[must_use]
    pub fn digest(&self) -> Digest {
        self.digest
    }

    /// What its script defined.
    #[must_use]
    pub fn def(&self) -> &PartDef {
        &self.def
    }

    /// A new script, and what it defines (the part's knots follow at its
    /// next layout).
    pub fn set_code(&mut self, code: String, def: PartDef) {
        self.digest = Digest::of(&code);
        self.code = code;
        self.def = clamp(def);
        self.outs.clear();
    }

    /// Whether its script may run, for its picture.
    pub fn set_allowed(&mut self, on: bool) {
        self.allowed = on;
    }

    #[must_use]
    pub fn allowed(&self) -> bool {
        self.allowed
    }

    /// Output knot `k`'s value.
    fn out(&self, k: usize) -> Option<&Val> {
        self.outs.get(k.checked_sub(self.def.inputs.len())?)
    }

    /// Its script failed: the part breaks, and says why.
    fn fail(&mut self, core: &mut Core, io: &mut Io<'_>, e: &ScriptError) {
        core.broken = true;
        self.ready = false;
        let text = format!("{} (#{}): {e}", self.def.name, core.id);
        io.env.parts.with(|p| p.log.push((Level::Error, text)));
    }
}

impl Behavior for Scripted {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        cx.frame_at_rect(core.sheet, if self.allowed { 1 } else { 2 }, core.h, core.v);
        for k in &core.knots {
            let frame = if k.is_input() { 3 } else { 4 };
            cx.frame_at_rect(
                core.sheet,
                frame,
                core.h + i32::from(k.def.at.h),
                core.v + i32::from(k.def.at.v),
            );
        }
        // The name, as much of it as the plate takes.
        let (px, py, pw, _) = PLATE;
        let fit = |cx: &DrawCx<'_>, s: &str, w: i32| {
            let mut t = crate::text::encode(s);
            while !t.is_empty() && cx.text_width(FACE, FACE_SIZE, false, &t) > w {
                t.pop();
            }
            t
        };
        let name = fit(cx, &self.def.name, pw - 4);
        let w = cx.text_width(FACE, FACE_SIZE, false, &name);
        let c = cx.color(NAME_COLOUR);
        cx.text(
            FACE,
            FACE_SIZE,
            false,
            core.h + px + (pw - w) / 2,
            core.v + py + 1,
            &name,
            c,
        );
        // The knots' names, beside them.
        let c = cx.color(KNOT_COLOUR);
        let half = SIZE.0 / 2 - NAME_IN - 2;
        let n_in = self.def.inputs.len();
        for (i, k) in core.knots.iter().enumerate() {
            let label = if k.is_input() {
                self.def.inputs.get(i)
            } else {
                self.def.outputs.get(i - n_in.min(i))
            };
            let Some(label) = label else { continue };
            let t = fit(cx, label, half);
            let y = core.v + i32::from(k.def.at.v) - 5;
            let x = if k.is_input() {
                core.h + NAME_IN
            } else {
                core.h + SIZE.0 - NAME_IN - cx.text_width(FACE, FACE_SIZE, false, &t)
            };
            cx.text(FACE, FACE_SIZE, false, x, y, &t, c);
        }
        if core.broken {
            crate::gadgets::sound_time::draw_broken(core, cx);
        }
    }

    fn start(&mut self, core: &mut Core, io: &mut Io<'_>) {
        io.base_start(core);
        self.outs = vec![Val::None; self.def.outputs.len()];
        self.allowed = io.env.parts.allowed(&self.digest);
        self.ready = false;
        if !self.allowed {
            return;
        }
        match io.env.parts.runtime(|rt| rt.start(core.id, &self.code)) {
            Some(Ok(())) => self.ready = true,
            Some(Err(e)) => self.fail(core, io, &e),
            None => {}
        }
    }

    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        if !self.ready || core.broken {
            return;
        }
        let n_in = self.def.inputs.len();
        let inputs: Vec<Val> = (0..n_in)
            .map(|k| {
                if io.wired(core, k) {
                    Val::from_value(&io.input(core, k))
                } else {
                    Val::None
                }
            })
            .collect();
        let ticks = io.env.ticks;
        match io.env.parts.runtime(|rt| rt.tick(core.id, &self.code, ticks, &inputs)) {
            Some(Ok(outs)) => {
                for (o, v) in outs.into_iter().enumerate().take(self.outs.len()) {
                    let Some(v) = v else { continue };
                    let v = match v {
                        Val::Bool(_) | Val::Number(_) => v,
                        _ => Val::None,
                    };
                    if v != self.outs[o] {
                        self.outs[o] = v;
                        io.fire(core, n_in + o);
                    }
                }
            }
            Some(Err(e)) => self.fail(core, io, &e),
            None => {}
        }
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        self.outs.clear();
        self.ready = false;
    }

    fn has_bool(&self, _core: &Core, k: usize) -> bool {
        matches!(self.out(k), Some(Val::Bool(_)))
    }

    fn bool_out(&self, _core: &Core, k: usize) -> bool {
        matches!(self.out(k), Some(Val::Bool(true)))
    }

    fn has_number(&self, _core: &Core, k: usize) -> bool {
        matches!(self.out(k), Some(Val::Number(_)))
    }

    fn number_out(&self, _core: &Core, k: usize) -> f64 {
        match self.out(k) {
            Some(Val::Number(x)) => *x,
            _ => 0.0,
        }
    }

    fn save(&self, _core: &Core) -> Vec<u8> {
        let mut out = vec![LAYOUT];
        put(&mut out, &self.def.name);
        for list in [&self.def.inputs, &self.def.outputs] {
            out.push(u8::try_from(list.len()).unwrap_or(0));
            for n in list {
                put(&mut out, n);
            }
        }
        out.extend_from_slice(self.code.as_bytes());
        out
    }

    fn load(&mut self, _core: &mut Core, d: &[u8]) {
        let read = || -> Option<(PartDef, String)> {
            if *d.first()? != LAYOUT {
                return None;
            }
            let mut at = 1;
            let name = get(d, &mut at)?;
            let inputs = names(d, &mut at)?;
            let outputs = names(d, &mut at)?;
            let code = std::str::from_utf8(d.get(at..)?).ok()?.to_owned();
            Some((PartDef { name, inputs, outputs }, code))
        };
        if let Some((def, code)) = read() {
            self.set_code(code, def);
        }
    }

    /// Its knots: table 1905's first inputs and outputs, as many as its
    /// script names; those it keeps keep their wires.
    fn layout(&mut self, core: &mut Core, assets: &Assets) {
        let table = crate::gadget::knots(assets, core.sheet);
        let ins = table.iter().filter(|k| k.def.kind == KnotKind::Input);
        let outs = table.iter().filter(|k| k.def.kind != KnotKind::Input);
        let new: Vec<Knot> = ins
            .take(self.def.inputs.len())
            .chain(outs.take(self.def.outputs.len()))
            .cloned()
            .enumerate()
            .map(|(i, mut k)| {
                k.wire = core
                    .knots
                    .get(i)
                    .filter(|old| old.def.kind == k.def.kind)
                    .and_then(|old| old.wire);
                k
            })
            .collect();
        if !table.is_empty() {
            core.knots = new;
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A new scripted part (`gadgets::Class::make`).
#[must_use]
pub fn scripted_part() -> Box<dyn Behavior> {
    Box::new(Scripted::default())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::assets::Assets;
    use crate::doc::{Document, new_gadget};
    use crate::script::parts::PartRuntime;

    /// A language for tests: a script `name:N in:a,b out:c` defines that;
    /// its tick gives the sum of its number inputs on every output; a
    /// script with `throw` fails its ticks.
    #[derive(Default)]
    pub(crate) struct Fake {
        pub started: Vec<u32>,
    }

    impl PartRuntime for Fake {
        fn language(&self) -> &'static str {
            "fake"
        }

        fn define(&mut self, _part: u32, code: &str) -> Result<PartDef, ScriptError> {
            let mut def = PartDef::default();
            for w in code.split_whitespace() {
                let list = |s: &str| s.split(',').filter(|x| !x.is_empty()).map(str::to_owned).collect();
                if let Some(n) = w.strip_prefix("name:") {
                    def.name = n.to_owned();
                } else if let Some(l) = w.strip_prefix("in:") {
                    def.inputs = list(l);
                } else if let Some(l) = w.strip_prefix("out:") {
                    def.outputs = list(l);
                }
            }
            if def.name.is_empty() {
                return Err(ScriptError::at(
                    crate::script::runtime::ErrKind::Syntax,
                    "no name",
                    "fake",
                    1,
                    1,
                    1,
                    Some(code),
                ));
            }
            Ok(def)
        }

        fn start(&mut self, part: u32, _code: &str) -> Result<(), ScriptError> {
            self.started.push(part);
            Ok(())
        }

        fn tick(
            &mut self,
            _part: u32,
            code: &str,
            _tick: u32,
            inputs: &[Val],
        ) -> Result<Vec<Option<Val>>, ScriptError> {
            if code.contains("throw") {
                return Err(ScriptError::at(
                    crate::script::runtime::ErrKind::Runtime,
                    "thrown",
                    "fake",
                    1,
                    1,
                    1,
                    None,
                ));
            }
            let sum: f64 = inputs
                .iter()
                .filter_map(|v| if let Val::Number(x) = v { Some(*x) } else { None })
                .sum();
            let n = self.define(0, code).map_or(0, |d| d.outputs.len());
            Ok(vec![Some(Val::Number(sum)); n])
        }

        fn clear(&mut self) {}

        fn take_log(&mut self) -> Vec<(Level, String)> {
            Vec::new()
        }
    }

    fn assets() -> Assets {
        Assets::load(None, Some(&crate::opendata::builtin())).unwrap()
    }

    /// A scripted part with `code` (as the fake defines it), at 100, 60.
    pub(crate) fn part(a: &Assets, id: u32, code: &str) -> crate::gadget::Gadget {
        let mut g = new_gadget(a, SCRIPTED, id, 100, 60);
        let def = Fake::default().define(id, code).unwrap();
        if let Some(s) = g.behavior.as_any_mut().downcast_mut::<Scripted>() {
            s.set_code(code.to_owned(), def);
        }
        g.behavior.layout(&mut g.core, a);
        g
    }

    #[test]
    fn a_new_one_passes_its_input_on() {
        let a = assets();
        let g = new_gadget(&a, SCRIPTED, 1, 0, 0);
        let s = g.behavior.as_any().downcast_ref::<Scripted>().unwrap();
        assert_eq!(s.code(), DEFAULT_CODE);
        assert_eq!(s.def(), &PartDef::default_part());
        let kinds: Vec<bool> = g.core.knots.iter().map(Knot::is_input).collect();
        assert_eq!(kinds, [true, false]);
        let r = g.core.rect();
        assert_eq!((r.width(), r.height()), SIZE);
    }

    #[test]
    fn its_knots_are_its_scripts_and_inside_its_picture() {
        let a = assets();
        let g = part(&a, 1, "name:Mix in:a,b,c out:x,y");
        let at: Vec<(i32, i32, bool)> = g
            .core
            .knots
            .iter()
            .map(|k| (i32::from(k.def.at.h), i32::from(k.def.at.v), k.is_input()))
            .collect();
        assert_eq!(
            at,
            [
                (4, 30, true),
                (4, 44, true),
                (4, 58, true),
                (123, 30, false),
                (123, 44, false)
            ]
        );
        // Every knot of the table at least 4 pixels inside the picture.
        for k in crate::gadget::knots(&a, SHEET) {
            let (h, v) = (i32::from(k.def.at.h), i32::from(k.def.at.v));
            assert!(h >= 4 && v >= 4 && h <= SIZE.0 - 5 && v <= SIZE.1 - 5, "{h}, {v}");
        }
        // Five of each are four.
        let g = part(&a, 1, "name:Big in:1,2,3,4,5 out:1,2,3,4,5");
        assert_eq!(g.core.knots.len(), 8);
    }

    #[test]
    fn it_survives_saving_and_hides_from_the_original() {
        let a = assets();
        let mut doc = Document::new();
        let id = doc.new_id();
        doc.gadgets
            .push(new_gadget(&a, crate::gadget::Tag::new(b"cnst"), id, 0, 0));
        let id = doc.new_id();
        doc.gadgets
            .push(part(&a, id, "name:Twice ✓ in:a out:b,c\n// the script"));
        let id = doc.new_id();
        doc.gadgets
            .push(new_gadget(&a, crate::gadget::Tag::new(b"bulb"), id, 300, 0));
        doc.connect(0, 0, 1, 0, vec![], 0).unwrap();
        doc.connect(1, 2, 2, 0, vec![(250, 40)], 0).unwrap();
        let saved = doc.save();
        let back = Document::load(&a, &saved).unwrap();
        assert_eq!(back.save(), saved, "byte for byte");
        let s = back.gadgets[1].behavior.as_any().downcast_ref::<Scripted>().unwrap();
        assert_eq!(s.code(), "name:Twice ✓ in:a out:b,c\n// the script");
        assert_eq!(s.def().name, "Twice");
        assert_eq!(back.gadgets[1].core.knots.len(), 3);
        assert_eq!(back.wires.len(), 2);
        assert_eq!(
            (back.wires[1].from_knot, back.wires[1].points.clone()),
            (2, vec![(250, 40)])
        );
        // The original sees a constant and a bulb, unwired.
        let form = formats::wgt::Form::parse(&saved).unwrap();
        assert!(form.chunks.iter().all(|c| c.tag != SCRIPTED.0));
        let mut plain = form.clone();
        plain.chunks.retain(|c| crate::port_parts::wgt::hidden(c).is_none());
        let other = Document::load(&a, &plain.write()).unwrap();
        assert_eq!(other.gadgets.len(), 2);
        assert_eq!(other.wires.len(), 0);
    }

    #[test]
    fn it_draws_inside_itself_allowed_or_not_and_broken() {
        use crate::gfx::{IRect, Pictures, Surface};
        let a = assets();
        let pictures = Pictures::default();
        let mut g = part(&a, 1, "name:AVeryLongNameThatWontFitOnThePlate in:a,b out:x");
        let r = g.core.rect();
        for (allowed, broken) in [(true, false), (false, false), (true, true)] {
            if let Some(s) = g.behavior.as_any_mut().downcast_mut::<Scripted>() {
                s.set_allowed(allowed);
            }
            g.core.broken = broken;
            let draw = |bg: u8| {
                let mut surface = Surface::new(300, 200);
                surface.fill(IRect::new(0, 0, 300, 200), bg);
                let mut cx = DrawCx {
                    surface: &mut surface,
                    assets: &a,
                    pictures: &pictures,
                    running: false,
                    ticks: 0,
                    lights: false,
                };
                g.behavior.draw(&g.core, &mut cx);
                surface
            };
            let (one, two) = (draw(3), draw(200));
            let mut n = 0;
            for i in 0..one.pixels.len() {
                let (x, y) = (i32::try_from(i % 300).unwrap(), i32::try_from(i / 300).unwrap());
                if one.pixels[i] != 3 || two.pixels[i] != 200 {
                    assert!(r.contains(x, y), "{x}, {y} outside {r:?}");
                    n += 1;
                }
            }
            assert!(n > 5000, "{allowed} {broken}: {n}");
        }
    }
}
