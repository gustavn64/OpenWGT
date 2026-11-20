//! The editor's side of scripts and debugging: what it keeps for them
//! ([`Debugger`]) and the [`crate::script::Host`] it is to them
//! (`host.rs`). A child of `editor`, so it can reach the editor's own
//! fields; the hooks in the editor itself are single lines.

mod host;
#[cfg(test)]
mod tests;

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
        }
    }
}

impl Debugger {
    /// A part ended the run, the puzzle `solved` or not.
    pub(super) fn run_ended(&mut self, solved: bool) {
        self.ended = Some(solved);
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
