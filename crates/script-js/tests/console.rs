//! JavaScript in the game's console (the debugging tools'), as the program
//! sets it up: lines typed run as JavaScript, `:` before one for the
//! built-in commands, and a script awaiting ticks goes on as the widget runs.

use engine::assets::{Assets, Packs};
use engine::doc::Document;
use engine::editor::Editor;
use engine::lang::Lang;

const VK_RETURN: u16 = 0x0d;
const VK_F8: u16 = 0x77;

fn console() -> Editor {
    let graphics = engine::opengfx::builtin_for(Lang::En);
    let sounds = engine::opensfx::builtin();
    let data = engine::opendata::builtin_for(Lang::En);
    let packs = Packs {
        graphics: Some(graphics.as_ref()),
        sounds: Some(&sounds),
        data: Some(&data),
    };
    let assets = Assets::load_with(None, packs, None).unwrap();
    let mut e = Editor::new(assets, Document::new(), 800, 600);
    e.set_enhanced(true);
    e.set_debug_tools(true);
    e.set_script_runtime(Box::new(script_js::Js::new()));
    e.key_down(VK_F8);
    e.key_up(VK_F8);
    e.update(16, (12, 0, 0));
    e
}

fn enter(e: &mut Editor, line: &str) {
    e.text_input(line);
    e.key_down(VK_RETURN);
    e.key_up(VK_RETURN);
    e.update(16, (12, 0, 0));
}

#[test]
fn the_console_runs_javascript() {
    let mut e = console();
    enter(&mut e, "1 + 2");
    let out = e.console_text();
    assert!(out.contains("> 1 + 2\n3\n"), "{out}");
    enter(&mut e, ":help");
    assert!(e.console_text().len() > out.len() + 100, "{}", e.console_text());
    // A script that waits for ticks goes on as the widget runs.
    enter(&mut e, "run(); await ticks(30); log('waited', clock.ticks >= 30)");
    for _ in 0..60 {
        e.update(16, (12, 0, 0));
    }
    let out = e.console_text();
    assert!(out.contains("waited true"), "{out}");
}

#[test]
fn a_widgets_javascript_runs_as_one_that_came_with_it() {
    let mut e = console();
    let mut w = Document::new();
    let id = w.new_id();
    w.gadgets.push(engine::doc::new_gadget(
        &e.assets,
        engine::gadget::Tag(*b"swit"),
        id,
        100,
        200,
    ));
    w.scripts.push(engine::port_parts::ext::Script {
        name: "go.js".into(),
        language: "JavaScript".into(),
        source: "run();\nawait ticks(2);\nlog('ran', clock.running);\nloadPuzzle(1);\nlog('loaded');\n".into(),
    });
    e.drop_file(engine::editor::Dropped {
        name: "go.wgt".into(),
        path: None,
        data: w.save(),
    });
    // Asked; Run.
    e.key_down(u16::from(b'R'));
    e.key_up(u16::from(b'R'));
    for _ in 0..30 {
        e.update(16, (12, 0, 0));
    }
    let out = e.console_text();
    assert!(out.contains("ran true"), "{out}");
    assert!(out.contains("go.js:4"), "{out}");
    assert!(!out.contains("loaded"), "{out}");
    assert_eq!(e.doc.gadgets.len(), 1, "the widget is still the one");
}
