//! Scripts run on the editor, with the port's own packs (no CD needed).

use std::cell::RefCell;
use std::rc::Rc;

use engine::assets::{Assets, Packs};
use engine::doc::{self, Document};
use engine::editor::Editor;
use engine::gadget::Tag;
use engine::lang::Lang;
use engine::script::runtime::{ErrKind, Limits, Report, Runtime, run_headless};
use engine::script::{Caps, Host, Level, PartId, Shot, Sink};

use super::{Js, api};

fn assets() -> Assets {
    let graphics = engine::opengfx::builtin_for(Lang::En);
    let sounds = engine::opensfx::builtin();
    let data = engine::opendata::builtin_for(Lang::En);
    let packs = Packs {
        graphics: Some(graphics.as_ref()),
        sounds: Some(&sounds),
        data: Some(&data),
    };
    let mut assets = Assets::load_with(None, packs, None).unwrap();
    assets.lang = Lang::En;
    assets
}

/// The log, kept for the test to read.
#[derive(Clone, Default)]
struct Log(Rc<RefCell<Vec<(Level, String)>>>);

impl Sink for Log {
    fn log(&mut self, level: Level, msg: &str) {
        self.0.borrow_mut().push((level, msg.to_owned()));
    }

    fn shot(&mut self, name: &str, _shot: &Shot) -> Result<(), String> {
        self.0.borrow_mut().push((Level::Info, format!("[shot {name}]")));
        Ok(())
    }
}

impl Log {
    fn text(&self) -> String {
        self.0
            .borrow()
            .iter()
            .map(|(_, l)| l.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn editor() -> (Editor, Log) {
    let mut e = Editor::new(assets(), Document::new(), 640, 433);
    let log = Log::default();
    e.debug.sink = Box::new(log.clone());
    (e, log)
}

fn add(e: &mut Editor, tag: [u8; 4], h: i32, v: i32) -> PartId {
    let id = e.doc.new_id();
    let g = doc::new_gadget(&e.assets, Tag(tag), id, h, v);
    e.doc.gadgets.push(g);
    PartId(id)
}

fn index(e: &Editor, id: PartId) -> usize {
    e.doc.gadgets.iter().position(|g| g.core.id == id.0).unwrap()
}

/// Wires output `out` (among the outputs) of `from` to input `inp` of `to`.
fn connect(e: &mut Editor, from: PartId, out: usize, to: PartId, inp: usize) {
    let (f, t) = (index(e, from), index(e, to));
    let nth = |e: &Editor, g: usize, input: bool, n: usize| {
        let knots = &e.doc.gadgets[g].core.knots;
        (0..knots.len())
            .filter(|&k| knots[k].is_input() == input)
            .nth(n)
            .unwrap()
    };
    let (fk, tk) = (nth(e, f, false, out), nth(e, t, true, inp));
    e.doc.connect(f, fk, t, tk, Vec::new(), 0).unwrap();
}

/// A Label reading `text` just above part `id`.
fn label(e: &mut Editor, id: PartId, text: &str) {
    let r = e.doc.gadgets[index(e, id)].core.rect();
    let l = add(e, *b"TxLb", 0, 0);
    let i = index(e, l);
    let assets = &e.assets;
    let g = &mut e.doc.gadgets[i];
    g.core.settings.set_label(0, text.as_bytes());
    g.behavior.settings_changed(&mut g.core);
    g.behavior.layout(&mut g.core, assets);
    let b = g.core.bounds;
    g.core.h = r.x0 - i32::from(b.left);
    g.core.v = r.y0 - 6 - i32::from(b.bottom);
}

/// A half adder: Switches A and B into an XOR (Sum) and an AND (Carry).
fn half_adder(e: &mut Editor) {
    let a = add(e, *b"swit", 40, 60);
    let b = add(e, *b"swit", 40, 220);
    let xor = add(e, *b"XorG", 220, 60);
    let and = add(e, *b"AndG", 220, 220);
    let sum = add(e, *b"bulb", 420, 60);
    let carry = add(e, *b"bulb", 420, 220);
    for (p, t) in [(a, "A"), (b, "B"), (sum, "Sum"), (carry, "Carry")] {
        label(e, p, t);
    }
    // A knot takes one wire: Splitters share the switches.
    let sa = add(e, *b"Spl2", 130, 60);
    let sb = add(e, *b"Spl2", 130, 220);
    connect(e, a, 0, sa, 0);
    connect(e, b, 0, sb, 0);
    for (n, g) in [xor, and].into_iter().enumerate() {
        connect(e, sa, n, g, 0);
        connect(e, sb, n, g, 1);
    }
    connect(e, xor, 0, sum, 0);
    connect(e, and, 0, carry, 0);
}

fn run(e: &mut Editor, js: &mut Js, src: &str) -> Report {
    run_headless(js, "test.js", src, Caps::ALL, e).unwrap()
}

/// The plan's example 2.
const HALF_ADDER: &str = r#"test("half adder truth table", async () => {
  const [a, b] = [part("A"), part("B")];   // two Switches labelled A and B
  const sum = part("Sum"), carry = part("Carry");
  run();
  for (const [x, y] of [[0,0],[0,1],[1,0],[1,1]]) {
    a.value = !!x; b.value = !!y;          // the switches' own on/off
    await ticks(2);                        // let it settle
    assertEqual(sum.on, (x ^ y) === 1, `sum for ${x}${y}`);
    assertEqual(carry.on, (x & y) === 1, `carry for ${x}${y}`);
  }
  screenshot("half-adder");
});
"#;

#[test]
fn the_half_adder_example_passes() {
    let (mut e, log) = editor();
    half_adder(&mut e);
    let r = run(&mut e, &mut Js::new(), HALF_ADDER);
    assert!(r.passed(), "{r:?}\n{}", log.text());
    let text = log.text();
    assert!(text.contains("\u{2713} half adder truth table (8 ticks)"), "{text}");
    assert!(text.contains("[shot half-adder]"), "{text}");
}

#[test]
fn a_failing_check_is_counted_where_it_is() {
    let (mut e, log) = editor();
    half_adder(&mut e);
    let src = "run();\npart(\"A\").value = true;\nawait ticks(2);\nassert(part(\"Carry\").on, \"carry\");\nlog(\"went on\");\n";
    let r = run(&mut e, &mut Js::new(), src);
    assert!(!r.passed());
    assert_eq!(r.failures, 1);
    assert_eq!(r.errors[0].kind, ErrKind::Assert);
    assert_eq!(
        r.errors[0].to_string(),
        "test.js:4:1  carry\n 4 | assert(part(\"Carry\").on, \"carry\");\n   | ^^^^^^"
    );
    assert!(log.text().contains("went on"));
}

#[test]
fn the_sandbox_has_no_way_out() {
    let (mut e, log) = editor();
    let src = r#"
for (const name of ["require", "fetch", "setTimeout", "setInterval", "XMLHttpRequest", "WeakRef",
    "FinalizationRegistry", "SharedArrayBuffer", "Atomics", "performance", "process", "std", "os", "print"]) {
  assertEqual(typeof globalThis[name], "undefined", name);
}
let st = "pending";
import("fs").then(() => st = "loaded", (e) => st = "refused");
await ticks(0);
assertEqual(st, "refused", "import()");
"#;
    let r = run(&mut e, &mut Js::new(), src);
    assert!(r.passed(), "{r:?}\n{}", log.text());
    // A static import can't be loaded either: the script doesn't start.
    let err = Js::new()
        .check("t.js", "import fs from \"fs\";\nlog(fs);\n")
        .unwrap_err();
    assert!(err.message.contains("could not load module 'fs'"), "{err}");
}

#[test]
fn date_follows_the_ticks_and_random_repeats() {
    let (mut e, log) = editor();
    let src = r"
const t0 = Date.now();
assertEqual(new Date().getTime(), t0);
assertEqual(new Date(t0).toISOString().slice(0, 10), '1995-09-01');
run();
await ticks(60);
assertEqual(Date.now() - t0, 1000, 'a second of ticks');
await seconds(0.5);
assertEqual(Date.now() - t0, 1500);
log(Math.random());
";
    let r = run(&mut e, &mut Js::new(), src);
    assert!(r.passed(), "{r:?}\n{}", log.text());
    let first = log.text();
    let (mut e, log) = editor();
    run(&mut e, &mut Js::new(), src);
    assert_eq!(first, log.text(), "the same random numbers");
}

#[test]
fn a_runaway_loop_is_stopped_and_the_game_goes_on() {
    let (mut e, log) = editor();
    half_adder(&mut e);
    let mut js = Js::new();
    let r = run(
        &mut e,
        &mut js,
        "run();\nlet n = 0;\ntry {\n  while (true) { n++; }\n} catch (x) {\n  log('caught');\n}\n",
    );
    assert!(!r.passed());
    assert_eq!(r.errors[0].kind, ErrKind::Budget, "{:?}", r.errors);
    assert!(r.errors[0].message.contains("await ticks(1)"), "{}", r.errors[0]);
    // (QuickJS places it at the loop's function: here the module's start.)
    assert!(r.errors[0].line >= 1, "{}", r.errors[0]);
    assert!(!log.text().contains("caught"), "the budget can't be caught");
    // The game goes on, and so do scripts.
    let before = e.clock().ticks;
    e.wait(500).unwrap();
    assert!(e.clock().ticks >= before + 25);
    let r = run(&mut e, &mut js, "await ticks(1);\nlog('fine');\n");
    assert!(r.passed(), "{r:?}");
    // In an async function, and with work in promises' jobs too.
    let r = run(&mut e, &mut js, "await ticks(1);\nfor (;;) { await null; }\n");
    assert_eq!(r.errors[0].kind, ErrKind::Budget, "{:?}", r.errors);
    let r = run(&mut e, &mut js, "await ticks(1);\nlog('fine again');\n");
    assert!(r.passed(), "{r:?}");
}

#[test]
fn an_allocation_bomb_runs_out_of_memory() {
    let (mut e, _log) = editor();
    let mut js = Js::with_memory(16 << 20);
    js.set_limits(Limits { budget: 1 << 40 });
    let r = run(
        &mut e,
        &mut js,
        "const a = [];\nfor (;;) a.push(new Array(1e5).fill(1));\n",
    );
    assert!(!r.passed());
    assert_eq!(r.errors[0].kind, ErrKind::Memory, "{:?}", r.errors);
    assert!(r.errors[0].message.contains("16 MiB"), "{}", r.errors[0]);
    let r = run(&mut e, &mut js, "log([1, 2, 3].map(x => x * 2).join());\n");
    assert!(r.passed(), "{r:?}");
}

#[test]
fn deep_recursion_is_a_range_error() {
    let (mut e, _log) = editor();
    let r = run(
        &mut e,
        &mut Js::new(),
        "function deep(n) { return deep(n + 1) + 1; }\ndeep(0);\n",
    );
    assert_eq!(r.errors[0].kind, ErrKind::Runtime);
    assert!(r.errors[0].message.starts_with("RangeError"), "{}", r.errors[0]);
    assert_eq!(r.errors[0].line, 1);
}

#[test]
fn syntax_errors_have_a_place() {
    let mut js = Js::new();
    let e = js.check("check.js", "let a = 1;\nlet b = (;\n").unwrap_err();
    assert_eq!(e.kind, ErrKind::Syntax);
    assert_eq!((e.line, e.column), (2, 10), "{e}");
    assert!(e.to_string().starts_with("check.js:2:10  SyntaxError: "), "{e}");
    assert!(e.to_string().ends_with(" 2 | let b = (;\n   |          ^"), "{e}");
    assert!(js.check("ok.js", "await ticks(1);\n").is_ok());
}

#[test]
fn the_games_refusals_are_widget_errors() {
    let (mut e, log) = editor();
    let lamp = add(&mut e, *b"bulb", 300, 200);
    label(&mut e, lamp, "Lamp");
    let src = r#"
try {
  part("Lamb");
} catch (x) {
  log(x instanceof WidgetError, x.name, x.code, x.part, x.suggestions.length > 0);
}
const lamp = part("Lamb");
"#;
    let r = run(&mut e, &mut Js::new(), src);
    let text = log.text();
    assert!(text.contains("true WidgetError UnknownPart Lamb true"), "{text}");
    let err = &r.errors[0];
    assert_eq!(err.kind, ErrKind::Host);
    assert!(err.message.contains("Did you mean"), "{err}");
    assert_eq!((err.line, err.column, err.width), (7, 19, 6), "{err}");
    // A value of the wrong kind, and a part that has no value of its own.
    let r = run(&mut e, &mut Js::new(), "part(\"Lamp\").value = \"hi\";\n");
    assert!(r.errors[0].message.contains("not the text \"hi\""), "{}", r.errors[0]);
}

#[test]
fn until_waits_and_times_out() {
    let (mut e, log) = editor();
    half_adder(&mut e);
    let src = r#"
run();
const sum = part("Sum");
part("A").value = true;
const n = await until(() => sum.on, { max: 30 });
log(`took ${n}`);
try {
  await until(() => part("Carry").on, { max: 10 });
} catch (x) {
  log(x.code, x.message);
}
"#;
    let r = run(&mut e, &mut Js::new(), src);
    assert!(r.passed(), "{r:?}\n{}", log.text());
    let text = log.text();
    assert!(text.contains("took 1") || text.contains("took 2"), "{text}");
    assert!(
        text.contains("Timeout until(() => part(\"Carry\").on) didn't happen within 10 ticks"),
        "{text}"
    );
}

#[test]
fn on_change_sees_each_change() {
    let (mut e, log) = editor();
    half_adder(&mut e);
    let src = r#"
const seen = [];
const off = onChange("Sum", (v, t) => seen.push(v));
run();
for (const v of [true, false, true]) {
  part("A").value = v;
  await ticks(3);
}
off();
part("A").value = false;
await ticks(3);
assertEqual(seen, [true, false, true]);
"#;
    let r = run(&mut e, &mut Js::new(), src);
    assert!(r.passed(), "{r:?}\n{}", log.text());
}

#[test]
fn widget_scripts_cant_open_widgets() {
    let (mut e, _log) = editor();
    let mut js = Js::new();
    let r = run_headless(&mut js, "w.js", "loadPuzzle(1);\n", Caps::EMBEDDED, &mut e).unwrap();
    assert_eq!(r.errors[0].kind, ErrKind::Host);
    assert!(
        r.errors[0].message.contains("can't open other widgets"),
        "{}",
        r.errors[0]
    );
}

#[test]
fn the_console_shows_values() {
    let (mut e, log) = editor();
    half_adder(&mut e);
    let mut js = Js::new();
    assert_eq!(js.repl("1 + 2", &mut e).unwrap().as_deref(), Some("3"));
    assert_eq!(js.repl("let s = part('Sum')", &mut e).unwrap(), None);
    assert_eq!(js.repl("s.kind", &mut e).unwrap().as_deref(), Some("LightBulb"));
    assert_eq!(
        js.repl("[s.on, {a: 1}, 'x']", &mut e).unwrap().as_deref(),
        Some("[false, { a: 1 }, \"x\"]")
    );
    assert_eq!(
        js.repl("String(s)", &mut e).unwrap().as_deref(),
        Some("Part \"Sum\" (Light Bulb #5)")
    );
    let err = js.repl("part('Summ')", &mut e).unwrap_err();
    assert_eq!(err.kind, ErrKind::Host);
    assert_eq!((err.file.as_str(), err.line, err.column), ("console", 1, 6), "{err}");
    // A line that waits: its value comes in the log when it's there.
    assert_eq!(js.repl("run(); await ticks(2); clock.ticks", &mut e).unwrap(), None);
    e.wait(100).unwrap();
    js.frame(&mut e, 100);
    assert!(
        log.text().lines().any(|l| l.parse::<u32>().is_ok_and(|n| n >= 2)),
        "{}",
        log.text()
    );
}

#[test]
fn completion_comes_from_the_api_and_the_widget() {
    let (mut e, _log) = editor();
    half_adder(&mut e);
    let js = Js::new();
    assert_eq!(js.complete("pa", 2, &e), ["part", "parts", "pause"]);
    assert_eq!(js.complete("clock.ti", 8, &e), ["ticks"]);
    assert_eq!(js.complete("part(\"Sum\").inp", 15, &e), ["input"]);
    assert_eq!(js.complete("const x = part(\"Ca", 18, &e), ["Carry"]);
    assert!(js.complete("part(\"", 6, &e).contains(&"Sum".to_owned()));
}

#[test]
fn the_declarations_are_in_sync() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/openwgt-script.d.ts");
    let now = api::declarations();
    if std::env::var_os("UPDATE_DTS").is_some() {
        std::fs::write(path, &now).unwrap();
    }
    let kept = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        kept == now,
        "openwgt-script.d.ts is out of date: run UPDATE_DTS=1 cargo test -p script-js"
    );
    // Every global the prelude defines is declared, and the other way round.
    let (mut e, log) = editor();
    let names: Vec<&str> = api::ITEMS
        .iter()
        .filter(|i| i.of == api::Of::Global)
        .map(|i| i.name)
        .collect();
    let src = format!(
        "const names = {names:?};\nfor (const n of names) assert(n in globalThis, n);\n\
         const own = Object.getOwnPropertyNames(globalThis).filter(n => !Object.getOwnPropertyDescriptor(globalThis, n).enumerable && /^[a-z]/.test(n) && !names.includes(n));\n\
         log(own.join());\n"
    );
    let r = run(&mut e, &mut Js::new(), &src);
    assert!(r.passed(), "{r:?}");
    let builtins = [
        "globalThis",
        "parseInt",
        "parseFloat",
        "isNaN",
        "isFinite",
        "decodeURI",
        "decodeURIComponent",
        "encodeURI",
        "encodeURIComponent",
        "escape",
        "unescape",
        "eval",
        "queueMicrotask",
        "undefined",
    ];
    let extra: Vec<String> = log
        .text()
        .split(',')
        .map(str::to_owned)
        .filter(|n| !n.is_empty() && !builtins.contains(&n.as_str()))
        .collect();
    assert!(extra.is_empty(), "undeclared globals: {extra:?}");
}

/// The plan's example 1, in the game's frames (as in its window).
#[test]
fn example_1_runs_frame_by_frame() {
    let (mut e, log) = editor();
    half_adder(&mut e);
    let sum = e.resolve(&engine::script::select::parse("Sum").unwrap().part).unwrap();
    e.set_name(sum, Some("Lamp")).unwrap();
    let a = e.resolve(&engine::script::select::parse("A").unwrap().part).unwrap();
    let src = r#"const lamp = part("Lamp");                 // the Light Bulb under the label "Lamp"
const gate = lamp.source();                // whatever is wired into it
graph(lamp, gate.input(0), gate.input(1)); // three traces in one window
let flips = 0;
onChange(lamp, (on, t) => {
  log(`tick ${t}: lamp ${on ? "on" : "off"}, inputs ${gate.input(0)} / ${gate.input(1)}`);
  if (++flips === 5) pause();              // then scrub back on the timeline
});
run();
"#;
    let mut js = Js::new();
    let task = js.start("flicker.js", src, Caps::ALL, &mut e).unwrap();
    for i in 0..120 {
        if i % 20 == 10 {
            let on = (i / 20) % 2 == 0;
            e.set_value(a, &engine::script::Val::Bool(on)).unwrap();
        }
        e.wait(16).unwrap();
        js.frame(&mut e, 16);
    }
    let info = js.tasks().into_iter().find(|t| t.id == task).unwrap();
    assert_eq!(
        info.state,
        engine::script::runtime::TaskState::Done,
        "{:?}",
        js.errors(task)
    );
    let text = log.text();
    let flips = text.lines().filter(|l| l.starts_with("tick ")).count();
    assert_eq!(flips, 6, "{text}");
    assert!(text.contains(": lamp on, inputs true / false"), "{text}");
    assert_eq!(text.matches("graph(): ").count(), 1, "{text}");
    assert_eq!(text.matches("pause(): ").count(), 1, "{text}");
    // Stop Scripts ends the watchers too.
    js.cancel(None);
    e.set_value(a, &engine::script::Val::Bool(true)).unwrap();
    e.wait(50).unwrap();
    js.frame(&mut e, 50);
    assert_eq!(log.text().lines().filter(|l| l.starts_with("tick ")).count(), 6);
}

#[test]
fn pause_and_slow_motion_with_the_debugging_tools() {
    let (mut e, log) = editor();
    e.set_enhanced(true);
    e.set_debug_tools(true);
    half_adder(&mut e);
    let src = r"
run();
await ticks(2);
pause();
assert(clock.paused, 'held');
resume();
assert(!clock.paused, 'going on');
assertEqual(speed(0.3), 0.25, 'the nearest speed');
assertEqual(speed(1), 1);
";
    let r = run(&mut e, &mut Js::new(), src);
    assert!(r.passed(), "{r:?}\n{}", log.text());
}

/// `docs/scripting.md`'s tables of the API are this one's
/// (`UPDATE_DOCS=1 cargo test -p script-js` writes them again).
#[test]
fn the_docs_list_the_api() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/scripting.md");
    let doc = std::fs::read_to_string(path).unwrap();
    let now = engine::script::api::fill(&doc, "javascript", &api::markdown()).expect("the marks");
    if std::env::var_os("UPDATE_DOCS").is_some() {
        std::fs::write(path, &now).unwrap();
    } else {
        assert!(
            doc == now,
            "docs/scripting.md's JavaScript API is out of date: UPDATE_DOCS=1 cargo test -p script-js"
        );
    }
}
