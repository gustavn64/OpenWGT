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
`crates/script-js` (below).

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

## The binding (`crates/script-js`)

`script_js::Js` implements `engine::script::runtime::Runtime`; `openwgt`
uses it for `--script FILE.js` (feature `js`, on by default; without it
`.js` scripts are refused and the command language stays). In the
browser build the whole binding adds 1,226,735 bytes to
`widget_bg.wasm` (+14 %, +502 KB gzipped), with no import from `env`.

- **The API** of `crates/script-js/openwgt-script.d.ts` is JavaScript
  (`src/prelude.js`) over one native function that only the prelude
  holds (`src/ops.rs`). The declarations are generated from the table
  in `src/api.rs`, and a test keeps the file in step
  (`UPDATE_DTS=1 cargo test -p script-js` writes it); `help()` and the
  console's completion come from the same table.
- **The host** reaches the natives through a scoped thread-local
  (`scoped-tls-hkt`) for the length of each call from the editor, so the
  crate has no unsafe code. Arguments are read before the host is taken,
  so a getter in the script can't take it twice.
- **A script** is an ES module (strict, top-level `await`) started at the
  next frame. `ticks(n)`, `seconds(s)` and `until(cond)` are promises
  resolved in the runtime's `frame`, `after_tick` and `resume`, in
  "beats": ticks while the widget runs, the time that passes (in ticks)
  while it doesn't. Headless, `resume` sleeps until the next one is due
  (a tick at a time while an `until` or an `onChange` is waiting).
- **Sandbox:** no module loader (static imports don't load, `import()`
  rejects); no `WeakRef`, `FinalizationRegistry`, `SharedArrayBuffer`,
  `Atomics`, `performance`, timers or IO. `Date` is the game's clock,
  1995-09-01 at the first tick and following the ticks; `Math.random` is
  a seeded generator (`seed(n)`), apart from the parts' own.
- **Limits:** the budget (`Limits::budget`, `--budget`) is counted in
  QuickJS's interrupt checks, one per 10,000 operations or so (the
  default 1,000,000 is 100 checks, about 17 ms of a tight loop). Going
  over ends the task with "the script ran too long without waiting (use
  `await ticks(1)` in long loops); stopped", which `try`/`catch` can't
  stop; what it queued runs out under a fresh budget, or QuickJS starts
  again. Memory is capped at 32 MiB ("the script took more memory than
  it may"), the stack at 256 KiB (a `RangeError`).
- **Errors** carry the file, line and column with the source line and a
  caret (QuickJS places a call at its last argument, so failed checks
  are underlined at the `assert` they're in, and a part not found at its
  name). The game's refusals are `WidgetError`s with `.code`, `.part`
  and `.suggestions`. A budget error's place is only as good as QuickJS
  gives it: the function the loop is in.
- `pause()`, `resume()` and `speed()` need the debugging tools (Settings,
  Debugging tools); `graph()` and `watch()` need windows the host doesn't
  have yet. Where they're missing they do nothing, with a warning once.
