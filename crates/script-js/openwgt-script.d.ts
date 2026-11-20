// What OpenWGT's scripts can use. Generated from crates/script-js/src/api.rs
// (cargo test -p script-js with UPDATE_DTS=1 writes it again); don't edit.

/** On/off, a number, a picture, a sound, or nothing. */
type Value = boolean | number | Picture | Sound | null;

interface Picture {
  readonly kind: "picture";
  readonly sheet: number;
}

interface Sound {
  readonly kind: "sound";
  /** Samples a second. */
  readonly rate: number;
  /** Samples. */
  readonly length: number;
}

interface Part {
  /** Its id, which stays as parts move or others go. */
  readonly id: number;
  /** Its kind: "LightBulb". */
  readonly kind: string;
  /** Its kind's tag: "bulb". */
  readonly tag: string;
  /** The name it was given (set it to name it). */
  name: string | null;
  /** The text of the Label next to it, or its own text. */
  readonly label: string | null;
  /** Whether its value is on (a number other than 0, or an object). */
  readonly on: boolean;
  /** Its value: its first output's, or for a part that only shows (a Light Bulb) its input's. Setting it works the part's own control (a Switch, Slider, Constant...). */
  value: Value;
  /** Whether it's broken. */
  readonly broken: boolean;
  /** Its left on the worksheet. */
  readonly x: number;
  /** Its top on the worksheet. */
  readonly y: number;
  /** Its width. */
  readonly width: number;
  /** Its height. */
  readonly height: number;
  /** What output knot (the nth output from 0, or "out2") gives. */
  output(knot?: number | string): Value;
  /** What input knot (the nth input from 0, or "in2") reads. */
  input(knot?: number | string): Value;
  /** The part wired into input knot (default the first). */
  source(knot?: number): Part | null;
  /** Clicks the running part (at x, y in it; default its middle). */
  press(x?: number, y?: number): void;
  /** One of its settings: "check0".."check31", "radio0".."radio3", "number0".."number3", "text0".."text3", "text". */
  setting(key: string): boolean | number | string;
  /** Changes one of its settings, as its dialog would. */
  setSetting(key: string, value: boolean | number | string): void;
}

interface Wire {
  /** What's on it. */
  readonly value: Value;
  /** Whether it changed and its reader hasn't looked yet. */
  readonly changed: boolean;
}

interface Clock {
  /** Ticks (1/60 s) since GO. */
  readonly ticks: number;
  /** Seconds since GO. */
  readonly seconds: number;
  /** Whether the widget runs. */
  readonly running: boolean;
  /** Whether the run is held. */
  readonly paused: boolean;
}

interface Mouse {
  /** Moves the mouse there. */
  move(x: number, y: number): void;
  /** Presses a button where the mouse is. */
  down(button?: "left" | "right"): void;
  /** Releases it. */
  up(button?: "left" | "right"): void;
}

interface Console {
  /** Writes to the log. */
  log(...values: unknown[]): void;
  /** Writes to the log. */
  info(...values: unknown[]): void;
  /** Writes to the log. */
  debug(...values: unknown[]): void;
  /** Writes a warning. */
  warn(...values: unknown[]): void;
  /** Writes an error. */
  error(...values: unknown[]): void;
}

/** An operation the game refused. */
declare class WidgetError extends Error {
  /** "UnknownPart", "Ambiguous", "WrongType", "NotSettable", "BadKnot", "BadArg", "NotRunning", "Locked", "Caps", "Timeout", "Document", "Stopped", "Unsupported" */
  readonly code: string;
  /** The part it was about, or what was asked for. */
  readonly part: Part | string | null;
  /** The parts meant, maybe. */
  readonly suggestions: string[];
}

/** A failed check with { fatal: true }. */
declare class AssertionError extends Error {}

/** The part a name, a label, a kind ("LightBulb", "bulb"), "Switch[2]", "#7" (its id) or "@120,80" says. Throws a WidgetError (with suggestions) if none or several answer. */
declare function part(query: string | number | Part): Part;

/** Every part, or those with that name, label or kind, in reading order (top to bottom, then left to right). */
declare function parts(query?: string): Part[];

/** The wire from a part (or its knot: "Gate.out") to another ("Lamp.in"). */
declare function wire(from: string | Part, to?: string | Part): Wire;

/** GO. */
declare function run(): void;

/** STOP. */
declare function stop(): void;

/** Holds the run (needs the debugging tools; elsewhere it does nothing). */
declare function pause(): void;

/** Lets a held run go on. */
declare function resume(): void;

/** Slow motion: 1 is full speed, 1/2, 1/4, 1/10 or 1/60 of it; gives the speed it took (needs the debugging tools). */
declare function speed(factor: number): number;

/** Runs that many ticks (default 1) of the running widget at once, without waiting. */
declare function step(ticks?: number): { ticks: number; stopped: boolean; solved: boolean };

/** Waits n ticks (1/60 s each) of the run: await ticks(2). */
declare function ticks(n: number): Promise<void>;

/** Waits s seconds of the run. */
declare function seconds(s: number): Promise<void>;

/** Waits until cond() is true, checked after each tick, for at most max ticks (default 600); gives the ticks it took, or throws a WidgetError (Timeout). */
declare function until(cond: () => unknown, opts?: { max?: number }): Promise<number>;

/** Presses a key ("A", "Enter", "Left" or a Windows key code) for hold ticks (default 5). */
declare function key(k: string | number, opts?: { hold?: number }): void;

/** Types the letters, digits and spaces of text, a key after another. */
declare function type(text: string): void;

/** The mouse, in the window's coordinates. */
declare const mouse: Mouse;

/** The run's clock. */
declare const clock: Clock;

/** Calls f whenever what's value changes (checked after each tick); gives a function that stops it. */
declare function onChange(what: string | Part, f: (value: Value, tick: number) => void): () => void;

/** Shows the values in a graph window (with the debugging tools). */
declare function graph(...what: (string | Part)[]): void;

/** Keeps the value in sight (with the debugging tools). */
declare function watch(what: string | Part): void;

/** Writes the values to the log. */
declare function log(...values: unknown[]): void;

/** console.log, warn and error, into the log. */
declare const console: Console;

/** A check: if cond is false, a failure is counted and logged where it is; with fatal the script stops too. */
declare function assert(cond: unknown, message?: string, opts?: { fatal?: boolean }): void;

/** A check that two values are the same (arrays and objects by their contents). */
declare function assertEqual<T>(actual: T, expected: T, message?: string, opts?: { fatal?: boolean }): void;

/** A test, run after the script with the others, one after another: ✓ or ✗ and the ticks it took. */
declare function test(name: string, f: () => void | Promise<void>): void;

/** A picture of the screen: NAME.png where the program keeps them. */
declare function screenshot(name?: string): void;

/** The puzzle loaded, and whether a run of it ended (solved or not). */
declare function puzzle(): { number: number; active: boolean; ended: boolean; solved: boolean } | null;

/** Opens a widget file (scripts from the command line or the console only). */
declare function open(path: string): void;

/** Loads puzzle n (command line or console only). */
declare function loadPuzzle(n: number): void;

/** A new, empty widget (command line or console only). */
declare function newWidget(): void;

/** Seeds the parts' random numbers and Math.random. */
declare function seed(n: number): void;

/** What there is, or what one name does. */
declare function help(name?: string): string;
