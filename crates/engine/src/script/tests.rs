//! The script module's pieces without an editor: selectors, values,
//! errors as shown. (`editor::debug`'s tests drive a real editor.)

use super::runtime::{ErrKind, ScriptError};
use super::*;

fn info(id: u32, kind: &str, tag: &str, name: Option<&str>, label: Option<&str>, at: (i32, i32)) -> PartInfo {
    PartInfo {
        id: PartId(id),
        tag: tag.into(),
        kind: kind.into(),
        name: name.map(Into::into),
        label: label.map(Into::into),
        rect: (at.0, at.1, at.0 + 20, at.1 + 20),
        locked: false,
        broken: false,
        knots: vec![KnotInfo {
            index: 0,
            input: true,
            at,
            wire: None,
            value: Val::None,
        }],
    }
}

fn sample() -> Vec<PartInfo> {
    vec![
        info(4, "LightBulb", "bulb", None, Some("Lamp"), (120, 80)),
        info(7, "LightBulb", "bulb", Some("Big lamp"), None, (300, 80)),
        info(9, "LightBulb", "bulb", None, None, (300, 200)),
        info(3, "Switch", "swit", None, Some("Lamp"), (10, 300)),
    ]
}

fn find(parts: &[PartInfo], s: &str) -> Result<u32, String> {
    let sel = select::parse(s)?;
    select::resolve(parts, &sel.part).map(|p| p.id.0).map_err(|e| e.message)
}

#[test]
fn selectors_read_and_show() {
    for s in [
        "#7",
        "Lamp",
        "\"Big lamp\"",
        "LightBulb[2]",
        "@120,80",
        "Gate.in2",
        "Gate.out",
        "#3.in",
        "\"Big lamp\"[2].3",
    ] {
        assert_eq!(select::show(&select::parse(s).unwrap()), s);
    }
    assert_eq!(select::show(&select::parse("Gate.in1").unwrap()), "Gate.in");
    assert_eq!(
        select::parse("Gate.in0").unwrap_err(),
        "Gate.in0: in0: inputs and outputs count from 1"
    );
    assert_eq!(select::parse("Gate[0]").unwrap_err(), "Gate[0]: [n] counts from 1");
    assert_eq!(
        select::parse("#x").unwrap_err(),
        "#x: an id is # and a number, such as #7"
    );
    assert!(select::parse("\"open").is_err());
    assert!(select::parse("#3[2]").is_err());
    assert!(select::parse(".in").is_err());
}

#[test]
fn names_come_before_labels_before_kinds() {
    let parts = sample();
    assert_eq!(find(&parts, "big LAMP"), Ok(7));
    assert_eq!(find(&parts, "#9"), Ok(9));
    assert_eq!(find(&parts, "LightBulb[3]"), Ok(9));
    assert_eq!(find(&parts, "bulb[2]"), Ok(7));
    assert_eq!(find(&parts, "Switch"), Ok(3));
    assert_eq!(find(&parts, "Lamp[2]"), Ok(3));
    assert_eq!(find(&parts, "@305,85"), Ok(7));
    assert_eq!(
        find(&parts, "Lamp"),
        Err("2 parts labelled Lamp: Light Bulb #4 at (120,80), Switch #3 at (10,300). Say which: \"Lamp[2]\", \"#3\", or name one".into())
    );
    assert_eq!(
        find(&parts, "LightBulb"),
        Err("3 Light Bulbs: #4 at (120,80), #7 at (300,80), #9 at (300,200). Say which: \"LightBulb[2]\", \"#7\", or name one".into())
    );
}

#[test]
fn did_you_mean() {
    let parts = sample();
    assert_eq!(
        find(&parts, "Lamb"),
        Err("no part called \"Lamb\". Did you mean \"Lamp\" (Light Bulb #4)?".into())
    );
    assert_eq!(
        find(&parts, "Swich"),
        Err("no part called \"Swich\". Did you mean \"Switch\" (Switch #3)?".into())
    );
    assert_eq!(
        find(&parts, "Big lamb"),
        Err("no part called \"Big lamb\". Did you mean \"Big lamp\" (Light Bulb #7)?".into())
    );
    assert_eq!(find(&parts, "Zebra"), Err("no part called \"Zebra\"".into()));
    assert_eq!(
        find(&parts, "AndGate"),
        Err("there's no And Gate in this widget".into())
    );
    assert_eq!(select::distance("kitten", "sitting"), 3);
    assert_eq!(select::distance("", "abc"), 3);
}

#[test]
fn words_and_plurals() {
    assert_eq!(select::words("LightBulb"), "Light Bulb");
    assert_eq!(select::words("RGBDisplay"), "RGB Display");
    assert_eq!(select::words("Display10"), "Display 10");
    assert_eq!(select::words("OneNumberCruncher"), "One Number Cruncher");
    assert_eq!(select::plural("Switch"), "Switches");
    assert_eq!(select::plural("Light Bulb"), "Light Bulbs");
}

#[test]
fn values_read_show_and_compare() {
    assert_eq!(value::parse("on"), Ok(Val::Bool(true)));
    assert_eq!(value::parse("OFF"), Ok(Val::Bool(false)));
    assert_eq!(value::parse("-2.5"), Ok(Val::Number(-2.5)));
    assert_eq!(value::parse("picture 1204"), Ok(Val::Picture(1204)));
    assert_eq!(value::parse("nothing"), Ok(Val::None));
    assert!(value::parse("inf").is_err());
    assert_eq!(
        value::parse("maybe").unwrap_err(),
        "maybe isn't a value: say on, off, a number, picture N or nothing"
    );
    for (v, s) in [
        (Val::Bool(true), "on"),
        (Val::Number(12.0), "12"),
        (Val::Number(12.5), "12.5"),
        (Val::Number(-0.0), "0"),
        (Val::Picture(1204), "picture 1204"),
        (
            Val::Sound {
                rate: 11025,
                len: 14333,
            },
            "sound 1.3 s @ 11025 Hz",
        ),
        (Val::None, "nothing"),
        (Val::NoObject, "no object"),
    ] {
        assert_eq!(v.to_string(), s);
    }
    let n = |x| Val::Number(x);
    assert_eq!(value::compare(&n(0.1 + 0.2), CmpOp::Eq, &n(0.3)), Ok(true));
    assert_eq!(value::compare(&n(3.0), CmpOp::Ge, &n(3.0)), Ok(true));
    assert_eq!(value::compare(&n(3.0), CmpOp::Gt, &n(3.0)), Ok(false));
    assert_eq!(value::compare(&Val::Bool(true), CmpOp::Ne, &Val::Bool(false)), Ok(true));
    assert_eq!(value::compare(&Val::None, CmpOp::Eq, &Val::Bool(true)), Ok(false));
    assert_eq!(
        value::compare(&n(3.0), CmpOp::Eq, &Val::Bool(true)),
        Err("is 3 (a number), which can't be compared with on (on/off)".into())
    );
    assert_eq!(
        value::compare(&Val::Bool(true), CmpOp::Lt, &n(1.0)),
        Err("is on (on/off); < needs numbers".into())
    );
    assert_eq!(value::ticks(600), "600 ticks (10 s)");
    assert_eq!(value::ticks(1), "1 tick (0.02 s)");
    assert_eq!(value::quote("Lamp"), "Lamp");
    assert_eq!(value::quote("Big lamp"), "\"Big lamp\"");
}

#[test]
fn errors_show_the_place() {
    let e = ScriptError::at(
        ErrKind::Host,
        "no part called \"Lamb\"",
        "check.txt",
        7,
        7,
        4,
        Some("press Lamb"),
    );
    assert_eq!(
        e.to_string(),
        "check.txt:7:7  no part called \"Lamb\"\n 7 | press Lamb\n   |       ^^^^"
    );
    let e = ScriptError::at(ErrKind::Host, "oops", "f", 0, 0, 0, None);
    assert_eq!(e.to_string(), "f  oops");
}

#[test]
fn shot_names_are_safe() {
    assert_eq!(file_name("after-a-second"), "after-a-second");
    assert_eq!(file_name("../x/y z"), "_x_y_z");
    assert_eq!(file_name(""), "shot");
}

#[test]
fn help_lists_every_command() {
    let h = api::help(None);
    for c in api::COMMANDS {
        assert!(h.contains(&api::usage(c)), "{}", c.name);
    }
    assert_eq!(
        api::help(Some("press")),
        format!("press PART [X Y] : {}", api::command("press").unwrap().doc)
    );
    assert_eq!(api::near("presss"), ["press"]);
}

#[test]
fn the_memory_sink_is_capped() {
    let mut s = MemorySink::default();
    for i in 0..3000 {
        s.log(Level::Info, &format!("line {i}"));
    }
    assert_eq!(s.lines.len(), MemorySink::MAX_LINES);
    assert_eq!(s.lines.back().unwrap().1, "line 2999");
    let shot = Shot {
        width: 1,
        height: 1,
        rgba: vec![0; 4],
    };
    for i in 0..20 {
        s.shot(&i.to_string(), &shot).unwrap();
    }
    assert_eq!(s.shots.len(), MemorySink::MAX_SHOTS);
}
