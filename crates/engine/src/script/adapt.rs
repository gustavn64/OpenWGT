//! Setting a part's value the way the player does, kind by kind: a
//! Switch is switched, a Slider's knob moved, a Constant's number changed
//! as its arrows would, a Keypad's number typed and entered, a Keypress's
//! key held. While editing, what the part starts with is set instead,
//! through its settings (as its dialog would), except the Slider, whose
//! knob is moved in either case.
//!
//! Parts not here have no value of their own a script could set: they
//! show their inputs, or work their outputs out themselves.

use super::{ErrorKind, SettingKey, SettingVal, Val, value};
use crate::gadget::{Core, Gadget, Tag};
use crate::gadgets::inputs::{NumericKeypad, Slider};
use crate::sim::Io;

/// A part's own control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// The Light Switch (`swit`): on or off.
    Switch,
    /// The Slider (`Sldr`): a number in its range.
    Slider,
    /// The Constant (`cnst`, with arrows) and the Mini Constant (`MiCo`):
    /// a whole number from 0 to 999.
    Constant { arrows: bool },
    /// The Numeric Keypad (`NmKP`): a number that fits its display.
    Keypad,
    /// The Keypress (`Key_`): on while its key is held.
    Keypress,
}

/// The control of the parts of kind `tag`, if they have one.
#[must_use]
pub fn control(tag: Tag) -> Option<Control> {
    Some(match &tag.0 {
        b"swit" => Control::Switch,
        b"Sldr" => Control::Slider,
        b"cnst" => Control::Constant { arrows: true },
        b"MiCo" => Control::Constant { arrows: false },
        b"NmKP" => Control::Keypad,
        b"Key_" => Control::Keypress,
        _ => return None,
    })
}

/// Why a value wasn't set; the host puts the part before it.
#[derive(Debug, Clone, PartialEq)]
pub struct Refusal {
    pub kind: ErrorKind,
    /// `takes on or off, not 1 (a number)`.
    pub why: String,
}

fn refuse(kind: ErrorKind, why: impl Into<String>) -> Refusal {
    Refusal { kind, why: why.into() }
}

fn wrong(takes: &str, v: &Val) -> Refusal {
    refuse(
        ErrorKind::WrongType,
        format!("takes {takes}, not {} ({})", value::format(v), v.kind()),
    )
}

fn bool_of(v: &Val) -> Result<bool, Refusal> {
    match v {
        Val::Bool(b) => Ok(*b),
        _ => Err(wrong("on or off", v)),
    }
}

/// A whole number from 0 to 999, as a Constant takes.
fn constant_of(v: &Val) -> Result<i32, Refusal> {
    match v {
        Val::Number(n) if n.fract() == 0.0 && (0.0..=999.0).contains(n) => {
            // In range and whole, so exact.
            #[allow(clippy::cast_possible_truncation)]
            Ok(*n as i32)
        }
        Val::Number(_) => Err(wrong("a whole number from 0 to 999", v)),
        _ => Err(wrong("a number", v)),
    }
}

fn number_of(v: &Val) -> Result<f64, Refusal> {
    match v {
        Val::Number(n) => Ok(*n),
        _ => Err(wrong("a number", v)),
    }
}

/// The Keypad's text for `n`: what its keys would type.
fn keypad_text(n: f64) -> String {
    value::number(n)
}

/// While editing: the settings that make the part start with `v`;
/// `None` where the control is worked while editing too ([`work`]).
///
/// # Errors
/// A value the part can't take.
pub fn start_settings(c: Control, core: &Core, v: &Val) -> Result<Option<Vec<(SettingKey, SettingVal)>>, Refusal> {
    Ok(Some(match c {
        Control::Switch => vec![(SettingKey::Radio(0), SettingVal::Int(u32::from(bool_of(v)?)))],
        Control::Slider => return Ok(None),
        Control::Constant { .. } => vec![(SettingKey::Text(0), SettingVal::Text(constant_of(v)?.to_string()))],
        Control::Keypad => {
            let n = number_of(v)?;
            let text = keypad_text(n);
            // As its settings would type it in.
            let mut k = NumericKeypad::default();
            let mut core = core.clone();
            core.settings.set_label(0, text.as_bytes());
            crate::gadget::Behavior::settings_changed(&mut k, &mut core);
            if !keypad_shows(&k, n) {
                return Err(refuse(
                    ErrorKind::BadArg,
                    format!("can't show {text} in its 10 characters"),
                ));
            }
            vec![(SettingKey::Text(0), SettingVal::Text(text))]
        }
        Control::Keypress => {
            return Err(refuse(
                ErrorKind::NotRunning,
                "is on only while its key is held in a running widget (run first)",
            ));
        }
    }))
}

/// Whether the Keypad's display reads `n`, exactly.
#[allow(clippy::float_cmp)]
fn keypad_shows(k: &NumericKeypad, n: f64) -> bool {
    let shown = String::from_utf8_lossy(k.lcd()).trim().to_owned();
    shown.parse::<f64>().is_ok_and(|x| x == n)
}

/// Works part `g`'s control to `v`, as the player would; the value it
/// took. Nothing changes when it's refused.
///
/// # Errors
/// A value the part can't take, or a control that's locked.
pub fn work(c: Control, g: &mut Gadget, io: &mut Io<'_>, v: &Val) -> Result<Val, Refusal> {
    match c {
        Control::Switch => {
            let on = bool_of(v)?;
            if g.core.state != on {
                // Its click, with its sound: it toggles wherever it's hit.
                g.behavior.click(&mut g.core, io, 0, 0);
            }
            Ok(Val::Bool(g.core.state))
        }
        Control::Slider => {
            let n = number_of(v)?;
            let Some(s) = g.behavior.as_any_mut().downcast_mut::<Slider>() else {
                return Err(refuse(ErrorKind::NotSettable, "isn't a slider as the game has it"));
            };
            s.drag_to(&mut g.core, io, n).map(Val::Number).map_err(|(lo, hi)| {
                refuse(
                    ErrorKind::BadArg,
                    format!(
                        "goes from {} to {}; {} is outside",
                        value::number(lo),
                        value::number(hi),
                        value::number(n)
                    ),
                )
            })
        }
        Control::Constant { arrows } => {
            let n = constant_of(v)?;
            if arrows && g.core.settings.check(0) {
                return Err(refuse(
                    ErrorKind::Locked,
                    "has its number locked (Lock Number in its settings)",
                ));
            }
            // Its number is what its settings' text makes of `n`; the
            // settings stay as they were, as when its arrows change it.
            let saved = g.core.settings.clone();
            g.core.settings.set_label(0, n.to_string().as_bytes());
            g.behavior.settings_changed(&mut g.core);
            g.core.settings = saved;
            io.fire(&g.core, 0);
            Ok(Val::from_value(&g.behavior.output(&g.core, 0)))
        }
        Control::Keypad => {
            let n = number_of(v)?;
            if g.core.settings.checks & 1 != 0 {
                return Err(refuse(
                    ErrorKind::Locked,
                    "has its number locked (Lock Number in its settings)",
                ));
            }
            let text = keypad_text(n);
            let Some(k) = g.behavior.as_any_mut().downcast_mut::<NumericKeypad>() else {
                return Err(refuse(ErrorKind::NotSettable, "isn't a keypad as the game has it"));
            };
            // Typed in as its settings would (which only types), then
            // ENTER; tried on a fresh one first.
            let mut core = g.core.clone();
            core.settings.set_label(0, text.as_bytes());
            let mut fresh = NumericKeypad::default();
            crate::gadget::Behavior::settings_changed(&mut fresh, &mut core);
            if !keypad_shows(&fresh, n) {
                return Err(refuse(
                    ErrorKind::BadArg,
                    format!("can't show {text} in its 10 characters"),
                ));
            }
            crate::gadget::Behavior::settings_changed(k, &mut core);
            g.core.number = n;
            io.fire(&g.core, 0);
            Ok(Val::Number(n))
        }
        Control::Keypress => {
            let on = bool_of(v)?;
            let vk = u16::from(g.core.settings.text[0].to_ascii_uppercase());
            let held = &mut io.env.keys_held;
            if on && !held.contains(&vk) {
                held.push(vk);
            } else if !on {
                held.retain(|&k| k != vk);
            }
            Ok(Val::Bool(on))
        }
    }
}
