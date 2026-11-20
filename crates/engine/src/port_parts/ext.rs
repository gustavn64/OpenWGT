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

/// What every extension chunk's tag starts with.
pub const PREFIX: [u8; 2] = *b"OW";

/// Whether a chunk tagged `tag` is one of the port's extension chunks.
#[must_use]
pub fn is_extension(tag: [u8; 4]) -> bool {
    tag[..2] == PREFIX
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;
    use crate::doc::{Document, new_gadget};
    use crate::gadget::Tag;
    use crate::port_parts::wgt::{BACK, hidden};
    use formats::wgt::{Chunk, Form};

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
