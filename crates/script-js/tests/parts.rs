//! Scripted parts in JavaScript on the game's test bench: the plan's
//! Smoother, a script that runs too long or throws (the part breaks, not
//! the game), consent, and what a part's script can't reach.

use engine::gadget::{Behavior, Value};
use engine::gadgets::bench::{Bench, Source};
use engine::openparts::scripted::Scripted;
use engine::script::parts::{PartHook, PartRuntime};
use engine::script::trust::Digest;
use formats::tables::KnotKind;
use script_js::parts::JsParts;

const SMOOTHER: &str = r#"
definePart({
  name: "Smoother",                          // shown on a generic "scripted part" body
  inputs: [{ name: "in", kind: "number" }],
  outputs: [{ name: "out", kind: "number" }],
  settings: { window: 10 },
  start(s) { s.last = []; },
  tick(s, { in: x }) {                        // synchronous, per tick, under a small budget
    s.last.push(x); if (s.last.length > this.settings.window) s.last.shift();
    return { out: s.last.reduce((p, q) => p + q, 0) / s.last.length };
  },
});
"#;

/// A bench with the part's knots (its inputs, then its outputs, each
/// wired), the JavaScript runtime, and the part given `code`, allowed to
/// run if `allowed`.
fn bench(code: &str, allowed: bool) -> (Bench, Scripted) {
    let mut rt = JsParts::new();
    let def = rt.define(1, code).unwrap();
    let mut kinds = vec![KnotKind::Input; def.inputs.len()];
    kinds.extend(vec![KnotKind::Output; def.outputs.len()]);
    let mut b = Bench::with_knots(&kinds);
    for k in 0..kinds.len() {
        b.wire(k);
    }
    b.env.parts = PartHook::new(Some(Box::new(rt)));
    if allowed {
        b.env.parts.allow(Digest::of(code));
    }
    let mut s = Scripted::default();
    s.set_code(code.to_owned(), def);
    b.fresh(&mut s);
    (b, s)
}

fn said(b: &Bench) -> String {
    b.env
        .parts
        .with(|p| p.log.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default()
}

#[test]
fn the_smoother_averages_its_last_ten() {
    let (mut b, mut s) = bench(SMOOTHER, true);
    assert_eq!(s.def().name, "Smoother");
    assert_eq!(
        (s.def().inputs.clone(), s.def().outputs.clone()),
        (vec!["in".to_owned()], vec!["out".to_owned()])
    );
    b.start(&mut s);
    assert!(s.allowed());
    for t in 0..30u32 {
        b.source(0, Source::number(f64::from(t)));
        b.tick_at(&mut s, t);
        let first = t.saturating_sub(9);
        let mean = f64::from(first + t) / 2.0;
        assert_eq!(b.out(&s, 1), Value::Number(mean), "tick {t}");
    }
    assert!(b.fired(1));
    // A steady input: the output settles and stops changing.
    for t in 30..45 {
        b.source(0, Source::number(5.0));
        b.tick_at(&mut s, t);
    }
    assert_eq!(b.out(&s, 1), Value::Number(5.0));
    b.fired(1);
    b.tick_at(&mut s, 45);
    assert!(!b.fired(1), "unchanged");
    // GO again: a new state.
    s.reset(&mut b.core);
    b.start(&mut s);
    b.source(0, Source::number(100.0));
    b.tick_at(&mut s, 0);
    assert_eq!(b.out(&s, 1), Value::Number(100.0));
    assert!(!b.core.broken);
}

#[test]
fn a_script_that_runs_too_long_breaks_its_part_not_the_game() {
    let code = r#"definePart({
  name: "Spin", inputs: ["in"], outputs: ["out"],
  tick(s, { in: x }) {
    if (x > 2) { while (true) {} }
    return { out: x };
  },
});"#;
    let (mut b, mut s) = bench(code, true);
    b.start(&mut s);
    for t in 0..3 {
        b.source(0, Source::number(f64::from(t)));
        b.tick_at(&mut s, t);
    }
    assert_eq!(b.out(&s, 1), Value::Number(2.0));
    assert!(!b.core.broken);
    let at = std::time::Instant::now();
    b.source(0, Source::number(3.0));
    b.tick_at(&mut s, 3);
    assert!(at.elapsed().as_secs() < 5, "stopped in time");
    assert!(b.core.broken, "broken");
    let log = said(&b);
    assert!(log.contains("Spin (#1)") && log.contains("ran too long"), "{log}");
    // It stays broken (doing nothing) until the run stops; the game goes on.
    b.tick_at(&mut s, 4);
    assert_eq!(b.out(&s, 1), Value::Number(2.0));
    s.reset(&mut b.core);
    assert!(!b.core.broken, "STOP mends it");
    b.start(&mut s);
    b.source(0, Source::number(1.0));
    b.tick_at(&mut s, 0);
    assert_eq!(b.out(&s, 1), Value::Number(1.0), "and it runs again");
}

#[test]
fn an_exception_breaks_its_part_and_says_where() {
    let code = "definePart({\n  name: \"Bad\", inputs: [\"in\"], outputs: [\"out\"],\n  tick(s, { in: x }) {\n    return { out: x.nope.deeper };\n  },\n});\n";
    let (mut b, mut s) = bench(code, true);
    b.start(&mut s);
    b.source(0, Source::number(1.0));
    b.tick_at(&mut s, 0);
    assert!(b.core.broken);
    let log = said(&b);
    assert!(log.contains("part1.js:4") && log.contains("TypeError"), "{log}");
    // A start that throws breaks it at GO.
    let code = "definePart({ outputs: [\"o\"], start() { throw new Error(\"no\"); }, tick() {} });";
    let (mut b, mut s) = bench(code, true);
    b.start(&mut s);
    assert!(b.core.broken);
    assert!(said(&b).contains("Error: no"));
}

#[test]
fn a_part_whose_script_isnt_allowed_is_inert() {
    let (mut b, mut s) = bench(SMOOTHER, false);
    b.start(&mut s);
    assert!(!s.allowed());
    b.source(0, Source::number(7.0));
    b.tick_at(&mut s, 0);
    assert_eq!(b.out(&s, 1), Value::None, "it gives nothing");
    assert!(!b.core.broken);
    // Allowed: at the next GO it runs.
    b.env.parts.allow(Digest::of(SMOOTHER));
    b.start(&mut s);
    assert!(s.allowed());
    b.tick_at(&mut s, 0);
    assert_eq!(b.out(&s, 1), Value::Number(7.0));
}

#[test]
fn a_parts_script_sees_little_and_outputs_what_wires_carry() {
    let code = r#"definePart({
  inputs: ["a", "b"], outputs: ["seen", "on", "pic"],
  tick(s, { a, b }, { tick }) {
    const seen = [typeof Date, typeof require, typeof fetch, typeof setTimeout, typeof globalThis.engine].join();
    console.log("seen", seen, Math.random() < 1);
    return { seen: seen === "undefined,undefined,undefined,undefined,undefined" ? tick : -1,
             on: a === null, pic: b && b.kind === "picture" ? b.sheet : null };
  },
});"#;
    let (mut b, mut s) = bench(code, true);
    assert_eq!(s.def().name, "Script", "no name: Script");
    b.start(&mut s);
    b.source(1, Source::object(Some(engine::gadget::Object::Picture(1204))));
    b.core.knots[0].wire = None;
    b.tick_at(&mut s, 9);
    assert_eq!(b.out(&s, 2), Value::Number(9.0));
    assert_eq!(b.out(&s, 3), Value::Bool(true), "unwired is null");
    assert_eq!(b.out(&s, 4), Value::Number(1204.0));
    // A string isn't an output's value.
    let code = r#"definePart({ outputs: ["o"], tick() { return { o: "hi" }; } });"#;
    let (mut b, mut s) = bench(code, true);
    b.start(&mut s);
    b.tick_at(&mut s, 0);
    assert!(b.core.broken);
    assert!(said(&b).contains("an output gives a number"), "{}", said(&b));
}

#[test]
fn what_a_script_defines_is_checked() {
    let mut rt = JsParts::new();
    for (code, says) in [
        ("1 + 1", "didn't call definePart"),
        ("definePart({ inputs: ['a'] })", "tick must be a function"),
        (
            "definePart({ inputs: ['a','b','c','d','e'], tick() {} })",
            "at most 4 inputs",
        ),
        (
            "definePart({ inputs: ['a','a'], tick() {} })",
            "two inputs are called \"a\"",
        ),
        ("definePart({ tick() {} }); definePart({ tick() {} })", "called already"),
        ("definePart({ tick() {}", "SyntaxError"),
    ] {
        let e = rt.define(1, code).unwrap_err();
        assert!(e.to_string().contains(says), "{code}: {e}");
    }
    let d = rt
        .define(
            2,
            "definePart({ name: 'Two', inputs: ['x', {name: 'y'}], outputs: [], tick() {} })",
        )
        .unwrap();
    assert_eq!(d.inputs, ["x", "y"]);
    assert_eq!(d.name, "Two");
    // A top level that never ends is stopped too.
    let e = rt.define(3, "while (true) {}").unwrap_err();
    assert!(e.to_string().contains("ran too long"), "{e}");
}
