//! Saving widgets with the port's own parts so that the original game,
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

/// Whether a chunk tagged `tag` is one of the port's own parts'.
pub(crate) fn is_hidden(tag: [u8; 4]) -> bool {
    super::is_port_part(crate::gadget::Tag(tag))
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
