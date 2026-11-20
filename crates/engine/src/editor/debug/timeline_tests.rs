//! Going back in a run: the record, seeks by running anew, the run going
//! on as it went until new input, and the determinism they rest on, over
//! the sample widgets and the puzzles.

use super::control::cmd;
use super::tests::{add, editor, index};
use super::window::Kind;
use crate::editor::{Button, Editor, TickEnd, menu::BAR_H};
use crate::script::{Host, Val, select};

fn tools() -> Editor {
    let mut e = editor();
    e.set_enhanced(true);
    e.set_debug_tools(true);
    e
}

/// What a tick leaves: the clock, the random numbers, every part's own
/// state (and place), what's on the wires, and the worksheet as drawn.
#[derive(Debug, Clone, PartialEq)]
struct Print {
    ticks: u32,
    rng: u64,
    cores: Vec<String>,
    wires: Vec<bool>,
    picture: Vec<u8>,
}

fn print(e: &mut Editor) -> Print {
    // Drawn without the tools' own (the pointer's note, the status).
    e.debug.tools = false;
    e.draw();
    e.debug.tools = true;
    Print {
        ticks: e.env.ticks,
        rng: e.env.rng,
        cores: e.doc.gadgets.iter().map(|g| format!("{:?}", g.core)).collect(),
        wires: e.doc.wires.iter().map(|w| w.changed).collect(),
        picture: e.surface.pixels.clone(),
    }
}

/// A small random number generator for the player's input.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, n: u32) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        u32::try_from((self.0 >> 33) % u64::from(n.max(1))).unwrap_or(0)
    }
}

/// A tick with the player's input before it: the pointer moving, clicks
/// on parts, keys, the clock going.
fn play_tick(e: &mut Editor, r: &mut Lcg, held: &mut Option<u16>) -> TickEnd {
    let t = e.env.ticks;
    e.env.local_time = (12, t / 3600 % 60, t / 60 % 60);
    if let Some(k) = held.take() {
        e.key_up(k);
    }
    let view = e.view();
    if r.next(8) == 0 {
        let p = (
            r.next(u32::try_from(view.x1).unwrap()),
            r.next(u32::try_from(view.y1).unwrap()),
        );
        e.mouse_move(i32::try_from(p.0).unwrap(), i32::try_from(p.1).unwrap() + BAR_H);
    }
    if r.next(25) == 0 && !e.doc.gadgets.is_empty() {
        let n = u32::try_from(e.doc.gadgets.len()).unwrap();
        let g = usize::try_from(r.next(n)).unwrap();
        let rect = e.doc.gadgets[g].core.rect();
        let p = (i32::midpoint(rect.x0, rect.x1), i32::midpoint(rect.y0, rect.y1) + BAR_H);
        let on_bar = e.main_bar.rect().contains(p.0, p.1 - BAR_H);
        if !on_bar && view.contains(p.0, p.1 - BAR_H) {
            e.mouse_down(p.0, p.1, Button::Left);
            e.mouse_up(p.0, p.1, Button::Left);
        }
    }
    if r.next(30) == 0 {
        let keys = [
            0x20, 0x25, 0x26, 0x27, 0x28, 0x30, 0x31, 0x35, 0x41, 0x44, 0x53, 0x57, 0x5a,
        ];
        let k = keys[usize::try_from(r.next(13)).unwrap()];
        e.key_down(k);
        *held = Some(k);
    }
    e.tick_once()
}

/// Runs `e`'s widget `n` ticks with random input, the prints at `at`.
fn record(e: &mut Editor, seed: u64, n: u32, at: &[u32]) -> Vec<Print> {
    let mut r = Lcg(seed);
    let mut held = None;
    e.run();
    let mut prints = Vec::new();
    for _ in 0..n {
        if at.contains(&e.env.ticks) {
            prints.push(print(e));
        }
        if let TickEnd::Stopped { .. } = play_tick(e, &mut r, &mut held) {
            break;
        }
    }
    if at.contains(&e.env.ticks) {
        prints.push(print(e));
    }
    prints
}

/// Each tick of the run recorded comes back the same by a seek.
fn check_widget(e: &mut Editor, what: &str, seed: u64) {
    let at = [0, 1, 150, 300, 600];
    let prints = record(e, seed, 600, &at);
    if prints.len() < at.len() {
        // A part ended the run: the ticks recorded still come back.
        assert!(!e.running || e.env.ticks < 600, "{what}");
    }
    let end = e.debug.timeline.end;
    for p in &prints {
        e.seek(p.ticks).unwrap_or_else(|err| panic!("{what}: {err:?}"));
        assert!(e.running, "{what}");
        let now = print(e);
        assert_eq!(now.ticks, p.ticks, "{what}");
        assert_eq!(now.rng, p.rng, "{what} at {}", p.ticks);
        assert_eq!(now.cores, p.cores, "{what} at {}", p.ticks);
        assert_eq!(now.wires, p.wires, "{what} at {}", p.ticks);
        assert!(
            now.picture == p.picture,
            "{what} at {}: the worksheet looks different",
            p.ticks
        );
        assert_eq!(e.debug.timeline.end, end, "{what}: a seek keeps the record");
    }
    // From the start the run goes on as it went, tick by tick.
    if let Some(p) = prints.iter().find(|p| p.ticks == 300) {
        e.seek(0).unwrap();
        for _ in 0..300 {
            e.command(cmd::STEP_TICK);
        }
        let now = print(e);
        assert_eq!(now.cores, p.cores, "{what}: replayed to 300");
        assert!(now.picture == p.picture, "{what}: replayed to 300 looks different");
    }
}

#[test]
fn every_sample_and_puzzle_comes_back_the_same() {
    let e = tools();
    let samples = e.assets.samples.clone();
    assert_ne!(samples.len(), 0);
    let mut checked = 0;
    for (i, (name, bytes)) in samples.iter().enumerate() {
        let mut e = tools();
        e.open_widget(name, bytes).unwrap();
        check_widget(&mut e, name, 7 + i as u64);
        checked += 1;
    }
    let numbers: Vec<i16> = crate::puzzle::open(&e.assets)
        .map(|f| f.puzzles.iter().map(|p| p.id).collect())
        .unwrap_or_default();
    for n in numbers {
        let mut e = tools();
        if e.load_puzzle(n) {
            check_widget(&mut e, &format!("puzzle {n}"), u64::from(n.unsigned_abs()));
            checked += 1;
        }
    }
    assert!(checked >= samples.len());
}

#[test]
fn new_input_after_going_back_drops_the_rest() {
    let mut e = tools();
    let sw = add(&mut e, *b"swit", 150, 280);
    e.graph(&select::parse("Switch").unwrap()).unwrap();
    e.run();
    e.step(100).unwrap();
    e.press(sw, None).unwrap();
    e.step(100).unwrap();
    assert_eq!(e.debug.timeline.end, 200);
    assert_eq!(e.debug.observed.probes[0].changes.len(), 2);
    // Back to 50: the switch off again, the record kept.
    e.seek(50).unwrap();
    assert_eq!(e.env.ticks, 50);
    assert!(e.debug.paused());
    assert_eq!(e.value(&select::parse("Switch").unwrap()).unwrap(), Val::Bool(false));
    assert_eq!(e.debug.timeline.end, 200);
    // On over the recorded press: it comes again.
    e.step(60).unwrap();
    assert_eq!(e.value(&select::parse("Switch").unwrap()).unwrap(), Val::Bool(true));
    assert_eq!(e.debug.observed.probes[0].changes.len(), 2, "not recorded twice");
    // Back again, and a click of the player's: the run goes its own way.
    e.seek(150).unwrap();
    let c = e.doc.gadgets[index(&e, sw)].core.rect();
    let p = (i32::midpoint(c.x0, c.x1), i32::midpoint(c.y0, c.y1) + BAR_H);
    e.mouse_down(p.0, p.1, Button::Left);
    e.mouse_up(p.0, p.1, Button::Left);
    assert_eq!(e.debug.timeline.end, 150);
    assert_eq!(e.debug.observed.probes[0].changes.len(), 2);
    e.step(10).unwrap();
    assert_eq!(e.debug.timeline.end, 160);
    assert_eq!(e.value(&select::parse("Switch").unwrap()).unwrap(), Val::Bool(false));
    // The new way is what's recorded now.
    e.seek(155).unwrap();
    assert_eq!(e.value(&select::parse("Switch").unwrap()).unwrap(), Val::Bool(false));
    e.seek(149).unwrap();
    assert_eq!(e.value(&select::parse("Switch").unwrap()).unwrap(), Val::Bool(true));
}

#[test]
fn after_stop_a_seek_runs_again_unless_the_widget_changed() {
    let mut e = tools();
    let sw = add(&mut e, *b"swit", 150, 280);
    e.run();
    e.step(10).unwrap();
    e.press(sw, None).unwrap();
    e.step(10).unwrap();
    e.stop();
    e.seek(15).unwrap();
    assert!(e.running);
    assert_eq!(e.env.ticks, 15);
    assert_eq!(e.value(&select::parse("Switch").unwrap()).unwrap(), Val::Bool(true));
    e.stop();
    // Changed: no going back.
    add(&mut e, *b"bulb", 380, 280);
    let err = e.seek(5).unwrap_err();
    assert!(err.message.contains("changed"), "{err:?}");
    // GO: a new record.
    e.run();
    assert_eq!(e.debug.timeline.end, 0);
    // Without the tools there's none.
    let mut plain = editor();
    plain.run();
    assert!(plain.seek(0).is_err());
}

#[test]
fn the_timeline_window_and_the_graph_seek() {
    let mut e = tools();
    let sw = add(&mut e, *b"swit", 150, 280);
    e.graph(&select::parse("Switch").unwrap()).unwrap();
    e.command(cmd::TIMELINE);
    assert!(e.debug.window(Kind::Timeline).is_some());
    e.run();
    e.step(50).unwrap();
    e.press(sw, None).unwrap();
    e.step(50).unwrap();
    e.draw();
    let w = e.debug.window(Kind::Timeline).unwrap().rect;
    let font = crate::editor::ui::font(&e.assets).unwrap().clone();
    let l = super::timeline::layout(w, &font);
    let click = |e: &mut Editor, r: crate::gfx::IRect| {
        e.mouse_down(r.x0 + 4, r.y0 + 4, Button::Left);
        e.mouse_up(r.x0 + 4, r.y0 + 4, Button::Left);
    };
    click(&mut e, l.first);
    assert_eq!(e.env.ticks, 0);
    click(&mut e, l.on);
    click(&mut e, l.on);
    assert_eq!(e.env.ticks, 2);
    click(&mut e, l.back);
    assert_eq!(e.env.ticks, 1);
    click(&mut e, l.last);
    assert_eq!(e.env.ticks, 100);
    // The bar: halfway.
    let bar = l.bar.inset(2);
    let mid = i32::midpoint(bar.x0, bar.x1);
    e.mouse_down(mid, bar.y0 + 3, Button::Left);
    e.mouse_up(mid, bar.y0 + 3, Button::Left);
    assert!((49..=51).contains(&e.env.ticks), "{}", e.env.ticks);
    e.draw();
    assert!(e.debug.status().unwrap().contains(" / 100"), "{:?}", e.debug.status());
    // A click on the graph's plot: the tick under it.
    let g = e.debug.window(Kind::Graph).unwrap().rect;
    let plot = super::graph::layout(g, &font).plot.inset(2);
    let view = e.debug.graph.view(plot, e.debug.time);
    let want = view.tick(plot, plot.x0 + 10).round();
    // (Below the timeline window, which is over its top.)
    e.mouse_down(plot.x0 + 10, plot.y1 - 10, Button::Left);
    e.mouse_up(plot.x0 + 10, plot.y1 - 10, Button::Left);
    assert_eq!(f64::from(e.env.ticks), want.clamp(0.0, 100.0));
}

#[test]
fn script_changes_and_externals_are_played_again() {
    let mut e = tools();
    let k = add(&mut e, *b"Cnst", 150, 280);
    let delivered = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let d = delivered.clone();
    e.set_redeliver(Box::new(move |_doc, _assets, env, id, payload| {
        d.borrow_mut().push((env.ticks, id, payload.to_vec()));
    }));
    e.run();
    e.step(5).unwrap();
    e.poke(k, &Val::Number(42.0)).unwrap();
    e.record_external(k.0, b"hello");
    e.step(5).unwrap();
    let before = e.doc.gadgets[0].core.number;
    assert_eq!(before, 42.0);
    e.seek(3).unwrap();
    assert_ne!(e.doc.gadgets[0].core.number, 42.0);
    e.seek(10).unwrap();
    assert_eq!(e.doc.gadgets[0].core.number, 42.0);
    assert_eq!(*delivered.borrow(), [(5, k.0, b"hello".to_vec())]);
}

/// How fast a seek re-simulates: `cargo test --release -p engine --lib
/// -- --ignored seek_speed --nocapture`.
#[test]
#[ignore = "a measurement, for --release"]
fn seek_speed() {
    let e = tools();
    for (name, bytes) in e.assets.samples.clone() {
        let mut e = tools();
        e.open_widget(&name, &bytes).unwrap();
        record(&mut e, 1, 3600, &[]);
        let t = std::time::Instant::now();
        e.seek(3600).unwrap();
        let s = t.elapsed().as_secs_f64();
        eprintln!(
            "{name}: 3600 ticks (a minute) in {:.0} ms, {:.0} ticks/s",
            s * 1000.0,
            3600.0 / s
        );
    }
}
