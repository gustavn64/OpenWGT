//! The tools' share of the mouse, the keys, the text typed and the
//! drawing: their windows ([`super::window`]) take what lands on them,
//! the console the keyboard while it has it. Each of the editor's input
//! paths asks here first in one line.

use super::super::{Button, Editor, TextField, menu, ui};
use super::window::{self, Colours, Drag, Hit, Kind, Win};
use super::{Debugger, context, watch};
use crate::assets::Assets;
use crate::gfx::{IRect, Surface};
use crate::lang::tr;

impl Editor {
    /// Whether a box the editor waits on (or the recorder) is up: the
    /// tools then take nothing.
    pub(super) fn debug_blocked(&self) -> bool {
        self.recording.is_some()
            || self.about.is_some()
            || self.prompt.is_some()
            || self.settings.is_some()
            || self.dialog.is_some()
            || self.file_dialog.is_some()
            || self.puzzle_dialog.is_some()
    }

    /// Where the tools' windows may be: the frame below the menu bar.
    fn debug_area(&self) -> IRect {
        self.surface.bounds()
    }

    /// Opens window `kind` (where it was, else where it first goes) on
    /// top, with the keyboard if it takes keys.
    pub(super) fn open_window(&mut self, kind: Kind) {
        let area = self.debug_area();
        let rect = self.debug.windows.iter().find(|w| w.kind == kind).map_or_else(
            || {
                self.debug
                    .closed
                    .iter()
                    .find(|w| w.kind == kind)
                    .map_or_else(|| first_place(kind, area), |w| w.rect)
            },
            |w| w.rect,
        );
        self.debug.windows.retain(|w| w.kind != kind);
        let rect = window::keep_in(rect, area, menu::BAR_H);
        self.debug.windows.push(Win { kind, rect });
        if kind == Kind::Console {
            self.debug.focus = Some(Kind::Console);
            if let Some(h) = &mut self.help {
                h.active = false;
            }
        }
    }

    /// Closes window `kind`, keeping where it was.
    pub(super) fn close_window(&mut self, kind: Kind) {
        if let Some(i) = self.debug.windows.iter().position(|w| w.kind == kind) {
            let w = self.debug.windows.remove(i);
            self.debug.closed.retain(|c| c.kind != kind);
            self.debug.closed.push(w);
        }
        if self.debug.focus == Some(kind) {
            self.debug.focus = None;
        }
    }

    /// Opens or closes window `kind`.
    pub(super) fn toggle_window(&mut self, kind: Kind) {
        if self.debug.window(kind).is_some() {
            self.close_window(kind);
        } else {
            self.open_window(kind);
        }
    }

    /// A button pressed at `(x, y)` (frame coordinates): the tools' if it
    /// lands on their windows, their popup, or (the right button on the
    /// worksheet, held with Ctrl while the widget runs unheld) asks for
    /// the popup. Whether they took it.
    pub(in crate::editor) fn debug_mouse_down(&mut self, x: i32, y: i32, button: Button) -> bool {
        if !self.debugging() || self.debug_blocked() {
            return false;
        }
        if self.debug.popup.is_some() {
            self.popup_click((x, y));
            return true;
        }
        if self.menu.is_open() || self.drag.is_some() {
            return false;
        }
        let Some(i) = self.debug.windows.iter().rposition(|w| w.rect.contains(x, y)) else {
            self.debug.focus = None;
            // The popup: on the worksheet, while editing or held, or with
            // Ctrl (the parts that poll the right button have it else).
            if button == Button::Right
                && self.over_view((x, y))
                && (!self.running || self.debug.run.paused || self.ctrl)
            {
                return self.open_popup((x, y));
            }
            return false;
        };
        // On top, and with the keyboard (the help window's gone).
        let w = self.debug.windows.remove(i);
        self.debug.windows.push(w);
        self.debug.focus = Some(w.kind);
        if let Some(h) = &mut self.help {
            h.active = false;
        }
        if button != Button::Left {
            return true;
        }
        match window::hit(w.rect, (x, y)) {
            Some(Hit::Close) => self.close_window(w.kind),
            Some(Hit::Caption) => {
                self.debug.drag = Some(Drag::Move {
                    kind: w.kind,
                    grab: (x - w.rect.x0, y - w.rect.y0),
                });
            }
            Some(Hit::Grip) => {
                self.debug.drag = Some(Drag::Size {
                    kind: w.kind,
                    grab: (w.rect.x1 - x, w.rect.y1 - y),
                });
            }
            Some(Hit::Client) | None => {
                if self.window_click(w, (x, y)) {
                    self.debug.drag = Some(Drag::Client { kind: w.kind });
                }
            }
        }
        true
    }

    /// A click inside window `w` at `p`; whether it goes on as a drag.
    fn window_click(&mut self, w: Win, p: (i32, i32)) -> bool {
        match w.kind {
            Kind::Console => {
                self.console_click(w.rect, p);
                false
            }
            Kind::Graph => self.graph_click(w.rect, p),
            Kind::Watch => {
                self.watch_click(w.rect, p);
                false
            }
        }
    }

    /// The pointer moved with a window's drag going: the window moves or
    /// sizes (or its inside follows). Whether it was the tools'.
    pub(super) fn window_drag_move(&mut self, x: i32, y: i32) -> bool {
        let Some(d) = self.debug.drag else { return false };
        let area = self.debug_area();
        match d {
            Drag::Move { kind, grab } => {
                if let Some(w) = self.debug.window_mut(kind) {
                    let r = IRect::sized(x - grab.0, y - grab.1, w.rect.width(), w.rect.height());
                    w.rect = window::keep_in(r, area, menu::BAR_H);
                }
            }
            Drag::Size { kind, grab } => {
                if let Some(w) = self.debug.window_mut(kind) {
                    let r = IRect::new(w.rect.x0, w.rect.y0, x + grab.0, y + grab.1);
                    w.rect = window::keep_in(r, area, menu::BAR_H);
                }
            }
            Drag::Client { kind: Kind::Graph } => self.graph_drag((x, y)),
            Drag::Client { .. } => {}
        }
        true
    }

    /// A button released: the end of a window's drag. Whether it was the
    /// tools'.
    pub(in crate::editor) fn debug_mouse_up(&mut self, x: i32, y: i32, button: Button) -> bool {
        if button != Button::Left {
            return false;
        }
        if self.debug.drag.is_none() {
            return false;
        }
        self.window_drag_move(x, y);
        self.graph_release();
        self.debug.drag = None;
        true
    }

    /// The wheel over a window: its own (the console's log scrolls, the
    /// graph zooms). Whether it was over one.
    pub(in crate::editor) fn debug_wheel(&mut self, dy: i32) -> bool {
        if !self.debugging() || self.debug_blocked() {
            return false;
        }
        let (x, y) = self.mouse;
        let Some(w) = self.debug.windows.iter().rev().find(|w| w.rect.contains(x, y)).copied() else {
            return false;
        };
        match w.kind {
            Kind::Console => self.console_wheel(dy),
            Kind::Graph => self.graph_wheel(dy),
            Kind::Watch => {}
        }
        true
    }

    /// Text typed: the console's while it has the keyboard. Whether it
    /// took it.
    pub(in crate::editor) fn debug_text(&mut self, typed: &str) -> bool {
        if !self.console_has_keys() {
            return false;
        }
        self.debug.console.type_text(typed);
        true
    }

    /// Whether the console takes the keys now.
    pub(super) fn console_has_keys(&self) -> bool {
        self.debugging()
            && self.debug.focus == Some(Kind::Console)
            && self.debug.window(Kind::Console).is_some()
            && !self.debug_blocked()
            && !self.menu.is_open()
    }

    /// The clipboard's text asked for by Ctrl+V: the console's, while it
    /// has the keyboard. Whether it took it.
    pub(in crate::editor) fn debug_paste(&mut self, text: &str) -> bool {
        if !self.console_has_keys() {
            return false;
        }
        self.console_paste(text);
        true
    }

    /// The console's line as the focused text field, while it has the
    /// keyboard.
    pub(in crate::editor) fn debug_focused_text(&self) -> Option<TextField> {
        self.console_has_keys().then(|| self.console_field())
    }

    /// The focused text field edited elsewhere: the console's line, if it's
    /// that. Whether it was.
    pub(in crate::editor) fn debug_set_focused_text(&mut self, text: &str) -> bool {
        if !self.console_has_keys() {
            return false;
        }
        self.set_console_line(text);
        true
    }

    /// Before drawing: what the windows show, brought up to date.
    pub(super) fn windows_prepare(&mut self) {
        if !self.debugging() {
            self.debug.windows.clear();
            self.debug.popup = None;
            self.debug.drag = None;
            self.debug.focus = None;
            return;
        }
        if self.debug.colours.is_none() {
            self.debug.colours = Some(Colours::from(&self.assets.palette()));
        }
        if self.debug.windows.is_empty() {
            return;
        }
        self.debug.console.tasks = self.console_tasks();
        self.debug.console.focused = self.console_has_keys();
        if self.debug.window(Kind::Watch).is_some() {
            self.watch_values();
        }
    }
}

/// Where window `kind` first opens in `area`.
fn first_place(kind: Kind, area: IRect) -> IRect {
    let w = area.width();
    match kind {
        Kind::Console => IRect::sized(area.x0 + 8, area.y1 - 210, (w - 16).min(440), 200),
        Kind::Graph => IRect::sized(area.x1 - 8 - (w - 16).min(400), area.y0 + 60, (w - 16).min(400), 220),
        Kind::Watch => IRect::sized(area.x1 - 8 - 280, area.y1 - 210, 280, 200),
    }
}

impl Debugger {
    /// Window `kind`, if it's open.
    #[must_use]
    pub(super) fn window(&self, kind: Kind) -> Option<Win> {
        self.windows.iter().find(|w| w.kind == kind).copied()
    }

    fn window_mut(&mut self, kind: Kind) -> Option<&mut Win> {
        self.windows.iter_mut().find(|w| w.kind == kind)
    }

    /// Whether the tools' windows (or popup) are over `p`: the worksheet
    /// isn't there then.
    #[must_use]
    pub fn covers(&self, p: (i32, i32)) -> bool {
        self.windows.iter().any(|w| w.rect.contains(p.0, p.1))
            || self.popup.as_ref().is_some_and(|m| m.rect.contains(p.0, p.1))
    }

    /// The windows, the topmost last, and the popup over them (frame
    /// coordinates).
    pub fn draw_windows(&self, s: &mut Surface, assets: &Assets) {
        let Some(font) = ui::font(assets) else { return };
        let Some(col) = self.colours else { return };
        let top = self.windows.len().saturating_sub(1);
        for (i, w) in self.windows.iter().enumerate() {
            let active = i == top && self.focus == Some(w.kind);
            let title = match w.kind {
                Kind::Console => tr("Console"),
                Kind::Graph => tr("Graph"),
                Kind::Watch => tr("Watch List"),
            };
            window::draw(s, font, w.rect, title, active);
            let old = s.set_clip(window::client(w.rect).intersect(s.clip()));
            match w.kind {
                Kind::Console => self.console.draw(s, font, w.rect, self.console.focused && active, col),
                Kind::Graph => self.graph.draw(s, font, w.rect, col, &self.observed.probes, self.time),
                Kind::Watch => watch::draw(s, font, w.rect, col, &self.observed),
            }
            s.set_clip(old);
        }
        if let Some(p) = &self.popup {
            context::draw(s, font, p);
        }
    }
}
