//! The Oscilloscope (`Scop`): what goes into its inputs A and B, numbers
//! or ON/OFF signals, drawn as two traces on a dark screen that keeps
//! scrolling as the widget runs (the Strip Chart, `StCh`, stops when its
//! chart is full). An ON at its third input, "freeze", holds the picture;
//! its output gives what goes into A, of the same kind, so that it can sit
//! in a wire.
//!
//! Each column of the screen is one to eight ticks (the time scale); a
//! column keeps the lowest, highest, first and last values of each input
//! in its time. Numbers share one range, the one the settings fix or,
//! automatically, the one the columns on the screen need (rounded out to
//! steps of 1, 2 or 5 times a power of ten); an ON/OFF signal is a square
//! wave in a band of its own, A's above B's. Only the settings are saved:
//! the columns are the run's, cleared at GO.
//!
//! Settings: texts 0 and 1, From and To; radio group 0, the range
//! (Automatic, From/to); radio group 1, the time scale (1×, 2×, 4×, 8×
//! ticks a column); check 0, dots only (else lines).

// Rows and pixels from floats on purpose, each checked to its range.
#![allow(clippy::cast_possible_truncation, clippy::float_cmp)]

use std::collections::VecDeque;

use crate::gadget::{Behavior, Core, DrawCx, Settings, Tag, Value};
use crate::gadgets::common::{scan_f64, truthy};
use crate::gfx::IRect;
use crate::sim::Io;

/// The Oscilloscope's tag.
pub const SCOPE: Tag = Tag::new(b"Scop");

/// The screen's columns: the newest on the right once it's full.
pub const COLUMNS: usize = 100;
/// The screen's rows, 0 at the bottom.
pub const ROWS: i32 = 64;
/// The screen in the part's picture (`h`, `v`, width, height), where the
/// traces go (`tools/openparts/draw.py` draws it there).
pub const PLOT: (i32, i32, i32, i32) = (25, 9, 100, ROWS);
/// Ticks a column, by the time scale's radio button.
pub const SCALES: [u32; 4] = [1, 2, 4, 8];
/// The traces' colours: A yellow, B blue (as the picture's letters).
pub const COLOURS: [[u8; 3]; 2] = [[255, 230, 40], [60, 220, 255]];
/// The range's numbers on the screen: the grid's green.
const LABEL_COLOUR: [u8; 3] = [52, 110, 72];
const LABEL_FACE: &str = "Small Fonts";
const LABEL_SIZE: u16 = 9;
/// The rows of an ON/OFF signal (off, on): A's band above B's.
pub const LEVELS: [(i32, i32); 2] = [(40, 56), (8, 24)];

/// The knots: inputs A, B and freeze, the output.
const A: usize = 0;
const B: usize = 1;
const FREEZE: usize = 2;
const OUT: usize = 3;

/// What an input gives, as the Oscilloscope reads it.
#[derive(Debug, Clone, Copy)]
pub enum Input {
    Bool(bool),
    Number(f64),
}

impl Input {
    /// Input `k` of `core`: an ON/OFF signal, a number, or an object as
    /// whether there is one; `None` unwired or for a part that gives
    /// nothing there.
    fn read(io: &Io<'_>, core: &Core, k: usize) -> Option<Input> {
        if !io.wired(core, k) {
            return None;
        }
        match io.input(core, k) {
            Value::Bool(b) => Some(Input::Bool(b)),
            Value::Number(x) => Some(Input::Number(x)),
            Value::Object(o) => Some(Input::Bool(o.is_some())),
            Value::None => None,
        }
    }

    /// The same value, the same kind (a NaN is a NaN).
    fn same(a: Option<Input>, b: Option<Input>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(Input::Bool(x)), Some(Input::Bool(y))) => x == y,
            (Some(Input::Number(x)), Some(Input::Number(y))) => x.to_bits() == y.to_bits(),
            _ => false,
        }
    }
}

/// One input over one column's ticks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Sample {
    /// Nothing came (unwired, or not a number).
    #[default]
    None,
    /// An ON/OFF signal: its first and last, and whether it was off or
    /// on at all.
    Bool {
        first: bool,
        last: bool,
        off: bool,
        on: bool,
    },
    /// Numbers: the first, the last, the lowest and the highest.
    Number { first: f64, last: f64, min: f64, max: f64 },
}

impl Sample {
    /// `v` taken in; one of another kind starts the column's sample again.
    /// A number that isn't finite is left out.
    pub fn add(&mut self, v: Option<Input>) {
        *self = match (*self, v) {
            (s, None) => s,
            (s, Some(Input::Number(x))) if !x.is_finite() => s,
            (Sample::Bool { first, off, on, .. }, Some(Input::Bool(b))) => Sample::Bool {
                first,
                last: b,
                off: off || !b,
                on: on || b,
            },
            (Sample::Number { first, min, max, .. }, Some(Input::Number(x))) => Sample::Number {
                first,
                last: x,
                min: min.min(x),
                max: max.max(x),
            },
            (_, Some(Input::Bool(b))) => Sample::Bool {
                first: b,
                last: b,
                off: !b,
                on: b,
            },
            (_, Some(Input::Number(x))) => Sample::Number {
                first: x,
                last: x,
                min: x,
                max: x,
            },
        };
    }
}

/// One column of the screen: A's and B's samples.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Column {
    pub traces: [Sample; 2],
}

/// The settings' range: From and to, if fixed and both read; swapped if
/// the wrong way round, widened by 1 each way if the same.
#[must_use]
pub fn fixed_range(s: &Settings) -> Option<(f64, f64)> {
    if s.radios[0] == 0 {
        return None;
    }
    let lo = scan_f64(s.label(0)).filter(|x| x.is_finite())?;
    let hi = scan_f64(s.label(1)).filter(|x| x.is_finite())?;
    let (lo, hi) = if lo > hi { (hi, lo) } else { (lo, hi) };
    Some(if lo == hi { (lo - 1.0, hi + 1.0) } else { (lo, hi) })
}

/// A range that holds `lo` to `hi`, its ends whole steps of 1, 2 or 5
/// times a power of ten, about four steps; one value gets 1 (or a
/// billionth of itself) on either side.
#[must_use]
pub fn nice_range(lo: f64, hi: f64) -> (f64, f64) {
    let (lo, hi) = if lo < hi {
        (lo, hi)
    } else {
        let d = (lo.abs() * 1e-9).max(1.0);
        (lo - d, hi + d)
    };
    let rough = (hi - lo) / 4.0;
    let power = 10f64.powf(rough.log10().floor());
    let m = rough / power;
    let step = power
        * if m <= 1.0 {
            1.0
        } else if m <= 2.0 {
            2.0
        } else if m <= 5.0 {
            5.0
        } else {
            10.0
        };
    let (l, h) = ((lo / step).floor() * step, (hi / step).ceil() * step);
    if step.is_finite() && step > 0.0 && l.is_finite() && h.is_finite() && l < h {
        (l, h)
    } else {
        (lo, hi)
    }
}

/// A number as the screen shows the range: whole numbers whole, others
/// with four figures at most.
#[must_use]
pub fn range_label(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let a = x.abs();
    if (1e-3..1e7).contains(&a) {
        if x.fract() == 0.0 {
            return format!("{x:.0}");
        }
        let digits = (3 - a.log10().floor() as i32).clamp(0, 6);
        let s = format!("{x:.*}", usize::try_from(digits).unwrap_or(0));
        let s = s.trim_end_matches('0').trim_end_matches('.');
        return s.to_owned();
    }
    format!("{x:.2e}")
}

/// The Oscilloscope (`Scop`).
#[derive(Debug, Default)]
pub struct Oscilloscope {
    /// The screen's columns, oldest first, at most [`COLUMNS`].
    cols: VecDeque<Column>,
    /// The run's column the newest one is (its ticks / the time scale).
    at: Option<u32>,
    /// Held by the freeze input.
    frozen: bool,
    /// What the output gives: A as last read.
    out: Option<Input>,
    /// The settings, as read at GO: ticks a column, the fixed range, dots.
    scale: u32,
    fixed: Option<(f64, f64)>,
    dots: bool,
}

impl Oscilloscope {
    /// The screen's columns, oldest first.
    #[must_use]
    pub fn columns(&self) -> &VecDeque<Column> {
        &self.cols
    }

    /// Whether the freeze input holds the picture.
    #[must_use]
    pub fn frozen(&self) -> bool {
        self.frozen
    }

    /// The numbers' range: the fixed one, else the one the columns on the
    /// screen need ([`nice_range`]); none without numbers.
    #[must_use]
    pub fn range(&self) -> Option<(f64, f64)> {
        if self.fixed.is_some() {
            return self.fixed;
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for c in &self.cols {
            for s in c.traces {
                if let Sample::Number { min, max, .. } = s {
                    lo = lo.min(min);
                    hi = hi.max(max);
                }
            }
        }
        (lo <= hi).then(|| nice_range(lo, hi))
    }

    /// The row of `x` in `range`, clamped to the screen.
    fn row(x: f64, range: Option<(f64, f64)>) -> i32 {
        let Some((lo, hi)) = range else { return ROWS / 2 };
        let t = (x - lo) / (hi - lo);
        if t.is_nan() {
            return ROWS / 2;
        }
        let r = (t * f64::from(ROWS - 1)).round().clamp(0.0, f64::from(ROWS - 1));
        r as i32
    }

    /// Trace `k`'s rows in each column (lowest, highest; 0 the bottom),
    /// `None` where it has nothing. With lines, each column reaches the
    /// last row of the one before it, so that the trace is unbroken and
    /// an ON/OFF signal a square wave.
    #[must_use]
    pub fn spans(&self, k: usize) -> Vec<Option<(i32, i32)>> {
        let range = self.range();
        let (off, on) = LEVELS[k];
        let level = |b: bool| if b { on } else { off };
        let mut prev: Option<i32> = None;
        self.cols
            .iter()
            .map(|c| {
                let (lo, hi, first, last) = match c.traces[k] {
                    Sample::None => {
                        prev = None;
                        return None;
                    }
                    Sample::Bool { first, last, off, on } => {
                        let lo = if off { level(false) } else { level(true) };
                        let hi = if on { level(true) } else { level(false) };
                        (lo, hi, level(first), level(last))
                    }
                    Sample::Number { first, last, min, max } => (
                        Self::row(min, range),
                        Self::row(max, range),
                        Self::row(first, range),
                        Self::row(last, range),
                    ),
                };
                let (mut lo, mut hi) = (lo, hi);
                if !self.dots
                    && let Some(p) = prev
                {
                    lo = lo.min(p).min(first);
                    hi = hi.max(p).max(first);
                }
                prev = Some(last);
                Some((lo, hi))
            })
            .collect()
    }

    /// Takes what comes into A as what the output gives, and tells the
    /// wire when it changed (or `always`).
    fn pass(&mut self, core: &mut Core, io: &mut Io<'_>, always: bool) {
        let a = Input::read(io, core, A);
        if !always && Input::same(a, self.out) {
            return;
        }
        self.out = a;
        let (state, number) = match a {
            Some(Input::Bool(b)) => (b, f64::from(u8::from(b))),
            Some(Input::Number(x)) => (truthy(x), x),
            None => (false, 0.0),
        };
        core.state = state;
        core.number = number;
        io.fire(core, OUT);
    }
}

impl Behavior for Oscilloscope {
    fn draw(&self, core: &Core, cx: &mut DrawCx<'_>) {
        let frame = if cx.running && self.frozen { 2 } else { 1 };
        cx.frame_at_rect(core.sheet, frame, core.h, core.v);
        if !cx.running {
            return;
        }
        let (px, py, _, ph) = PLOT;
        let (left, bottom) = (core.h + px, core.v + py + ph - 1);
        // The range, top left and bottom left.
        if let Some((lo, hi)) = self.range() {
            let c = cx.color(LABEL_COLOUR);
            let top = core.v + py + 1;
            let low = core.v + py + ph - 1 - i32::from(LABEL_SIZE);
            for (y, x) in [(top, hi), (low, lo)] {
                let text = crate::text::encode(&range_label(x));
                cx.text(LABEL_FACE, LABEL_SIZE, false, left + 2, y, &text, c);
            }
        }
        for k in [B, A] {
            let c = cx.color(COLOURS[k]);
            for (x, span) in (left..).zip(self.spans(k)) {
                let Some((lo, hi)) = span else { continue };
                if self.dots {
                    for r in [lo, hi] {
                        cx.surface.fill(IRect::new(x, bottom - r, x + 1, bottom - r + 1), c);
                    }
                } else {
                    cx.surface.fill(IRect::new(x, bottom - hi, x + 1, bottom - lo + 1), c);
                }
            }
        }
    }

    fn start(&mut self, core: &mut Core, io: &mut Io<'_>) {
        io.base_start(core);
        let s = &core.settings;
        self.scale = SCALES.get(usize::from(s.radios[1])).copied().unwrap_or(1);
        self.fixed = fixed_range(s);
        self.dots = s.checks & 1 != 0;
        self.cols.clear();
        self.at = None;
        self.frozen = false;
        self.pass(core, io, true);
    }

    fn tick(&mut self, core: &mut Core, io: &mut Io<'_>) {
        self.pass(core, io, false);
        self.frozen = io.wired(core, FREEZE) && truthy(io.number(core, FREEZE));
        if self.frozen {
            return;
        }
        let col = io.elapsed() / self.scale.max(1);
        if self.at != Some(col) {
            self.at = Some(col);
            if self.cols.len() == COLUMNS {
                self.cols.pop_front();
            }
            self.cols.push_back(Column::default());
        }
        let (a, b) = (Input::read(io, core, A), Input::read(io, core, B));
        if let Some(c) = self.cols.back_mut() {
            c.traces[0].add(a);
            c.traces[1].add(b);
        }
    }

    fn reset(&mut self, core: &mut Core) {
        core.base_reset();
        self.out = None;
        self.frozen = false;
    }

    fn init(&mut self, core: &mut Core) {
        core.settings.set_label(0, b"0");
        core.settings.set_label(1, b"100");
    }

    fn has_bool(&self, _core: &Core, _knot: usize) -> bool {
        matches!(self.out, None | Some(Input::Bool(_)))
    }

    fn has_number(&self, _core: &Core, _knot: usize) -> bool {
        matches!(self.out, None | Some(Input::Number(_)))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// A new Oscilloscope (`gadgets::Class::make`).
#[must_use]
pub fn oscilloscope() -> Box<dyn Behavior> {
    Box::new(Oscilloscope::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gadget::Object;
    use crate::gadgets::bench::{Bench, Source};
    use formats::tables::KnotKind;

    fn bench() -> (Bench, Oscilloscope) {
        let mut b = Bench::with_knots(&[KnotKind::Input, KnotKind::Input, KnotKind::Input, KnotKind::Output]);
        b.wire(OUT);
        let mut s = Oscilloscope::default();
        b.fresh(&mut s);
        (b, s)
    }

    fn number(s: Sample) -> (f64, f64, f64, f64) {
        match s {
            Sample::Number { first, last, min, max } => (first, last, min, max),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn it_scrolls_on_when_full() {
        let (mut b, mut s) = bench();
        b.start(&mut s);
        for t in 0..150 {
            b.source(A, Source::number(f64::from(t)));
            b.source(B, Source::boolean(t % 2 == 0));
            b.tick_at(&mut s, t);
            assert_eq!(s.columns().len(), (t as usize + 1).min(COLUMNS));
        }
        // The oldest column on the screen is tick 50's, the newest 149's:
        // the Strip Chart would have stopped at its end.
        assert_eq!(number(s.columns()[0].traces[0]), (50.0, 50.0, 50.0, 50.0));
        assert_eq!(number(s.columns()[COLUMNS - 1].traces[0]).1, 149.0);
        assert_eq!(
            s.columns()[COLUMNS - 1].traces[1],
            Sample::Bool {
                first: false,
                last: false,
                off: true,
                on: false
            }
        );
        // GO again: an empty screen.
        b.start(&mut s);
        assert!(s.columns().is_empty());
    }

    #[test]
    fn a_column_is_one_to_eight_ticks() {
        for (radio, ticks) in SCALES.iter().enumerate() {
            let (mut b, mut s) = bench();
            b.core.settings.radios[1] = u8::try_from(radio).unwrap();
            b.start(&mut s);
            for t in 0..32 {
                b.source(A, Source::number(f64::from(t)));
                b.tick_at(&mut s, t);
            }
            assert_eq!(s.columns().len(), 32 / *ticks as usize, "{ticks}");
            let last = f64::from(31 - ticks + 1);
            assert_eq!(
                number(s.columns()[s.columns().len() - 1].traces[0]),
                (last, 31.0, last, 31.0)
            );
            assert_eq!(s.columns()[0].traces[1], Sample::None, "B unwired");
        }
        // Past the four: one tick.
        let (mut b, mut s) = bench();
        b.core.settings.radios[1] = 9;
        b.start(&mut s);
        b.ticks(&mut s, 0, 1, 5);
        assert_eq!(s.columns().len(), 5);
    }

    #[test]
    fn freeze_holds_the_picture_not_the_output() {
        let (mut b, mut s) = bench();
        b.start(&mut s);
        b.source(A, Source::number(1.0));
        b.ticks(&mut s, 0, 1, 10);
        let held = s.columns().clone();
        b.source(FREEZE, Source::boolean(true));
        b.source(A, Source::number(7.0));
        b.tick_at(&mut s, 10);
        b.tick_at(&mut s, 11);
        assert!(s.frozen());
        assert_eq!(s.columns(), &held);
        assert_eq!(b.out(&s, OUT), Value::Number(7.0));
        // Let go: on it goes.
        b.source(FREEZE, Source::number(0.0));
        b.tick_at(&mut s, 12);
        assert!(!s.frozen());
        assert_eq!(s.columns().len(), 11);
        assert_eq!(number(s.columns()[10].traces[0]).0, 7.0);
    }

    #[test]
    fn the_output_is_a_passed_through() {
        let (mut b, mut s) = bench();
        // Unwired: off and 0.
        b.start(&mut s);
        assert!(b.fired(OUT));
        assert_eq!(b.out(&s, OUT), Value::Number(0.0));
        assert!(s.has_bool(&b.core, OUT));
        b.source(A, Source::number(2.5));
        b.tick_at(&mut s, 0);
        assert!(b.fired(OUT));
        assert_eq!(b.out(&s, OUT), Value::Number(2.5));
        assert!(!s.has_bool(&b.core, OUT));
        b.tick_at(&mut s, 1);
        assert!(!b.fired(OUT), "unchanged");
        b.source(A, Source::boolean(true));
        b.tick_at(&mut s, 2);
        assert!(b.fired(OUT));
        assert_eq!(b.out(&s, OUT), Value::Bool(true));
        assert!(!s.has_number(&b.core, OUT));
        // A picture: whether there is one.
        b.source(A, Source::object(Some(Object::Picture(1050))));
        b.tick_at(&mut s, 3);
        assert!(!b.fired(OUT));
        assert_eq!(b.out(&s, OUT), Value::Bool(true));
        // NaN is passed on, once.
        b.source(A, Source::number(f64::NAN));
        b.tick_at(&mut s, 4);
        assert!(b.fired(OUT));
        b.tick_at(&mut s, 5);
        assert!(!b.fired(OUT));
        assert!(matches!(b.out(&s, OUT), Value::Number(x) if x.is_nan()));
        // But not drawn.
        assert_eq!(
            (s.columns()[4].traces[0], s.columns()[5].traces[0]),
            (Sample::None, Sample::None)
        );
        // Stopped: off and 0 again.
        s.reset(&mut b.core);
        assert_eq!(b.out(&s, OUT), Value::Number(0.0));
    }

    #[test]
    fn the_screen_is_a_column_a_pixel() {
        assert_eq!(usize::try_from(PLOT.2).unwrap(), COLUMNS);
        assert_eq!(PLOT.3, ROWS);
    }

    #[test]
    fn ranges() {
        assert_eq!(nice_range(0.0, 100.0), (0.0, 100.0));
        assert_eq!(nice_range(3.0, 97.0), (0.0, 100.0));
        assert_eq!(nice_range(-0.3, 0.7), (-0.5, 1.0));
        assert_eq!(nice_range(5.0, 5.0), (4.0, 6.0));
        assert_eq!(nice_range(0.0, 0.0), (-1.0, 1.0));
        let (lo, hi) = nice_range(1e300, 1e300);
        assert!(lo < 1e300 && hi > 1e300);
        let (lo, hi) = nice_range(-f64::MAX, f64::MAX);
        assert!(lo <= -f64::MAX && hi >= f64::MAX);
        let mut s = Settings::default();
        assert_eq!(fixed_range(&s), None, "automatic");
        s.radios[0] = 1;
        assert_eq!(fixed_range(&s), None, "no numbers: automatic");
        s.set_label(0, b"10");
        s.set_label(1, b"-2.5");
        assert_eq!(fixed_range(&s), Some((-2.5, 10.0)));
        s.set_label(1, b"10");
        assert_eq!(fixed_range(&s), Some((9.0, 11.0)));
        for (x, l) in [
            (0.0, "0"),
            (100.0, "100"),
            (-5.0, "-5"),
            (0.25, "0.25"),
            (1.0 / 3.0, "0.3333"),
            (1234.5678, "1235"),
            (2e7, "2.00e7"),
            (-1e-5, "-1.00e-5"),
        ] {
            assert_eq!(range_label(x), l);
        }
    }

    #[test]
    fn traces_are_lines_or_dots_and_on_off_is_a_square_wave() {
        let (mut b, mut s) = bench();
        b.start(&mut s);
        for (t, (a, sw)) in [(0.0, false), (100.0, false), (100.0, true), (50.0, true)]
            .into_iter()
            .enumerate()
        {
            b.source(A, Source::number(a));
            b.source(B, Source::boolean(sw));
            b.tick_at(&mut s, u32::try_from(t).unwrap());
        }
        assert_eq!(s.range(), Some((0.0, 100.0)));
        assert_eq!(
            s.spans(0),
            [Some((0, 0)), Some((0, 63)), Some((63, 63)), Some((32, 63))]
        );
        let (off, on) = LEVELS[1];
        assert_eq!(
            s.spans(1),
            [Some((off, off)), Some((off, off)), Some((off, on)), Some((on, on))]
        );
        // Dots: each column on its own.
        s.dots = true;
        assert_eq!(
            s.spans(0),
            [Some((0, 0)), Some((63, 63)), Some((63, 63)), Some((32, 32))]
        );
        // A fixed range clamps.
        b.core.settings.radios[0] = 1;
        b.core.settings.set_label(0, b"0");
        b.core.settings.set_label(1, b"50");
        b.core.settings.checks = 0;
        b.start(&mut s);
        b.source(A, Source::number(100.0));
        b.tick_at(&mut s, 0);
        b.source(A, Source::number(-1.0));
        b.tick_at(&mut s, 1);
        assert_eq!(s.range(), Some((0.0, 50.0)));
        assert_eq!(s.spans(0), [Some((63, 63)), Some((0, 63))]);
        // Unwired: nothing, and no join over the gap.
        b.core.knots[A].wire = None;
        b.tick_at(&mut s, 2);
        assert_eq!(s.spans(0)[2], None);
    }

    /// The part draws inside its bounds, its traces and range inside its
    /// screen; with OpenWDATA, running and not, frozen and not.
    #[test]
    fn it_draws_on_its_screen() {
        use crate::gfx::{Pictures, Surface};
        let data = crate::opendata::builtin();
        let assets = crate::assets::Assets::load(None, Some(&data)).unwrap();
        let pictures = Pictures::default();
        let (size, at) = (260_i32, 40);
        let mut g = crate::doc::new_gadget(&assets, SCOPE, 1, at, at);
        let r = g.core.rect();
        assert_eq!((r.x1 - r.x0, r.y1 - r.y0), (148, 100));
        let (mut b, mut s) = bench();
        b.start(&mut s);
        for t in 0..300 {
            let x = [1e9, -1e9, 0.5, -0.25][(t / 7) as usize % 4];
            b.source(A, Source::number(x * f64::from(t)));
            b.source(B, Source::boolean(t % 13 < 6));
            b.tick_at(&mut s, t);
        }
        g.behavior = Box::new(s);
        let (px, py, pw, ph) = PLOT;
        let plot = IRect::new(at + px, at + py, at + px + pw, at + py + ph);
        for (running, frozen) in [(false, false), (true, false), (true, true)] {
            if let Some(s) = g.behavior.as_any_mut().downcast_mut::<Oscilloscope>() {
                s.frozen = frozen;
            }
            let draw = |bg: u8, traces: bool| {
                let mut surface = Surface::new(size.cast_unsigned(), size.cast_unsigned());
                surface.fill(IRect::new(0, 0, size, size), bg);
                let mut cx = DrawCx {
                    surface: &mut surface,
                    assets: &assets,
                    pictures: &pictures,
                    running,
                    ticks: 0,
                    lights: false,
                };
                if traces {
                    g.behavior.draw(&g.core, &mut cx);
                } else {
                    cx.frame_at_rect(g.core.sheet, if running && frozen { 2 } else { 1 }, at, at);
                }
                surface
            };
            let (one, two, bare) = (draw(3, true), draw(200, true), draw(3, false));
            let mut drawn = 0;
            for i in 0..one.pixels.len() {
                let x = i32::try_from(i).unwrap() % size;
                let y = i32::try_from(i).unwrap() / size;
                if one.pixels[i] != 3 || two.pixels[i] != 200 {
                    assert!(r.contains(x, y), "{x}, {y} outside {r:?}");
                }
                if one.pixels[i] != bare.pixels[i] {
                    drawn += 1;
                    assert!(plot.contains(x, y), "{x}, {y} off the screen");
                }
            }
            assert_eq!(drawn > 100, running, "{running} {frozen}: {drawn}");
        }
    }
}
