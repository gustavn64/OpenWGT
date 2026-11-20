//! Probes, the graph, the watch list, breakpoints and the popup menu.

use super::control::cmd;
use super::tests::{add, connect, editor, index};
use super::window::Kind;
use crate::editor::menu::BAR_H;
use crate::editor::{Button, Editor};
use crate::plot::Sample;
use crate::script::{Host, PartId, select};

const VK_CONTROL: u16 = 0x11;
const VK_DOWN: u16 = 0x28;
const VK_RETURN: u16 = 0x0d;

fn tools() -> Editor {
    let mut e = editor();
    e.set_enhanced(true);
    e.set_debug_tools(true);
    e
}

fn pass(e: &mut Editor, ms: u64) {
    let mut left = ms;
    while left > 0 {
        let step = left.min(16);
        e.update(step, (12, 0, 0));
        left -= step;
    }
}

fn centre(e: &Editor, id: PartId) -> (i32, i32) {
    let r = e.doc.gadgets[index(e, id)].core.rect();
    (i32::midpoint(r.x0, r.x1), i32::midpoint(r.y0, r.y1) + BAR_H)
}

fn sel(s: &str) -> crate::script::Selector {
    select::parse(s).unwrap()
}

/// A metronome into a counter.
fn metronome(e: &mut Editor) -> (PartId, PartId) {
    let m = add(e, *b"Metr", 150, 250);
    let c = add(e, *b"Cntr", 380, 250);
    connect(e, m, 0, c, 0);
    (m, c)
}

#[test]
fn a_probe_records_each_change() {
    let mut e = tools();
    let sw = add(&mut e, *b"swit", 150, 280);
    e.graph(&sel("Switch")).unwrap();
    assert!(e.debug.window(Kind::Graph).is_some());
    e.run();
    e.step(10).unwrap();
    e.press(sw, None).unwrap();
    e.step(5).unwrap();
    e.press(sw, None).unwrap();
    e.step(5).unwrap();
    let p = &e.debug.observed.probes[0];
    assert_eq!(p.name, "Switch #1.out");
    assert_eq!(
        p.changes,
        [
            (0, Sample::Bool(false)),
            (11, Sample::Bool(true)),
            (16, Sample::Bool(false))
        ]
    );
    // GO again: a new record.
    e.stop();
    e.run();
    e.step(3).unwrap();
    assert_eq!(e.debug.observed.probes[0].changes, [(0, Sample::Bool(false))]);
}

#[test]
fn a_metronome_draws_a_square_wave_and_a_counter_steps() {
    let mut e = tools();
    let (_, c) = metronome(&mut e);
    e.graph(&sel("Metronome")).unwrap();
    e.graph(&sel(&format!("#{}", c.0))).unwrap();
    e.run();
    e.step(600).unwrap();
    let o = &e.debug.observed;
    let beats = &o.probes[0].changes;
    assert!(beats.len() > 4, "{beats:?}");
    assert!(beats.windows(2).all(|w| w[0].1 != w[1].1), "changes only");
    let counts = &o.probes[1].changes;
    assert!(counts.iter().any(|(_, v)| *v == Sample::Number(2.0)), "{counts:?}");
    // Drawn, inside the window.
    e.draw();
    let w = e.debug.window(Kind::Graph).unwrap().rect;
    assert!(w.x1 <= e.surface.width && w.y0 >= BAR_H);
}

#[test]
fn a_breakpoint_holds_the_run_at_the_tick() {
    let mut e = tools();
    let (m, _) = metronome(&mut e);
    e.graph(&sel("Metronome")).unwrap();
    e.break_when(&crate::script::Cond::Changed(sel("Metronome"))).unwrap();
    e.run();
    pass(&mut e, 5000);
    // Held at its first change, nothing after.
    let first = e.debug.observed.probes[0].changes[1].0;
    assert!(e.debug.paused());
    assert_eq!(e.env.ticks, first);
    assert_eq!(e.debug.observed.breaks[0].hits, 1);
    assert!(e.debug.console.text().contains(&format!("Held at tick {first}")));
    // Resumed, it holds at the next.
    e.command(cmd::PAUSE);
    pass(&mut e, 5000);
    let second = e.debug.observed.probes[0].changes[2].0;
    assert_eq!(e.env.ticks, second);
    // Steps of a second stop there too.
    let mut at = Vec::new();
    for _ in 0..3 {
        e.command(cmd::STEP_SECOND);
        at.push(e.env.ticks);
    }
    let ch = &e.debug.observed.probes[0].changes;
    assert_eq!(at, [second + 60, ch[3].0, ch[4].0], "{ch:?}");
    // A condition: held as it comes to hold.
    let _ = m;
    let mut e = tools();
    let (_, c) = metronome(&mut e);
    let cond = crate::script::Cond::Cmp(
        sel(&format!("#{}", c.0)),
        crate::script::CmpOp::Ge,
        crate::script::Val::Number(3.0),
    );
    e.break_when(&cond).unwrap();
    e.run();
    pass(&mut e, 20_000);
    assert_eq!(
        e.value(&sel(&format!("#{}", c.0))).unwrap(),
        crate::script::Val::Number(3.0)
    );
    assert!(e.debug.paused());
}

#[test]
fn the_commands_reach_the_tools() {
    let mut e = tools();
    metronome(&mut e);
    e.open_window(Kind::Console);
    for line in [
        "graph Metronome Counter",
        "watch Counter",
        "break Counter >= 2",
        "run",
        "step 3",
        "pause",
    ] {
        e.text_input(line);
        e.key_down(VK_RETURN);
    }
    assert_eq!(e.debug.observed.probes.len(), 2);
    assert_eq!(e.debug.observed.watches.len(), 1);
    assert_eq!(e.debug.observed.breaks.len(), 1);
    assert!(e.debug.paused());
    e.text_input("resume");
    e.key_down(VK_RETURN);
    assert!(!e.debug.paused());
    e.draw();
    assert_eq!(e.debug.observed.watches[0].name, "Counter #2.out");
    let now = crate::script::value::format(&e.value(&sel("Counter")).unwrap());
    assert_eq!(e.debug.observed.watches[0].value, now);
    // Without the tools, the commands say why not.
    let mut r = crate::script::cmd::CommandRuntime::new();
    let mut plain = editor();
    add(&mut plain, *b"swit", 150, 280);
    let report =
        crate::script::runtime::run_headless(&mut r, "t", "graph Switch", crate::script::Caps::ALL, &mut plain)
            .unwrap();
    assert!(
        report.errors[0].message.contains("debugging tools are off"),
        "{report:?}"
    );
}

#[test]
fn the_popup_offers_the_tools_for_what_is_under_the_pointer() {
    let mut e = tools();
    let sw = add(&mut e, *b"swit", 150, 280);
    let bulb = add(&mut e, *b"bulb", 380, 280);
    connect(&mut e, sw, 0, bulb, 0);
    let c = centre(&e, sw);
    e.mouse_down(c.0, c.1, Button::Right);
    e.mouse_up(c.0, c.1, Button::Right);
    let pop = e.debug.popup.clone().expect("a popup");
    assert_eq!(pop.title, "Switch #1.out");
    // Down to the first (Show Graph), Enter.
    e.key_down(VK_DOWN);
    e.key_down(VK_RETURN);
    assert!(e.debug.popup.is_none());
    assert_eq!(e.debug.observed.probes.len(), 1);
    assert!(e.debug.window(Kind::Graph).is_some());
    // Watch, by a click on its item.
    e.mouse_down(c.0, c.1, Button::Right);
    e.mouse_up(c.0, c.1, Button::Right);
    let pop = e.debug.popup.clone().unwrap();
    let font = crate::editor::ui::font(&e.assets).unwrap();
    let h = i32::from(font.height) + 5;
    let watch_y = pop.rect.y0 + 2 + 3 * h + 3;
    e.mouse_move(pop.rect.x0 + 10, watch_y);
    e.mouse_down(pop.rect.x0 + 10, watch_y, Button::Left);
    e.mouse_up(pop.rect.x0 + 10, watch_y, Button::Left);
    assert_eq!(e.debug.observed.watches.len(), 1);
    // Elsewhere, a click closes it and does nothing else.
    e.mouse_down(c.0, c.1, Button::Right);
    e.mouse_up(c.0, c.1, Button::Right);
    e.mouse_down(5, 300, Button::Left);
    e.mouse_up(5, 300, Button::Left);
    assert!(e.debug.popup.is_none());
    // Name This Part: the console with the command to finish.
    e.mouse_down(c.0, c.1, Button::Right);
    for _ in 0..5 {
        e.key_down(VK_DOWN);
    }
    e.key_down(VK_RETURN);
    assert_eq!(e.debug.console.input, format!("name #{} ", sw.0));
    e.text_input("Lamp switch");
    e.key_down(VK_RETURN);
    assert_eq!(e.part_name(sw.0), Some("Lamp switch"));
    // Copy Reference: by its name now.
    assert_eq!(e.reference(&sel(&format!("#{}.0", sw.0))), "\"Lamp switch\".out");
    // On a knot (the windows out of the way).
    for id in [cmd::CONSOLE, cmd::GRAPH, cmd::WATCH] {
        e.command(id);
    }
    let b = index(&e, bulb);
    let at = e.doc.gadgets[b].core.knot_at(0).unwrap();
    e.mouse_down(at.0, at.1 + BAR_H, Button::Right);
    assert_eq!(e.debug.popup.as_ref().unwrap().title, "Light Bulb #2.in");
    e.mouse_down(5, 300, Button::Left);
    // Running: the right button is the parts', unless held with Ctrl.
    e.run();
    e.mouse_down(c.0, c.1, Button::Right);
    assert!(e.debug.popup.is_none());
    assert!(e.env.right_down);
    e.mouse_up(c.0, c.1, Button::Right);
    e.key_down(VK_CONTROL);
    e.mouse_down(c.0, c.1, Button::Right);
    e.key_up(VK_CONTROL);
    assert!(e.debug.popup.is_some());
    assert!(!e.env.right_down);
}

#[test]
fn the_watch_list_shows_values_and_takes_them_away() {
    let mut e = tools();
    let sw = add(&mut e, *b"swit", 150, 280);
    e.watch(&sel("Switch")).unwrap();
    e.graph(&sel("Switch")).unwrap();
    e.break_when(&crate::script::Cond::Changed(sel("Switch"))).unwrap();
    e.run();
    e.press(sw, None).unwrap();
    e.draw();
    assert_eq!(e.debug.observed.watches[0].value, "on");
    let w = e.debug.window(Kind::Watch).unwrap();
    let font = crate::editor::ui::font(&e.assets).unwrap().clone();
    let rows = super::watch::rows(w.rect, &font, &e.debug.observed);
    assert_eq!(rows.len(), 6, "three headings, three rows");
    // The watch's ×.
    let (_, x, _) = rows[3];
    e.mouse_down(x.x0 + 2, x.y0 + 2, Button::Left);
    e.mouse_up(x.x0 + 2, x.y0 + 2, Button::Left);
    assert_eq!(e.debug.observed.watches, []);
    assert_eq!(e.debug.observed.probes.len(), 1);
}

#[test]
fn the_graph_zooms_and_drags() {
    let mut e = tools();
    metronome(&mut e);
    e.graph(&sel("Metronome")).unwrap();
    e.run();
    e.step(300).unwrap();
    e.draw();
    let w = e.debug.window(Kind::Graph).unwrap().rect;
    let font = crate::editor::ui::font(&e.assets).unwrap().clone();
    let l = super::graph::layout(w, &font);
    e.mouse_down(l.minus.x0 + 3, l.minus.y0 + 3, Button::Left);
    e.mouse_up(l.minus.x0 + 3, l.minus.y0 + 3, Button::Left);
    assert_eq!(e.debug.graph.zoom, 1);
    // Dragged right: earlier ticks, no longer following.
    let y = l.plot.y0 + 20;
    e.mouse_down(l.plot.x0 + 100, y, Button::Left);
    e.mouse_move(l.plot.x0 + 150, y);
    e.mouse_up(l.plot.x0 + 150, y, Button::Left);
    assert!(e.debug.graph.t0.is_some());
    e.mouse_down(l.follow.x0 + 3, l.follow.y0 + 3, Button::Left);
    e.mouse_up(l.follow.x0 + 3, l.follow.y0 + 3, Button::Left);
    assert_eq!(e.debug.graph.t0, None);
    e.mouse_move(l.plot.x0 + 10, y);
    e.wheel(0, 1);
    assert_eq!(e.debug.graph.zoom, 0);
}
