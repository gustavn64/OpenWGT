//! The one native function the prelude holds, `host(op, ...args)`: every
//! operation of the API on the [`Host`].
//!
//! Each call reads its arguments first (which may run the script's own
//! code: getters, proxies) and only then takes the host, so the host is
//! never taken twice.

use std::cell::RefCell;
use std::rc::Rc;

use engine::script::{
    Button, Caps, ErrorKind, Host, HostError, KnotSel, Level, MouseEv, PartId, PartSel, Selector, SettingKey,
    SettingVal, StepEnd, Val, WireRef, file_name, select, value,
};
use rquickjs::prelude::Rest;
use rquickjs::{Array, Ctx, Function, Object, Persistent, Promise, Value};

use crate::errors::{self, Thrown};
use crate::{HOST, State, Wait, WaitKind, Watcher};

/// An argument as read from the script.
enum Arg {
    Undef,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Func(Persistent<Function<'static>>),
    List(Vec<Arg>),
    /// A picture value (`{kind: "picture", sheet}`).
    Val(Val),
    /// Anything else, in words.
    Other(String),
}

fn read<'js>(ctx: &Ctx<'js>, v: &Value<'js>, depth: u8) -> Arg {
    if v.is_undefined() {
        Arg::Undef
    } else if v.is_null() {
        Arg::Null
    } else if let Some(b) = v.as_bool() {
        Arg::Bool(b)
    } else if let Some(n) = v.as_number() {
        Arg::Num(n)
    } else if let Some(s) = v.as_string() {
        Arg::Str(s.to_string().unwrap_or_default())
    } else if let Some(f) = v.as_function() {
        Arg::Func(Persistent::save(ctx, f.clone()))
    } else if let Some(a) = v.as_array() {
        if depth > 0 {
            return Arg::Other("a list in a list".into());
        }
        Arg::List(
            a.iter::<Value>()
                .take(1000)
                .map(|x| x.map_or(Arg::Undef, |x| read(ctx, &x, depth + 1)))
                .collect(),
        )
    } else if let Some(o) = v.as_object() {
        let kind: Option<String> = o.get("kind").ok();
        let sheet: Option<f64> = o.get("sheet").ok();
        match (kind.as_deref(), sheet) {
            (Some("picture"), Some(n)) if n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) =>
            {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                Arg::Val(Val::Picture(n as u32))
            }
            (Some("sound"), _) => Arg::Other("a sound".into()),
            _ => Arg::Other("an object".into()),
        }
    } else {
        Arg::Other("a value of another kind".into())
    }
}

impl Arg {
    /// In words, for messages.
    fn said(&self) -> String {
        match self {
            Arg::Undef => "nothing (undefined)".into(),
            Arg::Null => "null".into(),
            Arg::Bool(b) => b.to_string(),
            Arg::Num(n) => value::number(*n),
            Arg::Str(s) => format!("the text \"{s}\""),
            Arg::Func(_) => "a function".into(),
            Arg::List(_) => "a list".into(),
            Arg::Val(v) => value::format(v),
            Arg::Other(s) => s.clone(),
        }
    }
}

/// Which part an error is about: its id, or what it was asked for as.
enum About {
    Nothing,
    Id(PartId),
    Query(String),
}

/// Why an operation failed.
enum Fault {
    Host(HostError, About),
    Js(rquickjs::Error),
}

impl From<rquickjs::Error> for Fault {
    fn from(e: rquickjs::Error) -> Fault {
        Fault::Js(e)
    }
}

type Done<'js> = Result<Value<'js>, Fault>;

fn bad(msg: impl Into<String>) -> Fault {
    Fault::Host(HostError::new(ErrorKind::BadArg, msg), About::Nothing)
}

fn on(id: PartId) -> impl FnOnce(HostError) -> Fault {
    move |e| Fault::Host(e, About::Id(id))
}

fn plain(e: HostError) -> Fault {
    Fault::Host(e, About::Nothing)
}

/// The native function.
pub fn host_fn<'js>(ctx: &Ctx<'js>, st: Rc<RefCell<State>>) -> rquickjs::Result<Function<'js>> {
    Function::new(
        ctx.clone(),
        move |ctx: Ctx<'js>, op: String, args: Rest<Value<'js>>| -> rquickjs::Result<Value<'js>> {
            let r = call(&ctx, &st, &op, &args.0);
            match r {
                Ok(v) => Ok(v),
                Err(Fault::Js(e)) => Err(e),
                Err(Fault::Host(e, about)) => Err(throw(&ctx, &e, about)),
            }
        },
    )
}

/// Throws `e` as the plain object the prelude turns into a `WidgetError`.
fn throw<'js>(ctx: &Ctx<'js>, e: &HostError, about: About) -> rquickjs::Error {
    let make = || -> rquickjs::Result<Value<'js>> {
        let o = Object::new(ctx.clone())?;
        o.set("widget", true)?;
        o.set("code", e.kind.code())?;
        o.set("message", e.message.as_str())?;
        let suggestions: Vec<String> = match &e.kind {
            ErrorKind::UnknownPart { suggestions, .. } => suggestions.clone(),
            ErrorKind::Ambiguous { candidates } => candidates.clone(),
            _ => Vec::new(),
        };
        o.set("suggestions", suggestions)?;
        match about {
            About::Nothing => o.set("part", Value::new_null(ctx.clone()))?,
            About::Id(id) => o.set("part", f64::from(id.0))?,
            About::Query(q) => o.set("part", q)?,
        }
        Ok(o.into_value())
    };
    match make() {
        Ok(v) => ctx.throw(v),
        Err(e) => e,
    }
}

/// What a script needs to be allowed `op`, and what it's told if not.
fn needs(op: &str) -> Option<(fn(Caps) -> bool, &'static str)> {
    Some(match op {
        "set" | "press" | "run" | "stop" | "pause" | "speed" | "step" | "key" | "type" | "mouse" | "seed" => {
            (|c: Caps| c.control, "a script that came with a widget can't do that")
        }
        "setSetting" | "setName" => (|c: Caps| c.edit, "this script can't change settings or names"),
        "open" | "loadPuzzle" | "newWidget" => (
            |c: Caps| c.document,
            "a script that came with a widget can't open other widgets",
        ),
        _ => return None,
    })
}

fn call<'js>(ctx: &Ctx<'js>, st: &Rc<RefCell<State>>, op: &str, args: &[Value<'js>]) -> Done<'js> {
    // Errors and checks are read whole first (their getters run here).
    match op {
        "error" => {
            let t = args.first().map(|v| Thrown::read(ctx, v)).unwrap_or_default();
            return Ok(report(ctx, st, Report::Error(t)));
        }
        "fail" => {
            let s = |i: usize| -> String {
                args.get(i)
                    .and_then(|v| v.as_string())
                    .and_then(|s| s.to_string().ok())
                    .unwrap_or_default()
            };
            return Ok(report(ctx, st, Report::Fail(s(0), s(1), s(2))));
        }
        _ => {}
    }
    let a: Vec<Arg> = args.iter().map(|v| read(ctx, v, 0)).collect();
    if let Some((allowed, why)) = needs(op)
        && !allowed(st.borrow().caps)
    {
        return Err(plain(HostError::new(ErrorKind::Caps, why)));
    }
    if !HOST.is_set() {
        return Err(plain(HostError::new(
            ErrorKind::Unsupported,
            "there's no game to work on here",
        )));
    }
    HOST.with(|host| run(ctx, st, host, op, &a))
}

enum Report {
    Error(Thrown),
    /// The message, the stack, the check (`assert`).
    Fail(String, String, String),
}

/// A failed check, or an error a test threw: logged where it happened,
/// and counted against the task.
fn report<'js>(ctx: &Ctx<'js>, st: &Rc<RefCell<State>>, r: Report) -> Value<'js> {
    let e = {
        let s = st.borrow();
        let (file, src) = s.source();
        match r {
            Report::Error(t) => {
                let kind = t.kind(s.blown());
                t.error(kind, &file, &src, s.memory)
            }
            Report::Fail(msg, stack, callee) => errors::at_call(
                engine::script::runtime::ErrKind::Assert,
                msg,
                &stack,
                &file,
                &src,
                None,
                Some(callee.as_str()).filter(|c| !c.is_empty()),
            ),
        }
    };
    if HOST.is_set() {
        HOST.with(|h| h.fail(&e.to_string()));
    }
    st.borrow_mut().failed(e);
    Value::new_undefined(ctx.clone())
}

fn num(a: &[Arg], i: usize) -> Option<f64> {
    match a.get(i) {
        Some(Arg::Num(n)) => Some(*n),
        _ => None,
    }
}

/// Argument `i`, a whole number from `lo` to `hi`, or why not.
fn whole(a: &[Arg], i: usize, lo: f64, hi: f64, what: &str) -> Result<i64, Fault> {
    match a.get(i) {
        Some(Arg::Num(n)) if n.fract() == 0.0 && *n >= lo && *n <= hi =>
        {
            #[allow(clippy::cast_possible_truncation)]
            Ok(*n as i64)
        }
        other => Err(bad(format!(
            "{what}, got {}",
            other.map_or_else(|| "nothing".into(), Arg::said)
        ))),
    }
}

fn text(a: &[Arg], i: usize) -> Option<&str> {
    match a.get(i) {
        Some(Arg::Str(s)) => Some(s),
        _ => None,
    }
}

/// The part (and maybe knot) argument `i` names, and the text it was.
fn selector(a: &[Arg], i: usize, f: &str) -> Result<(PartSel, Option<KnotSel>, String), Fault> {
    match a.get(i) {
        Some(Arg::Num(n)) if n.fract() == 0.0 && *n >= 0.0 && *n <= f64::from(u32::MAX) => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let id = *n as u32;
            Ok((PartSel::Id(id), None, format!("#{id}")))
        }
        Some(Arg::Str(s)) => {
            let sel = select::parse(s).map_err(|e| bad(format!("{f}: {e}")))?;
            Ok((sel.part, sel.knot, s.clone()))
        }
        other => Err(bad(format!(
            "{f}: a part is a Part, a name, a label, a kind or an id (\"#7\"), not {}",
            other.map_or_else(|| "nothing".into(), Arg::said)
        ))),
    }
}

/// The part argument `i` names (not a knot of it).
fn part(host: &dyn Host, a: &[Arg], i: usize, f: &str) -> Result<PartId, Fault> {
    let (ps, knot, q) = selector(a, i, f)?;
    if knot.is_some() {
        return Err(bad(format!(
            "{f}: {q} is a knot; give the part, and use .input(n) or .output(n) for its knots"
        )));
    }
    host.resolve(&ps).map_err(|e| Fault::Host(e, About::Query(q)))
}

/// A selector argument resolved to the part's id (so it stays on the
/// same part).
fn resolved(host: &dyn Host, a: &[Arg], i: usize, f: &str) -> Result<Selector, Fault> {
    let (ps, knot, q) = selector(a, i, f)?;
    let id = host.resolve(&ps).map_err(|e| Fault::Host(e, About::Query(q)))?;
    let sel = Selector {
        part: PartSel::Id(id.0),
        knot,
    };
    host.value(&sel).map_err(on(id))?;
    Ok(sel)
}

/// Argument `i` as a part's id (from a Part object).
fn id(a: &[Arg], i: usize) -> Result<PartId, Fault> {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    match num(a, i) {
        Some(n) if n.fract() == 0.0 && n >= 0.0 => Ok(PartId(n as u32)),
        _ => Err(bad("not a part")),
    }
}

pub fn val_to_js<'js>(ctx: &Ctx<'js>, v: &Val) -> rquickjs::Result<Value<'js>> {
    Ok(match v {
        Val::None | Val::NoObject => Value::new_null(ctx.clone()),
        Val::Bool(b) => Value::new_bool(ctx.clone(), *b),
        Val::Number(n) => Value::new_number(ctx.clone(), *n),
        Val::Picture(p) => {
            let o = Object::new(ctx.clone())?;
            o.set("kind", "picture")?;
            o.set("sheet", f64::from(*p))?;
            o.set("text", value::format(v))?;
            o.into_value()
        }
        Val::Sound { rate, len } => {
            let o = Object::new(ctx.clone())?;
            o.set("kind", "sound")?;
            o.set("rate", f64::from(*rate))?;
            o.set("length", f64::from(*len))?;
            o.set("text", value::format(v))?;
            o.into_value()
        }
    })
}

fn numbers<'js>(ctx: &Ctx<'js>, items: impl IntoIterator<Item = f64>) -> rquickjs::Result<Value<'js>> {
    let a = Array::new(ctx.clone())?;
    for (i, v) in items.into_iter().enumerate() {
        a.set(i, v)?;
    }
    Ok(a.into_value())
}

fn obj<'js>(ctx: &Ctx<'js>, fields: &[(&str, Value<'js>)]) -> rquickjs::Result<Value<'js>> {
    let o = Object::new(ctx.clone())?;
    for (k, v) in fields {
        o.set(*k, v.clone())?;
    }
    Ok(o.into_value())
}

fn n<'js>(ctx: &Ctx<'js>, v: f64) -> Value<'js> {
    Value::new_number(ctx.clone(), v)
}

fn b<'js>(ctx: &Ctx<'js>, v: bool) -> Value<'js> {
    Value::new_bool(ctx.clone(), v)
}

fn s<'js>(ctx: &Ctx<'js>, v: &str) -> rquickjs::Result<Value<'js>> {
    Ok(rquickjs::String::from_str(ctx.clone(), v)?.into_value())
}

fn opt_s<'js>(ctx: &Ctx<'js>, v: Option<&str>) -> rquickjs::Result<Value<'js>> {
    match v {
        Some(v) => s(ctx, v),
        None => Ok(Value::new_null(ctx.clone())),
    }
}

/// `check0`..`check31`, `radio0`..`radio3`, `number0`..`number3`,
/// `text0`..`text3`, `text`.
fn setting_key(k: &str) -> Option<SettingKey> {
    if k == "text" {
        return Some(SettingKey::LongText);
    }
    let digits = k.find(|c: char| c.is_ascii_digit())?;
    let n: u8 = k[digits..].parse().ok()?;
    match (&k[..digits], n) {
        ("check", 0..=31) => Some(SettingKey::Check(n)),
        ("radio", 0..=3) => Some(SettingKey::Radio(n)),
        ("number", 0..=3) => Some(SettingKey::Number(n)),
        ("text", 0..=3) => Some(SettingKey::Text(n)),
        _ => None,
    }
}

/// A key's Windows virtual-key code from its name or character.
fn vk(name: &str) -> Option<u16> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.clone().next()) {
        return vk_char(c);
    }
    let named: &[(&str, u16)] = &[
        ("Enter", 0x0D),
        ("Return", 0x0D),
        ("Space", 0x20),
        ("Tab", 0x09),
        ("Backspace", 0x08),
        ("Escape", 0x1B),
        ("Esc", 0x1B),
        ("Shift", 0x10),
        ("Ctrl", 0x11),
        ("Control", 0x11),
        ("Alt", 0x12),
        ("Left", 0x25),
        ("Up", 0x26),
        ("Right", 0x27),
        ("Down", 0x28),
        ("Delete", 0x2E),
        ("Home", 0x24),
        ("End", 0x23),
        ("PageUp", 0x21),
        ("PageDown", 0x22),
    ];
    if let Some(&(_, v)) = named.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
        return Some(v);
    }
    let f: u16 = name.strip_prefix(['F', 'f'])?.parse().ok()?;
    (1..=12).contains(&f).then_some(0x6F + f)
}

fn vk_char(c: char) -> Option<u16> {
    match c {
        'a'..='z' | 'A'..='Z' | '0'..='9' => u16::try_from(u32::from(c.to_ascii_uppercase())).ok(),
        ' ' => Some(0x20),
        '\n' => Some(0x0D),
        '.' => Some(0xBE),
        ',' => Some(0xBC),
        '-' => Some(0xBD),
        _ => None,
    }
}

/// One wire's ends as the prelude keeps them: `[from, knot, to, knot]`.
fn wire_ref(a: &[Arg]) -> Result<WireRef, Fault> {
    let Some(Arg::List(l)) = a.first() else {
        return Err(bad("not a wire"));
    };
    let k = |i: usize| -> Result<u32, Fault> {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        match l.get(i) {
            Some(Arg::Num(n)) if *n >= 0.0 => Ok(*n as u32),
            _ => Err(bad("not a wire")),
        }
    };
    let knot = |i| k(i).map(|v| u8::try_from(v).unwrap_or(u8::MAX));
    Ok(WireRef {
        from: (PartId(k(0)?), knot(1)?),
        to: (PartId(k(2)?), knot(3)?),
    })
}

fn unsupported_once(st: &Rc<RefCell<State>>, host: &mut dyn Host, what: &'static str, e: &HostError) {
    if st.borrow_mut().warn_once(what) {
        host.log(Level::Warn, &format!("{what}(): {} (it does nothing here)", e.message));
    }
}

#[allow(clippy::too_many_lines)]
fn run<'js>(ctx: &Ctx<'js>, st: &Rc<RefCell<State>>, host: &mut dyn Host, op: &str, a: &[Arg]) -> Done<'js> {
    let undef = || Value::new_undefined(ctx.clone());
    Ok(match op {
        // ---- parts ----
        "resolve" => n(ctx, f64::from(part(host, a, 0, "part(query)")?.0)),
        "parts" => {
            let all = host.parts();
            let ids: Vec<f64> = match text(a, 0) {
                None => all.iter().map(|p| f64::from(p.id.0)).collect(),
                Some(q) => {
                    let sel = select::parse(q).map_err(|e| bad(format!("parts(query): {e}")))?;
                    match sel.part {
                        PartSel::Text { text, nth: None } => {
                            let t = text.to_lowercase();
                            let squash = |s: &str| s.replace(' ', "").to_lowercase();
                            all.iter()
                                .filter(|p| {
                                    p.name.as_deref().is_some_and(|n| n.to_lowercase() == t)
                                        || p.label.as_deref().is_some_and(|l| l.to_lowercase() == t)
                                        || squash(&p.kind) == squash(&t)
                                        || p.tag.to_lowercase() == t
                                })
                                .map(|p| f64::from(p.id.0))
                                .collect()
                        }
                        other => vec![f64::from(
                            host.resolve(&other)
                                .map_err(|e| Fault::Host(e, About::Query(q.into())))?
                                .0,
                        )],
                    }
                }
            };
            numbers(ctx, ids)?
        }
        "info" => {
            let id = id(a, 0)?;
            let p = host.part(id).map_err(on(id))?;
            let (x0, y0, x1, y1) = p.rect;
            obj(
                ctx,
                &[
                    ("id", n(ctx, f64::from(p.id.0))),
                    ("kind", s(ctx, &p.kind)?),
                    ("tag", s(ctx, &p.tag)?),
                    ("name", opt_s(ctx, p.name.as_deref())?),
                    ("label", opt_s(ctx, p.label.as_deref())?),
                    ("x", n(ctx, f64::from(x0))),
                    ("y", n(ctx, f64::from(y0))),
                    ("width", n(ctx, f64::from(x1 - x0))),
                    ("height", n(ctx, f64::from(y1 - y0))),
                    ("locked", b(ctx, p.locked)),
                    ("broken", b(ctx, p.broken)),
                ],
            )?
        }
        "describe" => {
            let id = id(a, 0)?;
            s(ctx, &format!("Part {}", host.part(id).map_err(on(id))?.describe()))?
        }
        "on" | "value" => {
            let id = id(a, 0)?;
            let v = host
                .value(&Selector {
                    part: PartSel::Id(id.0),
                    knot: None,
                })
                .map_err(on(id))?;
            if op == "on" {
                b(ctx, v.truthy())
            } else {
                val_to_js(ctx, &v)?
            }
        }
        "output" | "input" => {
            let id = id(a, 0)?;
            let input = op == "input";
            let v = match a.get(1) {
                None | Some(Arg::Undef) => {
                    if input {
                        host.input(id, None)
                    } else {
                        host.output(id, None)
                    }
                }
                Some(Arg::Str(k)) => {
                    let sel = select::parse(&format!("#{}.{k}", id.0)).map_err(|e| bad(format!("{op}(knot): {e}")))?;
                    host.value(&sel)
                }
                Some(_) => {
                    let k = whole(
                        a,
                        1,
                        0.0,
                        255.0,
                        &format!("{op}(knot): knot is the {op}'s number (from 0) or its name (\"{op}2\")"),
                    )?;
                    let k = u8::try_from(k).unwrap_or(u8::MAX);
                    if input {
                        host.input(id, Some(k))
                    } else {
                        host.output(id, Some(k))
                    }
                }
            }
            .map_err(on(id))?;
            val_to_js(ctx, &v)?
        }
        "set" => {
            let id = id(a, 0)?;
            let v = match a.get(1) {
                Some(Arg::Bool(b)) => Val::Bool(*b),
                Some(Arg::Num(x)) if x.is_finite() => Val::Number(*x),
                Some(Arg::Val(v)) => v.clone(),
                other => {
                    let who = host.part(id).map_err(on(id))?.describe();
                    return Err(Fault::Host(
                        HostError::new(
                            ErrorKind::WrongType,
                            format!(
                                "{who}'s value is on/off (true, false), a number or a picture, not {}",
                                other.map_or_else(|| "nothing".into(), Arg::said)
                            ),
                        ),
                        About::Id(id),
                    ));
                }
            };
            let took = host.set_value(id, &v).map_err(on(id))?;
            val_to_js(ctx, &took)?
        }
        "source" => {
            let id = id(a, 0)?;
            let k = whole(a, 1, 0.0, 255.0, "source(knot): knot is the input's number, from 0")?;
            let p = host.part(id).map_err(on(id))?;
            let k = u8::try_from(k).unwrap_or(u8::MAX);
            let knot = select::nth_knot(&p, true, k).ok_or_else(|| {
                Fault::Host(
                    HostError::new(
                        ErrorKind::BadKnot,
                        format!("{} has no input {k} ({})", p.describe(), select::knots_said(&p)),
                    ),
                    About::Id(id),
                )
            })?;
            match host.source(id, knot).map_err(on(id))? {
                Some((from, _)) => n(ctx, f64::from(from.0)),
                None => Value::new_null(ctx.clone()),
            }
        }
        "press" => {
            let id = id(a, 0)?;
            let given = |i| !matches!(a.get(i), None | Some(Arg::Undef));
            let at = if given(1) || given(2) {
                let x = whole(a, 1, -10_000.0, 10_000.0, "press(x, y): x is a whole number")?;
                let y = whole(a, 2, -10_000.0, 10_000.0, "press(x, y): y is a whole number")?;
                Some((i32::try_from(x).unwrap_or(0), i32::try_from(y).unwrap_or(0)))
            } else {
                None
            };
            host.press(id, at).map_err(on(id))?;
            undef()
        }
        "setting" | "setSetting" => {
            let id = id(a, 0)?;
            let k = text(a, 1).unwrap_or_default();
            let key = setting_key(k).ok_or_else(|| {
                bad(format!(
                    "{op}(key): a key is \"check0\"..\"check31\", \"radio0\"..\"radio3\", \"number0\"..\"number3\", \"text0\"..\"text3\" or \"text\", not {}",
                    a.get(1).map_or_else(|| "nothing".into(), Arg::said)
                ))
            })?;
            if op == "setting" {
                match host.setting(id, key).map_err(on(id))? {
                    SettingVal::Bool(v) => b(ctx, v),
                    SettingVal::Int(v) => n(ctx, f64::from(v)),
                    SettingVal::Text(t) => s(ctx, &t)?,
                }
            } else {
                let v = match (key, a.get(2)) {
                    (SettingKey::Check(_), Some(Arg::Bool(v))) => SettingVal::Bool(*v),
                    (SettingKey::Radio(_) | SettingKey::Number(_), _) => {
                        let v = whole(
                            a,
                            2,
                            0.0,
                            f64::from(u32::MAX),
                            &format!("setSetting({k}, value): value is a whole number from 0"),
                        )?;
                        SettingVal::Int(u32::try_from(v).unwrap_or(0))
                    }
                    (SettingKey::Text(_) | SettingKey::LongText, Some(Arg::Str(t))) => SettingVal::Text(t.clone()),
                    (SettingKey::Check(_), other) => {
                        return Err(bad(format!(
                            "setSetting({k}, value): a check box is true or false, not {}",
                            other.map_or_else(|| "nothing".into(), Arg::said)
                        )));
                    }
                    (_, other) => {
                        return Err(bad(format!(
                            "setSetting({k}, value): a text is a string, not {}",
                            other.map_or_else(|| "nothing".into(), Arg::said)
                        )));
                    }
                };
                host.set_setting(id, key, &v).map_err(on(id))?;
                undef()
            }
        }
        "setName" => {
            let id = id(a, 0)?;
            let name = match a.get(1) {
                Some(Arg::Str(t)) => Some(t.as_str()),
                None | Some(Arg::Null | Arg::Undef) => None,
                Some(other) => return Err(bad(format!("a name is a string, not {}", other.said()))),
            };
            host.set_name(id, name).map_err(on(id))?;
            undef()
        }
        // ---- wires ----
        "wire" => {
            let end = |i: usize| -> Result<Option<(PartId, Option<u8>, String)>, Fault> {
                if matches!(a.get(i), None | Some(Arg::Undef)) {
                    return Ok(None);
                }
                let (ps, knot, q) = selector(a, i, "wire(from, to)")?;
                let id = host.resolve(&ps).map_err(|e| Fault::Host(e, About::Query(q.clone())))?;
                let knot = match knot {
                    Some(k) => {
                        let p = host.part(id).map_err(on(id))?;
                        Some(select::knot(&p, k).map_err(on(id))?.index)
                    }
                    None => None,
                };
                Ok(Some((id, knot, q)))
            };
            let from = end(0)?.ok_or_else(|| bad("wire(from, to): from is a part"))?;
            let to = end(1)?;
            let hits = |e: (PartId, u8), w: &(PartId, Option<u8>, String)| e.0 == w.0 && w.1.is_none_or(|k| k == e.1);
            let found = host.wires().into_iter().map(|(w, _)| w).find(|w| {
                (hits(w.from, &from) && to.as_ref().is_none_or(|t| hits(w.to, t)))
                    || (hits(w.to, &from) && to.as_ref().is_some_and(|t| hits(w.from, t)))
            });
            let Some(w) = found else {
                let msg = match &to {
                    Some(t) => format!("no wire from {} to {}", from.2, t.2),
                    None => format!("no wire from {}", from.2),
                };
                return Err(Fault::Host(HostError::new(ErrorKind::BadKnot, msg), About::Id(from.0)));
            };
            numbers(
                ctx,
                [
                    f64::from(w.from.0.0),
                    f64::from(w.from.1),
                    f64::from(w.to.0.0),
                    f64::from(w.to.1),
                ],
            )?
        }
        "wireValue" => val_to_js(ctx, &host.wire_value(&wire_ref(a)?).map_err(plain)?)?,
        "wireChanged" => b(ctx, host.wire_changed(&wire_ref(a)?).map_err(plain)?),
        "wireText" => {
            let w = wire_ref(a)?;
            let v = host.wire_value(&w).map_err(plain)?;
            let end = |(id, k): (PartId, u8)| {
                host.part(id)
                    .map_or_else(|_| format!("{id}.{k}"), |p| format!("{} knot {k}", p.describe()))
            };
            s(
                ctx,
                &format!("Wire {} \u{2192} {}: {}", end(w.from), end(w.to), value::format(&v)),
            )?
        }
        // ---- the run ----
        "run" => {
            host.run();
            undef()
        }
        "stop" => {
            host.stop();
            undef()
        }
        "pause" => {
            let on = matches!(a.first(), Some(Arg::Bool(true)));
            if let Err(e) = host.pause(on) {
                if e.kind != ErrorKind::Unsupported {
                    return Err(plain(e));
                }
                unsupported_once(st, host, if on { "pause" } else { "resume" }, &e);
            }
            undef()
        }
        "speed" => {
            let f = num(a, 0)
                .filter(|f| *f > 0.0 && *f <= 1.0)
                .ok_or_else(|| bad("speed(factor): factor is from 1 (full speed) down to 1/60"))?;
            match host.set_speed(f) {
                Ok(took) => n(ctx, took),
                Err(e) if e.kind == ErrorKind::Unsupported => {
                    unsupported_once(st, host, "speed", &e);
                    n(ctx, 1.0)
                }
                Err(e) => return Err(plain(e)),
            }
        }
        "step" => {
            let k = whole(a, 0, 0.0, 1e6, "step(ticks): ticks is a whole number of ticks")?;
            match host.step(u32::try_from(k).unwrap_or(0)).map_err(plain)? {
                StepEnd::Ran(t) => obj(
                    ctx,
                    &[
                        ("ticks", n(ctx, f64::from(t))),
                        ("stopped", b(ctx, false)),
                        ("solved", b(ctx, false)),
                    ],
                )?,
                StepEnd::Stopped { solved, at } => obj(
                    ctx,
                    &[
                        ("ticks", n(ctx, f64::from(at))),
                        ("stopped", b(ctx, true)),
                        ("solved", b(ctx, solved)),
                    ],
                )?,
                StepEnd::NotRunning => {
                    return Err(plain(HostError::new(
                        ErrorKind::NotRunning,
                        "step() needs the widget running (run() first)",
                    )));
                }
            }
        }
        "clock" => {
            let c = host.clock();
            obj(
                ctx,
                &[
                    ("ticks", n(ctx, f64::from(c.ticks))),
                    ("seconds", n(ctx, f64::from(c.ticks) / 60.0)),
                    ("running", b(ctx, c.running)),
                    ("paused", b(ctx, host.paused())),
                ],
            )?
        }
        "now" => {
            let beats = st.borrow().live(host.clock());
            #[allow(clippy::cast_precision_loss)]
            n(ctx, crate::EPOCH_MS + (beats as f64 * 1000.0 / 60.0).floor())
        }
        "beats" =>
        {
            #[allow(clippy::cast_precision_loss)]
            n(ctx, st.borrow().live(host.clock()) as f64)
        }
        "failures" => n(ctx, f64::from(st.borrow().failures())),
        // ---- waits ----
        "ticks" => {
            let k = whole(a, 0, 0.0, 1e9, "ticks(n): n is a whole number of ticks")?;
            let at = st.borrow().live(host.clock()) + u64::try_from(k).unwrap_or(0);
            wait(ctx, st, WaitKind::Ticks(at))?
        }
        "until" => {
            let Some(Arg::Func(f)) = a.first() else {
                return Err(bad("until(cond): cond is a function"));
            };
            let max = whole(a, 1, 0.0, 1e9, "until(cond, {max}): max is a whole number of ticks")?;
            let start = st.borrow().live(host.clock());
            let shown = text(a, 2).unwrap_or("cond").trim().to_owned();
            wait(
                ctx,
                st,
                WaitKind::Until {
                    cond: f.clone(),
                    start,
                    max: u64::try_from(max).unwrap_or(0),
                    text: shown,
                },
            )?
        }
        // ---- input ----
        "key" => {
            let code = match a.first() {
                Some(Arg::Str(k)) => vk(k),
                Some(Arg::Num(_)) => u16::try_from(whole(a, 0, 1.0, 254.0, "key(k)")?).ok(),
                _ => None,
            }
            .ok_or_else(|| {
                bad(format!(
                    "key(k): k is a key's name (\"A\", \"Enter\", \"Left\", \"F1\") or its code, not {}",
                    a.first().map_or_else(|| "nothing".into(), Arg::said)
                ))
            })?;
            let hold = whole(a, 1, 0.0, 6000.0, "key(k, {hold}): hold is a whole number of ticks")?;
            host.key(code, true);
            let at = st.borrow().live(host.clock()) + u64::try_from(hold).unwrap_or(0);
            st.borrow_mut().keys.push((at, code, false));
            undef()
        }
        "type" => {
            let t = text(a, 0).unwrap_or_default();
            let codes: Vec<u16> = t
                .chars()
                .map(|c| {
                    vk_char(c).ok_or_else(|| {
                        bad(format!(
                            "type(text): {c:?} can't be typed (letters, digits, spaces and . , - only)"
                        ))
                    })
                })
                .collect::<Result<_, _>>()?;
            let start = st.borrow().live(host.clock());
            let mut s = st.borrow_mut();
            for (i, c) in codes.into_iter().enumerate() {
                let at = start + 4 * u64::try_from(i).unwrap_or(0);
                s.keys.push((at, c, true));
                s.keys.push((at + 2, c, false));
            }
            undef()
        }
        "mouse" => {
            let (x, y) = st.borrow().mouse;
            let button = || match text(a, 1) {
                Some("right") => Ok(Button::Right),
                Some("left") | None => Ok(Button::Left),
                Some(o) => Err(bad(format!("mouse: a button is \"left\" or \"right\", not {o:?}"))),
            };
            let ev = match text(a, 0) {
                Some("move") => {
                    let x = whole(a, 1, -10_000.0, 10_000.0, "mouse.move(x, y): x is a whole number")?;
                    let y = whole(a, 2, -10_000.0, 10_000.0, "mouse.move(x, y): y is a whole number")?;
                    let p = (i32::try_from(x).unwrap_or(0), i32::try_from(y).unwrap_or(0));
                    st.borrow_mut().mouse = p;
                    MouseEv::Move(p.0, p.1)
                }
                Some("down") => MouseEv::Down(x, y, button()?),
                _ => MouseEv::Up(x, y, button()?),
            };
            host.mouse(ev);
            undef()
        }
        // ---- watching ----
        "onChange" => {
            let sel = resolved(host, a, 0, "onChange(what, f)")?;
            let Some(Arg::Func(f)) = a.get(1) else {
                return Err(bad("onChange(what, f): f is a function"));
            };
            let last = host.value(&sel).ok();
            let mut s = st.borrow_mut();
            s.next_watch += 1;
            let id = s.next_watch;
            let task = s.current;
            s.watchers.push(Watcher {
                id,
                task,
                sel,
                last,
                f: f.clone(),
            });
            n(ctx, f64::from(id))
        }
        "offChange" => {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let id = num(a, 0).unwrap_or(0.0) as u32;
            st.borrow_mut().watchers.retain(|w| w.id != id);
            undef()
        }
        "graph" => {
            let Some(Arg::List(l)) = a.first() else {
                return Err(bad("graph(...what): parts or selectors"));
            };
            let mut sels = Vec::new();
            for i in 0..l.len() {
                sels.push(resolved(host, l, i, "graph(...what)")?);
            }
            if let Err(e) = host.graph(&sels) {
                if e.kind != ErrorKind::Unsupported {
                    return Err(plain(e));
                }
                unsupported_once(st, host, "graph", &e);
            }
            undef()
        }
        "watch" => {
            let sel = resolved(host, a, 0, "watch(what)")?;
            if let Err(e) = host.watch(&sel) {
                if e.kind != ErrorKind::Unsupported {
                    return Err(plain(e));
                }
                unsupported_once(st, host, "watch", &e);
            }
            undef()
        }
        // ---- output ----
        "log" => {
            let level = match num(a, 0) {
                Some(l) if l >= 2.0 => Level::Error,
                Some(l) if l >= 1.0 => Level::Warn,
                _ => Level::Info,
            };
            host.log(level, text(a, 1).unwrap_or_default());
            undef()
        }
        "shot" => {
            host.shot(&file_name(text(a, 0).unwrap_or("shot"))).map_err(plain)?;
            undef()
        }
        "help" => s(ctx, &crate::api::help(text(a, 0)))?,
        // ---- documents ----
        "puzzle" => match host.puzzle() {
            None => Value::new_null(ctx.clone()),
            Some(p) => obj(
                ctx,
                &[
                    ("number", n(ctx, f64::from(p.number))),
                    ("active", b(ctx, p.active)),
                    ("ended", b(ctx, p.ended.is_some())),
                    ("solved", b(ctx, p.ended == Some(true))),
                ],
            )?,
        },
        "open" => {
            host.open_file(text(a, 0).unwrap_or_default()).map_err(plain)?;
            undef()
        }
        "loadPuzzle" => {
            let k = whole(a, 0, 1.0, f64::from(i16::MAX), "loadPuzzle(n): n is a puzzle's number")?;
            host.load_puzzle(i16::try_from(k).unwrap_or(0)).map_err(plain)?;
            undef()
        }
        "newWidget" => {
            host.new_widget();
            undef()
        }
        "seed" => {
            let k = whole(a, 0, 0.0, 9_007_199_254_740_991.0, "seed(n): n is a whole number")?;
            host.seed(u64::try_from(k).unwrap_or(0));
            undef()
        }
        _ => return Err(bad(format!("no operation {op}"))),
    })
}

/// A promise the runtime resolves when `kind` comes.
fn wait<'js>(ctx: &Ctx<'js>, st: &Rc<RefCell<State>>, kind: WaitKind) -> rquickjs::Result<Value<'js>> {
    let (p, resolve, reject) = Promise::new(ctx)?;
    let mut s = st.borrow_mut();
    let task = s.current;
    s.waits.push(Wait {
        task,
        kind,
        resolve: Persistent::save(ctx, resolve),
        reject: Persistent::save(ctx, reject),
    });
    Ok(p.into_value())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_settings_read() {
        assert_eq!(vk("a"), Some(0x41));
        assert_eq!(vk("Enter"), Some(0x0D));
        assert_eq!(vk("f5"), Some(0x74));
        assert_eq!(vk("Nope"), None);
        assert_eq!(setting_key("check31"), Some(SettingKey::Check(31)));
        assert_eq!(setting_key("radio4"), None);
        assert_eq!(setting_key("text"), Some(SettingKey::LongText));
        assert_eq!(setting_key("text2"), Some(SettingKey::Text(2)));
    }
}
