//! The open parts in the editor, on the port's own packs (no CD needed):
//! the Oscilloscope in the tray, its settings dialog and a running widget.

use super::*;
use crate::assets::Packs;
use crate::lang::Lang;
use crate::openparts::scope::{COLOURS, Oscilloscope, PLOT, SCOPE};

fn editor(lang: Lang) -> Editor {
    let graphics = crate::opengfx::builtin_for(lang);
    let sounds = crate::opensfx::builtin();
    let data = crate::opendata::builtin_for(lang);
    let packs = Packs {
        graphics: Some(graphics.as_ref()),
        sounds: Some(&sounds),
        data: Some(&data),
    };
    let mut assets = Assets::load_with(None, packs, None).unwrap();
    assets.lang = lang;
    Editor::new(assets, Document::new(), 800, 600)
}

/// The slot of tray index `i` shown now, in frame coordinates.
fn slot_at(e: &Editor, i: u8) -> Option<(i32, i32)> {
    let s = e.tray.slots().into_iter().find(|s| s.index == i)?;
    Some((
        e.tray.bar.pos.0 + tray::AREA.x0 + s.rect.x0 + 2,
        e.tray.bar.pos.1 + tray::AREA.y0 + s.rect.y0 + 2 + menu::BAR_H,
    ))
}

fn drag(e: &mut Editor, from: (i32, i32), to: (i32, i32)) {
    e.mouse_move(from.0, from.1);
    e.mouse_down(from.0, from.1, Button::Left);
    e.mouse_move(i32::midpoint(from.0, to.0), i32::midpoint(from.1, to.1));
    e.mouse_move(to.0, to.1);
    e.mouse_up(to.0, to.1, Button::Left);
    e.update(600, (0, 0, 0));
}

fn indexes(folders: &[Vec<Vec<tray::Slot>>]) -> Vec<Vec<u8>> {
    folders
        .iter()
        .map(|f| f.iter().flatten().map(|s| s.index).collect())
        .collect()
}

#[test]
fn the_oscilloscope_is_in_the_tray_only_with_the_enhancements() {
    let mut e = editor(Lang::En);
    assert!(!e.enhanced());
    let plain = e.tray.folders.clone();
    assert!(!indexes(&plain).iter().flatten().any(|&i| i == 67));
    e.set_enhanced(true);
    let f = crate::openparts::SCOPE_FOLDER;
    // Last in the Strip Chart's folder; the game's parts where they were.
    let with = indexes(&e.tray.folders);
    let without = indexes(&plain);
    assert_eq!(with[f].last(), Some(&67));
    assert_eq!(with[f][..without[f].len()], without[f][..]);
    let game = |l: &[u8]| l.iter().copied().filter(|&i| i <= 64).collect::<Vec<_>>();
    for (a, b) in with.iter().zip(&without) {
        assert_eq!(game(a), game(b));
    }
    assert_eq!(e.tray.folders[f].len(), plain[f].len(), "it fits the last page");
    let slot = e.tray.folders[f].iter().flatten().find(|s| s.index == 67).unwrap();
    assert_eq!((slot.rect.width(), slot.rect.height()), (50, 34));
    e.set_enhanced(false);
    assert_eq!(e.tray.folders, plain);
}

#[test]
fn the_tray_names_draws_and_gives_it() {
    for lang in Lang::ALL {
        let mut e = editor(lang);
        e.set_enhanced(true);
        assert_eq!(e.tray.folder, crate::openparts::SCOPE_FOLDER);
        let (x, y) = slot_at(&e, 67).unwrap();
        let p = (x - e.tray.bar.pos.0, y - menu::BAR_H - e.tray.bar.pos.1);
        assert_eq!(e.tray.hover_id(p), Some(2567));
        let name = e.assets.string(2567).unwrap();
        assert_eq!(
            name,
            if lang == Lang::Sv {
                "Oscilloskop"
            } else {
                "Oscilloscope"
            }
        );
        e.draw();
        let width = e.surface.bounds().width();
        let mut colours = std::collections::BTreeSet::new();
        for dy in 0..30 {
            for dx in 0..40 {
                colours.insert(e.surface.pixels[usize::try_from((y + dy) * width + x + dx).unwrap()]);
            }
        }
        assert!(colours.len() > 6, "{lang:?}");
        drag(&mut e, (x, y), (300, 400));
        assert_eq!(e.doc.gadgets.len(), 1);
        let g = &e.doc.gadgets[0];
        assert_eq!(g.core.tag, SCOPE);
        assert_eq!(
            (g.core.settings.label_str(0), g.core.settings.label_str(1)),
            ("0".into(), "100".into())
        );
    }
}

/// A click on control `id` of the open dialog.
fn click_control(e: &mut Editor, id: u16) {
    let (x, y) = e.dialog.as_ref().unwrap().centre_of(id).unwrap();
    e.mouse_move(x, y);
    e.mouse_down(x, y, Button::Left);
    e.mouse_up(x, y, Button::Left);
}

fn checked(e: &Editor, id: u16) -> bool {
    e.dialog.as_ref().unwrap().control(id).unwrap().0
}

fn text(e: &Editor, id: u16) -> String {
    e.dialog.as_ref().unwrap().control(id).unwrap().1.to_owned()
}

/// Replaces edit `id`'s text with `typed`, as the player does: a click
/// in it, everything selected, then typed over.
fn retype(e: &mut Editor, id: u16, typed: &str) {
    click_control(e, id);
    e.dialog.as_mut().unwrap().select_all();
    e.text_input(typed);
}

#[test]
fn the_dialog_keeps_the_settings() {
    for lang in Lang::ALL {
        let mut e = editor(lang);
        let id = e.doc.new_id();
        let g = doc::new_gadget(&e.assets, SCOPE, id, 100, 200);
        e.doc.gadgets.push(g);
        e.open_settings(0);
        assert!(e.dialog.is_some(), "{lang:?}");
        // As new: automatic, 1×, lines, From 0 to 100.
        assert!(checked(&e, 106) && !checked(&e, 107));
        assert!(checked(&e, 112) && !checked(&e, 115) && !checked(&e, 116));
        assert_eq!((text(&e, 108), text(&e, 110)), ("0".into(), "100".into()));
        // Typing a number fixes the range.
        retype(&mut e, 108, "-5");
        assert!(checked(&e, 107) && !checked(&e, 106), "{lang:?}");
        retype(&mut e, 110, "2.5");
        click_control(&mut e, 115);
        click_control(&mut e, 116);
        click_control(&mut e, 1);
        assert!(e.dialog.is_none());
        let s = &e.doc.gadgets[0].core.settings;
        assert_eq!((s.radios[0], s.radios[1], s.checks & 1), (1, 3, 1), "{lang:?}");
        assert_eq!((s.label_str(0), s.label_str(1)), ("-5".into(), "2.5".into()));
        assert_eq!(crate::openparts::scope::fixed_range(s), Some((-5.0, 2.5)));
        // Again: as it was left; Cancel changes nothing.
        e.open_settings(0);
        assert!(checked(&e, 107) && checked(&e, 115) && checked(&e, 116));
        assert_eq!((text(&e, 108), text(&e, 110)), ("-5".into(), "2.5".into()));
        click_control(&mut e, 106);
        click_control(&mut e, 112);
        click_control(&mut e, 2);
        let s = &e.doc.gadgets[0].core.settings;
        assert_eq!((s.radios[0], s.radios[1]), (1, 3));
    }
}

/// A Metronome through a Random Number Generator into A, another into B,
/// A out to a Three-Digit Display.
fn widget(a: &Assets) -> Document {
    let mut doc = Document::new();
    for (tag, h, v) in [
        (b"Metr", 20, 180),
        (b"rand", 120, 180),
        (b"Metr", 20, 330),
        (b"Scop", 300, 250),
        (b"disp", 500, 250),
    ] {
        let id = doc.new_id();
        doc.gadgets.push(doc::new_gadget(a, crate::gadget::Tag(*tag), id, h, v));
    }
    doc.connect(0, 0, 1, 0, vec![], 0).unwrap();
    doc.connect(1, 1, 3, 0, vec![], 0).unwrap();
    doc.connect(2, 0, 3, 1, vec![], 0).unwrap();
    doc.connect(3, 3, 4, 0, vec![], 0).unwrap();
    doc
}

#[test]
fn it_runs_and_draws_in_a_widget() {
    let mut e = editor(Lang::En);
    let doc = widget(&e.assets);
    e.doc = doc;
    assert_eq!(e.doc.wires.len(), 4);
    e.run();
    for _ in 0..300 {
        e.update(1000 / 60 + 1, (0, 0, 0));
    }
    let s = e.doc.gadgets[3]
        .behavior
        .as_any()
        .downcast_ref::<Oscilloscope>()
        .unwrap();
    assert_eq!(s.columns().len(), crate::openparts::scope::COLUMNS);
    assert!(s.spans(0).iter().all(Option::is_some) && s.spans(1).iter().all(Option::is_some));
    // The display shows A.
    let a = e.doc.gadgets[1].behavior.output(&e.doc.gadgets[1].core, 1);
    assert_eq!(e.doc.gadgets[3].behavior.output(&e.doc.gadgets[3].core, 3), a);
    e.draw();
    // Both colours on the screen.
    let g = &e.doc.gadgets[3].core;
    let pal = e.assets.palette();
    let ink = COLOURS.map(|c| crate::gadget::nearest(&pal, c));
    let (px, py, pw, ph) = PLOT;
    let (ox, oy) = (g.h + px, g.v + py + menu::BAR_H);
    let width = e.surface.bounds().width();
    let mut seen = [0; 2];
    for y in oy..oy + ph {
        for x in ox..ox + pw {
            let c = e.surface.pixels[usize::try_from(y * width + x).unwrap()];
            for k in 0..2 {
                seen[k] += usize::from(c == ink[k]);
            }
        }
    }
    assert!(seen[0] > 50 && seen[1] > 50, "{seen:?}");
}
