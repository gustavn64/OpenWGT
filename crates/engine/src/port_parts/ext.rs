//! The port's own chunks in a widget that aren't parts: what it keeps
//! about a widget beside the game's content. Their tags start with `OW`,
//! which no part of the game's or the families' has. A widget keeps
//! them in [`crate::doc::Document::ext`] as they were read, writes them
//! after the game's own chunks, and [`super::wgt`] hides them from the
//! original game's loader as it hides the port's parts (the original
//! would make a part of each, and probably crash on it).
//!
//! A chunk this version of the port doesn't know is kept as it was, so a
//! widget from a later version keeps what that one put in it.
//!
//! The ones it knows, big-endian as the game's:
//!
//! ```text
//! "OWnm"  the parts' names (the debugging tools'), once:
//!         01                 this layout
//!         then for each part, by id:
//!         u32 id, u16 length, the name (UTF-8)
//! "OWsc"  a script that comes with the widget, one chunk each, in order:
//!         01                 this layout
//!         u8 length, its language (ASCII: "JavaScript", "commands")
//!         u16 length, its name (UTF-8)
//!         its text (UTF-8), to the chunk's end
//! ```

use std::collections::BTreeMap;

use formats::wgt::Chunk;

/// What every extension chunk's tag starts with.
pub const PREFIX: [u8; 2] = *b"OW";

/// Whether a chunk tagged `tag` is one of the port's extension chunks.
#[must_use]
pub fn is_extension(tag: [u8; 4]) -> bool {
    tag[..2] == PREFIX
}

/// The parts' names.
pub const NAMES: [u8; 4] = *b"OWnm";
/// A script.
pub const SCRIPT: [u8; 4] = *b"OWsc";
/// The layout of the chunks this writes.
const LAYOUT: u8 = 1;

/// A script that comes with a widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub name: String,
    /// The language it's in, as its runtime calls it
    /// (`Runtime::language`: `JavaScript`, `commands`).
    pub language: String,
    pub source: String,
}

impl Script {
    /// Its text's digest, which the player's trust is kept by.
    #[must_use]
    pub fn digest(&self) -> crate::script::trust::Digest {
        crate::script::trust::Digest::of(&self.source)
    }
}

/// The text's length, a byte (`short`) or a big-endian `u16`, then the
/// text, cut to fit at a character.
fn put_text(out: &mut Vec<u8>, text: &str, short: bool) {
    let max = if short {
        usize::from(u8::MAX)
    } else {
        usize::from(u16::MAX)
    };
    let mut end = text.len().min(max);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let bytes = &text.as_bytes()[..end];
    if short {
        out.push(u8::try_from(bytes.len()).unwrap_or(0));
    } else {
        out.extend_from_slice(&u16::try_from(bytes.len()).unwrap_or(0).to_be_bytes());
    }
    out.extend_from_slice(bytes);
}

/// The `OWnm` chunk of `names`, if there's one to write.
#[must_use]
pub fn names_chunk(names: &BTreeMap<u32, String>) -> Option<Chunk> {
    if names.is_empty() {
        return None;
    }
    let mut data = vec![LAYOUT];
    for (id, name) in names {
        data.extend_from_slice(&id.to_be_bytes());
        put_text(&mut data, name, false);
    }
    Some(Chunk { tag: NAMES, data })
}

/// The names in an `OWnm` chunk's data, if it reads.
#[must_use]
pub fn read_names(d: &[u8]) -> Option<BTreeMap<u32, String>> {
    if *d.first()? != LAYOUT {
        return None;
    }
    let mut names = BTreeMap::new();
    let mut at = 1;
    while at < d.len() {
        let id = u32::from_be_bytes(d.get(at..at + 4)?.try_into().ok()?);
        let n = usize::from(u16::from_be_bytes(d.get(at + 4..at + 6)?.try_into().ok()?));
        let name = std::str::from_utf8(d.get(at + 6..at + 6 + n)?).ok()?;
        names.insert(id, name.to_owned());
        at += 6 + n;
    }
    Some(names)
}

/// A script's `OWsc` chunk.
#[must_use]
pub fn script_chunk(s: &Script) -> Chunk {
    let mut data = vec![LAYOUT];
    put_text(&mut data, &s.language, true);
    put_text(&mut data, &s.name, false);
    data.extend_from_slice(s.source.as_bytes());
    Chunk { tag: SCRIPT, data }
}

/// The script in an `OWsc` chunk's data, if it reads.
#[must_use]
pub fn read_script(d: &[u8]) -> Option<Script> {
    if *d.first()? != LAYOUT {
        return None;
    }
    let ln = usize::from(*d.get(1)?);
    let language = std::str::from_utf8(d.get(2..2 + ln)?).ok()?.to_owned();
    let at = 2 + ln;
    let nn = usize::from(u16::from_be_bytes(d.get(at..at + 2)?.try_into().ok()?));
    let name = std::str::from_utf8(d.get(at + 2..at + 2 + nn)?).ok()?.to_owned();
    let source = std::str::from_utf8(d.get(at + 2 + nn..)?).ok()?.to_owned();
    Some(Script { name, language, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;
    use crate::doc::{Document, new_gadget};
    use crate::gadget::Tag;
    use crate::port_parts::wgt::{BACK, hidden};
    use formats::wgt::Form;

    fn widget(a: &Assets) -> Document {
        let mut doc = Document::new();
        doc.background = 2;
        for (tag, h) in [(b"swit", 10), (b"bulb", 200)] {
            let id = doc.new_id();
            doc.gadgets.push(new_gadget(a, Tag(*tag), id, h, 30));
        }
        doc.connect(0, 0, 1, 0, vec![], 0).unwrap();
        doc.ext.push(Chunk {
            tag: *b"OWzz",
            data: b"from a later version".to_vec(),
        });
        doc.ext.push(Chunk {
            tag: *b"OWyy",
            data: vec![0, 1, 2, 255],
        });
        doc
    }

    #[test]
    fn extension_chunks_survive_saving_byte_for_byte() {
        let a = Assets::load(None, Some(&crate::opendata::builtin())).unwrap();
        let doc = widget(&a);
        let saved = doc.save();
        let back = Document::load(&a, &saved).unwrap();
        assert_eq!(back.ext, doc.ext);
        assert_eq!(back.gadgets.len(), 2, "no part made of them");
        assert_eq!(back.save(), saved);
        // Hidden in backgrounds right after `fver`, the real one last.
        let form = Form::parse(&saved).unwrap();
        let tags: Vec<[u8; 4]> = form.chunks.iter().map(|c| c.tag).collect();
        assert_eq!(&tags[..3], [*b"fver", BACK, BACK]);
        assert!(!tags.iter().any(|&t| is_extension(t)));
        assert_eq!(form.chunks.iter().filter(|c| hidden(c).is_some()).count(), 2);
    }

    #[test]
    fn a_reader_that_doesnt_know_them_sees_only_the_game() {
        let a = Assets::load(None, Some(&crate::opendata::builtin())).unwrap();
        let doc = widget(&a);
        let saved = doc.save();
        let form = Form::parse(&saved).unwrap();
        // The original's way: every chunk read as it is; a `back` is a
        // background (its first two bytes), the last one wins.
        let mut background = None;
        let mut parts = Vec::new();
        for c in &form.chunks {
            match &c.tag {
                b"fver" | b"conn" | b"numG" => {}
                b"back" => background = formats::wgt::be16(&c.data, 0).ok(),
                t => parts.push(*t),
            }
        }
        assert_eq!(background, Some(2));
        assert_eq!(parts, [*b"swit", *b"bulb"]);
        // A reader that skips the hidden chunks: the same widget, without
        // the port's chunks.
        let mut plain = form.clone();
        plain.chunks.retain(|c| hidden(c).is_none());
        let other = Document::load(&a, &plain.write()).unwrap();
        assert_eq!(other.ext, []);
        assert_eq!(other.gadgets.len(), 2);
        assert_eq!(other.wires.len(), 1);
        assert_eq!(other.background, 2);
    }

    #[test]
    fn names_and_scripts_read_back() {
        let mut names = BTreeMap::new();
        names.insert(7, "Lamp".to_owned());
        names.insert(70_000, "Växel ✓".to_owned());
        let c = names_chunk(&names).unwrap();
        assert_eq!(c.data[..11], [1, 0, 0, 0, 7, 0, 4, b'L', b'a', b'm', b'p']);
        assert_eq!(read_names(&c.data), Some(names));
        assert_eq!(names_chunk(&BTreeMap::new()), None);
        let s = Script {
            name: "blink.js".into(),
            language: "JavaScript".into(),
            source: "run();\nlog(\"å\");\n".into(),
        };
        let c = script_chunk(&s);
        assert_eq!(c.data[..12], *b"\x01\x0aJavaScript");
        assert_eq!(read_script(&c.data), Some(s));
        // What doesn't read: none.
        assert_eq!(read_script(&[2, 0, 0, 0]), None);
        assert_eq!(read_script(&[1, 9, b'x']), None);
        assert_eq!(read_names(&[1, 0, 0, 0, 7, 0, 9, b'x']), None);
    }

    #[test]
    fn plain_extension_chunks_read_too() {
        let a = Assets::load(None, Some(&crate::opendata::builtin())).unwrap();
        let mut form = Form::parse(&Document::new().save()).unwrap();
        form.chunks.push(Chunk {
            tag: *b"OWzz",
            data: vec![7],
        });
        let doc = Document::load(&a, &form.write()).unwrap();
        assert!(doc.gadgets.is_empty());
        assert_eq!(doc.ext.len(), 1);
    }
}
