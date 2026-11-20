# The port's own parts, and the Oscilloscope

The original has 64 parts in its tray. The port adds parts of its own,
in **families** (`crates/engine/src/port_parts/`): each family brings
its classes, its places in the tray and a pack of pictures, dialogs,
texts and tables, and nothing in the engine knows more of a family than
its entry in `port_parts::FAMILIES`.

| Family | Parts | Code | Pack |
|---|---|---|---|
| The open parts | Oscilloscope (`Scop`), scripted part (`Scrp`, [widget-scripts.md](widget-scripts.md)) | `crates/engine/src/openparts/` | `tools/openparts`, `crates/engine/openparts*` |
| The network parts | Sender (`NSnd`), Receiver (`NRcv`) | `crates/engine/src/net/` | `tools/opennet`, `crates/engine/opennet*` ([net-parts.md](net-parts.md)) |

The open parts need nothing but the game's wires; they don't use the
network parts, so they can go without them.

## What a family has

- **Classes** after the game's: `gadgets::class` finds a tag among the
  game's 64 classes, then the families' (`port_parts::class`). Their
  gadget ids, sheets and knot tables are numbers none of the CDs (English,
  Swedish, French) nor OpenWDATA uses.
- **The tray**: each part a tray index after the game's 64 (65 and 66 the
  network parts', 67 the Oscilloscope's, 68 the scripted part's, there
  only with the debugging tools on too; `port_parts::TRAY_SLOTS` counts
  them), its help line string `2500 +` the index, as the game's, its
  picture a frame of the family's own tray sheet, and what it adds to
  which folder. The tray shows them only with the enhancements on
  (Settings ▸ Enhancements); a widget that has them runs them either way.
- **A pack**, in the layout of OpenWGFX's pictures and OpenWDATA's
  resources, built into the program, with Swedish and French overlays.
  `port_parts::install` puts every family's pack behind all the game's
  data, the CD's or OpenWDATA, in the language of the game's words: they
  fill only what is missing. Families share string blocks (block 161
  holds the tray's names 2560 to 2575): the blocks are merged string by
  string, and `Assets::string` takes a string from them only where the
  game's block has it empty. An export (`--export`) leaves them out.
- **Saving** (`port_parts::wgt`): the original's loader makes a part of
  every chunk it doesn't know, and has no class for these, so the port
  saves every family's parts, with the wires that reach them, inside
  `back` chunks right after `fver`, each starting `00 00 "OWGT" 01` and
  the chunk's place in the file. The original, and any other reader,
  sees backgrounds before the real one (which, read last, wins) and opens
  the widget without the port's parts; the port puts them back where they
  were. A widget link carries them the same way.

## The Oscilloscope

A dark screen with a grid, on which what goes into its two inputs is
drawn as two traces that keep scrolling while the widget runs: **A** in
yellow, **B** in blue. Unlike the Strip Chart (`StCh`), which stops when
its chart is full, it never fills up: the newest column is on the right
and the oldest leaves the screen on the left. It is the last part of the
Display parts' folder (the Strip Chart's), with the enhancements on.

| Knot | At | What |
|---|---|---|
| A | input, (4, 26) | a number or an ON/OFF signal (a picture or a sound counts as ON) |
| B | input, (4, 50) | the same |
| freeze | input, (4, 87) | ON (or a number other than 0) holds the picture; the HOLD lamp lights |
| out | output, (143, 26) | what goes into A, of the same kind: a number, or ON/OFF; off and 0 unwired |

**The screen** is 100 columns of 64 rows (`openparts::scope::PLOT`,
COLUMNS, ROWS). Each column is one, two, four or eight ticks (the time
scale) and keeps, for each input, the first, last, lowest and highest
value in its time; a value of another kind starts the column again, and
a number that isn't finite is left out (it still goes out of the
output). With lines (the default), each column is drawn from its lowest
to its highest row and reaches the last row of the column before, so a
trace is unbroken and an ON/OFF signal is a square wave; with dots, only
the lowest and highest rows. Where an input has nothing (unwired, or a
part that gives nothing), its trace has a gap.

**Numbers** share one range. Automatic: the lowest and highest number on
the screen, widened to whole steps of 1, 2 or 5 times a power of ten
(about four steps; a single value gets ±1); it follows what is on the
screen, so it changes as old values scroll away. Fixed: From and to, as
typed (swapped if the wrong way round, ±1 if the same; a text that
doesn't read as a number makes it automatic). Numbers outside the range
stay at the screen's edge. The range's ends are written in the screen's
corners. **ON/OFF signals** don't use the range: A's are drawn at rows 40
(OFF) and 56 (ON), B's at 8 and 24.

**State**: the columns are the run's only, a ring of 100, cleared at GO,
drawn only while the widget runs; nothing but the settings is saved. The
part reads the settings at GO and uses only its inputs and the run's tick
count, so a run is the same each time.

**Settings** (dialog 2902):

| Control | Settings block | Default |
|---|---|---|
| Number Range: Automatic, From | radio group 0 (0, 1) | Automatic |
| From, to (`9.#-`) | texts 0 and 1; typing either selects From | 0, 100 |
| Time scale: 1×, 2×, 4×, 8× | radio group 1 (0 to 3): ticks a column | 1× |
| Dots only | check 0 | off |

A widget's settings of all zeros is automatic, 1× and lines.

### Numbers

| What | Id |
|---|---|
| Tag | `Scop` (class name `cOscilloscope`) |
| Gadget id (dialog, info `WWGADGET` 1021, kind 500 as the Strip Chart's) | 2902 |
| Sheet (frames 1 and 2, 148 × 100) and knot table | 1903 |
| Tray icon | sheet 1904, frame 1 (50 × 34) |
| Tray index, help string | 67, 2567 |
| Doc text, tip | `WWGADGET` 1020/2902, 4902 |

Checked against every resource of the English, Swedish and French CDs
(`openwgt --export`), OpenWDATA and the network parts: none of these is
used, and string 2567 is empty in each block 161.

### Where it is

- `crates/engine/src/openparts/scope.rs`: the part, and its tests (a
  bench: scrolling, the time scale, freeze, the output, ranges, lines and
  dots, drawing inside the screen).
- `crates/engine/src/openparts/pack.rs`: its pack, and that each
  language has it, with the CDs and without.
- `crates/engine/src/editor/openparts_tests.rs`: the tray with the
  enhancements on and off, the dialog, a running widget.
- `crates/engine/src/port_parts/wgt.rs`: saving it where the original
  doesn't see it.
- `tools/openparts/`: its pictures and data ([README](../tools/openparts/README.md)).
- OpenWDATA's help has a topic for it, The Oscilloscope, in English,
  Swedish and French.
