//! Going back in a run (Debug ▸ Timeline): the widget as it was at GO,
//! and what came in from outside before each tick since (the mouse, the
//! keys, the clock, the microphone, clicks on parts, what scripts did,
//! panning, deliveries from the network), so any tick of the run can be
//! had again by running it anew from GO with the same input. Parts' own
//! state can't be copied, so there are no snapshots between; a seek costs
//! the ticks up to it.
//!
//! After going back, the run goes on as it went (the input recorded is
//! played) until the player gives it new input: a click on a part, a key,
//! a script's change; from that tick on the rest is dropped and a new
//! record begins.
//!
//! The timeline window has the record as a bar to click or drag on, and
//! buttons to the start, a tick back, a tick on and to the end; the graph
//! follows, dimming what lies after the cursor.

use formats::fnt::Font;

use super::super::{Editor, TickEnd, ui, worksheet};
use super::window::{self, Colours};
use crate::assets::Assets;
use crate::doc::Document;
use crate::gfx::{IRect, Surface};
use crate::lang::{tr, trf};
use crate::script::{ErrorKind, Host, HostError, HostResult, PartId, Val};
use crate::sim::{Env, Listen, SoundCmd};

/// The longest run kept: 30 minutes of ticks.
pub const MAX_TICKS: u32 = 30 * 60 * 60;
/// The most microphone input kept.
const MAX_MIC: usize = 2 << 20;
/// Seeks while dragging the bar only for runs up to this long (longer
/// ones seek when it's let go).
const LIVE_DRAG: u32 = 6000;

/// What the running widget reads from outside, as a tick sees it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outside {
    mouse: (i32, i32),
    mouse_down: bool,
    right_down: bool,
    cursor_screen: (i32, i32),
    keys: Vec<u16>,
    keys_held: Vec<u16>,
    local_time: (u32, u32, u32),
}

impl Outside {
    fn of(env: &Env) -> Outside {
        Outside {
            mouse: env.mouse,
            mouse_down: env.mouse_down,
            right_down: env.right_down,
            cursor_screen: env.cursor_screen,
            keys: env.keys.clone(),
            keys_held: env.keys_held.clone(),
            local_time: env.local_time,
        }
    }

    fn put(&self, env: &mut Env) {
        env.mouse = self.mouse;
        env.mouse_down = self.mouse_down;
        env.right_down = self.right_down;
        env.cursor_screen = self.cursor_screen;
        env.keys.clone_from(&self.keys);
        env.keys_held.clone_from(&self.keys_held);
        env.local_time = self.local_time;
    }
}

/// Something that happened between two ticks.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A click on part `id` at `at` in it, the pointer at `mouse` (and
    /// `cursor` on the screen).
    Click {
        id: u32,
        at: (i32, i32),
        mouse: (i32, i32),
        cursor: (i32, i32),
    },
    /// A script worked part `id`'s control.
    SetValue { id: u32, v: Val },
    /// A script wrote part `id`'s own value.
    Poke { id: u32, v: Val },
    /// The worksheet panned.
    Pan((i32, i32)),
    /// The microphone's bytes from `at` on, and where it is then.
    Mic { at: u32, bytes: Vec<u8>, pos: Option<u32> },
    /// Something delivered to part `id` from outside (the network).
    External { id: u32, payload: Vec<u8> },
}

/// What came in before a tick: the outside (if it changed) and events.
#[derive(Debug, Clone, Default, PartialEq)]
struct Frame {
    outside: Option<Outside>,
    events: Vec<Event>,
}

/// What the timeline is doing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Doing {
    /// A seek re-simulates: no recording, no probes, no breakpoints.
    pub seeking: bool,
    /// Recorded events being played: they aren't new input.
    pub applying: bool,
    /// GO is a seek's, not the player's: the record stays.
    pub resuming: bool,
}

/// The run as it was at GO.
struct Start {
    doc: Vec<u8>,
    rng: u64,
    listen: Listen,
}

/// How a delivery from outside is made again when a seek replays it:
/// the document, the resources, the widget's surroundings, the part's id
/// and what came.
pub type Redeliver = Box<dyn FnMut(&mut Document, &Assets, &mut Env, u32, &[u8])>;

/// The record of the run since GO.
#[derive(Default)]
pub struct Timeline {
    start: Option<Start>,
    frames: Vec<Frame>,
    /// The ticks recorded (the cursor is the widget's tick count).
    pub end: u32,
    /// What came in since the last tick, for the next frame.
    pending: Vec<Event>,
    /// The outside as last recorded or played.
    last: Outside,
    /// Where the first part was after the last tick (a pan moves all).
    pan_ref: Option<(i32, i32)>,
    /// Where the microphone was after the last tick, and how much of it
    /// is kept.
    mic_seen: Option<u32>,
    mic_bytes: usize,
    /// Too long to keep: no going back.
    pub full: bool,
    /// What it's doing now.
    pub doing: Doing,
    /// The tick going on replays the record (the probes have it).
    pub replayed: bool,
    /// How deliveries are made again.
    pub redeliver: Option<Redeliver>,
    /// The bar being dragged.
    drag: Option<u32>,
}

impl Timeline {
    /// Whether the tick at `t` replays what was recorded.
    fn plays(&self, t: u32) -> bool {
        self.start.is_some() && t < self.end && (t as usize) < self.frames.len()
    }
}

impl Editor {
    /// GO: the widget as it is kept (unless a seek starts it again), the
    /// tools' run state reset.
    pub(in crate::editor) fn debug_go(&mut self) {
        if self.debug.timeline.doing.resuming {
            self.debug.run.paused = false;
            self.debug.run.carry = 0;
            return;
        }
        self.debug.on_go();
        if !self.debugging() {
            self.debug.timeline.start = None;
            return;
        }
        let tl = &mut self.debug.timeline;
        tl.start = Some(Start {
            doc: self.doc.save(),
            rng: self.env.rng,
            listen: self.env.listen.clone(),
        });
        tl.frames.clear();
        tl.end = 0;
        tl.pending.clear();
        tl.full = false;
        tl.mic_bytes = 0;
        tl.last = Outside::default();
        tl.pan_ref = None;
        tl.mic_seen = None;
    }

    fn pan_pos(&self) -> Option<(i32, i32)> {
        self.doc.gadgets.first().map(|g| (g.core.h, g.core.v))
    }

    /// Before a tick: what came in recorded (or, going over a recorded
    /// stretch, played again).
    pub(super) fn timeline_before_tick(&mut self) {
        let t = self.env.ticks;
        if t == 0 {
            self.debug.timeline.pan_ref = self.pan_pos();
            self.debug.timeline.mic_seen = self.env.listen.pos;
        }
        if self.debug.timeline.start.is_none() || self.debug.timeline.full {
            return;
        }
        // New keys while playing: the player's, so the run goes its own
        // way from here.
        if self.debug.timeline.plays(t) && !self.debug.timeline.doing.seeking && !self.env.keys.is_empty() {
            self.branch();
        }
        if self.debug.timeline.plays(t) {
            self.play_frame(t);
        } else {
            self.record_frame(t);
        }
    }

    /// After a tick: where it is, for the next.
    pub(super) fn timeline_after_tick(&mut self) {
        let tl = &mut self.debug.timeline;
        if tl.start.is_none() {
            return;
        }
        tl.pan_ref = self.doc.gadgets.first().map(|g| (g.core.h, g.core.v));
        tl.mic_seen = self.env.listen.pos;
        if !tl.full && !tl.doing.seeking {
            tl.end = tl.end.max(self.env.ticks);
        }
    }

    fn record_frame(&mut self, t: u32) {
        let mut events = std::mem::take(&mut self.debug.timeline.pending);
        // A pan since the last tick.
        if let (Some(now), Some(was)) = (self.pan_pos(), self.debug.timeline.pan_ref)
            && now != was
        {
            events.insert(0, Event::Pan((now.0 - was.0, now.1 - was.1)));
        }
        // The microphone's new bytes.
        let l = &self.env.listen;
        if l.pos != self.debug.timeline.mic_seen {
            let from = self
                .debug
                .timeline
                .mic_seen
                .filter(|&s| l.pos.is_some_and(|p| p >= s))
                .unwrap_or(0);
            let to = l.pos.unwrap_or(from);
            let bytes = l
                .buf
                .get(from as usize..(to as usize).min(l.buf.len()))
                .map(<[u8]>::to_vec)
                .unwrap_or_default();
            self.debug.timeline.mic_bytes += bytes.len();
            events.push(Event::Mic {
                at: from,
                bytes,
                pos: l.pos,
            });
        }
        let outside = Outside::of(&self.env);
        let tl = &mut self.debug.timeline;
        let changed = outside != tl.last;
        if changed {
            tl.last = outside.clone();
        }
        tl.frames.truncate(t as usize);
        tl.frames.push(Frame {
            outside: changed.then_some(outside),
            events,
        });
        if t >= MAX_TICKS || tl.mic_bytes > MAX_MIC {
            tl.full = true;
            tl.frames = Vec::new();
            self.debug.console.add(
                super::LineKind::Warn,
                tr("The run is too long to go back in; the timeline stops."),
            );
        }
    }

    fn play_frame(&mut self, t: u32) {
        let Some(frame) = self.debug.timeline.frames.get(t as usize).cloned() else {
            return;
        };
        self.debug.timeline.doing.applying = true;
        for e in &frame.events {
            self.play_event(e);
        }
        self.debug.timeline.doing.applying = false;
        if let Some(o) = frame.outside {
            self.debug.timeline.last = o;
        }
        self.debug.timeline.last.put(&mut self.env);
    }

    fn play_event(&mut self, e: &Event) {
        let index = |e: &Editor, id: u32| e.doc.gadgets.iter().position(|g| g.core.id == id);
        match e {
            Event::Click { id, at, mouse, cursor } => {
                self.env.mouse = *mouse;
                self.env.cursor_screen = *cursor;
                self.env.mouse_down = true;
                if let Some(i) = index(self, *id) {
                    self.doc.with_gadget(i, &self.assets, &mut self.env, |g, io| {
                        g.behavior.click(&mut g.core, io, at.0, at.1);
                    });
                }
            }
            Event::SetValue { id, v } => {
                let _ = self.set_value(PartId(*id), v);
            }
            Event::Poke { id, v } => {
                let _ = self.poke(PartId(*id), v);
            }
            Event::Pan(d) => worksheet::offset_all(&mut self.doc, *d),
            Event::Mic { at, bytes, pos } => {
                let l = &mut self.env.listen;
                let at = *at as usize;
                if at + bytes.len() <= l.buf.len() {
                    l.buf[at..at + bytes.len()].copy_from_slice(bytes);
                }
                l.pos = *pos;
            }
            Event::External { id, payload } => {
                if let Some(f) = &mut self.debug.timeline.redeliver {
                    f(&mut self.doc, &self.assets, &mut self.env, *id, payload);
                }
            }
        }
    }

    /// The player gave new input while the run went over a recorded
    /// stretch: the rest of the record goes, the run is recorded on.
    fn branch(&mut self) {
        let t = self.env.ticks;
        let tl = &mut self.debug.timeline;
        if tl.start.is_none() || t >= tl.end {
            return;
        }
        tl.frames.truncate(t as usize);
        tl.end = t;
        self.debug.observed.truncate(t);
    }

    /// Records `e`, which just happened (while the widget runs, not
    /// replayed); new input over a recorded stretch ends it there.
    pub(super) fn timeline_event(&mut self, e: Event) {
        if !self.running || !self.debugging() {
            return;
        }
        let tl = &self.debug.timeline;
        if tl.start.is_none() || tl.doing.applying || tl.doing.seeking || tl.full {
            return;
        }
        self.branch();
        self.debug.timeline.pending.push(e);
    }

    /// A click on running part `g` at `p` (worksheet coordinates), for the
    /// timeline.
    pub(in crate::editor) fn debug_record_click(&mut self, g: usize, p: (i32, i32)) {
        let Some(core) = self.doc.gadgets.get(g).map(|g| &g.core) else {
            return;
        };
        let e = Event::Click {
            id: core.id,
            at: (p.0 - core.h, p.1 - core.v),
            mouse: self.env.mouse,
            cursor: self.env.cursor_screen,
        };
        self.timeline_event(e);
    }

    /// Something came to part `id` from outside the widget (a network
    /// delivery), which a seek must deliver again (through
    /// [`Editor::set_redeliver`]): recorded for the timeline.
    pub fn record_external(&mut self, id: u32, payload: &[u8]) {
        self.timeline_event(Event::External {
            id,
            payload: payload.to_vec(),
        });
    }

    /// How deliveries from outside ([`Editor::record_external`]) are made
    /// again when a seek replays them.
    pub fn set_redeliver(&mut self, f: Redeliver) {
        self.debug.timeline.redeliver = Some(f);
    }

    /// Goes to tick `t` of the run recorded since GO: the widget run anew
    /// from GO to there with the same input, quietly, then held. From
    /// editing (after the run), the widget starts again for it, if it
    /// hasn't been changed since.
    ///
    /// # Errors
    /// Nothing recorded, a run too long, or a widget changed since.
    pub(super) fn timeline_seek(&mut self, t: u32) -> HostResult<()> {
        let refuse = |m: &str| Err(HostError::new(ErrorKind::BadArg, m));
        if !self.debugging() {
            return Err(crate::script::no_tools());
        }
        let Some(start) = &self.debug.timeline.start else {
            return refuse(tr("nothing is recorded yet: GO first"));
        };
        if self.debug.timeline.full {
            return refuse(tr("the run was too long to go back in"));
        }
        let t = t.min(self.debug.timeline.end);
        if !self.running {
            if self.doc.save() != start.doc {
                return refuse(tr("the widget changed since that run"));
            }
            self.debug.timeline.doing.resuming = true;
            self.run();
            self.debug.timeline.doing.resuming = false;
        }
        let Some(start) = &self.debug.timeline.start else {
            return refuse(tr("nothing is recorded yet: GO first"));
        };
        let Ok(doc) = Document::load(&self.assets, &start.doc) else {
            return refuse("the widget recorded at GO doesn't read");
        };
        let (rng, listen) = (start.rng, start.listen.clone());
        let sounds = self.env.sounds.len();
        self.doc = doc;
        self.env.ticks = 0;
        self.env.ms = 0;
        self.env.exclusive = None;
        self.env.end_run = None;
        self.env.rng = rng;
        self.env.listen = listen;
        self.env.keys.clear();
        self.ms_rest = 0;
        self.debug.timeline.last = Outside::default();
        self.doc.start(&self.assets, &mut self.env);
        self.debug.timeline.doing.seeking = true;
        for _ in 0..t {
            if let TickEnd::Stopped { .. } = self.tick_once() {
                break;
            }
        }
        self.debug.timeline.doing.seeking = false;
        self.env.sounds.truncate(sounds);
        self.env.sounds.push(SoundCmd::StopAll);
        if self.running {
            self.debug.run.paused = true;
        }
        self.debug.time = super::graph::Time {
            cursor: self.env.ticks,
            end: self.debug.timeline.end,
        };
        Ok(())
    }

    /// A seek asked for by the tools' windows: an error goes to the
    /// console.
    pub(super) fn timeline_seek_click(&mut self, t: u32) {
        if let Err(e) = self.timeline_seek(t) {
            self.debug.console.add(super::LineKind::Error, &e.message);
        }
    }
}

/// The timeline window's parts: the four buttons, the bar.
pub struct Layout {
    pub first: IRect,
    pub back: IRect,
    pub on: IRect,
    pub last: IRect,
    pub bar: IRect,
    pub text: (i32, i32),
}

#[must_use]
pub fn layout(r: IRect, font: &Font) -> Layout {
    let c = window::client(r).inset(4);
    let bh = i32::from(font.height) + 8;
    let b = |i: i32| IRect::sized(c.x0 + i * 30, c.y0, 28, bh);
    let bar = IRect::new(c.x0, c.y0 + bh + 6, c.x1, (c.y0 + bh + 6 + 18).min(c.y1));
    Layout {
        first: b(0),
        back: b(1),
        on: b(2),
        last: b(3),
        bar,
        text: (c.x0 + 4 * 30 + 6, c.y0 + 4),
    }
}

/// A small filled triangle pointing left or right, its point at `x`,
/// `h` high.
fn triangle(s: &mut Surface, x: i32, cy: i32, h: i32, left: bool, c: u8) {
    for i in 0..=h / 2 {
        let px = if left { x + i } else { x - i };
        s.fill(IRect::new(px, cy - i, px + 1, cy + i + 1), c);
    }
}

/// What the window shows: the record's end and the cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shown {
    pub cursor: u32,
    pub end: u32,
    pub full: bool,
    pub any: bool,
}

/// Draws the timeline window `r`.
pub fn draw(s: &mut Surface, font: &Font, r: IRect, col: Colours, st: Shown) {
    let l = layout(r, font);
    for (b, enabled) in [
        (l.first, st.cursor > 0),
        (l.back, st.cursor > 0),
        (l.on, st.any),
        (l.last, st.cursor < st.end),
    ] {
        ui::button(s, font, b, "", false, false, enabled);
    }
    let fg = |on: bool| if on { ui::BLACK } else { ui::SHADOW };
    let cy = |b: IRect| i32::midpoint(b.y0, b.y1);
    // |◀ ◀1 ▶1 ▶|
    let c = fg(st.cursor > 0);
    s.fill(
        IRect::new(l.first.x0 + 8, cy(l.first) - 4, l.first.x0 + 10, cy(l.first) + 5),
        c,
    );
    triangle(s, l.first.x0 + 11, cy(l.first), 8, true, c);
    triangle(s, l.back.x0 + 6, cy(l.back), 8, true, c);
    ui::label(s, font, l.back.x0 + 16, l.back.y0 + 3, "1", c);
    let c = fg(st.any);
    ui::label(s, font, l.on.x0 + 6, l.on.y0 + 3, "1", c);
    triangle(s, l.on.x1 - 7, cy(l.on), 8, false, c);
    let c = fg(st.cursor < st.end);
    triangle(s, l.last.x1 - 11, cy(l.last), 8, false, c);
    s.fill(
        IRect::new(l.last.x1 - 10, cy(l.last) - 4, l.last.x1 - 8, cy(l.last) + 5),
        c,
    );
    let text = if st.full {
        tr("too long to go back in").to_owned()
    } else if st.any {
        trf("tick {} of {}", &[&st.cursor.to_string(), &st.end.to_string()])
    } else {
        tr("GO to record a run").to_owned()
    };
    ui::label(s, font, l.text.0, l.text.1, &text, ui::BLACK);
    // The bar: the record, the part replayed dimmed past the cursor.
    ui::sunken(s, l.bar);
    let inner = l.bar.inset(2);
    s.fill(inner, ui::WHITE);
    if st.any && st.end > 0 {
        let x = |t: u32| {
            inner.x0
                + i32::try_from(u64::from(t) * u64::try_from(inner.width()).unwrap_or(0) / u64::from(st.end))
                    .unwrap_or(0)
        };
        let cx = x(st.cursor);
        s.fill(IRect::new(inner.x0, inner.y0 + 3, cx, inner.y1 - 3), col.blue);
        s.fill(IRect::new(cx, inner.y0 + 3, inner.x1, inner.y1 - 3), col.light_grey);
        s.fill(IRect::new(cx - 2, inner.y0, cx + 3, inner.y1), ui::BLACK);
        s.fill(IRect::new(cx - 1, inner.y0 + 1, cx + 2, inner.y1 - 1), ui::FACE);
    }
}

impl Editor {
    /// What the timeline window shows now.
    pub(super) fn timeline_shown(&self) -> Shown {
        let tl = &self.debug.timeline;
        Shown {
            cursor: if self.running { self.env.ticks } else { tl.end },
            end: tl.end,
            full: tl.full,
            any: tl.start.is_some(),
        }
    }

    /// A click in the timeline window `r` at `p`: a button, or the bar
    /// (dragged on). Whether it goes on as a drag.
    pub(super) fn timeline_click(&mut self, r: IRect, p: (i32, i32)) -> bool {
        let Some(font) = ui::font(&self.assets) else {
            return false;
        };
        let l = layout(r, font);
        let st = self.timeline_shown();
        if !st.any {
            return false;
        }
        let at = |b: IRect| b.contains(p.0, p.1);
        if at(l.first) {
            self.timeline_seek_click(0);
        } else if at(l.back) {
            self.timeline_seek_click(st.cursor.saturating_sub(1));
        } else if at(l.on) {
            self.command(super::control::cmd::STEP_TICK);
        } else if at(l.last) {
            self.timeline_seek_click(st.end);
        } else if at(l.bar) {
            self.timeline_drag(r, p, false);
            return true;
        }
        false
    }

    /// The bar pressed or dragged at `p` (`done`: let go): the cursor
    /// there, the run sought to it (while dragging only for short runs).
    pub(super) fn timeline_drag(&mut self, r: IRect, p: (i32, i32), done: bool) {
        let Some(font) = ui::font(&self.assets) else { return };
        let inner = layout(r, font).bar.inset(2);
        let end = self.debug.timeline.end;
        let w = i64::from(inner.width().max(1));
        let dx = i64::from((p.0 - inner.x0).clamp(0, inner.width()));
        let t = u32::try_from(dx * i64::from(end) / w).unwrap_or(0);
        if done || end <= LIVE_DRAG {
            if self.debug.timeline.drag != Some(t) || done {
                self.timeline_seek_click(t);
            }
            self.debug.timeline.drag = if done { None } else { Some(t) };
        } else {
            self.debug.timeline.drag = Some(t);
        }
    }
}
