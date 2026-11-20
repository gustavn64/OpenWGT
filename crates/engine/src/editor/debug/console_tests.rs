//! The console: typing, running lines, errors with carets, history,
//! completion, the clipboard, stopping scripts, a touch screen's box.

use std::cell::RefCell;
use std::rc::Rc;

use super::control::cmd;
use super::tests::{add, editor};
use super::window::Kind;
use crate::editor::{Button, Editor, Links};

const VK_RETURN: u16 = 0x0d;
const VK_TAB: u16 = 0x09;
const VK_ESCAPE: u16 = 0x1b;
const VK_UP: u16 = 0x26;
const VK_DOWN: u16 = 0x28;
const VK_BACK: u16 = 0x08;
const VK_LEFT: u16 = 0x25;
const VK_F8: u16 = 0x77;
const VK_CONTROL: u16 = 0x11;

/// An editor with the tools on and the console open, a switch and a
/// bulb on the worksheet.
fn console() -> Editor {
    let mut e = editor();
    e.set_enhanced(true);
    e.set_debug_tools(true);
    add(&mut e, *b"swit", 150, 280);
    add(&mut e, *b"bulb", 380, 280);
    key(&mut e, VK_F8);
    assert!(e.debug.window(Kind::Console).is_some());
    e.update(16, (12, 0, 0));
    e
}

fn key(e: &mut Editor, vk: u16) {
    e.key_down(vk);
    e.key_up(vk);
}

/// `line` typed as text, then Enter.
fn enter(e: &mut Editor, line: &str) {
    e.text_input(line);
    key(e, VK_RETURN);
}

fn log(e: &Editor) -> String {
    e.debug.console.text()
}

#[test]
fn lines_run_and_say_what_they_did() {
    let mut e = console();
    enter(&mut e, "parts");
    let out = log(&e);
    assert!(out.starts_with("> parts\n"), "{out}");
    assert!(out.contains("Switch"), "{out}");
    assert!(out.contains("Light Bulb"), "{out}");
    // A part alone: its value.
    enter(&mut e, "Switch");
    assert!(log(&e).ends_with("> Switch\nSwitch = off\n"), "{}", log(&e));
    // `:` for the built-in commands, the same here.
    enter(&mut e, ":run");
    assert!(e.running);
    enter(&mut e, "step 3");
    assert_eq!(e.env.ticks, 3);
    // The colours: what was typed, the values, errors.
    let kinds: Vec<_> = e.debug.console.lines.iter().map(|l| l.kind).collect();
    assert_eq!(kinds[0], super::LineKind::Input);
    assert!(kinds.contains(&super::LineKind::Result));
}

#[test]
fn an_error_points_at_its_place() {
    let mut e = console();
    enter(&mut e, "press Lamb");
    let out = log(&e);
    assert!(out.contains("no part called \"Lamb\""), "{out}");
    let rows: Vec<&str> = out.lines().collect();
    let at = rows.iter().position(|r| r.ends_with("| press Lamb")).expect(&out);
    let carets = rows[at + 1];
    let col = rows[at].find("Lamb").unwrap();
    assert_eq!(carets.find('^'), Some(col), "{out}");
    assert_eq!(carets.trim(), "^^^^");
    // A line that doesn't read.
    enter(&mut e, "until Switch ==");
    assert!(log(&e).lines().any(|r| r.trim_start().starts_with('^')), "{}", log(&e));
    let errors = e
        .debug
        .console
        .lines
        .iter()
        .filter(|l| l.kind == super::LineKind::Error)
        .count();
    assert!(errors >= 4, "{}", log(&e));
}

#[test]
fn typing_edits_the_line_and_history_comes_back() {
    let mut e = console();
    e.text_input("stpe");
    key(&mut e, VK_BACK);
    key(&mut e, VK_BACK);
    e.text_input("ep");
    assert_eq!(e.debug.console.input, "step");
    key(&mut e, VK_LEFT);
    e.text_input("å");
    assert_eq!(e.debug.console.input, "steåp");
    key(&mut e, VK_ESCAPE);
    assert_eq!(e.debug.console.input, "", "Esc clears the line");
    enter(&mut e, "help run");
    enter(&mut e, "parts");
    key(&mut e, VK_UP);
    assert_eq!(e.debug.console.input, "parts");
    key(&mut e, VK_UP);
    assert_eq!(e.debug.console.input, "help run");
    key(&mut e, VK_DOWN);
    key(&mut e, VK_DOWN);
    assert_eq!(e.debug.console.input, "");
    // Esc on an empty line gives the keyboard back.
    key(&mut e, VK_ESCAPE);
    assert!(!e.console_has_keys());
    e.text_input("x");
    assert_eq!(e.debug.console.input, "");
}

#[test]
fn tab_completes_commands_and_parts() {
    let mut e = console();
    e.text_input("pre");
    key(&mut e, VK_TAB);
    assert_eq!(e.debug.console.input, "press ");
    e.text_input("Sw");
    key(&mut e, VK_TAB);
    assert_eq!(e.debug.console.input, "press Switch ");
    // Several: what they share, and the list.
    e.debug.console.input.clear();
    e.debug.console.caret = 0;
    e.text_input("s");
    key(&mut e, VK_TAB);
    let last = e.debug.console.lines.back().unwrap().text.clone();
    assert!(
        last.contains("step") && last.contains("stop") && last.contains("set"),
        "{last}"
    );
    assert_eq!(e.debug.console.input, "s");
    // A knot after the dot.
    e.debug.console.input.clear();
    e.debug.console.caret = 0;
    e.text_input("print LightBulb.");
    key(&mut e, VK_TAB);
    assert_eq!(e.debug.console.input, "print LightBulb.in ");
    // `:` lines too.
    e.debug.console.input.clear();
    e.debug.console.caret = 0;
    e.text_input(":wir");
    key(&mut e, VK_TAB);
    assert_eq!(e.debug.console.input, ":wires ");
}

#[test]
fn the_keys_are_the_consoles_not_the_widgets() {
    let mut e = console();
    e.run();
    e.key_down(u16::from(b'A'));
    e.text_input("a");
    e.update(16, (12, 0, 0));
    assert_eq!(e.env.keys, []);
    assert_eq!(e.debug.console.input, "a");
    // Enter runs the line, not GO; Esc doesn't stop the widget.
    key(&mut e, VK_ESCAPE);
    key(&mut e, VK_ESCAPE);
    assert!(e.running);
    // A click on the worksheet takes the keyboard back to the widget.
    e.mouse_down(300, 100, Button::Left);
    e.mouse_up(300, 100, Button::Left);
    e.key_down(u16::from(b'B'));
    assert_eq!(e.env.keys, [u16::from(b'B')]);
    e.text_input("b");
    assert_eq!(e.debug.console.input, "");
}

#[test]
fn pasting_runs_whole_lines() {
    let mut e = console();
    let asked = Rc::new(RefCell::new(0));
    let copied = Rc::new(RefCell::new(String::new()));
    let (a, c) = (asked.clone(), copied.clone());
    e.set_links(Links {
        base: "https://example.org/".into(),
        copy: Box::new(move |t| {
            *c.borrow_mut() = t.to_owned();
            true
        }),
        paste: Box::new(move || *a.borrow_mut() += 1),
    });
    e.key_down(VK_CONTROL);
    key(&mut e, 0x56);
    e.key_up(VK_CONTROL);
    assert_eq!(*asked.borrow(), 1);
    e.paste_link("run\r\nstep 2\r\nstep");
    assert_eq!(e.env.ticks, 2);
    assert_eq!(e.debug.console.input, "step");
    // Ctrl+C: the line.
    e.key_down(VK_CONTROL);
    key(&mut e, 0x43);
    e.key_up(VK_CONTROL);
    assert_eq!(*copied.borrow(), "step");
}

#[test]
fn scripts_are_listed_and_stopped() {
    let mut e = console();
    enter(&mut e, "wait 5000");
    e.update(16, (12, 0, 0));
    e.draw();
    assert_eq!(e.debug.console.tasks.len(), 1);
    assert!(e.menu.enabled(cmd::STOP_SCRIPTS));
    key(&mut e, VK_ESCAPE);
    e.draw();
    assert_eq!(e.debug.console.tasks, []);
    assert!(log(&e).contains("Scripts stopped."));
    // The menu's Stop Scripts, and the list's stop box.
    enter(&mut e, "wait 5000");
    e.update(16, (12, 0, 0));
    e.command(cmd::STOP_SCRIPTS);
    e.draw();
    assert_eq!(e.debug.console.tasks, []);
    enter(&mut e, "wait 5000");
    e.update(16, (12, 0, 0));
    e.draw();
    let w = e.debug.window(Kind::Console).unwrap();
    let (_, stop) = super::console::layout(w.rect, super::console::Fonts::of(&e.assets).unwrap(), 0, 1, 1).tasks[0];
    e.mouse_down(stop.x0 + 3, stop.y0 + 3, Button::Left);
    e.mouse_up(stop.x0 + 3, stop.y0 + 3, Button::Left);
    e.draw();
    assert_eq!(e.debug.console.tasks, []);
    // Waits end on their own as the frames go.
    enter(&mut e, "wait 100");
    enter(&mut e, "log done");
    for _ in 0..10 {
        e.update(16, (12, 0, 0));
    }
    assert!(log(&e).contains("\ndone\n"), "{}", log(&e));
}

#[test]
fn a_touch_screen_edits_the_line_in_its_box() {
    let mut e = console();
    e.text_input("step");
    let f = e.focused_text().unwrap();
    assert_eq!(f.text, "step");
    assert!(!f.multiline);
    e.set_focused_text("step 4");
    assert_eq!(e.debug.console.input, "step 4");
    // A tap on the line asks for the box.
    let w = e.debug.window(Kind::Console).unwrap();
    let line = super::console::layout(w.rect, super::console::Fonts::of(&e.assets).unwrap(), 0, 0, 1).input;
    e.mouse_down(line.x0 + 30, line.y0 + 4, Button::Left);
    e.mouse_up(line.x0 + 30, line.y0 + 4, Button::Left);
    assert_eq!(e.take_text_tapped().map(|f| f.text), Some("step 4".to_owned()));
}

#[test]
fn the_window_moves_closes_and_isnt_the_worksheet() {
    let mut e = console();
    let w = e.debug.window(Kind::Console).unwrap().rect;
    // Not the worksheet's: no hover selection, no note under it.
    assert!(!e.over_view((w.x0 + 20, w.y0 + 40)));
    // Dragged by its caption.
    e.mouse_down(w.x0 + 20, w.y0 + 8, Button::Left);
    e.mouse_move(w.x0 + 30, w.y0 - 52);
    e.mouse_up(w.x0 + 30, w.y0 - 52, Button::Left);
    let m = e.debug.window(Kind::Console).unwrap().rect;
    assert_eq!((m.x0, m.y0), (w.x0 + 10, w.y0 - 60));
    assert_eq!(m.width(), w.width());
    // The close box.
    let b = super::window::close_box(m);
    e.mouse_down(b.x0 + 2, b.y0 + 2, Button::Left);
    e.mouse_up(b.x0 + 2, b.y0 + 2, Button::Left);
    assert!(e.debug.window(Kind::Console).is_none());
    // Back where it was, with its log.
    key(&mut e, VK_F8);
    assert_eq!(e.debug.window(Kind::Console).unwrap().rect, m);
    // The tools off: gone, and F8 is the widget's.
    e.set_debug_tools(false);
    e.draw();
    assert!(e.debug.window(Kind::Console).is_none());
    e.run();
    key(&mut e, VK_F8);
    assert_eq!(e.env.keys, [VK_F8]);
}

#[test]
#[allow(clippy::naive_bytecount)]
fn the_log_and_the_line_are_fixed_pitch_and_keep_ampersands() {
    let mut e = console();
    let fonts = super::console::Fonts::of(&e.assets).unwrap();
    assert_eq!(fonts.mono.face, "Widget Mono");
    let w = |t: &str| crate::editor::ui::width(fonts.mono, t);
    assert_eq!(w("iiii"), w("MMMM"));
    assert_eq!(crate::editor::ui::width(fonts.bold, "while"), w("while"));
    // `&` is drawn, not taken for a mnemonic.
    e.debug.console.add(super::console::LineKind::Info, "a && b & c");
    e.draw();
    let ink = |e: &Editor| e.surface.pixels.iter().filter(|&&p| p == 0).count();
    let with = ink(&e);
    e.debug.console.lines.back_mut().unwrap().text = "a    b   c".to_owned();
    e.draw();
    assert!(ink(&e) < with);
}

/// Writes a picture of the console with coloured lines to `CONSOLE_LOOK`.
#[test]
#[ignore = "writes a picture to CONSOLE_LOOK"]
fn console_look() {
    use super::console::LineKind;
    use super::highlight::Syntax;
    let Some(path) = std::env::var_os("CONSOLE_LOOK") else {
        return;
    };
    let mut e = console();
    e.debug.runtimes.has_program = true;
    let c = &mut e.debug.console;
    c.add_in(
        LineKind::Input,
        "> parts().filter(p => p.kind === \"bulb\" && p.on)",
        Some(Syntax::Js),
    );
    c.add_in(
        LineKind::Result,
        "[ { name: \"Lamp\", id: 2, on: true } ]",
        Some(Syntax::Js),
    );
    c.add_in(
        LineKind::Input,
        "> /* several\n  lines */ for (let i = 0; i < 3; i++) {\n    await ticks(0x3c); log(`i = ${i}`)\n  }",
        Some(Syntax::Js),
    );
    c.add(LineKind::Info, "i = 0");
    c.add_in(
        LineKind::Input,
        "> :until Lamp == on max 120 # wait",
        Some(Syntax::Commands),
    );
    c.add_in(
        LineKind::Input,
        "> :press \"Big switch\" #7 1.5",
        Some(Syntax::Commands),
    );
    c.add(
        LineKind::Error,
        "console:1:7  no part called \"Lamb\"\n 1 | press Lamb\n   |       ^^^^",
    );
    c.add(LineKind::Warn, "Scripts stopped.");
    e.text_input("const s = 'åäö'; // typed");
    e.draw();
    let pal = e.assets.palette();
    let s = &e.surface;
    let mut out = format!("P6 {} {} 255\n", s.width, s.height).into_bytes();
    for &i in &s.pixels {
        out.extend_from_slice(&pal[usize::from(i)]);
    }
    std::fs::write(path, out).unwrap();
}
