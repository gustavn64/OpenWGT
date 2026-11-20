//! The built-in command language: a command a line, `#` starting a
//! comment (unless a digit follows it: `#7` is a part). [`api::COMMANDS`]
//! lists the commands. The input commands (`move`, `click`, `drag`,
//! `key`, `wait`...) are the comparison scripts' (`tools/compare`), with
//! the same timing, so the original in the VM can play the same files;
//! the others work with parts by name and with the run:
//!
//! ```text
//! open lamp.wgt
//! name #7 Lamp
//! run
//! press Switch
//! until Lamp == on max 120
//! expect Counter > 3
//! step 60
//! shot after-a-second
//! ```

use std::collections::VecDeque;
use std::fmt::Write as _;

use super::api::{self, Cap};
use super::runtime::{ErrKind, Limits, Poll, Runtime, ScriptError, TaskId, TaskInfo, TaskState};
use super::{
    Button, Caps, CmpOp, Cond, Host, HostError, Level, MouseEv, Selector, SettingKey, SettingVal, StepEnd, Val, select,
    value,
};

/// Milliseconds a window's frame takes, as the input commands move.
pub const FRAME_MS: u64 = 16;
/// The longest `until` waits by default: 10 s.
pub const UNTIL_MAX: u32 = 600;

/// A word of a line, quotes kept, and where it is (columns from 1, in
/// characters).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Tok {
    text: String,
    col: u32,
    width: u32,
}

/// Splits a line into words: whitespace between them, `"..."` one word
/// (or part of one), `#` not before a digit ending the line.
fn tokenize(line: &str) -> Result<Vec<Tok>, (u32, String)> {
    let chars: Vec<char> = line.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    let col = |i: usize| u32::try_from(i + 1).unwrap_or(u32::MAX);
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '#' && !chars.get(i + 1).is_some_and(char::is_ascii_digit) {
            break;
        }
        let start = i;
        let mut text = String::new();
        while i < chars.len() && !chars[i].is_whitespace() {
            if chars[i] == '#' && i > start && !chars.get(i + 1).is_some_and(char::is_ascii_digit) {
                break;
            }
            if chars[i] == '"' {
                let open = i;
                text.push('"');
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    text.push(chars[i]);
                    i += 1;
                }
                if i == chars.len() {
                    return Err((col(open), "the quote isn't closed".into()));
                }
            }
            text.push(chars[i]);
            i += 1;
        }
        toks.push(Tok {
            text,
            col: col(start),
            width: u32::try_from(i - start).unwrap_or(0),
        });
        if i < chars.len() && chars[i] == '#' {
            break;
        }
    }
    Ok(toks)
}

/// A number as the comparison scripts write them: decimal or `0x..`.
fn int(s: &str) -> Option<i32> {
    s.strip_prefix("0x")
        .map_or_else(|| s.parse().ok(), |h| i32::from_str_radix(h, 16).ok())
}

/// `"text"` without its quotes; a word as it is.
fn unquote(s: &str) -> &str {
    s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s)
}

/// A piece of a `log` line.
#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Text(String),
    Value(Selector),
}

#[derive(Debug, Clone, PartialEq)]
enum Cmd {
    Size,
    Move(i32, i32),
    Down(i32, i32, Button),
    Up(i32, i32, Button),
    Click(i32, i32, Button, u8),
    Drag(i32, i32, i32, i32, Button),
    Key(u16),
    Wait(u64),
    Shot(String),
    Dump,
    Open(String),
    Puzzle(i16),
    New,
    Run,
    Stop,
    Step(u32),
    Until(Cond, u32),
    Press(Selector, Option<(i32, i32)>),
    Set(Selector, Val),
    Poke(Selector, Val),
    Setting(Selector, SettingKey, SettingVal),
    Name(Selector, Option<String>),
    Seed(u64),
    Expect(Cond),
    Assert(Cond),
    Log(Vec<Piece>),
    Print(Vec<Selector>),
    Parts,
    Wires,
    Help(Option<String>),
}

#[derive(Debug, Clone, PartialEq)]
struct Line {
    /// From 1.
    n: u32,
    cmd: Cmd,
    cap: Cap,
    /// Where an error in it points: its main argument, else its command.
    focus: (u32, u32),
}

/// A parse error: its column and width (0: the whole line), and why.
type ParseErr = (u32, u32, String);

fn err_at(t: &Tok, msg: impl Into<String>) -> ParseErr {
    (t.col, t.width, msg.into())
}

/// The span from token `a` to token `b`.
fn span(a: &Tok, b: &Tok) -> (u32, u32) {
    (a.col, (b.col + b.width).saturating_sub(a.col))
}

/// Reads one line; `None` if it's empty or a comment.
#[allow(clippy::too_many_lines)]
fn parse_line(src: &str) -> Result<Option<(Cmd, Cap, (u32, u32))>, ParseErr> {
    let toks = tokenize(src).map_err(|(c, m)| (c, 1, m))?;
    let Some(verb) = toks.first() else { return Ok(None) };
    let name = verb.text.as_str();
    let Some(def) = api::command(name) else {
        let near = api::near(name);
        let mut msg = format!("no command {}", value::quote(name));
        if !near.is_empty() {
            let _ = write!(msg, ". Did you mean {}?", near.join(" or "));
        }
        msg.push_str(" (help lists them)");
        return Err(err_at(verb, msg));
    };
    let usage = || format!("usage: {}", api::usage(def));
    let arg = |i: usize| toks.get(i).ok_or_else(|| (verb.col, verb.width, usage()));
    let num = |i: usize| -> Result<i32, ParseErr> {
        let t = arg(i)?;
        int(&t.text).ok_or_else(|| err_at(t, format!("{} isn't a number; {}", t.text, usage())))
    };
    let sel = |i: usize| -> Result<Selector, ParseErr> {
        let t = arg(i)?;
        select::parse(&t.text).map_err(|e| err_at(t, e))
    };
    // No more words than it takes (the input commands, as before, take
    // whatever follows).
    let at_most = |n: usize| -> Result<(), ParseErr> {
        match toks.get(n + 1) {
            Some(t) if !def.input => Err(err_at(t, format!("too many words; {}", usage()))),
            _ => Ok(()),
        }
    };
    let button = if toks.iter().any(|t| t.text == "right") {
        Button::Right
    } else {
        Button::Left
    };
    let rest = |i: usize| -> String {
        toks.get(i).map_or_else(String::new, |t| {
            let from: String = src.chars().skip(usize::try_from(t.col - 1).unwrap_or(0)).collect();
            let end = toks.last().map_or(0, |l| l.col + l.width - t.col);
            from.chars().take(usize::try_from(end).unwrap_or(0)).collect()
        })
    };
    let focus1 = toks.get(1).map_or((verb.col, verb.width), |t| (t.col, t.width));
    let focus_rest = toks
        .last()
        .filter(|_| toks.len() > 1)
        .map_or(focus1, |l| span(&toks[1], l));
    let mut focus = (verb.col, verb.width);
    let cmd = match name {
        "size" => Cmd::Size,
        "move" => Cmd::Move(num(1)?, num(2)?),
        "down" => Cmd::Down(num(1)?, num(2)?, button),
        "up" => Cmd::Up(num(1)?, num(2)?, button),
        "click" => Cmd::Click(num(1)?, num(2)?, button, 1),
        "dclick" => Cmd::Click(num(1)?, num(2)?, button, 2),
        "drag" => Cmd::Drag(num(1)?, num(2)?, num(3)?, num(4)?, button),
        "key" => {
            let t = arg(1)?;
            Cmd::Key(num(1).and_then(|v| u16::try_from(v).map_err(|_| err_at(t, format!("{v} isn't a key code"))))?)
        }
        "wait" => {
            let t = arg(1)?;
            Cmd::Wait(num(1).and_then(|v| u64::try_from(v).map_err(|_| err_at(t, "a wait can't be negative")))?)
        }
        "shot" => Cmd::Shot(toks.get(1).map_or("shot", |t| unquote(&t.text)).to_owned()),
        "dump" => Cmd::Dump,
        "open" => {
            focus = focus1;
            Cmd::Open(unquote(&arg(1)?.text).to_owned())
        }
        "puzzle" => {
            let t = arg(1)?;
            Cmd::Puzzle(num(1).and_then(|v| i16::try_from(v).map_err(|_| err_at(t, format!("no puzzle {v}"))))?)
        }
        "new" => {
            at_most(0)?;
            Cmd::New
        }
        "run" => {
            at_most(0)?;
            Cmd::Run
        }
        "stop" => {
            at_most(0)?;
            Cmd::Stop
        }
        "step" => {
            at_most(1)?;
            match toks.get(1) {
                None => Cmd::Step(1),
                Some(t) => {
                    Cmd::Step(num(1).and_then(|v| u32::try_from(v).map_err(|_| err_at(t, "a step can't be negative")))?)
                }
            }
        }
        "until" => {
            focus = focus_rest;
            let (end, max) = match toks.iter().rposition(|t| t.text == "max") {
                Some(m) if m > 1 => {
                    at_most(m + 1)?;
                    let t = arg(m + 1)?;
                    let v = num(m + 1)?;
                    (m, u32::try_from(v).map_err(|_| err_at(t, "max is a number of ticks"))?)
                }
                _ => (toks.len(), UNTIL_MAX),
            };
            focus = if end > 1 { span(&toks[1], &toks[end - 1]) } else { focus };
            let c = cond(&toks[1..end], verb, true)?;
            Cmd::Until(c, max)
        }
        "expect" | "assert" => {
            focus = focus_rest;
            let c = cond(&toks[1..], verb, false)?;
            if name == "expect" {
                Cmd::Expect(c)
            } else {
                Cmd::Assert(c)
            }
        }
        "press" => {
            focus = focus1;
            let s = sel(1)?;
            let at = match toks.len() {
                2 => None,
                4 => Some((num(2)?, num(3)?)),
                _ => return Err((verb.col, 0, usage())),
            };
            Cmd::Press(s, at)
        }
        "set" | "poke" => {
            focus = focus1;
            let s = sel(1)?;
            let t = arg(2)?;
            let v = value::parse(&rest(2)).map_err(|e| (t.col, 0, e))?;
            if name == "set" { Cmd::Set(s, v) } else { Cmd::Poke(s, v) }
        }
        "setting" => {
            focus = focus1;
            let s = sel(1)?;
            let k = arg(2)?;
            let key = setting_key(&k.text).ok_or_else(|| {
                err_at(
                    k,
                    format!(
                        "{}: a setting is check0..check31, radio0..radio3, number0..number3, text0..text3 or text",
                        k.text
                    ),
                )
            })?;
            let t = arg(3)?;
            let text = rest(3);
            let v = match key {
                SettingKey::Check(_) => match value::parse(&text) {
                    Ok(Val::Bool(b)) => SettingVal::Bool(b),
                    _ => return Err(err_at(t, "a check box is on or off")),
                },
                SettingKey::Radio(_) | SettingKey::Number(_) => SettingVal::Int(
                    text.parse()
                        .map_err(|_| err_at(t, format!("{text} isn't a whole number from 0")))?,
                ),
                SettingKey::Text(_) | SettingKey::LongText => SettingVal::Text(unquote(&text).to_owned()),
            };
            Cmd::Setting(s, key, v)
        }
        "name" => {
            focus = focus1;
            let s = sel(1)?;
            let n = toks.get(2).map(|_| unquote(&rest(2)).to_owned());
            Cmd::Name(s, n)
        }
        "seed" => {
            at_most(1)?;
            let t = arg(1)?;
            let v = t
                .text
                .strip_prefix("0x")
                .map_or_else(|| t.text.parse().ok(), |h| u64::from_str_radix(h, 16).ok())
                .ok_or_else(|| err_at(t, "a seed is a whole number from 0"))?;
            Cmd::Seed(v)
        }
        "log" => Cmd::Log(pieces(&rest(1), toks.get(1).map_or(0, |t| t.col))?),
        "print" => {
            focus = focus_rest;
            let sels = (1..toks.len()).map(sel).collect::<Result<Vec<_>, _>>()?;
            if sels.is_empty() {
                return Err((verb.col, verb.width, usage()));
            }
            Cmd::Print(sels)
        }
        "parts" => {
            at_most(0)?;
            Cmd::Parts
        }
        "wires" => {
            at_most(0)?;
            Cmd::Wires
        }
        "help" => Cmd::Help(toks.get(1).map(|t| t.text.clone())),
        _ => return Err(err_at(verb, format!("{name} isn't done yet"))),
    };
    Ok(Some((cmd, def.cap, focus)))
}

/// A setting's key: `check0`, `radio1`, `number2`, `text3`, `text`.
fn setting_key(s: &str) -> Option<SettingKey> {
    if s == "text" {
        return Some(SettingKey::LongText);
    }
    let split = s.find(|c: char| c.is_ascii_digit())?;
    let (k, n) = s.split_at(split);
    let n: u8 = n.parse().ok()?;
    match k {
        "check" if n < 32 => Some(SettingKey::Check(n)),
        "radio" if n < 4 => Some(SettingKey::Radio(n)),
        "number" if n < 4 => Some(SettingKey::Number(n)),
        "text" if n < 4 => Some(SettingKey::Text(n)),
        _ => None,
    }
}

/// `log`'s text: `{PART}` and `{PART.KNOT}` become values.
fn pieces(text: &str, col: u32) -> Result<Vec<Piece>, ParseErr> {
    let mut out = Vec::new();
    let mut rest = text;
    let mut at = col;
    let width = |s: &str| u32::try_from(s.chars().count()).unwrap_or(0);
    while let Some(open) = rest.find('{') {
        let close = rest[open..]
            .find('}')
            .ok_or_else(|| (at + width(&rest[..open]), 1, "the { isn't closed".to_owned()))?;
        if open > 0 {
            out.push(Piece::Text(rest[..open].to_owned()));
        }
        let inner = &rest[open + 1..open + close];
        let s = select::parse(inner.trim()).map_err(|e| (at + width(&rest[..open]), width(inner) + 2, e))?;
        out.push(Piece::Value(s));
        at += width(&rest[..=open + close]);
        rest = &rest[open + close + 1..];
    }
    if !rest.is_empty() {
        out.push(Piece::Text(rest.to_owned()));
    }
    Ok(out)
}

/// The operators, longest first.
const OPS: [(&str, CmpOp); 7] = [
    ("==", CmpOp::Eq),
    ("!=", CmpOp::Ne),
    ("<=", CmpOp::Le),
    (">=", CmpOp::Ge),
    ("<", CmpOp::Lt),
    (">", CmpOp::Gt),
    ("=", CmpOp::Eq),
];

/// Splits unquoted words around operators: `Lamp==on` is three.
fn split_ops(toks: &[Tok]) -> Vec<Tok> {
    let mut out = Vec::new();
    for t in toks {
        if t.text.starts_with('"') {
            out.push(t.clone());
            continue;
        }
        let mut s = t.text.as_str();
        let mut col = t.col;
        while !s.is_empty() {
            let hit = (0..s.len()).filter(|&i| s.is_char_boundary(i)).find_map(|i| {
                OPS.iter()
                    .find(|(o, _)| s[i..].starts_with(o))
                    .map(|(o, _)| (i, o.len()))
            });
            let w = |x: &str| u32::try_from(x.chars().count()).unwrap_or(0);
            match hit {
                Some((0, n)) => {
                    out.push(Tok {
                        text: s[..n].to_owned(),
                        col,
                        width: w(&s[..n]),
                    });
                    col += w(&s[..n]);
                    s = &s[n..];
                }
                Some((i, _)) => {
                    out.push(Tok {
                        text: s[..i].to_owned(),
                        col,
                        width: w(&s[..i]),
                    });
                    col += w(&s[..i]);
                    s = &s[i..];
                }
                None => {
                    out.push(Tok {
                        text: s.to_owned(),
                        col,
                        width: w(s),
                    });
                    s = "";
                }
            }
        }
    }
    out
}

/// A condition: `or` of `and`s of (`not`) atoms.
fn cond(toks: &[Tok], verb: &Tok, changed_ok: bool) -> Result<Cond, ParseErr> {
    let toks = split_ops(toks);
    if toks.is_empty() {
        return Err(err_at(
            verb,
            format!("{} what? Such as: {} Lamp == on", verb.text, verb.text),
        ));
    }
    let mut p = CondParser {
        toks: &toks,
        i: 0,
        changed_ok,
    };
    let c = p.or()?;
    if let Some(t) = toks.get(p.i) {
        return Err(err_at(
            t,
            format!("unexpected {} (join conditions with and / or)", t.text),
        ));
    }
    Ok(c)
}

struct CondParser<'a> {
    toks: &'a [Tok],
    i: usize,
    changed_ok: bool,
}

impl CondParser<'_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i)
    }

    fn or(&mut self) -> Result<Cond, ParseErr> {
        let mut v = vec![self.and()?];
        while self.peek().is_some_and(|t| t.text == "or") {
            self.i += 1;
            v.push(self.and()?);
        }
        Ok(if v.len() == 1 { v.remove(0) } else { Cond::Any(v) })
    }

    fn and(&mut self) -> Result<Cond, ParseErr> {
        let mut v = vec![self.unary()?];
        while self.peek().is_some_and(|t| t.text == "and") {
            self.i += 1;
            v.push(self.unary()?);
        }
        Ok(if v.len() == 1 { v.remove(0) } else { Cond::All(v) })
    }

    fn unary(&mut self) -> Result<Cond, ParseErr> {
        let last = self.toks.last().cloned();
        let Some(t) = self.peek().cloned() else {
            let at = last.map_or((0, 0), |l| (l.col + l.width, 1));
            return Err((at.0, at.1, "a condition is missing here".into()));
        };
        self.i += 1;
        match t.text.as_str() {
            "not" => return Ok(Cond::Not(Box::new(self.unary()?))),
            "running" => return Ok(Cond::Running),
            "stopped" => return Ok(Cond::Stopped),
            "solved" => return Ok(Cond::Solved),
            _ => {}
        }
        let sel = select::parse(&t.text).map_err(|e| err_at(&t, e))?;
        let Some(next) = self.peek().cloned() else {
            return Ok(Cond::Truthy(sel));
        };
        if next.text == "changed" {
            self.i += 1;
            if !self.changed_ok {
                return Err(err_at(&next, "changed only works with until"));
            }
            return Ok(Cond::Changed(sel));
        }
        let Some(&(_, op)) = OPS.iter().find(|(o, _)| *o == next.text) else {
            return Ok(Cond::Truthy(sel));
        };
        self.i += 1;
        let Some(v) = self.peek().cloned() else {
            return Err(err_at(&next, format!("{} what?", next.text)));
        };
        self.i += 1;
        let mut text = v.text.clone();
        let mut end = v.clone();
        if text == "picture"
            && let Some(n) = self.peek().cloned()
        {
            self.i += 1;
            text = format!("picture {}", n.text);
            end = n;
        }
        let val = value::parse(&text).map_err(|e| (v.col, span(&v, &end).1, e))?;
        Ok(Cond::Cmp(sel, op, val))
    }
}

/// What a task does next, below a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Mouse(MouseEv),
    Key(u16, bool),
    Wait(u64),
}

struct Task {
    id: TaskId,
    file: String,
    src: Vec<String>,
    lines: Vec<Line>,
    /// The next line's index in `lines`.
    pc: usize,
    caps: Caps,
    state: TaskState,
    /// The current command's steps still to do.
    ops: VecDeque<Op>,
    /// Where the input commands left the mouse.
    mouse: (i32, i32),
    errors: Vec<ScriptError>,
    failures: u32,
    /// The line being run (from 1).
    line: u32,
}

/// The built-in language's runtime.
#[derive(Default)]
pub struct CommandRuntime {
    tasks: Vec<Task>,
    next: u32,
    limits: Limits,
}

/// Reads a whole script.
fn parse(file: &str, src: &str) -> Result<Vec<Line>, ScriptError> {
    let mut lines = Vec::new();
    for (i, l) in src.lines().enumerate() {
        let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
        match parse_line(l) {
            Ok(Some((cmd, cap, focus))) => lines.push(Line { n, cmd, cap, focus }),
            Ok(None) => {}
            Err((col, width, msg)) => {
                return Err(ScriptError::at(
                    ErrKind::Syntax,
                    msg,
                    file,
                    n,
                    col,
                    width,
                    Some(l.trim_end()),
                ));
            }
        }
    }
    Ok(lines)
}

/// The editor's size a script asks for with `size W H`, if it does.
#[must_use]
pub fn size_of(src: &str) -> Option<(u32, u32)> {
    let l = src.lines().find(|l| l.trim_start().starts_with("size "))?;
    let n: Vec<u32> = l.split_whitespace().skip(1).filter_map(|v| v.parse().ok()).collect();
    match n[..] {
        [w, h, ..] => Some((w, h)),
        _ => None,
    }
}

impl CommandRuntime {
    #[must_use]
    pub fn new() -> CommandRuntime {
        CommandRuntime::default()
    }

    fn task(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }
}

impl Task {
    fn error(&self, line: &Line, e: &HostError) -> ScriptError {
        let src = self
            .src
            .get(usize::try_from(line.n - 1).unwrap_or(0))
            .map(|s| s.trim_end());
        let (mut col, mut width) = line.focus;
        // A part not found: under its name, where the line has it.
        if let (super::ErrorKind::UnknownPart { query, .. }, Some(src)) = (&e.kind, src) {
            let chars: Vec<char> = src.chars().collect();
            let q: Vec<char> = query.chars().collect();
            let from = usize::try_from(col.saturating_sub(1)).unwrap_or(0);
            let to = from + usize::try_from(width).unwrap_or(0);
            if let Some(at) = (from..to.min(chars.len()).saturating_sub(q.len().saturating_sub(1)))
                .find(|&i| !q.is_empty() && chars[i..].starts_with(&q))
            {
                col = u32::try_from(at + 1).unwrap_or(col);
                width = u32::try_from(q.len()).unwrap_or(width);
            }
        }
        ScriptError::host(e, &self.file, line.n, col, width, src)
    }

    /// Ends the task with `e`.
    fn fail(&mut self, host: &mut dyn Host, e: ScriptError) {
        host.log(Level::Error, &e.to_string());
        self.errors.push(e);
        self.state = TaskState::Failed;
        self.ops.clear();
    }

    /// The mouse to `(x, y)` in steps of 8 pixels, a frame each, as a hand
    /// moves it (a drag sees the way).
    fn to(&mut self, x: i32, y: i32) {
        let (x0, y0) = self.mouse;
        let n = (((x - x0).abs().max((y - y0).abs()) + 7) / 8).max(1);
        for i in 1..=n {
            self.ops
                .push_back(Op::Mouse(MouseEv::Move(x0 + (x - x0) * i / n, y0 + (y - y0) * i / n)));
            self.ops.push_back(Op::Wait(FRAME_MS));
        }
        self.mouse = (x, y);
    }

    /// Runs on until it has to wait or ends.
    fn resume(&mut self, host: &mut dyn Host, budget: u64) -> Poll {
        let mut spent = 0;
        loop {
            if !matches!(self.state, TaskState::Running | TaskState::Sleeping { .. }) {
                return Poll::Done;
            }
            self.state = TaskState::Running;
            while let Some(op) = self.ops.pop_front() {
                match op {
                    Op::Wait(ms) => {
                        self.state = TaskState::Sleeping { ms };
                        return Poll::Sleep(ms);
                    }
                    Op::Mouse(ev) => host.mouse(ev),
                    Op::Key(vk, down) => host.key(vk, down),
                }
            }
            let Some(line) = self.lines.get(self.pc).cloned() else {
                self.state = TaskState::Done;
                return Poll::Done;
            };
            self.pc += 1;
            self.line = line.n;
            spent += 1;
            if spent > budget {
                let e = ScriptError::at(
                    ErrKind::Budget,
                    "the script ran too long without waiting; stopped",
                    &self.file,
                    line.n,
                    0,
                    0,
                    None,
                );
                self.fail(host, e);
                return Poll::Done;
            }
            if !line.cap.allowed(self.caps) {
                let e = HostError::new(super::ErrorKind::Caps, "a script from a widget can't do that");
                let e = self.error(&line, &e);
                self.fail(host, e);
                return Poll::Done;
            }
            match self.exec(&line, host) {
                Ok(()) => {}
                Err(Fault::Check(e)) => {
                    host.fail(&e.to_string());
                    self.errors.push(e);
                    self.failures += 1;
                }
                Err(Fault::Fatal(e)) => {
                    if e.kind == ErrKind::Assert {
                        host.fail(&e.to_string());
                        self.errors.push(e);
                        self.failures += 1;
                        self.state = TaskState::Failed;
                        self.ops.clear();
                    } else {
                        self.fail(host, e);
                    }
                    return Poll::Done;
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn exec(&mut self, line: &Line, host: &mut dyn Host) -> Result<(), Fault> {
        let fatal = |t: &Task, e: HostError| Fault::Fatal(t.error(line, &e));
        let resolve = |t: &Task, host: &dyn Host, s: &Selector| host.resolve(&s.part).map_err(|e| fatal(t, e));
        match &line.cmd {
            Cmd::Size => {}
            &Cmd::Move(x, y) => self.to(x, y),
            &Cmd::Down(x, y, b) => {
                self.to(x, y);
                self.ops.push_back(Op::Mouse(MouseEv::Down(x, y, b)));
                self.ops.push_back(Op::Wait(FRAME_MS));
            }
            &Cmd::Up(x, y, b) => {
                self.to(x, y);
                self.ops.push_back(Op::Mouse(MouseEv::Up(x, y, b)));
                self.ops.push_back(Op::Wait(FRAME_MS));
            }
            &Cmd::Click(x, y, b, times) => {
                self.to(x, y);
                for _ in 0..times {
                    self.ops.push_back(Op::Mouse(MouseEv::Down(x, y, b)));
                    self.ops.push_back(Op::Wait(80));
                    self.ops.push_back(Op::Mouse(MouseEv::Up(x, y, b)));
                    self.ops.push_back(Op::Wait(120));
                }
            }
            &Cmd::Drag(x0, y0, x1, y1, b) => {
                self.to(x0, y0);
                self.ops.push_back(Op::Mouse(MouseEv::Down(x0, y0, b)));
                self.ops.push_back(Op::Wait(200));
                self.to(x1, y1);
                self.ops.push_back(Op::Wait(200));
                self.ops.push_back(Op::Mouse(MouseEv::Up(x1, y1, b)));
                self.ops.push_back(Op::Wait(200));
            }
            &Cmd::Key(vk) => {
                self.ops
                    .extend([Op::Key(vk, true), Op::Wait(80), Op::Key(vk, false), Op::Wait(80)]);
            }
            &Cmd::Wait(ms) => self.ops.push_back(Op::Wait(ms)),
            Cmd::Shot(name) => host.shot(name).map_err(|e| fatal(self, e))?,
            Cmd::Dump => {
                let d = host.dump();
                host.log(Level::Info, &d);
            }
            Cmd::Open(path) => {
                host.open_file(path).map_err(|e| fatal(self, e))?;
                self.ops.push_back(Op::Wait(300));
            }
            &Cmd::Puzzle(n) => {
                host.load_puzzle(n).map_err(|e| fatal(self, e))?;
                self.ops.push_back(Op::Wait(300));
            }
            Cmd::New => host.new_widget(),
            Cmd::Run => host.run(),
            Cmd::Stop => host.stop(),
            &Cmd::Step(n) => match host.step(n).map_err(|e| fatal(self, e))? {
                StepEnd::Ran(_) => {}
                StepEnd::Stopped { solved, at } => host.log(
                    Level::Info,
                    &format!(
                        "the run ended after {}{}",
                        value::ticks(at),
                        if solved { ", solved" } else { "" }
                    ),
                ),
                StepEnd::NotRunning => {
                    return Err(fatal(
                        self,
                        HostError::new(
                            super::ErrorKind::NotRunning,
                            "step needs the widget running (run first)",
                        ),
                    ));
                }
            },
            Cmd::Until(c, max) => {
                host.until(c, *max).map_err(|e| fatal(self, e))?;
            }
            Cmd::Press(s, at) => {
                let id = resolve(self, host, s)?;
                host.press(id, *at).map_err(|e| fatal(self, e))?;
            }
            Cmd::Set(s, v) => {
                let id = resolve(self, host, s)?;
                let took = host.set_value(id, v).map_err(|e| fatal(self, e))?;
                if took != *v {
                    let who = host.part(id).map_or_else(|_| id.to_string(), |p| p.describe());
                    host.log(
                        Level::Info,
                        &format!("{who} is {} (the nearest it can be)", value::format(&took)),
                    );
                }
            }
            Cmd::Poke(s, v) => {
                let id = resolve(self, host, s)?;
                host.poke(id, v).map_err(|e| fatal(self, e))?;
            }
            Cmd::Setting(s, k, v) => {
                let id = resolve(self, host, s)?;
                host.set_setting(id, *k, v).map_err(|e| fatal(self, e))?;
            }
            Cmd::Name(s, n) => {
                let id = resolve(self, host, s)?;
                host.set_name(id, n.as_deref()).map_err(|e| fatal(self, e))?;
            }
            &Cmd::Seed(n) => host.seed(n),
            Cmd::Expect(c) | Cmd::Assert(c) => {
                let ok = host.eval(c, &[]).map_err(|e| fatal(self, e))?;
                if !ok {
                    let src = self
                        .src
                        .get(usize::try_from(line.n - 1).unwrap_or(0))
                        .map(|s| s.trim_end());
                    let msg = format!("expected {}{}", value::show_cond(c), host.actual(c));
                    let e = ScriptError::at(
                        ErrKind::Assert,
                        msg,
                        &self.file,
                        line.n,
                        line.focus.0,
                        line.focus.1,
                        src,
                    );
                    return Err(if matches!(line.cmd, Cmd::Assert(_)) {
                        Fault::Fatal(e)
                    } else {
                        Fault::Check(e)
                    });
                }
            }
            Cmd::Log(pieces) => {
                let mut s = String::new();
                for p in pieces {
                    match p {
                        Piece::Text(t) => s.push_str(t),
                        Piece::Value(sel) => s.push_str(&value::format(&host.value(sel).map_err(|e| fatal(self, e))?)),
                    }
                }
                host.log(Level::Info, &s);
            }
            Cmd::Print(list) => {
                let mut out = Vec::new();
                for sel in list {
                    let v = host.value(sel).map_err(|e| fatal(self, e))?;
                    out.push(format!("{} = {}", select::show(sel), value::format(&v)));
                }
                host.log(Level::Info, &out.join(", "));
            }
            Cmd::Parts => {
                let text = parts_text(host);
                host.log(Level::Info, &text);
            }
            Cmd::Wires => {
                let text = wires_text(host);
                host.log(Level::Info, &text);
            }
            Cmd::Help(name) => host.log(Level::Info, &api::help(name.as_deref())),
        }
        Ok(())
    }
}

/// A command that went wrong: a failed `expect` (the script goes on) or
/// anything else (it ends).
enum Fault {
    Check(ScriptError),
    Fatal(ScriptError),
}

/// `parts`: a line a part, in reading order.
fn parts_text(host: &dyn Host) -> String {
    let parts = host.parts();
    if parts.is_empty() {
        return "no parts".into();
    }
    let mut out = Vec::new();
    for p in &parts {
        let mut s = format!("{:>5} {}", p.id.to_string(), p.kind_words());
        if let Some(n) = &p.name {
            let _ = write!(s, " named {}", value::quote(n));
        }
        if let Some(l) = &p.label {
            let _ = write!(s, " labelled {}", value::quote(l));
        }
        let _ = write!(s, " at {},{}", p.rect.0, p.rect.1);
        let ins: Vec<String> = p.inputs().map(|k| value::format(&k.value)).collect();
        let outs: Vec<String> = p.outputs().map(|k| value::format(&k.value)).collect();
        if !ins.is_empty() {
            let _ = write!(s, "  in: {}", ins.join(", "));
        }
        if !outs.is_empty() {
            let _ = write!(s, "  out: {}", outs.join(", "));
        }
        if p.locked {
            s.push_str("  (locked)");
        }
        if p.broken {
            s.push_str("  (broken)");
        }
        out.push(s);
    }
    out.join("\n")
}

/// `wires`: a line a wire.
fn wires_text(host: &dyn Host) -> String {
    let parts = host.parts();
    let end = |id: super::PartId, k: u8| -> String {
        let Some(p) = parts.iter().find(|p| p.id == id) else {
            return format!("{id}.{k}");
        };
        let Some(knot) = p.knots.get(usize::from(k)) else {
            return format!("{}.{k}", p.reference());
        };
        let nth = p.knots[..usize::from(k)]
            .iter()
            .filter(|o| o.input == knot.input)
            .count();
        let dir = if knot.input { "in" } else { "out" };
        let n = if nth == 0 { String::new() } else { (nth + 1).to_string() };
        format!("{}.{dir}{n}", p.reference())
    };
    let wires = host.wires();
    if wires.is_empty() {
        return "no wires".into();
    }
    let changed = |w| host.wire_changed(w).unwrap_or(false);
    wires
        .iter()
        .map(|(w, v)| {
            format!(
                "{} -> {}: {}{}",
                end(w.from.0, w.from.1),
                end(w.to.0, w.to.1),
                value::format(v),
                if changed(w) { " (changed)" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl Runtime for CommandRuntime {
    fn language(&self) -> &'static str {
        "commands"
    }

    fn check(&mut self, file: &str, src: &str) -> Result<(), ScriptError> {
        parse(file, src).map(|_| ())
    }

    fn start(&mut self, file: &str, src: &str, caps: Caps, _host: &mut dyn Host) -> Result<TaskId, ScriptError> {
        let lines = parse(file, src)?;
        self.next += 1;
        let id = TaskId(self.next);
        self.tasks.push(Task {
            id,
            file: file.to_owned(),
            src: src.lines().map(str::to_owned).collect(),
            lines,
            pc: 0,
            caps,
            state: TaskState::Running,
            ops: VecDeque::new(),
            mouse: (0, 0),
            errors: Vec::new(),
            failures: 0,
            line: 0,
        });
        Ok(id)
    }

    fn repl(&mut self, line: &str, host: &mut dyn Host) -> Result<Option<String>, ScriptError> {
        // A task of its own, run as far as it goes now; its waits go on
        // in the frames to come.
        let id = self.start("console", line, Caps::ALL, host)?;
        self.resume(id, host);
        Ok(None)
    }

    fn frame(&mut self, host: &mut dyn Host, ms: u64) {
        let budget = self.limits.budget;
        for t in &mut self.tasks {
            if let TaskState::Sleeping { ms: left } = t.state {
                let left = left.saturating_sub(ms);
                t.state = TaskState::Sleeping { ms: left };
                if left > 0 {
                    continue;
                }
            }
            if matches!(t.state, TaskState::Running | TaskState::Sleeping { .. }) {
                t.resume(host, budget);
            }
        }
    }

    fn after_tick(&mut self, _host: &mut dyn Host) {
        // The commands' waits for ticks run the ticks themselves.
    }

    fn resume(&mut self, task: TaskId, host: &mut dyn Host) -> Poll {
        let budget = self.limits.budget;
        match self.task(task) {
            Some(t) => t.resume(host, budget),
            None => Poll::Done,
        }
    }

    fn cancel(&mut self, task: Option<TaskId>) {
        for t in &mut self.tasks {
            if task.is_none_or(|id| id == t.id) && matches!(t.state, TaskState::Running | TaskState::Sleeping { .. }) {
                t.state = TaskState::Failed;
                t.ops.clear();
                t.errors.push(ScriptError::at(
                    ErrKind::Cancelled,
                    "stopped",
                    &t.file,
                    t.line,
                    0,
                    0,
                    None,
                ));
            }
        }
    }

    fn tasks(&self) -> Vec<TaskInfo> {
        self.tasks
            .iter()
            .map(|t| TaskInfo {
                id: t.id,
                file: t.file.clone(),
                state: t.state,
                line: t.line,
                failures: t.failures,
            })
            .collect()
    }

    fn errors(&self, task: TaskId) -> Vec<ScriptError> {
        self.tasks
            .iter()
            .find(|t| t.id == task)
            .map(|t| t.errors.clone())
            .unwrap_or_default()
    }

    fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
    }

    /// The command names for the first word; after it the parts (their
    /// names, labels, kinds and ids), a part's knots after its `.`, and
    /// the words values and conditions use.
    fn complete(&self, line: &str, cursor: usize, host: &dyn Host) -> Vec<String> {
        let head = line.get(..cursor.min(line.len())).unwrap_or(line);
        let start = head.rfind(char::is_whitespace).map_or(0, |i| i + 1);
        let word = &head[start..];
        if start == 0 || head[..start].trim().is_empty() {
            return api::COMMANDS
                .iter()
                .filter(|c| c.name.starts_with(word))
                .map(|c| c.name.to_owned())
                .collect();
        }
        complete_arg(word, host)
    }
}

/// What a command's argument starting `word` could be.
fn complete_arg(word: &str, host: &dyn Host) -> Vec<String> {
    let parts = host.parts();
    // A knot of a part: `Gate.in2`.
    if let Some(dot) = word.rfind('.')
        && let Ok(sel) = super::select::parse(&word[..dot])
        && let Ok(id) = host.resolve(&sel.part)
        && let Some(p) = parts.iter().find(|p| p.id == id)
    {
        let (mut ins, mut outs) = (0, 0);
        let mut v = Vec::new();
        for k in &p.knots {
            let (n, what) = if k.input { (&mut ins, "in") } else { (&mut outs, "out") };
            *n += 1;
            let name = if *n == 1 { what.to_owned() } else { format!("{what}{n}") };
            v.push(format!("{}.{name}", &word[..dot]));
        }
        return v.into_iter().filter(|c| c.starts_with(word)).collect();
    }
    let mut v: Vec<String> = Vec::new();
    for p in &parts {
        v.push(format!("#{}", p.id.0));
        v.push(p.kind.clone());
        for n in p.name.iter().chain(&p.label) {
            v.push(super::value::quote(n));
        }
    }
    let words = [
        "on", "off", "nothing", "picture", "changed", "running", "stopped", "solved", "not", "and", "or", "max",
    ];
    v.extend(words.iter().map(|w| (*w).to_owned()));
    let low = word.to_lowercase();
    v.retain(|c| c.to_lowercase().starts_with(&low));
    v.sort();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<String> {
        tokenize(s).unwrap().into_iter().map(|t| t.text).collect()
    }

    #[test]
    fn comments_end_lines_but_ids_dont() {
        assert_eq!(
            toks("click 330 53          # the tray's fifth folder"),
            ["click", "330", "53"]
        );
        assert_eq!(toks("name #7 Lamp # a comment"), ["name", "#7", "Lamp"]);
        assert_eq!(toks("press \"Big lamp\".in2 #comment"), ["press", "\"Big lamp\".in2"]);
        assert_eq!(toks("#7 isn't a comment"), ["#7", "isn't", "a", "comment"]);
        assert_eq!(toks("wait 5#x"), ["wait", "5"]);
        assert!(tokenize("press \"Big").is_err());
    }

    #[test]
    fn input_commands_read_as_before() {
        let l = parse(
            "t",
            "click 330 53\ndrag 160 92 200 260 right\nkey 0x20\nwait 500\nshot\nsize 640 480",
        )
        .unwrap();
        let cmds: Vec<Cmd> = l.into_iter().map(|l| l.cmd).collect();
        assert_eq!(
            cmds,
            [
                Cmd::Click(330, 53, Button::Left, 1),
                Cmd::Drag(160, 92, 200, 260, Button::Right),
                Cmd::Key(0x20),
                Cmd::Wait(500),
                Cmd::Shot("shot".into()),
                Cmd::Size,
            ]
        );
        assert_eq!(size_of("# x\nsize 800 600\n"), Some((800, 600)));
    }

    #[test]
    fn conditions_read() {
        let l = parse("t", "until Lamp == on and not Gate.in2 changed or stopped max 120").unwrap();
        let Cmd::Until(c, 120) = &l[0].cmd else {
            panic!("{:?}", l[0].cmd)
        };
        assert_eq!(value::show_cond(c), "Lamp == on and not Gate.in2 changed or stopped");
        let l = parse("t", "expect Counter>=3").unwrap();
        let Cmd::Expect(c) = &l[0].cmd else { panic!() };
        assert_eq!(value::show_cond(c), "Counter >= 3");
        let l = parse("t", "expect #3 == picture 1204").unwrap();
        let Cmd::Expect(Cond::Cmp(_, CmpOp::Eq, Val::Picture(1204))) = &l[0].cmd else {
            panic!()
        };
    }

    #[test]
    fn syntax_errors_point_at_the_place() {
        let e = parse("check.txt", "run\nexpect Lamp == maybe").unwrap_err();
        assert_eq!(e.kind, ErrKind::Syntax);
        assert_eq!((e.line, e.column), (2, 16));
        assert_eq!(
            e.to_string(),
            "check.txt:2:16  maybe isn't a value: say on, off, a number, picture N or nothing\n \
2 | expect Lamp == maybe\n   |                ^^^^^"
        );
        let e = parse("t", "presss Lamp").unwrap_err();
        assert_eq!(e.message, "no command presss. Did you mean press? (help lists them)");
        let e = parse("t", "expect Lamp changed").unwrap_err();
        assert_eq!(e.message, "changed only works with until");
        let e = parse("t", "run now").unwrap_err();
        assert_eq!(e.message, "too many words; usage: run");
        let e = parse("t", "click 10").unwrap_err();
        assert_eq!(e.message, "usage: click X Y [right]");
        let e = parse("t", "log {Lamp").unwrap_err();
        assert_eq!((e.column, e.message.as_str()), (5, "the { isn't closed"));
    }

    #[test]
    fn log_pieces() {
        let l = parse("t", "log the lamp is {Lamp} and {Gate.in2}!").unwrap();
        let Cmd::Log(p) = &l[0].cmd else { panic!() };
        assert_eq!(p.len(), 5);
        assert_eq!(p[0], Piece::Text("the lamp is ".into()));
        assert_eq!(p[4], Piece::Text("!".into()));
    }

    #[test]
    fn settings_and_names_read() {
        let l = parse(
            "t",
            "setting Slider check2 on\nsetting #3 text \"hello there\"\nname #3 Big lamp",
        )
        .unwrap();
        assert_eq!(
            l[0].cmd,
            Cmd::Setting(
                select::parse("Slider").unwrap(),
                SettingKey::Check(2),
                SettingVal::Bool(true)
            )
        );
        assert_eq!(
            l[1].cmd,
            Cmd::Setting(
                select::parse("#3").unwrap(),
                SettingKey::LongText,
                SettingVal::Text("hello there".into())
            )
        );
        assert_eq!(
            l[2].cmd,
            Cmd::Name(select::parse("#3").unwrap(), Some("Big lamp".into()))
        );
    }
}
