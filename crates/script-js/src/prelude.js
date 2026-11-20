// The API scripts see (openwgt-script.d.ts), over one native function,
// `host(op, ...args)`, which only this closure holds. Evaluated once per
// engine; it gives back what the runtime itself needs.
(function (host) {
  "use strict";
  const G = globalThis;

  // ---- errors --------------------------------------------------------

  /** An operation the game refused: `code` says which kind. */
  class WidgetError extends Error {
    constructor(message, code, part, suggestions) {
      super(message);
      this.code = code;
      this.part = part ?? null;
      this.suggestions = suggestions ?? [];
    }
  }
  Object.defineProperty(WidgetError.prototype, "name", { value: "WidgetError" });

  /** A failed assert({fatal: true}). */
  class AssertionError extends Error {}
  Object.defineProperty(AssertionError.prototype, "name", { value: "AssertionError" });

  // The host throws plain objects marked `widget`; scripts get them as
  // WidgetErrors, from here (so their stack starts at the call).
  function h(op, ...args) {
    try {
      return host(op, ...args);
    } catch (e) {
      if (e !== null && typeof e === "object" && e.widget === true) {
        const p = typeof e.part === "number" ? partOf(e.part) : e.part ?? null;
        throw new WidgetError(e.message, e.code, p, e.suggestions);
      }
      throw e;
    }
  }

  // ---- values --------------------------------------------------------

  class Picture {
    constructor(v) {
      this.kind = "picture";
      this.sheet = v.sheet;
      Object.defineProperty(this, "text", { value: v.text });
      Object.freeze(this);
    }
    toString() {
      return this.text;
    }
  }

  class Sound {
    constructor(v) {
      this.kind = "sound";
      this.rate = v.rate;
      this.length = v.length;
      Object.defineProperty(this, "text", { value: v.text });
      Object.freeze(this);
    }
    toString() {
      return this.text;
    }
  }

  /** A value from the host as scripts see it. */
  function val(v) {
    if (v !== null && typeof v === "object") {
      return v.kind === "picture" ? new Picture(v) : new Sound(v);
    }
    return v;
  }

  // ---- parts ---------------------------------------------------------

  const parts = new Map();
  function partOf(id) {
    let p = parts.get(id);
    if (p === undefined) {
      p = new Part(id);
      parts.set(id, p);
    }
    return p;
  }

  /** What a part argument is to the host: its id, or a selector text. */
  function sel(what, fn) {
    if (what instanceof Part) return what.id;
    if (typeof what === "string" || typeof what === "number") return what;
    throw new WidgetError(
      `${fn}(): a part is a Part, a name, a label, a kind ("LightBulb") or an id ("#7"), not ${describe(what)}`,
      "BadArg",
    );
  }

  function describe(v) {
    if (v === null) return "null";
    if (v === undefined) return "undefined";
    if (typeof v === "string") return `the text ${JSON.stringify(v)}`;
    if (typeof v === "function") return "a function";
    if (Array.isArray(v)) return "a list";
    return `${typeof v === "object" ? "an object" : `the ${typeof v}`} ${show(v)}`;
  }

  class Part {
    #id;
    constructor(id) {
      this.#id = id;
    }
    get id() {
      return this.#id;
    }
    get kind() {
      return h("info", this.#id).kind;
    }
    get tag() {
      return h("info", this.#id).tag;
    }
    get name() {
      return h("info", this.#id).name;
    }
    set name(n) {
      h("setName", this.#id, n ?? null);
    }
    get label() {
      return h("info", this.#id).label;
    }
    get on() {
      return h("on", this.#id);
    }
    get value() {
      return val(h("value", this.#id));
    }
    set value(v) {
      h("set", this.#id, v);
    }
    get broken() {
      return h("info", this.#id).broken;
    }
    get x() {
      return h("info", this.#id).x;
    }
    get y() {
      return h("info", this.#id).y;
    }
    get width() {
      return h("info", this.#id).width;
    }
    get height() {
      return h("info", this.#id).height;
    }
    output(knot) {
      return val(h("output", this.#id, knot));
    }
    input(knot) {
      return val(h("input", this.#id, knot));
    }
    source(knot) {
      const id = h("source", this.#id, knot ?? 0);
      return id === null ? null : partOf(id);
    }
    press(x, y) {
      h("press", this.#id, x, y);
    }
    setting(key) {
      return h("setting", this.#id, key);
    }
    setSetting(key, v) {
      h("setSetting", this.#id, key, v);
    }
    toString() {
      return h("describe", this.#id);
    }
  }

  // ---- showing values ----------------------------------------------

  function show(v, depth = 0) {
    if (typeof v === "string") return depth > 0 ? JSON.stringify(v) : v;
    if (v instanceof Part || v instanceof Picture || v instanceof Sound) return String(v);
    if (v === undefined) return "undefined";
    if (typeof v === "function") return `[function ${v.name || "(anonymous)"}]`;
    if (typeof v === "bigint") return `${v}n`;
    if (typeof v === "symbol") return v.toString();
    if (v instanceof Error) return `${v.name}: ${v.message}`;
    if (typeof v === "number" && Object.is(v, -0)) return "-0";
    if (Array.isArray(v)) {
      if (depth > 2) return "[...]";
      const items = v.slice(0, 100).map((x) => show(x, depth + 1));
      if (v.length > 100) items.push(`... ${v.length - 100} more`);
      return `[${items.join(", ")}]`;
    }
    if (v instanceof Promise) return "Promise";
    if (v !== null && typeof v === "object") {
      if (depth > 2) return "{...}";
      const keys = Object.keys(v);
      const items = keys.slice(0, 50).map((k) => `${k}: ${show(v[k], depth + 1)}`);
      if (keys.length > 50) items.push(`... ${keys.length - 50} more`);
      return items.length ? `{ ${items.join(", ")} }` : "{}";
    }
    return String(v);
  }

  function same(a, b) {
    if (Object.is(a, b)) return true;
    if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
    if (Array.isArray(a) !== Array.isArray(b)) return false;
    if (Object.getPrototypeOf(a) !== Object.getPrototypeOf(b)) return false;
    const ka = Object.keys(a);
    const kb = Object.keys(b);
    if (ka.length !== kb.length) return false;
    return ka.every((k) => Object.hasOwn(b, k) && same(a[k], b[k]));
  }

  // ---- the API -------------------------------------------------------

  const api = {
    part(query) {
      return partOf(h("resolve", sel(query, "part")));
    },
    parts(query) {
      return h("parts", query === undefined ? undefined : String(query)).map(partOf);
    },
    wire(from, to) {
      const w = h("wire", sel(from, "wire"), to === undefined ? undefined : sel(to, "wire"));
      return Object.freeze({
        get value() {
          return val(h("wireValue", w));
        },
        get changed() {
          return h("wireChanged", w);
        },
        toString() {
          return h("wireText", w);
        },
      });
    },
    run() {
      h("run");
    },
    stop() {
      h("stop");
    },
    pause() {
      h("pause", true);
    },
    resume() {
      h("pause", false);
    },
    speed(f) {
      return h("speed", f);
    },
    step(ticks) {
      return h("step", ticks ?? 1);
    },
    ticks(n) {
      return h("ticks", n);
    },
    seconds(s) {
      if (typeof s !== "number" || !(s >= 0)) {
        throw new WidgetError(`seconds(s): s is a number of seconds, not ${describe(s)}`, "BadArg");
      }
      return h("ticks", Math.round(s * 60));
    },
    until(cond, opts) {
      if (typeof cond !== "function") {
        throw new WidgetError(`until(cond): cond is a function, such as () => lamp.on, not ${describe(cond)}`, "BadArg");
      }
      return h("until", cond, opts?.max ?? 600, String(cond));
    },
    key(k, opts) {
      h("key", k, opts?.hold ?? 5);
    },
    type(text) {
      h("type", String(text));
    },
    mouse: Object.freeze({
      move(x, y) {
        h("mouse", "move", x, y);
      },
      down(button) {
        h("mouse", "down", button ?? "left");
      },
      up(button) {
        h("mouse", "up", button ?? "left");
      },
    }),
    clock: Object.freeze({
      get ticks() {
        return h("clock").ticks;
      },
      get seconds() {
        return h("clock").seconds;
      },
      get running() {
        return h("clock").running;
      },
      get paused() {
        return h("clock").paused;
      },
    }),
    onChange(what, f) {
      if (typeof f !== "function") {
        throw new WidgetError(`onChange(what, f): f is a function, not ${describe(f)}`, "BadArg");
      }
      const id = h("onChange", sel(what, "onChange"), (v, t) => f(val(v), t));
      return () => h("offChange", id);
    },
    graph(...what) {
      // Only parts and selectors; values (lamp.input(0)) have no source.
      h("graph", what.filter((w) => w instanceof Part || typeof w === "string").map((w) => sel(w, "graph")));
    },
    watch(what) {
      h("watch", sel(what, "watch"));
    },
    log(...a) {
      h("log", 0, a.map((x) => show(x)).join(" "));
    },
    assert(c, msg, opts) {
      if (!c) {
        const e = new AssertionError(msg ?? "assertion failed");
        h("fail", e.message, e.stack, "assert");
        if (opts?.fatal) throw e;
      }
    },
    assertEqual(a, b, msg, opts) {
      if (!same(a, b)) {
        const what = `expected ${show(b, 1)}, got ${show(a, 1)}`;
        const e = new AssertionError(msg === undefined ? what : `${msg}: ${what}`);
        h("fail", e.message, e.stack, "assertEqual");
        if (opts?.fatal) throw e;
      }
    },
    test(name, f) {
      if (typeof f !== "function") {
        throw new WidgetError(`test(name, f): f is a function, not ${describe(f)}`, "BadArg");
      }
      tests.push([String(name), f]);
    },
    screenshot(name) {
      h("shot", name === undefined ? "shot" : String(name));
    },
    puzzle() {
      return h("puzzle");
    },
    open(path) {
      h("open", String(path));
    },
    loadPuzzle(n) {
      h("loadPuzzle", n);
    },
    newWidget() {
      h("newWidget");
    },
    seed(n) {
      h("seed", n);
      seed = n | 0;
    },
    help(name) {
      return h("help", name === undefined ? undefined : String(name));
    },
  };

  // ---- tests ---------------------------------------------------------

  const tests = [];
  async function runTests() {
    while (tests.length > 0) {
      const [name, f] = tests.shift();
      const t0 = h("beats");
      const before = h("failures");
      try {
        await f();
      } catch (e) {
        h("error", e);
      }
      const n = h("beats") - t0;
      const ok = h("failures") === before;
      h("log", ok ? 0 : 2, `${ok ? "✓" : "✗"} ${name} (${n} tick${n === 1 ? "" : "s"})`);
    }
  }

  // ---- the sandbox ---------------------------------------------------

  for (const name of ["SharedArrayBuffer", "Atomics", "WeakRef", "FinalizationRegistry", "performance"]) {
    delete G[name];
  }

  // Date: the game's clock (it follows the ticks).
  const RealDate = Date;
  const now = () => h("now");
  function GameDate(...a) {
    if (new.target === undefined) return new RealDate(now()).toString();
    return a.length === 0 ? new RealDate(now()) : new RealDate(...a);
  }
  Object.setPrototypeOf(GameDate, RealDate);
  GameDate.prototype = RealDate.prototype;
  GameDate.now = now;
  Object.defineProperty(RealDate.prototype, "constructor", { value: GameDate, writable: true, configurable: true });
  G.Date = GameDate;

  // Math.random: the same numbers every time (seed() changes them), not
  // the parts' own stream.
  let seed = 0x5eed;
  Math.random = function random() {
    seed = (seed + 0x6d2b79f5) | 0;
    let t = seed;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };

  const say = (level) => (...a) => h("log", level, a.map((x) => show(x)).join(" "));
  G.console = Object.freeze({ log: say(0), info: say(0), debug: say(0), warn: say(1), error: say(2) });

  for (const [k, v] of Object.entries(api)) {
    Object.defineProperty(G, k, { value: v, writable: true, configurable: true, enumerable: false });
  }
  G.WidgetError = WidgetError;
  G.AssertionError = AssertionError;

  // What the runtime itself uses.
  return {
    show: (v) => show(v),
    // A script's promise, then its tests.
    finish: async (p) => {
      await p;
      await runTests();
    },
    // A console line's: the value its last statement had.
    settle: async (p) => (await p)?.value,
  };
})
