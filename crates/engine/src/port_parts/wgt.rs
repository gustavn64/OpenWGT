//! Saving widgets with the port's own parts (and its extension chunks,
//! [`super::ext`]) so that the original game,
//! and any other reader that doesn't know them, still opens them. The
//! original's loader makes a part of every chunk it doesn't know and has
//! no class for these (`NSnd`, `Scop`...: its factory gives nothing,
//! `0x41f194`, and the loader doesn't check, `0x425297`), so they go into
//! the file inside `back` chunks, which every reader takes as the
//! background and reads two bytes of. [`hide`] puts them there as the file is written, with the wires
//! that reach them; [`reveal`] takes them out where they were as it is
//! read.
//!
//! A hidden chunk is a `back` right after `fver`, before the real one (so
//! that the real one, read last, wins):
//!
//! ```text
//! 00 00        a background, for whoever reads it as one
//! "OWGT"       the port's
//! 01           this layout
//! u16          the chunk's place in the file as it was (big-endian)
//! tag, data    the chunk
//! ```
//!
//! A file with the parts' plain chunks reads as well.

use formats::wgt::{self, Chunk, Form};

pub(crate) const BACK: [u8; 4] = *b"back";
const MAGIC: &[u8; 4] = b"OWGT";
/// The hidden chunks' layout.
const LAYOUT: u8 = 1;
/// The bytes before the chunk's tag.
const HEAD: usize = 2 + 4 + 1 + 2;

/// Whether a chunk tagged `tag` is hidden: one of the port's own parts',
/// or one of its extension chunks ([`super::ext`]).
pub(crate) fn is_hidden(tag: [u8; 4]) -> bool {
    super::is_port_part(crate::gadget::Tag(tag)) || super::ext::is_extension(tag)
}

/// Hides the port's own parts of `form`, about to be written, and the
/// wires that reach them, in `back` chunks after `fver`.
pub fn hide(form: &mut Form) {
    let ids: Vec<u32> = form
        .chunks
        .iter()
        .filter(|c| is_hidden(c.tag))
        .filter_map(|c| wgt::be32(&c.data, 0).ok())
        .collect();
    if !form.chunks.iter().any(|c| is_hidden(c.tag)) {
        return;
    }
    let version = form.version().unwrap_or(1);
    let mut kept = Vec::with_capacity(form.chunks.len());
    let mut hidden = Vec::new();
    for (i, c) in std::mem::take(&mut form.chunks).into_iter().enumerate() {
        let wire_to_part = c.tag == *b"conn"
            && wgt::Connection::parse(&c.data, version).is_ok_and(|w| ids.contains(&w.from) || ids.contains(&w.to));
        if is_hidden(c.tag) || wire_to_part {
            let mut data = Vec::with_capacity(HEAD + 4 + c.data.len());
            data.extend_from_slice(&[0, 0]);
            data.extend_from_slice(MAGIC);
            data.push(LAYOUT);
            data.extend_from_slice(&u16::try_from(i).unwrap_or(u16::MAX).to_be_bytes());
            data.extend_from_slice(&c.tag);
            data.extend_from_slice(&c.data);
            hidden.push(Chunk { tag: BACK, data });
        } else {
            kept.push(c);
        }
    }
    let at = kept.iter().position(|c| c.tag == *b"fver").map_or(0, |i| i + 1);
    kept.splice(at..at, hidden);
    form.chunks = kept;
}

/// The chunk hidden in `c` and its place, if it's one [`hide`] made.
pub(crate) fn hidden(c: &Chunk) -> Option<(u16, Chunk)> {
    let d = &c.data;
    if c.tag != BACK || d.get(..2)? != [0, 0] || d.get(2..6)? != MAGIC || *d.get(6)? != LAYOUT {
        return None;
    }
    let at = u16::from_be_bytes([*d.get(7)?, *d.get(8)?]);
    let tag: [u8; 4] = d.get(HEAD..HEAD + 4)?.try_into().ok()?;
    Some((
        at,
        Chunk {
            tag,
            data: d[HEAD + 4..].to_vec(),
        },
    ))
}

/// Puts the chunks [`hide`] hid in `form`, just read, back where they were.
pub fn reveal(form: &mut Form) {
    let mut found = Vec::new();
    form.chunks.retain(|c| match hidden(c) {
        Some(h) => {
            found.push(h);
            false
        }
        None => true,
    });
    found.sort_by_key(|(at, _)| *at);
    for (at, c) in found {
        let at = usize::from(at).min(form.chunks.len());
        form.chunks.insert(at, c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Assets;
    use crate::doc::{Document, new_gadget};
    use crate::gadget::Tag;

    /// A Light Switch wired through an Oscilloscope to a Light Bulb, and
    /// a Metronome into its B; the Oscilloscope's range fixed.
    fn widget(a: &Assets) -> Document {
        let mut doc = Document::new();
        doc.background = 3;
        for (tag, h) in [(b"swit", 10), (b"Scop", 100), (b"bulb", 300), (b"Metr", 10)] {
            let id = doc.new_id();
            doc.gadgets
                .push(new_gadget(a, Tag(*tag), id, h, 20 + 100 * i32::from(tag == b"Metr")));
        }
        let s = &mut doc.gadgets[1].core.settings;
        s.radios = [1, 2, 0, 0];
        s.checks = 1;
        s.set_label(0, b"-3");
        s.set_label(1, b"3");
        doc.connect(0, 0, 1, 0, vec![(60, 30)], 0).unwrap();
        doc.connect(1, 3, 2, 0, vec![], 0).unwrap();
        doc.connect(3, 0, 1, 1, vec![], 0).unwrap();
        doc
    }

    #[test]
    fn the_oscilloscope_survives_saving_and_hides_from_other_readers() {
        let a = Assets::load(None, Some(&crate::opendata::builtin())).unwrap();
        let doc = widget(&a);
        let saved = doc.save();
        let back = Document::load(&a, &saved).unwrap();
        assert_eq!(back.save(), saved);
        let tags: Vec<Tag> = back.gadgets.iter().map(|g| g.core.tag).collect();
        assert_eq!(
            tags,
            [
                Tag::new(b"swit"),
                Tag::new(b"Scop"),
                Tag::new(b"bulb"),
                Tag::new(b"Metr")
            ]
        );
        assert_eq!(
            back.gadgets[1].core.settings.to_be(),
            doc.gadgets[1].core.settings.to_be()
        );
        assert_eq!(back.wires.len(), 3);
        assert_eq!(back.wires[0].points, [(60, 30)]);
        // In the file: the part and its three wires in backgrounds.
        let form = Form::parse(&saved).unwrap();
        let tags: Vec<[u8; 4]> = form.chunks.iter().map(|c| c.tag).collect();
        assert_eq!(&tags[..5], [*b"fver", BACK, BACK, BACK, BACK]);
        assert!(!tags.iter().any(|&t| is_hidden(t)));
        assert!(!tags.contains(b"conn"), "every wire reached it");
        // As a reader that doesn't know it: the other three parts, no
        // wires, the real background.
        let mut plain = form.clone();
        plain.chunks.retain(|c| hidden(c).is_none());
        let other = Document::load(&a, &plain.write()).unwrap();
        assert_eq!(other.gadgets.len(), 3);
        assert_eq!(other.wires.len(), 0);
        assert_eq!(other.background, 3);
        let mut background = None;
        for c in &form.chunks {
            if c.tag == BACK {
                background = wgt::be16(&c.data, 0).ok();
            }
        }
        assert_eq!(background, Some(3));
    }
}
