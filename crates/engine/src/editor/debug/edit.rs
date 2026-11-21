//! The script editor: a window of the tools where a scripted part's
//! script (a double-click on the part) or one of the widget's scripts (a
//! click on its name in the console's Scripts list) is written, in the
//! console's fixed-pitch font and colours, with line numbers. Apply
//! (Ctrl+S, vim's `:w`) gives the part or the widget the text; a script
//! that doesn't define a part says why under the text and the part keeps
//! its last one. Closed without applying, the text waits for the next
//! time it's opened. The keys are a plain editor's or, with vim keys on
//! in Settings, vim's ([`super::code`]).

use super::super::{Editor, TextField, ui};
use super::LineKind;
use super::code::{self, Code, Out, Pos};
use super::console::Fonts;
use super::highlight::{self, State, Syntax, Tok};
use super::window::{self, Colours, Kind};
use crate::gfx::{IRect, Surface};
use crate::lang::{tr, trf};
use crate::openparts::scripted::{SCRIPTED, Scripted};

/// What's being edited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// The scripted part with this id.
    Part(u32),
    /// The widget's script of this name.
    Script(String),
}

/// A text being edited, and what it's for.
#[derive(Debug, Clone)]
pub struct Editing {
    pub target: Target,
    pub syntax: Syntax,
    pub code: Code,
    /// Why the last Apply didn't take.
    pub error: Option<String>,
}

/// The buttons' bar under the text.
const BAR_H: i32 = 26;
const BUTTON_W: i32 = 70;
/// The most drafts kept for texts not being edited now.
const MAX_DRAFTS: usize = 8;

/// Where the window's parts are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// The text with its line numbers, and where the text itself starts.
    pub area: IRect,
    pub text_x: i32,
    pub apply: IRect,
    pub close: IRect,
    pub status: (i32, i32),
    pub cell: i32,
    pub line_h: i32,
    pub rows: usize,
    pub cols: usize,
}

/// The window at `r` laid out for a text of `lines` lines.
#[must_use]
pub fn layout(r: IRect, fonts: Fonts, lines: usize) -> Layout {
    let c = window::client(r);
    let cell = fonts.mono.glyph(b'M').map_or(7, |g| i32::from(g.width)).max(1);
    let line_h = i32::from(fonts.mono.height).max(1);
    let area = IRect::new(c.x0 + 2, c.y0 + 2, c.x1 - 2, c.y1 - BAR_H);
    let digits = i32::try_from(lines.max(1).to_string().len().max(2)).unwrap_or(2);
    let text_x = area.x0 + digits * cell + 12;
    let by = c.y1 - BAR_H + 2;
    let close = IRect::sized(c.x1 - 2 - BUTTON_W, by, BUTTON_W, BAR_H - 4);
    let apply = IRect::sized(close.x0 - 6 - BUTTON_W, by, BUTTON_W, BAR_H - 4);
    let rows = usize::try_from((area.height() - 4) / line_h).unwrap_or(1).max(1);
    let cols = usize::try_from((area.x1 - text_x - 4) / cell).unwrap_or(1).max(1);
    Layout {
        area,
        text_x,
        apply,
        close,
        status: (c.x0 + 4, by + 4),
        cell,
        line_h,
        rows,
        cols,
    }
}

impl Layout {
    /// The text's place under `p`.
    fn pos_at(&self, code: &Code, p: (i32, i32)) -> Pos {
        let row = usize::try_from((p.1 - self.area.y0 - 2).max(0) / self.line_h).unwrap_or(0) + code.top;
        let row = row.min(code.lines.len() - 1);
        let x = usize::try_from(((p.0 - self.text_x) * 2 + self.cell) / (2 * self.cell)).unwrap_or(0) + code.left;
        Pos::new(row, code::col_at(&code.lines[row], x))
    }

    /// Where screen column `x` of the text is drawn.
    fn x_of(&self, code: &Code, x: usize) -> i32 {
        let x = i32::try_from(x).unwrap_or(i32::MAX / 2) - i32::try_from(code.left).unwrap_or(0);
        self.text_x + x * self.cell
    }
}

/// `seg` with its tabs as spaces, it starting at screen column `x`.
fn expand(seg: &str, mut x: usize) -> String {
    let mut out = String::with_capacity(seg.len());
    for c in seg.chars() {
        if c == '\t' {
            let n = (x / code::TAB + 1) * code::TAB - x;
            out.extend(std::iter::repeat_n(' ', n));
            x += n;
        } else {
            out.push(c);
            x += 1;
        }
    }
    out
}

fn plain(s: &mut Surface, font: &formats::fnt::Font, x: i32, y: i32, text: &str, c: u8) -> i32 {
    s.text(font, x, y, &crate::text::encode(text), c)
}

impl Editing {
    /// The window's title: what's edited, `*` while changed.
    #[must_use]
    pub fn title(&self, name: &str) -> String {
        let t = trf("Script: {}", &[name]);
        if self.code.changed() { format!("{t} *") } else { t }
    }

    /// Draws the window's inside at `r`.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        s: &mut Surface,
        fonts: Fonts,
        font: &formats::fnt::Font,
        r: IRect,
        focused: bool,
        col: Colours,
    ) {
        let code = &self.code;
        let l = layout(r, fonts, code.lines.len());
        s.fill(window::client(r), ui::FACE);
        s.fill(l.area, ui::WHITE);
        ui::sunken(s, l.area);
        let gutter = IRect::new(l.area.x0 + 2, l.area.y0 + 2, l.text_x - 4, l.area.y1 - 2);
        s.fill(gutter, col.light_grey);
        // What's open (a comment, a template) where the view starts.
        let mut state = State::Code;
        for line in &code.lines[..code.top.min(code.lines.len())] {
            let _ = highlight::spans(line, self.syntax, &mut state);
        }
        let sel = code.selection();
        let inner = IRect::new(l.text_x - 1, l.area.y0 + 2, l.area.x1 - 2, l.area.y1 - 2);
        for (i, line) in code.lines.iter().enumerate().skip(code.top).take(l.rows) {
            let y = l.area.y0 + 2 + i32::try_from(i - code.top).unwrap_or(0) * l.line_h;
            // The number, right to the gutter's edge.
            let n = (i + 1).to_string();
            let nw = i32::try_from(n.len()).unwrap_or(0) * l.cell;
            plain(s, fonts.mono, gutter.x1 - 2 - nw, y, &n, col.grey);
            let old = s.set_clip(inner.intersect(s.clip()));
            // The selection behind the text.
            if let Some((a, b)) = sel
                && a.row <= i
                && i <= b.row
            {
                let from = if i == a.row { code::screen_col(line, a.col) } else { 0 };
                let to = if i == b.row {
                    code::screen_col(line, b.col)
                } else {
                    code::screen_col(line, line.len()) + 1
                };
                let (x0, x1) = (l.x_of(code, from), l.x_of(code, to));
                s.fill(IRect::new(x0, y, x1.max(x0 + 1), y + l.line_h), col.selection);
            }
            let spans = highlight::spans(line, self.syntax, &mut state);
            for (range, tok) in highlight::pieces(&spans, 0..line.len()) {
                let x = code::screen_col(line, range.start);
                let (f, c) = match tok {
                    None => (fonts.mono, ui::BLACK),
                    Some(Tok::Keyword) => (fonts.bold, col.blue),
                    Some(Tok::Prompt) => (fonts.mono, col.blue),
                    Some(Tok::Literal | Tok::Part) => (fonts.mono, col.purple),
                    Some(Tok::Number) => (fonts.mono, col.teal),
                    Some(Tok::Str) => (fonts.mono, col.maroon),
                    Some(Tok::Comment) => (fonts.mono, col.grey),
                };
                plain(s, f, l.x_of(code, x), y, &expand(&line[range], x), c);
            }
            // The caret.
            if focused && i == code.caret.row {
                let x = l.x_of(code, code::screen_col(line, code.caret.col));
                if code.block_caret() {
                    s.invert(IRect::sized(x, y, l.cell, l.line_h));
                } else {
                    s.fill(IRect::sized(x, y, 1, l.line_h), ui::BLACK);
                }
            }
            s.set_clip(old);
        }
        // The bar: what's wrong or where the caret is, Apply, Close.
        let (status, c) = match &self.error {
            // Its first line; the console has it all.
            Some(e) => (e.lines().next().unwrap_or_default().to_owned(), col.red),
            None => (code.status(), ui::BLACK),
        };
        let old = s.set_clip(IRect::new(l.status.0, l.status.1, l.apply.x0 - 4, l.status.1 + 20).intersect(s.clip()));
        plain(s, fonts.mono, l.status.0, l.status.1, &status, c);
        s.set_clip(old);
        ui::button(s, font, l.apply, tr("&Apply"), false, true, code.changed());
        ui::button(s, font, l.close, tr("Close"), false, false, true);
    }
}

impl Editor {
    /// Opens the editor on scripted part `g`'s script, with the tools on;
    /// whether it was one.
    pub(in crate::editor) fn edit_part(&mut self, g: usize) -> bool {
        let Some(gadget) = self.doc.gadgets.get(g) else {
            return false;
        };
        if !self.debugging() || gadget.core.tag != SCRIPTED {
            return false;
        }
        let Some(s) = gadget.behavior.as_any().downcast_ref::<Scripted>() else {
            return false;
        };
        let text = s.code().to_owned();
        self.open_editing(Target::Part(gadget.core.id), &text, Syntax::Js);
        true
    }

    /// Opens the editor on the scripted part with id `id`.
    pub(super) fn edit_part_id(&mut self, id: u32) {
        if let Some(g) = self.doc.gadgets.iter().position(|g| g.core.id == id) {
            self.edit_part(g);
        }
    }

    /// Opens the editor on the widget's script `i`.
    pub(super) fn edit_script(&mut self, i: usize) {
        let Some(s) = self.doc.scripts.get(i) else { return };
        let (name, text) = (s.name.clone(), s.source.clone());
        let syntax = super::scripts::syntax_of(&s.language).unwrap_or(Syntax::Js);
        self.open_editing(Target::Script(name), &text, syntax);
    }

    /// The editor on `target`'s `text`: the draft left of it, if there's
    /// one of that text, else the text.
    fn open_editing(&mut self, target: Target, text: &str, syntax: Syntax) {
        let d = &mut self.debug;
        let same = d.editing.as_ref().is_some_and(|e| e.target == target);
        if !same {
            if let Some(e) = d.editing.take()
                && e.code.changed()
            {
                d.drafts.retain(|x| x.target != e.target);
                d.drafts.push(e);
                if d.drafts.len() > MAX_DRAFTS {
                    d.drafts.remove(0);
                }
            }
            let draft = d
                .drafts
                .iter()
                .position(|x| x.target == target)
                .map(|i| d.drafts.remove(i));
            let vim = d.vim_keys;
            d.editing = Some(match draft {
                Some(mut e) if e.code.saved() == text => {
                    e.code.set_vim(vim);
                    e
                }
                _ => Editing {
                    target,
                    syntax,
                    code: Code::new(text, vim),
                    error: None,
                },
            });
        }
        self.open_window(Kind::Editor);
    }

    /// The name of what's edited, for the title.
    fn editing_name(&self) -> String {
        match self.debug.editing.as_ref().map(|e| &e.target) {
            Some(Target::Part(id)) => {
                let name = self
                    .doc
                    .gadgets
                    .iter()
                    .find(|g| g.core.id == *id)
                    .and_then(|g| g.behavior.as_any().downcast_ref::<Scripted>())
                    .map(|s| s.def().name.clone())
                    .unwrap_or_default();
                format!("{name} #{id}")
            }
            Some(Target::Script(n)) => n.clone(),
            None => String::new(),
        }
    }

    /// The window's title.
    pub(super) fn editor_title(&self) -> String {
        self.debug
            .editing
            .as_ref()
            .map_or_else(String::new, |e| e.title(&self.editing_name()))
    }

    /// Whether the editor takes the keys now.
    pub(super) fn editor_has_keys(&self) -> bool {
        self.debugging()
            && self.debug.focus == Some(Kind::Editor)
            && self.debug.window(Kind::Editor).is_some()
            && self.debug.editing.is_some()
            && !self.debug_blocked()
            && !self.menu.is_open()
    }

    /// A key in the editor.
    pub(super) fn editor_key(&mut self, vk: u16) -> bool {
        let (shift, ctrl) = (self.shift, self.ctrl && !self.alt);
        let Some(e) = &mut self.debug.editing else { return false };
        let out = e.code.key(vk, shift, ctrl);
        self.editor_out(out);
        true
    }

    /// Text typed in the editor.
    pub(super) fn editor_text(&mut self, t: &str) {
        let Some(e) = &mut self.debug.editing else { return };
        let out = e.code.type_text(t);
        self.editor_out(out);
    }

    /// The clipboard's text, pasted in the editor.
    pub(super) fn editor_paste(&mut self, t: &str) {
        if let Some(e) = &mut self.debug.editing {
            e.code.paste(t);
        }
    }

    fn editor_out(&mut self, out: Out) {
        match out {
            Out::None => {}
            Out::Apply => {
                self.editor_apply();
            }
            Out::ApplyClose => {
                if self.editor_apply() {
                    self.close_window(Kind::Editor);
                }
            }
            Out::Close => self.close_window(Kind::Editor),
            Out::Discard => {
                if let Some(e) = &mut self.debug.editing {
                    e.code.revert();
                    e.error = None;
                }
                self.close_window(Kind::Editor);
            }
            Out::Copy(t) => {
                if let Some(l) = &mut self.links {
                    let _ = (l.copy)(&t);
                }
            }
            Out::Paste => {
                if let Some(l) = &mut self.links {
                    self.paste_to_field = true;
                    (l.paste)();
                }
            }
        }
    }

    /// The text given to what it's for; whether it took.
    pub(super) fn editor_apply(&mut self) -> bool {
        let Some(e) = &self.debug.editing else { return false };
        let (target, text) = (e.target.clone(), e.code.text());
        let result = match &target {
            Target::Part(id) => self.set_part_code(*id, &text),
            Target::Script(name) => match self.doc.scripts.iter_mut().find(|s| &s.name == name) {
                Some(s) => {
                    if s.source != text {
                        s.source.clone_from(&text);
                        self.set_dirty();
                    }
                    Ok(())
                }
                None => Err(trf("{} is no longer in the widget.", &[name])),
            },
        };
        let Some(e) = &mut self.debug.editing else { return false };
        match result {
            Ok(()) => {
                e.code.mark_saved();
                e.error = None;
                let what = match &target {
                    Target::Part(id) => format!("#{id}"),
                    Target::Script(n) => n.clone(),
                };
                let text = trf("{}: the script is applied.", &[&what]);
                self.debug.console.add(LineKind::Info, &text);
                true
            }
            Err(msg) => {
                e.error = Some(msg.clone());
                self.debug.console.add(LineKind::Error, &msg);
                false
            }
        }
    }

    /// A click in the editor at `p`; whether a drag follows (a selection).
    pub(super) fn editor_click(&mut self, r: IRect, p: (i32, i32)) -> bool {
        let Some(fonts) = Fonts::of(&self.assets) else {
            return false;
        };
        let Some(e) = &self.debug.editing else { return false };
        let l = layout(r, fonts, e.code.lines.len());
        if l.apply.contains(p.0, p.1) {
            if e.code.changed() {
                self.editor_apply();
            }
            return false;
        }
        if l.close.contains(p.0, p.1) {
            self.close_window(Kind::Editor);
            return false;
        }
        if !l.area.contains(p.0, p.1) {
            return false;
        }
        let shift = self.shift;
        let Some(e) = &mut self.debug.editing else { return false };
        let at = l.pos_at(&e.code, p);
        e.code.move_to(at, shift);
        true
    }

    /// The pointer dragged in the editor's text: the selection follows.
    pub(super) fn editor_drag(&mut self, p: (i32, i32)) {
        let Some(fonts) = Fonts::of(&self.assets) else { return };
        let Some(w) = self.debug.window(Kind::Editor) else {
            return;
        };
        let Some(e) = &mut self.debug.editing else { return };
        let l = layout(w.rect, fonts, e.code.lines.len());
        let at = l.pos_at(&e.code, p);
        e.code.move_to(at, true);
        e.code.follow();
    }

    /// The wheel over the editor: its text scrolls.
    pub(super) fn editor_wheel(&mut self, dy: i32) {
        if let Some(e) = &mut self.debug.editing {
            e.code.scroll(dy.signum() * 3 * dy.abs().max(1));
        }
    }

    /// Before drawing: the editor's view the window's size.
    pub(super) fn editor_prepare(&mut self) {
        let Some(fonts) = Fonts::of(&self.assets) else { return };
        let Some(w) = self.debug.window(Kind::Editor) else {
            return;
        };
        let vim = self.debug.vim_keys;
        let Some(e) = &mut self.debug.editing else { return };
        let l = layout(w.rect, fonts, e.code.lines.len());
        e.code.page = l.rows;
        e.code.cols = l.cols;
        e.code.set_vim(vim);
    }

    /// The editor's text, for a touch screen's own text box.
    pub(super) fn editor_field(&self) -> Option<TextField> {
        let e = self.debug.editing.as_ref()?;
        Some(TextField {
            text: e.code.text(),
            multiline: true,
            max: code::MAX_BYTES,
        })
    }

    /// The editor's text as edited elsewhere.
    pub(super) fn set_editor_text(&mut self, text: &str) {
        if let Some(e) = &mut self.debug.editing {
            e.code.select_all();
            e.code.paste(text);
        }
    }

    /// Vim keys in the script editor on or off (Settings).
    pub fn set_vim_keys(&mut self, on: bool) {
        self.debug.vim_keys = on;
        if let Some(e) = &mut self.debug.editing {
            e.code.set_vim(on);
        }
    }

    #[must_use]
    pub fn vim_keys(&self) -> bool {
        self.debug.vim_keys
    }

    /// Vim keys switched in Settings: on now, and kept by the host.
    pub(in crate::editor) fn vim_keys_changed(&mut self, on: bool) {
        self.set_vim_keys(on);
        self.debug.new_vim_keys = Some(on);
    }

    /// The vim keys' switch as the player set it in Settings, once, for
    /// the host to keep.
    pub fn take_vim_keys(&mut self) -> Option<bool> {
        self.debug.new_vim_keys.take()
    }
}
