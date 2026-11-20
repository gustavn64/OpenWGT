//! The editor's side of scripted parts (`openparts::scripted`,
//! `script::parts`): the language the program gives them, which of them
//! may run (the player's consent, `consent`), their scripts given from
//! the console (a `.js` file that calls `definePart`, dropped onto the
//! window), and what they say.

use super::super::{Editor, Sel};
use super::LineKind;
use crate::doc;
use crate::lang::{tr, trf};
use crate::openparts::scripted::{SCRIPTED, Scripted};
use crate::script::parts::{PartHook, PartRuntime};
use crate::script::trust::Digest;

/// Whether a script file defines a part (it calls `definePart`).
#[must_use]
pub fn defines_part(source: &str) -> bool {
    source.contains("definePart(")
}

impl Editor {
    /// The language scripted parts run in, from the program.
    pub fn set_part_runtime(&mut self, rt: Box<dyn PartRuntime>) {
        self.env.parts = PartHook::new(Some(rt));
        self.sync_scripted_parts();
    }

    /// The scripted parts on the worksheet: index, id, their script's
    /// digest.
    pub(super) fn scripted_parts(&self) -> Vec<(usize, u32, Digest)> {
        self.doc
            .gadgets
            .iter()
            .enumerate()
            .filter_map(|(i, g)| {
                let s = g.behavior.as_any().downcast_ref::<Scripted>()?;
                Some((i, g.core.id, s.digest()))
            })
            .collect()
    }

    /// Each scripted part told whether its script may run (for its
    /// picture).
    pub(in crate::editor) fn sync_scripted_parts(&mut self) {
        let hook = self.env.parts.clone();
        for g in &mut self.doc.gadgets {
            if let Some(s) = g.behavior.as_any_mut().downcast_mut::<Scripted>() {
                let on = hook.allowed(&s.digest());
                s.set_allowed(on);
            }
        }
    }

    /// Lets the scripts with these texts run in parts.
    pub(super) fn allow_parts(&mut self, digests: &[Digest]) {
        for d in digests {
            self.env.parts.allow(*d);
        }
        self.sync_scripted_parts();
    }

    /// Whether the scripted part `id`'s script may run.
    pub(super) fn part_allowed(&self, id: u32) -> Option<bool> {
        let (_, _, d) = self.scripted_parts().into_iter().find(|p| p.1 == id)?;
        Some(self.env.parts.allowed(&d))
    }

    /// Lets the scripted part `id`'s script run, or not (this session).
    pub(super) fn set_part_allowed(&mut self, id: u32, on: bool) {
        let Some((_, _, d)) = self.scripted_parts().into_iter().find(|p| p.1 == id) else {
            return;
        };
        self.env.parts.with(|p| {
            if on {
                p.allowed.insert(d);
            } else {
                p.allowed.remove(&d);
            }
        });
        self.sync_scripted_parts();
    }

    /// Gives the scripted part `id` the script `code` (the player's own:
    /// it may run), which defines its name, inputs and outputs. Its wires
    /// go if its knots change. Undo takes it back.
    ///
    /// # Errors
    /// No part `id` that's a scripted part, no language for them, or the
    /// script doesn't define a part.
    pub fn set_part_code(&mut self, id: u32, code: &str) -> Result<(), String> {
        let Some(i) = self
            .doc
            .gadgets
            .iter()
            .position(|g| g.core.id == id && g.core.tag == SCRIPTED)
        else {
            return Err(format!("#{id} is not a scripted part"));
        };
        if !self.env.parts.has_runtime() {
            return Err(tr("There is no language here for scripted parts.").to_owned());
        }
        let def = self
            .env
            .parts
            .runtime(|rt| rt.define(id, code))
            .ok_or_else(|| tr("There is no language here for scripted parts.").to_owned())?
            .map_err(|e| e.to_string())?;
        self.stop();
        self.snapshot();
        let g = &self.doc.gadgets[i];
        let same =
            g.behavior.as_any().downcast_ref::<Scripted>().is_some_and(|s| {
                s.def().inputs.len() == def.inputs.len() && s.def().outputs.len() == def.outputs.len()
            });
        if !same {
            let mut ws: Vec<usize> = g.wires().collect();
            ws.sort_unstable();
            for w in ws.into_iter().rev() {
                self.doc.remove_wire(w);
            }
        }
        let g = &mut self.doc.gadgets[i];
        if let Some(s) = g.behavior.as_any_mut().downcast_mut::<Scripted>() {
            s.set_code(code.to_owned(), def);
        }
        g.behavior.layout(&mut g.core, &self.assets);
        self.env.parts.allow(Digest::of(code));
        self.sync_scripted_parts();
        self.set_dirty();
        Ok(())
    }

    /// A part's script dropped onto the window: the selected scripted
    /// part's, else a new one's, in the worksheet's top left.
    pub(super) fn drop_part_script(&mut self, name: &str, code: &str) {
        let id = match self.sel {
            Sel::Gadget(g) if self.doc.gadgets.get(g).is_some_and(|g| g.core.tag == SCRIPTED) => {
                self.doc.gadgets[g].core.id
            }
            _ => {
                if !self.env.parts.has_runtime() {
                    let text = tr("There is no language here for scripted parts.");
                    self.debug.console.add(LineKind::Error, text);
                    return;
                }
                let id = self.doc.new_id();
                let n = i32::try_from(self.scripted_parts().len() % 8).unwrap_or(0);
                let g = doc::new_gadget(&self.assets, SCRIPTED, id, 40 + 20 * n, 40 + 20 * n);
                self.snapshot();
                self.doc.gadgets.push(g);
                id
            }
        };
        match self.set_part_code(id, code) {
            Ok(()) => {
                let text = trf("{} is the script of part #{} now.", &[name, &id.to_string()]);
                self.debug.console.add(LineKind::Info, &text);
            }
            Err(e) => self.debug.console.add(LineKind::Error, &e),
        }
    }

    /// What the scripted parts said since the last frame, in the console
    /// (with the tools) and to the program's output.
    pub(in crate::editor) fn part_scripts_said(&mut self) {
        let mut said = self.env.parts.with(|p| std::mem::take(&mut p.log)).unwrap_or_default();
        said.extend(self.env.parts.runtime(|rt| rt.take_log()).unwrap_or_default());
        for (level, text) in said {
            self.debug.sink.log(level, &text);
            self.console_heard(level, &text);
        }
    }

    /// Another widget came: the parts' scripts forget the last one's.
    pub(in crate::editor) fn part_scripts_forget(&mut self) {
        self.env.parts.runtime(|rt| rt.clear());
        self.sync_scripted_parts();
    }
}
