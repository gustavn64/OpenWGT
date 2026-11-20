# The JavaScript engine for scripts

Scripts (`engine::script`) can be written in JavaScript as well as in
the built-in command language. Two engines were tried side by side
before choosing (the spike, `crates/script-js-spike` in the history of
the `js-spike` branch): **QuickJS-NG through `rquickjs` 0.14** and
**Boa 0.22**, each linked into `openwgt` natively (x86-64 Linux) and
into the browser build (`wasm32-unknown-unknown`, wasm-bindgen 0.2.130).

**Choice: QuickJS (rquickjs).** It builds for the browser with the
pinned wasm-bindgen, adds an eighth of Boa's size there, and is the only
one of the two with a real per-frame budget, a memory cap and a
catchable stack overflow. The cost is a C compiler: clang (with its
wasm32 backend) and `llvm-ar` for the browser build. The binding is
`crates/script-js`.

## Numbers

Measured on 2026-11-20. Natively a release build on x86-64 Linux; "wasm"
is the same code built for `wasm32-unknown-unknown` (release),
wasm-bindgen 0.2.130 `--target nodejs`, run in Node 24.

| | QuickJS-NG (rquickjs 0.14) | Boa (boa_engine 0.22) |
|---|---|---|
| Builds for wasm32 with wasm-bindgen =0.2.130 | yes, with clang 21 (`CC_wasm32_unknown_unknown`) and `llvm-ar` (`AR_wasm32_unknown_unknown`). rquickjs-sys brings its own libc pieces and a shim; the module's one import, `env.__rquickjs_host_now_us`, is defined in Rust (see below), so wasm-bindgen's glue imports nothing from `env` | yes, plain Rust (getrandom's `wasm_js` backend for its `rand`) |
| `openwgt` page module (`widget_bg.wasm`, release, after wasm-bindgen; without: 8,641,094 B, 3,385,635 B gzipped) | 9,772,388 B: **+1.13 MB (+13 %)**, gzipped +468 KB | 16,171,584 B: **+7.53 MB (+87 %)**, gzipped +1.90 MB |
| Syntax error position | `SyntaxError: unexpected token in expression: ';'`, place in `stack`: `at syn.js:2:10` (file, line, column). No `lineNumber`/`columnNumber` properties | `... at line 2, col 10` inside the message only; no file |
| Runtime error position | `stack`: `at f (rt.js:2:10)` per frame | `stack`: `at f (rt.js:2:14)` per frame |
| Stopping `while (true) {}` | interrupt handler, called about every 5,000 simple loop iterations (2,000 calls per 10⁷); returning true throws `InternalError: interrupted`, which `try`/`catch` **can't catch**. Counting calls is deterministic (the same loop stops at the same count). Also stops nested loops, loops in called functions, recursion, and a microtask flood when the host drains jobs under the same count. 1,000 calls ≈ 170 ms natively and in wasm | loop-iteration limit, **per loop**: an uncatchable `RuntimeLimitError`. Loops in called functions each count from 0 (a limit of 3,000 let 9·10⁶ iterations run: 0.2 s natively, 0.9 s in wasm); recursion without loops (2·10⁶ calls) isn't limited at all |
| Memory cap | `set_memory_limit(32 MiB)`: allocation bombs (arrays, string doubling, `'x'.repeat(1e8)`) throw `InternalError: out of memory` (or, when even the error can't be made, `null`) within 25–180 ms; the runtime goes on, and dropping the context frees everything | **none**: a bomb grows until the process or tab dies |
| Stack overflow | `set_max_stack_size(256 KiB)` (mandatory on wasm32): a catchable `RangeError: Maximum call stack size exceeded`, natively and in wasm | recursion limit (512): an uncatchable `RuntimeLimitError` |
| Lock-down | a context with chosen intrinsics (no `performance`, `WeakRef`, `FinalizationRegistry`); `SharedArrayBuffer`/`Atomics` deleted; no module loader: `import` in a script is a syntax error, in a module "could not load module", `import()` rejects | default context, the same four deleted; `import()` rejects |
| Async (`await ticks(n)`) | promises made and resolved from Rust, jobs run by the host; top-level await with `EvalOptions::promise` | the same with `JsPromise::new_pending` and `run_jobs` |
| Speed, native / wasm (ms): arithmetic 10⁶ | 75 / 48 | 54 / 183 |
| fib(25) | 13 / 13 | 45 / 129 |
| object property 10⁶ | 69 / 66 | 156 / 417 |
| array, string, Map 10⁵ | 28 / 44 | 140 / 440 |
| Native builtins that run long | `'x'.repeat(2e7)` 54 ms, not interruptible but bounded by the memory cap | `'x'.repeat(2e7)`: 270 ms unlimited |

## The browser build

- **Toolchain.** rquickjs-sys compiles QuickJS with the C compiler the
  `cc` crate finds for the target: set `CC_wasm32_unknown_unknown` to a
  clang with the wasm32 backend (an unwrapped clang in Nix: the wrapped
  one adds host flags) and `AR_wasm32_unknown_unknown` to `llvm-ar`
  (GNU `ar` writes no index for wasm objects). The flake's dev shell
  sets both; `tools/build-web.sh` falls back to `clang` and `llvm-ar`
  on the path, and the CI's web job installs them.
- **The clock import.** QuickJS's shim reads the time through an import,
  `env.__rquickjs_host_now_us`, which the browser can't resolve. The
  module defines it itself, so the import goes away at link time. That
  takes `#[unsafe(no_mangle)]`, which the workspace's
  `unsafe_code = "forbid"` doesn't allow, so it lives alone in
  `crates/script-js-clock` (its own lints: `unsafe_code = "deny"`, one
  `#[allow]`). It returns 0: scripts never see QuickJS's own clock, as
  `Date` is replaced by the game's.

## Windows (MSVC)

Not built here; from rquickjs's documentation and build script:

- rquickjs 0.14 ships bindings for both `x86_64-pc-windows-msvc` and
  `aarch64-pc-windows-msvc`. Its table marks x86-64 MSVC "tested" and
  arm64 MSVC "untested", and both "experimental" for QuickJS itself.
- The build script compiles with `/std:c11 /experimental:c11atomics`,
  which needs Visual Studio 2022 17.5 or later (the CI's
  `windows-latest` has it). The arm64 build is a cross build from the
  x64 runner with the cc crate finding the ARM64 tools.

**Risk:** the CI's Windows job may fail to build or link QuickJS,
mostly for arm64. The way out without changing code: build that target
with `--no-default-features`, which leaves the `js` feature out (the
game then has the command language only). The page and the other
targets keep it.

## Why not Boa

Boa needs no C toolchain and has no MSVC risk, but in this game it
would mean:
- the browser module almost doubling (+7.5 MB);
- no memory cap: an embedded script could take the tab down;
- a budget that a loop calling a looping function, or plain recursion,
  gets around;
- 3–10× slower in the browser.

The plan's fallback (Boa with loop and recursion limits) stays possible
behind the same `engine::script::Runtime` trait if QuickJS ever has to
go.
