//! The scripts that come with a widget (`Document::scripts`, kept in it
//! hidden, `port_parts::ext`), in the console's Scripts list: a row each
//! at the top of the console, with ▶ to run it (■ to stop it while it
//! runs) and ✕ to take it out of the widget. Below them, while there is
//! one, the console's last script (a file dropped onto the window while
//! the tools are on, or the last line run), with Keep to put it in the
//! widget.
//!
//! A script of the widget's runs with what a script that came with a
//! widget may do ([`Caps::EMBEDDED`]): never open or load other widgets,
//! however it was started. The console's own (typed, dropped) may do
//! everything.

use super::super::{Editor, ui};
use super::LineKind;
use super::console::Lang;
use super::window::Colours;
use crate::gfx::{IRect, Surface};
use crate::lang::{tr, trf};
use crate::port_parts::ext::Script;
use crate::script::Caps;
use crate::script::runtime::TaskId;
use crate::script::trust::Digest;

/// The most rows the list shows.
pub const MAX_ROWS: usize = 5;
/// The longest script file taken.
const MAX_FILE: usize = 1 << 20;

/// A row of the Scripts list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub which: Which,
    pub text: String,
    pub running: bool,
}

/// What a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    /// The widget's script `n`.
    Widget(usize),
    /// The console's last script, not in the widget.
    Last,
}

/// A row's buttons: run or stop, then remove or keep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    RunStop,
    Other,
}

/// The language a script file is in, by its name: `.js` JavaScript,
/// `.txt` and `.cmd` the built-in commands; `None` for another file.
#[must_use]
pub fn file_language(name: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(name)
        .extension()?
        .to_string_lossy()
        .to_ascii_lowercase();
    match ext.as_str() {
        "js" | "mjs" => Some("JavaScript"),
        "txt" | "cmd" => Some(crate::script::cmd::LANGUAGE),
        _ => None,
    }
}

impl Editor {
    /// The runtime of `language`, if the editor has one.
    fn script_lang(&self, language: &str) -> Option<Lang> {
        if language == crate::script::cmd::LANGUAGE {
            Some(Lang::Commands)
        } else if self.debug.runtimes.program_language == Some(language) {
            Some(Lang::Program)
        } else {
            None
        }
    }

    /// Starts `s`, able to do what `caps` allow, as a task of its
    /// language; what goes wrong in the console. Whether it started.
    pub(in crate::editor) fn start_script(&mut self, s: &Script, caps: Caps) -> bool {
        let Some(l) = self.script_lang(&s.language) else {
            let text = trf("{}: there is no {} here to run it.", &[&s.name, &s.language]);
            self.debug.console.add(LineKind::Error, &text);
            return false;
        };
        match self.with_runtime(l, |rt, e| rt.start(&s.name, &s.source, caps, e)) {
            Some(Ok(id)) => {
                self.debug.script_tasks.push(super::ScriptTask {
                    digest: s.digest(),
                    lang: l,
                    id,
                    widget: !caps.document,
                });
                true
            }
            Some(Err(e)) => {
                self.debug.console.add(LineKind::Error, &e.to_string());
                false
            }
            None => {
                let text = tr("A script is running; try again when it waits.");
                self.debug.console.add(LineKind::Warn, text);
                false
            }
        }
    }

    /// The task running the script with the text `d`, if one is.
    fn script_task(&self, d: &Digest) -> Option<(Lang, TaskId)> {
        let live = self.debug.runtimes.live();
        self.debug
            .script_tasks
            .iter()
            .rev()
            .find(|s| s.digest == *d && live.iter().any(|(l, t)| *l == s.lang && t.id == s.id))
            .map(|s| (s.lang, s.id))
    }

    /// Whether `l`'s task `id` runs one of the listed scripts (one of the
    /// widget's, if `widget`).
    pub(super) fn is_script_task(&self, l: Lang, id: TaskId, widget: bool) -> bool {
        self.debug
            .script_tasks
            .iter()
            .any(|s| s.lang == l && s.id == id && (s.widget || !widget))
    }

    /// The console's last script, while the widget doesn't have it.
    fn loose_script(&self) -> Option<&Script> {
        self.debug
            .console
            .last
            .as_ref()
            .filter(|l| !self.doc.scripts.iter().any(|s| s.source == l.source))
    }

    /// The Scripts list's rows, as they are now.
    pub(super) fn script_rows(&mut self) -> Vec<Row> {
        let live = self.debug.runtimes.live();
        self.debug
            .script_tasks
            .retain(|s| live.iter().any(|(l, t)| *l == s.lang && t.id == s.id));
        let mut rows: Vec<Row> = self
            .doc
            .scripts
            .iter()
            .enumerate()
            .map(|(i, s)| Row {
                which: Which::Widget(i),
                text: format!("{} · {}", s.name, s.language),
                running: self.script_task(&s.digest()).is_some(),
            })
            .collect();
        rows.truncate(MAX_ROWS - 1);
        if let Some(s) = self.loose_script() {
            rows.push(Row {
                which: Which::Last,
                text: format!("{} · {} · {}", s.name, s.language, tr("not in the widget")),
                running: self.script_task(&s.digest()).is_some(),
            });
        }
        rows
    }

    /// A row's button pressed.
    pub(super) fn script_press(&mut self, which: Which, press: Press) {
        let (script, caps) = match which {
            Which::Widget(i) => match self.doc.scripts.get(i) {
                Some(s) => (s.clone(), Caps::EMBEDDED),
                None => return,
            },
            Which::Last => match self.loose_script() {
                Some(s) => (s.clone(), Caps::ALL),
                None => return,
            },
        };
        match press {
            Press::RunStop => {
                if let Some(t) = self.script_task(&script.digest()) {
                    self.debug.runtimes.cancel(Some(t));
                } else {
                    self.start_script(&script, caps);
                }
            }
            Press::Other => match which {
                Which::Widget(i) => self.remove_script(i),
                Which::Last => self.keep_last_script(),
            },
        }
    }

    /// The widget's script `i` taken out of it (stopped if it runs).
    pub fn remove_script(&mut self, i: usize) {
        if i >= self.doc.scripts.len() {
            return;
        }
        let s = self.doc.scripts.remove(i);
        if let Some(t) = self.script_task(&s.digest()) {
            self.debug.runtimes.cancel(Some(t));
        }
        self.set_dirty();
        let text = trf("{} is no longer in the widget.", &[&s.name]);
        self.debug.console.add(LineKind::Info, &text);
    }

    /// The console's last script put in the widget, under a name the
    /// widget's others don't have.
    pub fn keep_last_script(&mut self) {
        let Some(mut s) = self.loose_script().cloned() else {
            return;
        };
        let base = s.name.clone();
        let mut n = 1;
        while self.doc.scripts.iter().any(|o| o.name.eq_ignore_ascii_case(&s.name)) {
            n += 1;
            s.name = format!("{base} {n}");
        }
        let text = trf("{} is in the widget now; it is saved with it.", &[&s.name]);
        self.doc.scripts.push(s);
        self.set_dirty();
        self.debug.console.add(LineKind::Info, &text);
    }

    /// The console's last script is now `s` (a file loaded, a line run).
    pub(super) fn set_last_script(&mut self, s: Script) {
        self.debug.console.last = Some(s);
    }

    /// A file dropped onto the window while the tools are on: a script
    /// file (by its name, [`file_language`]) goes to the console, as its
    /// last script, not run. Whether it was one.
    pub(in crate::editor) fn console_drop(&mut self, name: &str, data: &[u8]) -> bool {
        if !self.debugging() {
            return false;
        }
        let Some(language) = file_language(name) else {
            return false;
        };
        self.open_window(super::window::Kind::Console);
        let base = std::path::Path::new(name)
            .file_name()
            .map_or_else(|| name.to_owned(), |n| n.to_string_lossy().into_owned());
        let Ok(text) = std::str::from_utf8(&data[..data.len().min(MAX_FILE)]) else {
            let msg = trf("{}: not a text file.", &[&base]);
            self.debug.console.add(LineKind::Error, &msg);
            return true;
        };
        let source = text.replace("\r\n", "\n");
        let lines = source.lines().count().to_string();
        let msg = trf(
            "{} loaded ({} lines): its row above runs it, or keeps it in the widget.",
            &[&base, &lines],
        );
        self.set_last_script(Script {
            name: base,
            language: language.to_owned(),
            source,
        });
        self.debug.console.add(LineKind::Info, &msg);
        true
    }
}

/// Where a row's two buttons are in the row `r`: run or stop, then
/// remove (a cross) or Keep (wider, with its word).
#[must_use]
pub fn buttons(font: &formats::fnt::Font, r: IRect, which: Which) -> (IRect, IRect) {
    let h = r.height() - 2;
    let other_w = match which {
        Which::Widget(_) => h + 4,
        Which::Last => ui::width(font, tr("Keep")) + 14,
    };
    let other = IRect::sized(r.x1 - other_w, r.y0 + 1, other_w, h);
    let run = IRect::sized(other.x0 - 3 - (h + 4), r.y0 + 1, h + 4, h);
    (run, other)
}

/// Draws row `row` in `r`.
pub fn draw_row(s: &mut Surface, font: &formats::fnt::Font, r: IRect, row: &Row, col: Colours) {
    let (run, other) = buttons(font, r, row.which);
    let old = s.set_clip(IRect::new(r.x0, r.y0, run.x0 - 2, r.y1).intersect(s.clip()));
    let c = match row.which {
        Which::Widget(_) => ui::BLACK,
        Which::Last => col.blue,
    };
    ui::label(s, font, r.x0 + 2, r.y0 + 2, &row.text, c);
    s.set_clip(old);
    ui::button(s, font, run, "", false, false, true);
    let g = run.inset(4);
    if row.running {
        // ■
        let side = g.height().min(g.width());
        let x = run.x0 + (run.width() - side) / 2;
        s.fill(IRect::sized(x, g.y0, side, side), col.red);
    } else {
        // ▶
        let hgt = g.height();
        let x0 = run.x0 + (run.width() - hgt / 2 - 1) / 2;
        for y in 0..hgt {
            let reach = y.min(hgt - 1 - y);
            s.fill(IRect::new(x0, g.y0 + y, x0 + reach + 1, g.y0 + y + 1), col.green);
        }
    }
    match row.which {
        Which::Widget(_) => {
            ui::button(s, font, other, "", false, false, true);
            // ✕
            let g = other.inset(6);
            let n = g.width().min(g.height());
            let (x, y) = (other.x0 + (other.width() - n) / 2, other.y0 + (other.height() - n) / 2);
            for i in 0..n {
                s.fill(IRect::sized(x + i, y + i, 2, 1), ui::BLACK);
                s.fill(IRect::sized(x + n - 1 - i, y + i, 2, 1), ui::BLACK);
            }
        }
        Which::Last => ui::button(s, font, other, tr("Keep"), false, false, true),
    }
}
