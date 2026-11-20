//! Scripted parts in the editor: consent, the Scripts list, scripts given
//! from the console, breaking, the tray.

use std::cell::RefCell;
use std::rc::Rc;

use super::scripts::Which;
use super::tests::{add, editor};
use super::window::Kind;
use crate::editor::{Answer, Dropped, Editor, Links, Sel};
use crate::gadget::Value;
use crate::openparts::scripted::tests::{Fake, part};
use crate::openparts::scripted::{SCRIPTED, Scripted};

const VK_F8: u16 = 0x77;

fn key(e: &mut Editor, vk: u16) {
    e.key_down(vk);
    e.key_up(vk);
}

fn frames(e: &mut Editor, n: usize) {
    for _ in 0..n {
        e.update(17, (12, 0, 0));
    }
}

/// An editor as the player has it, with the fake language for parts.
fn player() -> Editor {
    let mut e = editor();
    e.set_enhanced(true);
    e.set_part_runtime(Box::new(Fake::default()));
    e
}

/// A link to a constant 2 wired into a scripted part (`code`) wired into
/// a bulb.
fn link(code: &str) -> String {
    let mut e = editor();
    let c = add(&mut e, *b"cnst", 20, 100);
    let i = e.doc.gadgets.len() - 1;
    e.doc.gadgets[i].core.settings.set_label(0, b"2");
    let g = part(&e.assets, e.doc.new_id(), code);
    e.doc.gadgets.push(g);
    add(&mut e, *b"bulb", 300, 100);
    e.doc.connect(0, 0, 1, 0, vec![], 0).unwrap();
    e.doc.connect(1, 1, 2, 0, vec![], 0).unwrap();
    let _ = c;
    let copied = Rc::new(RefCell::new(String::new()));
    let sink = copied.clone();
    e.set_links(Links {
        base: "https://example.org/widget.html".to_owned(),
        copy: Box::new(move |t| {
            *sink.borrow_mut() = t.to_owned();
            true
        }),
        paste: Box::new(|| {}),
    });
    e.copy_link();
    copied.take()
}

fn scripted(e: &Editor) -> &Scripted {
    e.doc
        .gadgets
        .iter()
        .find_map(|g| g.behavior.as_any().downcast_ref::<Scripted>())
        .unwrap()
}

/// What the scripted part gives on its output.
fn out(e: &Editor) -> Value {
    let g = e.doc.gadgets.iter().find(|g| g.core.tag == SCRIPTED).unwrap();
    g.behavior.output(&g.core, 1)
}

fn asked(e: &Editor) -> bool {
    e.prompt
        .as_ref()
        .is_some_and(|(p, _)| p.buttons == [Answer::Show, Answer::Run, Answer::DontRun])
}

const CODE: &str = "name:Doubler in:x out:y";

#[test]
fn a_widgets_scripted_part_asks_and_runs_once_allowed() {
    let url = link(CODE);
    let mut e = player();
    e.open_link(&url);
    assert!(asked(&e), "a scripted part asks too");
    e.draw();
    assert!(!scripted(&e).allowed(), "drawn as not allowed");
    key(&mut e, u16::from(b'R'));
    e.draw();
    assert!(scripted(&e).allowed());
    e.run();
    frames(&mut e, 5);
    assert_eq!(out(&e), Value::Number(2.0), "it ran (the fake gives the sum)");
    // Trusted: the next time it doesn't ask, and runs.
    let trust = std::mem::replace(
        &mut e.debug.trust,
        Box::new(crate::script::trust::MemoryTrust::default()),
    );
    let mut e = player();
    e.debug.trust = trust;
    e.open_link(&url);
    assert!(!asked(&e));
    e.draw();
    assert!(scripted(&e).allowed());
}

#[test]
fn not_allowed_its_inert_and_the_console_lets_it_run() {
    let mut e = player();
    e.open_link(&link(CODE));
    key(&mut e, 0x1b);
    assert!(e.prompt.is_none());
    e.run();
    frames(&mut e, 5);
    assert_eq!(out(&e), Value::None, "inert");
    assert!(!e.doc.gadgets.iter().any(|g| g.core.broken));
    e.stop();
    // The console lists it; ▶ lets it run.
    e.set_debug_tools(true);
    key(&mut e, VK_F8);
    e.draw();
    let rows = e.debug.console.rows.clone();
    assert_eq!(rows.len(), 1);
    assert!(matches!(rows[0].which, Which::Part(_)));
    assert!(
        rows[0].text.contains("Doubler") && rows[0].text.contains("not allowed"),
        "{}",
        rows[0].text
    );
    e.script_press(rows[0].which, super::scripts::Press::RunStop);
    e.draw();
    assert!(e.debug.console.rows[0].running);
    e.run();
    frames(&mut e, 5);
    assert_eq!(out(&e), Value::Number(2.0));
}

#[test]
fn a_failing_script_breaks_its_part_and_the_console_says_so() {
    let mut e = player();
    e.set_debug_tools(true);
    key(&mut e, VK_F8);
    e.open_link(&link("name:Thrower in:x out:y throw"));
    key(&mut e, u16::from(b'R'));
    e.run();
    frames(&mut e, 5);
    assert!(e.running, "the game goes on");
    let g = e.doc.gadgets.iter().find(|g| g.core.tag == SCRIPTED).unwrap();
    assert!(g.core.broken);
    assert!(
        e.debug.console.text().contains("Thrower (#2): fake:1:1  thrown"),
        "{}",
        e.debug.console.text()
    );
    e.stop();
    assert!(!e.doc.gadgets.iter().any(|g| g.core.broken), "STOP mends it");
}

#[test]
fn a_part_script_dropped_on_the_console_goes_into_a_part() {
    let mut e = player();
    e.set_debug_tools(true);
    key(&mut e, VK_F8);
    let drop = |e: &mut Editor, name: &str, text: &str| {
        e.drop_file(Dropped {
            name: name.into(),
            path: None,
            data: text.as_bytes().to_vec(),
        });
    };
    // None selected: a new part with it.
    drop(&mut e, "mix.js", "definePart( name:Mix in:a,b out:c");
    assert_eq!(e.doc.gadgets.len(), 1);
    assert_eq!(scripted(&e).def().name, "Mix");
    assert_eq!(e.doc.gadgets[0].core.knots.len(), 3);
    assert!(e.dirty);
    e.draw();
    assert!(scripted(&e).allowed(), "the player's own runs");
    // Selected: its script replaced; wires go as its knots change.
    let sw = add(&mut e, *b"swit", 300, 300);
    e.doc.connect(1, 0, 0, 0, vec![], 0).unwrap();
    e.sel = Sel::Gadget(0);
    drop(&mut e, "two.js", "definePart( name:Two in:p,q out:r");
    assert_eq!(e.doc.gadgets.len(), 2);
    assert_eq!(e.doc.wires.len(), 1, "the same knots: the wire stays");
    drop(&mut e, "one.js", "definePart( name:One in:p out:r,s");
    assert_eq!(e.doc.wires.len(), 0, "other knots: it goes");
    assert_eq!(scripted(&e).def().name, "One");
    // Undo: the script before.
    e.undo();
    assert_eq!(scripted(&e).def().name, "Two");
    let _ = sw;
    // A script that doesn't define a part says why.
    e.sel = Sel::Gadget(e.doc.gadgets.iter().position(|g| g.core.tag == SCRIPTED).unwrap());
    drop(&mut e, "bad.js", "definePart( nothing");
    assert!(e.debug.console.text().contains("no name"), "{}", e.debug.console.text());
    assert!(e.debug.window(Kind::Console).is_some());
}

#[test]
fn the_tray_has_it_only_with_the_debugging_tools() {
    let mut e = player();
    let has = |e: &Editor| e.tray.folders.iter().flatten().flatten().any(|s| s.index == 68);
    assert!(!has(&e));
    e.set_debug_tools(true);
    assert!(has(&e));
    e.set_debug_tools(false);
    assert!(!has(&e));
}
