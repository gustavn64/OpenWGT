//! The editor as scripts drive it ([`Host`]).

use super::super::{Editor, Pointer, TickEnd, menu, worksheet};
use crate::doc::Document;
use crate::gadget::{Object, Tag};
use crate::script::adapt;
use crate::script::{
    Button, Clock, ErrorKind, Host, HostError, HostResult, KnotInfo, Level, MouseEv, PartId, PartInfo, PartSel,
    PartState, PuzzleInfo, SettingKey, SettingVal, Shot, SoundEvent, StepEnd, Val, WireRef, cmd, file_name, select,
    value,
};
use crate::sim::SoundCmd;

/// The Label's tag (`TxLb`), whose text names the part next to it.
const LABEL: Tag = Tag::new(b"TxLb");
/// How far a Label may be from the part it names, in pixels.
const LABEL_GAP: i32 = 40;

fn error(kind: ErrorKind, msg: impl Into<String>) -> HostError {
    HostError::new(kind, msg)
}

fn byte(n: usize) -> u8 {
    u8::try_from(n).unwrap_or(u8::MAX)
}

impl Editor {
    /// The index of part `id` in the document.
    fn index_of(&self, id: PartId) -> HostResult<usize> {
        self.doc.gadgets.iter().position(|g| g.core.id == id.0).ok_or_else(|| {
            error(
                ErrorKind::UnknownPart {
                    query: id.to_string(),
                    suggestions: Vec::new(),
                },
                format!("no part {id} in this widget"),
            )
        })
    }

    /// The part and knot at the other end of wire `w` from gadget `i`'s
    /// knot `k`, as [`crate::sim::Io::source`] reads it.
    fn other_end(&self, w: usize, i: usize, k: usize) -> Option<(usize, usize)> {
        let w = self.doc.wires.get(w)?;
        Some(if w.to == i && w.to_knot == k {
            (w.from, w.from_knot)
        } else {
            (w.to, w.to_knot)
        })
    }

    /// What gadget `i`'s output `k` gives.
    fn out_value(&self, i: usize, k: usize) -> Val {
        self.doc
            .gadgets
            .get(i)
            .map_or(Val::None, |g| Val::from_value(&g.behavior.output(&g.core, k)))
    }

    fn wire_ref(&self, w: usize) -> Option<WireRef> {
        let w = self.doc.wires.get(w)?;
        let id = |i: usize| PartId(self.doc.gadgets.get(i).map_or(0, |g| g.core.id));
        Some(WireRef {
            from: (id(w.from), byte(w.from_knot)),
            to: (id(w.to), byte(w.to_knot)),
        })
    }

    /// The wire `r` names.
    fn wire_index(&self, r: &WireRef) -> HostResult<usize> {
        (0..self.doc.wires.len())
            .find(|&w| self.wire_ref(w).as_ref() == Some(r))
            .ok_or_else(|| {
                error(
                    ErrorKind::BadKnot,
                    format!("no wire from {}.{} to {}.{}", r.from.0, r.from.1, r.to.0, r.to.1),
                )
            })
    }

    /// The text of the Label naming gadget `i`: the nearest above it or
    /// to its left, overlapping it the other way, within [`LABEL_GAP`];
    /// for a Text Line or a Text Box its own text; none for a Label.
    fn label_of(&self, i: usize) -> Option<String> {
        let g = &self.doc.gadgets[i];
        let own = match &g.core.tag.0 {
            // A Label's text names the part next to it, not itself.
            b"TxLb" => return None,
            b"TxLn" => Some(g.core.settings.label_str(0)),
            b"TxBx" => {
                let t = crate::text::decode(g.core.settings.long_text());
                t.lines().next().map(str::to_owned)
            }
            _ => None,
        };
        if let Some(t) = own {
            let t = t.trim().to_owned();
            return (!t.is_empty()).then_some(t);
        }
        let r = g.core.rect();
        let mut best: Option<(i32, u8, String)> = None;
        for l in &self.doc.gadgets {
            if l.core.tag != LABEL {
                continue;
            }
            let text = l.core.settings.label_str(0).trim().to_owned();
            if text.is_empty() {
                continue;
            }
            let lr = l.core.rect();
            let across = lr.x0 < r.x1 && lr.x1 > r.x0;
            let down = lr.y0 < r.y1 && lr.y1 > r.y0;
            let gap = if across && lr.y1 <= r.y0 + 4 {
                Some((r.y0 - lr.y1, 0))
            } else if down && lr.x1 <= r.x0 + 4 {
                Some((r.x0 - lr.x1, 1))
            } else {
                None
            };
            if let Some((d, side)) = gap
                && d <= LABEL_GAP
                && best.as_ref().is_none_or(|b| (d, side) < (b.0, b.1))
            {
                best = Some((d, side, text));
            }
        }
        best.map(|b| b.2)
    }

    /// Gadget `i` as a script sees it.
    fn info(&self, i: usize) -> PartInfo {
        let g = &self.doc.gadgets[i];
        let r = g.core.rect();
        let knots = g
            .core
            .knots
            .iter()
            .enumerate()
            .map(|(k, knot)| {
                let input = knot.is_input();
                let value = if input {
                    knot.wire
                        .and_then(|w| self.other_end(w, i, k))
                        .map_or(Val::None, |(src, sk)| self.out_value(src, sk))
                } else {
                    self.out_value(i, k)
                };
                KnotInfo {
                    index: byte(k),
                    input,
                    at: g.core.knot_at(k).unwrap_or_default(),
                    wire: knot.wire.and_then(|w| self.wire_ref(w)),
                    value,
                }
            })
            .collect();
        let kind = crate::gadgets::class(g.core.tag)
            .map_or_else(|| g.core.tag.to_string(), |c| c.name.trim_start_matches('c').to_owned());
        PartInfo {
            id: PartId(g.core.id),
            tag: g.core.tag.to_string(),
            kind,
            name: self.debug.name(g.core.id).map(str::to_owned),
            label: self.label_of(i),
            rect: (r.x0, r.y0, r.x1, r.y1),
            locked: g.core.locked,
            broken: g.core.broken,
            knots,
        }
    }

    /// Part `id`'s info, and its index.
    fn found(&self, id: PartId) -> HostResult<(usize, PartInfo)> {
        let i = self.index_of(id)?;
        Ok((i, self.info(i)))
    }

    /// The `n`th input (or output) of part `p`, as a knot index.
    fn nth(p: &PartInfo, input: bool, n: Option<u8>) -> HostResult<u8> {
        let n = n.unwrap_or(0);
        select::nth_knot(p, input, n).ok_or_else(|| {
            let what = if input { "input" } else { "output" };
            error(
                ErrorKind::BadKnot,
                format!(
                    "{} has {}; {what} {} doesn't exist",
                    p.describe(),
                    select::knots_said(p),
                    n + 1
                ),
            )
        })
    }

    fn unlocked(p: &PartInfo) -> HostResult<()> {
        if p.locked {
            Err(error(
                ErrorKind::Locked,
                format!("{} belongs to the puzzle and can't be changed", p.describe()),
            ))
        } else {
            Ok(())
        }
    }

    /// Writes `changes` into gadget `i`'s settings as its dialog's OK
    /// does: an undo step, slot 0x50, the layout.
    fn apply_settings(&mut self, i: usize, changes: &[(SettingKey, SettingVal)]) {
        self.snapshot();
        let g = &mut self.doc.gadgets[i];
        let s = &mut g.core.settings;
        for (k, v) in changes {
            match (k, v) {
                (SettingKey::Check(n), SettingVal::Bool(b)) => s.set_check(u32::from(*n), *b),
                (SettingKey::Radio(n), SettingVal::Int(x)) => {
                    s.radios[usize::from(*n)] = u8::try_from(*x).unwrap_or(u8::MAX);
                }
                (SettingKey::Number(n), SettingVal::Int(x)) => s.numbers[usize::from(*n)] = *x,
                (SettingKey::Text(n), SettingVal::Text(t)) => s.set_label(usize::from(*n), &crate::text::encode(t)),
                (SettingKey::LongText, SettingVal::Text(t)) => s.set_long_text(&crate::text::encode(t)),
                _ => {}
            }
        }
        g.behavior.settings_changed(&mut g.core);
        g.behavior.layout(&mut g.core, &self.assets);
        if g.core.tag == LABEL {
            // As the dialog's OK: the Label lays itself out again.
            g.behavior.reset(&mut g.core);
        }
        self.set_dirty();
    }

    /// Why a part has no value to set, and what to set instead.
    fn not_settable(&self, p: &PartInfo) -> HostError {
        let msg = if p.outputs().next().is_none() && p.inputs().next().is_some() {
            let src = p
                .inputs()
                .next()
                .and_then(|k| k.wire)
                .map(|w| w.from.0)
                .filter(|&id| id != p.id)
                .and_then(|id| self.index_of(id).ok())
                .map(|i| self.info(i).describe());
            match src {
                Some(s) => format!(
                    "{} shows its input, it has no value of its own. Set what's wired into it: {s}",
                    p.describe()
                ),
                None => format!(
                    "{} shows its input, it has no value of its own (and nothing is wired into it)",
                    p.describe()
                ),
            }
        } else {
            format!(
                "{} has no control of its own a script can set; press it, poke it, or change its settings",
                p.describe()
            )
        };
        error(ErrorKind::NotSettable, msg)
    }

    /// Takes the sounds asked for into the log of them.
    fn hear(&mut self) {
        let tick = self.env.ticks;
        for c in self.take_sounds() {
            let e = match c {
                SoundCmd::Play { sound, looping, .. } => SoundEvent::Play {
                    tick,
                    rate: sound.rate,
                    len: u32::try_from(sound.samples.len()).unwrap_or(u32::MAX),
                    looping,
                },
                SoundCmd::Stop { .. } => SoundEvent::Stop { tick },
                SoundCmd::StopAll => SoundEvent::StopAll { tick },
            };
            self.debug.heard(e);
        }
    }

    fn check_time(&self) -> HostResult<()> {
        if self.debug.timed_out() {
            Err(error(
                ErrorKind::Stopped,
                "the script ran out of time (--timeout); stopped",
            ))
        } else {
            Ok(())
        }
    }
}

fn button(b: Button) -> super::super::Button {
    match b {
        Button::Left => super::super::Button::Left,
        Button::Right => super::super::Button::Right,
    }
}

impl Host for Editor {
    fn parts(&self) -> Vec<PartInfo> {
        let mut v: Vec<PartInfo> = (0..self.doc.gadgets.len()).map(|i| self.info(i)).collect();
        v.sort_by_key(|p| (p.rect.1, p.rect.0, p.id));
        v
    }

    fn part(&self, id: PartId) -> HostResult<PartInfo> {
        self.found(id).map(|(_, p)| p)
    }

    fn resolve(&self, sel: &PartSel) -> HostResult<PartId> {
        match sel {
            // The topmost there, as a click finds it.
            &PartSel::At(x, y) => worksheet::gadget_at(&self.doc, (x, y))
                .map(|i| PartId(self.doc.gadgets[i].core.id))
                .ok_or_else(|| {
                    error(
                        ErrorKind::UnknownPart {
                            query: format!("@{x},{y}"),
                            suggestions: Vec::new(),
                        },
                        format!("no part at {x},{y}"),
                    )
                }),
            _ => select::resolve(&self.parts(), sel).map(|p| p.id),
        }
    }

    fn wires(&self) -> Vec<(WireRef, Val)> {
        (0..self.doc.wires.len())
            .filter_map(|w| {
                let r = self.wire_ref(w)?;
                let wire = &self.doc.wires[w];
                Some((r, self.out_value(wire.from, wire.from_knot)))
            })
            .collect()
    }

    fn source(&self, id: PartId, knot: u8) -> HostResult<Option<(PartId, u8)>> {
        let (i, p) = self.found(id)?;
        let k = usize::from(Self::nth(&p, true, Some(knot))?);
        Ok(self.doc.gadgets[i].core.knots[k]
            .wire
            .and_then(|w| self.other_end(w, i, k))
            .map(|(g, gk)| (PartId(self.doc.gadgets[g].core.id), byte(gk))))
    }

    fn set_name(&mut self, id: PartId, name: Option<&str>) -> HostResult<()> {
        self.index_of(id)?;
        let Some(name) = name.map(str::trim) else {
            self.debug.names.remove(&id.0);
            return Ok(());
        };
        let plain = select::parse(&value::quote(name));
        if name.is_empty() || name.starts_with(['#', '@']) || name.contains('"') || plain.is_err() {
            return Err(error(
                ErrorKind::BadArg,
                format!(
                    "{} can't be a name: it would read as something else",
                    value::quote(name)
                ),
            ));
        }
        if let Some((&other, _)) = self
            .debug
            .names
            .iter()
            .find(|&(&o, n)| o != id.0 && n.eq_ignore_ascii_case(name))
        {
            if self.index_of(PartId(other)).is_ok() {
                return Err(error(
                    ErrorKind::BadArg,
                    format!("{} is the name of #{other} already", value::quote(name)),
                ));
            }
            self.debug.names.remove(&other);
        }
        self.debug.names.insert(id.0, name.to_owned());
        Ok(())
    }

    fn output(&self, id: PartId, knot: Option<u8>) -> HostResult<Val> {
        let (i, p) = self.found(id)?;
        let k = Self::nth(&p, false, knot)?;
        Ok(self.out_value(i, usize::from(k)))
    }

    fn input(&self, id: PartId, knot: Option<u8>) -> HostResult<Val> {
        let (_, p) = self.found(id)?;
        let k = Self::nth(&p, true, knot)?;
        Ok(p.knots[usize::from(k)].value.clone())
    }

    fn wire_value(&self, w: &WireRef) -> HostResult<Val> {
        let w = self.wire_index(w)?;
        let wire = &self.doc.wires[w];
        Ok(self.out_value(wire.from, wire.from_knot))
    }

    fn wire_changed(&self, w: &WireRef) -> HostResult<bool> {
        let w = self.wire_index(w)?;
        Ok(self.doc.wires[w].changed)
    }

    fn state(&self, id: PartId) -> HostResult<PartState> {
        let i = self.index_of(id)?;
        let c = &self.doc.gadgets[i].core;
        Ok(PartState {
            state: c.state,
            number: c.number,
            object: Val::from_value(&crate::gadget::Value::Object(c.object.clone())),
            broken: c.broken,
            phase: c.phase,
        })
    }

    fn setting(&self, id: PartId, key: SettingKey) -> HostResult<SettingVal> {
        let i = self.index_of(id)?;
        let s = &self.doc.gadgets[i].core.settings;
        Ok(match key {
            SettingKey::Check(n) if n < 32 => SettingVal::Bool(s.check(u32::from(n))),
            SettingKey::Radio(n) if n < 4 => SettingVal::Int(u32::from(s.radios[usize::from(n)])),
            SettingKey::Number(n) if n < 4 => SettingVal::Int(s.numbers[usize::from(n)]),
            SettingKey::Text(n) if n < 4 => SettingVal::Text(s.label_str(usize::from(n))),
            SettingKey::LongText => SettingVal::Text(crate::text::decode(s.long_text())),
            _ => return Err(error(ErrorKind::BadKnot, format!("no setting {key:?}"))),
        })
    }

    fn set_value(&mut self, id: PartId, v: &Val) -> HostResult<Val> {
        let (i, p) = self.found(id)?;
        Self::unlocked(&p)?;
        let Some(c) = adapt::control(self.doc.gadgets[i].core.tag) else {
            return Err(self.not_settable(&p));
        };
        let refused = |r: adapt::Refusal| error(r.kind, format!("{} {}", p.describe(), r.why));
        if !self.running {
            let changes = adapt::start_settings(c, &self.doc.gadgets[i].core, v).map_err(refused)?;
            if let Some(changes) = changes {
                self.apply_settings(i, &changes);
                return Ok(v.clone());
            }
        }
        let r = self
            .doc
            .with_gadget(i, &self.assets, &mut self.env, |g, io| adapt::work(c, g, io, v));
        r.map_err(refused)
    }

    fn set_setting(&mut self, id: PartId, key: SettingKey, v: &SettingVal) -> HostResult<()> {
        let (i, p) = self.found(id)?;
        Self::unlocked(&p)?;
        if self.running {
            return Err(error(
                ErrorKind::NotRunning,
                "settings can't be changed while the widget runs (stop first)",
            ));
        }
        if self.busy() {
            return Err(error(ErrorKind::Document, "the editor is busy (a box is open)"));
        }
        let fits = match (key, v) {
            (SettingKey::Check(n), SettingVal::Bool(_)) => n < 32,
            (SettingKey::Radio(n), SettingVal::Int(x)) => n < 4 && *x < 256,
            (SettingKey::Number(n), SettingVal::Int(_)) | (SettingKey::Text(n), SettingVal::Text(_)) => n < 4,
            (SettingKey::LongText, SettingVal::Text(_)) => true,
            _ => false,
        };
        if !fits {
            return Err(error(
                ErrorKind::WrongType,
                format!("{}'s setting {key:?} can't be {v}", p.describe()),
            ));
        }
        self.apply_settings(i, &[(key, v.clone())]);
        Ok(())
    }

    fn poke(&mut self, id: PartId, v: &Val) -> HostResult<()> {
        let (i, p) = self.found(id)?;
        Self::unlocked(&p)?;
        let core = &mut self.doc.gadgets[i].core;
        match v {
            Val::Bool(b) => core.state = *b,
            Val::Number(n) => core.number = *n,
            Val::Picture(n) => core.object = Some(Object::Picture(*n)),
            Val::NoObject => core.object = None,
            Val::None | Val::Sound { .. } => {
                return Err(error(
                    ErrorKind::WrongType,
                    format!(
                        "{} can't be poked into a part (on/off, a number or a picture can)",
                        value::format(v)
                    ),
                ));
            }
        }
        let outs: Vec<usize> = core
            .knots
            .iter()
            .filter(|k| !k.is_input())
            .filter_map(|k| k.wire)
            .collect();
        for w in outs {
            if let Some(w) = self.doc.wires.get_mut(w) {
                w.changed = true;
            }
        }
        Ok(())
    }

    fn press(&mut self, id: PartId, at: Option<(i32, i32)>) -> HostResult<()> {
        let (i, p) = self.found(id)?;
        if !self.running {
            return Err(error(
                ErrorKind::NotRunning,
                "press needs the widget running (run first)",
            ));
        }
        if self.env.exclusive.is_some() {
            return Err(error(
                ErrorKind::NotRunning,
                "a part holds the run (its ending plays); no clicks now",
            ));
        }
        let (x0, y0, x1, y1) = p.rect;
        let (x, y) = at.map_or_else(|| p.centre(), |(x, y)| (x0 + x, y0 + y));
        if !(x0..x1).contains(&x) || !(y0..y1).contains(&y) {
            return Err(error(
                ErrorKind::BadArg,
                format!(
                    "{} is {} by {}; {},{} is outside it",
                    p.describe(),
                    x1 - x0,
                    y1 - y0,
                    x - x0,
                    y - y0
                ),
            ));
        }
        let (gh, gv) = (self.doc.gadgets[i].core.h, self.doc.gadgets[i].core.v);
        self.doc.with_gadget(i, &self.assets, &mut self.env, |g, io| {
            g.behavior.click(&mut g.core, io, x - gh, y - gv);
        });
        Ok(())
    }

    fn mouse(&mut self, ev: MouseEv) {
        match ev {
            MouseEv::Move(x, y) => self.mouse_move(x, y),
            MouseEv::Down(x, y, b) => self.mouse_down(x, y, button(b)),
            MouseEv::Up(x, y, b) => self.mouse_up(x, y, button(b)),
        }
    }

    fn key(&mut self, vk: u16, down: bool) {
        if down {
            self.key_down(vk);
        } else {
            self.key_up(vk);
        }
    }

    fn wait(&mut self, ms: u64) -> HostResult<()> {
        let mut left = ms;
        while left > 0 {
            self.check_time()?;
            let step = left.min(cmd::FRAME_MS);
            self.update(step, self.debug.local_time);
            self.hear();
            // The host's part of Settings ▸ Language.
            if let Some(f) = &mut self.debug.assets_for
                && let Some(lang) = self.change_lang.take()
                && let Some(assets) = f(lang)
            {
                self.set_assets(assets);
            }
            left -= step;
        }
        self.draw();
        Ok(())
    }

    fn run(&mut self) {
        self.debug.ended = None;
        Editor::run(self);
    }

    fn stop(&mut self) {
        Editor::stop(self);
    }

    fn step(&mut self, ticks: u32) -> HostResult<StepEnd> {
        if !self.running {
            return Ok(StepEnd::NotRunning);
        }
        for n in 1..=ticks {
            self.check_time()?;
            let end = self.tick_once();
            self.hear();
            if let TickEnd::Stopped { solved } = end {
                return Ok(StepEnd::Stopped { solved, at: n });
            }
        }
        Ok(StepEnd::Ran(ticks))
    }

    fn clock(&self) -> Clock {
        Clock {
            ticks: self.env.ticks,
            ms: self.env.ms,
            running: self.running,
        }
    }

    fn seed(&mut self, rng: u64) {
        self.env.rng = rng;
    }

    fn new_widget(&mut self) {
        self.debug.forget();
        self.install(Document::new(), None);
    }

    fn open_file(&mut self, path: &str) -> HostResult<()> {
        let data = self
            .files
            .read(std::path::Path::new(path))
            .map_err(|e| error(ErrorKind::Document, format!("{path}: {e}")))?;
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_owned();
        self.open_dropped_file(name, Some(path.to_owned()), data)
    }

    fn open_widget(&mut self, name: &str, bytes: &[u8]) -> HostResult<()> {
        self.open_dropped_file(name.to_owned(), None, bytes.to_vec())
    }

    fn load_puzzle(&mut self, n: i16) -> HostResult<()> {
        if !Editor::load_puzzle(self, n) {
            return Err(error(ErrorKind::Document, format!("no puzzle {n}")));
        }
        self.debug.forget();
        Ok(())
    }

    fn puzzle(&self) -> Option<PuzzleInfo> {
        (self.puzzle.number != 0 || self.puzzle.active).then_some(PuzzleInfo {
            number: self.puzzle.number,
            active: self.puzzle.active,
            ended: self.debug.ended,
        })
    }

    fn widget_bytes(&mut self) -> Vec<u8> {
        // With the tray as it is, as Save writes it.
        self.doc.tray_limits = self.tray.limits();
        self.doc.save()
    }

    fn log(&mut self, level: Level, msg: &str) {
        self.debug.sink.log(level, msg);
    }

    fn screenshot(&mut self) -> Shot {
        self.draw();
        let palette = self.assets.palette();
        let s = &self.surface;
        let (w, h) = (s.width, s.height);
        let mut px: Vec<[u8; 4]> = s
            .pixels
            .iter()
            .map(|&i| {
                let c = palette[usize::from(i)];
                [c[0], c[1], c[2], 255]
            })
            .collect();
        // The game's pointer, as the original draws it (the system arrow
        // isn't drawn: the original's screenshots don't have it either).
        if let Pointer::Game(i) = self.pointer()
            && let Some(c) = self.cursor_picture(i)
        {
            let (cw, ch) = (i32::from(c.width), i32::from(c.height));
            for cy in 0..ch {
                for cx in 0..cw {
                    let a = usize::try_from((cy * cw + cx) * 4).unwrap_or(0);
                    if c.rgba[a + 3] == 0 {
                        continue;
                    }
                    let x = self.mouse.0 - i32::from(c.hot.0) + cx;
                    let y = self.mouse.1 - i32::from(c.hot.1) + cy;
                    if (0..w).contains(&x) && (0..h).contains(&y) {
                        px[usize::try_from(y * w + x).unwrap_or(0)] = [c.rgba[a], c.rgba[a + 1], c.rgba[a + 2], 255];
                    }
                }
            }
        }
        Shot {
            width: w.unsigned_abs(),
            height: h.unsigned_abs(),
            rgba: px.into_iter().flatten().collect(),
        }
    }

    fn shot(&mut self, name: &str) -> HostResult<()> {
        let shot = Host::screenshot(self);
        self.debug
            .sink
            .shot(&file_name(name), &shot)
            .map_err(|e| error(ErrorKind::Document, e))
    }

    fn fail(&mut self, msg: &str) {
        self.debug.failures += 1;
        self.debug.sink.log(Level::Error, msg);
    }

    fn sounds(&mut self) -> Vec<SoundEvent> {
        self.debug.sounds.drain(..).collect()
    }

    fn dump(&self) -> String {
        let bar = menu::BAR_H;
        let mut out = Vec::new();
        for (i, g) in self.doc.gadgets.iter().enumerate() {
            let r = g.core.rect();
            let knots: Vec<String> = (0..g.core.knots.len())
                .filter_map(|k| g.core.knot_at(k))
                .map(|(x, y)| format!("({x},{})", y + bar))
                .collect();
            out.push(format!(
                "{i} {} box ({},{})-({},{}) knots {}",
                g.core.tag,
                r.x0,
                r.y0 + bar,
                r.x1,
                r.y1 + bar,
                knots.join(" ")
            ));
        }
        out.join("\n")
    }
}

impl Editor {
    /// A widget's file, as dropped on the window.
    fn open_dropped_file(&mut self, name: String, path: Option<String>, data: Vec<u8>) -> HostResult<()> {
        if self.busy() {
            return Err(error(ErrorKind::Document, "the editor is busy (a box is open)"));
        }
        self.debug.forget();
        self.drop_file(super::super::Dropped { name, path, data });
        Ok(())
    }
}
