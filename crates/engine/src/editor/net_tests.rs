//! The network parts in the editor, on the port's own packs (no CD
//! needed): the tray's page, the new part's channel and the dialog's
//! Random.

use super::*;
use crate::assets::Packs;
use crate::lang::Lang;
use crate::net::parts::{RECEIVER, SENDER};

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

fn drag(e: &mut Editor, from: (i32, i32), to: (i32, i32)) {
    e.mouse_move(from.0, from.1);
    e.mouse_down(from.0, from.1, Button::Left);
    e.mouse_move(i32::midpoint(from.0, to.0), i32::midpoint(from.1, to.1));
    e.mouse_move(to.0, to.1);
    e.mouse_up(to.0, to.1, Button::Left);
    e.update(600, (0, 0, 0));
}

/// Every tray index in the folders.
fn indexes(e: &Editor) -> Vec<u8> {
    e.tray.folders.iter().flatten().flatten().map(|s| s.index).collect()
}

/// The slot of tray index `i` shown now, in frame coordinates.
fn slot_at(e: &Editor, i: u8) -> (i32, i32) {
    let s = e.tray.slots().into_iter().find(|s| s.index == i).unwrap();
    (
        e.tray.bar.pos.0 + tray::AREA.x0 + s.rect.x0 + 2,
        e.tray.bar.pos.1 + tray::AREA.y0 + s.rect.y0 + 2 + menu::BAR_H,
    )
}

/// Switches the tray to the Super Parts.
fn super_parts(e: &mut Editor) {
    let tab = e.tray.bar.get(toolbar::id::TAB0 + 6).unwrap().rect;
    let at = (
        e.tray.bar.pos.0 + tab.x0 + 5,
        e.tray.bar.pos.1 + tab.y0 + 5 + menu::BAR_H,
    );
    drag(e, at, at);
    e.update(400, (0, 0, 0));
    assert_eq!(e.tray.folder, crate::net::TRAY_FOLDER);
}

fn words(name: &str) -> bool {
    let parts: Vec<&str> = name.split('-').collect();
    parts.len() == 3 && parts.iter().all(|w| crate::net::pack::words().binary_search(w).is_ok())
}

#[test]
fn the_parts_are_in_the_tray_only_with_the_enhancements() {
    let mut e = editor(Lang::En);
    assert!(!e.enhanced());
    assert!(!indexes(&e).iter().any(|&i| i > 64));
    let plain = e.tray.folders.clone();
    e.set_enhanced(true);
    // A page of their own after the Super Parts', the others unchanged.
    let f = crate::net::TRAY_FOLDER;
    assert_eq!(e.tray.folders.len(), plain.len());
    for (k, (a, b)) in e.tray.folders.iter().zip(&plain).enumerate() {
        if k == f {
            assert_eq!(&a[..b.len()], &b[..]);
            assert_eq!(a.len(), b.len() + 1);
        } else {
            assert_eq!(a, b);
        }
    }
    let page = e.tray.folders[f].last().unwrap();
    assert_eq!(
        page.iter()
            .map(|s| (s.index, s.rect.width(), s.rect.height()))
            .collect::<Vec<_>>(),
        [(65, 44, 25), (66, 44, 25)]
    );
    // Off again: gone.
    e.set_enhanced(false);
    assert_eq!(e.tray.folders, plain);
}

#[test]
fn the_tray_names_and_draws_the_parts() {
    for lang in Lang::ALL {
        let mut e = editor(lang);
        e.set_enhanced(true);
        super_parts(&mut e);
        // The help line names them.
        for (i, want) in [(65, 2565), (66, 2566)] {
            let (x, y) = slot_at(&e, i);
            let p = (x - e.tray.bar.pos.0, y - menu::BAR_H - e.tray.bar.pos.1);
            assert_eq!(e.tray.hover_id(p), Some(want));
            e.tray.set_help(want, &e.assets);
            assert_eq!(Some(e.tray.help.clone()), e.assets.string(u32::try_from(want).unwrap()));
            assert_ne!(e.tray.help, "");
        }
        // Their pictures are drawn in their slots.
        e.draw();
        let px = |e: &Editor, i: u8| {
            let (x, y) = slot_at(e, i);
            let s = e.tray.slots().into_iter().find(|s| s.index == i).unwrap();
            let (w, h) = (s.rect.width(), s.rect.height());
            let width = e.surface.bounds().width();
            let mut colours = std::collections::BTreeSet::new();
            for dy in -2..h - 2 {
                for dx in -2..w - 2 {
                    let i = usize::try_from((y + dy) * width + x + dx).unwrap();
                    colours.insert(e.surface.pixels[i]);
                }
            }
            colours.len()
        };
        assert!(px(&e, 65) > 4 && px(&e, 66) > 4, "{lang:?}");
    }
}

#[test]
fn counts_and_limits_take_the_parts_too() {
    let mut e = editor(Lang::En);
    e.set_enhanced(true);
    e.tray.apply_limits(&[(SENDER, 2), (RECEIVER, 0)]);
    assert_eq!((e.tray.counts[65], e.tray.counts[66]), (2, 0));
    assert!(e.tray.limits().contains(&(SENDER, 2)));
    e.tray.reset_counts();
    assert_eq!(e.tray.counts.len(), tray::SLOTS);
    assert!(e.tray.counts.iter().all(|&c| c == -1));
}

#[test]
fn a_part_from_the_tray_comes_with_a_random_channel() {
    let mut e = editor(Lang::En);
    e.set_enhanced(true);
    super_parts(&mut e);
    for (n, (i, tag)) in [(65, SENDER), (66, RECEIVER)].into_iter().enumerate() {
        let at = slot_at(&e, i);
        drag(&mut e, at, (300 + 100 * i32::try_from(n).unwrap(), 450));
        let g = e.doc.gadgets.last().unwrap();
        assert_eq!(e.doc.gadgets.len(), n + 1);
        assert_eq!(g.core.tag, tag);
        let name = crate::net::parts::channel(&g.core);
        assert!(words(&name), "{name}");
        assert_eq!(g.core.settings.label_str(0), name);
    }
    // Each its own.
    let names: Vec<String> = e
        .doc
        .gadgets
        .iter()
        .map(|g| crate::net::parts::channel(&g.core))
        .collect();
    assert_ne!(names[0], names[1]);
    // The game's parts get nothing new.
    let n = e.doc.gadgets.len();
    let at = slot_at(&e, e.tray.folders[crate::net::TRAY_FOLDER][0][0].index);
    drag(&mut e, at, (300, 300));
    assert_eq!(e.doc.gadgets.len(), n + 1);
    assert_ne!(e.doc.gadgets[n].core.tag, SENDER);
    assert!(!words(&e.doc.gadgets[n].core.settings.label_str(0)));
}

/// A click on control `id` of the open dialog.
fn click_control(e: &mut Editor, id: u16) {
    let (x, y) = e.dialog.as_ref().unwrap().centre_of(id).unwrap();
    e.mouse_move(x, y);
    e.mouse_down(x, y, Button::Left);
    e.mouse_up(x, y, Button::Left);
}

#[test]
fn random_in_the_dialog_gives_a_new_name() {
    for lang in Lang::ALL {
        for tag in [SENDER, RECEIVER] {
            let mut e = editor(lang);
            let id = e.doc.new_id();
            let mut g = doc::new_gadget(&e.assets, tag, id, 100, 200);
            g.core.settings.set_label(0, b"cat-sun-oak");
            e.doc.gadgets.push(g);
            // Cancel: the name stays.
            e.open_settings(0);
            assert!(e.dialog.is_some(), "{lang:?} {tag}");
            click_control(&mut e, 107);
            click_control(&mut e, 2);
            assert!(e.dialog.is_none());
            assert_eq!(e.doc.gadgets[0].core.settings.label(0), b"cat-sun-oak");
            // OK: a new one each time.
            let mut seen = Vec::new();
            for _ in 0..3 {
                e.open_settings(0);
                click_control(&mut e, 107);
                click_control(&mut e, 1);
                assert!(e.dialog.is_none());
                seen.push(e.doc.gadgets[0].core.settings.label_str(0));
            }
            assert!(seen.iter().all(|n| words(n)), "{lang:?} {seen:?}");
            assert!(seen[0] != seen[1] || seen[1] != seen[2], "{seen:?}");
        }
    }
}

#[test]
fn a_typed_name_is_cleaned_on_ok() {
    let mut e = editor(Lang::En);
    let id = e.doc.new_id();
    let g = doc::new_gadget(&e.assets, SENDER, id, 100, 200);
    e.doc.gadgets.push(g);
    e.open_settings(0);
    click_control(&mut e, 106);
    for k in [b'C', b'A', b'T', 0x20, b'S', b'U', b'N'] {
        e.key_down(u16::from(k));
        e.key_up(u16::from(k));
    }
    click_control(&mut e, 1);
    assert_eq!(e.doc.gadgets[0].core.settings.label(0), b"cat-sun");
}
