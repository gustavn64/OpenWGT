//! What the tools keep an eye on: probes (a knot's or a part's values
//! recorded tick by tick, for the graph), watches (values listed as they
//! are now) and data breakpoints (the run held the tick a condition comes
//! to hold, or a value changes). Recorded after each tick the player sees,
//! not while the timeline re-simulates.

use super::super::Editor;
use super::overlay::{knot_name, who};
use super::window::Kind;
use crate::lang::{tr, trf};
use crate::plot::Sample;
use crate::script::{Cond, ErrorKind, Host, HostError, HostResult, KnotSel, Selector, Val, select, value};

/// The most changes a probe keeps (the oldest go).
const MAX_CHANGES: usize = 100_000;
/// The most probes, watches and breakpoints.
pub const MAX_EACH: usize = 16;

/// A value recorded as the widget runs.
#[derive(Debug, Clone, PartialEq)]
pub struct Probe {
    pub sel: Selector,
    /// What the graph calls it.
    pub name: String,
    /// Its colour's turn ([`super::window::Colours::series`]).
    pub colour: usize,
    /// The changes since GO: from which tick on, what.
    pub changes: Vec<(u32, Sample)>,
    last: Option<Val>,
}

/// A value listed as it is now.
#[derive(Debug, Clone, PartialEq)]
pub struct Watch {
    pub sel: Selector,
    pub name: String,
    /// As last shown.
    pub value: String,
}

/// A condition that holds the run.
#[derive(Debug, Clone, PartialEq)]
pub struct Break {
    pub cond: Cond,
    pub text: String,
    /// The values `changed` compares with: the tick before's.
    start: Vec<(Selector, Val)>,
    /// Whether it held after the last tick (it holds the run as it
    /// comes to hold, not each tick it holds).
    held: bool,
    pub hits: u32,
}

/// Everything the tools keep an eye on.
#[derive(Debug, Clone, Default)]
pub struct Observed {
    pub probes: Vec<Probe>,
    pub watches: Vec<Watch>,
    pub breaks: Vec<Break>,
    next_colour: usize,
    /// A breakpoint held the run in the last tick.
    pub broke: bool,
}

/// `v` for the graph.
#[must_use]
pub fn sample(v: &Val) -> Sample {
    match v {
        Val::None => Sample::Nothing,
        Val::Bool(b) => Sample::Bool(*b),
        Val::Number(n) => Sample::Number(*n),
        other => Sample::Object(super::overlay::show(other)),
    }
}

impl Observed {
    /// Forgets what was recorded (a new run, or the timeline gone back).
    pub fn clear_history(&mut self) {
        for p in &mut self.probes {
            p.changes.clear();
            p.last = None;
        }
    }

    /// GO: the records start again, the breakpoints from the start.
    pub fn go(&mut self) {
        self.clear_history();
        self.broke = false;
        for b in &mut self.breaks {
            b.held = false;
            b.start.clear();
        }
    }

    /// The probes' records after tick `t` dropped (the run goes another
    /// way from there).
    pub fn truncate(&mut self, t: u32) {
        for p in &mut self.probes {
            let n = p.changes.partition_point(|(c, _)| *c <= t);
            p.changes.truncate(n);
            p.last = None;
        }
    }
}

fn full(what: &str) -> HostError {
    HostError::new(ErrorKind::BadArg, trf("at most {} {}", &[&MAX_EACH.to_string(), what]))
}

impl Editor {
    /// What `sel` stands for, as the tools call it: `"Lamp".in`,
    /// `Switch 3.out`.
    pub(super) fn sel_name(&self, sel: &Selector) -> String {
        let Ok(id) = self.resolve(&sel.part) else {
            return select::show(sel);
        };
        let Ok(i) = self.doc.gadgets.iter().position(|g| g.core.id == id.0).ok_or(()) else {
            return select::show(sel);
        };
        let p = self.info(i);
        let k = match sel.knot {
            None => p
                .knots
                .iter()
                .position(|k| !k.input)
                .or_else(|| p.knots.iter().position(|k| k.input)),
            Some(KnotSel::Index(n)) => Some(usize::from(n)),
            Some(k) => select::knot(&p, k).ok().map(|k| usize::from(k.index)),
        };
        match k {
            Some(k) => format!("{}.{}", who(&p), knot_name(&p, k)),
            None => who(&p),
        }
    }

    /// `sel` with its part by id, so it stays the same part as names and
    /// labels change.
    fn pinned(&self, sel: &Selector) -> HostResult<Selector> {
        let id = self.resolve(&sel.part)?;
        let pinned = Selector {
            part: crate::script::PartSel::Id(id.0),
            knot: sel.knot,
        };
        self.value(&pinned)?;
        Ok(pinned)
    }

    fn tools_on(&self) -> HostResult<()> {
        if self.debugging() {
            Ok(())
        } else {
            Err(crate::script::no_tools())
        }
    }

    /// A probe on `sel` (if it hasn't one), and the graph shown if `show`.
    pub(super) fn add_probe(&mut self, sel: &Selector, show: bool) -> HostResult<()> {
        self.tools_on()?;
        let sel = self.pinned(sel)?;
        if !self.debug.observed.probes.iter().any(|p| p.sel == sel) {
            if self.debug.observed.probes.len() >= MAX_EACH {
                return Err(full(tr("probes")));
            }
            let name = self.sel_name(&sel);
            let o = &mut self.debug.observed;
            let colour = o.next_colour;
            o.next_colour += 1;
            let mut p = Probe {
                sel,
                name,
                colour,
                changes: Vec::new(),
                last: None,
            };
            // What it is now, from now on.
            if self.running {
                let t = self.env.ticks;
                if let Ok(v) = self.value(&p.sel) {
                    p.changes.push((t, sample(&v)));
                    p.last = Some(v);
                }
            }
            self.debug.observed.probes.push(p);
        }
        if show {
            self.open_window(Kind::Graph);
        }
        Ok(())
    }

    /// `sel` in the watch list (opened).
    pub(super) fn add_watch(&mut self, sel: &Selector) -> HostResult<()> {
        self.tools_on()?;
        let sel = self.pinned(sel)?;
        if !self.debug.observed.watches.iter().any(|w| w.sel == sel) {
            if self.debug.observed.watches.len() >= MAX_EACH {
                return Err(full(tr("watches")));
            }
            let name = self.sel_name(&sel);
            self.debug.observed.watches.push(Watch {
                sel,
                name,
                value: String::new(),
            });
        }
        self.open_window(Kind::Watch);
        Ok(())
    }

    /// A breakpoint on `c` (listed in the watch list, opened).
    pub(super) fn add_break(&mut self, c: &Cond) -> HostResult<()> {
        self.tools_on()?;
        if self.debug.observed.breaks.len() >= MAX_EACH {
            return Err(full(tr("breakpoints")));
        }
        let start = Host::snapshot(self, c)?;
        let held = self.running && !matches!(c, Cond::Changed(_)) && self.eval(c, &start)?;
        self.debug.observed.breaks.push(Break {
            cond: c.clone(),
            text: value::show_cond(c),
            start,
            held,
            hits: 0,
        });
        self.open_window(Kind::Watch);
        Ok(())
    }

    /// Debug ▸ Pause or Resume, as a script asks.
    pub(super) fn set_paused(&mut self, on: bool) -> HostResult<()> {
        self.tools_on()?;
        if !self.running {
            return Err(HostError::new(
                ErrorKind::NotRunning,
                "the widget doesn't run (run first)",
            ));
        }
        self.debug.run.paused = on;
        Ok(())
    }

    /// Records the probes' values now, at tick `t`.
    pub(super) fn record_probes(&mut self, t: u32) {
        for i in 0..self.debug.observed.probes.len() {
            let v = self.value(&self.debug.observed.probes[i].sel).unwrap_or(Val::None);
            let p = &mut self.debug.observed.probes[i];
            if p.last.as_ref() != Some(&v) {
                p.changes.push((t, sample(&v)));
                if p.changes.len() > MAX_CHANGES {
                    p.changes.drain(..MAX_CHANGES / 10);
                }
                p.last = Some(v);
            }
        }
    }

    /// At GO: what the breakpoints' `changed` compare with first.
    pub(super) fn break_starts(&mut self) {
        for i in 0..self.debug.observed.breaks.len() {
            let c = self.debug.observed.breaks[i].cond.clone();
            self.debug.observed.breaks[i].start = Host::snapshot(self, &c).unwrap_or_default();
        }
    }

    /// After a tick the player sees: the breakpoints checked; one coming
    /// to hold holds the run, says so in the console, and the ticks still
    /// due this frame wait.
    pub(super) fn check_breaks(&mut self) {
        self.debug.observed.broke = false;
        let mut hit = Vec::new();
        for i in 0..self.debug.observed.breaks.len() {
            let b = &self.debug.observed.breaks[i];
            let (c, start) = (b.cond.clone(), b.start.clone());
            let first = start.is_empty() && has_changed(&c);
            let holds = !first && self.eval(&c, &start).unwrap_or(false);
            let fresh = Host::snapshot(self, &c).unwrap_or_default();
            let b = &mut self.debug.observed.breaks[i];
            if holds && !b.held {
                b.hits += 1;
                hit.push(b.text.clone());
            }
            b.held = holds;
            b.start = fresh;
        }
        if hit.is_empty() {
            return;
        }
        self.debug.observed.broke = true;
        self.debug.run.paused = true;
        self.ms_rest = 0;
        let t = self.env.ticks.to_string();
        for text in hit {
            let msg = trf("Held at tick {}: {}", &[&t, &text]);
            self.debug.console.add(super::LineKind::Warn, &msg);
        }
    }

    /// The watch list's values, as they are now.
    pub(super) fn watch_values(&mut self) {
        for i in 0..self.debug.observed.watches.len() {
            let v = self
                .value(&self.debug.observed.watches[i].sel)
                .map_or_else(|e| e.message, |v| value::format(&v));
            self.debug.observed.watches[i].value = v;
        }
    }
}

/// Whether `c` asks after a change (which needs a tick before to compare
/// with).
fn has_changed(c: &Cond) -> bool {
    match c {
        Cond::Changed(_) => true,
        Cond::Not(c) => has_changed(c),
        Cond::All(cs) | Cond::Any(cs) => cs.iter().any(has_changed),
        _ => false,
    }
}
