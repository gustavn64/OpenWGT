//! The port's own data, "OpenWDATA": everything the game reads from its CD
//! besides the pictures (`crate::opengfx`) and the sounds
//! (`crate::opensfx`), made for the port, so that it runs with no CD at
//! all, as OpenTTD runs on OpenGFX, OpenSFX and OpenMSX. The tables and
//! layouts are the game's (the knots' places, the dialogs' controls: they
//! are what the code and saved widgets expect); the texts, the help, the
//! puzzles and the sample widgets are the project's own.
//!
//! Built from `tools/opendata` into `crates/engine/opendata`, and built
//! into the port ([`builtin`]):
//!
//! ```text
//! <MODULE>/<type>/<id>.bin   resources, in the overrides' layout
//!                            (crate::assets): dialogs, menus, strings,
//!                            texts, tables, the palette
//! WIDGET.PZL                 the puzzles (formats::pzl)
//! HELP.TXT                   the help (formats::helptext)
//! samples/<name>.WGT         the sample widgets
//! ```

use std::collections::BTreeMap;

use crate::assets::{Module, PUZZLE_FILE, Source};
use crate::opengfx::Embedded;

/// The help file in the pack.
pub const HELP_FILE: &str = "HELP.TXT";
/// Where the sample widgets are in the pack.
pub const SAMPLE_DIR: &str = "samples";

/// The pack built into the port: the files of `crates/engine/opendata`
/// (`build.rs`).
#[must_use]
pub fn builtin() -> Embedded {
    Embedded(BUILTIN)
}

include!(concat!(env!("OUT_DIR"), "/opendata_files.rs"));

/// What a pack gives.
#[derive(Default)]
pub struct Loaded {
    pub resources: BTreeMap<(Module, u32, u32), Vec<u8>>,
    pub samples: Vec<(String, Vec<u8>)>,
    pub puzzles: Option<Vec<u8>>,
    pub help: Option<Vec<u8>>,
    pub problems: Vec<String>,
}

/// The data of the pack `pack`.
#[must_use]
pub fn load(pack: &dyn Source) -> Loaded {
    let mut out = Loaded {
        resources: crate::assets::read_resources(pack),
        puzzles: pack.read(PUZZLE_FILE),
        help: pack.read(HELP_FILE),
        ..Loaded::default()
    };
    let mut names = pack.list(SAMPLE_DIR);
    names.sort();
    for name in names {
        if name.to_ascii_uppercase().ends_with(".WGT")
            && let Some(b) = pack.read(&format!("{SAMPLE_DIR}/{name}"))
        {
            out.samples.push((name, b));
        }
    }
    if out.resources.is_empty() {
        out.problems.push("opendata: the pack has no resources".into());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The built-in pack has what the editor needs to start: the menu,
    /// the palette, the dialogs, the parts' tables.
    #[test]
    fn builtin_pack_has_the_basics() {
        use crate::assets::rt;
        let l = load(&builtin());
        assert!(l.problems.is_empty(), "{:?}", l.problems);
        for key in [
            (Module::Exe, formats::pe::RT_MENU, 3),
            (Module::Exe, rt::PALETTE, 101),
            (Module::Exe, formats::pe::RT_DIALOG, 2040),
            (Module::Gadget, rt::KNOTS, 1029),
            (Module::Gadget, rt::INFO, 2008),
        ] {
            assert!(l.resources.contains_key(&key), "{key:?}");
        }
    }
}
