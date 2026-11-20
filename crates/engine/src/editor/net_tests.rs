//! The network parts in the editor, on the port's own packs (no CD
//! needed): the tray's page, the new part's channel, the dialog's Random,
//! and the parts talking to a relay in memory.

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
    // A page of their own after the Super Parts', the others without
    // them (other families' parts may come too).
    let f = crate::net::TRAY_FOLDER;
    assert_eq!(e.tray.folders.len(), plain.len());
    let ours = |s: &tray::Slot| crate::net::tray_tag(s.index).is_some();
    for (k, (a, b)) in e.tray.folders.iter().zip(&plain).enumerate() {
        if k == f {
            assert_eq!(&a[..b.len()], &b[..]);
            assert_eq!(a.len(), b.len() + 1);
        } else {
            assert!(!a.iter().flatten().any(ours));
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
    e.text_input("Cat Sun");
    click_control(&mut e, 1);
    assert_eq!(e.doc.gadgets[0].core.settings.label(0), b"cat-sun");
}

/// Adds a part of class `tag` with `label` as its first text.
fn add(e: &mut Editor, tag: crate::gadget::Tag, label: &str, h: i32) -> usize {
    let id = e.doc.new_id();
    let mut g = doc::new_gadget(&e.assets, tag, id, h, 300);
    g.core.settings.set_label(0, label.as_bytes());
    g.behavior.settings_changed(&mut g.core);
    e.doc.gadgets.push(g);
    e.doc.gadgets.len() - 1
}

fn status(e: &mut Editor, g: usize) -> crate::net::Status {
    let any = e.doc.gadgets[g].behavior.as_any_mut();
    if let Some(s) = any.downcast_mut::<crate::net::parts::Sender>() {
        s.status
    } else {
        any.downcast_mut::<crate::net::parts::Receiver>().unwrap().status
    }
}

#[test]
fn the_editor_keeps_the_parts_talking() {
    use crate::net::SocketEvent;
    let mut e = editor(Lang::En);
    let fake = crate::net::fake::Fake::default();
    e.net
        .set_transport(Box::new(fake.clone()), "wss://relay.test/openwgt/relay");
    // A Constant of 42 wired into a Sender; a Receiver.
    let c = add(&mut e, crate::gadget::Tag::new(b"cnst"), "42", 100);
    let s = add(&mut e, SENDER, "cat-sun-oak", 250);
    let r = add(&mut e, RECEIVER, "dog-moon-elm", 400);
    let out = e.doc.gadgets[c].core.knots.iter().position(|k| !k.is_input()).unwrap();
    e.doc.connect(c, out, s, 0, Vec::new(), 0).unwrap();
    // Open: connected at once.
    e.update(16, (0, 0, 0));
    assert_eq!(fake.count(), 2);
    assert_eq!(fake.url(0), "wss://relay.test/openwgt/relay/cat-sun-oak");
    assert_eq!(fake.url(1), "wss://relay.test/openwgt/relay/dog-moon-elm");
    fake.event(0, SocketEvent::Open);
    fake.say(0, r#"{"t":"busy","busy":false}"#);
    fake.say(0, r#"{"t":"peer","s":"open"}"#);
    fake.event(1, SocketEvent::Open);
    fake.say(1, r#"{"t":"peer","s":"run"}"#);
    fake.say(1, r#"{"t":"val","k":"n","v":7}"#);
    e.update(16, (0, 0, 0));
    assert_eq!(fake.sent(0), [r#"{"t":"hello","role":"send","run":false,"v":1}"#]);
    assert_eq!(fake.sent(1), [r#"{"t":"hello","role":"recv","run":false,"v":1}"#]);
    assert_eq!((status(&mut e, s).lamp(), status(&mut e, r).lamp()), (3, 4));
    // GO: the relay hears it and the Sender's value; the Receiver gives
    // what came.
    e.run();
    e.update(20, (0, 0, 0));
    assert_eq!(
        fake.sent(0),
        [r#"{"t":"run","run":true}"#, r#"{"t":"val","k":"n","v":42.0}"#]
    );
    assert_eq!(fake.sent(1), [r#"{"t":"run","run":true}"#]);
    assert_eq!(e.doc.gadgets[r].core.number, 7.0);
    fake.say(1, r#"{"t":"val","k":"b","v":true}"#);
    e.update(20, (0, 0, 0));
    assert!(e.doc.gadgets[r].core.state);
    // The input changes: sent on the next frame.
    let g = &mut e.doc.gadgets[c];
    g.core.settings.set_label(0, b"43");
    g.behavior.settings_changed(&mut g.core);
    e.update(20, (0, 0, 0));
    e.update(20, (0, 0, 0));
    assert_eq!(fake.sent(0), [r#"{"t":"val","k":"n","v":43.0}"#]);
    // STOP.
    e.stop();
    e.update(20, (0, 0, 0));
    assert_eq!(fake.sent(0), [r#"{"t":"run","run":false}"#]);
    // Undo makes the parts again: they get their state back, on the
    // same connections.
    e.snapshot();
    e.undo();
    assert_eq!(status(&mut e, r).lamp(), 1);
    e.update(16, (0, 0, 0));
    assert_eq!((status(&mut e, s).lamp(), status(&mut e, r).lamp()), (3, 4));
    let rcv = e.doc.gadgets[r]
        .behavior
        .as_any()
        .downcast_ref::<crate::net::parts::Receiver>()
        .unwrap();
    assert_eq!(rcv.inbox, Some(crate::net::NetValue::Bool(true)));
    assert_eq!(fake.count(), 2);
    // Deleted: its connection closes.
    e.delete_gadget(r);
    e.update(16, (0, 0, 0));
    assert!(fake.dropped(1) && !fake.dropped(0));
    // Another widget: the old one's go.
    e.install(Document::new(), None);
    e.update(16, (0, 0, 0));
    assert!(fake.dropped(0));
}

/// A widget link for the browser test of a Sender
/// (tools/opennet/browser-sender.mjs, docs/net-parts.md):
/// a Mini Number of 42 wired to a Sender on `$SENDER_CHANNEL`, for the page
/// `$SENDER_PAGE`; written to `$SENDER_LINK`.
#[test]
#[ignore = "writes a link for the browser test"]
fn browser_sender_link() {
    let (Ok(channel), Ok(page), Ok(out)) = (
        std::env::var("SENDER_CHANNEL"),
        std::env::var("SENDER_PAGE"),
        std::env::var("SENDER_LINK"),
    ) else {
        return;
    };
    let e = editor(Lang::En);
    let mut doc = Document::new();
    let mut c = doc::new_gadget(&e.assets, crate::gadget::Tag(*b"MiCo"), doc.new_id(), 300, 300);
    c.core.settings.set_label(0, b"42");
    c.behavior.settings_changed(&mut c.core);
    let mut s = doc::new_gadget(&e.assets, SENDER, doc.new_id(), 450, 300);
    s.core.settings.set_label(0, channel.as_bytes());
    s.behavior.settings_changed(&mut s.core);
    doc.gadgets.push(s);
    doc.gadgets.push(c);
    doc.connect(1, 0, 0, 0, Vec::new(), 0).unwrap();
    std::fs::write(out, crate::link::encode(&page, Some("SEND42.WGT"), &doc.save())).unwrap();
}
