//! The script editor: a scripted part's script on a double-click, the
//! widget's scripts from the Scripts list; typing, Apply, a script that
//! defines no part, the text kept when closed, vim keys.

use super::code::{Mode, Pos};
use super::console::Fonts;
use super::edit::{Target, layout};
use super::highlight::Syntax;
use super::tests::editor;
use super::window::Kind;
use crate::editor::menu::BAR_H;
use crate::editor::{Button, Editor};
use crate::openparts::scripted::Scripted;
use crate::openparts::scripted::tests::{Fake, part};
use crate::port_parts::ext::Script;

const VK_RETURN: u16 = 0x0d;
const VK_ESCAPE: u16 = 0x1b;
const VK_END: u16 = 0x23;
const VK_CONTROL: u16 = 0x11;

fn key(e: &mut Editor, vk: u16) {
    e.key_down(vk);
    e.key_up(vk);
}

fn ctrl(e: &mut Editor, vk: u16) {
    e.key_down(VK_CONTROL);
    key(e, vk);
    e.key_up(VK_CONTROL);
}

/// The tools on and a scripted part of the fake language's.
fn with_part() -> (Editor, usize) {
    let mut e = editor();
    e.set_enhanced(true);
    e.set_debug_tools(true);
    e.set_part_runtime(Box::new(Fake::default()));
    let id = e.doc.new_id();
    let mut g = part(&e.assets, id, "name:Adder in:a,b out:sum");
    // Clear of the toolbars.
    g.core.h += 200;
    g.core.v += 150;
    g.behavior.layout(&mut g.core, &e.assets);
    e.doc.gadgets.push(g);
    e.draw();
    let g = e.doc.gadgets.len() - 1;
    (e, g)
}

fn scripted(e: &Editor) -> &Scripted {
    e.doc
        .gadgets
        .iter()
        .find_map(|g| g.behavior.as_any().downcast_ref::<Scripted>())
        .unwrap()
}

fn double_click(e: &mut Editor, g: usize) {
    let r = e.doc.gadgets[g].core.rect();
    let (x, y) = (i32::midpoint(r.x0, r.x1), i32::midpoint(r.y0, r.y1) + BAR_H);
    for _ in 0..2 {
        e.mouse_down(x, y, Button::Left);
        e.mouse_up(x, y, Button::Left);
    }
    e.draw();
}

fn text(e: &Editor) -> String {
    e.debug.editing.as_ref().unwrap().code.text()
}

#[test]
fn a_double_click_on_a_scripted_part_edits_its_script() {
    let (mut e, g) = with_part();
    double_click(&mut e, g);
    assert!(e.dialog.is_none(), "not the settings dialog");
    assert!(e.debug.window(Kind::Editor).is_some());
    assert_eq!(text(&e), "name:Adder in:a,b out:sum");
    assert!(e.debug.editor_title.ends_with("Adder #1"), "{}", e.debug.editor_title);
    // Typing goes to it.
    ctrl(&mut e, VK_END);
    e.text_input(",carry");
    e.draw();
    assert!(e.debug.editor_title.ends_with('*'));
    // Ctrl+S applies it: the part has the new output.
    ctrl(&mut e, 0x53);
    assert_eq!(scripted(&e).def().outputs, ["sum", "carry"]);
    assert!(e.console_text().contains("#1: the script is applied."));
    // Undo on the worksheet takes it back; Ctrl+S gives it again.
    e.undo();
    assert_eq!(scripted(&e).def().outputs, ["sum"]);
    ctrl(&mut e, 0x53);
    assert_eq!(scripted(&e).def().outputs, ["sum", "carry"]);
    e.draw();
    assert!(!e.debug.editor_title.ends_with('*'));
    // A click puts the caret where it lands.
    let w = e.debug.window(Kind::Editor).unwrap();
    let l = layout(w.rect, Fonts::of(&e.assets).unwrap(), 1);
    let (x, y) = (l.text_x + 5 * l.cell + 1, l.area.y0 + 3);
    e.mouse_down(x, y, Button::Left);
    e.mouse_up(x, y, Button::Left);
    assert_eq!(e.debug.editing.as_ref().unwrap().code.caret, Pos::new(0, 5));
    // A script that defines no part says why; the part keeps its own.
    ctrl(&mut e, 0x41);
    e.text_input("oops");
    ctrl(&mut e, 0x53);
    let error = e.debug.editing.as_ref().unwrap().error.clone().unwrap();
    assert!(error.starts_with("fake:1:1  no name"), "{error}");
    assert_eq!(scripted(&e).def().name, "Adder");
    // Closed with Esc, the text waits for the next time.
    key(&mut e, VK_ESCAPE);
    assert!(e.debug.window(Kind::Editor).is_none());
    double_click(&mut e, g);
    assert_eq!(text(&e), "oops");
}

#[test]
fn vim_keys_edit_and_apply() {
    let (mut e, g) = with_part();
    e.set_vim_keys(true);
    assert!(e.edit_part(g));
    let code = |e: &Editor| e.debug.editing.as_ref().unwrap().code.clone();
    assert_eq!(code(&e).mode(), Some(Mode::Normal));
    e.text_input("A out2:x");
    assert_eq!(code(&e).mode(), Some(Mode::Insert));
    key(&mut e, VK_ESCAPE);
    // Esc in vim is the mode's, not the window's.
    assert!(e.debug.window(Kind::Editor).is_some());
    e.text_input("0dw:w");
    key(&mut e, VK_RETURN);
    assert_eq!(text(&e), ":Adder in:a,b out:sum out2:x");
    // It defines no part (no name): `:q` won't leave it unapplied.
    assert!(e.debug.editing.as_ref().unwrap().error.is_some());
    e.text_input(":q");
    key(&mut e, VK_RETURN);
    assert!(e.debug.window(Kind::Editor).is_some());
    e.text_input(":q!");
    key(&mut e, VK_RETURN);
    assert!(e.debug.window(Kind::Editor).is_none());
    assert_eq!(text(&e), "name:Adder in:a,b out:sum", "dropped");
    // Switched off, the editor's keys are plain again.
    e.set_vim_keys(false);
    assert_eq!(e.debug.editing.as_ref().unwrap().code.mode(), None);
}

#[test]
fn a_widgets_script_is_edited_from_the_scripts_list() {
    let (mut e, _) = with_part();
    e.doc.scripts.push(Script {
        name: "blink".to_owned(),
        language: crate::script::cmd::LANGUAGE.to_owned(),
        source: "log hi\n".to_owned(),
    });
    e.dirty = false;
    e.edit_script(0);
    let ed = e.debug.editing.as_ref().unwrap();
    assert_eq!(
        (&ed.target, ed.syntax),
        (&Target::Script("blink".into()), Syntax::Commands)
    );
    ctrl(&mut e, VK_END);
    e.text_input("step 5");
    ctrl(&mut e, 0x53);
    assert_eq!(e.doc.scripts[0].source, "log hi\nstep 5");
    assert!(e.dirty);
}

/// The editor drawn, to look at: `EDITOR_LOOK=out.ppm cargo test -p
/// engine editor_look -- --ignored`.
#[test]
#[ignore = "writes a picture to look at"]
fn editor_look() {
    let Some(path) = std::env::var_os("EDITOR_LOOK") else {
        return;
    };
    let (mut e, g) = with_part();
    if std::env::var_os("EDITOR_VIM").is_some() {
        e.set_vim_keys(true);
    }
    let code = "// A part that keeps the larger of its inputs.\n\
definePart({\n  name: \"Bigger\",\n  inputs: [\"a\", \"b\"],\n  outputs: [\"out\"],\n  \
/* the state */ start(s) { s.n = 0; },\n  tick(s, { a, b }) {\n    s.n++;\n    \
return { out: a > b ? a : b ?? null };\n  },\n});\n";
    let id = e.doc.gadgets[g].core.id;
    e.set_part_code(id, "name:Bigger in:a,b out:out").unwrap();
    assert!(e.edit_part(g));
    let ed = e.debug.editing.as_mut().unwrap();
    ed.code = super::code::Code::new(code, e.debug.vim_keys);
    ed.code.caret = Pos::new(6, 12);
    ed.code.anchor = Some(Pos::new(6, 4));
    e.draw();
    let (w, h) = (e.surface.width, e.surface.height);
    let pal = e.assets.palette();
    let mut out = format!("P6 {w} {h} 255\n").into_bytes();
    for &p in &e.surface.pixels {
        out.extend_from_slice(&pal[usize::from(p)]);
    }
    std::fs::write(path, out).unwrap();
}
