//! Widgets with network parts, saved where other readers don't see them
//! (`crate::port_parts::wgt`).

use crate::assets::Assets;
use crate::doc::{Document, new_gadget};
use crate::gadget::Tag;
use crate::port_parts::wgt::*;
use formats::wgt::{self, Chunk, Form};

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
    assert!(!tags.iter().any(|&t| is_hidden(t)));
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
    assert!(form.chunks.iter().any(|c| is_hidden(c.tag)));
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
