//! The debugging tools in the editor: the switch, the Debug menu's run
//! control, the pointer's note and the status.

use formats::dialog::MenuItem;

use super::control::cmd;
use super::overlay::{HOVER_MS, Target, Tip, show, tip_rect};
use super::tests::{add, connect, editor, index};
use super::{Speed, is_command};
use crate::assets::{Assets, Packs};
use crate::doc::Document;
use crate::editor::menu::BAR_H;
use crate::editor::settings::{Outcome, SettingsBox};
use crate::editor::{Editor, ui};
use crate::gadget::Tag;
use crate::gfx::IRect;
use crate::lang::Lang;
use crate::script::{Host, PartId, Val};
use crate::sim::RunEnd;

const VK_F6: u16 = 0x75;
const VK_F7: u16 = 0x76;

/// An editor with the debugging tools on.
fn debugging() -> Editor {
    let mut e = editor();
    e.set_enhanced(true);
    e.set_debug_tools(true);
    e
}

fn has_debug_menu(e: &Editor) -> bool {
    e.menu.items.iter().any(|p| match p {
        MenuItem::Popup { items, .. } => items
            .iter()
            .any(|i| matches!(i, MenuItem::Command { id, .. } if is_command(*id))),
        _ => false,
    })
}

/// `ms` of real time in frames of 16 ms (and what's left).
fn pass(e: &mut Editor, ms: u64) {
    let mut left = ms;
    while left > 0 {
        let step = left.min(16);
        e.update(step, (12, 0, 0));
        left -= step;
    }
}

/// The pointer still over `p` (worksheet coordinates) long enough, and
/// the frame drawn.
fn hover(e: &mut Editor, p: (i32, i32)) {
    e.mouse_move(p.0, p.1 + BAR_H);
    pass(e, HOVER_MS + 16);
    e.draw();
}

fn centre(e: &Editor, id: PartId) -> (i32, i32) {
    let r = e.doc.gadgets[index(e, id)].core.rect();
    (i32::midpoint(r.x0, r.x1), i32::midpoint(r.y0, r.y1))
}

/// The port's own packs in `lang`.
fn packs(lang: Lang) -> Assets {
    let graphics = crate::opengfx::builtin_for(lang);
    let sounds = crate::opensfx::builtin();
    let data = crate::opendata::builtin_for(lang);
    let packs = Packs {
        graphics: Some(graphics.as_ref()),
        sounds: Some(&sounds),
        data: Some(&data),
    };
    let mut assets = Assets::load_with(None, packs, None).unwrap();
    assets.lang = lang;
    assets
}

/// A switch wired to a light bulb, clear of the toolbars.
fn switch_and_bulb(e: &mut Editor) -> (PartId, PartId) {
    let sw = add(e, *b"swit", 150, 280);
    let bulb = add(e, *b"bulb", 380, 280);
    connect(e, sw, 0, bulb, 0);
    (sw, bulb)
}

#[test]
fn nothing_shows_without_the_switch_or_the_enhancements() {
    for (enhanced, tools) in [(false, false), (true, false), (false, true)] {
        let mut e = editor();
        e.set_enhanced(enhanced);
        e.set_debug_tools(tools);
        let (sw, _) = switch_and_bulb(&mut e);
        e.update(16, (12, 0, 0));
        assert!(!has_debug_menu(&e), "{enhanced} {tools}");
        e.run();
        // F6 goes to the parts; the run goes on.
        e.key_down(VK_F6);
        e.key_up(VK_F6);
        pass(&mut e, 1000);
        assert_eq!(e.env.ticks, 60, "{enhanced} {tools}");
        e.command(cmd::PAUSE);
        pass(&mut e, 100);
        assert_eq!(e.env.ticks, 66);
        let c = centre(&e, sw);
        hover(&mut e, c);
        assert_eq!(e.debug.tip(), None);
        assert_eq!(e.debug.status(), None);
    }
    // The switch alone (the enhancements off) leaves the picture as it was.
    let picture = |tools: bool| {
        let mut e = editor();
        e.set_enhanced(false);
        e.set_debug_tools(tools);
        let (sw, _) = switch_and_bulb(&mut e);
        e.run();
        let c = centre(&e, sw);
        hover(&mut e, c);
        e.surface.pixels.clone()
    };
    assert_eq!(picture(false), picture(true));
}

#[test]
fn the_menu_comes_and_goes_with_the_switches() {
    let mut e = editor();
    e.set_enhanced(true);
    e.update(16, (12, 0, 0));
    assert!(!has_debug_menu(&e));
    e.set_debug_tools(true);
    assert!(has_debug_menu(&e));
    // Before Help, the last.
    let n = e.menu.items.len();
    assert!(matches!(&e.menu.items[n - 2], MenuItem::Popup { text, .. } if text == "&Debug"));
    e.set_enhanced(false);
    e.update(16, (12, 0, 0));
    assert!(!has_debug_menu(&e));
    e.set_enhanced(true);
    e.update(16, (12, 0, 0));
    assert!(has_debug_menu(&e));
    // Editing: Pause and Stop Scripts greyed, the speed checked.
    assert!(!e.menu.enabled(cmd::PAUSE));
    assert!(!e.menu.enabled(cmd::STOP_SCRIPTS));
    assert!(e.menu.enabled(cmd::STEP_TICK));
    assert!(e.menu.checked.contains(&cmd::SPEED_1));
    // Running: not greyed as the other menus' are.
    e.run();
    for id in [cmd::PAUSE, cmd::STEP_TICK, cmd::STEP_SECOND, cmd::SPEED_4] {
        assert!(e.menu.enabled(id), "{id:x}");
    }
    e.command(cmd::PAUSE);
    e.command(cmd::SPEED_10);
    assert!(e.menu.checked.contains(&cmd::PAUSE));
    assert!(e.menu.checked.contains(&cmd::SPEED_10));
    assert!(!e.menu.checked.contains(&cmd::SPEED_1));
    // Stop Scripts, for the program running them.
    e.debug.scripts_running = true;
    e.command(cmd::STOP_SCRIPTS);
    assert!(e.debug.take_stop_scripts());
    assert!(!e.debug.take_stop_scripts());
}

#[test]
fn the_debug_menu_bar_letters_are_free_in_each_language() {
    for lang in Lang::ALL {
        crate::lang::set(lang);
        let mut e = Editor::new(packs(lang), Document::new(), 640, 433);
        e.set_enhanced(true);
        e.set_debug_tools(true);
        let letters: Vec<char> = e
            .menu
            .items
            .iter()
            .filter_map(|p| match p {
                MenuItem::Popup { text, .. } => ui::mnemonic(text),
                _ => None,
            })
            .collect();
        let mut unique = letters.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), letters.len(), "{lang:?}: {letters:?}");
        // And the Debug menu's own.
        let Some(MenuItem::Popup { items, .. }) = e.menu.items.iter().find(|p| has_items(p)) else {
            panic!("no Debug menu");
        };
        let mut own: Vec<char> = items
            .iter()
            .filter_map(|i| match i {
                MenuItem::Command { text, .. } => ui::mnemonic(text),
                _ => None,
            })
            .collect();
        let n = own.len();
        own.sort_unstable();
        own.dedup();
        assert_eq!(own.len(), n, "{lang:?}");
    }
    crate::lang::set(Lang::En);
}

fn has_items(p: &MenuItem) -> bool {
    matches!(p, MenuItem::Popup { items, .. }
        if items.iter().any(|i| matches!(i, MenuItem::Command { id, .. } if *id == cmd::PAUSE)))
}

#[test]
fn pause_holds_the_run() {
    let mut e = debugging();
    switch_and_bulb(&mut e);
    e.run();
    pass(&mut e, 500);
    assert_eq!(e.env.ticks, 30);
    e.key_down(VK_F6);
    e.key_up(VK_F6);
    assert!(e.debug.paused());
    pass(&mut e, 2000);
    assert_eq!(e.env.ticks, 30, "no ticks while paused");
    assert!(e.env.keys.is_empty(), "F6 isn't the parts'");
    e.command(cmd::PAUSE);
    pass(&mut e, 500);
    assert_eq!(e.env.ticks, 60);
    // Stopping lets go; the next run isn't held.
    e.command(cmd::PAUSE);
    e.stop();
    e.run();
    assert!(!e.debug.paused());
}

#[test]
fn a_step_is_one_tick() {
    let mut e = debugging();
    switch_and_bulb(&mut e);
    // From editing: GO, held, one tick.
    e.key_down(VK_F7);
    e.key_up(VK_F7);
    assert!(e.running);
    assert!(e.debug.paused());
    assert_eq!(e.env.ticks, 1);
    pass(&mut e, 1000);
    assert_eq!(e.env.ticks, 1);
    e.command(cmd::STEP_TICK);
    assert_eq!(e.env.ticks, 2);
    e.command(cmd::STEP_SECOND);
    assert_eq!(e.env.ticks, 62);
    // From a running widget: held, then the tick.
    e.command(cmd::PAUSE);
    assert!(!e.debug.paused());
    e.command(cmd::STEP_TICK);
    assert!(e.debug.paused());
    assert_eq!(e.env.ticks, 63);
}

#[test]
fn slow_motion_slows_the_ticks() {
    // A tick a second at 1/60, ticks of real time 16 ms frames apart.
    for (speed, id, ms, ticks) in [
        (Speed::Normal, cmd::SPEED_1, 1000, 60),
        (Speed::Half, cmd::SPEED_2, 1000, 30),
        (Speed::Quarter, cmd::SPEED_4, 1000, 15),
        (Speed::Tenth, cmd::SPEED_10, 1000, 6),
        (Speed::Sixtieth, cmd::SPEED_60, 6000, 6),
    ] {
        let mut e = debugging();
        switch_and_bulb(&mut e);
        e.command(id);
        assert_eq!(e.debug.speed(), speed);
        e.run();
        pass(&mut e, ms);
        assert_eq!(e.env.ticks, ticks, "{speed:?}");
        // The speed stays for the next run.
        e.stop();
        e.run();
        pass(&mut e, ms);
        assert_eq!(e.env.ticks, ticks, "{speed:?}");
    }
    // Off, the run goes at its speed again, not held.
    let mut e = debugging();
    e.command(cmd::SPEED_4);
    e.run();
    e.command(cmd::PAUSE);
    e.set_debug_tools(false);
    pass(&mut e, 1000);
    assert_eq!(e.env.ticks, 60);
}

#[test]
fn values_read_as_the_note_shows_them() {
    assert_eq!(show(&Val::None), "—");
    assert_eq!(show(&Val::Bool(true)), "on");
    assert_eq!(show(&Val::Bool(false)), "off");
    assert_eq!(show(&Val::Number(12.5)), "12.5");
    assert_eq!(show(&Val::Number(3.0)), "3");
    assert_eq!(show(&Val::Number(-0.25)), "-0.25");
    assert_eq!(show(&Val::Number(1e9)), "1.000e+09");
    assert_eq!(show(&Val::Picture(1204)), "picture 1204");
    assert_eq!(
        show(&Val::Sound {
            rate: 11025,
            len: 14333
        }),
        "sound 1.3 s"
    );
    assert_eq!(show(&Val::NoObject), "no object");
}

#[test]
fn the_note_shows_wires_knots_and_parts() {
    let mut e = debugging();
    let (sw, bulb) = switch_and_bulb(&mut e);
    let (s, b) = (index(&e, sw), index(&e, bulb));
    let out = (0..e.doc.gadgets[s].core.knots.len())
        .find(|&k| !e.doc.gadgets[s].core.knots[k].is_input())
        .unwrap();
    let inp = (0..e.doc.gadgets[b].core.knots.len())
        .find(|&k| e.doc.gadgets[b].core.knots[k].is_input())
        .unwrap();
    let w = Format {
        sw: format!("Light Switch {sw}"),
        bulb: format!("Light Bulb {bulb}"),
    };
    // Editing: nothing on the wire yet.
    assert_eq!(
        e.tip_lines(Target::Wire(0)).0,
        [format!("{}.out -> {}.in", w.sw, w.bulb), "off".to_owned()]
    );
    e.run();
    e.press(sw, None).unwrap();
    assert_eq!(
        e.tip_lines(Target::Wire(0)).0,
        [
            format!("{}.out -> {}.in", w.sw, w.bulb),
            "on  (changed, unread)".to_owned()
        ]
    );
    e.step(2).unwrap();
    assert_eq!(e.tip_lines(Target::Knot(b, inp)).0, [format!("{}.in: on", w.bulb)]);
    assert_eq!(e.tip_lines(Target::Knot(s, out)).0, [format!("{}.out: on", w.sw)]);
    // A part: its outputs; one without, its inputs.
    assert_eq!(e.tip_lines(Target::Part(s)).0, [w.sw.clone(), "out: on".to_owned()]);
    e.set_name(bulb, Some("Lamp")).unwrap();
    assert_eq!(
        e.tip_lines(Target::Part(b)).0,
        [format!("\"Lamp\" ({})", w.bulb), "in: on".to_owned()]
    );

    // Under the pointer, once it's still.
    let c = centre(&e, sw);
    e.mouse_move(c.0, c.1 + BAR_H);
    pass(&mut e, 200);
    e.draw();
    assert_eq!(e.debug.tip(), None, "not yet");
    pass(&mut e, HOVER_MS);
    e.draw();
    let tip = e.debug.tip().unwrap();
    assert_eq!(tip.lines, [w.sw.clone(), "out: on".to_owned()]);
    assert_eq!(tip.at, c);
    // The wire, between the two.
    let (a, z) = (
        e.doc.gadgets[s].core.knot_at(out).unwrap(),
        e.doc.gadgets[b].core.knot_at(inp).unwrap(),
    );
    hover(&mut e, (i32::midpoint(a.0, z.0), i32::midpoint(a.1, z.1)));
    assert_eq!(e.debug.tip().unwrap().lines[0], format!("{}.out -> \"Lamp\".in", w.sw));
    hover(&mut e, z);
    assert_eq!(e.debug.tip().unwrap().lines, ["\"Lamp\".in: on"]);
    // Off the parts, nothing; a move hides it.
    hover(&mut e, (500, 300));
    assert_eq!(e.debug.tip(), None);
    hover(&mut e, c);
    assert!(e.debug.tip().is_some());
    e.mouse_move(c.0 + 1, c.1 + BAR_H);
    e.draw();
    assert_eq!(e.debug.tip(), None);
}

struct Format {
    sw: String,
    bulb: String,
}

#[test]
fn the_note_stays_in_the_worksheet_and_shows_pictures() {
    let mut e = debugging();
    let font = ui::font(&e.assets).unwrap().clone();
    let view = e.view();
    let tip = |at: (i32, i32)| Tip {
        lines: vec!["\"Lamp\" (Light Bulb #2)".to_owned(), "in: on".to_owned()],
        picture: None,
        at,
    };
    let inside = |r: IRect| r.x0 >= view.x0 && r.y0 >= view.y0 && r.x1 <= view.x1 && r.y1 <= view.y1;
    let r = tip_rect(&tip((100, 100)), &font, false, view);
    assert!(r.x0 > 100 && r.y0 > 100, "below and right: {r:?}");
    for at in [(view.x1 - 1, view.y1 - 1), (view.x1 - 5, 3), (2, view.y1 - 2), (0, 0)] {
        let r = tip_rect(&tip(at), &font, true, view);
        assert!(inside(r), "{at:?}: {r:?}");
    }
    // Below the pointer at the bottom, it goes above it.
    let r = tip_rect(&tip((300, view.y1 - 3)), &font, false, view);
    assert!(r.y1 < view.y1 - 3);
    // A picture: its thumbnail beside the lines.
    let sheet = crate::gadgets::class(Tag(*b"bulb")).unwrap().sheet;
    let mut t = tip((100, 100));
    t.lines = vec![format!("picture {sheet}")];
    let plain = tip_rect(&t, &font, false, view);
    let with = tip_rect(&t, &font, true, view);
    assert!(with.width() >= plain.width() + 32 && with.height() >= 32);
    e.debug.hover.tip = Some(Tip {
        picture: Some(sheet),
        ..t
    });
    e.surface.fill(e.surface.bounds(), ui::BLACK);
    e.debug.draw(
        &mut e.surface,
        &e.assets,
        &e.pictures,
        IRect::new(0, 0, view.x1, view.y1),
    );
    let thumb = IRect::sized(with.x1 - 5 - 32, with.y0 + 5, 32, 32);
    let px: Vec<u8> = (thumb.y0..thumb.y1)
        .flat_map(|y| (thumb.x0..thumb.x1).map(move |x| (x, y)))
        .filter_map(|(x, y)| e.surface.get(x, y))
        .collect();
    assert!(
        px.iter().any(|&c| c != ui::WHITE && c != ui::FACE),
        "the picture is drawn"
    );
}

#[test]
fn the_status_counts_ticks() {
    let mut e = debugging();
    switch_and_bulb(&mut e);
    e.draw();
    assert_eq!(e.debug.status(), None, "editing");
    e.run();
    pass(&mut e, 100);
    e.draw();
    assert_eq!(e.debug.status(), Some("tick 6"));
    e.command(cmd::PAUSE);
    e.command(cmd::SPEED_4);
    e.draw();
    assert_eq!(e.debug.status(), Some("tick 6 · paused · ¼×"));
    crate::lang::set(Lang::Sv);
    e.draw();
    assert_eq!(e.debug.status(), Some("tick 6 · pausad · ¼×"));
    crate::lang::set(Lang::En);
    e.command(cmd::SPEED_10);
    e.command(cmd::PAUSE);
    e.draw();
    assert_eq!(e.debug.status(), Some("tick 6 · 1/10×"));
}

#[test]
fn the_settings_window_has_the_switch_with_the_enhancements() {
    let mut e = debugging();
    e.set_debug_tools(false);
    e.command(crate::editor::menu::cmd::SETTINGS);
    let b: &SettingsBox = e.settings.as_ref().unwrap();
    assert!(!b.debug_tools);
    let h = b.window().height();
    e.key_down(u16::from(b'D'));
    assert!(e.debug_tools());
    assert_eq!(e.take_debug_tools(), Some(true));
    assert_eq!(e.take_debug_tools(), None);
    // The enhancements off: the switch goes, the window shrinks.
    e.key_down(u16::from(b'E'));
    let b = e.settings.as_ref().unwrap();
    assert!(b.window().height() < h);
    assert!(!e.debugging());
    let mut b = b.clone();
    assert_eq!(b.key(u16::from(b'D')), Outcome::None);
}

#[test]
fn a_script_changing_the_run_keeps_the_puzzle_unsolved() {
    let mut e = debugging();
    if !e.load_puzzle(1) {
        return;
    }
    let part = PartId(e.doc.gadgets[0].core.id);
    let win = |e: &mut Editor| {
        e.env.end_run = Some(RunEnd { solved: true, sound: 0 });
        e.tick_once();
    };
    e.run();
    e.press(part, None).unwrap();
    assert!(e.debug.assisted());
    win(&mut e);
    assert!(!e.running);
    let key = crate::puzzle::done_key(&e.assets);
    assert_eq!(e.profile.get(key), None, "a script helped");
    // The next run is the player's again.
    e.run();
    assert!(!e.debug.assisted());
    win(&mut e);
    assert!(e.profile.get(key).is_some());
}

#[test]
fn the_wire_under_the_pointer_is_wide_and_orange_while_it_runs() {
    let mut e = debugging();
    let (sw, _) = switch_and_bulb(&mut e);
    let pts = crate::editor::worksheet::wire_points(&e.doc, 0, None);
    let (a, b) = (pts[0], pts[1]);
    let mid = (i32::midpoint(a.0, b.0), i32::midpoint(a.1, b.1));
    let orange = crate::gadget::nearest(&e.assets.palette(), super::HOT_WIRE);
    let at = |e: &Editor, p: (i32, i32)| e.surface.get(p.0, p.1 + BAR_H);
    // Editing: the selection's green, not orange.
    hover(&mut e, mid);
    assert_ne!(at(&e, mid), Some(orange));
    e.run();
    e.press(sw, None).unwrap();
    hover(&mut e, mid);
    assert_eq!(at(&e, mid), Some(orange));
    assert_eq!(e.hot_wire(), Some(0));
    // Off it, none.
    hover(&mut e, (mid.0, mid.1 + 30));
    assert_eq!(e.hot_wire(), None);
    assert_ne!(at(&e, mid), Some(orange));
}
