//! The open parts' own pictures, dialogs, texts and tables, built into
//! the port from `crates/engine/openparts` (`tools/openparts`), with
//! their words in Swedish and French in `openparts-sv` and `openparts-fr`:
//! the layout of OpenWGFX's pictures (`crate::opengfx`) and OpenWDATA's
//! resources (`crate::opendata`) in one folder.
//!
//! The game's data has nothing for these parts, the CD's no more than
//! OpenWDATA, so they go behind all of it (`crate::port_parts::install`):
//! they fill only what is missing, and a string only where its block
//! leaves it empty (`Assets::string`). Exporting the resources leaves
//! them out.

use crate::assets::{Embedded, Source};
use crate::lang::Lang;

include!(concat!(env!("OUT_DIR"), "/openparts_files.rs"));
include!(concat!(env!("OUT_DIR"), "/openparts-sv_files.rs"));
include!(concat!(env!("OUT_DIR"), "/openparts-fr_files.rs"));

/// The pack built into the port, in English.
#[must_use]
pub fn builtin() -> Embedded {
    Embedded::new(BUILTIN)
}

/// The pack in `lang`: the English one, under the language's own
/// dialogs and texts.
#[must_use]
pub fn builtin_for(lang: Lang) -> Box<dyn Source> {
    crate::port_parts::embedded(BUILTIN, BUILTIN_SV, BUILTIN_FR, lang)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{Assets, Module, Packs, rt};
    use crate::gfx::Pictures;
    use formats::dialog::{Dialog, Field};
    use formats::tables::KnotKind;

    /// The words each language's pack has.
    fn words(lang: Lang) -> (&'static str, &'static str) {
        match lang {
            Lang::En => ("Oscilloscope", "Automatic"),
            Lang::Sv => ("Oscilloskop", "Automatiskt"),
            Lang::Fr => ("Oscilloscope", "Automatique"),
        }
    }

    /// What the Oscilloscope needs, in `lang`, whatever the rest of `a` is.
    fn check(a: &Assets, lang: Lang) {
        let (name, auto) = words(lang);
        assert!(a.pack_problems.is_empty(), "{:?}", a.pack_problems);
        assert_eq!(a.string(2567).as_deref(), Some(name));
        let k = crate::gadget::knots(a, 1903);
        let at: Vec<(i16, i16, KnotKind)> = k.iter().map(|k| (k.def.at.h, k.def.at.v, k.def.kind)).collect();
        assert_eq!(
            at,
            [
                (4, 26, KnotKind::Input),
                (4, 50, KnotKind::Input),
                (4, 87, KnotKind::Input),
                (143, 26, KnotKind::Output)
            ]
        );
        let pictures = Pictures::default();
        for (sheet, frames, size) in [(1903, 2, (148, 100)), (1904, 2, (50, 34))] {
            for n in 1..=frames {
                let r = pictures.rect(a, Module::Gadget, sheet, n).unwrap();
                assert_eq!((r.width(), r.height()), size, "{sheet}/{n}");
                assert!(pictures.frame(a, Module::Gadget, sheet, n).is_some());
            }
            assert!(pictures.rect(a, Module::Gadget, sheet, frames + 1).is_none());
        }
        // Each knot whole on the picture: its ring, 3.9 pixels round, too.
        for k in &k {
            let (h, v) = (k.def.at.h, k.def.at.v);
            assert!((4..148 - 4).contains(&h) && (4..100 - 4).contains(&v), "{h}, {v}");
        }
        let d = Dialog::parse(a.get(Module::Exe, formats::pe::RT_DIALOG, 2902).unwrap()).unwrap();
        assert_eq!(d.title, name);
        let text = |i: u16| d.items.iter().find(|c| c.id == i).map(|c| c.title.clone());
        assert_eq!(text(106), Some(Field::Text(format!("{auto}1"))));
        assert_eq!(text(112), Some(Field::Text("1\u{d7}2".into())));
        let info = formats::tables::gadget_info(a.get(Module::Gadget, rt::INFO, 2902).unwrap()).unwrap();
        assert_eq!((info.id, info.dialog, info.doc), (2902, 2902, 4902));
        for t in [info.dialog, info.doc] {
            let text = a.text(Module::Gadget, u32::from(t)).unwrap();
            let n = text.chars().count();
            assert!((100..300).contains(&n), "{lang:?} {t}: {n}");
        }
        scripted(a, lang);
    }

    /// What the scripted part needs, in `lang`.
    fn scripted(a: &Assets, lang: Lang) {
        let name = match lang {
            Lang::En => "Scripted Part",
            Lang::Sv => "Skriptdel",
            Lang::Fr => "Pièce scriptée",
        };
        assert_eq!(a.string(2568).as_deref(), Some(name));
        let k = crate::gadget::knots(a, 1905);
        assert_eq!(k.len(), 8);
        let pictures = Pictures::default();
        for n in 1..=4 {
            assert!(pictures.frame(a, Module::Gadget, 1905, n).is_some(), "{n}");
        }
        let r = pictures.rect(a, Module::Gadget, 1905, 1).unwrap();
        assert_eq!((r.width(), r.height()), (128, 84));
        assert!(pictures.rect(a, Module::Gadget, 1905, 5).is_none());
        let info = formats::tables::gadget_info(a.get(Module::Gadget, rt::INFO, 2903).unwrap()).unwrap();
        assert_eq!((info.id, info.dialog, info.doc), (2903, 2903, 4903));
        for t in [info.dialog, info.doc] {
            let text = a.text(Module::Gadget, u32::from(t)).unwrap();
            let n = text.chars().count();
            assert!((100..500).contains(&n), "{lang:?} {t}: {n}");
        }
    }

    #[test]
    fn without_the_cd_every_language_has_the_oscilloscope() {
        for lang in Lang::ALL {
            let data = crate::opendata::builtin_for(lang);
            let packs = Packs {
                data: Some(&data),
                ..Packs::default()
            };
            let a = Assets::load_with(None, packs, None).unwrap();
            check(&a, lang);
        }
    }

    #[test]
    fn with_a_cd_the_oscilloscope_fills_its_gaps() {
        for (lang, cd, label) in crate::port_parts::testing::cds() {
            let a = Assets::load(Some(&*cd), None).unwrap();
            assert_eq!(a.lang, lang);
            check(&a, lang);
            // The CD's own names stay.
            assert_eq!(a.string(2564).as_deref(), Some(label));
            // An export is the CD's alone.
            assert!(
                !a.all()
                    .any(|(m, t, i, _)| (m, t) == (Module::Gadget, rt::PICTURES) && (1903..=1905).contains(&i))
            );
        }
    }
}
