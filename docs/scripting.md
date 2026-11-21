# Scripting

Scripts drive a widget: they press its switches, wait for its bulbs,
check what its parts show, take pictures, and log what they see. They
are written in one of two languages:

- **the command language**, one command a line, built into every
  build: `press Switch`, `until Lamp == on`, `expect Counter > 3`;
- **JavaScript** (QuickJS, `docs/scripting-js-engine.md`), for anything
  with variables, loops and functions: `await until(() => lamp.on)`.

They run in four places:

| Where | How | What it may do |
|---|---|---|
| The console (F8, with Settings ▸ Debugging tools) | typed, or a `.js`, `.mjs`, `.txt` or `.cmd` file dropped onto the window | everything |
| A widget's own scripts | kept in the widget (Keep in the console's Scripts list), run with ▶ | everything but open, load or start other widgets (`docs/widget-scripts.md`) |
| A scripted part | its script calls `definePart`: a part of your own | its own inputs and outputs, each tick |
| The command line | `openwgt --script FILE` | everything |

`examples/scripts/` has scripts to start from: they run without the CD,
on the sample widgets the port comes with, and a test runs them all.

## A first script

In the command language, `examples/scripts/adder-test.txt`:

```text
open ADDER.WGT
run
press A
step 2
expect #11 == on        # 1 + 0: the sum is on
expect #10 == off       # no carry
```

In JavaScript, `examples/scripts/adder-test.js`:

```js
open("ADDER.WGT");
const a = part("A"), sum = part("#11");
run();
test("1 + 0", async () => {
  a.value = true;
  await ticks(2);
  assertEqual(sum.on, true, "the sum");
});
```

Run either in the game's window, or with no window, as fast as it goes:

```sh
openwgt --open --script examples/scripts/adder-test.txt              # in the window
openwgt --open --headless --script examples/scripts/adder-test.js    # no window
```

`--open` plays on the port's own packs (no CD); with a CD, give it as
usual. Headless, the log goes to the standard output, errors and failed
checks to the standard error, and the exit code says how it went: **0**
it ran through with every check holding, **1** a check failed or it
stopped on an error, **2** it couldn't start (no such file, a syntax
error). That makes a script a test for CI:

```sh
openwgt --open --headless --script tests/my-widget.txt --timeout 60 || exit 1
```

| Option | What it does |
|---|---|
| `--script FILE` | the script: JavaScript if `FILE` ends in `.js` or `.mjs`, else commands |
| `--headless` | no window: runs as fast as it goes, then exits with the code above |
| `--watch` | runs it again each time `FILE` is saved: in the window at once (what ran is stopped), headless once the last run has ended |
| `--out DIR` | where `shot NAME` / `screenshot(NAME)` write `NAME.png` (default: here) |
| `--seed N` | the parts' random numbers (and JavaScript's `Math.random`) start from `N` |
| `--budget N` | the work a script may do between two waits |
| `--timeout S` | stops a headless script after `S` seconds of real time |

A widget given on the command line (`openwgt --script s.txt my.wgt`) is
open when the script starts. `open FILE` looks for a relative `FILE` next
to the script first, then from the current folder, then among the
sample widgets by name (`ADDER.WGT`, as File ▸ Open finds them).

## Writing and editing scripts

### In the game

A double-click on a scripted part, or a click on a script's name in the
console's Scripts list, opens the **script editor**: the console's
fixed-pitch font and colours, line numbers, a caret and a selection.
**Apply** (Ctrl+S) gives the part or the widget the text; a part's
script that doesn't define a part says why under the text, and the part
keeps the script it had. Closed without applying (Esc, Close), the text
waits for the next time it's opened. Undo on the worksheet takes an
applied part's script back.

| Key | What it does |
|---|---|
| arrows, Home, End, Page Up/Down | move (Home: the line's text, then its start); with Shift, select |
| Ctrl+Left/Right, Ctrl+Home/End | a word; the whole text |
| Enter | a new line with the line's indent, one step more after `{`, `(` or `[` |
| Tab, Shift+Tab | two spaces; with lines selected, indent them or take a step back |
| Backspace in an indent | a step back |
| Ctrl+A, C, X, V | select all, copy (the line without a selection), cut, paste |
| Ctrl+Z, Ctrl+Y (Ctrl+Shift+Z) | undo, redo |
| Ctrl+S | apply |
| Esc | the selection away, else the window closed |

**Vim keys**: Settings ▸ Vim keys in the script editor (under Debugging
tools) gives the editor vim's keys instead: normal, insert and visual
(`v`, `V`) modes; counts (`3dd`, `d2w`); the motions `h j k l w b e 0 ^
$ gg G { }`; `i a I A o O`; `x X D C s S Y J ~ r`; the operators `d c y
> <` with a motion or doubled (`dd`, `cw`, `>>`); `p P u` and Ctrl+R;
`/text` with `n N`; and `:w` (apply), `:q` (close, refused with changes
not applied), `:q!` (close and drop them), `:wq` or `:x`, `:N` (line N).

### In your own editor

The script editor's **External editor** button (in the program, not the
browser) writes the text to a file in the temporary folder's
`openwgt-scripts` and opens it in your editor; each time you save it
there, it comes back into the window and is applied. The editor is
`$OPENWGT_EDITOR` if it's set (a command; the file goes at its end, so
`OPENWGT_EDITOR="code --wait"` or `"gvim"`), else the system's: the
desktop's for the file (`xdg-open`), TextEdit (`open -t`) on macOS,
Notepad on Windows.

From a terminal, `--watch` keeps a script running as you write it:

```sh
openwgt --open --watch --script my-test.js my-widget.wgt               # the window follows each save
openwgt --open --headless --watch --script my-test.js my-widget.wgt    # the results, each save
```

For JavaScript, editors that read TypeScript's declarations (VS Code
and others) complete and check the API with
`crates/script-js/openwgt-script.d.ts`:

```js
/// <reference path="path/to/openwgt-script.d.ts" />
// @ts-check
```

## The command language

A command a line; words apart by spaces; `"a quoted text"` is one word;
`#` starts a comment to the line's end, unless a digit follows it (`#7`
is a part). In the console, a line that starts with `:` is a command
where JavaScript is the default (`:press Switch`). `help` lists the
commands, `help press` tells about one.

- **Parts**: `#7` (its id, shown in the debugging notes and kept as
  parts move), a name the debugging tools gave it, the text of the
  Label next to it, its kind (`LightBulb`, `Light Bulb`, `bulb`), a
  `"quoted text"` for one with spaces, `[2]` for the second of several
  (in reading order: top to bottom, then left to right), or `@X,Y` for
  the part there. A name is looked for first, then a label, then a kind.
  Labels are in the widget's language; `#7` and the kinds are the same
  in every language.
- **Knots**: a part's `.in`, `.in2`..., `.out`, `.out2`..., or `.N` (the
  Nth knot, from 0): `Gate.in2`. A part alone is its first output, or
  for a part that only shows (a bulb) its input.
- **Values**: `on`, `off`, numbers (`3`, `-1.5`, `0x1f`), `picture N`,
  `nothing`.
- **Conditions**: `PART == VALUE` (also `!=`, `<`, `<=`, `>`, `>=`),
  `PART changed`, `PART` (it's on), `running`, `stopped`, `solved`;
  `not` before one, `and` and `or` between them.
- **Settings** (`setting`): `check0`..`check31`, `radio0`..`radio3`,
  `number0`..`number3`, `text0`..`text3`, `text`.

The input commands (`move`, `click`, `drag`, `key`, `wait`...) work in
the editor's coordinates, the menu bar's top left being 0, 0, with the
original's timing, so the original game in a VM can play the same files
(`tools/compare/`). The others work with parts by name and with the run.

<!-- commands -->
| Command | What it does |
|---|---|
| `size W H` | the editor's size, before the script starts (default 640 433) |
| `move X Y` | the mouse there, in steps as a hand moves it |
| `down X Y [right]` | a button pressed there |
| `up X Y [right]` | a button released there |
| `click X Y [right]` | pressed and released there |
| `dclick X Y` | a double click there |
| `drag X0 Y0 X1 Y1` | pressed at the first point, moved, released at the second |
| `key VK` | a key (a Windows virtual-key code) pressed and released |
| `wait MS` | time passes, MS milliseconds |
| `shot [NAME]` | a picture of the screen: NAME.png |
| `dump` | the parts in the editor's order: tag, box and knots, in its coordinates |
| `open FILE` | opens a widget file, as dropped on the window *(not in a widget's own scripts)* |
| `puzzle N` | loads puzzle N, as File > Load Puzzle *(not in a widget's own scripts)* |
| `new` | a new, empty widget *(not in a widget's own scripts)* |
| `run` | GO |
| `stop` | STOP |
| `step [N]` | N ticks (default 1) of the running widget, at once |
| `until COND [max N]` | ticks until COND holds, at most N (default 600, 10 s); fails if it doesn't |
| `press PART [X Y]` | clicks the running part (at X Y in it; default its middle) |
| `set PART VALUE` | works the part's own control (a Switch, Slider, Constant, Keypad, Keypress); while editing, sets what it starts with |
| `poke PART VALUE` | writes the part's own on/off, number or picture; it may change it again |
| `setting PART KEY VALUE` | changes one of the part's settings, as its dialog would |
| `name PART [NAME]` | names the part (no NAME: takes its name away) |
| `seed N` | seeds the parts' random numbers |
| `expect COND` | checks COND; a failure is counted and the script goes on |
| `assert COND` | checks COND; a failure ends the script |
| `log TEXT` | writes TEXT, {PART} and {PART.KNOT} replaced by their values |
| `print KNOT...` | writes the values: Lamp = on |
| `parts` | lists the parts in reading order, with their values |
| `wires` | lists the wires and what's on them |
| `help [COMMAND]` | lists the commands, or tells about one |
| `graph KNOT...` | records the values as the widget runs, and shows them in the graph (debugging tools) |
| `watch KNOT...` | keeps the values in the watch list (debugging tools) |
| `break COND` | holds the run at the tick COND comes to hold; PART changed: at each change (debugging tools) |
| `pause` | holds the run (debugging tools) |
| `resume` | lets the held run go on (debugging tools) |
| `seek TICK` | goes back or forward to TICK of the run recorded since GO, held there (debugging tools) |
<!-- /commands -->

## JavaScript

A script is an ES module: strict, with top-level `await`. It runs from
the next frame; its waits (`ticks`, `seconds`, `until`) are promises the
game resolves as the widget ticks, so a script can wait for a widget
without ever stopping the game. `test(name, f)` runs `f` after the
script, one test after another, and logs ✓ or ✗ with the ticks it took.

- **Errors**: what the game refuses throws a `WidgetError` with `.code`
  (`UnknownPart`, `Ambiguous`, `NotRunning`, `Timeout`...), `.part` and
  `.suggestions` ("did you mean"). Errors are shown at the script's line
  and column.
- **Checks**: `assert` and `assertEqual` count a failure and go on; with
  `{ fatal: true }` the script stops too. A failure makes a headless run
  exit with 1.
- **The sandbox**: no modules, files, network, timers, `WeakRef` or
  `SharedArrayBuffer`. `Date` is the game's clock (it follows the ticks,
  from 1995-09-01); `Math.random` gives the same numbers each run
  (`seed(n)` changes them).
- **Limits**: a budget of work between two waits (going over stops the
  task with an error it can't catch), 32 MiB of memory, a stack cap.

<!-- javascript -->
**Globals**

| | |
|---|---|
| `part(query: string \| number \| Part): Part` | The part a name, a label, a kind ("LightBulb", "bulb"), "Switch[2]", "#7" (its id) or "@120,80" says. Throws a WidgetError (with suggestions) if none or several answer. |
| `parts(query?: string): Part[]` | Every part, or those with that name, label or kind, in reading order (top to bottom, then left to right). |
| `wire(from: string \| Part, to?: string \| Part): Wire` | The wire from a part (or its knot: "Gate.out") to another ("Lamp.in"). |
| `run(): void` | GO. |
| `stop(): void` | STOP. |
| `pause(): void` | Holds the run (needs the debugging tools; elsewhere it does nothing). |
| `resume(): void` | Lets a held run go on. |
| `speed(factor: number): number` | Slow motion: 1 is full speed, 1/2, 1/4, 1/10 or 1/60 of it; gives the speed it took (needs the debugging tools). |
| `step(ticks?: number): { ticks: number; stopped: boolean; solved: boolean }` | Runs that many ticks (default 1) of the running widget at once, without waiting. |
| `ticks(n: number): Promise<void>` | Waits n ticks (1/60 s each) of the run: await ticks(2). |
| `seconds(s: number): Promise<void>` | Waits s seconds of the run. |
| `until(cond: () => unknown, opts?: { max?: number }): Promise<number>` | Waits until cond() is true, checked after each tick, for at most max ticks (default 600); gives the ticks it took, or throws a WidgetError (Timeout). |
| `key(k: string \| number, opts?: { hold?: number }): void` | Presses a key ("A", "Enter", "Left" or a Windows key code) for hold ticks (default 5). |
| `type(text: string): void` | Types the letters, digits and spaces of text, a key after another. |
| `const mouse: Mouse` | The mouse, in the window's coordinates. |
| `const clock: Clock` | The run's clock. |
| `onChange(what: string \| Part, f: (value: Value, tick: number) => void): () => void` | Calls f whenever what's value changes (checked after each tick); gives a function that stops it. |
| `graph(...what: (string \| Part)[]): void` | Shows the values in a graph window (with the debugging tools). |
| `watch(what: string \| Part): void` | Keeps the value in sight (with the debugging tools). |
| `log(...values: unknown[]): void` | Writes the values to the log. |
| `const console: Console` | console.log, warn and error, into the log. |
| `assert(cond: unknown, message?: string, opts?: { fatal?: boolean }): void` | A check: if cond is false, a failure is counted and logged where it is; with fatal the script stops too. |
| `assertEqual<T>(actual: T, expected: T, message?: string, opts?: { fatal?: boolean }): void` | A check that two values are the same (arrays and objects by their contents). |
| `test(name: string, f: () => void \| Promise<void>): void` | A test, run after the script with the others, one after another: ✓ or ✗ and the ticks it took. |
| `screenshot(name?: string): void` | A picture of the screen: NAME.png where the program keeps them. |
| `puzzle(): { number: number; active: boolean; ended: boolean; solved: boolean } \| null` | The puzzle loaded, and whether a run of it ended (solved or not). |
| `open(path: string): void` | Opens a widget file (scripts from the command line or the console only). |
| `loadPuzzle(n: number): void` | Loads puzzle n (command line or console only). |
| `newWidget(): void` | A new, empty widget (command line or console only). |
| `seed(n: number): void` | Seeds the parts' random numbers and Math.random. |
| `help(name?: string): string` | What there is, or what one name does. |

**A part (`part(...)`)**

| | |
|---|---|
| `readonly id: number` | Its id, which stays as parts move or others go. |
| `readonly kind: string` | Its kind: "LightBulb". |
| `readonly tag: string` | Its kind's tag: "bulb". |
| `name: string \| null` | The name it was given (set it to name it). |
| `readonly label: string \| null` | The text of the Label next to it, or its own text. |
| `readonly on: boolean` | Whether its value is on (a number other than 0, or an object). |
| `value: Value` | Its value: its first output's, or for a part that only shows (a Light Bulb) its input's. Setting it works the part's own control (a Switch, Slider, Constant...). |
| `readonly broken: boolean` | Whether it's broken. |
| `readonly x: number` | Its left on the worksheet. |
| `readonly y: number` | Its top on the worksheet. |
| `readonly width: number` | Its width. |
| `readonly height: number` | Its height. |
| `output(knot?: number \| string): Value` | What output knot (the nth output from 0, or "out2") gives. |
| `input(knot?: number \| string): Value` | What input knot (the nth input from 0, or "in2") reads. |
| `source(knot?: number): Part \| null` | The part wired into input knot (default the first). |
| `press(x?: number, y?: number): void` | Clicks the running part (at x, y in it; default its middle). |
| `setting(key: string): boolean \| number \| string` | One of its settings: "check0".."check31", "radio0".."radio3", "number0".."number3", "text0".."text3", "text". |
| `setSetting(key: string, value: boolean \| number \| string): void` | Changes one of its settings, as its dialog would. |

**A wire (`wire(...)`)**

| | |
|---|---|
| `readonly value: Value` | What's on it. |
| `readonly changed: boolean` | Whether it changed and its reader hasn't looked yet. |

**`clock`**

| | |
|---|---|
| `readonly ticks: number` | Ticks (1/60 s) since GO. |
| `readonly seconds: number` | Seconds since GO. |
| `readonly running: boolean` | Whether the widget runs. |
| `readonly paused: boolean` | Whether the run is held. |

**`mouse`**

| | |
|---|---|
| `move(x: number, y: number): void` | Moves the mouse there. |
| `down(button?: "left" \| "right"): void` | Presses a button where the mouse is. |
| `up(button?: "left" \| "right"): void` | Releases it. |

**`console`**

| | |
|---|---|
| `log(...values: unknown[]): void` | Writes to the log. |
| `info(...values: unknown[]): void` | Writes to the log. |
| `debug(...values: unknown[]): void` | Writes to the log. |
| `warn(...values: unknown[]): void` | Writes a warning. |
| `error(...values: unknown[]): void` | Writes an error. |
<!-- /javascript -->

## Scripted parts

A scripted part (the last part of the numbers' and logic's folder while
the debugging tools are on) is a part whose behaviour is a JavaScript
script's. The script calls `definePart` once, with the part's name, up
to four inputs and four outputs, and its `tick`:

```js
definePart({
  name: "Bigger",
  inputs: ["a", "b"],
  outputs: ["out"],
  tick(state, { a, b }) {
    if (a === null) return { out: b };
    if (b === null) return { out: a };
    return { out: a > b ? a : b };
  },
});
```

That's `examples/scripts/bigger-part.js`. Give it to a part with the
script editor (a double-click on the part), or drop the file onto the
window. `docs/widget-scripts.md` tells the rest: values, `start` and the
state, settings, limits, how it's saved, and asking before a widget's
part scripts run.

## Examples

| File | What it shows |
|---|---|
| `examples/scripts/adder-test.txt` | commands: a half adder's switches pressed and its bulbs checked |
| `examples/scripts/adder-test.js` | the same as JavaScript tests, a test for each pair of inputs |
| `examples/scripts/egg-timer.txt` | waiting for a widget (`until`), reading a value (`print`), a picture (`shot`) |
| `examples/scripts/bigger-part.js` | a scripted part of your own |

The first three run headless on the port's own packs, in every language
(`crates/openwgt/tests/examples.rs`).
