//! The script editor's text ([`super::edit`]): its lines, a caret and a
//! selection, Undo, and the keys of a plain editor (the arrows, Shift to
//! select, Ctrl+Z, Ctrl+S to apply) or, with vim keys on (Settings), a
//! part of vim's: normal, insert and visual modes, counts, the operators
//! `d`, `c`, `y`, `>` and `<` with motions, `p`, `u`, `/` and `n`, and
//! `:w`, `:q`, `:wq`.
//!
//! Positions are a row and a byte in it. A tab shows as spaces to the next
//! multiple of [`TAB`]; indenting puts in [`INDENT`] spaces.

/// A tab's width on screen.
pub const TAB: usize = 4;
/// What Tab, `>` and an opened block indent by.
const INDENT: &str = "  ";
/// The most the text takes, in bytes.
pub const MAX_BYTES: usize = 256 * 1024;
/// How many edits Undo keeps.
const MAX_UNDO: usize = 200;

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

/// A place in the text: a row and a byte in its line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub row: usize,
    pub col: usize,
}

impl Pos {
    #[must_use]
    pub const fn new(row: usize, col: usize) -> Pos {
        Pos { row, col }
    }
}

/// What a key or a text asks of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    None,
    /// The text to the part or the script (Ctrl+S, `:w`).
    Apply,
    /// Applied, then the window closed (`:wq`).
    ApplyClose,
    /// The window closed, the changes kept for the next time (Esc).
    Close,
    /// The window closed, the changes not applied dropped (`:q!`).
    Discard,
    /// Text for the clipboard.
    Copy(String),
    /// The clipboard's text wanted (Ctrl+V): it comes to [`Code::paste`].
    Paste,
}

/// Vim's modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
    VisualLine,
}

/// The vim keys' state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vim {
    pub mode: Mode,
    /// The keys of a command typed so far (`2d`).
    pending: String,
    /// The line after `:` or `/`, while one is typed.
    pub command: Option<String>,
    /// The last search.
    search: Option<String>,
    /// What `d`, `c`, `y` took last, and whether it was whole lines.
    register: (String, bool),
}

impl Vim {
    fn new() -> Vim {
        Vim {
            mode: Mode::Normal,
            pending: String::new(),
            command: None,
            search: None,
            register: (String::new(), false),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Snap {
    lines: Vec<String>,
    caret: Pos,
}

/// The text being edited.
#[derive(Debug, Clone)]
pub struct Code {
    pub lines: Vec<String>,
    pub caret: Pos,
    /// Where a selection began; the caret is its other end.
    pub anchor: Option<Pos>,
    /// The first row and (on screen) column shown.
    pub top: usize,
    pub left: usize,
    /// How many rows and columns show (the window says).
    pub page: usize,
    pub cols: usize,
    /// The column Up and Down keep to (on screen).
    want: Option<usize>,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
    /// The last edit was typing, which the next typing joins (one Undo).
    typing: bool,
    pub vim: Option<Vim>,
    /// A word for the status line (a vim error, a search not found).
    pub message: Option<String>,
    /// The text as last applied.
    saved: String,
}

/// What a character is, for words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    Space,
    Word,
    Punct,
}

fn class(c: char) -> Class {
    if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || c == '_' || c == '$' {
        Class::Word
    } else {
        Class::Punct
    }
}

/// Byte `col` of `line` on screen: tabs to the next multiple of [`TAB`].
#[must_use]
pub fn screen_col(line: &str, col: usize) -> usize {
    let mut x = 0;
    for (i, c) in line.char_indices() {
        if i >= col {
            break;
        }
        x = if c == '\t' { (x / TAB + 1) * TAB } else { x + 1 };
    }
    x
}

/// The byte of `line` at screen column `x` (the nearest character's
/// start).
#[must_use]
pub fn col_at(line: &str, x: usize) -> usize {
    let mut at = 0;
    for (i, c) in line.char_indices() {
        let next = if c == '\t' { (at / TAB + 1) * TAB } else { at + 1 };
        if next > x {
            // Past its middle: after it.
            return if x * 2 >= at + next { i + c.len_utf8() } else { i };
        }
        at = next;
    }
    line.len()
}

/// The leading spaces and tabs of `line`.
fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start_matches([' ', '\t']).len()]
}

impl Code {
    /// `text` to edit, with vim keys or not.
    #[must_use]
    pub fn new(text: &str, vim: bool) -> Code {
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        Code {
            lines: text.split('\n').map(str::to_owned).collect(),
            caret: Pos::default(),
            anchor: None,
            top: 0,
            left: 0,
            page: 20,
            cols: 60,
            want: None,
            undo: Vec::new(),
            redo: Vec::new(),
            typing: false,
            vim: vim.then(Vim::new),
            message: None,
            saved: text,
        }
    }

    /// The text, lines apart by `\n`.
    #[must_use]
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Whether it's changed since it was last applied.
    #[must_use]
    pub fn changed(&self) -> bool {
        self.text() != self.saved
    }

    /// The text as last applied.
    #[must_use]
    pub fn saved(&self) -> &str {
        &self.saved
    }

    /// The text as it is now is applied.
    pub fn mark_saved(&mut self) {
        self.saved = self.text();
    }

    /// Back to the text as last applied.
    pub fn revert(&mut self) {
        let vim = self.vim.is_some();
        let saved = self.saved.clone();
        *self = Code::new(&saved, vim);
    }

    /// Vim keys on or off (Settings).
    pub fn set_vim(&mut self, on: bool) {
        if on != self.vim.is_some() {
            self.vim = on.then(Vim::new);
            self.anchor = None;
        }
    }

    /// The vim mode, with vim keys on.
    #[must_use]
    pub fn mode(&self) -> Option<Mode> {
        self.vim.as_ref().map(|v| v.mode)
    }

    fn line(&self) -> &str {
        &self.lines[self.caret.row]
    }

    fn len(&self, row: usize) -> usize {
        self.lines.get(row).map_or(0, String::len)
    }

    /// The text's end.
    fn end(&self) -> Pos {
        let row = self.lines.len() - 1;
        Pos::new(row, self.len(row))
    }

    /// `p` inside the text, on a character's start.
    fn clamp(&self, p: Pos) -> Pos {
        let row = p.row.min(self.lines.len() - 1);
        let line = &self.lines[row];
        let mut col = p.col.min(line.len());
        while !line.is_char_boundary(col) {
            col -= 1;
        }
        Pos::new(row, col)
    }

    /// In vim's normal mode the caret is on a character, not past the
    /// last.
    fn settle(&mut self) {
        if matches!(self.mode(), Some(Mode::Normal | Mode::Visual | Mode::VisualLine)) {
            let line = self.line();
            if !line.is_empty() && self.caret.col >= line.len() {
                let last = line.char_indices().last().map_or(0, |(i, _)| i);
                self.caret.col = last;
            }
        }
    }

    /// The character after `p` (at the line's end: the next line's
    /// start).
    fn next(&self, p: Pos) -> Pos {
        let line = &self.lines[p.row];
        match line[p.col..].chars().next() {
            Some(c) => Pos::new(p.row, p.col + c.len_utf8()),
            None if p.row + 1 < self.lines.len() => Pos::new(p.row + 1, 0),
            None => p,
        }
    }

    /// The character before `p`.
    fn prev(&self, p: Pos) -> Pos {
        if p.col > 0 {
            let line = &self.lines[p.row];
            let c = line[..p.col].chars().next_back().map_or(1, char::len_utf8);
            Pos::new(p.row, p.col - c)
        } else if p.row > 0 {
            Pos::new(p.row - 1, self.len(p.row - 1))
        } else {
            p
        }
    }

    /// The character at `p`, a line's end being a space.
    fn char_at(&self, p: Pos) -> Option<char> {
        let line = &self.lines[p.row];
        match line[p.col..].chars().next() {
            Some(c) => Some(c),
            None if p.row + 1 < self.lines.len() => Some('\n'),
            None => None,
        }
    }

    /// The selection, first end first, if it isn't empty.
    #[must_use]
    pub fn selection(&self) -> Option<(Pos, Pos)> {
        let a = self.anchor?;
        let (s, e) = if a <= self.caret {
            (a, self.caret)
        } else {
            (self.caret, a)
        };
        match self.mode() {
            // Vim's visual modes take the caret's character, or whole
            // lines.
            Some(Mode::VisualLine) => {
                let end = if e.row + 1 < self.lines.len() {
                    Pos::new(e.row + 1, 0)
                } else {
                    Pos::new(e.row, self.len(e.row))
                };
                let start = if e.row + 1 >= self.lines.len() && s.row > 0 {
                    Pos::new(s.row - 1, self.len(s.row - 1))
                } else {
                    Pos::new(s.row, 0)
                };
                Some((start, end))
            }
            Some(Mode::Visual) => Some((s, self.next(e))),
            _ => (s != e).then_some((s, e)),
        }
    }

    /// The text from `a` to `b`.
    #[must_use]
    pub fn slice(&self, a: Pos, b: Pos) -> String {
        if a.row == b.row {
            return self.lines[a.row][a.col..b.col].to_owned();
        }
        let mut s = self.lines[a.row][a.col..].to_owned();
        for row in a.row + 1..b.row {
            s.push('\n');
            s.push_str(&self.lines[row]);
        }
        s.push('\n');
        s.push_str(&self.lines[b.row][..b.col]);
        s
    }

    /// The selected text.
    #[must_use]
    pub fn selected(&self) -> Option<String> {
        self.selection().map(|(a, b)| self.slice(a, b))
    }

    // ---- edits --------------------------------------------------------

    /// Before an edit: Undo's step (typing joins the last typing's).
    fn begin(&mut self, typing: bool) {
        if !(typing && self.typing) {
            self.undo.push(Snap {
                lines: self.lines.clone(),
                caret: self.caret,
            });
            if self.undo.len() > MAX_UNDO {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.typing = typing;
    }

    fn undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            self.redo.push(Snap {
                lines: std::mem::replace(&mut self.lines, s.lines),
                caret: self.caret,
            });
            self.caret = self.clamp(s.caret);
            self.anchor = None;
        } else {
            self.message = Some("Already at the oldest change".into());
        }
        self.typing = false;
    }

    fn redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(Snap {
                lines: std::mem::replace(&mut self.lines, s.lines),
                caret: self.caret,
            });
            self.caret = self.clamp(s.caret);
            self.anchor = None;
        }
        self.typing = false;
    }

    /// Takes out the text from `a` to `b`; the caret goes to `a`.
    fn remove(&mut self, a: Pos, b: Pos) -> String {
        let cut = self.slice(a, b);
        let tail = self.lines[b.row][b.col..].to_owned();
        self.lines[a.row].truncate(a.col);
        self.lines[a.row].push_str(&tail);
        self.lines.drain(a.row + 1..=b.row);
        self.caret = a;
        self.anchor = None;
        cut
    }

    /// Puts `t` in at the caret (over the selection), the caret after it.
    fn put(&mut self, t: &str) {
        if let Some((a, b)) = self.selection() {
            self.remove(a, b);
        }
        let room = MAX_BYTES.saturating_sub(self.lines.iter().map(|l| l.len() + 1).sum());
        let mut t = t;
        if t.len() > room {
            let mut n = room;
            while !t.is_char_boundary(n) {
                n -= 1;
            }
            t = &t[..n];
        }
        let p = self.caret;
        let tail = self.lines[p.row][p.col..].to_owned();
        self.lines[p.row].truncate(p.col);
        let mut parts = t.split('\n');
        if let Some(first) = parts.next() {
            self.lines[p.row].push_str(first);
        }
        let mut row = p.row;
        for part in parts {
            row += 1;
            self.lines.insert(row, part.to_owned());
        }
        let col = self.lines[row].len();
        self.lines[row].push_str(&tail);
        self.caret = Pos::new(row, col);
        self.anchor = None;
    }

    /// A line break, the line's indent kept (and one more after an
    /// opening bracket).
    fn new_line(&mut self) {
        self.begin(false);
        let line = self.line();
        let before = &line[..self.caret.col];
        let mut indent = indent_of(before).to_owned();
        if before.trim_end().ends_with(['{', '(', '[']) {
            indent.push_str(INDENT);
        }
        self.put(&format!("\n{indent}"));
    }

    /// Rows `a..=b` indented by one step, or one step less.
    fn indent_rows(&mut self, a: usize, b: usize, more: bool) {
        for row in a..=b {
            let line = &mut self.lines[row];
            if more {
                if !line.is_empty() {
                    line.insert_str(0, INDENT);
                }
            } else {
                let n = line.len() - line.trim_start_matches(' ').len();
                let n = if line.starts_with('\t') { 1 } else { n.min(INDENT.len()) };
                line.drain(..n);
            }
        }
        self.caret = self.clamp(self.caret);
        if let Some(a) = self.anchor {
            self.anchor = Some(self.clamp(a));
        }
    }

    /// Text typed (or pasted): in at the caret.
    pub fn type_text(&mut self, t: &str) -> Out {
        self.message = None;
        if let Some(v) = &self.vim
            && v.mode != Mode::Insert
        {
            for (i, c) in t.char_indices() {
                let out = self.vim_char(c);
                if out != Out::None {
                    self.follow();
                    return out;
                }
                // `i`, `a`, `o`...: the rest is typed in.
                if self.mode() == Some(Mode::Insert) {
                    return self.type_text(&t[i + c.len_utf8()..]);
                }
            }
            self.follow();
            return Out::None;
        }
        let t: String = t
            .chars()
            .filter(|&c| !c.is_control() || c == '\n' || c == '\t')
            .collect();
        if t.is_empty() {
            return Out::None;
        }
        // A closing bracket alone on its line goes back a step.
        let line = self.line();
        if t == "}" && line[..self.caret.col].trim().is_empty() && self.caret.col >= INDENT.len() {
            self.begin(true);
            let row = self.caret.row;
            self.indent_rows(row, row, false);
        }
        self.begin(!t.contains('\n'));
        self.put(&t);
        self.want = None;
        self.follow();
        Out::None
    }

    /// The clipboard's text, pasted at the caret.
    pub fn paste(&mut self, t: &str) {
        let t = t.replace("\r\n", "\n").replace('\r', "\n");
        self.begin(false);
        if let Some(v) = &mut self.vim
            && v.mode != Mode::Insert
        {
            v.mode = Mode::Normal;
        }
        self.put(&t);
        self.settle();
        self.follow();
    }

    /// Everything selected.
    pub fn select_all(&mut self) {
        self.anchor = Some(Pos::default());
        self.caret = self.end();
    }

    // ---- the caret ---------------------------------------------------

    /// The caret to `p`, selecting from where it was with `extend`.
    pub fn move_to(&mut self, p: Pos, extend: bool) {
        let visual = matches!(self.mode(), Some(Mode::Visual | Mode::VisualLine));
        if extend || visual {
            self.anchor.get_or_insert(self.caret);
        } else {
            self.anchor = None;
        }
        self.caret = self.clamp(p);
        self.typing = false;
    }

    /// Up or down `n` rows, keeping to the column.
    fn rows(&self, down: bool, n: usize) -> Pos {
        let want = self.want.unwrap_or_else(|| screen_col(self.line(), self.caret.col));
        let row = if down {
            (self.caret.row + n).min(self.lines.len() - 1)
        } else {
            self.caret.row.saturating_sub(n)
        };
        Pos::new(row, col_at(&self.lines[row], want))
    }

    fn vertical(&mut self, down: bool, n: usize, extend: bool) {
        let want = self.want.unwrap_or_else(|| screen_col(self.line(), self.caret.col));
        let p = self.rows(down, n);
        self.move_to(p, extend);
        self.want = Some(want);
        self.settle();
    }

    /// The first character that isn't a space on row `row`.
    fn first_char(&self, row: usize) -> Pos {
        Pos::new(row, indent_of(&self.lines[row]).len())
    }

    /// The next word's start (vim's `w`; an empty line is a word).
    fn word_start(&self, p: Pos) -> Pos {
        let mut q = p;
        if let Some(c) = self.char_at(q) {
            let k = class(c);
            if k != Class::Space {
                while self.char_at(q).is_some_and(|c| class(c) == k && c != '\n') {
                    q = self.next(q);
                }
            }
        }
        while let Some(c) = self.char_at(q) {
            if c == '\n' {
                q = self.next(q);
                if self.lines[q.row].is_empty() {
                    return q;
                }
                continue;
            }
            if class(c) != Class::Space {
                break;
            }
            q = self.next(q);
        }
        q
    }

    /// The word's start before `p` (vim's `b`).
    fn word_back(&self, p: Pos) -> Pos {
        let mut q = self.prev(p);
        while q > Pos::default() && self.char_at(q).is_some_and(|c| class(c) == Class::Space) {
            if q.col == 0 && self.lines[q.row].is_empty() {
                return q;
            }
            q = self.prev(q);
        }
        let Some(k) = self.char_at(q).map(class) else { return q };
        while q.col > 0 {
            let b = self.prev(q);
            if self.char_at(b).map(class) != Some(k) {
                break;
            }
            q = b;
        }
        q
    }

    /// The word's end after `p` (vim's `e`: its last character).
    fn word_end(&self, p: Pos) -> Pos {
        let mut q = self.next(p);
        while self.char_at(q).is_some_and(|c| class(c) == Class::Space) && q < self.end() {
            q = self.next(q);
        }
        let Some(k) = self.char_at(q).map(class) else { return q };
        loop {
            let n = self.next(q);
            if n == q || n.row != q.row || self.char_at(n).map(class) != Some(k) {
                return q;
            }
            q = n;
        }
    }

    /// The caret's row and column in the window's view.
    pub fn follow(&mut self) {
        let page = self.page.max(1);
        if self.caret.row < self.top {
            self.top = self.caret.row;
        } else if self.caret.row >= self.top + page {
            self.top = self.caret.row + 1 - page;
        }
        let x = screen_col(self.line(), self.caret.col);
        let cols = self.cols.max(4);
        if x < self.left {
            self.left = x.saturating_sub(4);
        } else if x >= self.left + cols {
            self.left = x + 4 - cols;
        }
    }

    /// The view scrolled by `n` rows (the wheel).
    pub fn scroll(&mut self, n: i32) {
        let most = self.lines.len().saturating_sub(1);
        let by = usize::try_from(n.unsigned_abs()).unwrap_or(0);
        self.top = if n < 0 {
            self.top.saturating_sub(by)
        } else {
            (self.top + by).min(most)
        };
    }

    // ---- keys ---------------------------------------------------------

    /// A key, with Shift and Ctrl as they're held.
    pub fn key(&mut self, vk: u16, shift: bool, ctrl: bool) -> Out {
        self.message = None;
        let out = match self.mode() {
            Some(Mode::Insert) | None => self.plain_key(vk, shift, ctrl),
            Some(_) => self.vim_key(vk, ctrl),
        };
        self.follow();
        out
    }

    /// A key of the plain editor (and vim's insert mode).
    #[allow(clippy::too_many_lines)]
    fn plain_key(&mut self, vk: u16, shift: bool, ctrl: bool) -> Out {
        if ctrl {
            return match vk {
                // A, C, X, V, Z, Y, S.
                0x41 => {
                    self.select_all();
                    Out::None
                }
                0x43 => Out::Copy(self.selected().unwrap_or_else(|| format!("{}\n", self.line()))),
                0x58 => {
                    let Some((a, b)) = self.selection() else {
                        return Out::None;
                    };
                    self.begin(false);
                    Out::Copy(self.remove(a, b))
                }
                0x56 => Out::Paste,
                0x5a if shift => {
                    self.redo();
                    Out::None
                }
                0x5a => {
                    self.undo();
                    Out::None
                }
                0x59 => {
                    self.redo();
                    Out::None
                }
                0x53 => Out::Apply,
                VK_HOME => {
                    self.move_to(Pos::default(), shift);
                    Out::None
                }
                VK_END => {
                    self.move_to(self.end(), shift);
                    Out::None
                }
                VK_LEFT => {
                    self.move_to(self.word_back(self.caret), shift);
                    Out::None
                }
                VK_RIGHT => {
                    self.move_to(self.word_start(self.caret), shift);
                    Out::None
                }
                _ => Out::None,
            };
        }
        if vk != VK_UP && vk != VK_DOWN && vk != VK_PRIOR && vk != VK_NEXT {
            self.want = None;
        }
        match vk {
            VK_ESCAPE => {
                if let Some(v) = &mut self.vim {
                    v.mode = Mode::Normal;
                    self.anchor = None;
                    self.caret = self.prev_on_line(self.caret);
                    self.typing = false;
                } else if self.anchor.is_some() {
                    self.anchor = None;
                } else {
                    return Out::Close;
                }
            }
            VK_LEFT => match self.selection() {
                Some((a, _)) if !shift => self.move_to(a, false),
                _ => self.move_to(self.prev(self.caret), shift),
            },
            VK_RIGHT => match self.selection() {
                Some((_, b)) if !shift => self.move_to(b, false),
                _ => self.move_to(self.next(self.caret), shift),
            },
            VK_UP => self.vertical(false, 1, shift),
            VK_DOWN => self.vertical(true, 1, shift),
            VK_PRIOR => self.vertical(false, self.page.max(2) - 1, shift),
            VK_NEXT => self.vertical(true, self.page.max(2) - 1, shift),
            VK_HOME => {
                // The line's text, then its very start.
                let first = self.first_char(self.caret.row);
                let p = if self.caret == first {
                    Pos::new(self.caret.row, 0)
                } else {
                    first
                };
                self.move_to(p, shift);
            }
            VK_END => self.move_to(Pos::new(self.caret.row, self.len(self.caret.row)), shift),
            VK_BACK => {
                if let Some((a, b)) = self.selection() {
                    self.begin(false);
                    self.remove(a, b);
                } else if self.caret > Pos::default() {
                    self.begin(true);
                    let p = self.caret;
                    let before = &self.line()[..p.col];
                    // In an indent, a step back.
                    let a = if p.col > 0 && before.trim().is_empty() && before.ends_with(INDENT) {
                        Pos::new(p.row, p.col - INDENT.len())
                    } else {
                        self.prev(p)
                    };
                    self.remove(a, p);
                }
            }
            VK_DELETE => {
                if let Some((a, b)) = self.selection() {
                    self.begin(false);
                    self.remove(a, b);
                } else if self.caret < self.end() {
                    self.begin(true);
                    let (a, b) = (self.caret, self.next(self.caret));
                    self.remove(a, b);
                }
            }
            VK_RETURN => self.new_line(),
            VK_TAB => match self.selection() {
                Some((a, b)) if a.row != b.row => {
                    self.begin(false);
                    let last = if b.col == 0 { b.row - 1 } else { b.row };
                    self.indent_rows(a.row, last, !shift);
                }
                _ if shift => {
                    self.begin(false);
                    let row = self.caret.row;
                    self.indent_rows(row, row, false);
                }
                _ => {
                    self.begin(true);
                    let x = screen_col(self.line(), self.caret.col);
                    let n = INDENT.len() - x % INDENT.len();
                    self.put(&" ".repeat(n));
                }
            },
            _ => {}
        }
        Out::None
    }

    /// The character before `p` on its line (vim leaving insert mode).
    fn prev_on_line(&self, p: Pos) -> Pos {
        if p.col == 0 { p } else { self.prev(p) }
    }

    // ---- vim ----------------------------------------------------------

    fn vim(&mut self) -> &mut Vim {
        self.vim.get_or_insert_with(Vim::new)
    }

    fn set_mode(&mut self, m: Mode) {
        let caret = self.caret;
        let v = self.vim();
        v.mode = m;
        v.pending.clear();
        match m {
            Mode::Visual | Mode::VisualLine => {
                self.anchor.get_or_insert(caret);
            }
            Mode::Normal => {
                self.anchor = None;
                self.settle();
            }
            Mode::Insert => self.anchor = None,
        }
        self.typing = false;
    }

    /// A key in vim's normal and visual modes (the letters come as text).
    fn vim_key(&mut self, vk: u16, ctrl: bool) -> Out {
        if self.vim.as_ref().is_some_and(|v| v.command.is_some()) {
            return self.command_key(vk);
        }
        if ctrl {
            match vk {
                0x52 => self.redo(),
                0x53 => return Out::Apply,
                0x44 => self.vertical(true, self.page / 2, false),
                0x55 => self.vertical(false, self.page / 2, false),
                0x43 | 0x5b => self.set_mode(Mode::Normal),
                0x56 => return Out::Paste,
                _ => {}
            }
            return Out::None;
        }
        let visual = matches!(self.mode(), Some(Mode::Visual | Mode::VisualLine));
        match vk {
            VK_ESCAPE => {
                self.vim().pending.clear();
                if visual {
                    self.set_mode(Mode::Normal);
                }
            }
            VK_LEFT | VK_BACK => self.vim_motion('h'),
            VK_RIGHT => self.vim_motion('l'),
            VK_UP => self.vim_motion('k'),
            VK_DOWN => self.vim_motion('j'),
            VK_HOME => self.vim_motion('0'),
            VK_END => self.vim_motion('$'),
            VK_PRIOR => self.vertical(false, self.page.max(2) - 1, visual),
            VK_NEXT => self.vertical(true, self.page.max(2) - 1, visual),
            VK_RETURN => {
                let row = (self.caret.row + 1).min(self.lines.len() - 1);
                self.move_to(self.first_char(row), visual);
            }
            VK_DELETE => return self.vim_char('x'),
            _ => {}
        }
        Out::None
    }

    /// A motion key alone, as typed.
    fn vim_motion(&mut self, c: char) {
        self.vim().pending.clear();
        let _ = self.vim_char(c);
    }

    /// A key of the line after `:` or `/`.
    fn command_key(&mut self, vk: u16) -> Out {
        let Some(v) = &mut self.vim else { return Out::None };
        let Some(line) = &mut v.command else { return Out::None };
        match vk {
            VK_ESCAPE => v.command = None,
            VK_BACK => {
                if line.len() <= 1 {
                    v.command = None;
                } else {
                    line.pop();
                }
            }
            VK_RETURN => {
                let line = v.command.take().unwrap_or_default();
                return self.run_command(&line);
            }
            _ => {}
        }
        Out::None
    }

    /// The line after `:` or `/` entered.
    fn run_command(&mut self, line: &str) -> Out {
        if let Some(pat) = line.strip_prefix('/') {
            if !pat.is_empty() {
                self.vim().search = Some(pat.to_owned());
            }
            self.search(true);
            return Out::None;
        }
        let cmd = line.trim_start_matches(':').trim();
        match cmd {
            "w" => Out::Apply,
            "wq" | "x" => Out::ApplyClose,
            "q" if self.changed() => {
                self.message = Some("E37: No write since last change (add ! to override)".into());
                Out::None
            }
            "q" => Out::Close,
            "q!" => Out::Discard,
            "" => Out::None,
            _ => {
                if let Ok(n) = cmd.parse::<usize>() {
                    let row = n.max(1).min(self.lines.len()) - 1;
                    self.move_to(self.first_char(row), false);
                } else {
                    self.message = Some(format!("E492: Not an editor command: {cmd}"));
                }
                Out::None
            }
        }
    }

    /// The last search's next match after the caret (or before it).
    fn search(&mut self, forward: bool) {
        let Some(pat) = self.vim.as_ref().and_then(|v| v.search.clone()) else {
            return;
        };
        let text = self.text();
        // The caret as an offset into the text.
        let at: usize = self.lines[..self.caret.row].iter().map(|l| l.len() + 1).sum::<usize>() + self.caret.col;
        let found = if forward {
            let from = (at + 1).min(text.len());
            text[from..].find(&pat).map(|i| i + from).or_else(|| text.find(&pat))
        } else {
            text[..at].rfind(&pat).or_else(|| text.rfind(&pat))
        };
        let Some(off) = found else {
            self.message = Some(format!("E486: Pattern not found: {pat}"));
            return;
        };
        let mut rest = off;
        for (row, l) in self.lines.iter().enumerate() {
            if rest <= l.len() {
                let p = Pos::new(row, rest);
                let visual = matches!(self.mode(), Some(Mode::Visual | Mode::VisualLine));
                self.move_to(p, visual);
                return;
            }
            rest -= l.len() + 1;
        }
    }

    /// A character typed in vim's normal or visual mode.
    #[allow(clippy::too_many_lines)]
    fn vim_char(&mut self, c: char) -> Out {
        if let Some(line) = self.vim.as_mut().and_then(|v| v.command.as_mut()) {
            line.push(c);
            return Out::None;
        }
        let mode = self.mode().unwrap_or(Mode::Normal);
        let visual = matches!(mode, Mode::Visual | Mode::VisualLine);
        let v = self.vim();
        v.pending.push(c);
        let keys = v.pending.clone();
        // The count, then an operator, then a motion or the command.
        let digits = keys.chars().take_while(|d| d.is_ascii_digit() && *d != '0').count();
        let digits = if digits > 0 {
            digits + keys[digits..].chars().take_while(char::is_ascii_digit).count()
        } else {
            0
        };
        let count: Option<usize> = keys[..digits].parse().ok();
        let rest = &keys[digits..];
        if rest.is_empty() {
            return Out::None;
        }
        let n = count.unwrap_or(1);
        // `r` and a character: the characters under the caret replaced.
        if let Some(r) = rest.strip_prefix('r') {
            let Some(with) = r.chars().next() else { return Out::None };
            self.vim().pending.clear();
            let line = self.line();
            let from = self.caret.col;
            let chars: Vec<(usize, char)> = line[from..].char_indices().take(n).collect();
            if chars.len() == n {
                self.begin(false);
                let end = from + chars.last().map_or(0, |(i, c)| i + c.len_utf8());
                let s = with.to_string().repeat(n);
                self.lines[self.caret.row].replace_range(from..end, &s);
                self.caret.col = from + s.len() - with.len_utf8();
            }
            return Out::None;
        }
        let (op, motion) = match rest.chars().next() {
            Some(o @ ('d' | 'c' | 'y' | '>' | '<')) if !visual => (Some(o), &rest[1..]),
            _ => (None, rest),
        };
        if op.is_some() && motion.is_empty() {
            return Out::None;
        }
        // An operator twice: whole lines.
        if let Some(o) = op
            && motion.starts_with(o)
        {
            self.vim().pending.clear();
            let a = self.caret.row;
            let b = (a + n - 1).min(self.lines.len() - 1);
            return self.operate_rows(o, a, b);
        }
        // A motion with its count digits after the operator (`d2w`).
        let (n, motion) = {
            let d = motion.chars().take_while(|d| d.is_ascii_digit() && *d != '0').count();
            if d > 0 {
                let d = d + motion[d..].chars().take_while(char::is_ascii_digit).count();
                let m: usize = motion[..d].parse().unwrap_or(1);
                (n * m, &motion[d..])
            } else {
                (n, motion)
            }
        };
        if motion.is_empty() || motion == "g" {
            return Out::None;
        }
        let motion = motion.to_owned();
        self.vim().pending.clear();
        if let Some((to, lines, inclusive)) = self.motion(&motion, n, count, op) {
            return match op {
                None => {
                    self.move_to(to, visual);
                    if !visual {
                        self.settle();
                    }
                    if !matches!(motion.as_str(), "j" | "k") {
                        self.want = None;
                    }
                    Out::None
                }
                Some(o) if lines => {
                    let (a, b) = if to.row < self.caret.row {
                        (to.row, self.caret.row)
                    } else {
                        (self.caret.row, to.row)
                    };
                    self.operate_rows(o, a, b)
                }
                Some(o) => {
                    let (a, mut b) = if to < self.caret {
                        (to, self.caret)
                    } else {
                        (self.caret, to)
                    };
                    if inclusive {
                        b = self.next(b).max(b);
                        if b.row != a.row && b.col == 0 && to.row == a.row {
                            b = Pos::new(a.row, self.len(a.row));
                        }
                    }
                    self.operate(o, a, b)
                }
            };
        }
        if visual {
            return self.visual_command(&motion);
        }
        self.normal_command(&motion, n)
    }

    /// Where motion `m` goes `n` times from the caret; whether it takes
    /// whole lines, and whether its end character is in it.
    #[allow(clippy::too_many_lines)]
    fn motion(&self, m: &str, n: usize, count: Option<usize>, op: Option<char>) -> Option<(Pos, bool, bool)> {
        let p = self.caret;
        let last_row = self.lines.len() - 1;
        Some(match m {
            "h" => {
                let mut q = p;
                for _ in 0..n {
                    q = self.prev_on_line(q);
                }
                (q, false, false)
            }
            "l" | " " => {
                let mut q = p;
                for _ in 0..n {
                    let next = self.next(q);
                    if next.row != q.row {
                        break;
                    }
                    // In normal mode, not past the last character.
                    if op.is_none() && next.col >= self.len(q.row) {
                        break;
                    }
                    q = next;
                }
                (q, false, false)
            }
            "j" => (self.rows(true, n), true, false),
            "k" => (self.rows(false, n), true, false),
            "w" | "W" => {
                // `cw` is `ce`: the spaces after the word stay.
                if op == Some('c') && self.char_at(p).is_some_and(|c| class(c) != Class::Space) {
                    let mut q = p;
                    for i in 0..n {
                        q = if i == 0 && self.word_end_here(q) {
                            q
                        } else {
                            self.word_end(q)
                        };
                    }
                    return Some((q, false, true));
                }
                let mut q = p;
                for _ in 0..n {
                    q = self.word_start(q);
                }
                // An operator stops at the line's end.
                if op.is_some() && q.row > p.row {
                    q = Pos::new(p.row, self.len(p.row));
                }
                (q, false, false)
            }
            "b" | "B" => {
                let mut q = p;
                for _ in 0..n {
                    q = self.word_back(q);
                }
                (q, false, false)
            }
            "e" | "E" => {
                let mut q = p;
                for _ in 0..n {
                    q = self.word_end(q);
                }
                (q, false, true)
            }
            "0" => (Pos::new(p.row, 0), false, false),
            "^" => (self.first_char(p.row), false, false),
            "$" => {
                let row = (p.row + n - 1).min(last_row);
                let len = self.len(row);
                let end = if op.is_some() || len == 0 {
                    len
                } else {
                    self.lines[row].char_indices().last().map_or(0, |(i, _)| i)
                };
                (Pos::new(row, end), false, op.is_none())
            }
            "gg" => (
                self.first_char(count.unwrap_or(1).max(1).min(last_row + 1) - 1),
                true,
                false,
            ),
            "G" => (
                self.first_char(count.map_or(last_row, |c| c.max(1).min(last_row + 1) - 1)),
                true,
                false,
            ),
            "}" => {
                let mut row = p.row;
                for _ in 0..n {
                    row += 1;
                    while row < last_row && !self.lines[row].trim().is_empty() {
                        row += 1;
                    }
                }
                (Pos::new(row.min(last_row), 0), false, false)
            }
            "{" => {
                let mut row = p.row;
                for _ in 0..n {
                    row = row.saturating_sub(1);
                    while row > 0 && !self.lines[row].trim().is_empty() {
                        row -= 1;
                    }
                }
                (Pos::new(row, 0), false, false)
            }
            _ => return None,
        })
    }

    /// Whether `p` is on a word's last character.
    fn word_end_here(&self, p: Pos) -> bool {
        let Some(k) = self.char_at(p).map(class) else {
            return false;
        };
        let n = self.next(p);
        n.row != p.row || self.char_at(n).map(class) != Some(k)
    }

    /// Operator `o` over `a..b`.
    fn operate(&mut self, o: char, a: Pos, b: Pos) -> Out {
        if a == b && o != 'c' {
            return Out::None;
        }
        let text = self.slice(a, b);
        match o {
            'y' => {
                self.vim().register = (text.clone(), false);
                self.caret = a;
                Out::Copy(text)
            }
            'd' | 'c' => {
                self.begin(false);
                self.remove(a, b);
                self.vim().register = (text, false);
                if o == 'c' {
                    self.set_mode(Mode::Insert);
                } else {
                    self.settle();
                }
                Out::None
            }
            '>' | '<' => self.operate_rows(o, a.row, b.row),
            _ => Out::None,
        }
    }

    /// Operator `o` over rows `a..=b`, whole.
    fn operate_rows(&mut self, o: char, a: usize, b: usize) -> Out {
        let text: String = self.lines[a..=b].iter().flat_map(|l| [l.as_str(), "\n"]).collect();
        match o {
            'y' => {
                self.vim().register = (text.clone(), true);
                Out::Copy(text)
            }
            'd' => {
                self.begin(false);
                self.vim().register = (text, true);
                if self.lines.len() == b - a + 1 {
                    self.lines = vec![String::new()];
                } else {
                    self.lines.drain(a..=b);
                }
                let row = a.min(self.lines.len() - 1);
                self.caret = self.first_char(row);
                self.anchor = None;
                Out::None
            }
            'c' => {
                self.begin(false);
                self.vim().register = (text, true);
                let indent = indent_of(&self.lines[a]).to_owned();
                self.lines.drain(a + 1..=b);
                self.lines[a].clone_from(&indent);
                self.caret = Pos::new(a, indent.len());
                self.set_mode(Mode::Insert);
                Out::None
            }
            '>' | '<' => {
                self.begin(false);
                self.indent_rows(a, b, o == '>');
                self.caret = self.first_char(self.caret.row);
                Out::None
            }
            _ => Out::None,
        }
    }

    /// A command of visual mode: an operator over the selection.
    fn visual_command(&mut self, k: &str) -> Out {
        let lines = self.mode() == Some(Mode::VisualLine);
        let Some(a) = self.anchor else { return Out::None };
        let (s, e) = if a <= self.caret {
            (a, self.caret)
        } else {
            (self.caret, a)
        };
        let out = match k {
            "v" => {
                let m = if lines { Mode::Visual } else { Mode::Normal };
                self.set_mode(m);
                return Out::None;
            }
            "V" => {
                let m = if lines { Mode::Normal } else { Mode::VisualLine };
                self.set_mode(m);
                return Out::None;
            }
            "o" => {
                self.anchor = Some(self.caret);
                self.caret = a;
                return Out::None;
            }
            ":" => {
                self.set_mode(Mode::Normal);
                self.vim().command = Some(":".into());
                return Out::None;
            }
            "d" | "x" | "y" | "c" | ">" | "<" | "s" => {
                let o = match k {
                    "x" => 'd',
                    "s" => 'c',
                    _ => k.chars().next().unwrap_or('d'),
                };
                self.anchor = None;
                self.vim().mode = Mode::Normal;
                if lines || o == '>' || o == '<' {
                    self.operate_rows(o, s.row, e.row)
                } else {
                    let end = self.next(e);
                    self.operate(o, s, end)
                }
            }
            _ => return Out::None,
        };
        if self.mode() == Some(Mode::Visual) || self.mode() == Some(Mode::VisualLine) {
            self.set_mode(Mode::Normal);
        }
        out
    }

    /// A command of normal mode, `n` times where that means something.
    #[allow(clippy::too_many_lines)]
    fn normal_command(&mut self, k: &str, n: usize) -> Out {
        let row = self.caret.row;
        match k {
            "i" => self.set_mode(Mode::Insert),
            "a" => {
                if !self.line().is_empty() {
                    self.caret = self.next(self.caret).min(Pos::new(row, self.len(row)));
                }
                self.set_mode(Mode::Insert);
            }
            "I" => {
                self.caret = self.first_char(row);
                self.set_mode(Mode::Insert);
            }
            "A" => {
                self.caret = Pos::new(row, self.len(row));
                self.set_mode(Mode::Insert);
            }
            "o" | "O" => {
                self.begin(false);
                let indent = indent_of(&self.lines[row]).to_owned();
                let at = if k == "o" { row + 1 } else { row };
                self.lines.insert(at, indent.clone());
                self.caret = Pos::new(at, indent.len());
                self.set_mode(Mode::Insert);
            }
            "v" => self.set_mode(Mode::Visual),
            "V" => self.set_mode(Mode::VisualLine),
            "x" | "X" => {
                let (a, b) = if k == "x" {
                    let mut b = self.caret;
                    for _ in 0..n {
                        let nb = self.next(b);
                        if nb.row != row {
                            b = Pos::new(row, self.len(row));
                            break;
                        }
                        b = nb;
                    }
                    (self.caret, b)
                } else {
                    let mut a = self.caret;
                    for _ in 0..n {
                        a = self.prev_on_line(a);
                    }
                    (a, self.caret)
                };
                if a != b {
                    let out = self.operate('d', a, b);
                    self.settle();
                    return out;
                }
            }
            "D" | "C" => {
                let end = Pos::new(row, self.len(row));
                let o = if k == "D" { 'd' } else { 'c' };
                let out = self.operate(o, self.caret, end);
                return out;
            }
            "s" => {
                let b = self.next(self.caret).min(Pos::new(row, self.len(row)));
                return self.operate('c', self.caret, b);
            }
            "S" => return self.operate_rows('c', row, row),
            "Y" => return self.operate_rows('y', row, (row + n - 1).min(self.lines.len() - 1)),
            "p" | "P" => {
                let (text, lines) = self.vim().register.clone();
                if text.is_empty() {
                    return Out::None;
                }
                self.begin(false);
                if lines {
                    let at = if k == "p" { row + 1 } else { row };
                    let mut new: Vec<String> = Vec::new();
                    for _ in 0..n {
                        new.extend(text.trim_end_matches('\n').split('\n').map(str::to_owned));
                    }
                    for (i, l) in new.into_iter().enumerate() {
                        self.lines.insert(at + i, l);
                    }
                    self.caret = self.first_char(at);
                } else {
                    if k == "p" && !self.line().is_empty() {
                        self.caret = self.next(self.caret).min(Pos::new(row, self.len(row)));
                    }
                    self.put(&text.repeat(n));
                    self.caret = self.prev(self.caret);
                }
                self.settle();
            }
            "J" => {
                let last = (row + n.max(2) - 1).min(self.lines.len() - 1);
                if last > row {
                    self.begin(false);
                    for _ in row..last {
                        let next = self.lines.remove(row + 1);
                        let line = &mut self.lines[row];
                        let keep = line.trim_end().len();
                        line.truncate(keep);
                        self.caret = Pos::new(row, line.len());
                        let next = next.trim_start();
                        if !next.is_empty() && !line.is_empty() {
                            line.push(' ');
                        }
                        line.push_str(next);
                    }
                }
            }
            "u" => {
                for _ in 0..n {
                    self.undo();
                }
                self.settle();
            }
            "n" | "N" => {
                for _ in 0..n {
                    self.search(k == "n");
                }
            }
            ":" => self.vim().command = Some(":".into()),
            "/" => self.vim().command = Some("/".into()),
            "~" => {
                let line = self.line();
                if let Some(ch) = line[self.caret.col..].chars().next() {
                    self.begin(false);
                    let swapped: String = if ch.is_uppercase() {
                        ch.to_lowercase().collect()
                    } else {
                        ch.to_uppercase().collect()
                    };
                    let from = self.caret.col;
                    self.lines[row].replace_range(from..from + ch.len_utf8(), &swapped);
                    self.caret = self.next(Pos::new(row, from)).min(Pos::new(row, self.len(row)));
                    self.settle();
                }
            }
            // A start of a longer command: wait for the rest.
            "g" | "r" | "d" | "c" | "y" | ">" | "<" => {
                k.clone_into(&mut self.vim().pending);
            }
            _ => {}
        }
        Out::None
    }

    /// What the status line says: vim's mode or command line, a message,
    /// where the caret is.
    #[must_use]
    pub fn status(&self) -> String {
        let at = format!("{}:{}", self.caret.row + 1, screen_col(self.line(), self.caret.col) + 1);
        let left = if let Some(m) = &self.message {
            m.clone()
        } else if let Some(v) = &self.vim {
            if let Some(c) = &v.command {
                c.clone()
            } else {
                match v.mode {
                    Mode::Insert => "-- INSERT --".into(),
                    Mode::Visual => "-- VISUAL --".into(),
                    Mode::VisualLine => "-- VISUAL LINE --".into(),
                    Mode::Normal => v.pending.clone(),
                }
            }
        } else {
            String::new()
        };
        if left.is_empty() { at } else { format!("{left}   {at}") }
    }

    /// Whether the caret shows as a block (vim's normal and visual modes).
    #[must_use]
    pub fn block_caret(&self) -> bool {
        matches!(self.mode(), Some(Mode::Normal | Mode::Visual | Mode::VisualLine))
            && self.vim.as_ref().is_some_and(|v| v.command.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(text: &str) -> Code {
        Code::new(text, false)
    }

    fn vim(text: &str) -> Code {
        Code::new(text, true)
    }

    /// Keys typed as vim gets them: letters as text, `<Esc>`, `<CR>`.
    fn keys(c: &mut Code, ks: &str) -> Out {
        let mut out = Out::None;
        let mut rest = ks;
        'keys: while let Some(ch) = rest.chars().next() {
            for (name, vk) in [("<Esc>", VK_ESCAPE), ("<CR>", VK_RETURN), ("<BS>", VK_BACK)] {
                if let Some(r) = rest.strip_prefix(name) {
                    out = c.key(vk, false, false);
                    rest = r;
                    continue 'keys;
                }
            }
            out = c.type_text(&ch.to_string());
            rest = &rest[ch.len_utf8()..];
        }
        out
    }

    #[test]
    fn typing_enter_and_indent() {
        let mut c = plain("");
        c.type_text("if (x) {");
        c.key(VK_RETURN, false, false);
        c.type_text("go();");
        c.key(VK_RETURN, false, false);
        c.type_text("}");
        assert_eq!(c.text(), "if (x) {\n  go();\n}");
        // Backspace in an indent takes a step.
        let mut c = plain("  ");
        c.caret = Pos::new(0, 2);
        c.key(VK_BACK, false, false);
        assert_eq!(c.text(), "");
        // Tab with lines selected indents them; Shift+Tab takes it back.
        let mut c = plain("a\nb\nc");
        c.move_to(Pos::new(1, 1), true);
        c.key(VK_TAB, false, false);
        assert_eq!(c.text(), "  a\n  b\nc");
        c.key(VK_TAB, true, false);
        assert_eq!(c.text(), "a\nb\nc");
    }

    #[test]
    fn selecting_cutting_and_undo() {
        let mut c = plain("hello world");
        c.key(VK_END, true, false);
        assert_eq!(c.selected().as_deref(), Some("hello world"));
        c.key(VK_HOME, false, false);
        c.key(VK_RIGHT, true, true);
        assert_eq!(c.selected().as_deref(), Some("hello "));
        assert_eq!(c.key(0x58, false, true), Out::Copy("hello ".into()));
        assert_eq!(c.text(), "world");
        c.type_text("big ");
        c.type_text("w");
        assert_eq!(c.text(), "big wworld");
        // One Undo takes the typing, another the cut.
        c.key(0x5a, false, true);
        assert_eq!(c.text(), "world");
        c.key(0x5a, false, true);
        assert_eq!(c.text(), "hello world");
        c.key(0x59, false, true);
        assert_eq!(c.text(), "world");
        // Up and Down keep the column; åäö are one character each.
        let mut c = plain("åäö x\nab\nåäö y");
        c.caret = Pos::new(0, "åäö".len());
        c.key(VK_DOWN, false, false);
        assert_eq!(c.caret, Pos::new(1, 2));
        c.key(VK_DOWN, false, false);
        assert_eq!(c.caret, Pos::new(2, "åäö".len()));
        assert!(!c.changed() && c.status() == "3:4");
        assert_eq!(c.key(0x53, false, true), Out::Apply);
        assert_eq!(c.key(VK_ESCAPE, false, false), Out::Close);
    }

    #[test]
    fn vim_moves_and_edits() {
        let mut c = vim("one two three\nfour five\n\nsix");
        keys(&mut c, "w");
        assert_eq!(c.caret, Pos::new(0, 4));
        keys(&mut c, "dw");
        assert_eq!(c.lines[0], "one three");
        keys(&mut c, "u");
        assert_eq!(c.lines[0], "one two three");
        keys(&mut c, "cwTWO<Esc>");
        assert_eq!(c.lines[0], "one TWO three");
        assert_eq!(c.caret, Pos::new(0, 6));
        keys(&mut c, "$");
        assert_eq!(c.caret, Pos::new(0, 12));
        keys(&mut c, "0d$");
        assert_eq!(c.lines[0], "");
        keys(&mut c, "jyyP");
        assert_eq!(c.lines[..3], ["", "four five", "four five"]);
        keys(&mut c, "2dd");
        assert_eq!(c.lines, ["", "", "six"]);
        keys(&mut c, "GA!<Esc>");
        assert_eq!(c.lines[2], "six!");
        keys(&mut c, "ggOtop<Esc>");
        assert_eq!(c.lines[0], "top");
        keys(&mut c, "0xp");
        assert_eq!(c.lines[0], "otp");
        keys(&mut c, "03rz");
        assert_eq!(c.lines[0], "zzz");
        assert_eq!(c.mode(), Some(Mode::Normal));
    }

    #[test]
    fn vim_visual_search_and_commands() {
        let mut c = vim("let a = 1;\nlet b = 2;\nlog(a + b);");
        keys(&mut c, "Vjd");
        assert_eq!(c.lines, ["log(a + b);"]);
        keys(&mut c, "/b<CR>");
        assert_eq!(c.caret, Pos::new(0, 8));
        keys(&mut c, "0vey");
        assert_eq!(c.vim.as_ref().unwrap().register.0, "log");
        keys(&mut c, ">>");
        assert_eq!(c.lines[0], "  log(a + b);");
        assert_eq!(keys(&mut c, ":q<CR>"), Out::None);
        assert!(c.status().starts_with("E37"));
        assert_eq!(keys(&mut c, ":w<CR>"), Out::Apply);
        c.mark_saved();
        assert_eq!(keys(&mut c, ":q<CR>"), Out::Close);
        assert_eq!(keys(&mut c, ":wq<CR>"), Out::ApplyClose);
        assert_eq!(keys(&mut c, "x:q!<CR>"), Out::Discard);
        keys(&mut c, "i<Esc>");
        assert_eq!(c.status(), "1:2");
        keys(&mut c, "i");
        assert!(c.status().starts_with("-- INSERT --"));
    }

    #[test]
    fn columns_on_screen() {
        assert_eq!(screen_col("\tx", 1), 4);
        assert_eq!(screen_col("åx", "å".len()), 1);
        assert_eq!(col_at("\tx", 4), 1);
        assert_eq!(col_at("\tx", 1), 0);
        assert_eq!(col_at("åäö", 2), "åä".len());
        assert_eq!(col_at("ab", 9), 2);
    }
}
