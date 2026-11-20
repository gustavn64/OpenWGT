//! The editor as a script host, on the port's own packs (no CD needed).

use super::super::{Editor, Pointer};
use crate::assets::{Assets, Packs};
use crate::doc::{self, Document};
use crate::gadget::Tag;
use crate::lang::Lang;
use crate::script::cmd::CommandRuntime;
use crate::script::runtime::run_headless;
use crate::script::{
    Caps, Cond, ErrorKind, Host, Level, PartId, PartSel, SettingKey, SettingVal, Shot, Sink, StepEnd, Val, select,
};

/// The port's own packs in English, as without a CD.
pub(super) fn assets() -> Assets {
    let graphics = crate::opengfx::builtin_for(Lang::En);
    let sounds = crate::opensfx::builtin();
    let data = crate::opendata::builtin_for(Lang::En);
    let packs = Packs {
        graphics: Some(graphics.as_ref()),
        sounds: Some(&sounds),
        data: Some(&data),
    };
    let mut assets = Assets::load_with(None, packs, None).unwrap();
    assets.lang = Lang::En;
    assets
}

pub(super) fn editor() -> Editor {
    Editor::new(assets(), Document::new(), 640, 433)
}

pub(super) fn add(e: &mut Editor, tag: [u8; 4], h: i32, v: i32) -> PartId {
    let id = e.doc.new_id();
    let g = doc::new_gadget(&e.assets, Tag(tag), id, h, v);
    e.doc.gadgets.push(g);
    PartId(id)
}

pub(super) fn index(e: &Editor, id: PartId) -> usize {
    e.doc.gadgets.iter().position(|g| g.core.id == id.0).unwrap()
}

/// Wires output `out` (counted among the outputs) of `from` to input `inp`
/// of `to`.
pub(super) fn connect(e: &mut Editor, from: PartId, out: usize, to: PartId, inp: usize) {
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
fn label_above(e: &mut Editor, id: PartId, text: &str) -> PartId {
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
    l
}

fn part(s: &str) -> PartSel {
    select::parse(s).unwrap().part
}

fn sel(s: &str) -> crate::script::Selector {
    select::parse(s).unwrap()
}

#[test]
fn parts_are_found_by_id_name_label_and_kind() {
    let mut e = editor();
    let s1 = add(&mut e, *b"swit", 100, 100);
    let s2 = add(&mut e, *b"swit", 300, 50);
    let bulb = add(&mut e, *b"bulb", 400, 250);
    label_above(&mut e, bulb, "Lamp");
    assert_eq!(e.part(bulb).unwrap().label.as_deref(), Some("Lamp"));
    assert_eq!(e.part(s1).unwrap().label, None);
    // Reading order: the higher switch first.
    let order: Vec<PartId> = e.parts().iter().filter(|p| p.tag == "swit").map(|p| p.id).collect();
    assert_eq!(order, [s2, s1]);
    assert_eq!(e.resolve(&part("Lamp")).unwrap(), bulb);
    assert_eq!(e.resolve(&part("lamp")).unwrap(), bulb);
    assert_eq!(e.resolve(&part("LightBulb")).unwrap(), bulb);
    assert_eq!(e.resolve(&part("\"Light Bulb\"")).unwrap(), bulb);
    assert_eq!(e.resolve(&part("bulb")).unwrap(), bulb);
    assert_eq!(e.resolve(&part(&format!("#{}", s1.0))).unwrap(), s1);
    assert_eq!(e.resolve(&part("Switch[1]")).unwrap(), s2);
    assert_eq!(e.resolve(&part("Switch[2]")).unwrap(), s1);
    let r = e.part(s1).unwrap().rect;
    assert_eq!(e.resolve(&PartSel::At(r.0 + 2, r.1 + 2)).unwrap(), s1);
    // Several.
    let err = e.resolve(&part("Switch")).unwrap_err();
    assert!(matches!(err.kind, ErrorKind::Ambiguous { .. }));
    let (a, b) = (e.part(s2).unwrap().rect, e.part(s1).unwrap().rect);
    assert_eq!(
        err.message,
        format!(
            "2 Switches: {s2} at ({},{}), {s1} at ({},{}). Say which: \"Switch[2]\", \"{s1}\", or name one",
            a.0, a.1, b.0, b.1
        )
    );
    let err = e.resolve(&part("Switch[3]")).unwrap_err();
    assert_eq!(err.message, "there are only 2 Switches, so no Switch[3]");
    // A name comes first, and ignores case.
    e.set_name(s1, Some("Power")).unwrap();
    assert_eq!(e.resolve(&part("power")).unwrap(), s1);
    assert_eq!(e.part(s1).unwrap().describe(), format!("\"Power\" (Switch {s1})"));
    assert_eq!(
        e.set_name(s2, Some("power")).unwrap_err().message,
        format!("power is the name of {s1} already")
    );
    assert!(e.set_name(s2, Some("#3")).is_err());
    assert!(e.set_name(s2, Some("say \"hi\"")).is_err());
    // Any other text, quoted where it has to be.
    e.set_name(s2, Some("a.b")).unwrap();
    assert_eq!(e.resolve(&part("\"a.b\"")).unwrap(), s2);
    e.set_name(s1, None).unwrap();
    assert!(e.resolve(&part("Power")).is_err());
    // None: the nearest, or a kind the widget hasn't.
    let err = e.resolve(&part("Lamb")).unwrap_err();
    assert_eq!(
        err.message,
        format!("no part called \"Lamb\". Did you mean \"Lamp\" (Light Bulb {bulb})?")
    );
    let ErrorKind::UnknownPart { suggestions, .. } = err.kind else {
        panic!()
    };
    assert_eq!(suggestions, ["Lamp"]);
    assert_eq!(
        e.resolve(&part("Counter")).unwrap_err().message,
        "there's no Counter in this widget"
    );
    assert_eq!(
        e.resolve(&part("#999")).unwrap_err().message,
        "no part #999 in this widget"
    );
}

#[test]
fn wire_and_knot_values() {
    let mut e = editor();
    let a = add(&mut e, *b"swit", 50, 50);
    let b = add(&mut e, *b"swit", 50, 150);
    let and = add(&mut e, *b"AndG", 200, 100);
    let bulb = add(&mut e, *b"bulb", 350, 100);
    connect(&mut e, a, 0, and, 0);
    connect(&mut e, b, 0, and, 1);
    connect(&mut e, and, 0, bulb, 0);
    e.set_name(a, Some("A")).unwrap();
    e.set_name(b, Some("B")).unwrap();
    // Editing: nothing on the wires.
    assert_eq!(e.value(&sel("LightBulb")).unwrap(), Val::Bool(false));
    e.run();
    e.set_value(a, &Val::Bool(true)).unwrap();
    assert_eq!(e.until(&Cond::Truthy(sel("AndGate.in")), 5), Ok(0));
    e.step(3).unwrap();
    assert_eq!(e.output(and, None).unwrap(), Val::Bool(false));
    assert_eq!(e.value(&sel("AndGate.in2")).unwrap(), Val::Bool(false));
    e.set_value(b, &Val::Bool(true)).unwrap();
    let n = e.until(&Cond::Truthy(sel("LightBulb")), 10).unwrap();
    assert!(n > 0, "{n}");
    assert_eq!(e.input(and, Some(1)).unwrap(), Val::Bool(true));
    assert_eq!(e.input(bulb, None).unwrap(), Val::Bool(true));
    assert_eq!(e.value(&sel("AndGate")).unwrap(), Val::Bool(true));
    let and_out = e.part(and).unwrap().outputs().next().unwrap().index;
    assert_eq!(e.source(bulb, 0).unwrap(), Some((and, and_out)));
    assert_eq!(e.source(a, 0).unwrap_err().kind, ErrorKind::BadKnot);
    let wires = e.wires();
    assert_eq!(wires.len(), 3);
    let to_bulb = wires.iter().find(|(w, _)| w.to.0 == bulb).unwrap();
    assert_eq!(to_bulb.1, Val::Bool(true));
    assert_eq!(e.wire_value(&to_bulb.0).unwrap(), Val::Bool(true));
    assert_eq!(
        e.value(&sel("LightBulb.out")).unwrap_err().message,
        format!("Light Bulb {bulb} has 1 input (in) and no outputs; output 1 doesn't exist")
    );
    assert_eq!(
        e.value(&sel("AndGate.in3")).unwrap_err().message,
        format!("And Gate {and} has 2 inputs (in to in2) and 1 output (out); input 3 doesn't exist")
    );
}

#[test]
fn set_value_works_each_control() {
    let mut e = editor();
    let sw = add(&mut e, *b"swit", 50, 50);
    let cnst = add(&mut e, *b"cnst", 50, 150);
    let slider = add(&mut e, *b"Sldr", 200, 50);
    let mini = add(&mut e, *b"MiCo", 200, 200);
    let pad = add(&mut e, *b"NmKP", 400, 50);
    let key = add(&mut e, *b"Key_", 400, 300);
    let bulb = add(&mut e, *b"bulb", 50, 300);
    connect(&mut e, sw, 0, bulb, 0);

    // Editing: what they start with, through their settings, undoable.
    e.set_value(sw, &Val::Bool(true)).unwrap();
    assert_eq!(e.setting(sw, SettingKey::Radio(0)).unwrap(), SettingVal::Int(1));
    assert!(e.dirty);
    e.undo();
    assert_eq!(e.setting(sw, SettingKey::Radio(0)).unwrap(), SettingVal::Int(0));
    e.set_value(sw, &Val::Bool(true)).unwrap();
    e.set_value(cnst, &Val::Number(7.0)).unwrap();
    assert_eq!(
        e.setting(cnst, SettingKey::Text(0)).unwrap(),
        SettingVal::Text("7".into())
    );
    assert_eq!(e.output(cnst, None).unwrap(), Val::Number(7.0));
    e.set_value(pad, &Val::Number(2.5)).unwrap();
    assert_eq!(
        e.setting(pad, SettingKey::Text(0)).unwrap(),
        SettingVal::Text("2.5".into())
    );
    assert_eq!(
        e.set_value(key, &Val::Bool(true)).unwrap_err().kind,
        ErrorKind::NotRunning
    );

    e.run();
    assert_eq!(e.output(sw, None).unwrap(), Val::Bool(true));
    assert_eq!(e.output(pad, None).unwrap(), Val::Number(2.5));
    // Running: their controls.
    e.set_value(sw, &Val::Bool(false)).unwrap();
    assert_eq!(e.output(sw, None).unwrap(), Val::Bool(false));
    let err = e.set_value(sw, &Val::Number(1.0)).unwrap_err();
    assert_eq!(err.kind, ErrorKind::WrongType);
    assert_eq!(err.message, format!("Switch {sw} takes on or off, not 1 (a number)"));

    assert_eq!(e.set_value(cnst, &Val::Number(42.0)), Ok(Val::Number(42.0)));
    assert_eq!(e.output(cnst, None).unwrap(), Val::Number(42.0));
    // Its settings stay as typed, as with its arrows.
    assert_eq!(
        e.setting(cnst, SettingKey::Text(0)).unwrap(),
        SettingVal::Text("7".into())
    );
    assert_eq!(
        e.set_value(cnst, &Val::Number(1000.0)).unwrap_err().message,
        format!("Constant {cnst} takes a whole number from 0 to 999, not 1000 (a number)")
    );
    assert_eq!(e.set_value(mini, &Val::Number(12.0)), Ok(Val::Number(12.0)));
    assert_eq!(e.output(mini, None).unwrap(), Val::Number(12.0));

    // The default Slider goes from 0 to 100 over 100 places.
    assert_eq!(e.set_value(slider, &Val::Number(25.0)), Ok(Val::Number(25.0)));
    assert_eq!(e.output(slider, None).unwrap(), Val::Number(25.0));
    assert_eq!(e.set_value(slider, &Val::Number(3.25)), Ok(Val::Number(3.0)));
    let err = e.set_value(slider, &Val::Number(150.0)).unwrap_err();
    assert_eq!(err.kind, ErrorKind::BadArg);
    assert_eq!(
        err.message,
        format!("Slider {slider} goes from 0 to 100; 150 is outside")
    );
    e.step(20).unwrap();
    assert_eq!(e.output(slider, None).unwrap(), Val::Number(3.0));

    assert_eq!(e.set_value(pad, &Val::Number(-12.5)), Ok(Val::Number(-12.5)));
    assert_eq!(e.output(pad, None).unwrap(), Val::Number(-12.5));
    assert_eq!(
        e.set_value(pad, &Val::Number(123_456_789_012.0)).unwrap_err().message,
        format!("Numeric Key Pad {pad} can't show 123456789012 in its 10 characters")
    );

    // The Keypress is on while its key (A) is held.
    e.set_value(key, &Val::Bool(true)).unwrap();
    e.step(1).unwrap();
    assert_eq!(e.output(key, None).unwrap(), Val::Bool(true));
    e.set_value(key, &Val::Bool(false)).unwrap();
    e.step(1).unwrap();
    assert_eq!(e.output(key, None).unwrap(), Val::Bool(false));

    // A part that only shows its input.
    let err = e.set_value(bulb, &Val::Bool(true)).unwrap_err();
    assert_eq!(err.kind, ErrorKind::NotSettable);
    assert_eq!(
        err.message,
        format!("Light Bulb {bulb} shows its input, it has no value of its own. Set what's wired into it: Switch {sw}")
    );
    // A puzzle's part.
    let i = index(&e, sw);
    e.doc.gadgets[i].core.locked = true;
    assert_eq!(e.set_value(sw, &Val::Bool(true)).unwrap_err().kind, ErrorKind::Locked);
    // A Constant with Lock Number.
    e.stop();
    e.set_setting(cnst, SettingKey::Check(0), &SettingVal::Bool(true))
        .unwrap();
    e.run();
    assert_eq!(
        e.set_value(cnst, &Val::Number(1.0)).unwrap_err().kind,
        ErrorKind::Locked
    );
}

#[test]
fn settings_change_as_the_dialog_would() {
    let mut e = editor();
    let label = add(&mut e, *b"TxLb", 50, 50);
    let w0 = e.part(label).unwrap().rect;
    e.set_setting(
        label,
        SettingKey::Text(0),
        &SettingVal::Text("A much longer label".into()),
    )
    .unwrap();
    assert_eq!(
        e.setting(label, SettingKey::Text(0)).unwrap(),
        SettingVal::Text("A much longer label".into())
    );
    // Laid out again for the text.
    assert!(e.part(label).unwrap().rect.2 > w0.2);
    assert_eq!(
        e.set_setting(label, SettingKey::Radio(0), &SettingVal::Text("x".into()))
            .unwrap_err()
            .kind,
        ErrorKind::WrongType
    );
    e.run();
    assert_eq!(
        e.set_setting(label, SettingKey::Text(0), &SettingVal::Text("x".into()))
            .unwrap_err()
            .kind,
        ErrorKind::NotRunning
    );
}

#[test]
fn until_waits_and_times_out_with_a_message() {
    let mut e = editor();
    let sw = add(&mut e, *b"swit", 50, 50);
    let bulb = add(&mut e, *b"bulb", 200, 50);
    let key = add(&mut e, *b"Key_", 50, 200);
    let bulb2 = add(&mut e, *b"bulb", 200, 200);
    connect(&mut e, sw, 0, bulb, 0);
    connect(&mut e, key, 0, bulb2, 0);
    e.set_name(bulb, Some("Lamp")).unwrap();
    e.set_name(bulb2, Some("Lamp2")).unwrap();
    let cmp = Cond::Cmp(sel("Lamp"), crate::script::CmpOp::Eq, Val::Bool(true));
    // Not running.
    let err = e.until(&cmp, 30).unwrap_err();
    assert_eq!(err.kind, ErrorKind::NotRunning);
    assert_eq!(
        err.message,
        "waiting for Lamp == on needs the widget running (run first); Lamp is off"
    );
    e.run();
    let err = e.until(&cmp, 30).unwrap_err();
    assert_eq!(err.kind, ErrorKind::Timeout);
    assert_eq!(
        err.message,
        "Lamp == on didn't happen within 30 ticks (0.5 s); Lamp is off"
    );
    assert_eq!(e.clock().ticks, 30);
    // At once: the switch gives its value as soon as it's pressed.
    e.press(sw, None).unwrap();
    assert_eq!(e.until(&cmp, 30), Ok(0));
    // Changed: from when the wait began. The Keypress sees its key at
    // its next tick.
    let changed = Cond::Changed(sel("Lamp2"));
    assert_eq!(e.until(&changed, 5).unwrap_err().kind, ErrorKind::Timeout);
    e.set_value(key, &Val::Bool(true)).unwrap();
    assert_eq!(e.until(&changed, 5), Ok(1));
    assert_eq!(
        e.until(
            &Cond::All(vec![cmp.clone(), Cond::Not(Box::new(Cond::Truthy(sel("Lamp2"))))]),
            2
        )
        .unwrap_err()
        .message,
        "Lamp == on and not Lamp2 didn't happen within 2 ticks (0.03 s); Lamp is on, Lamp2 is on"
    );
    // Steps are ticks.
    let t = e.clock().ticks;
    assert_eq!(e.step(1), Ok(StepEnd::Ran(1)));
    assert_eq!(e.clock().ticks, t + 1);
    e.stop();
    assert_eq!(e.step(1), Ok(StepEnd::NotRunning));
    assert_eq!(e.until(&Cond::Stopped, 0), Ok(0));
}

#[test]
fn press_and_poke() {
    let mut e = editor();
    let sw = add(&mut e, *b"swit", 50, 50);
    let cnst = add(&mut e, *b"cnst", 50, 150);
    let bulb = add(&mut e, *b"bulb", 200, 50);
    connect(&mut e, sw, 0, bulb, 0);
    assert_eq!(e.press(sw, None).unwrap_err().kind, ErrorKind::NotRunning);
    e.run();
    e.press(sw, None).unwrap();
    assert_eq!(e.output(sw, None).unwrap(), Val::Bool(true));
    // The switch's click sound.
    assert_eq!(e.sounds(), []);
    e.step(1).unwrap();
    assert_ne!(e.sounds(), []);
    let r = e.part(sw).unwrap().rect;
    assert_eq!(e.press(sw, Some((r.2 - r.0, 0))).unwrap_err().kind, ErrorKind::BadArg);
    e.poke(bulb, &Val::Bool(true)).unwrap();
    assert!(e.state(bulb).unwrap().state);
    e.poke(cnst, &Val::Number(5.0)).unwrap();
    assert_eq!(e.state(cnst).unwrap().number, 5.0);
    assert_eq!(e.poke(cnst, &Val::None).unwrap_err().kind, ErrorKind::WrongType);
}

#[test]
fn commands_run_against_the_editor() {
    let mut e = editor();
    let sw = add(&mut e, *b"swit", 50, 50);
    let bulb = add(&mut e, *b"bulb", 200, 50);
    connect(&mut e, sw, 0, bulb, 0);
    let src = "name LightBulb Lamp\nrun\nset Switch on\nuntil Lamp == on max 10\nexpect Lamp == off\nlog lamp {Lamp}\nexpect Lamb\nlog not reached\n";
    let mut rt = CommandRuntime::new();
    let report = run_headless(&mut rt, "t.txt", src, Caps::ALL, &mut e).unwrap();
    assert!(!report.passed());
    assert!(!report.completed);
    assert_eq!(report.failures, 1);
    assert_eq!(report.errors.len(), 2);
    assert_eq!(
        report.errors[0].to_string(),
        "t.txt:5:8  expected Lamp == off; Lamp is on\n 5 | expect Lamp == off\n   |        ^^^^^^^^^^^"
    );
    assert_eq!(
        report.errors[1].to_string(),
        format!(
            "t.txt:7:8  no part called \"Lamb\". Did you mean \"Lamp\" (Light Bulb {bulb})?\n 7 | expect Lamb\n   |        ^^^^"
        )
    );
    assert_eq!(e.debug.failures, 1);
    // A script from a widget can't open others.
    let report = run_headless(&mut rt, "w", "new", Caps::EMBEDDED, &mut e).unwrap();
    assert_eq!(report.errors[0].message, "a script from a widget can't do that");
    // A syntax error runs nothing.
    let err = run_headless(&mut rt, "s", "stop\nuntil", Caps::ALL, &mut e).unwrap_err();
    assert_eq!(
        (err.line, err.message.as_str()),
        (2, "until what? Such as: until Lamp == on")
    );
    assert!(e.running);
}

/// The output kept: the log and every shot.
#[derive(Default)]
struct Kept {
    log: Vec<String>,
    shots: Vec<(String, Shot)>,
}

impl Sink for std::rc::Rc<std::cell::RefCell<Kept>> {
    fn log(&mut self, _level: Level, msg: &str) {
        self.borrow_mut().log.push(msg.to_owned());
    }

    fn shot(&mut self, name: &str, shot: &Shot) -> Result<(), String> {
        self.borrow_mut().shots.push((name.to_owned(), shot.clone()));
        Ok(())
    }
}

/// The comparison scripts' runner as `examples/script.rs` had it before
/// it moved into the engine: the same editor calls, in the same order.
struct Old {
    editor: Editor,
    palette: [[u8; 3]; 256],
    mouse: (i32, i32),
    shots: Vec<(String, Shot)>,
    log: Vec<String>,
}

impl Old {
    fn wait(&mut self, ms: u64) {
        let mut left = ms;
        while left > 0 {
            let step = left.min(16);
            self.editor.update(step, (12, 0, 0));
            drop(self.editor.take_sounds());
            left -= step;
        }
        self.editor.draw();
    }

    fn to(&mut self, x: i32, y: i32) {
        let (x0, y0) = self.mouse;
        let n = ((x - x0).abs().max((y - y0).abs()) + 7) / 8;
        let n = n.max(1);
        for i in 1..=n {
            self.editor.mouse_move(x0 + (x - x0) * i / n, y0 + (y - y0) * i / n);
            self.wait(16);
        }
        self.mouse = (x, y);
    }

    fn shot(&mut self, name: &str) {
        self.editor.draw();
        let s = &self.editor.surface;
        let (w, h) = (s.width, s.height);
        let mut px: Vec<[u8; 4]> = s
            .pixels
            .iter()
            .map(|&i| {
                let c = self.palette[usize::from(i)];
                [c[0], c[1], c[2], 255]
            })
            .collect();
        if let Pointer::Game(i) = self.editor.pointer()
            && let Some(c) = self.editor.cursor_picture(i)
        {
            let (cw, ch) = (i32::from(c.width), i32::from(c.height));
            for cy in 0..ch {
                for cx in 0..cw {
                    let a = usize::try_from((cy * cw + cx) * 4).unwrap();
                    if c.rgba[a + 3] == 0 {
                        continue;
                    }
                    let x = self.mouse.0 - i32::from(c.hot.0) + cx;
                    let y = self.mouse.1 - i32::from(c.hot.1) + cy;
                    if (0..w).contains(&x) && (0..h).contains(&y) {
                        px[usize::try_from(y * w + x).unwrap()] = [c.rgba[a], c.rgba[a + 1], c.rgba[a + 2], 255];
                    }
                }
            }
        }
        self.shots.push((
            name.to_owned(),
            Shot {
                width: w.unsigned_abs(),
                height: h.unsigned_abs(),
                rgba: px.into_iter().flatten().collect(),
            },
        ));
    }

    fn dump(&mut self) {
        let bar = super::super::menu::BAR_H;
        for (i, g) in self.editor.doc.gadgets.iter().enumerate() {
            let r = g.core.rect();
            let knots: Vec<String> = (0..g.core.knots.len())
                .filter_map(|k| g.core.knot_at(k))
                .map(|(x, y)| format!("({x},{})", y + bar))
                .collect();
            self.log.push(format!(
                "{i} {} box ({},{})-({},{}) knots {}",
                String::from_utf8_lossy(&g.core.tag.0),
                r.x0,
                r.y0 + bar,
                r.x1,
                r.y1 + bar,
                knots.join(" ")
            ));
        }
    }

    fn play(&mut self, script: &str) {
        use super::super::Button;
        self.wait(500);
        for line in script.lines() {
            let line = line.split('#').next().unwrap_or_default().trim();
            let w: Vec<&str> = line.split_whitespace().collect();
            let Some(&cmd) = w.first() else { continue };
            let num = |i: usize| -> i32 { w[i].parse().unwrap() };
            let button = if w.contains(&"right") {
                Button::Right
            } else {
                Button::Left
            };
            match cmd {
                "move" => self.to(num(1), num(2)),
                "down" => {
                    self.to(num(1), num(2));
                    self.editor.mouse_down(num(1), num(2), button);
                    self.wait(16);
                }
                "up" => {
                    self.to(num(1), num(2));
                    self.editor.mouse_up(num(1), num(2), button);
                    self.wait(16);
                }
                "click" | "dclick" => {
                    self.to(num(1), num(2));
                    for _ in 0..if cmd == "dclick" { 2 } else { 1 } {
                        self.editor.mouse_down(num(1), num(2), button);
                        self.wait(80);
                        self.editor.mouse_up(num(1), num(2), button);
                        self.wait(120);
                    }
                }
                "drag" => {
                    self.to(num(1), num(2));
                    self.editor.mouse_down(num(1), num(2), button);
                    self.wait(200);
                    self.to(num(3), num(4));
                    self.wait(200);
                    self.editor.mouse_up(num(3), num(4), button);
                    self.wait(200);
                }
                "key" => {
                    let vk = u16::try_from(num(1)).unwrap();
                    self.editor.key_down(vk);
                    self.wait(80);
                    self.editor.key_up(vk);
                    self.wait(80);
                }
                "wait" => self.wait(u64::try_from(num(1)).unwrap()),
                "shot" => self.shot(w.get(1).copied().unwrap_or("shot")),
                "dump" => self.dump(),
                _ => panic!("{cmd}"),
            }
        }
    }
}

/// The comparison scripts give the same pictures, pixel for pixel, and
/// the same `dump`, through the engine's runner as through the old one.
#[test]
fn comparison_scripts_play_as_before() {
    for (name, script) in [
        ("first", include_str!("../../../../../tools/compare/scripts/first.txt")),
        ("lamp", include_str!("../../../../../tools/compare/scripts/lamp.txt")),
    ] {
        for enhanced in [false, true] {
            let mut old = Old {
                editor: editor(),
                palette: assets().palette(),
                mouse: (0, 0),
                shots: Vec::new(),
                log: Vec::new(),
            };
            old.editor.set_enhanced(enhanced);
            old.play(script);

            let mut e = editor();
            e.set_enhanced(enhanced);
            let kept = std::rc::Rc::new(std::cell::RefCell::new(Kept::default()));
            e.debug.sink = Box::new(kept.clone());
            e.wait(500).unwrap();
            let report = run_headless(&mut CommandRuntime::new(), name, script, Caps::ALL, &mut e).unwrap();
            assert!(report.passed(), "{report:?}");
            let kept = kept.borrow();
            assert_ne!(old.shots.len(), 0);
            assert_eq!(kept.shots.len(), old.shots.len(), "{name}");
            for ((n0, s0), (n1, s1)) in old.shots.iter().zip(&kept.shots) {
                assert_eq!(n0, n1);
                assert!(s0 == s1, "{name} (enhanced {enhanced}): shot {n0} differs");
            }
            let log: Vec<String> = kept.log.iter().flat_map(|l| l.lines().map(str::to_owned)).collect();
            assert_eq!(log, old.log, "{name}");
        }
    }
}
