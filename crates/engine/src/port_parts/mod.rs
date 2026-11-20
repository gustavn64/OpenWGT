//! The port's own parts, the ones the original doesn't have, in families
//! that each bring their classes, their places in the tray and a pack of
//! pictures, dialogs, texts and tables: the open parts
//! (`crate::openparts`, such as the Oscilloscope) and the network parts
//! (`crate::net`). Each family is one [`Family`] in [`FAMILIES`]; nothing
//! here knows more of them than that, so a family comes or goes with its
//! line there.
//!
//! - **Classes** after the game's: `gadgets::class` asks [`class`].
//! - **The tray**: each part a tray index after the game's 64 (up to
//!   [`TRAY_SLOTS`]), its help line string `2500 +` the index as the
//!   game's, its picture a frame of the family's own sheet, and what each
//!   folder gets while the port's enhancements are on ([`tray_extra`]).
//! - **Resources**: every family's pack goes behind all the game's data
//!   ([`install`]), the first family's first where two have the same
//!   resource; a string block is shared, string by string.
//! - **Saving**: [`wgt`] hides the parts from the original game's loader,
//!   and with them what the port keeps about a widget beside the game's
//!   content ([`ext`]).

pub mod ext;
pub mod wgt;

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use crate::assets::{Assets, Embedded, Module, Source};
use crate::gadget::{Core, Tag};
use crate::gadgets::Class;
use crate::lang::Lang;
use crate::opengfx::Overlay;

/// A part of a family in the tray.
#[derive(Debug, Clone, Copy)]
pub struct TrayPart {
    /// Its tray index, after the game's 64 and below [`TRAY_SLOTS`].
    pub index: u8,
    pub tag: Tag,
    /// Its picture in the tray: a sheet and a frame.
    pub picture: (u32, u16),
}

/// A family of the port's own parts.
pub struct Family {
    /// What it's called, for the problems the packs have.
    pub name: &'static str,
    /// Its classes.
    pub classes: &'static [Class],
    /// Its parts in the tray.
    pub tray: &'static [TrayPart],
    /// What it adds to the tray's folders while the enhancements are on:
    /// a folder and the tray indexes put at its end (−2 a page break).
    pub folders: &'static [(usize, &'static [i8])],
    /// Its pictures, dialogs, texts and tables, in a language: in the
    /// layout of OpenWGFX's pictures and OpenWDATA's resources.
    pub pack: fn(Lang) -> Box<dyn Source>,
    /// What a part just taken from the tray gets (the network parts a
    /// channel name), if anything.
    pub taken: Option<fn(&mut Core)>,
}

/// Every family, in the order their resources win a tie.
pub static FAMILIES: &[&Family] = &[&crate::openparts::FAMILY, &crate::net::FAMILY];

/// The tray indexes there are counts for: the game's 64 (index 0
/// unused) and the families' ones after them.
pub const TRAY_SLOTS: usize = 68;

fn tray_parts() -> impl Iterator<Item = &'static TrayPart> {
    FAMILIES.iter().flat_map(|f| f.tray)
}

/// Every family's classes.
pub fn classes() -> impl Iterator<Item = &'static Class> {
    FAMILIES.iter().flat_map(|f| f.classes)
}

/// The class tagged `tag`, if a family has it.
#[must_use]
pub fn class(tag: Tag) -> Option<&'static Class> {
    classes().find(|c| c.tag == tag)
}

/// Whether `tag` is one of the port's own parts' (the original game
/// has no class for it).
#[must_use]
pub fn is_port_part(tag: Tag) -> bool {
    class(tag).is_some()
}

/// The tag of tray index `i`, if it is a family's.
#[must_use]
pub fn tray_tag(i: u8) -> Option<Tag> {
    tray_parts().find(|p| p.index == i).map(|p| p.tag)
}

/// The tray index of a family's `tag`.
#[must_use]
pub fn tray_index(tag: Tag) -> Option<u8> {
    tray_parts().find(|p| p.tag == tag).map(|p| p.index)
}

/// Tray index `i`'s picture, if it is a family's.
#[must_use]
pub fn tray_picture(i: u8) -> Option<(u32, u16)> {
    tray_parts().find(|p| p.index == i).map(|p| p.picture)
}

/// What the families add to folder `f`'s list of tray indexes, in the
/// order of [`FAMILIES`].
#[must_use]
pub fn tray_extra(f: usize) -> Vec<i8> {
    FAMILIES
        .iter()
        .flat_map(|fam| fam.folders)
        .filter(|(folder, _)| *folder == f)
        .flat_map(|(_, list)| list.iter().copied())
        .collect()
}

/// A part just taken from the tray: what its family gives a new one.
pub fn taken_from_tray(core: &mut Core) {
    for f in FAMILIES {
        if let Some(taken) = f.taken
            && f.classes.iter().any(|c| c.tag == core.tag)
        {
            taken(core);
        }
    }
}

/// A pack built into the program (`build.rs`) in `lang`: the English
/// files `en` under the language's own, which have only what differs.
#[must_use]
pub fn embedded(
    en: &'static [(&'static str, &'static [u8])],
    sv: &'static [(&'static str, &'static [u8])],
    fr: &'static [(&'static str, &'static [u8])],
    lang: Lang,
) -> Box<dyn Source> {
    let top = match lang {
        Lang::Sv => sv,
        Lang::Fr => fr,
        Lang::En => &[],
    };
    if top.is_empty() {
        Box::new(Embedded::new(en))
    } else {
        Box::new(Overlay {
            top: Box::new(Embedded::new(top)),
            base: Box::new(Embedded::new(en)),
        })
    }
}

/// Puts every family's pack, in the language of the game's words (its
/// word for Cancel, the CD's or OpenWDATA's), behind everything `a` has:
/// they fill only what is missing, and a string only where its block
/// leaves it empty (`Assets::string`).
pub(crate) fn install(a: &mut Assets) {
    let lang = a.string(0xf221).and_then(|w| Lang::of_cancel(&w)).unwrap_or(a.lang);
    let palette = a.palette();
    let mut extra: BTreeMap<(Module, u32, u32), Vec<u8>> = BTreeMap::new();
    for family in FAMILIES {
        let pack = (family.pack)(lang);
        let mut res = crate::assets::read_resources(&*pack);
        let mut pictures = crate::opengfx::load(&*pack, &palette);
        res.append(&mut pictures.resources);
        a.pack_problems.append(&mut pictures.problems);
        for (key, bytes) in res {
            match extra.entry(key) {
                Entry::Vacant(e) => {
                    e.insert(bytes);
                }
                Entry::Occupied(mut e) if key.1 == formats::pe::RT_STRING => {
                    *e.get_mut() = merge_strings(e.get(), &bytes);
                }
                Entry::Occupied(_) => a
                    .pack_problems
                    .push(format!("{}: {key:?} is another family's", family.name)),
            }
        }
    }
    a.extra = extra;
}

/// Two string blocks as one: each of the 16 strings `first`'s, else
/// `second`'s where `first` has it empty.
fn merge_strings(first: &[u8], second: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for n in 0..16 {
        let s = formats::pe::string_in_block(first, n)
            .filter(|s| !s.is_empty())
            .or_else(|| formats::pe::string_in_block(second, n))
            .unwrap_or_default();
        let units: Vec<u16> = s.encode_utf16().collect();
        out.extend_from_slice(&u16::try_from(units.len()).unwrap_or(0).to_le_bytes());
        for u in units {
            out.extend_from_slice(&u.to_le_bytes());
        }
    }
    out
}

/// What the families' tests share.
#[cfg(test)]
pub(crate) mod testing {
    use crate::assets::Source;
    use crate::lang::Lang;

    /// A CD image in memory, through `formats::iso`.
    struct Iso(Vec<u8>);

    impl Source for Iso {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            formats::iso::read_file(&self.0, path)
        }

        fn list(&self, path: &str) -> Vec<String> {
            self.read(path)
                .and_then(|d| formats::iso::records(&d).ok())
                .map_or_default(|r| r.into_iter().map(|r| r.name).collect())
        }
    }

    /// The CDs: the English one extracted to `orig` (as `tools/` leaves
    /// it), the Swedish and French ones where `WIDGET_SV_CD` and
    /// `WIDGET_FR_CD` say (a folder or an image); each skipped without.
    pub(crate) fn cds() -> Vec<(Lang, Box<dyn Source>, &'static str)> {
        let mut out: Vec<(Lang, Box<dyn Source>, &'static str)> = Vec::new();
        let orig = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../orig");
        if orig.exists() {
            out.push((Lang::En, Box::new(crate::assets::Dir(orig)), "Label"));
        } else {
            eprintln!("no orig: skipped");
        }
        for (lang, var, label) in [(Lang::Sv, "WIDGET_SV_CD", "Etikett"), (Lang::Fr, "WIDGET_FR_CD", "Nom")] {
            let Some(p) = std::env::var_os(var).map(std::path::PathBuf::from) else {
                eprintln!("no {var}: skipped");
                continue;
            };
            if p.is_dir() {
                out.push((lang, Box::new(crate::assets::Dir(p)), label));
            } else if let Ok(image) = std::fs::read(&p) {
                out.push((lang, Box::new(Iso(image)), label));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_families_dont_meet() {
        let parts: Vec<&TrayPart> = tray_parts().collect();
        for (i, p) in parts.iter().enumerate() {
            assert!((65..TRAY_SLOTS).contains(&usize::from(p.index)), "{}", p.tag);
            assert!(parts[..i].iter().all(|q| q.index != p.index && q.tag != p.tag));
            assert!(class(p.tag).is_some(), "{}", p.tag);
            assert_eq!((tray_tag(p.index), tray_index(p.tag)), (Some(p.tag), Some(p.index)));
            assert_eq!(tray_picture(p.index), Some(p.picture));
        }
        let classes: Vec<&Class> = classes().collect();
        for (i, c) in classes.iter().enumerate() {
            assert!(
                classes[..i]
                    .iter()
                    .all(|d| d.tag != c.tag && d.gadget_id != c.gadget_id && d.sheet != c.sheet)
            );
            // None of the game's has their tags or numbers.
            assert!(
                crate::gadgets::CLASSES
                    .iter()
                    .all(|g| g.tag != c.tag && g.gadget_id != c.gadget_id && g.sheet != c.sheet && g.knots != c.knots)
            );
            assert!(is_port_part(c.tag));
        }
        assert!(!is_port_part(Tag::new(b"cnst")));
        assert_eq!((tray_tag(64), tray_tag(0), tray_picture(30)), (None, None, None));
        // The folders' extras name only the families' parts.
        for f in 0..7 {
            for i in tray_extra(f) {
                assert!(i == -2 || tray_tag(u8::try_from(i).unwrap()).is_some(), "{f}: {i}");
            }
        }
    }

    #[test]
    fn string_blocks_are_shared() {
        let block = |strings: &[(u32, &str)]| {
            let mut out = Vec::new();
            for n in 0..16 {
                let s = strings.iter().find(|(k, _)| *k == n).map_or("", |(_, s)| s);
                let units: Vec<u16> = s.encode_utf16().collect();
                out.extend_from_slice(&u16::try_from(units.len()).unwrap().to_le_bytes());
                for u in units {
                    out.extend_from_slice(&u.to_le_bytes());
                }
            }
            out
        };
        let a = block(&[(5, "Sender"), (6, "Receiver")]);
        let b = block(&[(6, "Other"), (7, "Oscilloskop")]);
        assert_eq!(
            merge_strings(&a, &b),
            block(&[(5, "Sender"), (6, "Receiver"), (7, "Oscilloskop")])
        );
        assert_eq!(merge_strings(&a, &[]), a);
    }
}
