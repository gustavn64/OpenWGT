//! The console (Debug ▸ Console): a modeless window with the widget's
//! scripts (`scripts`), the log of what scripts said, in colour, the
//! scripts running (each with a stop box), and a line to type commands
//! into, with its history (Up, Down), completion (Tab) and the clipboard
//! (Ctrl+V). Ctrl+Enter (or Shift+Enter) starts another line in it, for
//! a script of several lines; Enter runs them all. Such a script is the
//! console's last script, which the Scripts list offers to keep in the
//! widget.
//!
//! A line starting with `:` is in the built-in command language
//! ([`crate::script::cmd`]); any other goes to the program's language
//! ([`Editor::set_script_runtime`]), or without one to the built-in
//! language too. Typing comes as text ([`Editor::text_input`]), in the
//! keyboard's own layout; the keys only edit.

use std::collections::VecDeque;

use formats::fnt::Font;

use super::super::{Editor, TextField, ui};
use super::window::{self, Colours};
use crate::gfx::{IRect, Surface};
use crate::lang::tr;
use crate::script::runtime::{Runtime, TaskId, TaskInfo, TaskState};
use crate::script::{Host, Level, api, select, value};

/// The most lines kept, and characters in them.
const MAX_LINES: usize = 2000;
const MAX_BYTES: usize = 200 * 1024;
/// The most lines the history keeps.
const MAX_HISTORY: usize = 100;
/// The most characters the line takes (all its lines).
pub const MAX_INPUT: usize = 8000;
/// The most rows of it shown at once.
const MAX_INPUT_ROWS: usize = 6;
/// The most completions listed.
const MAX_LISTED: usize = 40;
/// The scripts' list shows at most so many.
const MAX_TASKS: usize = 3;

const VK_BACK: u16 = 0x08;
const VK_TAB: u16 = 0x09;
const VK_RETURN: u16 = 0x0d;
const VK_ESCAPE: u16 = 0x1b;
const VK_PRIOR: u16 = 0x21;
const VK_NEXT: u16 = 0x22;
const VK_END: u16 = 0x23;
const VK_HOME: u16 = 0x24;
const VK_LEFT: u16 = 0x25;
const VK_UP: u16 = 0x26;
const VK_RIGHT: u16 = 0x27;
const VK_DOWN: u16 = 0x28;
const VK_DELETE: u16 = 0x2e;

/// What a line of the log is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// A line typed, after `> `.
    Input,
    Info,
    Warn,
    Error,
    /// A value asked for.
    Result,
}

/// A line of the log; `mark` underlines characters of it with carets on
/// a row of their own (where an error is): the first and how many.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub kind: LineKind,
    pub text: String,
    pub mark: Option<(usize, usize)>,
}

/// Which language a task is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    /// The built-in commands.
    Commands,
    /// The program's ([`Editor::set_script_runtime`]).
    Program,
}

/// The languages the console and the tools run, each out while it runs
/// (`None` then), as the runtime needs the editor it belongs to.
pub struct Runtimes {
    pub commands: Option<Box<dyn Runtime>>,
    pub program: Option<Box<dyn Runtime>>,
    /// Whether the program gave a language (it may be out), and its name.
    pub has_program: bool,
    pub program_language: Option<&'static str>,
    /// One is running: the frames' hook leaves them be.
    pub busy: bool,
}

impl Default for Runtimes {
    fn default() -> Runtimes {
        Runtimes {
            commands: Some(Box::new(crate::script::cmd::CommandRuntime::new())),
            program: None,
            has_program: false,
            program_language: None,
            busy: false,
        }
    }
}

impl Runtimes {
    fn slot(&mut self, l: Lang) -> &mut Option<Box<dyn Runtime>> {
        match l {
            Lang::Commands => &mut self.commands,
            Lang::Program => &mut self.program,
        }
    }

    /// The scripts running or waiting, in either language.
    #[must_use]
    pub fn live(&self) -> Vec<(Lang, TaskInfo)> {
        let mut v = Vec::new();
        for (l, rt) in [(Lang::Commands, &self.commands), (Lang::Program, &self.program)] {
            if let Some(rt) = rt {
                v.extend(
                    rt.tasks()
                        .into_iter()
                        .filter(|t| matches!(t.state, TaskState::Running | TaskState::Sleeping { .. }))
                        .map(|t| (l, t)),
                );
            }
        }
        v
    }

    /// Stops `task` of `l`, or every one.
    pub fn cancel(&mut self, which: Option<(Lang, TaskId)>) {
        for l in [Lang::Commands, Lang::Program] {
            if let Some(rt) = self.slot(l) {
                match which {
                    None => rt.cancel(None),
                    Some((wl, id)) if wl == l => rt.cancel(Some(id)),
                    Some(_) => {}
                }
            }
        }
    }
}

/// The console's log and line.
#[derive(Debug, Default)]
pub struct Console {
    pub lines: VecDeque<Line>,
    bytes: usize,
    /// The line being typed, and the caret in it (a byte index).
    pub input: String,
    pub caret: usize,
    history: Vec<String>,
    /// Where Up and Down are in the history, and the line typed before.
    hist_at: Option<usize>,
    draft: String,
    /// Rows scrolled back from the end.
    scroll: usize,
    /// The scripts listed and their stop boxes, as last drawn.
    pub tasks: Vec<(Lang, TaskInfo)>,
    /// The Scripts list's rows, as last drawn (`scripts`).
    pub rows: Vec<super::scripts::Row>,
    /// The last script run or loaded here, which Keep puts in the widget.
    pub last: Option<crate::port_parts::ext::Script>,
    /// Whether it has the keyboard (as last drawn).
    pub focused: bool,
}

impl Console {
    /// Adds `text` (lines apart by `\n`) as `kind`. An error's place, as a
    /// script error shows it (`7 | line` then `  | ^^^^`), becomes a mark
    /// under its line.
    pub fn add(&mut self, kind: LineKind, text: &str) {
        let rows: Vec<&str> = text.split('\n').collect();
        let mut i = 0;
        while i < rows.len() {
            let row = rows[i].trim_end_matches('\r');
            let mut line = Line {
                kind,
                text: row.to_owned(),
                mark: None,
            };
            if let Some(next) = rows.get(i + 1)
                && let Some(mark) = carets(row, next)
            {
                line.mark = Some(mark);
                i += 1;
            }
            self.push(line);
            i += 1;
        }
    }

    fn push(&mut self, line: Line) {
        self.bytes += line.text.len();
        self.lines.push_back(line);
        while self.lines.len() > MAX_LINES || self.bytes > MAX_BYTES {
            let Some(old) = self.lines.pop_front() else { break };
            self.bytes -= old.text.len();
        }
        self.scroll = 0;
    }

    /// The log as text, the marks as caret rows under their lines.
    #[must_use]
    pub fn text(&self) -> String {
        let mut s = String::new();
        for l in &self.lines {
            s.push_str(&l.text);
            s.push('\n');
            if let Some((at, n)) = l.mark {
                s.push_str(&" ".repeat(at));
                s.push_str(&"^".repeat(n.max(1)));
                s.push('\n');
            }
        }
        s
    }

    /// A new line at the caret (Ctrl+Enter), if there's room.
    fn new_line(&mut self) {
        if self.input.chars().count() < MAX_INPUT {
            self.input.insert(self.caret, '\n');
            self.caret += 1;
            self.hist_at = None;
        }
    }

    /// Where the caret's line starts and ends.
    fn line_bounds(&self) -> (usize, usize) {
        let start = self.input[..self.caret].rfind('\n').map_or(0, |i| i + 1);
        let end = self.input[self.caret..]
            .find('\n')
            .map_or(self.input.len(), |i| self.caret + i);
        (start, end)
    }

    /// The caret a line up (`up`) or down, at the same character of it;
    /// false at the first or last line.
    fn line_move(&mut self, up: bool) -> bool {
        let (start, end) = self.line_bounds();
        let col = self.input[start..self.caret].chars().count();
        let (from, to) = if up {
            if start == 0 {
                return false;
            }
            let s = self.input[..start - 1].rfind('\n').map_or(0, |i| i + 1);
            (s, start - 1)
        } else {
            if end == self.input.len() {
                return false;
            }
            let e = self.input[end + 1..]
                .find('\n')
                .map_or(self.input.len(), |i| end + 1 + i);
            (end + 1, e)
        };
        let line = &self.input[from..to];
        self.caret = from + line.char_indices().nth(col).map_or(line.len(), |(i, _)| i);
        true
    }

    /// How many rows the line has.
    #[must_use]
    pub fn input_rows(&self) -> usize {
        self.input.matches('\n').count() + 1
    }

    /// The first row of the line shown, `shown` rows of it showing: so
    /// the caret's row shows.
    fn first_row(&self, shown: usize) -> usize {
        let at = self.input[..self.caret].matches('\n').count();
        at.saturating_sub(shown.saturating_sub(1))
            .min(self.input_rows().saturating_sub(shown))
    }

    /// Text typed at the caret.
    pub fn type_text(&mut self, t: &str) {
        let room = MAX_INPUT.saturating_sub(self.input.chars().count());
        let t: String = t
            .chars()
            .map(|c| if c == '\t' { ' ' } else { c })
            .filter(|c| !c.is_control())
            .take(room)
            .collect();
        self.input.insert_str(self.caret, &t);
        self.caret += t.len();
        self.hist_at = None;
    }

    fn set_input(&mut self, t: &str) {
        t.clone_into(&mut self.input);
        self.caret = self.input.len();
    }

    fn prev_char(&self) -> usize {
        self.input[..self.caret].char_indices().last().map_or(0, |(i, _)| i)
    }

    fn next_char(&self) -> usize {
        self.input[self.caret..]
            .chars()
            .next()
            .map_or(self.caret, |c| self.caret + c.len_utf8())
    }

    /// Up (`back`) or Down in the history.
    fn recall(&mut self, back: bool) {
        if self.history.is_empty() {
            return;
        }
        let at = match (self.hist_at, back) {
            (None, true) => {
                self.draft = self.input.clone();
                Some(self.history.len() - 1)
            }
            (Some(i), true) => Some(i.saturating_sub(1)),
            (Some(i), false) if i + 1 < self.history.len() => Some(i + 1),
            (None | Some(_), false) => None,
        };
        self.hist_at = at;
        let t = at.map_or_else(|| self.draft.clone(), |i| self.history[i].clone());
        self.set_input(&t);
    }

    /// The keys that only edit the line, move in the history or scroll.
    fn edit_key(&mut self, vk: u16, ctrl: bool) {
        match vk {
            VK_UP => {
                if !self.line_move(true) {
                    self.recall(true);
                }
            }
            VK_DOWN => {
                if !self.line_move(false) {
                    self.recall(false);
                }
            }
            VK_LEFT => self.caret = self.prev_char(),
            VK_RIGHT => self.caret = self.next_char(),
            VK_HOME => self.caret = if ctrl { 0 } else { self.line_bounds().0 },
            VK_END => self.caret = if ctrl { self.input.len() } else { self.line_bounds().1 },
            VK_PRIOR => self.scroll += 5,
            VK_NEXT => self.scroll = self.scroll.saturating_sub(5),
            VK_BACK => {
                let p = self.prev_char();
                self.input.replace_range(p..self.caret, "");
                self.caret = p;
            }
            VK_DELETE => {
                let n = self.next_char();
                self.input.replace_range(self.caret..n, "");
            }
            // Ctrl+L: the log cleared.
            0x4c if ctrl => {
                self.lines.clear();
                self.bytes = 0;
            }
            _ => {}
        }
    }

    /// The line typed, taken out (and kept in the history).
    fn take_line(&mut self) -> String {
        let line = std::mem::take(&mut self.input);
        self.caret = 0;
        self.hist_at = None;
        if !line.trim().is_empty() && self.history.last() != Some(&line) {
            self.history.push(line.clone());
            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }
        }
        line
    }

    /// Where the word the caret ends starts.
    fn word_start(&self) -> usize {
        self.input[..self.caret].rfind(char::is_whitespace).map_or(0, |i| i + 1)
    }
}

/// The mark the caret row `next` puts under `row`: `row` reads ` 7 | …`
/// and `next` `   | ^^^`, the same up to their bars.
fn carets(row: &str, next: &str) -> Option<(usize, usize)> {
    let bar = next.find('|')?;
    if !next[..bar].trim().is_empty() || row.find('|')? != bar {
        return None;
    }
    let after = &next[bar + 1..];
    let first = after.find('^')?;
    if !after[..first].chars().all(|c| c == ' ' || c == '\t') {
        return None;
    }
    let n = after[first..].chars().take_while(|&c| c == '^').count();
    if after[first + n..].trim().is_empty() {
        let at = next[..bar + 1 + first].chars().count();
        Some((at, n))
    } else {
        None
    }
}

/// What the console's window has where: the Scripts list's rows, the
/// tasks' rows (each with its stop box), the log, the line.
pub struct Layout {
    pub rows: Vec<IRect>,
    pub tasks: Vec<(IRect, IRect)>,
    pub log: IRect,
    pub input: IRect,
}

/// The console's parts in the window at `r`, with `rows` rows of the
/// Scripts list, `tasks` tasks and a line of `input_rows` rows.
#[must_use]
pub fn layout(r: IRect, font: &Font, rows: usize, tasks: usize, input_rows: usize) -> Layout {
    let c = window::client(r).inset(2);
    let lh = i32::from(font.height);
    let row = lh + 4;
    let m = i32::try_from(rows.min(super::scripts::MAX_ROWS)).unwrap_or(0);
    let rows = (0..m)
        .map(|i| IRect::sized(c.x0, c.y0 + i * (row + 2), c.width(), row + 2))
        .collect();
    let top = c.y0 + m * (row + 2);
    let n = i32::try_from(tasks.min(MAX_TASKS)).unwrap_or(0);
    let tasks = (0..n)
        .map(|i| {
            let r = IRect::sized(c.x0, top + i * row, c.width(), row);
            let stop = IRect::sized(r.x1 - 40, r.y0 + 1, 40, row - 2);
            (r, stop)
        })
        .collect();
    let shown = i32::try_from(input_rows.clamp(1, MAX_INPUT_ROWS)).unwrap_or(1);
    let input = IRect::new(c.x0, c.y1 - shown * lh - 6, c.x1, c.y1);
    let log = IRect::new(c.x0, top + n * row, c.x1, input.y0 - 3);
    Layout {
        rows,
        tasks,
        log,
        input,
    }
}

/// The prompt before the line.
const PROMPT: &str = "> ";

/// What a task's row says: `console:3 · line 2 · waits 120 ms`.
fn task_text(t: &TaskInfo) -> String {
    let state = match t.state {
        TaskState::Running => tr("runs").to_owned(),
        TaskState::Sleeping { ms } => format!("{} {ms} ms", tr("waits")),
        TaskState::Done | TaskState::Failed => String::new(),
    };
    format!("{} · {} {} · {state}", t.file, tr("line"), t.line)
}

impl Console {
    /// Its parts in its window `r`, as they are now.
    #[must_use]
    pub fn layout(&self, r: IRect, font: &Font) -> Layout {
        layout(r, font, self.rows.len(), self.tasks.len(), self.input_rows())
    }

    /// Draws the console in its window `r`, its line's caret shown while it
    /// has the keyboard.
    pub fn draw(&self, s: &mut Surface, font: &Font, r: IRect, focused: bool, col: Colours) {
        let l = self.layout(r, font);
        let lh = i32::from(font.height);
        for (rr, row) in l.rows.iter().zip(&self.rows) {
            super::scripts::draw_row(s, font, *rr, row, col);
        }
        for ((row, stop), (_, t)) in l.tasks.iter().zip(&self.tasks) {
            let old = s.set_clip(IRect::new(row.x0, row.y0, stop.x0 - 2, row.y1).intersect(s.clip()));
            ui::label(s, font, row.x0 + 2, row.y0 + 2, &task_text(t), ui::BLACK);
            s.set_clip(old);
            ui::button(s, font, *stop, tr("Stop"), false, false, true);
        }
        // The log, from the end up.
        ui::sunken(s, l.log);
        let inner = l.log.inset(2);
        s.fill(inner, ui::WHITE);
        let old = s.set_clip(inner.intersect(s.clip()));
        let rows = self.rows(font, inner.width() - 4);
        let fit = usize::try_from(inner.height() / lh.max(1)).unwrap_or(0);
        let end = rows
            .len()
            .saturating_sub(self.scroll.min(rows.len().saturating_sub(fit)));
        let start = end.saturating_sub(fit);
        let mut y = inner.y1 - i32::try_from(end - start).unwrap_or(0) * lh;
        for row in &rows[start..end] {
            let c = match row.kind {
                LineKind::Input => col.blue,
                LineKind::Info => ui::BLACK,
                LineKind::Warn => col.orange,
                LineKind::Error => col.red,
                LineKind::Result => col.green,
            };
            match &row.carets {
                Some((x, w)) => {
                    // `^`s under the characters marked.
                    let x0 = inner.x0 + 2 + x;
                    let step = ui::width(font, "^").max(1);
                    let mut cx = x0;
                    while cx < x0 + (*w).max(step) {
                        ui::label(s, font, cx, y, "^", col.red);
                        cx += step;
                    }
                }
                None => {
                    ui::label(s, font, inner.x0 + 2, y, &row.text, c);
                }
            }
            y += lh;
        }
        s.set_clip(old);
        // The line, the rows about the caret's.
        ui::sunken(s, l.input);
        let field = l.input.inset(2);
        s.fill(field, ui::WHITE);
        let old = s.set_clip(field.intersect(s.clip()));
        let x0 = field.x0 + 2 + ui::width(font, PROMPT);
        let room = field.x1 - 4 - x0;
        let shown = self.input_rows().min(MAX_INPUT_ROWS);
        let first = self.first_row(shown);
        let (cs, _) = self.line_bounds();
        let mut start = 0;
        for (row, line) in self.input.split('\n').enumerate() {
            let line_start = start;
            start += line.len() + 1;
            if row < first || row >= first + shown {
                continue;
            }
            let y = field.y0 + 1 + i32::try_from(row - first).unwrap_or(0) * lh;
            if row == 0 {
                ui::label(s, font, field.x0 + 2, y, PROMPT, col.blue);
            }
            // The caret's row scrolled so the caret shows.
            let mut from = 0;
            if line_start == cs {
                let at = self.caret - cs;
                while from < at && ui::width(font, &line[from..at]) > room {
                    from += line[from..].chars().next().map_or(1, char::len_utf8);
                }
                if focused {
                    let cx = x0 + ui::width(font, &line[from..at]);
                    s.fill(IRect::new(cx, y, cx + 1, y + lh), ui::BLACK);
                }
            }
            ui::label(s, font, x0, y, &line[from..], ui::BLACK);
        }
        s.set_clip(old);
    }

    /// The log's lines as drawn rows `w` wide: long ones wrapped, a mark's
    /// carets on a row of their own (where in pixels, how wide).
    fn rows(&self, font: &Font, w: i32) -> Vec<Row> {
        let mut rows = Vec::new();
        for l in &self.lines {
            if let Some((at, n)) = l.mark {
                rows.push(Row {
                    kind: l.kind,
                    text: l.text.clone(),
                    carets: None,
                });
                let before: String = l.text.chars().take(at).collect();
                let marked: String = l.text.chars().skip(at).take(n.max(1)).collect();
                let x = ui::width(font, &before);
                let mw = ui::width(font, &marked);
                rows.push(Row {
                    kind: l.kind,
                    text: String::new(),
                    carets: Some((x, mw)),
                });
                continue;
            }
            for r in ui::wrap_ranges(font, &l.text, w) {
                rows.push(Row {
                    kind: l.kind,
                    text: l.text[r].to_owned(),
                    carets: None,
                });
            }
        }
        rows
    }
}

struct Row {
    kind: LineKind,
    text: String,
    carets: Option<(i32, i32)>,
}

impl Editor {
    /// The console's log, a line each: what was typed and what came of it.
    #[must_use]
    pub fn console_text(&self) -> String {
        self.debug.console.text()
    }

    /// The program's own scripting language for the console's lines (the
    /// built-in commands stay for lines starting with `:`).
    pub fn set_script_runtime(&mut self, rt: Box<dyn Runtime>) {
        self.debug.runtimes.program_language = Some(rt.language());
        self.debug.runtimes.program = Some(rt);
        self.debug.runtimes.has_program = true;
    }

    /// Runs `f` with runtime `l` taken out of the editor (so it can have
    /// the editor as its host), then puts it back; `None` if it's out.
    pub(super) fn with_runtime<T>(&mut self, l: Lang, f: impl FnOnce(&mut dyn Runtime, &mut Editor) -> T) -> Option<T> {
        let mut rt = self.debug.runtimes.slot(l).take()?;
        let was = std::mem::replace(&mut self.debug.runtimes.busy, true);
        let r = f(rt.as_mut(), self);
        self.debug.runtimes.busy = was;
        *self.debug.runtimes.slot(l) = Some(rt);
        Some(r)
    }

    /// A frame of `ms` passed: the scripts run on (none while one of
    /// them runs). Without the tools the console's are stopped; the
    /// widget's own run on.
    pub(in crate::editor) fn debug_frame(&mut self, ms: u64) {
        if self.debug.runtimes.busy {
            return;
        }
        if !self.debugging() {
            let live = self.debug.runtimes.live();
            if live.is_empty() {
                return;
            }
            for (l, t) in &live {
                if !self.is_script_task(*l, t.id, true) {
                    self.debug.runtimes.cancel(Some((*l, t.id)));
                }
            }
            if self.debug.runtimes.live().is_empty() {
                return;
            }
        }
        for l in [Lang::Commands, Lang::Program] {
            self.with_runtime(l, |rt, e| rt.frame(e, ms));
        }
    }

    /// Stops the console's scripts, saying so if there were any.
    pub(super) fn stop_console_scripts(&mut self) {
        let n = self.debug.runtimes.live().len();
        self.debug.runtimes.cancel(None);
        if n > 0 {
            self.debug.console.add(LineKind::Warn, tr("Scripts stopped."));
        }
    }

    /// Runs the line typed in the console.
    pub(super) fn console_submit(&mut self) {
        let line = self.debug.console.take_line();
        let shown = line.replace('\n', &format!("\n{}", " ".repeat(PROMPT.len())));
        self.debug.console.add(LineKind::Input, &format!("{PROMPT}{shown}"));
        let text = line.trim();
        if text.is_empty() {
            return;
        }
        let (l, text) = match text.strip_prefix(':') {
            Some(t) => (Lang::Commands, t.trim()),
            None if self.debug.runtimes.has_program => (Lang::Program, text),
            None => (Lang::Commands, text),
        };
        if l == Lang::Commands && self.console_value(text) {
            return;
        }
        let file = "console";
        let language = match l {
            Lang::Commands => Some(crate::script::cmd::LANGUAGE),
            Lang::Program => self.debug.runtimes.program_language,
        };
        // A script of several lines is the console's last script.
        if let Some(language) = language.filter(|_| text.contains('\n')) {
            self.set_last_script(crate::port_parts::ext::Script {
                name: file.to_owned(),
                language: language.to_owned(),
                source: text.to_owned(),
            });
        }
        let r = self.with_runtime(l, |rt, e| rt.repl(text, e));
        match r {
            Some(Ok(Some(v))) => self.debug.console.add(LineKind::Result, &v),
            Some(Ok(None)) => {}
            Some(Err(e)) => {
                let mut e = e;
                if e.file.is_empty() {
                    file.clone_into(&mut e.file);
                }
                self.debug.console.add(LineKind::Error, &e.to_string());
            }
            None => self
                .debug
                .console
                .add(LineKind::Warn, tr("A script is running; try again when it waits.")),
        }
    }

    /// A line that isn't a command but names a part or a knot: its value,
    /// as `Lamp = on`. Whether it was one.
    fn console_value(&mut self, text: &str) -> bool {
        let first = text.split_whitespace().next().unwrap_or("");
        if api::command(first).is_some() {
            return false;
        }
        let Ok(sel) = select::parse(text) else { return false };
        match self.value(&sel) {
            Ok(v) => {
                let s = format!("{} = {}", select::show(&sel), value::format(&v));
                self.debug.console.add(LineKind::Result, &s);
                true
            }
            // A command misspelt, more likely: the language says.
            Err(_) => false,
        }
    }

    /// Tab: the word the caret ends completed, or the ways it could go
    /// listed (and as much of it as they share filled in).
    fn console_complete(&mut self) {
        let c = &self.debug.console;
        let (input, caret, start) = (c.input.clone(), c.caret, c.word_start());
        let (l, line, cut) = match input.strip_prefix(':') {
            Some(rest) => (Lang::Commands, rest.to_owned(), 1),
            None if self.debug.runtimes.has_program => (Lang::Program, input.clone(), 0),
            None => (Lang::Commands, input.clone(), 0),
        };
        let at = caret.saturating_sub(cut);
        let Some(mut options) = self.with_runtime(l, |rt, e| rt.complete(&line, at, e)) else {
            return;
        };
        options.sort();
        options.dedup();
        let word = &input[start.max(cut)..caret];
        let fill = match options.as_slice() {
            [] => return,
            [one] => format!("{one} "),
            many => {
                let mut shown: Vec<&str> = many.iter().take(MAX_LISTED).map(String::as_str).collect();
                if many.len() > MAX_LISTED {
                    shown.push("…");
                }
                let list = shown.join("  ");
                self.debug.console.add(LineKind::Info, &list);
                common_prefix(many)
            }
        };
        if fill.trim_end().len() < word.len() {
            return;
        }
        let c = &mut self.debug.console;
        let from = start.max(cut);
        c.input.replace_range(from..caret, &fill);
        c.caret = from + fill.len();
    }

    /// A key while the console has the keyboard; whether it took it (it
    /// takes all but the tools' own and the menus').
    pub(super) fn console_key(&mut self, vk: u16) -> bool {
        let ctrl = self.ctrl && !self.alt;
        match vk {
            VK_RETURN if ctrl || self.shift => self.debug.console.new_line(),
            VK_RETURN => self.console_submit(),
            VK_ESCAPE => {
                if !self.debug.runtimes.live().is_empty() || self.debug.scripts_running {
                    self.debug.stop_scripts = self.debug.scripts_running;
                    self.stop_console_scripts();
                } else if !self.debug.console.input.is_empty() {
                    self.debug.console.set_input("");
                } else {
                    self.debug.focus = None;
                }
            }
            VK_TAB => self.console_complete(),
            // Ctrl+V: the clipboard's text, asked for (`paste_link`).
            0x56 if ctrl => {
                if let Some(l) = &mut self.links {
                    self.paste_to_field = true;
                    (l.paste)();
                }
            }
            // Ctrl+C: the line, else the log's last line, copied.
            0x43 if ctrl => {
                let c = &self.debug.console;
                let t = if c.input.is_empty() {
                    c.lines.back().map(|l| l.text.clone()).unwrap_or_default()
                } else {
                    c.input.clone()
                };
                if let Some(l) = &mut self.links {
                    let _ = (l.copy)(&t);
                }
            }
            _ => self.debug.console.edit_key(vk, ctrl),
        }
        true
    }

    /// The clipboard's text pasted into the console's line: each whole
    /// line run in turn, the last left to edit.
    pub(super) fn console_paste(&mut self, text: &str) {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let mut parts = text.split('\n').peekable();
        while let Some(p) = parts.next() {
            self.debug.console.type_text(p);
            if parts.peek().is_some() {
                self.console_submit();
            }
        }
    }

    /// The console's line, for a touch screen's own text box.
    pub(super) fn console_field(&self) -> TextField {
        TextField {
            text: self.debug.console.input.clone(),
            multiline: false,
            max: MAX_INPUT,
        }
    }

    /// The console's line as edited elsewhere.
    pub(super) fn set_console_line(&mut self, text: &str) {
        let one: String = text.chars().filter(|c| !c.is_control()).take(MAX_INPUT).collect();
        self.debug.console.set_input(&one);
    }

    /// The scripts' list, for the console to show.
    pub(super) fn console_tasks(&self) -> Vec<(Lang, TaskInfo)> {
        let mut v = self.debug.runtimes.live();
        // The Scripts list's show there.
        v.retain(|(l, t)| !self.is_script_task(*l, t.id, false));
        v.truncate(MAX_TASKS);
        v
    }

    /// What scripts write, in the console too (with the tools).
    pub(super) fn console_heard(&mut self, level: Level, msg: &str) {
        if !self.debugging() {
            return;
        }
        let kind = match level {
            Level::Info => LineKind::Info,
            Level::Warn => LineKind::Warn,
            Level::Error => LineKind::Error,
        };
        self.debug.console.add(kind, msg);
    }

    /// A click in the console's window `r` at `p`: a stop box, the line
    /// (which a touch screen then edits in its own box).
    pub(super) fn console_click(&mut self, r: IRect, p: (i32, i32)) {
        let Some(font) = ui::font(&self.assets) else { return };
        let c = &self.debug.console;
        let l = c.layout(r, font);
        let rows = c.rows.clone();
        for (rr, row) in l.rows.iter().zip(&rows) {
            let (run, other) = super::scripts::buttons(font, *rr, row.which);
            if run.contains(p.0, p.1) {
                self.script_press(row.which, super::scripts::Press::RunStop);
                return;
            }
            if other.contains(p.0, p.1) {
                self.script_press(row.which, super::scripts::Press::Other);
                return;
            }
        }
        let tasks = self.debug.console.tasks.clone();
        for ((_, stop), (lang, t)) in l.tasks.iter().zip(&tasks) {
            if stop.contains(p.0, p.1) {
                self.debug.runtimes.cancel(Some((*lang, t.id)));
                return;
            }
        }
        if l.input.contains(p.0, p.1) {
            self.text_tapped = true;
            // The caret to the character clicked, in the row clicked.
            let field = l.input.inset(2);
            let x0 = field.x0 + 2 + ui::width(font, PROMPT);
            let lh = i32::from(font.height).max(1);
            let c = &mut self.debug.console;
            let shown = c.input_rows().min(MAX_INPUT_ROWS);
            let row = c.first_row(shown) + usize::try_from((p.1 - field.y0 - 1).max(0) / lh).unwrap_or(0);
            let row = row.min(c.input_rows() - 1);
            let start: usize = c.input.split('\n').take(row).map(|l| l.len() + 1).sum();
            let line = c.input[start..].split('\n').next().unwrap_or("");
            let mut at = start;
            for (i, ch) in line.char_indices() {
                let w = ui::width(font, &line[..i + ch.len_utf8()]);
                if x0 + w > p.0 + 2 {
                    break;
                }
                at = start + i + ch.len_utf8();
            }
            c.caret = at;
        }
    }

    /// The wheel over the console: the log scrolls.
    pub(super) fn console_wheel(&mut self, dy: i32) {
        let c = &mut self.debug.console;
        let n = usize::try_from(dy.unsigned_abs()).unwrap_or(0).max(1) * 3;
        if dy > 0 {
            c.scroll += n;
        } else {
            c.scroll = c.scroll.saturating_sub(n);
        }
    }
}

/// What `words` all start with.
fn common_prefix(words: &[String]) -> String {
    let Some(first) = words.first() else {
        return String::new();
    };
    let mut n = first.len();
    for w in &words[1..] {
        n = first
            .char_indices()
            .zip(w.chars())
            .take_while(|((_, a), b)| a == b)
            .last()
            .map_or(0, |((i, a), _)| i + a.len_utf8())
            .min(n);
    }
    first[..n].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carets_become_marks() {
        let mut c = Console::default();
        c.add(
            LineKind::Error,
            "console:1:7  no part called \"Lamb\"\n 1 | press Lamb\n   |       ^^^^",
        );
        assert_eq!(c.lines.len(), 2);
        assert_eq!(c.lines[1].text, " 1 | press Lamb");
        assert_eq!(c.lines[1].mark, Some((11, 4)));
        assert_eq!(
            c.text(),
            "console:1:7  no part called \"Lamb\"\n 1 | press Lamb\n           ^^^^\n"
        );
    }

    #[test]
    fn the_log_is_capped() {
        let mut c = Console::default();
        for i in 0..MAX_LINES + 10 {
            c.add(LineKind::Info, &i.to_string());
        }
        assert_eq!(c.lines.len(), MAX_LINES);
        assert_eq!(c.lines[0].text, "10");
    }

    #[test]
    fn history_goes_back_and_forth() {
        let mut c = Console::default();
        for l in ["run", "step 5"] {
            c.type_text(l);
            c.take_line();
        }
        c.type_text("pa");
        c.recall(true);
        assert_eq!(c.input, "step 5");
        c.recall(true);
        assert_eq!(c.input, "run");
        c.recall(true);
        assert_eq!(c.input, "run");
        c.recall(false);
        c.recall(false);
        assert_eq!(c.input, "pa", "the line typed before comes back");
    }

    #[test]
    fn shared_starts() {
        let w = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(common_prefix(&w(&["step", "stop", "setting"])), "s");
        assert_eq!(common_prefix(&w(&["Lamp", "Lamp.in"])), "Lamp");
        assert_eq!(common_prefix(&w(&["åb", "åc"])), "å");
    }
}
