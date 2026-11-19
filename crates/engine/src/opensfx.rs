//! The port's own sounds, "OpenWSFX": new sounds for the game's, made for
//! the port, that stand in for the CD's (`WWSOUND.DLL`'s type 1006) as
//! `OpenSFX` stands in for Transport Tycoon's. Each keeps the original's
//! number and job (the Animal Sound's dog is still a dog, the Speaker's
//! tone samples are still 220 Hz and 1760 Hz); the sounds themselves are
//! new. The game has no music besides its intro's, which the port leaves
//! out, so there is no music set.
//!
//! Started with `--opensfx`, the port plays the copy built into it
//! ([`builtin`], `crates/engine/opensfx`); `--opensfx=DIR` reads a folder
//! of the same layout:
//!
//! ```text
//! sounds/<id>.wav   sound <id> (WWSOUND 1006/<id>): PCM, mono or stereo,
//!                   8 or 16 bits, any rate
//! ```
//!
//! A sound the pack leaves out is the CD's. `tools/opensfx/` makes the
//! pack.

use std::collections::BTreeMap;

use crate::assets::{Embedded, Module, Source, rt};

/// The pack built into the port: the files of `crates/engine/opensfx`
/// (`build.rs`).
#[must_use]
pub fn builtin() -> Embedded {
    Embedded::new(BUILTIN)
}

include!(concat!(env!("OUT_DIR"), "/opensfx_files.rs"));

/// What a pack gives: the sounds as resources, and what it couldn't use.
#[derive(Default)]
pub struct Loaded {
    pub resources: BTreeMap<(Module, u32, u32), Vec<u8>>,
    pub problems: Vec<String>,
}

/// The sounds of the pack `pack`.
#[must_use]
pub fn load(pack: &dyn Source) -> Loaded {
    let mut out = Loaded::default();
    for file in pack.list("sounds") {
        let Some((stem, ext)) = file.rsplit_once('.') else {
            continue;
        };
        if !ext.eq_ignore_ascii_case("wav") {
            continue;
        }
        let Ok(id) = stem.parse::<u32>() else { continue };
        let path = format!("sounds/{file}");
        let Some(bytes) = pack.read(&path) else { continue };
        if formats::tables::Wave::parse(&bytes).is_none_or(|w| w.samples.is_empty() || w.rate == 0) {
            out.problems.push(format!("opensfx: {path}: not a PCM wave"));
            continue;
        }
        out.resources.insert((Module::Sound, rt::SOUND, id), bytes);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_pack_is_whole() {
        let l = load(&builtin());
        assert!(l.problems.is_empty(), "{:?}", l.problems);
    }

    #[test]
    fn takes_waves_only() {
        let wave = formats::tables::Wave {
            rate: 11025,
            samples: vec![0x80, 0x90],
        }
        .write();
        let files: Vec<(&'static str, &'static [u8])> = vec![
            ("sounds/200.wav", Box::leak(wave.into_boxed_slice())),
            ("sounds/201.wav", b"not a wave"),
            ("sounds/notes.txt", b"hi"),
        ];
        let l = load(&Embedded::new(Box::leak(files.into_boxed_slice())));
        assert_eq!(l.resources.keys().map(|k| k.2).collect::<Vec<_>>(), [200]);
        assert_eq!(l.problems.len(), 1);
    }
}
