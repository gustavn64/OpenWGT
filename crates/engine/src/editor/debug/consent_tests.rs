//! Asking before a widget's scripts run: a link, a file, trust kept by the
//! scripts' texts, Show, Run, Don't run, and what a widget's script can't
//! do.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use super::tests::{add, editor};
use super::window::Kind;
use crate::doc::Document;
use crate::editor::{Answer, Editor, Links};
use crate::port_parts::ext::Script;
use crate::script::Host;
use crate::script::trust::{Digest, Trust};

const VK_ESCAPE: u16 = 0x1b;
const VK_RETURN: u16 = 0x0d;

/// Trust shared by several editors, as the settings file is.
#[derive(Clone, Default)]
struct Shared(Rc<RefCell<BTreeSet<Digest>>>);

impl Trust for Shared {
    fn trusted(&self, d: &Digest) -> bool {
        self.0.borrow().contains(d)
    }

    fn trust(&mut self, d: &Digest) {
        self.0.borrow_mut().insert(*d);
    }
}

fn key(e: &mut Editor, vk: u16) {
    e.key_down(vk);
    e.key_up(vk);
}

fn frames(e: &mut Editor, n: usize) {
    for _ in 0..n {
        e.update(16, (12, 0, 0));
    }
}

/// A switch and a bulb, with the script `source` (in the built-in
/// commands), as a link.
fn link(source: &str) -> String {
    let mut e = editor();
    add(&mut e, *b"swit", 150, 280);
    add(&mut e, *b"bulb", 380, 280);
    e.doc.scripts.push(Script {
        name: "go.txt".into(),
        language: "commands".into(),
        source: source.into(),
    });
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

/// An editor as the player has it (the enhancements on, the tools off),
/// with `trust`.
fn player(trust: &Shared) -> Editor {
    let mut e = editor();
    e.set_enhanced(true);
    e.debug.trust = Box::new(trust.clone());
    e
}

fn asked(e: &Editor) -> bool {
    e.prompt.as_ref().is_some_and(|(p, _)| {
        p.text == "This widget comes with scripts." && p.buttons == [Answer::Show, Answer::Run, Answer::DontRun]
    })
}

fn live(e: &Editor) -> usize {
    e.debug.runtimes.live().len()
}

const GO: &str = "log ran\nrun\nwait 10000\n";

#[test]
fn a_link_with_scripts_asks_and_run_runs_them() {
    let trust = Shared::default();
    let mut e = player(&trust);
    e.open_link(&link(GO));
    assert!(asked(&e));
    assert_eq!(e.doc.gadgets.len(), 2, "the widget is open behind the question");
    frames(&mut e, 5);
    assert!(!e.running, "nothing runs while it asks");
    // Run (its letter): it runs, with the tools off, and is trusted.
    key(&mut e, u16::from(b'R'));
    assert!(e.prompt.is_none());
    frames(&mut e, 5);
    assert!(e.running, "the script ran");
    assert_eq!(live(&e), 1, "it waits, the tools off");
    assert!(!e.debugging(), "no tools for it");
    assert!(trust.trusted(&Digest::of(GO)));
    // Another widget comes: the script stops.
    e.stop();
    e.open_link(&link("log other\n"));
    assert_eq!(live(&e), 0);
}

#[test]
fn trusted_scripts_dont_ask_and_a_changed_one_asks_again() {
    let trust = Shared::default();
    trust.clone().trust(&Digest::of(GO));
    let mut e = player(&trust);
    e.open_link(&link(GO));
    assert!(!asked(&e));
    frames(&mut e, 5);
    assert!(e.running, "trusted: it ran");
    // The same widget with its script changed: asked again.
    let mut e = player(&trust);
    e.open_link(&link(&format!("{GO}log changed\n")));
    assert!(asked(&e));
}

#[test]
fn dont_run_runs_nothing() {
    let trust = Shared::default();
    let mut e = player(&trust);
    e.open_link(&link(GO));
    assert!(asked(&e));
    // Esc is Don't run.
    key(&mut e, VK_ESCAPE);
    assert!(e.prompt.is_none());
    frames(&mut e, 10);
    assert!(!e.running);
    assert_eq!(live(&e), 0);
    assert!(!trust.trusted(&Digest::of(GO)));
    assert_eq!(e.doc.scripts.len(), 1, "still in the widget");
}

#[test]
fn show_lists_them_in_the_console_without_running_them() {
    let trust = Shared::default();
    let mut e = player(&trust);
    e.open_link(&link(GO));
    // Enter is Show, the first.
    key(&mut e, VK_RETURN);
    assert!(e.debugging(), "the tools on for it");
    assert_eq!(e.take_debug_tools(), None, "for the session only");
    assert!(e.debug.window(Kind::Console).is_some());
    frames(&mut e, 5);
    e.draw();
    assert!(!e.running);
    assert_eq!(e.debug.console.rows.len(), 1);
    assert!(!e.debug.console.rows[0].running);
    let log = e.debug.console.text();
    assert!(log.contains("none of them runs until you start it"));
    assert!(log.contains("log ran\nrun\nwait 10000\n"), "its text to read: {log}");
    assert!(!trust.trusted(&Digest::of(GO)));
}

#[test]
fn a_widgets_script_cant_open_widgets() {
    let trust = Shared::default();
    let mut e = player(&trust);
    e.set_debug_tools(true);
    e.open_link(&link("new\nrun\n"));
    key(&mut e, u16::from(b'R'));
    frames(&mut e, 5);
    assert_eq!(e.doc.gadgets.len(), 2, "not cleared");
    assert!(!e.running, "it stopped at new");
    let log = e.debug.console.text();
    assert!(log.contains("go.txt:1") && log.contains("can't do that"), "{log}");
}

#[test]
fn files_ask_too_and_the_original_way_doesnt() {
    let dir = std::env::temp_dir().join(format!("openwgt-consent-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("go.wgt");
    let mut w = editor();
    add(&mut w, *b"swit", 150, 280);
    w.doc.scripts.push(Script {
        name: "go.txt".into(),
        language: "commands".into(),
        source: GO.into(),
    });
    std::fs::write(&path, w.doc.save()).unwrap();
    let trust = Shared::default();
    let mut e = player(&trust);
    assert!(e.open_path(&path.to_string_lossy()));
    assert!(asked(&e));
    // The enhancements off: nothing asks, nothing runs.
    let mut e = player(&trust);
    e.set_enhanced(false);
    assert!(e.open_path(&path.to_string_lossy()));
    assert!(!asked(&e));
    frames(&mut e, 5);
    assert!(!e.running);
    // A widget a script opens: not asked about, its scripts don't run.
    let mut e = player(&trust);
    e.open_widget("go.wgt", &std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(e.doc.scripts.len(), 1);
    assert!(!asked(&e));
    frames(&mut e, 5);
    assert!(!e.running);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_widget_without_scripts_doesnt_ask() {
    let mut e = player(&Shared::default());
    let mut w = editor();
    add(&mut w, *b"swit", 150, 280);
    let bytes = w.doc.save();
    assert_eq!(Document::load(&e.assets, &bytes).unwrap().scripts, []);
    e.drop_file(crate::editor::Dropped {
        name: "plain.wgt".into(),
        path: None,
        data: bytes,
    });
    assert!(e.prompt.is_none());
}
