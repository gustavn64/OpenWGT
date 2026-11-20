//! What JavaScript scripts can use, as a table: the declarations of
//! `openwgt-script.d.ts` (generated from it, and checked by a test),
//! `help()` and completion in a console come from here.

use std::fmt::Write as _;

/// Where a name is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Of {
    /// A global.
    Global,
    /// A [`Of::Part`]'s member (`part("Lamp").on`).
    Part,
    /// What `wire()` gives.
    Wire,
    Clock,
    Mouse,
    Console,
}

impl Of {
    /// The interface it's a member of in the declarations.
    #[must_use]
    pub fn interface(self) -> Option<&'static str> {
        match self {
            Of::Global => None,
            Of::Part => Some("Part"),
            Of::Wire => Some("Wire"),
            Of::Clock => Some("Clock"),
            Of::Mouse => Some("Mouse"),
            Of::Console => Some("Console"),
        }
    }
}

/// A function, constant or member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub of: Of,
    pub name: &'static str,
    /// Its TypeScript declaration (a global's without `declare`).
    pub decl: &'static str,
    pub doc: &'static str,
}

macro_rules! items {
    ($($of:ident $name:literal $decl:literal $doc:literal;)*) => {
        /// Everything, in the order the declarations and `help()` list it.
        pub static ITEMS: &[Item] = &[$(Item { of: Of::$of, name: $name, decl: $decl, doc: $doc },)*];
    };
}

items! {
    Global "part" "function part(query: string | number | Part): Part"
        "The part a name, a label, a kind (\"LightBulb\", \"bulb\"), \"Switch[2]\", \"#7\" (its id) or \"@120,80\" says. Throws a WidgetError (with suggestions) if none or several answer.";
    Global "parts" "function parts(query?: string): Part[]"
        "Every part, or those with that name, label or kind, in reading order (top to bottom, then left to right).";
    Global "wire" "function wire(from: string | Part, to?: string | Part): Wire"
        "The wire from a part (or its knot: \"Gate.out\") to another (\"Lamp.in\").";
    Global "run" "function run(): void" "GO.";
    Global "stop" "function stop(): void" "STOP.";
    Global "pause" "function pause(): void" "Holds the run (needs the debugging tools; elsewhere it does nothing).";
    Global "resume" "function resume(): void" "Lets a held run go on.";
    Global "speed" "function speed(factor: number): number"
        "Slow motion: 1 is full speed, 1/2, 1/4, 1/10 or 1/60 of it; gives the speed it took (needs the debugging tools).";
    Global "step" "function step(ticks?: number): { ticks: number; stopped: boolean; solved: boolean }"
        "Runs that many ticks (default 1) of the running widget at once, without waiting.";
    Global "ticks" "function ticks(n: number): Promise<void>" "Waits n ticks (1/60 s each) of the run: await ticks(2).";
    Global "seconds" "function seconds(s: number): Promise<void>" "Waits s seconds of the run.";
    Global "until" "function until(cond: () => unknown, opts?: { max?: number }): Promise<number>"
        "Waits until cond() is true, checked after each tick, for at most max ticks (default 600); gives the ticks it took, or throws a WidgetError (Timeout).";
    Global "key" "function key(k: string | number, opts?: { hold?: number }): void"
        "Presses a key (\"A\", \"Enter\", \"Left\" or a Windows key code) for hold ticks (default 5).";
    Global "type" "function type(text: string): void" "Types the letters, digits and spaces of text, a key after another.";
    Global "mouse" "const mouse: Mouse" "The mouse, in the window's coordinates.";
    Global "clock" "const clock: Clock" "The run's clock.";
    Global "onChange" "function onChange(what: string | Part, f: (value: Value, tick: number) => void): () => void"
        "Calls f whenever what's value changes (checked after each tick); gives a function that stops it.";
    Global "graph" "function graph(...what: (string | Part)[]): void" "Shows the values in a graph window (with the debugging tools).";
    Global "watch" "function watch(what: string | Part): void" "Keeps the value in sight (with the debugging tools).";
    Global "log" "function log(...values: unknown[]): void" "Writes the values to the log.";
    Global "console" "const console: Console" "console.log, warn and error, into the log.";
    Global "assert" "function assert(cond: unknown, message?: string, opts?: { fatal?: boolean }): void"
        "A check: if cond is false, a failure is counted and logged where it is; with fatal the script stops too.";
    Global "assertEqual" "function assertEqual<T>(actual: T, expected: T, message?: string, opts?: { fatal?: boolean }): void"
        "A check that two values are the same (arrays and objects by their contents).";
    Global "test" "function test(name: string, f: () => void | Promise<void>): void"
        "A test, run after the script with the others, one after another: \u{2713} or \u{2717} and the ticks it took.";
    Global "screenshot" "function screenshot(name?: string): void" "A picture of the screen: NAME.png where the program keeps them.";
    Global "puzzle" "function puzzle(): { number: number; active: boolean; ended: boolean; solved: boolean } | null"
        "The puzzle loaded, and whether a run of it ended (solved or not).";
    Global "open" "function open(path: string): void" "Opens a widget file (scripts from the command line or the console only).";
    Global "loadPuzzle" "function loadPuzzle(n: number): void" "Loads puzzle n (command line or console only).";
    Global "newWidget" "function newWidget(): void" "A new, empty widget (command line or console only).";
    Global "seed" "function seed(n: number): void" "Seeds the parts' random numbers and Math.random.";
    Global "help" "function help(name?: string): string" "What there is, or what one name does.";

    Part "id" "readonly id: number" "Its id, which stays as parts move or others go.";
    Part "kind" "readonly kind: string" "Its kind: \"LightBulb\".";
    Part "tag" "readonly tag: string" "Its kind's tag: \"bulb\".";
    Part "name" "name: string | null" "The name it was given (set it to name it).";
    Part "label" "readonly label: string | null" "The text of the Label next to it, or its own text.";
    Part "on" "readonly on: boolean" "Whether its value is on (a number other than 0, or an object).";
    Part "value" "value: Value" "Its value: its first output's, or for a part that only shows (a Light Bulb) its input's. Setting it works the part's own control (a Switch, Slider, Constant...).";
    Part "broken" "readonly broken: boolean" "Whether it's broken.";
    Part "x" "readonly x: number" "Its left on the worksheet.";
    Part "y" "readonly y: number" "Its top on the worksheet.";
    Part "width" "readonly width: number" "Its width.";
    Part "height" "readonly height: number" "Its height.";
    Part "output" "output(knot?: number | string): Value" "What output knot (the nth output from 0, or \"out2\") gives.";
    Part "input" "input(knot?: number | string): Value" "What input knot (the nth input from 0, or \"in2\") reads.";
    Part "source" "source(knot?: number): Part | null" "The part wired into input knot (default the first).";
    Part "press" "press(x?: number, y?: number): void" "Clicks the running part (at x, y in it; default its middle).";
    Part "setting" "setting(key: string): boolean | number | string"
        "One of its settings: \"check0\"..\"check31\", \"radio0\"..\"radio3\", \"number0\"..\"number3\", \"text0\"..\"text3\", \"text\".";
    Part "setSetting" "setSetting(key: string, value: boolean | number | string): void" "Changes one of its settings, as its dialog would.";

    Wire "value" "readonly value: Value" "What's on it.";
    Wire "changed" "readonly changed: boolean" "Whether it changed and its reader hasn't looked yet.";

    Clock "ticks" "readonly ticks: number" "Ticks (1/60 s) since GO.";
    Clock "seconds" "readonly seconds: number" "Seconds since GO.";
    Clock "running" "readonly running: boolean" "Whether the widget runs.";
    Clock "paused" "readonly paused: boolean" "Whether the run is held.";

    Mouse "move" "move(x: number, y: number): void" "Moves the mouse there.";
    Mouse "down" "down(button?: \"left\" | \"right\"): void" "Presses a button where the mouse is.";
    Mouse "up" "up(button?: \"left\" | \"right\"): void" "Releases it.";

    Console "log" "log(...values: unknown[]): void" "Writes to the log.";
    Console "info" "info(...values: unknown[]): void" "Writes to the log.";
    Console "debug" "debug(...values: unknown[]): void" "Writes to the log.";
    Console "warn" "warn(...values: unknown[]): void" "Writes a warning.";
    Console "error" "error(...values: unknown[]): void" "Writes an error.";
}

/// The declarations for editors (`openwgt-script.d.ts`): completion and,
/// with `// @ts-check`, checking in plain `.js` files.
#[must_use]
pub fn declarations() -> String {
    let mut s = String::from(
        "// What OpenWGT's scripts can use. Generated from crates/script-js/src/api.rs\n\
         // (cargo test -p script-js with UPDATE_DTS=1 writes it again); don't edit.\n\n\
         /** On/off, a number, a picture, a sound, or nothing. */\n\
         type Value = boolean | number | Picture | Sound | null;\n\n\
         interface Picture {\n  readonly kind: \"picture\";\n  readonly sheet: number;\n}\n\n\
         interface Sound {\n  readonly kind: \"sound\";\n  /** Samples a second. */\n  readonly rate: number;\n  /** Samples. */\n  readonly length: number;\n}\n\n",
    );
    for of in [Of::Part, Of::Wire, Of::Clock, Of::Mouse, Of::Console] {
        let _ = writeln!(s, "interface {} {{", of.interface().unwrap_or_default());
        for i in ITEMS.iter().filter(|i| i.of == of) {
            let _ = writeln!(s, "  /** {} */\n  {};", i.doc, i.decl);
        }
        s.push_str("}\n\n");
    }
    s.push_str(
        "/** An operation the game refused. */\n\
         declare class WidgetError extends Error {\n  \
         /** \"UnknownPart\", \"Ambiguous\", \"WrongType\", \"NotSettable\", \"BadKnot\", \"BadArg\", \"NotRunning\", \"Locked\", \"Caps\", \"Timeout\", \"Document\", \"Stopped\", \"Unsupported\" */\n  \
         readonly code: string;\n  \
         /** The part it was about, or what was asked for. */\n  \
         readonly part: Part | string | null;\n  \
         /** The parts meant, maybe. */\n  \
         readonly suggestions: string[];\n}\n\n\
         /** A failed check with { fatal: true }. */\n\
         declare class AssertionError extends Error {}\n",
    );
    for i in ITEMS.iter().filter(|i| i.of == Of::Global) {
        let _ = write!(s, "\n/** {} */\ndeclare {};\n", i.doc, i.decl);
    }
    s
}

/// `help()`: the globals, or what `name` is.
#[must_use]
pub fn help(name: Option<&str>) -> String {
    if let Some(n) = name {
        let found: Vec<String> = ITEMS
            .iter()
            .filter(|i| i.name == n)
            .map(|i| match i.of.interface() {
                Some(of) => format!("{of}.{}: {}", i.decl, i.doc),
                None => format!("{}: {}", i.decl, i.doc),
            })
            .collect();
        if !found.is_empty() {
            return found.join("\n");
        }
    }
    let mut s = String::new();
    for i in ITEMS.iter().filter(|i| i.of == Of::Global) {
        let _ = writeln!(s, "{}", i.decl);
    }
    s.push_str("Part: ");
    let members: Vec<&str> = ITEMS.iter().filter(|i| i.of == Of::Part).map(|i| i.name).collect();
    s.push_str(&members.join(", "));
    s.push_str("\nhelp(\"name\") tells more.");
    s
}

/// Where a console line's cursor is, for completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Context {
    /// In a name: the start of the word.
    Global(String),
    /// After `something.`: what's before the dot and the word after it.
    Member(Of, String),
    /// In the quoted text given to `part(`, `parts(`, `wire(`...: what's
    /// typed of it.
    Part(String),
}

/// What's being typed at `cursor` (in bytes) in `line`.
#[must_use]
pub fn context(line: &str, cursor: usize) -> Context {
    let mut c = cursor.min(line.len());
    while !line.is_char_boundary(c) {
        c -= 1;
    }
    let head = &line[..c];
    // An open quote: inside a string.
    let mut quote = None;
    for (i, ch) in head.char_indices() {
        match quote {
            Some((q, _)) if ch == q => quote = None,
            None if matches!(ch, '"' | '\'' | '`') => quote = Some((ch, i)),
            _ => {}
        }
    }
    if let Some((_, at)) = quote {
        let before = head[..at].trim_end();
        let call = before
            .strip_suffix('(')
            .or_else(|| before.strip_suffix(','))
            .unwrap_or("");
        let call = call.trim_end();
        let name: String = call
            .chars()
            .rev()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if matches!(
            name.as_str(),
            "part" | "parts" | "wire" | "onChange" | "graph" | "watch"
        ) || before.ends_with(',')
        {
            return Context::Part(head[at + 1..].to_owned());
        }
        return Context::Global(String::new());
    }
    let word_len = head
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
        .map(char::len_utf8)
        .sum::<usize>();
    let word = head[head.len() - word_len..].to_owned();
    let before = &head[..head.len() - word_len];
    let Some(obj) = before.strip_suffix('.') else {
        return Context::Global(word);
    };
    // What the dot follows: a name, or a call (`part("x").`).
    let obj = obj.trim_end();
    let callee = if let Some(inner) = obj.strip_suffix(')') {
        let mut depth = 1;
        let mut start = inner.len();
        for (i, ch) in inner.char_indices().rev() {
            match ch {
                ')' => depth += 1,
                '(' => {
                    depth -= 1;
                    if depth == 0 {
                        start = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        &inner[..start]
    } else {
        obj
    };
    let name: String = callee
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let of = match name.as_str() {
        "clock" => Of::Clock,
        "mouse" => Of::Mouse,
        "console" => Of::Console,
        "wire" => Of::Wire,
        _ => Of::Part,
    };
    Context::Member(of, word)
}

/// The names in `of` that start with `word`.
#[must_use]
pub fn names(of: Of, word: &str) -> Vec<String> {
    ITEMS
        .iter()
        .filter(|i| i.of == of && i.name.starts_with(word))
        .map(|i| i.name.to_owned())
        .collect()
}
