//! Scripted parts (`openparts::scripted`): a part whose behaviour is a
//! script's, `definePart({name, inputs, outputs, settings, start,
//! tick})`. A language runs them through [`PartRuntime`], which the
//! running widget reaches through [`crate::sim::Env::parts`] ([`PartHook`]),
//! so a part's tick calls its script at once, in the widget's tick
//! order. Each call has a budget of its own; a script that runs over it,
//! or throws, breaks its part (the game's broken state), not the game.
//!
//! A part's script runs only once the player allowed it: its text's
//! digest is in [`PartScripts::allowed`] (the editor puts there what the
//! player trusts or lets run, `editor::debug::consent`), or it is the
//! port's own [`DEFAULT_CODE`]. Until then the part is inert.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;
use std::sync::LazyLock;

use super::runtime::ScriptError;
use super::trust::Digest;
use super::{Level, Val};

/// The most inputs, and outputs, a scripted part has.
pub const MAX_KNOTS: usize = 4;

/// What a new scripted part does: its input on to its output.
pub const DEFAULT_CODE: &str = "\
// A scripted part. Give it a script of yours: with the debugging tools
// on, select it and drop a .js file that calls definePart onto the window.
definePart({
  name: \"Script\",
  inputs: [\"in\"],
  outputs: [\"out\"],
  tick(s, { in: x }) {
    return { out: x };
  },
});
";

static DEFAULT_DIGEST: LazyLock<Digest> = LazyLock::new(|| Digest::of(DEFAULT_CODE));

/// What a part's script says it is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartDef {
    /// Shown on the part.
    pub name: String,
    /// Its inputs' and outputs' names, at most [`MAX_KNOTS`] each.
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

impl PartDef {
    /// [`DEFAULT_CODE`]'s.
    #[must_use]
    pub fn default_part() -> PartDef {
        PartDef {
            name: "Script".into(),
            inputs: vec!["in".into()],
            outputs: vec!["out".into()],
        }
    }
}

/// A language's side of scripted parts. Parts are told apart by their ids
/// in the widget; each part's script runs in a world of its own.
pub trait PartRuntime {
    /// The language's name (`JavaScript`).
    fn language(&self) -> &'static str;

    /// Runs `code` (which calls `definePart` once) for part `part`, anew:
    /// what it defines.
    ///
    /// # Errors
    /// It doesn't read, throws, runs too long, or defines nothing usable.
    fn define(&mut self, part: u32, code: &str) -> Result<PartDef, ScriptError>;

    /// The widget starts running: part `part`'s `start`, with a new state
    /// (its script run anew first if it isn't `code`).
    ///
    /// # Errors
    /// As [`PartRuntime::define`], or its `start` fails.
    fn start(&mut self, part: u32, code: &str) -> Result<(), ScriptError>;

    /// A tick (run tick `tick`): part `part`'s `tick` with its inputs'
    /// values; for each output, what it gives from now on (`None`: what
    /// it gave before).
    ///
    /// # Errors
    /// Its `tick` throws, runs too long or gives what an output can't.
    fn tick(&mut self, part: u32, code: &str, tick: u32, inputs: &[Val]) -> Result<Vec<Option<Val>>, ScriptError>;

    /// Forgets every part's (another widget came).
    fn clear(&mut self);

    /// What the parts' scripts said (`console.log`) since the last call.
    fn take_log(&mut self) -> Vec<(Level, String)>;
}

/// What the scripted parts share: the language, what may run, and what
/// they had to say.
pub struct PartScripts {
    pub runtime: Option<Box<dyn PartRuntime>>,
    /// The texts the player allowed to run.
    pub allowed: BTreeSet<Digest>,
    /// The parts' errors, for the editor to show.
    pub log: Vec<(Level, String)>,
}

/// The running widget's way to its scripted parts' language (shared by
/// every copy of the [`crate::sim::Env`]); none without one.
#[derive(Clone, Default)]
pub struct PartHook(Option<Rc<RefCell<PartScripts>>>);

impl fmt::Debug for PartHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0.is_some() {
            "PartHook(..)"
        } else {
            "PartHook(none)"
        })
    }
}

impl PartHook {
    /// A hook on `runtime` (none: the parts can't run).
    #[must_use]
    pub fn new(runtime: Option<Box<dyn PartRuntime>>) -> PartHook {
        PartHook(Some(Rc::new(RefCell::new(PartScripts {
            runtime,
            allowed: BTreeSet::new(),
            log: Vec::new(),
        }))))
    }

    /// Whether a script with the text of `d` may run (and can: there is a
    /// language).
    #[must_use]
    pub fn allowed(&self, d: &Digest) -> bool {
        self.0.as_ref().is_some_and(|p| {
            p.try_borrow()
                .is_ok_and(|p| p.runtime.is_some() && (p.allowed.contains(d) || *d == *DEFAULT_DIGEST))
        })
    }

    /// Lets the script with the text of `d` run.
    pub fn allow(&self, d: Digest) {
        self.with(|p| p.allowed.insert(d));
    }

    /// Whether there's a language for the parts.
    #[must_use]
    pub fn has_runtime(&self) -> bool {
        self.0
            .as_ref()
            .is_some_and(|p| p.try_borrow().is_ok_and(|p| p.runtime.is_some()))
    }

    /// `f` on what the parts share, if there's a hook (and it isn't in use).
    pub fn with<R>(&self, f: impl FnOnce(&mut PartScripts) -> R) -> Option<R> {
        let p = self.0.as_ref()?;
        let mut p = p.try_borrow_mut().ok()?;
        Some(f(&mut p))
    }

    /// `f` on the language, if there is one.
    pub fn runtime<R>(&self, f: impl FnOnce(&mut dyn PartRuntime) -> R) -> Option<R> {
        self.with(|p| p.runtime.as_mut().map(|rt| f(rt.as_mut()))).flatten()
    }
}
