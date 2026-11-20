//! Values as text: `on`, `off`, `12.5`, `picture 1204`,
//! `sound 1.3 s @ 11025 Hz`, `nothing`; read back, compared, and
//! conditions shown as a script writes them.

use super::{CmpOp, Cond, Val, select};

/// `v` as a script shows it.
#[must_use]
pub fn format(v: &Val) -> String {
    match v {
        Val::None => "nothing".into(),
        Val::Bool(b) => (if *b { "on" } else { "off" }).into(),
        Val::Number(n) => number(*n),
        Val::Picture(p) => format!("picture {p}"),
        Val::Sound { rate, len } => {
            let secs = if *rate == 0 {
                0.0
            } else {
                f64::from(*len) / f64::from(*rate)
            };
            format!("sound {secs:.1} s @ {rate} Hz")
        }
        Val::NoObject => "no object".into(),
    }
}

/// A number as short as it reads back the same: `12`, `12.5`, `-0.25`.
#[must_use]
pub fn number(n: f64) -> String {
    if n == 0.0 {
        // No "-0".
        "0".into()
    } else {
        format!("{n}")
    }
}

/// Reads a value: `on`/`off` (or `true`/`false`, `yes`/`no`), a number,
/// `picture N`, `nothing`.
///
/// # Errors
/// What it is instead.
pub fn parse(s: &str) -> Result<Val, String> {
    let s = s.trim();
    match s.to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" => return Ok(Val::Bool(true)),
        "off" | "false" | "no" => return Ok(Val::Bool(false)),
        "nothing" | "none" => return Ok(Val::None),
        _ => {}
    }
    if let Some(p) = s.strip_prefix("picture") {
        return p
            .trim()
            .parse()
            .map(Val::Picture)
            .map_err(|_| format!("{s}: a picture is \"picture\" and its number"));
    }
    match s.parse::<f64>() {
        Ok(n) if n.is_finite() => Ok(Val::Number(n)),
        _ => Err(format!(
            "{} isn't a value: say on, off, a number, picture N or nothing",
            quote(s)
        )),
    }
}

/// `s` as a script writes it: bare if it's one word, else in quotes.
#[must_use]
pub fn quote(s: &str) -> String {
    let bare =
        !s.is_empty() && !s.starts_with(['#', '@']) && s.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'));
    if bare { s.to_owned() } else { format!("\"{s}\"") }
}

/// Whether `a` `op` `b` holds. Values of different kinds are never equal
/// when one is nothing; otherwise they can't be compared, and the error
/// says so (`is 3 (a number), which can't be compared with on (on/off)`).
///
/// # Errors
/// Values that can't be compared that way.
pub fn compare(a: &Val, op: CmpOp, b: &Val) -> Result<bool, String> {
    let close = |x: f64, y: f64| (x - y).abs() <= 1e-9 * x.abs().max(y.abs()).max(1.0);
    let ordered = matches!(op, CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge);
    if let (Val::Number(x), Val::Number(y)) = (a, b) {
        let eq = close(*x, *y);
        return Ok(match op {
            CmpOp::Eq => eq,
            CmpOp::Ne => !eq,
            CmpOp::Lt => x < y && !eq,
            CmpOp::Le => x < y || eq,
            CmpOp::Gt => x > y && !eq,
            CmpOp::Ge => x > y || eq,
        });
    }
    let mismatch = || {
        format!(
            "is {} ({}), which can't be compared with {} ({})",
            format(a),
            a.kind(),
            format(b),
            b.kind()
        )
    };
    if ordered {
        return Err(if matches!(a, Val::Number(_)) {
            mismatch()
        } else {
            format!("is {} ({}); {} needs numbers", format(a), a.kind(), op.symbol())
        });
    }
    let nothing = |v: &Val| matches!(v, Val::None | Val::NoObject);
    let same_kind = std::mem::discriminant(a) == std::mem::discriminant(b);
    if !same_kind && !nothing(a) && !nothing(b) {
        return Err(mismatch());
    }
    let eq = a == b;
    Ok(if op == CmpOp::Eq { eq } else { !eq })
}

/// `n` ticks, with seconds: `600 ticks (10 s)`.
#[must_use]
pub fn ticks(n: u32) -> String {
    let s = f64::from(n) / 60.0;
    let unit = if n == 1 { "tick" } else { "ticks" };
    format!("{n} {unit} ({} s)", number((s * 100.0).round() / 100.0))
}

/// `c` as a script writes it.
#[must_use]
pub fn show_cond(c: &Cond) -> String {
    match c {
        Cond::Cmp(s, op, v) => format!("{} {} {}", select::show(s), op.symbol(), format(v)),
        Cond::Truthy(s) => select::show(s),
        Cond::Changed(s) => format!("{} changed", select::show(s)),
        Cond::Running => "running".into(),
        Cond::Stopped => "stopped".into(),
        Cond::Solved => "solved".into(),
        Cond::Not(c) => format!("not {}", show_cond(c)),
        Cond::All(cs) => cs.iter().map(show_cond).collect::<Vec<_>>().join(" and "),
        // `and` binds tighter, as the commands read it.
        Cond::Any(cs) => cs.iter().map(show_cond).collect::<Vec<_>>().join(" or "),
    }
}
