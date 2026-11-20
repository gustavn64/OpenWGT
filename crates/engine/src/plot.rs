//! Values over time drawn on a [`Surface`]: the debugging tools' graph
//! window, and for a part that shows its inputs as traces. Each series
//! gets a lane of its own: on/off as a square wave, numbers scaled to the
//! lane (their least and greatest shown), objects as marks where they
//! change; a time axis under them, the cursor's line, and what lies past
//! the cursor (recorded, not yet replayed) dimmed.

use formats::fnt::Font;

use crate::gfx::{IRect, Surface};

/// A value at a time.
#[derive(Debug, Clone, PartialEq)]
pub enum Sample {
    /// Nothing there: no line.
    Nothing,
    Bool(bool),
    Number(f64),
    /// An object, by what's said of it (`picture 1204`).
    Object(String),
}

/// A series: its name, colour and changes, by tick, in order; each value
/// holds until the next.
#[derive(Debug, Clone, Copy)]
pub struct Series<'a> {
    pub name: &'a str,
    pub colour: u8,
    pub changes: &'a [(u32, Sample)],
}

impl Series<'_> {
    /// The value at tick `t`.
    #[must_use]
    pub fn at(&self, t: u32) -> Option<&Sample> {
        let i = self.changes.partition_point(|(c, _)| *c <= t);
        i.checked_sub(1).map(|i| &self.changes[i].1)
    }
}

/// The part of time shown: from tick `t0`, `ticks_per_px` a pixel; the
/// cursor (if any) and the last tick recorded (dimmed past the cursor).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    pub t0: f64,
    pub ticks_per_px: f64,
    pub cursor: Option<u32>,
    pub end: u32,
}

impl View {
    /// The x of tick `t` in `area`.
    #[must_use]
    pub fn x(&self, area: IRect, t: f64) -> i32 {
        #[allow(clippy::cast_possible_truncation)]
        let dx = ((t - self.t0) / self.ticks_per_px).floor().clamp(-1e6, 1e6) as i32;
        area.x0 + dx
    }

    /// The tick at `x` in `area`.
    #[must_use]
    pub fn tick(&self, area: IRect, x: i32) -> f64 {
        self.t0 + f64::from(x - area.x0) * self.ticks_per_px
    }

    /// The last tick shown in `area`.
    #[must_use]
    pub fn t1(&self, area: IRect) -> f64 {
        self.tick(area, area.x1)
    }
}

/// The colours a plot is drawn in (palette indexes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Style {
    pub background: u8,
    pub grid: u8,
    pub text: u8,
    pub cursor: u8,
    /// What dims the recorded future.
    pub dim: u8,
}

/// The time axis's height under the lanes.
#[must_use]
pub fn axis_height(font: Option<&Font>) -> i32 {
    font.map_or(0, |f| i32::from(f.height) + 3)
}

/// The lanes' rectangles in `area` for `n` series (the axis under them).
#[must_use]
pub fn lanes(area: IRect, n: usize, font: Option<&Font>) -> Vec<IRect> {
    let n32 = i32::try_from(n).unwrap_or(1).max(1);
    let plot = IRect::new(area.x0, area.y0, area.x1, area.y1 - axis_height(font));
    (0..n32)
        .map(|i| {
            let y0 = plot.y0 + plot.height() * i / n32;
            let y1 = plot.y0 + plot.height() * (i + 1) / n32;
            IRect::new(plot.x0, y0, plot.x1, y1)
        })
        .collect()
}

/// Draws `series` in `area` as `view` says; nothing outside `area`.
pub fn draw(s: &mut Surface, area: IRect, font: Option<&Font>, series: &[Series], view: &View, style: Style) {
    if area.is_empty() {
        return;
    }
    let old = s.set_clip(area.intersect(s.clip()));
    s.fill(area, style.background);
    let lanes = lanes(area, series.len(), font);
    let axis = IRect::new(area.x0, area.y1 - axis_height(font), area.x1, area.y1);
    draw_axis(s, axis, area, font, view, style);
    for (lane, se) in lanes.iter().zip(series) {
        s.fill(IRect::new(lane.x0, lane.y1 - 1, lane.x1, lane.y1), style.grid);
        draw_series(s, *lane, se, view, font);
        if let Some(f) = font {
            s.text(f, lane.x0 + 2, lane.y0 + 1, &crate::text::encode(se.name), se.colour);
        }
    }
    // The recorded future, dimmed (every other pixel), and the cursor.
    if let Some(c) = view.cursor {
        let x = view.x(area, f64::from(c)).max(area.x0 - 1);
        let xe = view.x(area, f64::from(view.end)).min(area.x1);
        for y in area.y0..axis.y0 {
            let mut px = x + 1 + (x + 1 + y).rem_euclid(2);
            while px <= xe {
                s.pset(px, y, style.dim);
                px += 2;
            }
        }
        s.fill(IRect::new(x, area.y0, x + 1, axis.y0), style.cursor);
    }
    s.set_clip(old);
}

/// Ticks a second.
const TPS: u32 = 60;

/// A step between the axis's marks, in ticks, at least `min` ticks.
fn axis_step(min: f64) -> u32 {
    const STEPS: [u32; 16] = [
        1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 1200, 1800, 3600, 7200, 18_000, 36_000,
    ];
    STEPS.into_iter().find(|&s| f64::from(s) >= min).unwrap_or(108_000)
}

/// A tick count as the axis says it: `30`, `2 s`, `1:30`.
#[must_use]
pub fn tick_label(t: u32) -> String {
    if !t.is_multiple_of(TPS) || t == 0 {
        return t.to_string();
    }
    let secs = t / TPS;
    if secs < 60 {
        format!("{secs} s")
    } else {
        format!("{}:{:02}", secs / 60, secs % 60)
    }
}

fn draw_axis(s: &mut Surface, axis: IRect, area: IRect, font: Option<&Font>, view: &View, style: Style) {
    let Some(f) = font else { return };
    s.fill(IRect::new(axis.x0, axis.y0, axis.x1, axis.y0 + 1), style.text);
    // Marks at least 50 pixels apart.
    let step = axis_step(50.0 * view.ticks_per_px);
    let first = (view.t0.max(0.0) / f64::from(step)).ceil();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mut t = (first as u32).saturating_mul(step);
    while f64::from(t) <= view.t1(area) {
        let x = view.x(area, f64::from(t));
        s.fill(IRect::new(x, axis.y0, x + 1, axis.y0 + 3), style.text);
        for y in (area.y0..axis.y0).step_by(3) {
            s.pset(x, y, style.grid);
        }
        s.text(f, x + 2, axis.y0 + 2, &crate::text::encode(&tick_label(t)), style.text);
        t = t.saturating_add(step);
        if step == 0 {
            break;
        }
    }
}

/// The least and greatest number of `se` in the ticks shown.
fn range(se: &Series, view: &View, area: IRect) -> Option<(f64, f64)> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let t0 = view.t0.max(0.0) as u32;
    let t1 = view.t1(area);
    let start = se.changes.partition_point(|(c, _)| *c <= t0).saturating_sub(1);
    let mut r: Option<(f64, f64)> = None;
    for (t, v) in &se.changes[start..] {
        if f64::from(*t) > t1 {
            break;
        }
        if let Sample::Number(n) = v
            && n.is_finite()
        {
            r = Some(r.map_or((*n, *n), |(lo, hi)| (lo.min(*n), hi.max(*n))));
        }
    }
    r
}

fn draw_series(s: &mut Surface, lane: IRect, se: &Series, view: &View, font: Option<&Font>) {
    let lh = font.map_or(0, |f| i32::from(f.height));
    // The trace's band: under the name, above the lane's bottom.
    let top = lane.y0 + lh.min(lane.height() / 3) + 2;
    let bottom = lane.y1 - 3;
    if bottom <= top {
        return;
    }
    let range = range(se, view, lane);
    let y_of = |v: &Sample| -> Option<i32> {
        match v {
            Sample::Nothing => None,
            Sample::Bool(b) => Some(if *b { top } else { bottom }),
            Sample::Number(n) => {
                let (lo, hi) = range?;
                if !n.is_finite() {
                    return None;
                }
                let k = if hi > lo { (n - lo) / (hi - lo) } else { 0.5 };
                #[allow(clippy::cast_possible_truncation)]
                Some(bottom - (k * f64::from(bottom - top)).round() as i32)
            }
            Sample::Object(_) => Some(i32::midpoint(top, bottom)),
        }
    };
    let end_x = view.x(lane, f64::from(view.end)).min(lane.x1);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let t0 = view.t0.max(0.0) as u32;
    let start = se.changes.partition_point(|(c, _)| *c <= t0).saturating_sub(1);
    let changes = &se.changes[start..];
    let mut last_label_x = i32::MIN;
    for (i, (t, v)) in changes.iter().enumerate() {
        let x0 = view.x(lane, f64::from(*t)).max(lane.x0);
        if x0 > lane.x1 {
            break;
        }
        let x1 = changes
            .get(i + 1)
            .map_or(end_x, |(n, _)| view.x(lane, f64::from(*n)))
            .min(end_x);
        let y = y_of(v);
        // The rise or fall from the value before.
        if i > 0
            && let (Some(a), Some(b)) = (y_of(&changes[i - 1].1), y)
        {
            s.fill(IRect::new(x0, a.min(b), x0 + 1, a.max(b) + 1), se.colour);
        }
        if let Some(y) = y
            && x1 >= x0
        {
            s.fill(IRect::new(x0, y, x1 + 1, y + 1), se.colour);
        }
        if let Sample::Object(what) = v {
            s.fill(IRect::new(x0, top, x0 + 1, bottom + 1), se.colour);
            s.fill(IRect::sized(x0 - 1, top, 3, 3), se.colour);
            if let Some(f) = font
                && x0 >= last_label_x
            {
                let w = s.text(
                    f,
                    x0 + 3,
                    i32::midpoint(top, bottom) - lh,
                    &crate::text::encode(what),
                    se.colour,
                );
                last_label_x = x0 + 3 + w + 4;
            }
        }
    }
    // The least and greatest numbers, at the right.
    if let (Some((lo, hi)), Some(f)) = (range, font) {
        let fmt = |n: f64| crate::script::value::number(n);
        let hi_t = crate::text::encode(&fmt(hi));
        let lo_t = crate::text::encode(&fmt(lo));
        let wh = i32::try_from(f.width(&hi_t)).unwrap_or(0);
        let wl = i32::try_from(f.width(&lo_t)).unwrap_or(0);
        s.text(f, lane.x1 - wh - 2, top - 1, &hi_t, se.colour);
        if bottom - lh > top + lh {
            s.text(f, lane.x1 - wl - 2, bottom - lh + 1, &lo_t, se.colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLE: Style = Style {
        background: 255,
        grid: 13,
        text: 0,
        cursor: 1,
        dim: 2,
    };

    #[test]
    fn values_hold_until_they_change() {
        let ch = [
            (0, Sample::Bool(false)),
            (10, Sample::Bool(true)),
            (20, Sample::Nothing),
        ];
        let se = Series {
            name: "a",
            colour: 5,
            changes: &ch,
        };
        assert_eq!(se.at(0), Some(&Sample::Bool(false)));
        assert_eq!(se.at(9), Some(&Sample::Bool(false)));
        assert_eq!(se.at(10), Some(&Sample::Bool(true)));
        assert_eq!(se.at(25), Some(&Sample::Nothing));
        let late = [(5, Sample::Number(1.0))];
        let se = Series { changes: &late, ..se };
        assert_eq!(se.at(4), None);
    }

    #[test]
    fn the_axis_says_ticks_and_seconds() {
        assert_eq!(tick_label(0), "0");
        assert_eq!(tick_label(30), "30");
        assert_eq!(tick_label(120), "2 s");
        assert_eq!(tick_label(60 * 90), "1:30");
        assert_eq!(axis_step(7.0), 10);
        assert_eq!(axis_step(50.0), 60);
    }

    #[test]
    fn it_draws_inside_its_area_only() {
        let mut s = Surface::new(200, 120);
        s.fill(s.bounds(), 99);
        let area = IRect::new(20, 10, 180, 100);
        let wave = [
            (0, Sample::Bool(false)),
            (30, Sample::Bool(true)),
            (60, Sample::Bool(false)),
        ];
        let nums = [
            (0, Sample::Number(-5.0)),
            (40, Sample::Number(1e9)),
            (50, Sample::Number(f64::NAN)),
        ];
        let objs = [(10, Sample::Object("picture 1204".into())), (70, Sample::Nothing)];
        let series = [
            Series {
                name: "wave",
                colour: 3,
                changes: &wave,
            },
            Series {
                name: "n",
                colour: 4,
                changes: &nums,
            },
            Series {
                name: "o",
                colour: 6,
                changes: &objs,
            },
        ];
        for (tpp, t0) in [(1.0, 0.0), (0.25, -40.0), (8.0, 30.0), (0.5, 1e7)] {
            let view = View {
                t0,
                ticks_per_px: tpp,
                cursor: Some(45),
                end: 90,
            };
            draw(&mut s, area, None, &series, &view, STYLE);
            for y in 0..120 {
                for x in 0..200 {
                    if !area.contains(x, y) {
                        assert_eq!(s.get(x, y), Some(99), "({x}, {y}) at {tpp} {t0}");
                    }
                }
            }
        }
        // The square wave: high in its lane after tick 30, low before.
        let view = View {
            t0: 0.0,
            ticks_per_px: 1.0,
            cursor: None,
            end: 90,
        };
        draw(&mut s, area, None, &series[..1], &view, STYLE);
        let lane = lanes(area, 1, None)[0];
        let col = |x: i32| (lane.y0..lane.y1).find(|&y| s.get(x, y) == Some(3));
        let (low, high) = (col(area.x0 + 15).unwrap(), col(area.x0 + 45).unwrap());
        assert!(high < low, "{high} {low}");
        // Past what's recorded, nothing.
        assert_eq!(col(area.x0 + 120), None);
    }
}
