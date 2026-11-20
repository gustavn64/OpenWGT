//! Phase 0 spike (throwaway, not for merging): QuickJS-NG through
//! `rquickjs` against Boa, on the questions of the scripting plan:
//! sandbox, budget, memory cap, error positions, async, speed.
//!
//! `run_all()` runs every enabled engine and returns a text report.

use web_time::Instant;

#[cfg(feature = "boa")]
pub mod boa;
#[cfg(feature = "quickjs")]
pub mod qjs;

/// The game clock the engines see (µs since the epoch); fixed here.
pub const GAME_CLOCK_US: f64 = 1_000_000_000_000_000.0;

/// QuickJS on wasm32-unknown-unknown imports `env.__rquickjs_host_now_us`
/// unless the module itself defines it. This is the only unsafe glue
/// (`no_mangle`), hence the crate's own `[lints]`.
#[cfg(all(feature = "quickjs", target_arch = "wasm32"))]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn __rquickjs_host_now_us() -> f64 {
    GAME_CLOCK_US
}

/// Lines of `name: result`.
#[derive(Default)]
pub struct Report(pub Vec<(String, String)>);

impl Report {
    pub fn add(&mut self, name: &str, result: impl Into<String>) {
        self.0.push((name.to_string(), result.into()));
    }
}

/// Milliseconds since `t`.
#[must_use]
pub fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// The sandbox probe: what a script can reach.
pub const SANDBOX: &str = "[typeof require, typeof fetch, typeof setTimeout, typeof setInterval, \
     typeof queueMicrotask, typeof WeakRef, typeof FinalizationRegistry, \
     typeof SharedArrayBuffer, typeof Atomics, typeof performance, typeof console, \
     typeof print, typeof std, typeof os, typeof process, typeof Deno, typeof gc, \
     typeof Date, typeof eval, typeof Function, typeof Intl, typeof Temporal].join(',')";

/// Speed probes: (name, source); each returns a number.
pub const SPEED: &[(&str, &str)] = &[
    (
        "arith 1e6 (global block)",
        "{ let s = 0; for (let i = 0; i < 1e6; i++) { s = (s + i * 7) % 1000003; } s }",
    ),
    (
        "arith 1e6 (in a function)",
        "(function () { let s = 0; for (let i = 0; i < 1e6; i++) { s = (s + i * 7) % 1000003; } return s; })()",
    ),
    (
        "fib(25) (2.4e5 calls)",
        "(function () { function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); } return fib(25); })()",
    ),
    (
        "obj prop 1e6 (in a function)",
        "(function () { let o = { x: 0, y: 1 }; for (let i = 0; i < 1e6; i++) { o.x = (o.x + o.y * i) % 65521; } return o.x; })()",
    ),
    (
        "array+string 1e5 (in a function)",
        "(function () { let a = []; for (let i = 0; i < 1e5; i++) a.push('k' + (i % 100)); let m = new Map(); for (const k of a) m.set(k, (m.get(k) || 0) + 1); return m.size; })()",
    ),
];

pub const SYNTAX_ERR: &str = "let a = 1;\nlet b = (;\n";
pub const RUNTIME_ERR: &str = "function f(o) {\n  return o.x.y;\n}\nfunction g() {\n  return f({});\n}\ng();\n";

/// The async probe: two awaits on host promises.
pub const ASYNC: &str = "(async () => {\n  log('start');\n  await ticks(3);\n  log('after 3');\n  await ticks(2);\n  log('after 5');\n  return 42;\n})().then(v => log('done ' + v), e => log('rejected ' + e));\n";

/// Runs every enabled engine.
#[must_use]
pub fn run_all() -> String {
    run()
}

/// Runs every enabled engine.
#[must_use]
pub fn run() -> String {
    #[allow(unused_mut)]
    let mut out = String::new();
    #[cfg(feature = "quickjs")]
    {
        out.push_str("== QuickJS-NG (rquickjs 0.14) ==\n");
        for (k, v) in qjs::run().0 {
            out.push_str(&format!("{k}: {v}\n"));
        }
    }
    #[cfg(feature = "boa")]
    {
        out.push_str("== Boa (boa_engine 0.22) ==\n");
        for (k, v) in boa::run().0 {
            out.push_str(&format!("{k}: {v}\n"));
        }
    }
    out
}

#[cfg(all(feature = "node", target_arch = "wasm32"))]
mod node {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn spike() -> String {
        super::run()
    }

    #[cfg(feature = "quickjs")]
    #[wasm_bindgen]
    pub fn probe_qjs(src: &str, budget: f64) -> String {
        super::qjs::probe(src, budget as u64)
    }

    #[cfg(feature = "boa")]
    #[wasm_bindgen]
    pub fn probe_boa(src: &str, loop_limit: f64) -> String {
        super::boa::probe(src, loop_limit as u64)
    }
}

/// Native probe: `spike --probe qjs|boa BUDGET JS`.
#[must_use]
pub fn probe(engine: &str, src: &str, budget: u64) -> String {
    match engine {
        #[cfg(feature = "quickjs")]
        "qjs" => qjs::probe(src, budget),
        #[cfg(feature = "boa")]
        "boa" => boa::probe(src, budget),
        _ => {
            let _ = (src, budget);
            "engine not built".into()
        }
    }
}
