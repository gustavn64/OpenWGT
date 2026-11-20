//! The editor's side of scripts and debugging: what it keeps for them
//! ([`Debugger`]), the [`crate::script::Host`] it is to them (`host.rs`),
//! and the debugging tools: the Debug menu's run control (`control.rs`),
//! the pointer's note of values and the run's status (`overlay.rs`). A
//! child of `editor`, so it can reach the editor's own fields; the hooks
//! in the editor itself are single lines.
//!
//! The tools are there only with the enhancements on and their own
//! switch in Settings ("Debugging tools", off at first).

mod console;
#[cfg(test)]
mod console_tests;
mod context;
mod control;
mod graph;
mod host;
mod input;
mod overlay;
#[cfg(test)]
mod probe_tests;
mod probes;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod ui_tests;
mod watch;
mod window;

pub use console::{Line, LineKind};
pub use control::{Speed, is_command};

use std::collections::{BTreeMap, VecDeque};

use crate::assets::Assets;
use crate::lang::Lang;
use crate::script::{MemorySink, Sink, SoundEvent};

/// The most sound events kept for [`crate::script::Host::sounds`].
const MAX_SOUNDS: usize = 1000;

/// What the editor keeps for scripts and debugging.
pub struct Debugger {
    /// The names scripts gave parts, by id. (Kept in memory for now;
    /// they're to be saved with the widget.)
    names: BTreeMap<u32, String>,
    /// Where scripts' output goes; in memory unless the program says.
    pub sink: Box<dyn Sink>,
    sounds: VecDeque<SoundEvent>,
    /// Checks that failed.
    pub failures: u32,
    /// How the last run a part ended ended: solved or not.
    ended: Option<bool>,
    /// Builds the game's resources in another language, for Settings ▸
    /// Language while a script lets time pass ([`crate::script::Host::wait`]);
    /// without it the change waits for the program.
    pub assets_for: Option<Box<dyn FnMut(Lang) -> Option<Assets>>>,
    /// Whether a script has run out of time (the program's clock): its
    /// waits then end with an error.
    pub deadline: Option<Box<dyn Fn() -> bool>>,
    /// The wall clock the widget sees while a script lets time pass:
    /// hours, minutes, seconds.
    pub local_time: (u32, u32, u32),
    /// The debugging tools' switch (Settings), and its change for the
    /// host to keep.
    tools: bool,
    new_tools: Option<bool>,
    /// The run's control: held, slowed, helped.
    run: RunControl,
    /// Debug ▸ Stop Scripts, for the program running them.
    stop_scripts: bool,
    /// Whether the program has scripts running (Stop Scripts is greyed
    /// without).
    pub scripts_running: bool,
    hover: overlay::Hover,
    /// The tools' windows, the topmost last, and where the closed ones
    /// were.
    windows: Vec<window::Win>,
    closed: Vec<window::Win>,
    /// The window with the keyboard.
    focus: Option<window::Kind>,
    /// A window being dragged.
    drag: Option<window::Drag>,
    /// The windows' colours, from the game's palette.
    colours: Option<window::Colours>,
    /// The console: its log and line.
    pub console: console::Console,
    /// The languages it runs.
    runtimes: console::Runtimes,
    /// The popup menu (a right-click on the worksheet).
    popup: Option<context::Popup>,
    /// What the tools keep an eye on.
    pub observed: probes::Observed,
    /// The graph's view.
    graph: graph::Graph,
    /// Where the run is, for the windows.
    time: graph::Time,
}

/// How the run goes under the tools.
#[derive(Debug, Clone, Copy, Default)]
struct RunControl {
    /// Held (Pause).
    paused: bool,
    /// How fast it goes.
    speed: Speed,
    /// What's left of the milliseconds slowed down
    /// ([`Editor::debug_scale`](super::Editor)).
    carry: u64,
    /// A script changed this run's state: it doesn't solve the puzzle.
    assisted: bool,
}

impl Default for Debugger {
    fn default() -> Debugger {
        Debugger {
            names: BTreeMap::new(),
            sink: Box::new(MemorySink::default()),
            sounds: VecDeque::new(),
            failures: 0,
            ended: None,
            assets_for: None,
            deadline: None,
            local_time: (12, 0, 0),
            tools: false,
            new_tools: None,
            run: RunControl::default(),
            stop_scripts: false,
            scripts_running: false,
            hover: overlay::Hover::default(),
            windows: Vec::new(),
            closed: Vec::new(),
            focus: None,
            drag: None,
            colours: None,
            console: console::Console::default(),
            runtimes: console::Runtimes::default(),
            popup: None,
            observed: probes::Observed::default(),
            graph: graph::Graph::default(),
            time: graph::Time::default(),
        }
    }
}

impl Debugger {
    /// A part ended the run, the puzzle `solved` or not.
    pub(super) fn run_ended(&mut self, solved: bool) {
        self.ended = Some(solved);
    }

    /// GO: a run of the player's, at first; not held.
    pub(super) fn on_go(&mut self) {
        self.run.assisted = false;
        self.run.paused = false;
        self.run.carry = 0;
        self.observed.go();
        self.time = graph::Time::default();
    }

    /// Whether a script changed this run's state ([`Debugger::on_go`]): a
    /// win then doesn't mark the puzzle solved.
    #[must_use]
    pub fn assisted(&self) -> bool {
        self.run.assisted
    }

    /// Whether the run is held (Debug ▸ Pause).
    #[must_use]
    pub fn paused(&self) -> bool {
        self.run.paused
    }

    /// How fast the run goes (Debug ▸ Slow Motion).
    #[must_use]
    pub fn speed(&self) -> Speed {
        self.run.speed
    }

    /// Debug ▸ Stop Scripts was chosen, once: the program running scripts
    /// stops them.
    pub fn take_stop_scripts(&mut self) -> bool {
        std::mem::take(&mut self.stop_scripts)
    }

    fn heard(&mut self, e: SoundEvent) {
        self.sounds.push_back(e);
        if self.sounds.len() > MAX_SOUNDS {
            self.sounds.pop_front();
        }
    }

    fn timed_out(&self) -> bool {
        self.deadline.as_ref().is_some_and(|d| d())
    }

    /// The name of part `id`.
    #[must_use]
    pub fn name(&self, id: u32) -> Option<&str> {
        self.names.get(&id).map(String::as_str)
    }

    /// Forgets the names (another widget).
    fn forget(&mut self) {
        self.names.clear();
        self.ended = None;
    }
}
