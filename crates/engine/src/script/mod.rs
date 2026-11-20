//! Scripts: what a script sees of the editor and the widget in it, and
//! what it can do there, whatever the language it's written in.
//!
//! - [`Host`] is the editor's side (implemented in `editor::debug`): the
//!   parts, their values, the run, input and output.
//! - [`runtime::Runtime`] is a language's side; [`cmd`] is the built-in
//!   one, a command a line (a superset of the comparison scripts in
//!   `tools/compare`).
//! - [`select`] finds parts by id, name, label or kind; [`value`] reads
//!   and writes values as text; [`adapt`] sets a part's own control;
//!   [`api`] lists what there is, for help and argument checks.
//!
//! Nothing here does IO: files, the clock and where output goes are the
//! host's.

pub mod adapt;
pub mod api;
pub mod cmd;
pub mod runtime;
pub mod select;
pub mod value;

use std::fmt;

/// A part, by the id the widget keeps it under ([`crate::gadget::Core::id`]),
/// which stays as parts move, are raised or others are deleted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartId(pub u32);

impl fmt::Display for PartId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// A wire, by its two ends: the part and knot it comes from (the output)
/// and the one it goes to (the input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WireRef {
    pub from: (PartId, u8),
    pub to: (PartId, u8),
}

/// A value on a knot or a wire, as a script sees it.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    /// Nothing: an unwired input, or an output that gives nothing.
    None,
    Bool(bool),
    Number(f64),
    /// A picture, by its sheet.
    Picture(u32),
    /// A sound: its rate in Hz and its length in samples.
    Sound {
        rate: u16,
        len: u32,
    },
    /// An object output with no object on it (a silent sound part).
    NoObject,
}

impl Val {
    /// The value a part's output gives.
    #[must_use]
    pub fn from_value(v: &crate::gadget::Value) -> Val {
        use crate::gadget::{Object, Value};
        match v {
            Value::None => Val::None,
            Value::Bool(b) => Val::Bool(*b),
            Value::Number(n) => Val::Number(*n),
            Value::Object(None) => Val::NoObject,
            Value::Object(Some(Object::Picture(p))) => Val::Picture(*p),
            Value::Object(Some(Object::Sound(s))) => Val::Sound {
                rate: s.rate,
                len: u32::try_from(s.samples.len()).unwrap_or(u32::MAX),
            },
        }
    }

    /// What kind of value it is, in words ("on/off", "a number").
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Val::None => "nothing",
            Val::Bool(_) => "on/off",
            Val::Number(_) => "a number",
            Val::Picture(_) => "a picture",
            Val::Sound { .. } => "a sound",
            Val::NoObject => "no object",
        }
    }

    /// As a test reads it: on, a number other than 0, or an object.
    #[must_use]
    pub fn truthy(&self) -> bool {
        match self {
            Val::None | Val::NoObject => false,
            Val::Bool(b) => *b,
            Val::Number(n) => *n != 0.0,
            Val::Picture(_) | Val::Sound { .. } => true,
        }
    }
}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&value::format(self))
    }
}

/// A knot of a part.
#[derive(Debug, Clone, PartialEq)]
pub struct KnotInfo {
    /// Its index among the part's knots.
    pub index: u8,
    pub input: bool,
    /// Where it is on the worksheet.
    pub at: (i32, i32),
    pub wire: Option<WireRef>,
    /// What it reads (an input) or gives (an output).
    pub value: Val,
}

/// A part, as a script sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct PartInfo {
    pub id: PartId,
    /// Its class's tag (`bulb`).
    pub tag: String,
    /// Its class's name without the `c` (`LightBulb`).
    pub kind: String,
    /// The name a script or the player gave it.
    pub name: Option<String>,
    /// The text of the Label next to it, or its own text.
    pub label: Option<String>,
    /// Its box on the worksheet: left, top, right, bottom.
    pub rect: (i32, i32, i32, i32),
    /// A puzzle's part, which can't be changed.
    pub locked: bool,
    pub broken: bool,
    pub knots: Vec<KnotInfo>,
}

impl PartInfo {
    /// Its kind in words: `Light Bulb`.
    #[must_use]
    pub fn kind_words(&self) -> String {
        select::words(&self.kind)
    }

    /// The part for messages: `Lamp (Light Bulb #7)`, or `Light Bulb #7`.
    #[must_use]
    pub fn describe(&self) -> String {
        let what = format!("{} {}", self.kind_words(), self.id);
        match self.name.as_ref().or(self.label.as_ref()) {
            Some(n) => format!("\"{n}\" ({what})"),
            None => what,
        }
    }

    /// How to refer to it in a script: its name, else its label, else its
    /// id.
    #[must_use]
    pub fn reference(&self) -> String {
        match self.name.as_ref().or(self.label.as_ref()) {
            Some(n) => value::quote(n),
            None => self.id.to_string(),
        }
    }

    /// Its middle on the worksheet.
    #[must_use]
    pub fn centre(&self) -> (i32, i32) {
        let (x0, y0, x1, y1) = self.rect;
        (i32::midpoint(x0, x1), i32::midpoint(y0, y1))
    }

    pub fn inputs(&self) -> impl Iterator<Item = &KnotInfo> {
        self.knots.iter().filter(|k| k.input)
    }

    pub fn outputs(&self) -> impl Iterator<Item = &KnotInfo> {
        self.knots.iter().filter(|k| !k.input)
    }
}

/// A kind of part the game has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindInfo {
    /// `LightBulb`.
    pub kind: String,
    /// `bulb`.
    pub tag: String,
}

/// Which part, and maybe which of its knots: `#7`, `Lamp`, `"Big lamp"`,
/// `LightBulb[2]`, `@120,80`, each with `.in`, `.in2`, `.out`, `.out2` or
/// `.3` (knot 3) after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub part: PartSel,
    pub knot: Option<KnotSel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartSel {
    /// `#7`: by id.
    Id(u32),
    /// A name, else a label, else a kind or tag; `[n]` picks the `n`th
    /// (from 1) of several in reading order.
    Text { text: String, nth: Option<u32> },
    /// `@x,y`: the topmost part at that point of the worksheet.
    At(i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnotSel {
    /// Knot `n` of the part, inputs and outputs counted together.
    Index(u8),
    /// The `n`th input (from 0).
    In(u8),
    /// The `n`th output (from 0).
    Out(u8),
}

/// One of a part's settings, as its dialog has them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKey {
    /// Check box `n` (0 to 31).
    Check(u8),
    /// Radio group `n`'s choice (0 to 3).
    Radio(u8),
    /// Number field `n` (0 to 3).
    Number(u8),
    /// Text `n` (0 to 3).
    Text(u8),
    /// The one long text of the parts that have one.
    LongText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingVal {
    Bool(bool),
    Int(u32),
    Text(String),
}

impl fmt::Display for SettingVal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingVal::Bool(b) => f.write_str(if *b { "on" } else { "off" }),
            SettingVal::Int(n) => write!(f, "{n}"),
            SettingVal::Text(t) => f.write_str(&value::quote(t)),
        }
    }
}

/// What a script may do. Scripts from elsewhere (saved in a widget, or
/// in a link) can't open other widgets.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    /// Look at the parts and values.
    pub read: bool,
    /// Run, stop, press, set values, give input.
    pub control: bool,
    /// Change settings and names.
    pub edit: bool,
    /// Open, load and start widgets and puzzles.
    pub document: bool,
}

impl Caps {
    /// Everything: the console, and scripts given on the command line.
    pub const ALL: Caps = Caps {
        read: true,
        control: true,
        edit: true,
        document: true,
    };
    /// A script that came with a widget.
    pub const EMBEDDED: Caps = Caps {
        read: true,
        control: true,
        edit: true,
        document: false,
    };
}

/// A part's own state, as the base class keeps it.
#[derive(Debug, Clone, PartialEq)]
pub struct PartState {
    pub state: bool,
    pub number: f64,
    pub object: Val,
    pub broken: bool,
    pub phase: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CmpOp {
    #[must_use]
    pub fn symbol(self) -> &'static str {
        match self {
            CmpOp::Eq => "==",
            CmpOp::Ne => "!=",
            CmpOp::Lt => "<",
            CmpOp::Le => "<=",
            CmpOp::Gt => ">",
            CmpOp::Ge => ">=",
        }
    }
}

/// Something to wait for or check.
#[derive(Debug, Clone, PartialEq)]
pub enum Cond {
    /// `Lamp == on`, `Counter.out >= 3`.
    Cmp(Selector, CmpOp, Val),
    /// `Lamp`: on, a number other than 0, or an object.
    Truthy(Selector),
    /// `Lamp changed`: different from when the wait began.
    Changed(Selector),
    /// The widget runs.
    Running,
    /// The widget doesn't run.
    Stopped,
    /// The last run was ended by a part with the puzzle solved.
    Solved,
    Not(Box<Cond>),
    All(Vec<Cond>),
    Any(Vec<Cond>),
}

/// How a [`Host::step`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepEnd {
    /// All the ticks ran.
    Ran(u32),
    /// A part ended the run at tick `at` of these.
    Stopped { solved: bool, at: u32 },
    /// The widget wasn't running.
    NotRunning,
}

/// The run's clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    /// Ticks (1/60 s) since GO.
    pub ticks: u32,
    pub ms: u64,
    pub running: bool,
}

/// The puzzle loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PuzzleInfo {
    pub number: i16,
    /// Still the puzzle (not saved as a widget since).
    pub active: bool,
    /// How the last run ended by a part ended: solved or not; `None` if
    /// none has.
    pub ended: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Info,
    Warn,
    Error,
}

/// A picture of the screen, the game's pointer on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shot {
    pub width: u32,
    pub height: u32,
    /// Red, green, blue, alpha a pixel, rows from the top.
    pub rgba: Vec<u8>,
}

impl Shot {
    /// As a PNG file.
    #[must_use]
    pub fn png(&self) -> Vec<u8> {
        formats::png::encode(&formats::png::Rgba {
            width: self.width,
            height: self.height,
            pixels: self.rgba.as_chunks::<4>().0.to_vec(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
}

/// Mouse input, in the editor's coordinates (the menu bar's top left is
/// 0, 0), as a window gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEv {
    Move(i32, i32),
    Down(i32, i32, Button),
    Up(i32, i32, Button),
}

/// A sound the widget or the editor asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEvent {
    Play {
        tick: u32,
        rate: u16,
        len: u32,
        looping: bool,
    },
    Stop {
        tick: u32,
    },
    StopAll {
        tick: u32,
    },
}

/// Why a host operation didn't happen. Nothing was changed.
#[derive(Debug, Clone, PartialEq)]
pub struct HostError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// No part by that id, name, label or kind; the closest ones.
    UnknownPart { query: String, suggestions: Vec<String> },
    /// Several parts answer to it; how to say each.
    Ambiguous { candidates: Vec<String> },
    /// A value of the wrong kind.
    WrongType,
    /// The part has no value of its own to set.
    NotSettable,
    /// No such knot, setting or wire.
    BadKnot,
    /// The arguments don't fit.
    BadArg,
    /// It needs the widget running (or stopped).
    NotRunning,
    /// A puzzle's part, which can't be changed.
    Locked,
    /// The script may not do that.
    Caps,
    /// A wait that didn't end in time.
    Timeout,
    /// A file or a widget that didn't open.
    Document,
    /// The script was stopped (from outside, or its time ran out).
    Stopped,
    /// This host can't do it (no debugging tools, no graph window).
    Unsupported,
}

impl ErrorKind {
    /// Its name, for scripts that catch it (`"UnknownPart"`).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            ErrorKind::UnknownPart { .. } => "UnknownPart",
            ErrorKind::Ambiguous { .. } => "Ambiguous",
            ErrorKind::WrongType => "WrongType",
            ErrorKind::NotSettable => "NotSettable",
            ErrorKind::BadKnot => "BadKnot",
            ErrorKind::BadArg => "BadArg",
            ErrorKind::NotRunning => "NotRunning",
            ErrorKind::Locked => "Locked",
            ErrorKind::Caps => "Caps",
            ErrorKind::Timeout => "Timeout",
            ErrorKind::Document => "Document",
            ErrorKind::Stopped => "Stopped",
            ErrorKind::Unsupported => "Unsupported",
        }
    }
}

impl HostError {
    #[must_use]
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> HostError {
        HostError {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HostError {}

pub type HostResult<T> = Result<T, HostError>;

/// What a host without `what` says.
fn unsupported(what: &str) -> HostError {
    HostError::new(ErrorKind::Unsupported, format!("{what} isn't there in this game"))
}

/// The editor as scripts drive it. Every operation checks everything
/// first and then does it whole, or does nothing and says why.
///
/// Coordinates: parts and knots are on the worksheet (its top left is
/// 0, 0); [`MouseEv`] is in the editor's (the menu bar's top left).
pub trait Host {
    // ---- discovery ---------------------------------------------------

    /// Every part, in reading order: top to bottom, then left to right.
    fn parts(&self) -> Vec<PartInfo>;

    /// Part `id`.
    ///
    /// # Errors
    /// No such part.
    fn part(&self, id: PartId) -> HostResult<PartInfo> {
        self.parts().into_iter().find(|p| p.id == id).ok_or_else(|| {
            HostError::new(
                ErrorKind::UnknownPart {
                    query: id.to_string(),
                    suggestions: Vec::new(),
                },
                format!("no part {id}"),
            )
        })
    }

    /// The part `sel` names: by id, then name, then label, then kind.
    ///
    /// # Errors
    /// None or several answer to it (with suggestions or how to say each).
    fn resolve(&self, sel: &PartSel) -> HostResult<PartId> {
        select::resolve(&self.parts(), sel).map(|p| p.id)
    }

    /// Every wire and what's on it.
    fn wires(&self) -> Vec<(WireRef, Val)>;

    /// What feeds input `knot` of part `id`: the part and knot at the
    /// wire's other end.
    ///
    /// # Errors
    /// No such part or input.
    fn source(&self, id: PartId, knot: u8) -> HostResult<Option<(PartId, u8)>>;

    /// Names part `id` (`None`: takes its name away).
    ///
    /// # Errors
    /// No such part, a name another part has, or one that can't be
    /// told from an id or a selector.
    fn set_name(&mut self, id: PartId, name: Option<&str>) -> HostResult<()>;

    /// Every kind of part the game has.
    fn kinds(&self) -> Vec<KindInfo> {
        crate::gadgets::CLASSES
            .iter()
            .map(|c| KindInfo {
                kind: c.name.trim_start_matches('c').to_owned(),
                tag: c.tag.to_string(),
            })
            .collect()
    }

    // ---- reading -----------------------------------------------------

    /// What output `knot` (the `n`th output; default the first) gives.
    ///
    /// # Errors
    /// No such part or output.
    fn output(&self, id: PartId, knot: Option<u8>) -> HostResult<Val>;

    /// What input `knot` (the `n`th input; default the first) reads.
    ///
    /// # Errors
    /// No such part or input.
    fn input(&self, id: PartId, knot: Option<u8>) -> HostResult<Val>;

    /// What's on wire `w`.
    ///
    /// # Errors
    /// No such wire.
    fn wire_value(&self, w: &WireRef) -> HostResult<Val>;

    /// Whether wire `w`'s value changed and its reader hasn't looked yet.
    ///
    /// # Errors
    /// No such wire.
    fn wire_changed(&self, w: &WireRef) -> HostResult<bool>;

    /// Part `id`'s own state.
    ///
    /// # Errors
    /// No such part.
    fn state(&self, id: PartId) -> HostResult<PartState>;

    /// One of part `id`'s settings.
    ///
    /// # Errors
    /// No such part or setting.
    fn setting(&self, id: PartId, key: SettingKey) -> HostResult<SettingVal>;

    /// The value `sel` stands for: a knot's, else the part's first
    /// output's, else (a part that only shows, as a Light Bulb) its first
    /// input's.
    ///
    /// # Errors
    /// No such part or knot.
    fn value(&self, sel: &Selector) -> HostResult<Val> {
        let id = self.resolve(&sel.part)?;
        let p = self.part(id)?;
        let knot = match sel.knot {
            Some(k) => select::knot(&p, k)?,
            None => match p.outputs().next().or_else(|| p.inputs().next()) {
                Some(k) => k,
                None => {
                    return Err(HostError::new(
                        ErrorKind::BadKnot,
                        format!("{} has no knots, so no value", p.describe()),
                    ));
                }
            },
        };
        Ok(knot.value.clone())
    }

    // ---- writing -----------------------------------------------------

    /// Sets part `id`'s own control, as the player would ([`adapt`]):
    /// while running a Switch is switched, a Slider's knob moved...; while
    /// editing, what it starts with is set (undoable, as its settings
    /// would). Returns the value it took (a Slider's knob only stops at
    /// so many places).
    ///
    /// # Errors
    /// No such part, a part without a control of its own, a locked part,
    /// or a value it can't take.
    fn set_value(&mut self, id: PartId, v: &Val) -> HostResult<Val>;

    /// Changes one of part `id`'s settings, as its dialog's OK would
    /// (undoable).
    ///
    /// # Errors
    /// No such part or setting, a locked part, a value of the wrong
    /// kind, or the widget running.
    fn set_setting(&mut self, id: PartId, key: SettingKey, v: &SettingVal) -> HostResult<()>;

    /// Writes `v` into part `id`'s own state (on/off, number or picture)
    /// and flags its outputs changed; the part may change it again on its
    /// next tick.
    ///
    /// # Errors
    /// No such part, a locked one, or a value it has no place for.
    fn poke(&mut self, id: PartId, v: &Val) -> HostResult<()>;

    // ---- input -------------------------------------------------------

    /// A click on running part `id`, at `at` in it (from its top left;
    /// default its middle), as the player's.
    ///
    /// # Errors
    /// No such part, or the widget not running.
    fn press(&mut self, id: PartId, at: Option<(i32, i32)>) -> HostResult<()>;

    /// The mouse, as a window gives it.
    fn mouse(&mut self, ev: MouseEv);

    /// A key (a Windows virtual-key code) down or up.
    fn key(&mut self, vk: u16, down: bool);

    // ---- time --------------------------------------------------------

    /// Lets `ms` milliseconds pass as a window's frames do (16 ms each,
    /// and what's left), then draws. For hosts without a window; in one,
    /// the game's own frames pass instead.
    ///
    /// # Errors
    /// The host stopped the script (its time ran out).
    fn wait(&mut self, ms: u64) -> HostResult<()>;

    /// GO.
    fn run(&mut self);

    /// STOP.
    fn stop(&mut self);

    /// Runs `ticks` ticks of the running widget at once.
    ///
    /// # Errors
    /// The host stopped the script (its time ran out).
    fn step(&mut self, ticks: u32) -> HostResult<StepEnd>;

    fn clock(&self) -> Clock;

    /// Seeds the parts' random numbers.
    fn seed(&mut self, rng: u64);

    // ---- debugging (a host with the debugging tools) ----------------

    /// Holds the running widget (`true`), or lets it go on.
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] where nothing can hold it.
    fn pause(&mut self, _on: bool) -> HostResult<()> {
        Err(unsupported("holding the run"))
    }

    /// Whether the run is held.
    fn paused(&self) -> bool {
        false
    }

    /// Runs the widget at `factor` of the real time (1 for full speed);
    /// the speed it took (the nearest one there is).
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] where there's no slow motion.
    fn set_speed(&mut self, _factor: f64) -> HostResult<f64> {
        Err(unsupported("slow motion"))
    }

    /// Shows the values `what` stands for in a graph.
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] where there are no graphs; a part or knot
    /// that isn't there.
    fn graph(&mut self, _what: &[Selector]) -> HostResult<()> {
        Err(unsupported("graphs"))
    }

    /// Keeps the value `what` stands for in sight.
    ///
    /// # Errors
    /// [`ErrorKind::Unsupported`] where nothing shows them; a part or knot
    /// that isn't there.
    fn watch(&mut self, _what: &Selector) -> HostResult<()> {
        Err(unsupported("watches"))
    }

    /// Whether `c` holds; `start` holds the values [`Cond::Changed`]
    /// compares with (from [`Host::snapshot`]).
    ///
    /// # Errors
    /// A part or knot that isn't there, or a comparison of values that
    /// can't be compared.
    fn eval(&self, c: &Cond, start: &[(Selector, Val)]) -> HostResult<bool> {
        Ok(match c {
            Cond::Cmp(sel, op, want) => value::compare(&self.value(sel)?, *op, want).map_err(|e| {
                let what = select::show(sel);
                HostError::new(ErrorKind::WrongType, format!("{what} {e}"))
            })?,
            Cond::Truthy(sel) => self.value(sel)?.truthy(),
            Cond::Changed(sel) => {
                let now = self.value(sel)?;
                start.iter().find(|(s, _)| s == sel).is_some_and(|(_, v)| *v != now)
            }
            Cond::Running => self.clock().running,
            Cond::Stopped => !self.clock().running,
            Cond::Solved => self.puzzle().and_then(|p| p.ended) == Some(true),
            Cond::Not(c) => !self.eval(c, start)?,
            Cond::All(cs) => {
                for c in cs {
                    if !self.eval(c, start)? {
                        return Ok(false);
                    }
                }
                true
            }
            Cond::Any(cs) => {
                for c in cs {
                    if self.eval(c, start)? {
                        return Ok(true);
                    }
                }
                false
            }
        })
    }

    /// The values `c`'s [`Cond::Changed`] parts compare with.
    ///
    /// # Errors
    /// A part or knot that isn't there.
    fn snapshot(&self, c: &Cond) -> HostResult<Vec<(Selector, Val)>> {
        let mut out = Vec::new();
        let mut todo = vec![c];
        while let Some(c) = todo.pop() {
            match c {
                Cond::Changed(sel) => out.push((sel.clone(), self.value(sel)?)),
                Cond::Not(c) => todo.push(c),
                Cond::All(cs) | Cond::Any(cs) => todo.extend(cs),
                _ => {}
            }
        }
        Ok(out)
    }

    /// Runs ticks until `c` holds, at most `max`; the ticks it took.
    ///
    /// # Errors
    /// [`ErrorKind::Timeout`] if it didn't, saying what the values are;
    /// [`ErrorKind::NotRunning`] if the widget isn't running (or stops)
    /// before.
    fn until(&mut self, c: &Cond, max: u32) -> HostResult<u32> {
        let start = self.snapshot(c)?;
        let mut n = 0;
        loop {
            if self.eval(c, &start)? {
                return Ok(n);
            }
            if n >= max {
                return Err(HostError::new(
                    ErrorKind::Timeout,
                    format!(
                        "{} didn't happen within {}{}",
                        value::show_cond(c),
                        value::ticks(max),
                        self.actual(c)
                    ),
                ));
            }
            match self.step(1)? {
                StepEnd::Ran(k) => n += k,
                StepEnd::Stopped { .. } => {
                    n += 1;
                    if self.eval(c, &start)? {
                        return Ok(n);
                    }
                    return Err(HostError::new(
                        ErrorKind::NotRunning,
                        format!(
                            "the run ended after {} before {} happened{}",
                            value::ticks(n),
                            value::show_cond(c),
                            self.actual(c)
                        ),
                    ));
                }
                StepEnd::NotRunning => {
                    return Err(HostError::new(
                        ErrorKind::NotRunning,
                        format!(
                            "waiting for {} needs the widget running (run first){}",
                            value::show_cond(c),
                            self.actual(c)
                        ),
                    ));
                }
            }
        }
    }

    /// What `c`'s parts are now, for a message: `; Lamp is off`.
    fn actual(&self, c: &Cond) -> String {
        let mut sels = Vec::new();
        let mut todo = vec![c];
        while let Some(c) = todo.pop() {
            match c {
                Cond::Cmp(s, ..) | Cond::Truthy(s) | Cond::Changed(s) => {
                    if !sels.contains(&s) {
                        sels.push(s);
                    }
                }
                Cond::Not(c) => todo.push(c),
                Cond::All(cs) | Cond::Any(cs) => todo.extend(cs.iter().rev()),
                Cond::Running | Cond::Stopped | Cond::Solved => {}
            }
        }
        let shown: Vec<String> = sels
            .iter()
            .filter_map(|s| Some(format!("{} is {}", select::show(s), self.value(s).ok()?)))
            .collect();
        if shown.is_empty() {
            String::new()
        } else {
            format!("; {}", shown.join(", "))
        }
    }

    // ---- documents ---------------------------------------------------

    /// A new, empty widget.
    fn new_widget(&mut self);

    /// Opens the widget file `path`, as dropped on the window.
    ///
    /// # Errors
    /// It can't be read, or the editor is busy (a box is open).
    fn open_file(&mut self, path: &str) -> HostResult<()>;

    /// Opens a widget from its bytes, `name` its file's name.
    ///
    /// # Errors
    /// The editor is busy (a box is open).
    fn open_widget(&mut self, name: &str, bytes: &[u8]) -> HostResult<()>;

    /// Loads puzzle `n`, as File ▸ Load Puzzle does.
    ///
    /// # Errors
    /// No such puzzle.
    fn load_puzzle(&mut self, n: i16) -> HostResult<()>;

    /// The puzzle loaded, if one is.
    fn puzzle(&self) -> Option<PuzzleInfo>;

    /// The widget as Save writes it.
    fn widget_bytes(&mut self) -> Vec<u8>;

    // ---- output ------------------------------------------------------

    fn log(&mut self, level: Level, msg: &str);

    /// The screen as it is now, drawn afresh.
    fn screenshot(&mut self) -> Shot;

    /// Takes a screenshot and hands it on under `name` (a file `NAME.png`
    /// where the host writes files).
    ///
    /// # Errors
    /// It couldn't be written.
    fn shot(&mut self, name: &str) -> HostResult<()>;

    /// A failed check: logged, and counted.
    fn fail(&mut self, msg: &str);

    /// The sounds asked for since the last call.
    fn sounds(&mut self) -> Vec<SoundEvent>;

    /// The parts in the editor's own order, as `tools/compare` reads
    /// them: a line each with the index, tag, box and knots, in the
    /// editor's coordinates.
    fn dump(&self) -> String;
}

/// Where a host's output goes: the log and the screenshots.
pub trait Sink {
    fn log(&mut self, level: Level, msg: &str);

    /// A screenshot a script took, named `name`.
    ///
    /// # Errors
    /// It couldn't be kept.
    fn shot(&mut self, name: &str, shot: &Shot) -> Result<(), String>;
}

/// The log and the screenshots kept in memory, the oldest dropped past
/// [`MemorySink::MAX_LINES`] lines (or the size) and
/// [`MemorySink::MAX_SHOTS`] shots.
#[derive(Debug, Default)]
pub struct MemorySink {
    pub lines: std::collections::VecDeque<(Level, String)>,
    bytes: usize,
    pub shots: std::collections::VecDeque<(String, Shot)>,
}

impl MemorySink {
    pub const MAX_LINES: usize = 2000;
    pub const MAX_BYTES: usize = 200 * 1024;
    pub const MAX_SHOTS: usize = 8;
}

impl Sink for MemorySink {
    fn log(&mut self, level: Level, msg: &str) {
        self.bytes += msg.len();
        self.lines.push_back((level, msg.to_owned()));
        while self.lines.len() > Self::MAX_LINES || (self.bytes > Self::MAX_BYTES && self.lines.len() > 1) {
            if let Some((_, l)) = self.lines.pop_front() {
                self.bytes -= l.len();
            }
        }
    }

    fn shot(&mut self, name: &str, shot: &Shot) -> Result<(), String> {
        self.shots.push_back((name.to_owned(), shot.clone()));
        if self.shots.len() > Self::MAX_SHOTS {
            self.shots.pop_front();
        }
        Ok(())
    }
}

/// `name` made safe as a file name: letters, digits, `-`, `_` and `.`
/// kept (no leading dot), anything else `_`; `shot` if nothing's left.
#[must_use]
pub fn file_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_start_matches('.');
    if s.is_empty() { "shot".to_owned() } else { s.to_owned() }
}

#[cfg(test)]
mod tests;
