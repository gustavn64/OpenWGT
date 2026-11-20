//! What scripts can do, as a table: each command's name, arguments, what
//! it may need and what it does. The command language checks its lines
//! against it and `help` shows it; other languages' bindings (their
//! declarations, completion) are to come from it too.

use std::fmt::Write as _;

use super::Caps;

/// What a command needs a script to be allowed ([`Caps`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cap {
    Read,
    Control,
    Edit,
    Document,
}

impl Cap {
    /// Whether `caps` allow it.
    #[must_use]
    pub fn allowed(self, caps: Caps) -> bool {
        match self {
            Cap::Read => caps.read,
            Cap::Control => caps.control,
            Cap::Edit => caps.edit,
            Cap::Document => caps.document,
        }
    }
}

/// A kind of argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    /// A part: `#7`, `Lamp`, `"Big lamp"`, `LightBulb[2]`, `@120,80`.
    Part,
    /// A part or one of its knots: `Lamp`, `Gate.in2`, `Gate.out`.
    Knot,
    /// `on`, `off`, a number, `picture N`, `nothing`.
    Value,
    /// A whole number (`0x..` too).
    Int,
    /// A word or a quoted text.
    Text,
    /// A condition: `Lamp == on`, `Counter > 3`, `Lamp changed`,
    /// `stopped`, joined with `and`/`or`, `not` before one.
    Cond,
    /// A setting: `check0`..`check31`, `radio0`..`radio3`,
    /// `number0`..`number3`, `text0`..`text3`, `text`.
    Setting,
    /// The rest of the line.
    Rest,
}

/// A command of the built-in language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command {
    pub name: &'static str,
    /// Its arguments as written in [`Command::usage`].
    pub usage: &'static str,
    pub args: &'static [(Ty, bool)],
    pub cap: Cap,
    pub doc: &'static str,
    /// One of the comparison scripts' (`tools/compare`): input at pixel
    /// coordinates and time, which the original in the VM plays as well.
    pub input: bool,
}

const REQ: bool = false;
const OPT: bool = true;

macro_rules! commands {
    ($($name:literal $usage:literal [$($ty:ident $opt:ident),*] $cap:ident $input:literal $doc:literal;)*) => {
        /// Every command, in the order `help` lists them.
        pub static COMMANDS: &[Command] = &[$(Command {
            name: $name,
            usage: $usage,
            args: &[$((Ty::$ty, $opt)),*],
            cap: Cap::$cap,
            doc: $doc,
            input: $input,
        },)*];
    };
}

commands! {
    "size" "W H" [Int REQ, Int REQ] Read true "the editor's size, before the script starts (default 640 433)";
    "move" "X Y" [Int REQ, Int REQ] Control true "the mouse there, in steps as a hand moves it";
    "down" "X Y [right]" [Int REQ, Int REQ, Text OPT] Control true "a button pressed there";
    "up" "X Y [right]" [Int REQ, Int REQ, Text OPT] Control true "a button released there";
    "click" "X Y [right]" [Int REQ, Int REQ, Text OPT] Control true "pressed and released there";
    "dclick" "X Y" [Int REQ, Int REQ, Text OPT] Control true "a double click there";
    "drag" "X0 Y0 X1 Y1" [Int REQ, Int REQ, Int REQ, Int REQ, Text OPT] Control true "pressed at the first point, moved, released at the second";
    "key" "VK" [Int REQ] Control true "a key (a Windows virtual-key code) pressed and released";
    "wait" "MS" [Int REQ] Read true "time passes, MS milliseconds";
    "shot" "[NAME]" [Text OPT] Read true "a picture of the screen: NAME.png";
    "dump" "" [] Read true "the parts in the editor's order: tag, box and knots, in its coordinates";
    "open" "FILE" [Text REQ] Document true "opens a widget file, as dropped on the window";
    "puzzle" "N" [Int REQ] Document true "loads puzzle N, as File > Load Puzzle";
    "new" "" [] Document false "a new, empty widget";
    "run" "" [] Control false "GO";
    "stop" "" [] Control false "STOP";
    "step" "[N]" [Int OPT] Control false "N ticks (default 1) of the running widget, at once";
    "until" "COND [max N]" [Cond REQ] Control false "ticks until COND holds, at most N (default 600, 10 s); fails if it doesn't";
    "press" "PART [X Y]" [Part REQ, Int OPT, Int OPT] Control false "clicks the running part (at X Y in it; default its middle)";
    "set" "PART VALUE" [Part REQ, Value REQ] Control false "works the part's own control (a Switch, Slider, Constant, Keypad, Keypress); while editing, sets what it starts with";
    "poke" "PART VALUE" [Part REQ, Value REQ] Control false "writes the part's own on/off, number or picture; it may change it again";
    "setting" "PART KEY VALUE" [Part REQ, Setting REQ, Rest REQ] Edit false "changes one of the part's settings, as its dialog would";
    "name" "PART [NAME]" [Part REQ, Text OPT] Edit false "names the part (no NAME: takes its name away)";
    "seed" "N" [Int REQ] Control false "seeds the parts' random numbers";
    "expect" "COND" [Cond REQ] Read false "checks COND; a failure is counted and the script goes on";
    "assert" "COND" [Cond REQ] Read false "checks COND; a failure ends the script";
    "log" "TEXT" [Rest OPT] Read false "writes TEXT, {PART} and {PART.KNOT} replaced by their values";
    "print" "KNOT..." [Knot REQ, Rest OPT] Read false "writes the values: Lamp = on";
    "parts" "" [] Read false "lists the parts in reading order, with their values";
    "wires" "" [] Read false "lists the wires and what's on them";
    "help" "[COMMAND]" [Text OPT] Read false "lists the commands, or tells about one";
    "graph" "KNOT..." [Knot REQ, Rest OPT] Read false "records the values as the widget runs, and shows them in the graph (debugging tools)";
    "watch" "KNOT..." [Knot REQ, Rest OPT] Read false "keeps the values in the watch list (debugging tools)";
    "break" "COND" [Cond REQ] Read false "holds the run at the tick COND comes to hold; PART changed: at each change (debugging tools)";
    "pause" "" [] Control false "holds the run (debugging tools)";
    "resume" "" [] Control false "lets the held run go on (debugging tools)";
    "seek" "TICK" [Int REQ] Control false "goes back or forward to TICK of the run recorded since GO, held there (debugging tools)";
}

/// The command called `name`.
#[must_use]
pub fn command(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| c.name == name)
}

/// `name`'s usage line: `press PART [X Y]`.
#[must_use]
pub fn usage(c: &Command) -> String {
    if c.usage.is_empty() {
        c.name.to_owned()
    } else {
        format!("{} {}", c.name, c.usage)
    }
}

/// The help: every command, or one.
#[must_use]
pub fn help(name: Option<&str>) -> String {
    if let Some(c) = name.and_then(command) {
        format!("{} : {}", usage(c), c.doc)
    } else {
        let w = COMMANDS.iter().map(|c| usage(c).len()).max().unwrap_or(0);
        let mut s = String::new();
        for c in COMMANDS {
            let _ = writeln!(s, "{:w$}  {}", usage(c), c.doc);
        }
        s.push_str(
            "\nPARTS: #7 (its id), a name, a label's text, a kind (LightBulb, bulb), \
\"quoted text\", [2] for the second of several, @X,Y; .in .in2 .out .out2 or .N for a knot.\n\
VALUES: on, off, numbers, picture N, nothing.  CONDS: PART == VALUE (!= < <= > >=), \
PART changed, PART, running, stopped, solved; not, and, or.",
        );
        s
    }
}

/// The nearest command names to `name`, for "did you mean".
#[must_use]
pub fn near(name: &str) -> Vec<&'static str> {
    let mut v: Vec<(usize, &str)> = COMMANDS
        .iter()
        .map(|c| (super::select::distance(c.name, name), c.name))
        .filter(|&(d, _)| d <= (name.len() / 3).max(1))
        .collect();
    v.sort_unstable();
    v.into_iter().map(|(_, n)| n).take(3).collect()
}
