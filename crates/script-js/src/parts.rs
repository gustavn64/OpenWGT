//! Scripted parts in JavaScript: an [`engine::script::parts::PartRuntime`]
//! on QuickJS. Each part's script runs in a context of its own (its
//! globals are its own) on one QuickJS runtime, apart from the console's
//! (`Js`): a part's script sees only `definePart`, `console` and
//! ECMAScript's built-ins, without `Date` (a part has the tick it's told)
//! and with `Math.random` the same numbers each run.
//!
//! ```js
//! definePart({
//!   name: "Smoother",
//!   inputs: [{ name: "in", kind: "number" }],   // or just "in"
//!   outputs: [{ name: "out", kind: "number" }],
//!   settings: { window: 10 },                   // this.settings
//!   start(s) { s.last = []; },                  // at GO, s a new state
//!   tick(s, { in: x }, { tick }) {              // each tick
//!     s.last.push(x); if (s.last.length > this.settings.window) s.last.shift();
//!     return { out: s.last.reduce((p, q) => p + q, 0) / s.last.length };
//!   },
//! });
//! ```
//!
//! An input is `null` unwired or giving nothing, `true`/`false`, a
//! number, or `{kind: "picture", sheet}` / `{kind: "sound", rate,
//! length}`. `tick` returns the outputs it changes (`null` gives
//! nothing); one it leaves out keeps its value. Each call (the script,
//! `start`, a `tick`) has a budget ([`BUDGET_CHECKS`]); going over is an
//! error, as an exception is, and breaks the part.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use engine::script::parts::{MAX_KNOTS, PartDef, PartRuntime};
use engine::script::runtime::{ErrKind, ScriptError};
use engine::script::{Level, Val};
use rquickjs::{CatchResultExt, Context, Ctx, Function, IntoJs, Object, Persistent, Value};

use crate::errors::{self, Thrown};

/// The memory every part's script shares.
pub const MEMORY: usize = 16 << 20;
/// QuickJS's interrupt checks a call may take (each about 10,000 of its
/// operations).
pub const BUDGET_CHECKS: u64 = 50;
/// The most lines a part may log a tick.
const MAX_SAID: usize = 1000;

/// The prelude each part's context runs: it gives `definePart` and
/// `console`, and the hooks the runtime calls.
const PRELUDE: &str = r#"(function () {
  "use strict";
  const G = globalThis;
  for (const n of ["SharedArrayBuffer", "Atomics", "WeakRef", "FinalizationRegistry", "performance", "Date"]) {
    delete G[n];
  }
  let seed = 0x5eed;
  Math.random = function random() {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = seed;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const said = [];
  const show = (x) => {
    if (typeof x === "string") return x;
    try { const j = JSON.stringify(x); return j === undefined ? String(x) : j; } catch (e) { return String(x); }
  };
  const say = (level) => (...a) => { if (said.length < 200) said.push(level + a.map(show).join(" ")); };
  G.console = Object.freeze({ log: say(0), info: say(0), debug: say(0), warn: say(1), error: say(2) });
  let defined = null;
  const names = (list, what) => {
    if (list === undefined) return [];
    if (!Array.isArray(list)) throw new TypeError(`definePart: ${what} is a list of names`);
    if (list.length > MAX) throw new RangeError(`definePart: at most ${MAX} ${what}, not ${list.length}`);
    const out = list.map((k) => (typeof k === "string" ? k : k && typeof k.name === "string" ? k.name : null));
    out.forEach((n, i) => {
      if (n === null) throw new TypeError(`definePart: ${what} ${i + 1} has no name`);
      if (out.indexOf(n) !== i) throw new TypeError(`definePart: two ${what} are called "${n}"`);
    });
    return out;
  };
  G.definePart = function definePart(def) {
    if (defined) throw new Error("definePart was called already: a script defines one part");
    if (typeof def !== "object" || def === null) throw new TypeError("definePart takes {name, inputs, outputs, tick}");
    if (typeof def.tick !== "function") throw new TypeError("definePart: tick must be a function");
    if (def.start !== undefined && typeof def.start !== "function") throw new TypeError("definePart: start must be a function");
    const name = def.name === undefined ? "Script" : String(def.name);
    defined = { def, name, inputs: names(def.inputs, "inputs"), outputs: names(def.outputs, "outputs") };
  };
  let part = null;
  return {
    def() {
      if (!defined) throw new Error("the script didn't call definePart");
      return { name: defined.name, inputs: defined.inputs, outputs: defined.outputs };
    },
    start() {
      const d = defined.def;
      const self = { name: defined.name, settings: Object.assign({}, d.settings) };
      const s = {};
      seed = 0x5eed;
      part = { self, s };
      if (d.start) d.start.call(self, s);
    },
    tick(ins, tick) {
      const o = {};
      defined.inputs.forEach((n, i) => { o[n] = ins[i]; });
      const r = defined.def.tick.call(part.self, part.s, o, { tick });
      if (r === undefined || r === null) return [];
      if (typeof r !== "object") throw new TypeError("tick returns the outputs it changes, as {name: value}");
      return defined.outputs.map((n) => {
        const v = r[n];
        if (v === undefined || v === null || typeof v === "boolean" || typeof v === "number") return v;
        throw new TypeError(`output "${n}" got ${typeof v}: an output gives a number, true or false, or null`);
      });
    },
    said: () => said.splice(0),
  };
})"#;

/// A part's world: its context, the prelude's hooks, its script.
struct Part {
    hooks: Persistent<Object<'static>>,
    code: String,
    /// The lines of its script, for errors' places.
    src: Vec<String>,
    started: bool,
    ctx: Context,
}

/// Scripted parts in JavaScript.
pub struct JsParts {
    parts: HashMap<u32, Part>,
    said: Vec<(Level, String)>,
    checks: Rc<Cell<u64>>,
    limit: Rc<Cell<u64>>,
    blown: Rc<Cell<bool>>,
    rt: rquickjs::Runtime,
}

impl Default for JsParts {
    fn default() -> JsParts {
        JsParts::new()
    }
}

impl Drop for JsParts {
    fn drop(&mut self) {
        // The contexts and their values before the runtime.
        self.parts.clear();
    }
}

/// The file name a part's script's errors give: `part7.js`.
fn file(part: u32) -> String {
    format!("part{part}.js")
}

impl JsParts {
    /// # Panics
    /// QuickJS couldn't start (out of memory).
    #[must_use]
    pub fn new() -> JsParts {
        script_js_clock::link();
        let rt = rquickjs::Runtime::new().expect("QuickJS starts");
        rt.set_memory_limit(MEMORY);
        rt.set_max_stack_size(256 << 10);
        let (checks, limit, blown) = (
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(u64::MAX)),
            Rc::new(Cell::new(false)),
        );
        let (c, l, b) = (checks.clone(), limit.clone(), blown.clone());
        rt.set_interrupt_handler(Some(Box::new(move || {
            c.set(c.get() + 1);
            if c.get() > l.get() {
                b.set(true);
                return true;
            }
            false
        })));
        JsParts {
            parts: HashMap::new(),
            said: Vec::new(),
            checks,
            limit,
            blown,
            rt,
        }
    }

    /// A fresh budget for a call.
    fn arm(&self) {
        self.checks.set(0);
        self.limit.set(BUDGET_CHECKS);
        self.blown.set(false);
    }

    /// An error of part `part`'s from what was thrown.
    fn error(&self, part: u32, src: &[String], t: &Thrown) -> ScriptError {
        let kind = t.kind(self.blown.get());
        let message = match kind {
            ErrKind::Budget => format!(
                "the part's script ran too long (more than about {} operations in one call); the part is broken",
                BUDGET_CHECKS * 10_000
            ),
            ErrKind::Memory => format!(
                "the parts' scripts took more memory than they may ({} MiB)",
                MEMORY >> 20
            ),
            _ if t.name.is_empty() => format!("uncaught {}", t.message),
            _ => format!("{}: {}", t.name, t.message),
        };
        errors::at(kind, message, &t.stack, &file(part), src, None)
    }

    /// Part `part` with `code` run anew: its world, and what it defines.
    fn make(&mut self, part: u32, code: &str) -> Result<(Part, PartDef), ScriptError> {
        self.parts.remove(&part);
        let ctx = Context::custom::<crate::Intrinsics>(&self.rt)
            .map_err(|e| ScriptError::at(ErrKind::Memory, e.to_string(), &file(part), 0, 0, 0, None))?;
        let src: Vec<String> = code.lines().map(str::to_owned).collect();
        self.arm();
        let name = file(part);
        let r = ctx.with(|ctx| -> Result<(Persistent<Object<'static>>, PartDef), Box<Thrown>> {
            let run = || -> rquickjs::Result<(Object, Object)> {
                let prelude = PRELUDE.replace("MAX", &MAX_KNOTS.to_string());
                let f: Function = ctx.eval_with_options(prelude, crate::eval_options(errors::PRELUDE, true, false))?;
                let hooks: Object = f.call(())?;
                ctx.eval_with_options::<(), _>(code, crate::eval_options(&name, true, false))?;
                let def: Function = hooks.get("def")?;
                let d: Object = def.call(())?;
                Ok((hooks, d))
            };
            let (hooks, d) = run().catch(&ctx).map_err(|e| Box::new(crate::thrown(&ctx, e)))?;
            let list = |k: &str| -> Vec<String> { d.get::<_, Vec<String>>(k).unwrap_or_default() };
            let def = PartDef {
                name: d.get::<_, String>("name").unwrap_or_default(),
                inputs: list("inputs"),
                outputs: list("outputs"),
            };
            Ok((Persistent::save(&ctx, hooks), def))
        });
        self.collect(None);
        match r {
            Ok((hooks, def)) => Ok((
                Part {
                    hooks,
                    code: code.to_owned(),
                    src,
                    started: false,
                    ctx,
                },
                def,
            )),
            Err(t) => Err(self.error(part, &src, &t)),
        }
    }

    /// Part `part` with `code`: as it is, or made anew if its script
    /// changed.
    fn part(&mut self, part: u32, code: &str) -> Result<(), ScriptError> {
        if self.parts.get(&part).is_some_and(|p| p.code == code) {
            return Ok(());
        }
        let (p, _) = self.make(part, code)?;
        self.parts.insert(part, p);
        Ok(())
    }

    /// Runs `f` on part `part`'s hooks under a budget.
    fn call<R>(
        &mut self,
        part: u32,
        f: impl for<'js> FnOnce(&Ctx<'js>, Object<'js>) -> rquickjs::Result<R>,
    ) -> Result<R, ScriptError> {
        self.arm();
        let Some(p) = self.parts.get(&part) else {
            return Err(ScriptError::at(
                ErrKind::Runtime,
                "no such part",
                &file(part),
                0,
                0,
                0,
                None,
            ));
        };
        let r = p.ctx.with(|ctx| -> Result<R, Box<Thrown>> {
            let run = || -> rquickjs::Result<R> {
                let hooks = p.hooks.clone().restore(&ctx)?;
                f(&ctx, hooks)
            };
            run().catch(&ctx).map_err(|e| Box::new(crate::thrown(&ctx, e)))
        });
        self.collect(Some(part));
        r.map_err(|t| {
            let src = self.parts.get(&part).map(|p| p.src.clone()).unwrap_or_default();
            self.error(part, &src, &t)
        })
    }

    /// What part `part` (or every part) logged, kept for
    /// [`PartRuntime::take_log`].
    fn collect(&mut self, part: Option<u32>) {
        let ids: Vec<u32> = match part {
            Some(p) => vec![p],
            None => self.parts.keys().copied().collect(),
        };
        for id in ids {
            let Some(p) = self.parts.get(&id) else { continue };
            self.checks.set(0);
            self.limit.set(u64::MAX);
            let lines = p.ctx.with(|ctx| -> rquickjs::Result<Vec<String>> {
                let hooks = p.hooks.clone().restore(&ctx)?;
                let f: Function = hooks.get("said")?;
                f.call(())
            });
            for line in lines.unwrap_or_default() {
                if self.said.len() >= MAX_SAID {
                    break;
                }
                // The level first, a digit.
                let (level, text) = line.split_at(line.len().min(1));
                let level = match level {
                    "1" => Level::Warn,
                    "2" => Level::Error,
                    _ => Level::Info,
                };
                self.said.push((level, format!("{}: {text}", file(id))));
            }
        }
    }
}

/// An input's value in JavaScript.
fn input<'js>(ctx: &Ctx<'js>, v: &Val) -> rquickjs::Result<Value<'js>> {
    Ok(match v {
        Val::None | Val::NoObject => Value::new_null(ctx.clone()),
        Val::Bool(b) => Value::new_bool(ctx.clone(), *b),
        Val::Number(x) => Value::new_float(ctx.clone(), *x),
        Val::Picture(sheet) => {
            let o = Object::new(ctx.clone())?;
            o.set("kind", "picture")?;
            o.set("sheet", *sheet)?;
            o.into_value()
        }
        Val::Sound { rate, len } => {
            let o = Object::new(ctx.clone())?;
            o.set("kind", "sound")?;
            o.set("rate", *rate)?;
            o.set("length", *len)?;
            o.into_value()
        }
    })
}

/// An output's value from JavaScript: `undefined` keeps it.
fn output(v: &Value<'_>) -> Option<Val> {
    if v.is_undefined() {
        None
    } else if let Some(b) = v.as_bool() {
        Some(Val::Bool(b))
    } else if let Some(x) = v.as_number() {
        Some(Val::Number(x))
    } else {
        Some(Val::None)
    }
}

impl PartRuntime for JsParts {
    fn language(&self) -> &'static str {
        "JavaScript"
    }

    fn define(&mut self, part: u32, code: &str) -> Result<PartDef, ScriptError> {
        let (p, def) = self.make(part, code)?;
        self.parts.insert(part, p);
        Ok(def)
    }

    fn start(&mut self, part: u32, code: &str) -> Result<(), ScriptError> {
        self.part(part, code)?;
        self.call(part, |_, hooks| {
            let f: Function = hooks.get("start")?;
            f.call::<_, ()>(())
        })?;
        if let Some(p) = self.parts.get_mut(&part) {
            p.started = true;
        }
        Ok(())
    }

    fn tick(&mut self, part: u32, code: &str, tick: u32, inputs: &[Val]) -> Result<Vec<Option<Val>>, ScriptError> {
        if !self.parts.get(&part).is_some_and(|p| p.code == code && p.started) {
            self.start(part, code)?;
        }
        let inputs = inputs.to_vec();
        self.call(part, move |ctx, hooks| {
            let list = rquickjs::Array::new(ctx.clone())?;
            for (i, v) in inputs.iter().enumerate() {
                list.set(i, input(ctx, v)?)?;
            }
            let f: Function = hooks.get("tick")?;
            let r: rquickjs::Array = f.call((list, tick.into_js(ctx)?))?;
            r.iter::<Value>()
                .map(|v| v.map(|v| output(&v)))
                .collect::<rquickjs::Result<Vec<Option<Val>>>>()
        })
    }

    fn clear(&mut self) {
        self.parts.clear();
        self.rt.run_gc();
    }

    fn take_log(&mut self) -> Vec<(Level, String)> {
        std::mem::take(&mut self.said)
    }
}
