//! What the widget keeps of the debugging tools': the parts' names and
//! its scripts, through saving, Undo and links; the console's Scripts
//! list; the console's line of several lines.

use std::cell::RefCell;
use std::rc::Rc;

use super::scripts::{Press, Which, buttons};
use super::tests::{add, editor};
use super::window::Kind;
use crate::doc::Document;
use crate::editor::{Button, Editor, Links, download::Dropped};
use crate::port_parts::ext::Script;
use crate::script::{Host, PartId};

const VK_RETURN: u16 = 0x0d;
const VK_UP: u16 = 0x26;
const VK_DOWN: u16 = 0x28;
const VK_F8: u16 = 0x77;
const VK_CONTROL: u16 = 0x11;
const VK_SHIFT: u16 = 0x10;

fn key(e: &mut Editor, vk: u16) {
    e.key_down(vk);
    e.key_up(vk);
}

/// The tools on, the console open, a switch and a bulb.
fn console() -> (Editor, PartId, PartId) {
    let mut e = editor();
    e.set_enhanced(true);
    e.set_debug_tools(true);
    let sw = add(&mut e, *b"swit", 150, 280);
    let bulb = add(&mut e, *b"bulb", 380, 280);
    key(&mut e, VK_F8);
    assert!(e.debug.window(Kind::Console).is_some());
    e.update(16, (12, 0, 0));
    (e, sw, bulb)
}

fn frames(e: &mut Editor, n: usize) {
    for _ in 0..n {
        e.update(16, (12, 0, 0));
    }
}

/// Clicks row `n`'s button of the Scripts list.
fn press(e: &mut Editor, n: usize, which: Press) {
    e.draw();
    let w = e.debug.window(Kind::Console).unwrap();
    let font = crate::editor::ui::font(&e.assets).unwrap();
    let l = e
        .debug
        .console
        .layout(w.rect, super::console::Fonts::of(&e.assets).unwrap());
    let row = &e.debug.console.rows[n];
    let (run, other) = buttons(font, l.rows[n], row.which);
    let b = match which {
        Press::RunStop => run,
        Press::Other => other,
        Press::Read => crate::gfx::IRect::new(l.rows[n].x0, l.rows[n].y0, l.rows[n].x0 + 20, l.rows[n].y1),
    };
    let (x, y) = (b.x0 + b.width() / 2, b.y0 + b.height() / 2);
    e.mouse_down(x, y, Button::Left);
    e.mouse_up(x, y, Button::Left);
    e.draw();
}

fn whiches(e: &Editor) -> Vec<(Which, bool)> {
    e.debug.console.rows.iter().map(|r| (r.which, r.running)).collect()
}

fn drop_file(e: &mut Editor, name: &str, text: &str) {
    e.drop_file(Dropped {
        name: name.to_owned(),
        path: None,
        data: text.as_bytes().to_vec(),
    });
}

#[test]
fn names_are_kept_in_the_widget() {
    let (mut e, sw, bulb) = console();
    e.dirty = false;
    e.set_name(sw, Some("Lamp switch")).unwrap();
    assert!(e.dirty, "a name is a change to save");
    e.set_name(bulb, Some("Lamp")).unwrap();
    // Saved and read back.
    let saved = e.doc.save();
    let back = Document::load(&e.assets, &saved).unwrap();
    assert_eq!(back.names.get(&sw.0).map(String::as_str), Some("Lamp switch"));
    assert_eq!(back.save(), saved, "byte for byte");
    // Undo doesn't take them: a part moved, then Undo.
    Editor::snapshot(&mut e);
    e.doc.gadgets[0].core.h += 40;
    e.undo();
    assert_eq!(e.doc.gadgets[0].core.h, 150);
    assert_eq!(e.part_name(sw.0), Some("Lamp switch"));
    // A deleted part's name isn't saved.
    let i = e.doc.gadgets.iter().position(|g| g.core.id == bulb.0).unwrap();
    e.delete_gadget(i);
    let back = Document::load(&e.assets, &e.doc.save()).unwrap();
    assert_eq!(back.names.len(), 1);
    // A link carries them.
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
    let mut other = editor();
    other.open_link(&copied.borrow());
    assert_eq!(other.part_name(sw.0), Some("Lamp switch"));
    // And a script finds the part by it.
    assert_eq!(
        other.resolve(&crate::script::select::parse("\"Lamp switch\"").unwrap().part),
        Ok(sw)
    );
}

#[test]
fn a_dropped_script_is_kept_run_and_removed() {
    let (mut e, ..) = console();
    e.dirty = false;
    // Without the tools a script file isn't the console's.
    e.set_debug_tools(false);
    drop_file(&mut e, "blink.txt", "log hello\n");
    assert_eq!(e.debug.console.last, None);
    e.prompt = None;
    e.dropped = None;
    e.set_debug_tools(true);
    key(&mut e, VK_F8);
    drop_file(&mut e, "blink.txt", "log hello\r\nwait 50\r\nopen other.wgt\r\n");
    assert!(e.prompt.is_none(), "not opened as a widget");
    let last = e.debug.console.last.clone().unwrap();
    assert_eq!(
        (last.name.as_str(), last.language.as_str(), last.source.as_str()),
        ("blink.txt", "commands", "log hello\nwait 50\nopen other.wgt\n")
    );
    e.draw();
    assert_eq!(whiches(&e), [(Which::Last, false)]);
    assert!(e.debug.console.rows[0].text.contains("not in the widget"));
    // Keep: in the widget, a change to save.
    press(&mut e, 0, Press::Other);
    assert_eq!(e.doc.scripts.len(), 1);
    assert!(e.dirty);
    assert_eq!(whiches(&e), [(Which::Widget(0), false)]);
    // ▶: it runs as a widget's script, which can't open other widgets.
    press(&mut e, 0, Press::RunStop);
    assert_eq!(whiches(&e), [(Which::Widget(0), true)]);
    assert_eq!(e.debug.console.tasks, [], "listed in its row only");
    frames(&mut e, 10);
    e.draw();
    let log = e.debug.console.text();
    assert!(log.contains("hello"), "{log}");
    assert!(log.contains("can't do that"), "{log}");
    assert_eq!(whiches(&e), [(Which::Widget(0), false)]);
    // ■ stops it.
    press(&mut e, 0, Press::RunStop);
    assert_eq!(whiches(&e), [(Which::Widget(0), true)]);
    press(&mut e, 0, Press::RunStop);
    assert_eq!(whiches(&e), [(Which::Widget(0), false)]);
    // Saved with the widget, and Undo leaves it.
    let back = Document::load(&e.assets, &e.doc.save()).unwrap();
    assert_eq!(back.scripts, e.doc.scripts);
    Editor::snapshot(&mut e);
    e.undo();
    assert_eq!(e.doc.scripts.len(), 1);
    // ✕ takes it out.
    press(&mut e, 0, Press::Other);
    assert_eq!(e.doc.scripts, []);
    // The last script is offered again; kept twice, it gets a name of its
    // own.
    e.draw();
    assert_eq!(whiches(&e), [(Which::Last, false)]);
    e.keep_last_script();
    e.doc.scripts[0].source.push_str("log again\n");
    e.keep_last_script();
    let names: Vec<&str> = e.doc.scripts.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["blink.txt", "blink.txt 2"]);
    // A file that isn't text says so.
    e.drop_file(Dropped {
        name: "bad.js".into(),
        path: None,
        data: vec![0xff, 0xfe, 0],
    });
    assert!(e.debug.console.text().contains("bad.js: not a text file."));
}

#[test]
fn a_script_in_a_language_the_editor_lacks_says_so() {
    let (mut e, ..) = console();
    e.doc.scripts.push(Script {
        name: "x.js".into(),
        language: "JavaScript".into(),
        source: "log(1)".into(),
    });
    press(&mut e, 0, Press::RunStop);
    assert!(
        e.debug
            .console
            .text()
            .contains("x.js: there is no JavaScript here to run it."),
        "{}",
        e.debug.console.text()
    );
}

#[test]
fn the_line_takes_several_lines() {
    let (mut e, ..) = console();
    e.text_input("log one");
    e.key_down(VK_CONTROL);
    key(&mut e, VK_RETURN);
    e.key_up(VK_CONTROL);
    e.text_input("log two");
    e.key_down(VK_SHIFT);
    key(&mut e, VK_RETURN);
    e.key_up(VK_SHIFT);
    e.text_input("log three");
    assert_eq!(e.debug.console.input, "log one\nlog two\nlog three");
    assert_eq!(e.debug.console.input_rows(), 3);
    // Up and Down go from line to line, keeping the column.
    key(&mut e, VK_UP);
    key(&mut e, VK_UP);
    // ("log three" is longer: the caret was at its end, 9; line one has 7.)
    let c = &e.debug.console;
    assert_eq!(&c.input[..c.caret], "log one");
    key(&mut e, VK_DOWN);
    e.text_input("s");
    assert_eq!(e.debug.console.input, "log one\nlog twos\nlog three");
    // The window's line grows with it, and draws.
    e.draw();
    let w = e.debug.window(Kind::Console).unwrap();
    let font = crate::editor::ui::font(&e.assets).unwrap();
    let l = e
        .debug
        .console
        .layout(w.rect, super::console::Fonts::of(&e.assets).unwrap());
    assert_eq!(l.input.height(), 3 * i32::from(font.height) + 6);
    // Enter runs it all, a script: the console's last one.
    key(&mut e, VK_RETURN);
    frames(&mut e, 3);
    let log = e.debug.console.text();
    assert!(log.contains("> log one\n  log twos\n  log three\n"), "{log}");
    assert!(log.contains("one\ntwos\nthree\n"), "{log}");
    let last = e.debug.console.last.clone().unwrap();
    assert_eq!(last.source, "log one\nlog twos\nlog three");
    // A line alone isn't a script to keep.
    e.text_input("log four");
    key(&mut e, VK_RETURN);
    assert_eq!(e.debug.console.last.unwrap().source, last.source);
}

#[test]
fn a_click_on_a_scripts_name_opens_it_in_the_editor() {
    use super::console::LineKind;
    use super::highlight::Syntax;
    let (mut e, ..) = console();
    e.doc.scripts.push(Script {
        name: "blink".to_owned(),
        language: crate::script::cmd::LANGUAGE.to_owned(),
        source: "press Switch # on\nstep 5\n".to_owned(),
    });
    e.draw();
    press(&mut e, 0, Press::Read);
    let ed = e.debug.editing.as_ref().unwrap();
    assert_eq!(ed.code.text(), "press Switch # on\nstep 5\n");
    assert_eq!(ed.syntax, Syntax::Commands);
    assert!(e.debug.window(Kind::Editor).is_some());
    // Nothing ran.
    assert_eq!(whiches(&e), [(Which::Widget(0), false)]);
    // The console's last script, not the widget's, is written in the log.
    e.doc.scripts.clear();
    drop_file(&mut e, "two.txt", "log a\nlog b\n");
    key(&mut e, VK_F8);
    key(&mut e, VK_F8);
    e.draw();
    assert_eq!(whiches(&e), [(Which::Last, false)]);
    press(&mut e, 0, Press::Read);
    let lines: Vec<_> = e.debug.console.lines.iter().rev().take(3).collect();
    assert_eq!(lines[2].text, "\u{2014} two.txt (commands) \u{2014}");
    assert_eq!((lines[1].text.as_str(), lines[0].text.as_str()), ("log a", "log b"));
    assert!(
        lines[..2]
            .iter()
            .all(|l| l.kind == LineKind::Info && l.syntax == Some(Syntax::Commands))
    );
}
