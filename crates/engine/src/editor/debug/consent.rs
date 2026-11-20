//! Asking before a widget's scripts run. A widget opened (File ▸ Open, a
//! file dropped onto the window, a link, an imported stand-alone) that
//! comes with scripts asks, in the game's own message box, "This widget
//! comes with scripts." with three answers:
//!
//! - **Show** (the first, what Enter does): the console opens with the
//!   scripts listed (`scripts`), none of them run; each runs only when
//!   its ▶ is pressed. The debugging tools are turned on for the session
//!   for it if they were off (not in the settings: they are off again
//!   next time).
//! - **Run**: they run, each as a script that came with a widget
//!   ([`Caps::EMBEDDED`]: never opening or loading other widgets), and
//!   the player's trust in them is remembered by their texts' SHA-256
//!   ([`crate::script::trust`]) where the program keeps the player's
//!   settings.
//! - **Don't run** (Esc): nothing runs.
//!
//! A widget whose every script is trusted doesn't ask: they run. A
//! script whose text changed is another script and asks again. With the
//! port's enhancements off nothing asks and only trusted scripts run. A
//! widget a script opens (with the console's or the command line's
//! rights) doesn't ask either, and its scripts don't run: the script is
//! in charge.
//!
//! The scripts that came with the widget run on with the debugging tools
//! off (they then say nothing anywhere), until another widget comes.

use super::super::{Answer, Editor, Then};
use super::LineKind;
use super::window::Kind;
use crate::lang::tr;
use crate::script::Caps;
use crate::script::trust::Digest;

impl Editor {
    /// The texts of everything of the widget's that needs the player's
    /// consent to run.
    pub(super) fn consent_digests(&self) -> Vec<Digest> {
        self.doc
            .scripts
            .iter()
            .map(crate::port_parts::ext::Script::digest)
            .collect()
    }

    /// A widget just opened: its scripts run if the player trusts them
    /// all, else (with the enhancements on) the question. The editor
    /// asks for the widgets it opens; the program for one it opened
    /// itself, at the start.
    pub fn ask_consent(&mut self) {
        let digests = self.consent_digests();
        if digests.is_empty() {
            return;
        }
        if digests.iter().all(|d| self.debug.trust.trusted(d)) {
            self.run_widget_scripts();
            return;
        }
        if !self.tray.enhanced || self.prompt.is_some() {
            return;
        }
        self.message(
            tr("This widget comes with scripts."),
            &[Answer::Show, Answer::Run, Answer::DontRun],
            Then::Scripts,
        );
    }

    /// The answer to "This widget comes with scripts.".
    pub(in crate::editor) fn consent_answer(&mut self, a: Answer) {
        match a {
            Answer::Run => {
                for d in self.consent_digests() {
                    self.debug.trust.trust(&d);
                }
                self.run_widget_scripts();
            }
            Answer::Show => {
                if !self.debugging() {
                    // For this session only: the host isn't told.
                    self.set_debug_tools(true);
                }
                self.open_window(Kind::Console);
                let text = tr("The widget's scripts are listed above; none of them runs until you start it.");
                self.debug.console.add(LineKind::Info, text);
            }
            _ => {}
        }
    }

    /// Starts every script of the widget's, as scripts that came with it.
    fn run_widget_scripts(&mut self) {
        for s in self.doc.scripts.clone() {
            self.start_script(&s, Caps::EMBEDDED);
        }
    }
}
