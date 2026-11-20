//! The network parts' own pictures, dialogs, texts and tables, built into
//! the port from `crates/engine/opennet` (`tools/opennet`), with their
//! words in Swedish and French in `opennet-sv` and `opennet-fr`: the
//! layout of OpenWGFX's pictures (`crate::opengfx`) and OpenWDATA's
//! resources (`crate::opendata`) in one folder.
//!
//! The game's data has nothing for these parts, the CD's no more than
//! OpenWDATA, so they go behind all of it (`crate::port_parts::install`):
//! they fill only what is missing, and a string only where its block
//! leaves it empty (`Assets::string`). Exporting the resources leaves
//! them out.
//!
//! The pack also has the word list of the random channel names
//! ([`words`]).

use std::sync::OnceLock;

use crate::assets::{Embedded, Source};
use crate::lang::Lang;

include!(concat!(env!("OUT_DIR"), "/opennet_files.rs"));
include!(concat!(env!("OUT_DIR"), "/opennet-sv_files.rs"));
include!(concat!(env!("OUT_DIR"), "/opennet-fr_files.rs"));

/// The random channel names' words, in the pack.
pub const WORDS_FILE: &str = "words.txt";

/// The pack built into the port, in English.
#[must_use]
pub fn builtin() -> Embedded {
    Embedded::new(BUILTIN)
}

/// The pack in `lang`: the English one, under the language's own
/// pictures, dialogs and texts.
#[must_use]
pub fn builtin_for(lang: Lang) -> Box<dyn Source> {
    crate::port_parts::embedded(BUILTIN, BUILTIN_SV, BUILTIN_FR, lang)
}

/// The words random channel names are made of (`tools/opennet/src/words.txt`):
/// 2048 short English words, the same in every language, sorted.
#[must_use]
pub fn words() -> &'static [&'static str] {
    static WORDS: OnceLock<Vec<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| {
        let text = BUILTIN
            .iter()
            .find(|(path, _)| *path == WORDS_FILE)
            .and_then(|(_, b)| std::str::from_utf8(b).ok())
            .unwrap_or_default();
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{Assets, Module, Packs, rt};
    use crate::gfx::Pictures;
    use formats::tables::KnotKind;

    /// The words each language's pack has, and its CD's string 2564 (the
    /// Text Label's name, the last of block 161 the game has).
    fn words(lang: Lang) -> (&'static str, &'static str, &'static str, &'static str) {
        match lang {
            Lang::En => ("Sender", "Receiver", "BUSY", "Channel name:"),
            Lang::Sv => ("Sändare", "Mottagare", "UPPTAGEN", "Kanalnamn:"),
            Lang::Fr => ("Émetteur", "Récepteur", "OCCUPÉ", "Nom du canal\u{a0}:"),
        }
    }

    /// What the parts need, in `lang`, whatever the rest of `a` is.
    fn check(a: &Assets, lang: Lang) {
        let (send, recv, busy, channel) = words(lang);
        assert!(a.pack_problems.is_empty(), "{:?}", a.pack_problems);
        assert_eq!(a.string(2565).as_deref(), Some(send));
        assert_eq!(a.string(2566).as_deref(), Some(recv));
        assert_eq!(a.string(super::super::parts::BUSY_STRING).as_deref(), Some(busy));
        let knots = |t| crate::gadget::knots(a, t);
        let s = knots(1900);
        let r = knots(1901);
        assert_eq!((s.len(), r.len()), (1, 1));
        assert_eq!((s[0].def.at.v, s[0].def.at.h, s[0].def.kind), (36, 4, KnotKind::Input));
        assert_eq!(
            (r[0].def.at.v, r[0].def.at.h, r[0].def.kind),
            (36, 83, KnotKind::Output)
        );
        let pictures = Pictures::default();
        for (sheet, frames, size) in [(1900, 4, (88, 50)), (1901, 4, (88, 50)), (1902, 2, (44, 25))] {
            for n in 1..=frames {
                let r = pictures.rect(a, Module::Gadget, sheet, n).unwrap();
                assert_eq!((r.width(), r.height()), size, "{sheet}/{n}");
                assert!(pictures.frame(a, Module::Gadget, sheet, n).is_some());
            }
            assert!(pictures.rect(a, Module::Gadget, sheet, frames + 1).is_none());
        }
        // Each knot is on its picture.
        for k in [&s[0], &r[0]] {
            assert!((0..88).contains(&k.def.at.h) && (0..50).contains(&k.def.at.v));
        }
        for (id, title) in [(2900u16, send), (2901, recv)] {
            let d = formats::dialog::Dialog::parse(a.get(Module::Exe, formats::pe::RT_DIALOG, u32::from(id)).unwrap())
                .unwrap();
            assert_eq!(d.title, title);
            let text = |i: u16| d.items.iter().find(|c| c.id == i).map(|c| c.title.clone());
            assert_eq!(text(105), Some(formats::dialog::Field::Text(channel.into())));
            assert!(text(102).is_some() && text(103).is_some() && text(106).is_some() && text(107).is_some());
            let info = formats::tables::gadget_info(a.get(Module::Gadget, rt::INFO, u32::from(id)).unwrap()).unwrap();
            assert_eq!((info.id, info.dialog, info.doc), (id, id, id + 2000));
            for t in [info.dialog, info.doc] {
                let text = a.text(Module::Gadget, u32::from(t)).unwrap();
                let n = text.chars().count();
                assert!((100..300).contains(&n), "{lang:?} {t}: {n}");
            }
        }
    }

    #[test]
    fn without_the_cd_every_language_has_the_parts() {
        for lang in Lang::ALL {
            let data = crate::opendata::builtin_for(lang);
            let packs = Packs {
                data: Some(&data),
                ..Packs::default()
            };
            let a = Assets::load_with(None, packs, None).unwrap();
            check(&a, lang);
            // The game's own strings in the same block are OpenWDATA's.
            assert_eq!(
                a.string(2564),
                crate::opendata::load(&data)
                    .resources
                    .get(&(Module::Exe, 6, 161))
                    .and_then(|b| formats::pe::string_in_block(b, 4))
            );
        }
    }

    #[test]
    fn the_languages_have_their_own_words_on_the_parts() {
        let palette = Assets::load(None, Some(&crate::opendata::builtin())).unwrap().palette();
        let frame = |lang| {
            let mut l = crate::opengfx::load(&*builtin_for(lang), &palette);
            l.resources.remove(&(Module::Gadget, rt::PICTURES, 1900)).unwrap()
        };
        let en = frame(Lang::En);
        assert!(frame(Lang::Sv) != en && frame(Lang::Fr) != en);
    }

    #[test]
    fn with_a_cd_the_parts_fill_its_gaps() {
        for (lang, cd, label) in crate::port_parts::testing::cds() {
            let a = Assets::load(Some(&*cd), None).unwrap();
            assert_eq!(a.lang, lang);
            check(&a, lang);
            // The CD's resources stay as they are: its block 161 (the
            // tray's names) is its own, with its own last name.
            assert_eq!(a.string(2564).as_deref(), Some(label));
            assert_ne!(
                a.get(Module::Exe, formats::pe::RT_STRING, 161),
                a.extra
                    .get(&(Module::Exe, formats::pe::RT_STRING, 161))
                    .map(Vec::as_slice)
            );
            // An export is the CD's alone.
            assert!(
                !a.all()
                    .any(|(m, t, i, _)| (m, t, i) == (Module::Gadget, rt::PICTURES, 1900))
            );
        }
    }
}
