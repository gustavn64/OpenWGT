//! A scripting language's side: what the editor asks of it. The built-in
//! command language ([`super::cmd`]) is one; others are given to the
//! editor by the program that has them.

use std::fmt;

use super::{Caps, Host, HostError};

/// What went wrong in a script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrKind {
    /// It doesn't read: nothing of it ran.
    Syntax,
    /// It failed while running.
    Runtime,
    /// The editor refused an operation ([`HostError`]).
    Host,
    /// It ran too long without waiting.
    Budget,
    /// It took too much memory.
    Memory,
    /// It was stopped.
    Cancelled,
    /// A check failed.
    Assert,
    /// A wait didn't end in time.
    Timeout,
}

/// An error at a place in a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    pub kind: ErrKind,
    pub message: String,
    pub file: String,
    /// From 1; 0 when it isn't known.
    pub line: u32,
    /// From 1, in characters; 0 when it isn't known.
    pub column: u32,
    /// How many characters to underline from `column`.
    pub width: u32,
    pub source_line: Option<String>,
    /// The calls it happened in, innermost first: function, line, column.
    pub stack: Vec<(String, u32, u32)>,
}

impl ScriptError {
    /// An error at line `line` of `file`, column `column`, `width`
    /// characters long, in `source`.
    #[must_use]
    pub fn at(
        kind: ErrKind,
        message: impl Into<String>,
        file: &str,
        line: u32,
        column: u32,
        width: u32,
        source: Option<&str>,
    ) -> ScriptError {
        ScriptError {
            kind,
            message: message.into(),
            file: file.to_owned(),
            line,
            column,
            width,
            source_line: source.map(str::to_owned),
            stack: Vec::new(),
        }
    }

    /// The editor's refusal, at that place.
    #[must_use]
    pub fn host(e: &HostError, file: &str, line: u32, column: u32, width: u32, source: Option<&str>) -> ScriptError {
        let kind = match e.kind {
            super::ErrorKind::Timeout => ErrKind::Timeout,
            super::ErrorKind::Stopped => ErrKind::Cancelled,
            _ => ErrKind::Host,
        };
        ScriptError::at(kind, e.message.clone(), file, line, column, width, source)
    }
}

/// `file:7:19  message`, then the line and a caret under the place:
///
/// ```text
/// check.txt:7:7  no part called "Lamb". Did you mean "Lamp" (Light Bulb #7)?
///    7 | press Lamb
///      |       ^^^^
/// ```
impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (0, _) => write!(f, "{}  {}", self.file, self.message)?,
            (l, 0) => write!(f, "{}:{l}  {}", self.file, self.message)?,
            (l, c) => write!(f, "{}:{l}:{c}  {}", self.file, self.message)?,
        }
        if let Some(src) = &self.source_line
            && self.line > 0
        {
            let n = self.line.to_string();
            let pad = " ".repeat(n.len());
            write!(f, "\n {n} | {src}")?;
            if self.column > 0 {
                let before: String = src
                    .chars()
                    .take(usize::try_from(self.column - 1).unwrap_or(0))
                    .map(|c| if c == '\t' { '\t' } else { ' ' })
                    .collect();
                let carets = "^".repeat(usize::try_from(self.width.max(1)).unwrap_or(1));
                write!(f, "\n {pad} | {before}{carets}")?;
            }
        }
        for (func, l, c) in &self.stack {
            write!(f, "\n    at {func} ({}:{l}:{c})", self.file)?;
        }
        Ok(())
    }
}

impl std::error::Error for ScriptError {}

/// A script started with [`Runtime::start`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Runs on at the next frame or tick.
    Running,
    /// Waiting for time to pass.
    Sleeping { ms: u64 },
    /// Ran to its end.
    Done,
    /// Ended by an error (or a failed `assert`), or stopped.
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInfo {
    pub id: TaskId,
    pub file: String,
    pub state: TaskState,
    /// The line it's at (from 1), where that's known.
    pub line: u32,
    /// Checks that failed so far.
    pub failures: u32,
}

/// What a script may use before it has to wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Steps of work a script may do between two waits (for a language
    /// with loops: its instructions, roughly).
    pub budget: u64,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits { budget: 1_000_000 }
    }
}

/// What a task does next, for a host that drives it itself
/// ([`Runtime::resume`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    /// It waits for `ms` of time to pass.
    Sleep(u64),
    /// It's done (or failed).
    Done,
}

/// A scripting language, as the editor drives it.
pub trait Runtime {
    /// The language's name: `commands`, `JavaScript`.
    fn language(&self) -> &'static str;

    /// Whether `src` reads, without running any of it.
    ///
    /// # Errors
    /// The first place it doesn't.
    fn check(&mut self, file: &str, src: &str) -> Result<(), ScriptError>;

    /// Starts `src` as a task; it runs from the next [`Runtime::frame`]
    /// (or [`Runtime::resume`]).
    ///
    /// # Errors
    /// It doesn't read.
    fn start(&mut self, file: &str, src: &str, caps: Caps, host: &mut dyn Host) -> Result<TaskId, ScriptError>;

    /// Runs one line typed in a console at once; what it gives back to
    /// show, if anything.
    ///
    /// # Errors
    /// It doesn't read, or fails.
    fn repl(&mut self, line: &str, host: &mut dyn Host) -> Result<Option<String>, ScriptError>;

    /// A frame of `ms` milliseconds passed: the tasks run on as far as
    /// they can.
    fn frame(&mut self, host: &mut dyn Host, ms: u64);

    /// The widget ticked: waits for ticks and conditions end.
    fn after_tick(&mut self, host: &mut dyn Host);

    /// Runs `task` on until it has to wait, for a host without frames of
    /// its own (headless), which then lets that time pass and resumes.
    fn resume(&mut self, task: TaskId, host: &mut dyn Host) -> Poll;

    /// Stops `task`, or every task.
    fn cancel(&mut self, task: Option<TaskId>);

    fn tasks(&self) -> Vec<TaskInfo>;

    /// The errors (and failed checks) of `task` so far.
    fn errors(&self, task: TaskId) -> Vec<ScriptError>;

    /// What could come at `cursor` in `line`: the commands that start so.
    fn complete(&self, line: &str, cursor: usize, _host: &dyn Host) -> Vec<String> {
        let head = &line[..cursor.min(line.len())];
        if head.contains(char::is_whitespace) {
            return Vec::new();
        }
        super::api::COMMANDS
            .iter()
            .filter(|c| c.name.starts_with(head))
            .map(|c| c.name.to_owned())
            .collect()
    }

    fn set_limits(&mut self, limits: Limits);
}

/// The result of a script run to its end ([`run_headless`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Failed checks and errors, in order.
    pub errors: Vec<ScriptError>,
    /// How many checks failed.
    pub failures: u32,
    /// Whether it ran to its end (no error, no failed `assert`).
    pub completed: bool,
}

impl Report {
    /// Whether it all went well.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.completed && self.failures == 0 && self.errors.is_empty()
    }
}

/// Runs `src` to its end in a host without frames of its own, letting
/// time pass ([`Host::wait`]) whenever it waits.
///
/// # Errors
/// It doesn't read.
pub fn run_headless(
    rt: &mut dyn Runtime,
    file: &str,
    src: &str,
    caps: Caps,
    host: &mut dyn Host,
) -> Result<Report, ScriptError> {
    let task = rt.start(file, src, caps, host)?;
    while let Poll::Sleep(ms) = rt.resume(task, host) {
        if let Err(e) = host.wait(ms) {
            rt.cancel(Some(task));
            let mut report = report(rt, task);
            report.errors.push(ScriptError::host(&e, file, 0, 0, 0, None));
            report.completed = false;
            return Ok(report);
        }
    }
    Ok(report(rt, task))
}

fn report(rt: &dyn Runtime, task: TaskId) -> Report {
    let info = rt.tasks().into_iter().find(|t| t.id == task);
    Report {
        errors: rt.errors(task),
        failures: info.as_ref().map_or(0, |t| t.failures),
        completed: info.is_some_and(|t| t.state == TaskState::Done),
    }
}
