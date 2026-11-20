//! Boa (boa_engine 0.22).

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::rc::Rc;

use boa_engine::context::time::FixedClock;

use boa_engine::object::builtins::{JsFunction, JsPromise};
use boa_engine::{Context, JsArgs, JsResult, JsValue, NativeFunction, Source, js_string};
use web_time::Instant;

use crate::{ASYNC, RUNTIME_ERR, Report, SANDBOX, SPEED, SYNTAX_ERR, ms};

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static NOW: Cell<u64> = const { Cell::new(0) };
    static WAITS: RefCell<Vec<(u64, JsFunction)>> = const { RefCell::new(Vec::new()) };
}

fn new_ctx() -> Context {
    // std::time panics on wasm32-unknown-unknown: the game clock.
    let clock = Rc::new(FixedClock::from_millis((crate::GAME_CLOCK_US / 1000.0) as u64));
    let mut ctx = Context::builder().clock(clock).build().unwrap();
    ctx.runtime_limits_mut().set_recursion_limit(512);
    ctx
}

fn lock_down(ctx: &mut Context) {
    let g = ctx.global_object();
    for n in ["WeakRef", "FinalizationRegistry", "SharedArrayBuffer", "Atomics"] {
        let _ = g.delete_property_or_throw(js_string!(n), ctx);
    }
}

fn eval(ctx: &mut Context, src: &str) -> String {
    let r = ctx.eval(Source::from_bytes(src));
    show(ctx, r)
}

fn eval_file(ctx: &mut Context, src: &str, file: &str) -> String {
    let r = ctx.eval(Source::from_bytes(src).with_path(Path::new(file)));
    show(ctx, r)
}

fn show(ctx: &mut Context, r: JsResult<JsValue>) -> String {
    match r {
        Ok(v) => format!("ok {}", v.display()),
        Err(e) => {
            let disp = e.to_string();
            match e.into_opaque(ctx) {
                Ok(v) => {
                    let stack = v
                        .as_object()
                        .and_then(|o| o.get(js_string!("stack"), ctx).ok())
                        .map(|s| s.display().to_string());
                    format!("THROW {disp} | stack={stack:?}")
                }
                Err(_) => format!("ENGINE ERROR (uncatchable) {disp}"),
            }
        }
    }
}

fn log_fn(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let s = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
    LOG.with(|l| l.borrow_mut().push(s));
    Ok(JsValue::undefined())
}

fn ticks_fn(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let n = args.get_or_undefined(0).to_number(ctx)? as u64;
    let (p, f) = JsPromise::new_pending(ctx);
    WAITS.with(|w| w.borrow_mut().push((NOW.get() + n, f.resolve)));
    Ok(p.into())
}

fn limit(ctx: &mut Context, n: u64) {
    ctx.runtime_limits_mut().set_loop_iteration_limit(n);
}

fn timed(r: &mut Report, ctx: &mut Context, name: &str, src: &str) {
    let t = Instant::now();
    let res = eval(ctx, src);
    r.add(name, format!("{:.1} ms; {res}", ms(t)));
}

/// Boa has no memory cap: run only in a process under a ulimit.
pub fn memory(src: &str) -> Report {
    let mut r = Report::default();
    let mut ctx = new_ctx();
    eprintln!("boa memory test: {src}");
    let t = Instant::now();
    let res = eval(&mut ctx, src);
    r.add("mem", format!("{:.1} ms; {res}", ms(t)));
    r
}

/// One snippet in a fresh context with a loop-iteration limit.
pub fn probe(src: &str, loop_limit: u64) -> String {
    let mut ctx = new_ctx();
    lock_down(&mut ctx);
    limit(&mut ctx, loop_limit);
    let t = Instant::now();
    let res = eval(&mut ctx, src);
    format!("{:.1} ms; {res}", ms(t))
}

pub fn run() -> Report {
    let mut r = Report::default();

    // 3. Sandbox.
    let mut ctx = new_ctx();
    r.add("sandbox (default context)", eval(&mut ctx, SANDBOX));
    lock_down(&mut ctx);
    r.add("sandbox (after deleting 4)", eval(&mut ctx, SANDBOX));
    r.add(
        "import statement (script)",
        eval_file(&mut ctx, "import x from 'y';", "imp.js"),
    );
    r.add(
        "dynamic import()",
        eval(
            &mut ctx,
            "var st = 'pending'; import('y').then(() => st = 'loaded', err => st = 'rejected: ' + err); st",
        ),
    );
    let _ = ctx.run_jobs();
    r.add("dynamic import() after jobs", eval(&mut ctx, "st"));
    r.add("Date.now()", eval(&mut ctx, "Date.now()"));
    r.add(
        "eval/Function ctor",
        eval(&mut ctx, "eval('1+1') + new Function('return 2')()"),
    );

    // 4. Budget: the loop-iteration limit (per loop).
    for n in [100_000u64, 1_000_000] {
        limit(&mut ctx, n);
        timed(
            &mut r,
            &mut ctx,
            &format!("while(true) loop limit {n}"),
            "var spin = 0; while (true) { spin++; }",
        );
        r.add(&format!("  spin count at stop ({n})"), eval(&mut ctx, "spin"));
    }
    limit(&mut ctx, 1_000_000);
    r.add(
        "try/catch around while(true)",
        eval(
            &mut ctx,
            "var caught = 'no'; try { while (true) {} } catch (x) { caught = 'yes ' + x } caught",
        ),
    );
    r.add("  caught? (read after)", eval(&mut ctx, "caught"));
    limit(&mut ctx, 1000);
    timed(
        &mut r,
        &mut ctx,
        "nested loops same frame, limit 1000",
        "for (;;) { for (let j = 0; j < 999; j++) {} }",
    );
    limit(&mut ctx, 1000);
    timed(
        &mut r,
        &mut ctx,
        "loop calling a looping fn, limit 1000 (1e6 iterations)",
        "function inner() { for (let j = 0; j < 999; j++) {} } for (;;) inner();",
    );
    limit(&mut ctx, 3000);
    timed(
        &mut r,
        &mut ctx,
        "loop calling a looping fn, limit 3000 (9e6 iterations)",
        "function inner3() { for (let j = 0; j < 2999; j++) {} } for (;;) inner3();",
    );
    limit(&mut ctx, 100);
    timed(
        &mut r,
        &mut ctx,
        "3 frames of loops, limit 100 (1e6 iterations)",
        "function k3() { for (let k = 0; k < 99; k++) {} } function j3() { for (let j = 0; j < 99; j++) k3(); } for (;;) j3();",
    );
    limit(&mut ctx, 1_000_000);
    timed(
        &mut r,
        &mut ctx,
        "'x'.repeat(2e5) (limit 1e6)",
        "'x'.repeat(2e5).length",
    );
    limit(&mut ctx, 1_000_000);
    timed(
        &mut r,
        &mut ctx,
        "recursion tree depth 20 (2e6 calls, no loop)",
        "{ const t = n => n >= 20 ? 0 : t(n + 1) + t(n + 1) + 1; t(0) }",
    );
    timed(
        &mut r,
        &mut ctx,
        "while(true) [..].map (loop limit 1e6)",
        "while (true) { [1,2,3].map(x => x * 2); }",
    );
    timed(
        &mut r,
        &mut ctx,
        "'x'.repeat(2e7) (limit 1e6)",
        "'x'.repeat(2e7).length",
    );
    limit(&mut ctx, u64::MAX);
    if cfg!(not(target_arch = "wasm32")) {
        timed(&mut r, &mut ctx, "'x'.repeat(2e7) (no limit)", "'x'.repeat(2e7).length");
    }
    limit(&mut ctx, 1_000_000);
    timed(
        &mut r,
        &mut ctx,
        "sort 3e5 (native)",
        "{ let big = []; for (let i = 0; i < 3e5; i++) big.push(i * 7 % 1000); big.sort(); big.length }",
    );
    timed(
        &mut r,
        &mut ctx,
        "deep recursion (limit 512)",
        "function deep(n) { return deep(n + 1) + 1; } deep(0)",
    );
    r.add("usable after limits", eval(&mut ctx, "1 + 2"));
    drop(ctx);

    r.add("mem", "see memory() (run the example with --boa-memory under a ulimit)");

    // 6. Errors.
    let mut ctx = new_ctx();
    r.add("syntax error", eval_file(&mut ctx, SYNTAX_ERR, "syn.js"));
    r.add("runtime error", eval_file(&mut ctx, RUNTIME_ERR, "rt.js"));
    r.add(
        "thrown in nested fn, caught in JS",
        eval_file(
            &mut ctx,
            "function h() { throw new Error('boom'); }\ntry { h(); } catch (x) { x.stack }",
            "c.js",
        ),
    );
    drop(ctx);

    // 7. Async.
    let mut ctx = new_ctx();
    ctx.register_global_callable(js_string!("log"), 1, NativeFunction::from_fn_ptr(log_fn))
        .unwrap();
    ctx.register_global_callable(js_string!("ticks"), 1, NativeFunction::from_fn_ptr(ticks_fn))
        .unwrap();
    LOG.with(|l| l.borrow_mut().clear());
    let res = eval_file(&mut ctx, ASYNC, "async.js");
    LOG.with(|l| l.borrow_mut().push(format!("[eval returned: {res}]")));
    let _ = ctx.run_jobs();
    for tick in 1..=6u64 {
        NOW.set(tick);
        LOG.with(|l| l.borrow_mut().push(format!("tick {tick}")));
        let due: Vec<_> = WAITS.with(|w| {
            let mut w = w.borrow_mut();
            let (due, rest) = w.drain(..).partition(|(at, _)| *at <= tick);
            *w = rest;
            due
        });
        for (_, f) in due {
            f.call(&JsValue::undefined(), &[JsValue::from(tick as f64)], &mut ctx)
                .unwrap();
        }
        let _ = ctx.run_jobs();
    }
    r.add("async log", LOG.with(|l| l.borrow().join(" / ")));
    WAITS.with(|w| w.borrow_mut().clear());
    drop(ctx);

    // 8. Speed.
    let mut ctx = new_ctx();
    for (name, src) in SPEED {
        let mut best = f64::MAX;
        let mut res = String::new();
        for _ in 0..5 {
            let t = Instant::now();
            res = eval(&mut ctx, src);
            best = best.min(ms(t));
        }
        r.add(&format!("speed {name}"), format!("{best:.1} ms ({res})"));
    }
    r
}
