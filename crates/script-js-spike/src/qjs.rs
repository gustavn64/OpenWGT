//! QuickJS-NG through rquickjs.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rquickjs::context::EvalOptions;
use rquickjs::context::intrinsic as i;
use rquickjs::{CatchResultExt, CaughtError, Context, Ctx, Function, Persistent, Promise, Runtime, Value};
use web_time::Instant;

use crate::{ASYNC, RUNTIME_ERR, Report, SANDBOX, SPEED, SYNTAX_ERR, ms};

/// The intrinsics the plan chose: no Performance, no WeakRef, no
/// DOMException.
type Intrinsics = (
    i::Date,
    i::Eval,
    i::RegExpCompiler,
    i::RegExp,
    i::Json,
    i::Proxy,
    i::MapSet,
    i::TypedArrays,
    i::Promise,
);

pub const MEMORY_LIMIT: usize = 32 << 20;

struct Engine {
    rt: Runtime,
    ctx: Context,
    calls: Rc<Cell<u64>>,
    limit: Rc<Cell<u64>>,
}

impl Engine {
    fn new() -> Self {
        let rt = Runtime::new().unwrap();
        rt.set_memory_limit(MEMORY_LIMIT);
        rt.set_max_stack_size(256 << 10);
        let calls = Rc::new(Cell::new(0u64));
        let limit = Rc::new(Cell::new(u64::MAX));
        {
            let (c, l) = (calls.clone(), limit.clone());
            rt.set_interrupt_handler(Some(Box::new(move || {
                c.set(c.get() + 1);
                c.get() > l.get()
            })));
        }
        let ctx = Context::custom::<Intrinsics>(&rt).unwrap();
        Engine { rt, ctx, calls, limit }
    }

    /// A fresh budget of `n` interrupt-handler calls.
    fn budget(&self, n: u64) {
        self.calls.set(0);
        self.limit.set(n);
    }

    fn lock_down(&self) {
        self.ctx.with(|ctx| {
            let g = ctx.globals();
            for n in ["WeakRef", "FinalizationRegistry", "SharedArrayBuffer", "Atomics"] {
                let _ = g.remove(n);
            }
        });
    }

    fn eval(&self, src: &str) -> String {
        self.eval_opts(src, EvalOptions::default())
    }

    fn eval_opts(&self, src: &str, opts: EvalOptions) -> String {
        self.ctx.with(|ctx| {
            let r = ctx.eval_with_options::<Value, _>(src, opts).catch(&ctx);
            show(&ctx, r)
        })
    }
}

fn opts(file: &str) -> EvalOptions {
    let mut o = EvalOptions::default();
    o.strict = false;
    o.filename = Some(file.to_string());
    o
}

fn to_string<'js>(ctx: &Ctx<'js>, v: Value<'js>) -> String {
    let f: Function = ctx.globals().get("String").unwrap();
    f.call::<_, String>((v,)).unwrap_or_else(|e| format!("<{e}>"))
}

fn show<'js>(ctx: &Ctx<'js>, r: Result<Value<'js>, CaughtError<'js>>) -> String {
    match r {
        Ok(v) => format!("ok {}", to_string(ctx, v)),
        Err(CaughtError::Exception(e)) => {
            let o = e.as_object();
            let name: String = o.get::<_, Option<String>>("name").ok().flatten().unwrap_or_default();
            let line: Option<i32> = o.get("lineNumber").ok().flatten();
            let col: Option<i32> = o.get("columnNumber").ok().flatten();
            let file: Option<String> = o.get("fileName").ok().flatten();
            format!(
                "THROW {name}: {} | lineNumber={line:?} columnNumber={col:?} fileName={file:?} | stack={:?}",
                e.message().unwrap_or_default(),
                e.stack().unwrap_or_default()
            )
        }
        Err(CaughtError::Value(v)) => format!("THROW value {}", to_string(ctx, v)),
        Err(CaughtError::Error(e)) => format!("ERROR {e}"),
    }
}

/// One snippet in a fresh engine with a budget of interrupt calls.
pub fn probe(src: &str, budget: u64) -> String {
    let e = Engine::new();
    e.lock_down();
    e.budget(budget);
    let t = Instant::now();
    let res = e.eval(src);
    format!(
        "{:.1} ms; handler calls {}; malloc_size {}; {res}",
        ms(t),
        e.calls.get(),
        e.rt.memory_usage().malloc_size
    )
}

pub fn run() -> Report {
    let mut r = Report::default();

    // 3. Sandbox.
    let e = Engine::new();
    r.add("sandbox (custom intrinsics)", e.eval(SANDBOX));
    e.lock_down();
    r.add("sandbox (after deleting 4)", e.eval(SANDBOX));
    r.add(
        "import statement (script)",
        e.eval_opts("import x from 'y';", opts("imp.js")),
    );
    let mut m = opts("imp.mjs");
    m.global = false;
    r.add("import statement (module)", e.eval_opts("import x from 'y'; x", m));
    r.add(
        "dynamic import()",
        e.eval("let st = 'pending'; import('y').then(() => st = 'loaded', err => st = 'rejected: ' + err); st"),
    );
    while e.rt.is_job_pending() {
        let _ = e.rt.execute_pending_job();
    }
    r.add("dynamic import() after jobs", e.eval("st"));
    r.add("Date.now() (shim clock)", e.eval("Date.now()"));
    r.add("eval/Function ctor", e.eval("eval('1+1') + new Function('return 2')()"));

    // 4. Budget: interrupt-handler calls per loop iteration first.
    e.budget(u64::MAX);
    e.eval("for (let i = 0; i < 1e7; i++) {}");
    r.add("handler calls per 1e7 empty iterations", e.calls.get().to_string());
    for n in [100u64, 1000] {
        e.budget(n);
        let t = Instant::now();
        let res = e.eval("var spin = 0; while (true) { spin++; }");
        let el = ms(t);
        r.add(&format!("while(true) budget {n} calls"), format!("{el:.1} ms; {res}"));
        r.add(&format!("  spin count at stop (budget {n})"), e.eval("spin"));
    }
    e.budget(1000);
    e.eval("var spin = 0; while (true) { spin++; }");
    let first = e.eval("spin");
    e.budget(1000);
    e.eval("var spin = 0; while (true) { spin++; }");
    r.add(
        "deterministic (same spin twice)",
        format!("{}", first == e.eval("spin")),
    );
    e.budget(1000);
    r.add(
        "try/catch around while(true)",
        e.eval("let caught = 'no'; try { while (true) {} } catch (x) { caught = 'yes ' + x } caught"),
    );
    r.add("  caught? (read after)", {
        e.budget(u64::MAX);
        e.eval("typeof caught == 'undefined' ? 'never assigned' : caught")
    });
    e.budget(1000);
    let t = Instant::now();
    let res = e.eval("for (;;) { for (let j = 0; j < 1e5; j++) {} }");
    r.add("nested loops budget 1000", format!("{:.1} ms; {res}", ms(t)));
    e.budget(1000);
    let t = Instant::now();
    let res = e.eval("function rec() { while (true) { [1,2,3].map(x => x * 2); } } rec()");
    r.add("loop with calls budget 1000", format!("{:.1} ms; {res}", ms(t)));
    e.budget(u64::MAX);
    let t = Instant::now();
    let res = e.eval("{ const t = n => n >= 20 ? 0 : t(n + 1) + t(n + 1) + 1; t(0) }");
    r.add(
        "recursion tree depth 20 (2e6 calls)",
        format!("{:.1} ms; calls {}; {res}", ms(t), e.calls.get()),
    );
    e.budget(1000);
    let t = Instant::now();
    let res = e.eval("{ const t = n => n >= 400 ? 0 : t(n + 1) + t(n + 1) + 1; t(0) }");
    r.add(
        "recursion tree depth 400, budget 1000",
        format!("{:.1} ms; {res}", ms(t)),
    );
    e.budget(1000);
    let t = Instant::now();
    let res = e.eval("{ let n = 0; const f = () => { n++; Promise.resolve().then(f); }; f(); }");
    let mut jobs = 0u64;
    while e.rt.is_job_pending() && e.calls.get() <= 1000 && jobs < 50_000_000 {
        let _ = e.rt.execute_pending_job();
        jobs += 1;
    }
    r.add(
        "microtask flood, host drains with budget",
        format!(
            "{:.1} ms; {res}; jobs run {jobs}; handler calls {}",
            ms(t),
            e.calls.get()
        ),
    );
    e.budget(1000);
    let t = Instant::now();
    let res = e.eval("'x'.repeat(1e8).length");
    r.add("'x'.repeat(1e8) (32 MiB cap)", format!("{:.1} ms; {res}", ms(t)));
    e.budget(1000);
    let t = Instant::now();
    let res = e.eval("'x'.repeat(2e7).length");
    r.add(
        "'x'.repeat(2e7) (native, no interrupt)",
        format!("{:.1} ms; {res}", ms(t)),
    );
    let e2 = Engine::new();
    e2.budget(1000);
    let t = Instant::now();
    let res = e2.eval("let big = []; for (let i = 0; i < 3e5; i++) big.push(i * 7 % 1000); big.sort(); big.length");
    r.add(
        "sort 3e5 (native, 32 MiB)",
        format!(
            "{:.1} ms; calls {}; malloc_size {}; {res}",
            ms(t),
            e2.calls.get(),
            e2.rt.memory_usage().malloc_size
        ),
    );
    drop(e2);
    let e2 = Engine::new();
    e2.rt.set_memory_limit(512 << 20);
    e2.budget(1000);
    let t = Instant::now();
    let res = e2.eval("let big = []; for (let i = 0; i < 3e5; i++) big.push(i * 7 % 1000); big.sort(); big.length");
    r.add(
        "sort 3e5 (native, 512 MiB)",
        format!(
            "{:.1} ms; calls {}; malloc_size {}; {res}",
            ms(t),
            e2.calls.get(),
            e2.rt.memory_usage().malloc_size
        ),
    );
    drop(e2);
    e.budget(u64::MAX);
    r.add("usable after interrupts", e.eval("1 + 2"));
    e.budget(10_000);
    let t = Instant::now();
    let res = e.eval("function deep(n) { return deep(n + 1) + 1; } deep(0)");
    r.add("deep recursion (256 KiB stack)", format!("{:.1} ms; {res}", ms(t)));
    drop(e);

    // 5. Memory cap: each in a fresh runtime.
    for (name, src) in [
        (
            "push(new Array(1e6)) global",
            "let a = []; for (;;) a.push(new Array(1e6));",
        ),
        (
            "push(new Array(1e6).fill(0)) global",
            "let b = []; for (;;) b.push(new Array(1e6).fill(0));",
        ),
        (
            "push(fill) in a function",
            "(function () { let c = []; for (;;) c.push(new Array(1e6).fill(0)); })()",
        ),
        (
            "string doubling",
            "(function () { let s = 'x'; for (;;) s = s + s; })()",
        ),
        (
            "object churn (gc keeps up?)",
            "(function () { let n = 0; for (let i = 0; i < 3e6; i++) { let o = { i, a: [i] }; n += o.a[0] & 1; } return n; })()",
        ),
    ] {
        let e = Engine::new();
        e.budget(100_000);
        let t = Instant::now();
        let res = e.eval(src);
        r.add(
            &format!("mem {name}"),
            format!(
                "{:.1} ms; calls {}; malloc_size {}; {res}",
                ms(t),
                e.calls.get(),
                e.rt.memory_usage().malloc_size
            ),
        );
        e.budget(u64::MAX);
        r.add("  usable after", e.eval("[1,2,3].map(x => x * 2).join()"));
        // recovery: drop the context, new one on the same runtime
        let Engine { rt, ctx, .. } = e;
        drop(ctx);
        rt.run_gc();
        let used = rt.memory_usage().malloc_size;
        let ctx2 = Context::custom::<Intrinsics>(&rt).unwrap();
        let ok = ctx2.with(|ctx| {
            let r = ctx.eval::<Value, _>("[1,2,3].map(x => x * 2).join()").catch(&ctx);
            show(&ctx, r)
        });
        r.add(
            "  after dropping the context",
            format!("malloc_size {used}; new context: {ok}"),
        );
    }

    // 6. Errors.
    let e = Engine::new();
    r.add("syntax error", e.eval_opts(SYNTAX_ERR, opts("syn.js")));
    r.add("runtime error", e.eval_opts(RUNTIME_ERR, opts("rt.js")));
    r.add(
        "thrown in nested fn, caught in JS",
        e.eval_opts(
            "function h() { throw new Error('boom'); }\ntry { h(); } catch (x) { x.stack }",
            opts("c.js"),
        ),
    );
    drop(e);

    // 7. Async: ticks(n) returns a promise the host resolves later.
    let e = Engine::new();
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let now = Rc::new(Cell::new(0u64));
    let waits: Rc<RefCell<Vec<(u64, Persistent<Function<'static>>)>>> = Rc::new(RefCell::new(Vec::new()));
    e.ctx.with(|ctx| install(&ctx, &log, &now, &waits));
    let res = e.eval_opts(ASYNC, opts("async.js"));
    log.borrow_mut().push(format!("[eval returned: {res}]"));
    for tick in 1..=6u64 {
        now.set(tick);
        log.borrow_mut().push(format!("tick {tick}"));
        let due: Vec<_> = {
            let mut w = waits.borrow_mut();
            let (due, rest) = w.drain(..).partition(|(at, _)| *at <= tick);
            *w = rest;
            due
        };
        e.ctx.with(|ctx| {
            for (_, res) in due {
                let f = res.restore(&ctx).unwrap();
                f.call::<_, ()>((tick,)).unwrap();
            }
        });
        let mut jobs = 0;
        while e.rt.is_job_pending() {
            let _ = e.rt.execute_pending_job();
            jobs += 1;
        }
        if jobs > 0 {
            log.borrow_mut().push(format!("  ({jobs} jobs)"));
        }
    }
    r.add("async log", log.borrow().join(" / "));
    // top-level await (promise option)
    let mut o = opts("tla.js");
    o.promise = true;
    let res = e.ctx.with(|ctx| {
        let p = ctx.eval_with_options::<Promise, _>("log('tla'); await ticks(1); log('tla done'); 7", o);
        match p {
            Ok(p) => format!("promise {:?}", p.state()),
            Err(x) => format!("ERR {x}"),
        }
    });
    r.add("top-level await eval", res);
    for tick in 7..=8u64 {
        now.set(tick);
        let due: Vec<_> = waits.borrow_mut().drain(..).collect();
        e.ctx.with(|ctx| {
            for (_, res) in due {
                res.restore(&ctx).unwrap().call::<_, ()>((tick,)).unwrap();
            }
        });
        while e.rt.is_job_pending() {
            let _ = e.rt.execute_pending_job();
        }
    }
    r.add(
        "  log after ticks 7-8",
        log.borrow()
            .iter()
            .rev()
            .take(2)
            .cloned()
            .collect::<Vec<_>>()
            .join(" / "),
    );
    waits.borrow_mut().clear();
    drop(e);

    // 8. Speed.
    let e = Engine::new();
    for (name, src) in SPEED {
        let mut best = f64::MAX;
        let mut res = String::new();
        for _ in 0..5 {
            let t = Instant::now();
            res = e.eval(src);
            best = best.min(ms(t));
        }
        r.add(&format!("speed {name}"), format!("{best:.1} ms ({res})"));
    }
    r
}

fn install<'js>(
    ctx: &Ctx<'js>,
    log: &Rc<RefCell<Vec<String>>>,
    now: &Rc<Cell<u64>>,
    waits: &Rc<RefCell<Vec<(u64, Persistent<Function<'static>>)>>>,
) {
    let g = ctx.globals();
    let l = log.clone();
    g.set(
        "log",
        Function::new(ctx.clone(), move |s: String| l.borrow_mut().push(s)).unwrap(),
    )
    .unwrap();
    let (now, waits) = (now.clone(), waits.clone());
    g.set(
        "ticks",
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, n: f64| -> rquickjs::Result<Promise<'js>> {
                let (p, resolve, _reject) = Promise::new(&ctx)?;
                waits
                    .borrow_mut()
                    .push((now.get() + n as u64, Persistent::save(&ctx, resolve)));
                Ok(p)
            },
        )
        .unwrap(),
    )
    .unwrap();
}
