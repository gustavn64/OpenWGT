//! Saving widgets with network parts so that the original game, and any
//! other reader that doesn't know them, still opens them. The original's
//! loader makes a part of every chunk it doesn't know and has no class for
//! `NSnd` or `NRcv` (its factory gives nothing, `0x41f194`, and the loader
//! doesn't check, `0x425297`), so they go into the file inside `back`
//! chunks, which every reader takes as the background and reads two bytes
//! of. [`hide`] puts them there as the file is written, with the wires
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
//! A file with plain `NSnd` and `NRcv` chunks reads as well.

use formats::wgt::{self, Chunk, Form};

use super::parts::{RECEIVER, SENDER};

const BACK: [u8; 4] = *b"back";
const MAGIC: &[u8; 4] = b"OWGT";
/// The hidden chunks' layout.
const LAYOUT: u8 = 1;
/// The bytes before the chunk's tag.
const HEAD: usize = 2 + 4 + 1 + 2;

fn is_net(tag: [u8; 4]) -> bool {
    tag == SENDER.0 || tag == RECEIVER.0
}

/// Hides the network parts of `form`, about to be written, and the wires
/// that reach them, in `back` chunks after `fver`.
pub fn hide(form: &mut Form) {
    let ids: Vec<u32> = form
        .chunks
        .iter()
        .filter(|c| is_net(c.tag))
        .filter_map(|c| wgt::be32(&c.data, 0).ok())
        .collect();
    if !form.chunks.iter().any(|c| is_net(c.tag)) {
        return;
    }
    let version = form.version().unwrap_or(1);
    let mut kept = Vec::with_capacity(form.chunks.len());
    let mut hidden = Vec::new();
    for (i, c) in std::mem::take(&mut form.chunks).into_iter().enumerate() {
        let wire_to_net = c.tag == *b"conn"
            && wgt::Connection::parse(&c.data, version).is_ok_and(|w| ids.contains(&w.from) || ids.contains(&w.to));
        if is_net(c.tag) || wire_to_net {
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
fn hidden(c: &Chunk) -> Option<(u16, Chunk)> {
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

    /// The parts' knots: OpenWDATA's tables with the network parts'.
    fn assets() -> Assets {
        Assets::load(None, Some(&crate::opendata::builtin())).unwrap()
    }

    /// A Switch wired to a Sender, a Receiver wired to a Light Bulb, and a
    /// Switch wired to a Light Bulb between them.
    fn widget(a: &Assets) -> Document {
        let mut doc = Document::new();
        doc.background = 7;
        for (tag, h) in [
            (b"swit", 10),
            (b"NSnd", 100),
            (b"swit", 200),
            (b"bulb", 300),
            (b"NRcv", 400),
            (b"bulb", 500),
        ] {
            let id = doc.new_id();
            let mut g = new_gadget(a, Tag(*tag), id, h, 20);
            g.core.settings.set_label(0, format!("chan{id}").as_bytes());
            doc.gadgets.push(g);
        }
        // Switch → Sender, Switch → Light Bulb, Receiver → Light Bulb.
        doc.connect(0, 0, 1, 0, vec![(50, 30)], 0).unwrap();
        doc.connect(2, 0, 3, 0, vec![], 0).unwrap();
        doc.connect(4, 0, 5, 0, vec![(450, 40), (460, 50)], 0).unwrap();
        doc
    }

    /// The widget's parts and wires, as ids and numbers.
    fn shape(
        doc: &Document,
    ) -> (
        Vec<(Tag, u32, i32, i32, Vec<u8>)>,
        Vec<(u32, usize, u32, usize, Vec<(i32, i32)>)>,
        u16,
    ) {
        let gadgets = doc
            .gadgets
            .iter()
            .map(|g| (g.core.tag, g.core.id, g.core.h, g.core.v, g.core.settings.to_be()))
            .collect();
        let wires = doc
            .wires
            .iter()
            .map(|w| {
                let id = |i: usize| doc.gadgets[i].core.id;
                (id(w.from), w.from_knot, id(w.to), w.to_knot, w.points.clone())
            })
            .collect();
        (gadgets, wires, doc.background)
    }

    #[test]
    fn network_parts_survive_saving() {
        let a = assets();
        let doc = widget(&a);
        assert_eq!(doc.wires.len(), 3);
        let saved = doc.save();
        let back = Document::load(&a, &saved).unwrap();
        assert_eq!(shape(&back), shape(&doc));
        // And again, the same bytes.
        assert_eq!(back.save(), saved);
    }

    #[test]
    fn the_file_hides_them_from_other_readers() {
        let a = assets();
        let doc = widget(&a);
        let form = Form::parse(&doc.save()).unwrap();
        let tags: Vec<[u8; 4]> = form.chunks.iter().map(|c| c.tag).collect();
        // The two parts, their two wires, then the real background.
        assert_eq!(&tags[..6], [*b"fver", BACK, BACK, BACK, BACK, BACK]);
        assert!(!tags.iter().any(|&t| is_net(t)));
        // As the original's loader reads it: every part a class it knows,
        // every wire between parts it has, the background the real one.
        let mut background = None;
        let mut ids = Vec::new();
        let mut conns = Vec::new();
        for c in &form.chunks {
            match &c.tag {
                b"fver" | b"numG" => {}
                b"back" => background = wgt::be16(&c.data, 0).ok(),
                b"conn" => conns.push(wgt::Connection::parse(&c.data, 4).unwrap()),
                tag => {
                    assert!(crate::gadgets::CLASSES.iter().any(|k| k.tag.0 == *tag), "{tag:?}");
                    ids.push(wgt::be32(&c.data, 0).unwrap());
                }
            }
        }
        assert_eq!(background, Some(7));
        assert_eq!(ids, [1, 3, 4, 6]);
        assert_eq!(conns.len(), 1);
        assert!(conns.iter().all(|w| ids.contains(&w.from) && ids.contains(&w.to)));
        // A loader that doesn't know them (this one, without `reveal`, as
        // upstream's) opens it with the rest.
        let mut plain = form.clone();
        plain.chunks.retain(|c| hidden(c).is_none());
        let other = Document::load(&a, &plain.write()).unwrap();
        assert_eq!(other.gadgets.len(), 4);
        assert_eq!(other.wires.len(), 1);
        assert_eq!(other.background, 7);
    }

    #[test]
    fn plain_chunks_and_odd_ones_read_too() {
        let a = assets();
        let doc = widget(&a);
        // Written without hiding: read as they are.
        let mut form = Form::parse(&doc.save()).unwrap();
        reveal(&mut form);
        assert!(form.chunks.iter().any(|c| is_net(c.tag)));
        let back = Document::load(&a, &form.write()).unwrap();
        assert_eq!(shape(&back), shape(&doc));
        // A `back` that only looks like one of ours stays a background.
        let mut odd = Form::default();
        for data in [b"\0\0OWGT\x02\0\0NSnd".to_vec(), b"\0\0OWGT\x01\0".to_vec(), vec![0, 9]] {
            odd.chunks.push(Chunk { tag: BACK, data });
        }
        let before = odd.clone();
        reveal(&mut odd);
        assert_eq!(odd, before);
        // One whose place is past the end goes last.
        let mut late = Form::default();
        late.chunks.push(Chunk {
            tag: BACK,
            data: b"\0\0OWGT\x01\x01\x00NRcvdata".to_vec(),
        });
        late.chunks.push(Chunk {
            tag: BACK,
            data: vec![0, 3],
        });
        reveal(&mut late);
        assert_eq!(
            late.chunks[1],
            Chunk {
                tag: *b"NRcv",
                data: b"data".to_vec()
            }
        );
        // Nothing to hide: the file is as it was.
        let mut none = Form::parse(&Document::new().save()).unwrap();
        let before = none.clone();
        hide(&mut none);
        assert_eq!(none, before);
    }
}
