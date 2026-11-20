//! `openwgt --headless --script FILE.js`: the scripting plan's examples in
//! JavaScript, on widgets made here with the port's own packs.

#![cfg(all(feature = "js", not(target_arch = "wasm32")))]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use engine::assets::{Assets, Packs};
use engine::doc::{self, Document};
use engine::editor::Editor;
use engine::gadget::Tag;
use engine::lang::Lang;
use engine::script::{Host, PartId, SettingKey, SettingVal, Val};

fn folder(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("openwgt-cli-js-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Runs `script` (saved as `name`) headless in `dir`, with `extra` after.
fn run(dir: &Path, name: &str, script: &str, extra: &[&str]) -> Output {
    let file = dir.join(name);
    std::fs::write(&file, script).unwrap();
    Command::new(env!("CARGO_BIN_EXE_openwgt"))
        .args([
            "--open",
            "--lang=en",
            "--no-audio",
            "--headless",
            "--seed",
            "1",
            "--script",
        ])
        .arg(&file)
        .arg("--out")
        .arg(dir.join("out"))
        .args(extra)
        .current_dir(dir)
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("HOME", dir)
        .env("APPDATA", dir)
        .output()
        .unwrap()
}

fn text(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

fn editor() -> Editor {
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
    Editor::new(assets, Document::new(), 640, 433)
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

/// Switches A and B, through Splitters, into an XOR (bulb Sum) and an AND
/// (bulb Carry); `xor` false makes the "XOR" an OR (a wrong adder).
fn half_adder(xor: bool) -> Vec<u8> {
    let mut e = editor();
    let a = add(&mut e, *b"swit", 40, 60);
    let b = add(&mut e, *b"swit", 40, 220);
    let sa = add(&mut e, *b"Spl2", 130, 60);
    let sb = add(&mut e, *b"Spl2", 130, 220);
    let x = add(&mut e, if xor { *b"XorG" } else { *b"OrGa" }, 220, 60);
    let and = add(&mut e, *b"AndG", 220, 220);
    let sum = add(&mut e, *b"bulb", 420, 60);
    let carry = add(&mut e, *b"bulb", 420, 220);
    for (p, t) in [(a, "A"), (b, "B"), (sum, "Sum"), (carry, "Carry")] {
        label(&mut e, p, t);
    }
    connect(&mut e, a, 0, sa, 0);
    connect(&mut e, b, 0, sb, 0);
    for (n, g) in [x, and].into_iter().enumerate() {
        connect(&mut e, sa, n, g, 0);
        connect(&mut e, sb, n, g, 1);
    }
    connect(&mut e, x, 0, sum, 0);
    connect(&mut e, and, 0, carry, 0);
    e.widget_bytes()
}

/// A Switch that starts on into a Win / Lose Checker: the run ends by
/// itself, the puzzle solved (`win`) or not.
fn ends_itself(win: bool) -> Vec<u8> {
    let mut e = editor();
    let s = add(&mut e, *b"swit", 60, 100);
    let w = add(&mut e, *b"Stop", 260, 100);
    connect(&mut e, s, 0, w, 0);
    e.set_value(s, &Val::Bool(true)).unwrap();
    if !win {
        e.set_setting(w, SettingKey::Radio(3), &SettingVal::Int(1)).unwrap();
    }
    e.widget_bytes()
}

/// The plan's example 2, as it is.
const EXAMPLE_2: &str = r#"test("half adder truth table", async () => {
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

/// The plan's example 3, with puzzle 2 for 12: the port's own data has
/// puzzles 1 to 5.
const EXAMPLE_3: &str = r#"loadPuzzle(2); open("solutions/2.wgt");  // CLI capabilities
run();
const n = await until(() => puzzle().ended, { max: 60 * 60 });   // up to a minute
assert(puzzle().solved, `puzzle 2 ended after ${n} ticks but wasn't solved`);
"#;

#[test]
fn example_2_a_half_adder_test() {
    let dir = folder("adder");
    std::fs::write(dir.join("adder.wgt"), half_adder(true)).unwrap();
    let out = run(&dir, "half-adder.test.js", EXAMPLE_2, &["adder.wgt"]);
    assert_eq!(out.status.code(), Some(0), "{}{}", text(&out.stdout), text(&out.stderr));
    assert!(
        text(&out.stdout).contains("\u{2713} half adder truth table (8 ticks)"),
        "{}",
        text(&out.stdout)
    );
    assert!(
        std::fs::read(dir.join("out/half-adder.png"))
            .unwrap()
            .starts_with(b"\x89PNG")
    );
    // A wrong adder: the checks fail where they are, and it exits 1.
    std::fs::write(dir.join("adder.wgt"), half_adder(false)).unwrap();
    let out = run(&dir, "half-adder.test.js", EXAMPLE_2, &["adder.wgt"]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(
        err.contains("half-adder.test.js:8:5  sum for 11: expected false, got true"),
        "{err}"
    );
    assert!(
        err.contains(" 8 |     assertEqual(sum.on, (x ^ y) === 1, `sum for ${x}${y}`);\n   |     ^^^^^^^^^^^"),
        "{err}"
    );
    assert!(err.contains("\u{2717} half adder truth table (8 ticks)"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn example_3_a_puzzle_solution() {
    let dir = folder("puzzle");
    std::fs::create_dir_all(dir.join("solutions")).unwrap();
    std::fs::write(dir.join("solutions/2.wgt"), ends_itself(true)).unwrap();
    let out = run(&dir, "check.js", EXAMPLE_3, &[]);
    assert_eq!(out.status.code(), Some(0), "{}{}", text(&out.stdout), text(&out.stderr));
    std::fs::write(dir.join("solutions/2.wgt"), ends_itself(false)).unwrap();
    let out = run(&dir, "check.js", EXAMPLE_3, &[]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("check.js:4:1  puzzle 2 ended after "), "{err}");
    assert!(err.contains(" ticks but wasn't solved"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn javascript_errors_say_where() {
    let dir = folder("errors");
    let out = run(&dir, "t.js", "log('first');\nlet b = (;\n", &[]);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("t.js:2:10  SyntaxError: "), "{err}");
    assert!(!text(&out.stdout).contains("first"), "nothing ran");
    let out = run(&dir, "t.js", "run();\nwhile (true) {}\n", &["--budget", "100000"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("ran too long without waiting"),
        "{}",
        text(&out.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
