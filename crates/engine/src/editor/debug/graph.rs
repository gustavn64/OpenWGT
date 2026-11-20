//! The graph window (Debug ▸ Graph): the probes' records as traces over
//! time ([`crate::plot`]), following the run (or the timeline's cursor);
//! dragged sideways to look back, zoomed with − and + (or the wheel);
//! a click goes to that tick ([`super::timeline`]).

use formats::fnt::Font;

use super::super::{Editor, ui};
use super::probes::Probe;
use super::window::{self, Colours};
use crate::gfx::{IRect, Surface};
use crate::lang::tr;
use crate::plot::{self, Series, Style, View};

/// The zoom's range, as powers of two of ticks a pixel.
const ZOOM_MIN: i32 = -3;
const ZOOM_MAX: i32 = 8;
/// A drag shorter than this is a click.
const CLICK_PX: i32 = 3;

/// The graph's view of time.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Graph {
    /// Ticks a pixel: 2 to this power.
    pub zoom: i32,
    /// The first tick shown, or `None` to follow the cursor.
    pub t0: Option<f64>,
    /// A drag on the plot: where it began and the first tick then.
    drag: Option<(i32, f64)>,
    moved: bool,
}

/// Where the run is for the windows: the tick now (the cursor), and the
/// last one recorded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Time {
    pub cursor: u32,
    pub end: u32,
}

/// The graph window's parts: zoom out, zoom in, follow, the plot.
pub struct Layout {
    pub minus: IRect,
    pub plus: IRect,
    pub follow: IRect,
    pub plot: IRect,
}

#[must_use]
pub fn layout(r: IRect, font: &Font) -> Layout {
    let c = window::client(r).inset(2);
    let bh = i32::from(font.height) + 6;
    let minus = IRect::sized(c.x0, c.y0, 22, bh);
    let plus = IRect::sized(minus.x1 + 2, c.y0, 22, bh);
    let follow = IRect::sized(plus.x1 + 6, c.y0, 60, bh);
    let plot = IRect::new(c.x0, c.y0 + bh + 3, c.x1, c.y1);
    Layout {
        minus,
        plus,
        follow,
        plot,
    }
}

impl Graph {
    fn ticks_per_px(&self) -> f64 {
        2f64.powi(self.zoom)
    }

    /// What the plot at `plot` shows at `time`: from where it was dragged
    /// to, or ending a little after the cursor.
    #[must_use]
    pub fn view(&self, plot: IRect, time: Time) -> View {
        let tpp = self.ticks_per_px();
        let span = f64::from(plot.width()) * tpp;
        let t0 = self.t0.unwrap_or_else(|| {
            let right = f64::from(time.cursor) + 8.0 * tpp;
            (right - span).max(0.0)
        });
        View {
            t0,
            ticks_per_px: tpp,
            cursor: (time.cursor < time.end).then_some(time.cursor),
            end: time.end,
        }
    }

    pub fn draw(&self, s: &mut Surface, font: &Font, r: IRect, col: Colours, probes: &[Probe], time: Time) {
        let l = layout(r, font);
        ui::button(s, font, l.minus, "-", false, false, self.zoom < ZOOM_MAX);
        ui::button(s, font, l.plus, "+", false, false, self.zoom > ZOOM_MIN);
        ui::button(s, font, l.follow, tr("Follow"), self.t0.is_none(), false, true);
        let tpp = self.ticks_per_px();
        let scale = if tpp >= 1.0 {
            format!("{tpp} {}", tr("ticks/pixel"))
        } else {
            format!("{} {}", 1.0 / tpp, tr("pixels/tick"))
        };
        ui::label(s, font, l.follow.x1 + 8, l.follow.y0 + 3, &scale, ui::BLACK);
        ui::sunken(s, l.plot);
        let area = l.plot.inset(2);
        if probes.is_empty() {
            s.fill(area, ui::WHITE);
            let lines = ui::wrap(
                font,
                tr("Right-click a wire, a knot or a part, then Show Graph."),
                area.width() - 8,
            );
            let mut y = area.y0 + 4;
            for line in lines {
                ui::label(s, font, area.x0 + 4, y, &line, col.grey);
                y += i32::from(font.height);
            }
            return;
        }
        let series: Vec<Series> = probes
            .iter()
            .map(|p| Series {
                name: &p.name,
                colour: col.series(p.colour),
                changes: &p.changes,
            })
            .collect();
        let style = Style {
            background: ui::WHITE,
            grid: col.light_grey,
            text: ui::BLACK,
            cursor: col.red,
            dim: col.light_grey,
        };
        plot::draw(s, area, Some(font), &series, &self.view(area, time), style);
    }
}

impl Editor {
    /// A click in the graph window `r` at `p`: the buttons, or a drag on
    /// the plot begun. Whether it goes on as a drag.
    pub(super) fn graph_click(&mut self, r: IRect, p: (i32, i32)) -> bool {
        let Some(font) = ui::font(&self.assets) else {
            return false;
        };
        let l = layout(r, font);
        let time = self.debug.time;
        let g = &mut self.debug.graph;
        if l.minus.contains(p.0, p.1) {
            g.zoom = (g.zoom + 1).min(ZOOM_MAX);
        } else if l.plus.contains(p.0, p.1) {
            g.zoom = (g.zoom - 1).max(ZOOM_MIN);
        } else if l.follow.contains(p.0, p.1) {
            g.t0 = None;
        } else if l.plot.contains(p.0, p.1) {
            let t0 = g.view(l.plot.inset(2), time).t0;
            g.drag = Some((p.0, t0));
            g.moved = false;
            return true;
        }
        false
    }

    /// The plot dragged to `p`: earlier or later ticks.
    pub(super) fn graph_drag(&mut self, p: (i32, i32)) {
        let g = &mut self.debug.graph;
        let Some((x0, t0)) = g.drag else { return };
        if (p.0 - x0).abs() > CLICK_PX {
            g.moved = true;
        }
        if g.moved {
            g.t0 = Some((t0 - f64::from(p.0 - x0) * g.ticks_per_px()).max(-8.0 * g.ticks_per_px()));
        }
    }

    /// The button let go over the plot: a click (not a drag) goes to the
    /// tick under it.
    pub(super) fn graph_release(&mut self, r: IRect) {
        let Some(font) = ui::font(&self.assets) else { return };
        let time = self.debug.time;
        let g = &mut self.debug.graph;
        let Some((x0, t0)) = g.drag.take() else { return };
        if g.moved {
            return;
        }
        let area = layout(r, font).plot.inset(2);
        let view = View {
            t0,
            ..g.view(area, time)
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let t = view.tick(area, x0).round().max(0.0) as u32;
        if self.debug.timeline.end > 0 {
            self.timeline_seek_click(t);
        }
    }

    /// The wheel over the graph: zoomed in (up) or out.
    pub(super) fn graph_wheel(&mut self, dy: i32) {
        let g = &mut self.debug.graph;
        g.zoom = (g.zoom - dy.signum()).clamp(ZOOM_MIN, ZOOM_MAX);
    }
}
