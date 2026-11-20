//! Scripts in JavaScript: an [`engine::script::runtime::Runtime`] on
//! QuickJS (QuickJS-NG through rquickjs; `docs/scripting-js-engine.md`
//! says why).
//!
//! - **The API** (`openwgt-script.d.ts`, from [`api::ITEMS`]) is written
//!   in JavaScript (`prelude.js`) over one native function, which only the
//!   prelude holds ([`ops`]).
//! - **A script** is an ES module (strict, with top-level `await`), run
//!   from the next frame. Its waits (`ticks`, `seconds`, `until`) are
//!   promises the runtime resolves as the widget ticks; `test()`s run
//!   after it, one after another.
//! - **The sandbox**: no modules can be loaded, nor files, the network,
//!   timers, `WeakRef`, `SharedArrayBuffer`; `Date` is the game's clock
//!   (it follows the ticks), `Math.random` gives the same numbers each
//!   time.
//! - **Limits**: a budget of work between two waits (QuickJS's interrupt
//!   checks, about one per 10,000 operations; going over stops the task
//!   with an error scripts can't catch), a memory cap (32 MiB), a stack
//!   cap (256 KiB).
//! - **Errors** become [`ScriptError`]s at the script's line and column;
//!   the game's refusals are `WidgetError`s with `.code`, `.part` and
//!   `.suggestions`.

pub mod api;
pub mod errors;
mod ops;
pub mod parts;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use engine::script::runtime::{ErrKind, Limits, Poll, Runtime, ScriptError, TaskId, TaskInfo, TaskState};
use engine::script::{Caps, Clock, Host, Level, Selector, Val};
use rquickjs::context::{EvalOptions, intrinsic};
use rquickjs::promise::PromiseState;
use rquickjs::{CatchResultExt, CaughtError, Context, Ctx, Function, Module, Object, Persistent, Promise, Value};
use scoped_tls_hkt::scoped_thread_local;

use crate::errors::Thrown;

// The host while the runtime works for the editor: the natives reach it
// through this, for the length of a call from the editor.
scoped_thread_local!(static mut HOST: for<'a> &'a mut (dyn Host + 'a));

/// What a script may allocate.
pub const MEMORY: usize = 32 << 20;
/// The deepest the stack may go (QuickJS needs a cap on wasm32).
const STACK: usize = 256 << 10;
/// QuickJS's operations between two interrupt checks, roughly: the
/// budget ([`Limits::budget`]) is counted in these checks.
const PER_CHECK: u64 = 10_000;
/// `Date.now()` at tick 0: 1995-09-01, the game's autumn.
const EPOCH_MS: f64 = 809_913_600_000.0;
/// How long to sleep to see the next tick (a tick is 16⅔ ms).
const ONE_TICK_MS: u64 = 17;
/// The prelude's source: the API.
const PRELUDE: &str = include_str!("prelude.js");

/// The intrinsics scripts get: ECMAScript's, without `performance`,
/// `WeakRef` and `FinalizationRegistry`.
pub(crate) type Intrinsics = (
    intrinsic::Date,
    intrinsic::Eval,
    intrinsic::RegExpCompiler,
    intrinsic::RegExp,
    intrinsic::Json,
    intrinsic::Proxy,
    intrinsic::MapSet,
    intrinsic::TypedArrays,
    intrinsic::Promise,
);

/// A promise the runtime resolves.
struct Wait {
    task: Option<TaskId>,
    kind: WaitKind,
    resolve: Persistent<Function<'static>>,
    reject: Persistent<Function<'static>>,
}

enum WaitKind {
    /// Until the beat ([`State::beats`]) reaches it.
    Ticks(u64),
    /// Until `cond()` is true, checked after each tick, for `max` beats.
    Until {
        cond: Persistent<Function<'static>>,
        start: u64,
        max: u64,
        text: String,
    },
}

/// An `onChange`.
struct Watcher {
    id: u32,
    task: Option<TaskId>,
    sel: Selector,
    last: Option<Val>,
    f: Persistent<Function<'static>>,
}

/// What a task's checks and errors came to.
#[derive(Default)]
struct Record {
    errors: Vec<ScriptError>,
    failures: u32,
}

/// What the natives share with the runtime.
struct State {
    /// The task running now (its waits and watchers are its).
    current: Option<TaskId>,
    caps: Caps,
    /// Each task's file and lines, for errors' places.
    sources: Vec<(TaskId, String, Rc<[String]>)>,
    records: Vec<(TaskId, Record)>,
    /// The time scripts wait in: ticks while the widget runs, the time
    /// that passes (in ticks) while it doesn't.
    beats: u64,
    last_ticks: u32,
    last_running: bool,
    /// Milliseconds × 60 not yet a beat.
    ms_rest: u64,
    waits: Vec<Wait>,
    watchers: Vec<Watcher>,
    next_watch: u32,
    /// Keys to press or release: at which beat, the key, down.
    keys: Vec<(u64, u16, bool)>,
    /// Where `mouse.move` left the mouse.
    mouse: (i32, i32),
    /// The things it said once that the host can't do.
    warned: Vec<&'static str>,
    memory: usize,
    blown: Rc<Cell<bool>>,
}

impl State {
    fn new(blown: Rc<Cell<bool>>, memory: usize) -> State {
        State {
            current: None,
            caps: Caps::ALL,
            sources: Vec::new(),
            records: Vec::new(),
            beats: 0,
            last_ticks: 0,
            last_running: false,
            ms_rest: 0,
            waits: Vec::new(),
            watchers: Vec::new(),
            next_watch: 0,
            keys: Vec::new(),
            mouse: (0, 0),
            warned: Vec::new(),
            memory,
            blown,
        }
    }

    /// The beat now: what's been seen, and the ticks run since.
    fn live(&self, c: Clock) -> u64 {
        if !c.running {
            self.beats
        } else if self.last_running && c.ticks >= self.last_ticks {
            self.beats + u64::from(c.ticks - self.last_ticks)
        } else {
            self.beats + u64::from(c.ticks)
        }
    }

    /// `ms` passed and the clock is `c` now.
    fn advance(&mut self, c: Clock, ms: u64) {
        if c.running {
            self.beats = self.live(c);
            self.ms_rest = 0;
        } else {
            self.ms_rest += ms * 60;
            self.beats += self.ms_rest / 1000;
            self.ms_rest %= 1000;
        }
        self.last_ticks = c.ticks;
        self.last_running = c.running;
    }

    fn blown(&self) -> bool {
        self.blown.get()
    }

    /// The current task's file and lines.
    fn source(&self) -> (String, Rc<[String]>) {
        self.current
            .and_then(|t| self.sources.iter().find(|(id, ..)| *id == t))
            .map_or_else(
                || ("script".into(), Rc::from(Vec::new())),
                |(_, f, s)| (f.clone(), s.clone()),
            )
    }

    fn record(&mut self, task: TaskId) -> &mut Record {
        if let Some(i) = self.records.iter().position(|(t, _)| *t == task) {
            return &mut self.records[i].1;
        }
        self.records.push((task, Record::default()));
        &mut self.records.last_mut().expect("just pushed").1
    }

    /// A failed check or a test's error in the current task.
    fn failed(&mut self, e: ScriptError) {
        if let Some(t) = self.current {
            let r = self.record(t);
            r.errors.push(e);
            r.failures += 1;
        }
    }

    /// The current task's failed checks.
    fn failures(&self) -> u32 {
        self.current
            .and_then(|t| self.records.iter().find(|(id, _)| *id == t))
            .map_or(0, |(_, r)| r.failures)
    }

    /// Whether `what` wasn't said yet (and now is).
    fn warn_once(&mut self, what: &'static str) -> bool {
        if self.warned.contains(&what) {
            return false;
        }
        self.warned.push(what);
        true
    }

    /// Forgets task `t`'s waits and watchers (it ended badly).
    fn drop_task(&mut self, t: TaskId) {
        self.waits.retain(|w| w.task != Some(t));
        self.watchers.retain(|w| w.task != Some(t));
    }
}

/// The budget's counter, shared with QuickJS's interrupt handler.
#[derive(Default)]
struct Meter {
    checks: Cell<u64>,
    limit: Cell<u64>,
}

/// A task: a script started, or a console line still waiting.
struct Task {
    id: TaskId,
    file: String,
    text: String,
    src: Rc<[String]>,
    caps: Caps,
    state: TaskState,
    started: bool,
    promise: Option<Persistent<Promise<'static>>>,
    /// A console line's: its value is logged when it settles.
    console: bool,
}

/// QuickJS: the runtime, the context, and the prelude's hooks.
struct Engine {
    hooks: Persistent<Object<'static>>,
    ctx: Context,
    rt: rquickjs::Runtime,
}

/// The JavaScript runtime.
pub struct Js {
    state: Rc<RefCell<State>>,
    meter: Rc<Meter>,
    blown: Rc<Cell<bool>>,
    tasks: Vec<Task>,
    next: u32,
    limits: Limits,
    memory: usize,
    /// What the last [`Runtime::resume`] asked to sleep.
    slept: u64,
    /// A task that ran out of budget or memory in the last [`Js::js`].
    runaway: Option<(Option<TaskId>, Option<Thrown>, ErrKind)>,
    engine: Engine,
}

impl Default for Js {
    fn default() -> Js {
        Js::new()
    }
}

impl Drop for Js {
    fn drop(&mut self) {
        // Every handle on a JavaScript value goes before the engine.
        self.forget_values();
    }
}

pub(crate) fn eval_options(file: &str, global: bool, promise: bool) -> EvalOptions {
    let mut o = EvalOptions::default();
    o.global = global;
    o.strict = !global;
    o.promise = promise;
    o.backtrace_barrier = true;
    o.filename = Some(file.to_owned());
    o
}

/// A caught error as what it says about itself.
pub(crate) fn thrown<'js>(ctx: &Ctx<'js>, e: CaughtError<'js>) -> Thrown {
    match e {
        CaughtError::Exception(x) => Thrown::read(ctx, x.as_value()),
        CaughtError::Value(v) => Thrown::read(ctx, &v),
        CaughtError::Error(e) => Thrown {
            name: "InternalError".into(),
            message: e.to_string(),
            ..Thrown::default()
        },
    }
}

impl Js {
    /// A runtime with the default memory cap ([`MEMORY`]).
    ///
    /// # Panics
    /// QuickJS couldn't start (out of memory).
    #[must_use]
    pub fn new() -> Js {
        Js::with_memory(MEMORY)
    }

    /// A runtime that lets scripts allocate `memory` bytes.
    ///
    /// # Panics
    /// QuickJS couldn't start (out of memory).
    #[must_use]
    pub fn with_memory(memory: usize) -> Js {
        script_js_clock::link();
        let blown = Rc::new(Cell::new(false));
        let state = Rc::new(RefCell::new(State::new(blown.clone(), memory)));
        let meter = Rc::new(Meter::default());
        let engine = Js::engine(&state, &meter, &blown, memory);
        Js {
            state,
            meter,
            blown,
            tasks: Vec::new(),
            next: 0,
            limits: Limits::default(),
            memory,
            slept: 0,
            runaway: None,
            engine,
        }
    }

    fn engine(state: &Rc<RefCell<State>>, meter: &Rc<Meter>, blown: &Rc<Cell<bool>>, memory: usize) -> Engine {
        let rt = rquickjs::Runtime::new().expect("QuickJS starts");
        rt.set_memory_limit(memory);
        rt.set_max_stack_size(STACK);
        let (m, b) = (meter.clone(), blown.clone());
        rt.set_interrupt_handler(Some(Box::new(move || {
            let n = m.checks.get() + 1;
            m.checks.set(n);
            if n > m.limit.get() {
                b.set(true);
                return true;
            }
            false
        })));
        let ctx = Context::custom::<Intrinsics>(&rt).expect("QuickJS starts");
        meter.limit.set(u64::MAX);
        let hooks = ctx.with(|ctx| -> rquickjs::Result<Persistent<Object<'static>>> {
            let f: Function = ctx.eval_with_options(PRELUDE, eval_options(errors::PRELUDE, true, false))?;
            let host = ops::host_fn(&ctx, state.clone())?;
            let hooks: Object = f.call((host,))?;
            Ok(Persistent::save(&ctx, hooks))
        });
        Engine {
            hooks: hooks.expect("the prelude runs"),
            ctx,
            rt,
        }
    }

    /// Lets go of every JavaScript value held from Rust.
    fn forget_values(&mut self) {
        let mut s = self.state.borrow_mut();
        s.waits.clear();
        s.watchers.clear();
        drop(s);
        for t in &mut self.tasks {
            t.promise = None;
        }
    }

    /// QuickJS anew: what a runaway left (a flood of jobs, memory still
    /// held) is gone, and so is every task.
    fn restart(&mut self, why: &str) {
        for t in &mut self.tasks {
            if matches!(t.state, TaskState::Running | TaskState::Sleeping { .. }) {
                t.state = TaskState::Failed;
                let e = ScriptError::at(ErrKind::Cancelled, why, &t.file, 0, 0, 0, None);
                self.state.borrow_mut().record(t.id).errors.push(e);
            }
        }
        self.forget_values();
        let engine = Js::engine(&self.state, &self.meter, &self.blown, self.memory);
        self.engine = engine;
    }

    /// The interrupt checks a frame may take.
    fn budget(&self) -> u64 {
        (self.limits.budget / PER_CHECK).max(1)
    }

    fn task_mut(&mut self, id: TaskId) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Runs `f` in QuickJS for `task` (with its capabilities) under a
    /// fresh budget, then the jobs it queued. If the budget (or memory)
    /// ran out, [`Js::after`] ends the task.
    fn js<R>(&mut self, task: Option<TaskId>, f: impl for<'js> FnOnce(&Ctx<'js>, &Object<'js>) -> R) -> R {
        let caps = task
            .and_then(|t| self.tasks.iter().find(|x| x.id == t))
            .map_or(Caps::ALL, |t| t.caps);
        {
            let mut s = self.state.borrow_mut();
            s.current = task;
            s.caps = caps;
        }
        self.meter.checks.set(0);
        self.meter.limit.set(self.budget());
        self.blown.set(false);
        let hooks = self.engine.hooks.clone();
        let r = self.engine.ctx.with(|ctx| {
            let hooks = hooks.restore(&ctx).expect("the hooks are this engine's");
            f(&ctx, &hooks)
        });
        let mut thrown = None;
        while !self.blown.get() && self.engine.rt.is_job_pending() {
            if let Err(e) = self.engine.rt.execute_pending_job() {
                thrown = Some(e.0.with(|ctx| Thrown::read(&ctx, &ctx.catch())));
                break;
            }
        }
        if self.blown.get() {
            self.runaway = Some((task, thrown, ErrKind::Budget));
        } else if let Some(t) = thrown.filter(|t| t.kind(false) == ErrKind::Memory) {
            self.runaway = Some((task, Some(t), ErrKind::Memory));
        }
        r
    }

    /// After a [`Js::js`] and what its caller made of the result: a task
    /// that ran away ends here if it didn't already.
    fn after(&mut self) {
        if let Some((task, thrown, kind)) = self.runaway.take() {
            self.ran_away(task, thrown, kind);
        }
    }

    /// Task `task` ran out of budget or memory: it ends, what it queued
    /// runs out under a fresh budget, or QuickJS starts again.
    fn ran_away(&mut self, task: Option<TaskId>, thrown: Option<Thrown>, kind: ErrKind) {
        // The error a job ended with (an uncatchable one ends the job).
        let drain = |js: &mut Js| -> Option<Thrown> {
            js.meter.checks.set(0);
            js.meter.limit.set(js.budget());
            js.blown.set(false);
            let mut first = None;
            let mut n = 0;
            while js.engine.rt.is_job_pending() && n < 100_000 && !js.blown.get() {
                if let Err(e) = js.engine.rt.execute_pending_job()
                    && first.is_none()
                {
                    first = Some(e.0.with(|ctx| Thrown::read(&ctx, &ctx.catch())));
                }
                n += 1;
            }
            first
        };
        let job = drain(self);
        if let Some(t) = task {
            // Where it was: the error its promise ended with, if it has.
            let p = self.tasks.iter().find(|x| x.id == t).and_then(|x| x.promise.clone());
            let reason = p.and_then(|p| {
                self.engine.ctx.with(|ctx| {
                    let p = p.restore(&ctx).ok()?;
                    (p.state() == PromiseState::Rejected).then(|| {
                        let _ = p.result::<Value>();
                        Thrown::read(&ctx, &ctx.catch())
                    })
                })
            });
            let placed = |t: &Thrown| !t.stack.is_empty();
            let e = [reason, thrown, job]
                .into_iter()
                .flatten()
                .find(placed)
                .unwrap_or_default();
            self.fail(t, &e, kind);
        }
        drain(self);
        self.engine.rt.run_gc();
        let used = usize::try_from(self.engine.rt.memory_usage().malloc_size).unwrap_or(usize::MAX);
        if self.engine.rt.is_job_pending() || self.blown.get() || used > self.memory / 4 * 3 {
            self.restart("stopped: a script ran away, and the scripts started again");
        }
    }

    /// Ends task `t` with the error `e` (of `kind`), logged.
    fn fail(&mut self, t: TaskId, e: &Thrown, kind: ErrKind) {
        let Some(task) = self.tasks.iter_mut().find(|x| x.id == t) else {
            return;
        };
        if !matches!(task.state, TaskState::Running | TaskState::Sleeping { .. }) {
            return;
        }
        task.state = TaskState::Failed;
        task.promise = None;
        let err = e.error(kind, &task.file, &task.src, self.memory);
        if HOST.is_set() {
            HOST.with(|h| h.log(Level::Error, &err.to_string()));
        }
        let mut s = self.state.borrow_mut();
        s.record(t).errors.push(err);
        s.drop_task(t);
        drop(s);
        if kind == ErrKind::Memory {
            self.engine.rt.run_gc();
            let used = usize::try_from(self.engine.rt.memory_usage().malloc_size).unwrap_or(usize::MAX);
            if used > self.memory / 4 * 3 {
                self.restart("stopped: a script took all the memory, and the scripts started again");
            }
        }
    }

    /// The editor's frame (`ms` of time) or tick: the clock, keys, tasks
    /// to start, waits and watchers, tasks that ended. The host is set.
    fn work(&mut self, ms: u64) {
        let c = HOST.with(|h| h.clock());
        self.state.borrow_mut().advance(c, ms);
        self.keys();
        let starting: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|t| !t.started && t.state == TaskState::Running)
            .map(|t| t.id)
            .collect();
        for id in starting {
            self.begin(id);
        }
        self.wake();
        self.settle();
    }

    /// The keys due now.
    fn keys(&mut self) {
        let c = HOST.with(|h| h.clock());
        let mut s = self.state.borrow_mut();
        let now = s.live(c);
        let mut due: Vec<(u64, u16, bool)> = s.keys.iter().copied().filter(|k| k.0 <= now).collect();
        s.keys.retain(|k| k.0 > now);
        drop(s);
        due.sort_by_key(|k| k.0);
        for (_, vk, down) in due {
            HOST.with(|h| h.key(vk, down));
        }
    }

    /// Runs task `id`'s module.
    fn begin(&mut self, id: TaskId) {
        let Some(t) = self.task_mut(id) else { return };
        t.started = true;
        let (file, text) = (t.file.clone(), t.text.clone());
        let r = self.js(Some(id), |ctx, hooks| {
            let run = || -> rquickjs::Result<Promise> {
                let m = Module::declare(ctx.clone(), file.as_str(), text.as_str())?;
                let (_, p) = m.eval()?;
                let finish: Function = hooks.get("finish")?;
                finish.call((p,))
            };
            match run().catch(ctx) {
                Ok(p) => Ok(Persistent::save(ctx, p)),
                Err(e) => Err(Box::new(thrown(ctx, e))),
            }
        });
        match r {
            Ok(p) => {
                if let Some(t) = self.task_mut(id)
                    && t.state == TaskState::Running
                {
                    t.promise = Some(p);
                }
            }
            Err(e) => {
                let kind = e.kind(self.blown.get());
                self.fail(id, &e, kind);
            }
        }
        self.after();
    }

    /// Resolves the waits that are due, checks the `until`s, and calls
    /// the watchers whose values changed. Waits made meanwhile that are
    /// due already (`ticks(0)`, an `until` true at once) go too.
    fn wake(&mut self) {
        let mut later: Vec<Wait> = Vec::new();
        for _ in 0..16 {
            let now = self.state.borrow().live(HOST.with(|h| h.clock()));
            let waits = std::mem::take(&mut self.state.borrow_mut().waits);
            let (due, rest): (Vec<Wait>, Vec<Wait>) = waits.into_iter().partition(|w| match w.kind {
                WaitKind::Ticks(at) => at <= now,
                WaitKind::Until { .. } => true,
            });
            self.state.borrow_mut().waits = rest;
            if due.is_empty() {
                break;
            }
            for w in due {
                if let Some(w) = self.settle_wait(w, now) {
                    later.push(w);
                }
                self.after();
            }
        }
        let mut s = self.state.borrow_mut();
        later.append(&mut s.waits);
        s.waits = later;
        drop(s);
        self.watch();
    }

    /// Resolves (or rejects) `wait` if it's due at beat `now`; gives it
    /// back if not.
    fn settle_wait(&mut self, wait: Wait, now: u64) -> Option<Wait> {
        let task = wait.task;
        self.js(task, |ctx, _| -> Option<Wait> {
            let settle = |f: &Persistent<Function<'static>>, v| call1(ctx, f, v);
            match &wait.kind {
                WaitKind::Ticks(_) => {
                    settle(&wait.resolve, Value::new_undefined(ctx.clone()));
                    None
                }
                WaitKind::Until { cond, start, max, text } => {
                    let r = cond
                        .clone()
                        .restore(ctx)
                        .and_then(|f| f.call::<_, Value>(()))
                        .catch(ctx);
                    let took = now.saturating_sub(*start);
                    match r {
                        Ok(v) if truthy(ctx, &v) => {
                            #[allow(clippy::cast_precision_loss)]
                            settle(&wait.resolve, Value::new_number(ctx.clone(), took as f64));
                            None
                        }
                        Ok(_) if took >= *max => {
                            let msg = format!(
                                "until({text}) didn't happen within {}",
                                engine::script::value::ticks(u32::try_from(*max).unwrap_or(u32::MAX))
                            );
                            if let Ok(e) = widget_error(ctx, &msg, "Timeout") {
                                settle(&wait.reject, e);
                            }
                            None
                        }
                        Ok(_) => Some(wait),
                        Err(e) => {
                            let v = match e {
                                CaughtError::Exception(x) => x.into_value(),
                                CaughtError::Value(v) => v,
                                CaughtError::Error(e) => rquickjs::String::from_str(ctx.clone(), &e.to_string())
                                    .map_or_else(|_| Value::new_null(ctx.clone()), rquickjs::String::into_value),
                            };
                            settle(&wait.reject, v);
                            None
                        }
                    }
                }
            }
        })
    }

    /// Calls the watchers whose values changed.
    fn watch(&mut self) {
        let ids: Vec<u32> = self.state.borrow().watchers.iter().map(|w| w.id).collect();
        for id in ids {
            let (sel, last, task, f) = {
                let s = self.state.borrow();
                let Some(w) = s.watchers.iter().find(|w| w.id == id) else {
                    continue;
                };
                (w.sel.clone(), w.last.clone(), w.task, w.f.clone())
            };
            let (now, tick) = HOST.with(|h| (h.value(&sel).ok(), h.clock().ticks));
            if now == last {
                continue;
            }
            if let Some(w) = self.state.borrow_mut().watchers.iter_mut().find(|w| w.id == id) {
                w.last.clone_from(&now);
            }
            let v = now.unwrap_or(Val::None);
            let r = self.js(task, |ctx, _| {
                let r = f
                    .restore(ctx)
                    .and_then(|f| {
                        let v = ops::val_to_js(ctx, &v)?;
                        f.call::<_, Value>((v, f64::from(tick)))
                    })
                    .catch(ctx);
                r.err().map(|e| thrown(ctx, e))
            });
            if let Some(e) = r {
                // The watcher goes; its task (if it's still running) with
                // it.
                self.state.borrow_mut().watchers.retain(|w| w.id != id);
                let kind = e.kind(self.blown.get());
                if let Some(t) = task.filter(|t| self.tasks.iter().any(|x| x.id == *t && x.state == TaskState::Running))
                {
                    self.fail(t, &e, kind);
                } else {
                    let (file, src) = task.and_then(|t| self.tasks.iter().find(|x| x.id == t)).map_or_else(
                        || ("script".to_owned(), Rc::from(Vec::new())),
                        |t| (t.file.clone(), t.src.clone()),
                    );
                    let err = e.error(kind, &file, &src, self.memory);
                    HOST.with(|h| h.log(Level::Error, &format!("onChange: {err}")));
                }
            }
            self.after();
        }
    }

    /// Tasks whose promises settled are done (or failed).
    fn settle(&mut self) {
        let pending: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|t| t.promise.is_some() && t.state == TaskState::Running)
            .map(|t| t.id)
            .collect();
        for id in pending {
            let Some(p) = self.tasks.iter().find(|t| t.id == id).and_then(|t| t.promise.clone()) else {
                continue;
            };
            let console = self.tasks.iter().any(|t| t.id == id && t.console);
            let r = self.js(Some(id), |ctx, hooks| -> Option<Result<Option<String>, Thrown>> {
                let p = p.restore(ctx).ok()?;
                match p.state() {
                    PromiseState::Pending => None,
                    PromiseState::Resolved => {
                        let shown = if console {
                            p.result::<Value>()
                                .and_then(Result::ok)
                                .filter(|v| !v.is_undefined())
                                .and_then(|v| show(hooks, v))
                        } else {
                            None
                        };
                        Some(Ok(shown))
                    }
                    PromiseState::Rejected => {
                        let _ = p.result::<Value>();
                        Some(Err(Thrown::read(ctx, &ctx.catch())))
                    }
                }
            });
            match r {
                None => {}
                Some(Ok(shown)) => {
                    if let Some(t) = self.task_mut(id) {
                        t.state = TaskState::Done;
                        t.promise = None;
                    }
                    if let Some(s) = shown {
                        HOST.with(|h| h.log(Level::Info, &s));
                    }
                }
                Some(Err(e)) => {
                    let kind = e.kind(false);
                    self.fail(id, &e, kind);
                }
            }
            self.after();
        }
    }

    /// How long until something is due for `task` (headless), or `None`
    /// if nothing ever will be.
    fn next_due(&self, c: Clock) -> Option<u64> {
        let s = self.state.borrow();
        let now = s.live(c);
        let ms = |at: u64| (at.saturating_sub(now) * 1000).div_ceil(60).max(1);
        let due = s
            .waits
            .iter()
            .map(|w| match w.kind {
                WaitKind::Ticks(at) => ms(at),
                WaitKind::Until { .. } => ONE_TICK_MS,
            })
            .chain(s.keys.iter().map(|k| ms(k.0)))
            .min()?;
        // Watchers see every tick.
        Some(if s.watchers.is_empty() {
            due
        } else {
            due.min(ONE_TICK_MS)
        })
    }
}

/// Calls `f` with `v`, whatever it gives or throws (a promise's resolve or
/// reject: they don't throw).
fn call1<'js>(ctx: &Ctx<'js>, f: &Persistent<Function<'static>>, v: Value<'js>) {
    if let Ok(f) = f.clone().restore(ctx) {
        let _ = f.call::<_, ()>((v,));
    }
}

/// A `WidgetError` (the prelude's class) with `message` and `code`.
fn widget_error<'js>(ctx: &Ctx<'js>, message: &str, code: &str) -> rquickjs::Result<Value<'js>> {
    let class: rquickjs::Constructor = ctx.globals().get("WidgetError")?;
    let e: Value = class.construct((message, code))?;
    Ok(e)
}

/// Whether `v` is true as JavaScript says.
fn truthy<'js>(ctx: &Ctx<'js>, v: &Value<'js>) -> bool {
    let f: rquickjs::Result<Function> = ctx.globals().get("Boolean");
    f.and_then(|f| f.call::<_, bool>((v.clone(),))).unwrap_or(false)
}

/// `v` as the console shows it.
fn show<'js>(hooks: &Object<'js>, v: Value<'js>) -> Option<String> {
    let f: Function = hooks.get("show").ok()?;
    f.call::<_, String>((v,)).ok()
}

impl Runtime for Js {
    fn language(&self) -> &'static str {
        "JavaScript"
    }

    fn check(&mut self, file: &str, src: &str) -> Result<(), ScriptError> {
        let lines: Vec<String> = src.lines().map(str::to_owned).collect();
        self.meter.limit.set(u64::MAX);
        let r = self.engine.ctx.with(|ctx| {
            Module::declare(ctx.clone(), file, src)
                .map(|_| ())
                .catch(&ctx)
                .map_err(|e| Box::new(thrown(&ctx, e)))
        });
        r.map_err(|e| {
            let kind = e.kind(false);
            e.error(kind, file, &lines, self.memory)
        })
    }

    fn start(&mut self, file: &str, src: &str, caps: Caps, _host: &mut dyn Host) -> Result<TaskId, ScriptError> {
        self.check(file, src)?;
        self.next += 1;
        let id = TaskId(self.next);
        let lines: Rc<[String]> = src.lines().map(str::to_owned).collect::<Vec<_>>().into();
        self.state
            .borrow_mut()
            .sources
            .push((id, file.to_owned(), lines.clone()));
        self.tasks.push(Task {
            id,
            file: file.to_owned(),
            text: src.to_owned(),
            src: lines,
            caps,
            state: TaskState::Running,
            started: false,
            promise: None,
            console: false,
        });
        Ok(id)
    }

    fn repl(&mut self, line: &str, host: &mut dyn Host) -> Result<Option<String>, ScriptError> {
        self.next += 1;
        let id = TaskId(self.next);
        let lines: Rc<[String]> = line.lines().map(str::to_owned).collect::<Vec<_>>().into();
        self.state
            .borrow_mut()
            .sources
            .push((id, "console".into(), lines.clone()));
        self.tasks.push(Task {
            id,
            file: "console".into(),
            text: line.to_owned(),
            src: lines.clone(),
            caps: Caps::ALL,
            state: TaskState::Running,
            started: true,
            promise: None,
            console: true,
        });
        let awaits = line.contains("await");
        HOST.set(host, || {
            let c = HOST.with(|h| h.clock());
            self.state.borrow_mut().advance(c, 0);
            let r = self.js(
                Some(id),
                |ctx, hooks| -> Result<Result<Option<String>, Persistent<Promise<'static>>>, Box<Thrown>> {
                    let v: Value = ctx
                        .eval_with_options(line, eval_options("console", true, awaits))
                        .catch(ctx)
                        .map_err(|e| Box::new(thrown(ctx, e)))?;
                    if awaits && let Some(p) = v.as_promise() {
                        let settle: Function = hooks
                            .get("settle")
                            .map_err(|e| Box::new(thrown(ctx, CaughtError::Error(e))))?;
                        let p: Promise = settle
                            .call((p.clone(),))
                            .catch(ctx)
                            .map_err(|e| Box::new(thrown(ctx, e)))?;
                        return Ok(Err(Persistent::save(ctx, p)));
                    }
                    Ok(Ok(if v.is_undefined() { None } else { show(hooks, v) }))
                },
            );
            let blown = self.blown.get();
            self.after();
            if blown || self.tasks.iter().any(|t| t.id == id && t.state == TaskState::Failed) {
                let e = self.errors(id).pop();
                return Err(
                    e.unwrap_or_else(|| ScriptError::at(ErrKind::Budget, errors::BUDGET, "console", 1, 0, 0, None))
                );
            }
            match r {
                Ok(Ok(shown)) => {
                    if let Some(t) = self.task_mut(id) {
                        t.state = TaskState::Done;
                    }
                    self.wake();
                    self.settle();
                    Ok(shown)
                }
                Ok(Err(p)) => {
                    if let Some(t) = self.task_mut(id) {
                        t.promise = Some(p);
                    }
                    self.wake();
                    self.settle();
                    Ok(None)
                }
                Err(e) => {
                    if let Some(t) = self.task_mut(id) {
                        t.state = TaskState::Failed;
                    }
                    let kind = e.kind(false);
                    Err(e.error(kind, "console", &lines, self.memory))
                }
            }
        })
    }

    fn frame(&mut self, host: &mut dyn Host, ms: u64) {
        HOST.set(host, || self.work(ms));
    }

    fn after_tick(&mut self, host: &mut dyn Host) {
        HOST.set(host, || self.work(0));
    }

    fn resume(&mut self, task: TaskId, host: &mut dyn Host) -> Poll {
        let ms = std::mem::take(&mut self.slept);
        HOST.set(host, || {
            self.work(ms);
            let done = |js: &Js| {
                js.tasks
                    .iter()
                    .find(|t| t.id == task)
                    .is_none_or(|t| matches!(t.state, TaskState::Done | TaskState::Failed))
            };
            if done(self) {
                return Poll::Done;
            }
            let c = HOST.with(|h| h.clock());
            if let Some(ms) = self.next_due(c) {
                self.slept = ms;
                return Poll::Sleep(ms);
            }
            // Waiting on a promise nothing will resolve.
            let e = Thrown {
                name: "Error".into(),
                message: "the script waits for something that will never happen (a promise nothing resolves)".into(),
                ..Thrown::default()
            };
            self.fail(task, &e, ErrKind::Runtime);
            Poll::Done
        })
    }

    fn cancel(&mut self, task: Option<TaskId>) {
        let ids: Vec<TaskId> = self
            .tasks
            .iter()
            .filter(|t| task.is_none_or(|id| id == t.id))
            .filter(|t| matches!(t.state, TaskState::Running | TaskState::Sleeping { .. }))
            .map(|t| t.id)
            .collect();
        for id in ids {
            if let Some(t) = self.task_mut(id) {
                t.state = TaskState::Failed;
                t.promise = None;
                let e = ScriptError::at(ErrKind::Cancelled, "stopped", &t.file, 0, 0, 0, None);
                let mut s = self.state.borrow_mut();
                s.record(id).errors.push(e);
                s.drop_task(id);
            }
        }
        if task.is_none() {
            // Stop Scripts: the watchers of finished scripts too.
            self.state.borrow_mut().watchers.clear();
        }
    }

    fn tasks(&self) -> Vec<TaskInfo> {
        let s = self.state.borrow();
        self.tasks
            .iter()
            .filter(|t| !t.console)
            .map(|t| {
                let r = s.records.iter().find(|(id, _)| *id == t.id).map(|(_, r)| r);
                TaskInfo {
                    id: t.id,
                    file: t.file.clone(),
                    state: t.state,
                    line: r.and_then(|r| r.errors.last()).map_or(0, |e| e.line),
                    failures: r.map_or(0, |r| r.failures),
                }
            })
            .collect()
    }

    fn errors(&self, task: TaskId) -> Vec<ScriptError> {
        self.state
            .borrow()
            .records
            .iter()
            .find(|(id, _)| *id == task)
            .map(|(_, r)| r.errors.clone())
            .unwrap_or_default()
    }

    fn complete(&self, line: &str, cursor: usize, host: &dyn Host) -> Vec<String> {
        match api::context(line, cursor) {
            api::Context::Global(w) => api::names(api::Of::Global, &w),
            api::Context::Member(of, w) => api::names(of, &w),
            api::Context::Part(typed) => {
                let t = typed.to_lowercase();
                let mut out: Vec<String> = Vec::new();
                for p in host.parts() {
                    for n in [p.name.clone(), p.label.clone(), Some(p.kind.clone())]
                        .into_iter()
                        .flatten()
                    {
                        if n.to_lowercase().starts_with(&t) && !out.contains(&n) {
                            out.push(n);
                        }
                    }
                }
                out.sort();
                out
            }
        }
    }

    fn set_limits(&mut self, limits: Limits) {
        self.limits = limits;
    }
}

#[cfg(test)]
mod tests;
