//! The port's own graphics, "OpenWGFX": new pictures for the game's
//! picture sheets and pointers, drawn for the port, that stand in for the
//! CD's as `OpenGFX` stands in for Transport Tycoon's. Each frame keeps the
//! original's size, offset and meaning (the knots where the wires plug in,
//! the places the digits are drawn), so the game's code and tables work
//! unchanged; the pictures themselves are new. The sounds, the texts, the
//! dialogs and the tables still come from the CD.
//!
//! Started with `--opengfx`, the port draws with the copy built into it
//! ([`builtin`], `crates/engine/opengfx`); `--opengfx=DIR` reads a folder
//! of the same layout instead, for drawing new pictures:
//!
//! ```text
//! sheets.txt                 <sheet> <frame> <top> <left> <bottom> <right>
//! gadgets/<sheet>/<frame>.png  frame <frame> (from 1) of sheet <sheet>
//! cursors.txt                <pointer> <hot x> <hot y>
//! cursors/<pointer>.png      the editor's pointer <pointer> (0 to 6)
//! ```
//!
//! A PNG is the size of its frame's rectangle; its colours are taken to
//! the nearest in the game's palette (the pack's own are exactly the
//! palette's), and pixels less than half opaque are transparent. A sheet
//! is used only when every one of its frames has a picture; a sheet the
//! pack leaves out is the CD's. The pointers are `WIDGET.EXE`'s cursor
//! groups 5007 to 5013 (`editor::Pointer`). With the pack the port's own
//! fonts come before the CD's (`assets::bundled_fonts`).
//! `tools/opengfx/` draws the pack.

use std::collections::{BTreeMap, HashMap};

use formats::hr::{self, Image, Rect};

use crate::assets::{Module, Source, rt};

/// The frames' rectangles.
pub const SHEETS_FILE: &str = "sheets.txt";
/// The pointers' hot spots.
pub const CURSORS_FILE: &str = "cursors.txt";
/// The first of the editor's pointers' cursor groups.
const FIRST_GROUP: u32 = 5007;
/// The pointers' own `RT_CURSOR` ids, out of the way of the CD's.
const FIRST_CURSOR: u16 = 31000;

/// The pack built into the port: the files of `crates/engine/opengfx`
/// (`build.rs`).
#[must_use]
pub fn builtin() -> Embedded {
    Embedded(BUILTIN)
}

include!(concat!(env!("OUT_DIR"), "/opengfx_files.rs"));
include!(concat!(env!("OUT_DIR"), "/opengfx-sv_files.rs"));

/// The built-in pack for `lang`: the English pictures, under the
/// language's own where it has them (its pictures with words,
/// `crates/engine/opengfx-sv`).
#[must_use]
pub fn builtin_for(lang: crate::lang::Lang) -> Box<dyn Source> {
    match lang {
        crate::lang::Lang::Sv if !BUILTIN_SV.is_empty() => Box::new(Overlay {
            top: Box::new(Embedded(BUILTIN_SV)),
            base: Box::new(builtin()),
        }),
        _ => Box::new(builtin()),
    }
}

/// A pack's pictures over another's: a frame `top` has stands in for
/// `base`'s. The sheets' and pointers' tables are `base`'s (`top` draws
/// the same frames at the same places).
pub struct Overlay {
    pub top: Box<dyn Source>,
    pub base: Box<dyn Source>,
}

impl Source for Overlay {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        if path.eq_ignore_ascii_case(SHEETS_FILE) || path.eq_ignore_ascii_case(CURSORS_FILE) {
            return self.base.read(path);
        }
        self.top.read(path).or_else(|| self.base.read(path))
    }

    fn list(&self, path: &str) -> Vec<String> {
        let mut names = self.base.list(path);
        for n in self.top.list(path) {
            if !names.iter().any(|m| m.eq_ignore_ascii_case(&n)) {
                names.push(n);
            }
        }
        names
    }
}

/// Files built into the program, as a [`Source`].
pub struct Embedded(pub &'static [(&'static str, &'static [u8])]);

impl Source for Embedded {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.0
            .iter()
            .find(|(p, _)| p.eq_ignore_ascii_case(path))
            .map(|(_, b)| b.to_vec())
    }

    fn list(&self, path: &str) -> Vec<String> {
        let prefix = if path.is_empty() {
            String::new()
        } else {
            format!("{}/", path.trim_end_matches('/').to_ascii_lowercase())
        };
        let mut names: Vec<String> = self
            .0
            .iter()
            .filter_map(|(p, _)| {
                let rest = p.to_ascii_lowercase().strip_prefix(&prefix)?.to_owned();
                let name = rest.split('/').next()?;
                // Back in the stored case.
                let start = prefix.len();
                Some(p[start..start + name.len()].to_owned())
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

/// The numbers on the lines of `text` that aren't comments (`#`).
fn rows(text: &str) -> impl Iterator<Item = Vec<i32>> + '_ {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or_default())
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split_whitespace().filter_map(|n| n.parse().ok()).collect())
}

/// Colours to palette indices: the exact ones first, else the nearest.
struct Mapper<'a> {
    palette: &'a [[u8; 3]; 256],
    seen: HashMap<[u8; 3], u8>,
}

impl<'a> Mapper<'a> {
    fn new(palette: &'a [[u8; 3]; 256]) -> Mapper<'a> {
        let mut seen = HashMap::new();
        // The first of equal colours, as the pack's tools pick it.
        for (i, c) in palette.iter().enumerate().rev() {
            seen.insert(*c, u8::try_from(i).unwrap_or(0));
        }
        Mapper { palette, seen }
    }

    fn index(&mut self, rgb: [u8; 3]) -> u8 {
        let palette = self.palette;
        *self
            .seen
            .entry(rgb)
            .or_insert_with(|| crate::gadget::nearest(palette, rgb))
    }

    /// `png` as a frame, if it decodes and is `w` × `h`.
    fn image(&mut self, png: &[u8], w: u16, h: u16) -> Result<Image, String> {
        let p = formats::png::decode(png).map_err(|e| e.to_string())?;
        if (p.width, p.height) != (u32::from(w), u32::from(h)) {
            return Err(format!("{}×{}, not {w}×{h}", p.width, p.height));
        }
        let mut img = Image::new(w, h);
        for (i, px) in p.pixels.iter().enumerate() {
            if px[3] >= 0x80 {
                img.pixels[i] = self.index([px[0], px[1], px[2]]);
                img.opaque[i] = true;
            }
        }
        Ok(img)
    }
}

/// What a pack gives: resources in the CD's layout, and what it couldn't
/// use.
#[derive(Default)]
pub struct Loaded {
    pub resources: BTreeMap<(Module, u32, u32), Vec<u8>>,
    /// The sheets with every frame drawn.
    pub sheets: usize,
    pub problems: Vec<String>,
}

/// Turns the pack `pack` into resources: its sheets into HR picture
/// sheets, its pointers into cursors, in `palette`.
#[must_use]
pub fn load(pack: &dyn Source, palette: &[[u8; 3]; 256]) -> Loaded {
    let mut out = Loaded::default();
    let mut map = Mapper::new(palette);
    let text = |name: &str| String::from_utf8_lossy(&pack.read(name).unwrap_or_default()).into_owned();
    let mut sheets: BTreeMap<u32, Vec<(u16, Rect)>> = BTreeMap::new();
    for r in rows(&text(SHEETS_FILE)) {
        let [sheet, frame, top, left, bottom, right] = r[..] else {
            out.problems
                .push(format!("{SHEETS_FILE}: a line of {} numbers, not 6", r.len()));
            continue;
        };
        let n = |v: i32| i16::try_from(v).unwrap_or(0);
        let (Ok(sheet), Ok(frame)) = (u32::try_from(sheet), u16::try_from(frame)) else {
            continue;
        };
        sheets
            .entry(sheet)
            .or_default()
            .push((frame, Rect::new(n(top), n(left), n(bottom), n(right))));
    }
    'sheets: for (sheet, mut frames) in sheets {
        frames.sort_by_key(|f| f.0);
        let mut pics = Vec::with_capacity(frames.len());
        for (i, (frame, rect)) in frames.iter().enumerate() {
            if usize::from(*frame) != i + 1 {
                out.problems
                    .push(format!("opengfx: sheet {sheet}: frame {} missing", i + 1));
                continue 'sheets;
            }
            let w = u16::try_from(rect.width().max(0)).unwrap_or(0);
            let h = u16::try_from(rect.height().max(0)).unwrap_or(0);
            let path = format!("gadgets/{sheet}/{frame}.png");
            let Some(png) = pack.read(&path) else {
                // Not drawn yet: the CD's sheet, all of it.
                continue 'sheets;
            };
            match map.image(&png, w, h) {
                Ok(img) => pics.push((*rect, img)),
                Err(e) => {
                    out.problems.push(format!("opengfx: {path}: {e}"));
                    continue 'sheets;
                }
            }
        }
        out.resources
            .insert((Module::Gadget, rt::PICTURES, sheet), hr::encode(&pics));
        out.sheets += 1;
    }
    for r in rows(&text(CURSORS_FILE)) {
        let [i, hx, hy] = r[..] else { continue };
        let (Ok(i), Ok(hx), Ok(hy)) = (u16::try_from(i), u16::try_from(hx), u16::try_from(hy)) else {
            continue;
        };
        let path = format!("cursors/{i}.png");
        let Some(png) = pack.read(&path) else { continue };
        let p = match formats::png::decode(&png) {
            Ok(p) => p,
            Err(e) => {
                out.problems.push(format!("opengfx: {path}: {e}"));
                continue;
            }
        };
        let (Ok(w), Ok(h)) = (u16::try_from(p.width), u16::try_from(p.height)) else {
            continue;
        };
        let px: Vec<Option<u8>> = p
            .pixels
            .iter()
            .map(|c| (c[3] >= 0x80).then(|| map.index([c[0], c[1], c[2]])))
            .collect();
        let cur = formats::cursor::encode(w, h, (hx, hy), &px, palette);
        let id = FIRST_CURSOR + i;
        let group = formats::cursor::group(w, h, id, u32::try_from(cur.len()).unwrap_or(0));
        out.resources
            .insert((Module::Exe, formats::pe::RT_CURSOR, u32::from(id)), cur);
        out.resources.insert(
            (Module::Exe, formats::pe::RT_GROUP_CURSOR, FIRST_GROUP + u32::from(i)),
            group,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, c: [u8; 4]) -> Vec<u8> {
        formats::png::encode(&formats::png::Rgba {
            width: w,
            height: h,
            pixels: vec![c; (w * h) as usize],
        })
    }

    #[test]
    fn makes_sheets_of_whole_packs_only() {
        let mut pal = [[0u8; 3]; 256];
        pal[7] = [200, 10, 10];
        let a = png(2, 3, [200, 10, 10, 255]);
        let b = png(1, 1, [0, 0, 0, 0]);
        let files: Vec<(&'static str, &'static [u8])> = vec![
            (
                "sheets.txt",
                b"# sheet frame t l b r\n5 1 0 0 3 2\n5 2 0 0 1 1\n6 1 0 0 3 2\n",
            ),
            ("gadgets/5/1.png", Box::leak(a.clone().into_boxed_slice())),
            ("gadgets/5/2.png", Box::leak(b.into_boxed_slice())),
            ("gadgets/6/1.png", Box::leak(png(4, 4, [0; 4]).into_boxed_slice())),
        ];
        let pack = Embedded(Box::leak(files.into_boxed_slice()));
        assert_eq!(pack.list("gadgets"), ["5", "6"]);
        let l = load(&pack, &pal);
        assert_eq!(l.sheets, 1);
        assert_eq!(l.problems.len(), 1, "{:?}", l.problems);
        let s = hr::Sheet::parse(&l.resources[&(Module::Gadget, rt::PICTURES, 5)]).unwrap();
        let f = s.decode(1).unwrap();
        assert_eq!(f.get(1, 2), Some(7));
        assert_eq!(s.decode(2).unwrap().get(0, 0), None);
    }

    #[test]
    fn builtin_pack_is_whole() {
        let pal = [[0u8; 3]; 256];
        let l = load(&builtin(), &pal);
        assert!(l.problems.is_empty(), "{:?}", l.problems);
    }
}
