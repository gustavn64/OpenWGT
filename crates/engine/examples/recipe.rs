//! Builds widgets from recipes: OpenWDATA's sample widgets and puzzles
//! (`tools/opendata/samples`, `tools/opendata/puzzles`) into the pack
//! (`crates/engine/opendata`: `samples/*.WGT`, `WIDGET.PZL`).
//!
//! `cargo run -p engine --example recipe -- tools/opendata crates/engine/opendata`
//! `cargo run -p engine --example recipe -- --parts [TAG...]`   (the parts' knots)
//!
//! A recipe, a command a line (`#` starts a comment; text in "quotes",
//! `\n` a line break):
//!
//! ```text
//! title "Light the Lamp"   a puzzle's title (puzzles only)
//! background 701           the worksheet's background (sheet 700..702 or 0)
//! part s swit 100 100      a part named s: its tag, where (worksheet h, v)
//! wire s 0 b 0             from s's knot 0 to b's knot 0
//! radio s 0 1              radio group 0 set to 1
//! number s 0 42            number field 0
//! check s 3 on             check box 3
//! label s 0 "Hello"        text field 0 (of 60 bytes)
//! text s "Line 1\nLine 2"  the one long text (the billboards')
//! lock s                   a puzzle's lock on the part (or `lock wire N`)
//! limit bulb 2             the tray hands out 2 lamps (0: none)
//! ```
//!
//! Puzzles are numbered by their files' order (`01-name.txt`, ...).

use std::path::{Path, PathBuf};

use engine::assets::{Assets, Packs};
use engine::doc::{Document, new_gadget};
use engine::gadget::Tag;

/// A recipe's words: quoted texts kept whole.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if quoted {
                    out.push(std::mem::take(&mut cur));
                }
                quoted = !quoted;
            }
            '\\' if quoted && chars.peek() == Some(&'n') => {
                chars.next();
                cur.push('\n');
            }
            '#' if !quoted => break,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn tag(s: &str) -> Tag {
    let b = s.as_bytes();
    assert!(b.len() == 4, "a tag is four characters: {s}");
    Tag([b[0], b[1], b[2], b[3]])
}

/// Windows-1252 bytes of `s`, line breaks as `\r\n`.
fn ansi(s: &str) -> Vec<u8> {
    s.replace('\n', "\r\n")
        .chars()
        .map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?'))
        .collect()
}

/// The widget of `recipe`, and its title.
fn build(assets: &Assets, recipe: &str, file: &Path) -> (Document, Option<String>) {
    let mut doc = Document::new();
    let mut names: Vec<(String, usize)> = Vec::new();
    let mut title = None;
    let find = |names: &Vec<(String, usize)>, n: &str, line: usize| -> usize {
        names
            .iter()
            .find(|(m, _)| m == n)
            .map_or_else(|| panic!("{}:{line}: no part {n}", file.display()), |(_, i)| *i)
    };
    for (n, line) in recipe.lines().enumerate() {
        let w = words(line);
        let Some(cmd) = w.first() else { continue };
        let line = n + 1;
        let num = |i: usize| -> i32 {
            w.get(i)
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(|| panic!("{}:{line}: a number expected", file.display()))
        };
        match cmd.as_str() {
            "title" => title = w.get(1).cloned(),
            "background" => doc.background = u16::try_from(num(1)).unwrap(),
            "part" => {
                let id = doc.new_id();
                let g = new_gadget(assets, tag(&w[2]), id, num(3), num(4));
                names.push((w[1].clone(), doc.gadgets.len()));
                doc.gadgets.push(g);
            }
            "wire" => {
                let (a, b) = (find(&names, &w[1], line), find(&names, &w[3], line));
                let ka = usize::try_from(num(2)).unwrap();
                let kb = usize::try_from(num(4)).unwrap();
                doc.connect(a, ka, b, kb, Vec::new(), 0)
                    .unwrap_or_else(|| panic!("{}:{line}: no such knots", file.display()));
            }
            "lock" if w.get(1).map(String::as_str) == Some("wire") => {
                let i = usize::try_from(num(2)).unwrap();
                doc.wires
                    .get_mut(i)
                    .unwrap_or_else(|| panic!("{}:{line}: no wire {i}", file.display()))
                    .flag = 1;
            }
            "lock" => {
                let i = find(&names, &w[1], line);
                doc.gadgets[i].core.locked = true;
            }
            "limit" => doc.tray_limits.push((tag(&w[1]), i16::try_from(num(2)).unwrap())),
            "radio" | "number" | "check" | "label" | "text" => {
                let i = find(&names, &w[1], line);
                let g = &mut doc.gadgets[i];
                let s = &mut g.core.settings;
                match cmd.as_str() {
                    "radio" => s.radios[usize::try_from(num(2)).unwrap()] = u8::try_from(num(3)).unwrap(),
                    "number" => s.numbers[usize::try_from(num(2)).unwrap()] = u32::try_from(num(3)).unwrap(),
                    "check" => {
                        let bit = 1u32 << num(2);
                        if w.get(3).map(String::as_str) == Some("on") {
                            s.checks |= bit;
                        } else {
                            s.checks &= !bit;
                        }
                    }
                    "label" => s.set_label(usize::try_from(num(2)).unwrap(), &ansi(&w[3])),
                    _ => s.set_long_text(&ansi(&w[2])),
                }
                g.behavior.settings_changed(&mut g.core);
                g.behavior.layout(&mut g.core, assets);
            }
            _ => panic!("{}:{line}: unknown command {cmd}", file.display()),
        }
    }
    (doc, title)
}

/// Recipes in `dir`, in name order.
fn recipes(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "txt"))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let data = engine::opendata::builtin();
    let packs = Packs {
        graphics: None,
        sounds: None,
        data: Some(&data),
    };
    let assets = Assets::load_with(None, packs, None).unwrap();
    if args.first().map(String::as_str) == Some("--parts") {
        for class in engine::gadgets::CLASSES {
            let name = String::from_utf8_lossy(&class.tag.0).into_owned();
            if args.len() > 1 && !args[1..].contains(&name) {
                continue;
            }
            let g = new_gadget(&assets, class.tag, 1, 0, 0);
            let knots: Vec<String> = g
                .core
                .knots
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    let kind = if k.def.kind == formats::tables::KnotKind::Output {
                        "out"
                    } else {
                        "in"
                    };
                    format!("{i}:{kind}@({},{})", k.def.at.h, k.def.at.v)
                })
                .collect();
            let r = g.core.rect();
            println!(
                "{name} {} size {}x{} knots {}",
                class.name,
                r.width(),
                r.height(),
                knots.join(" ")
            );
        }
        return;
    }
    let (src, out) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    let samples = out.join(engine::opendata::SAMPLE_DIR);
    std::fs::create_dir_all(&samples).unwrap();
    for p in recipes(&src.join("samples")) {
        let (doc, _) = build(&assets, &std::fs::read_to_string(&p).unwrap(), &p);
        let name = format!("{}.WGT", p.file_stem().unwrap().to_string_lossy().to_uppercase());
        std::fs::write(samples.join(&name), doc.save()).unwrap();
        println!("sample {name}: {} parts, {} wires", doc.gadgets.len(), doc.wires.len());
    }
    let mut puzzles = Vec::new();
    for (n, p) in recipes(&src.join("puzzles")).iter().enumerate() {
        let (doc, title) = build(&assets, &std::fs::read_to_string(p).unwrap(), p);
        let title = title.unwrap_or_else(|| panic!("{}: no title", p.display()));
        println!("puzzle {}: {title}: {} parts", n + 1, doc.gadgets.len());
        puzzles.push((i16::try_from(n + 1).unwrap(), title.into_bytes(), doc.save()));
    }
    if !puzzles.is_empty() {
        let list: Vec<(i16, &[u8], &[u8])> = puzzles.iter().map(|(i, t, w)| (*i, &t[..], &w[..])).collect();
        std::fs::write(out.join(engine::assets::PUZZLE_FILE), formats::pzl::write(&list)).unwrap();
    }
}
