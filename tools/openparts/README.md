# The open parts' own pictures and data

The open parts are the port's own parts for everyone, with nothing but
the game's wires in and out (`engine::openparts`): the
**Oscilloscope** (`Scop`, [docs/oscilloscope.md](../../docs/oscilloscope.md))
and the **scripted part** (`Scrp`, [docs/widget-scripts.md](../../docs/widget-scripts.md)),
whose behaviour is a script's.
Neither the CDs nor OpenWDATA have anything for them, so they bring their
own pictures, dialogs, texts and tables, built into the program from
`crates/engine/openparts` (and `openparts-sv`, `openparts-fr` for their
words in Swedish and French). The engine puts them behind everything
else (`engine::port_parts::install`): they fill only what is missing,
with a CD as without one. They don't need the network parts
(`tools/opennet`), and the network parts don't need them.

```text
sheets.txt                   <sheet> <frame> <top> <left> <bottom> <right>
gadgets/<sheet>/<frame>.png  the pictures (draw.py)
<MODULE>/<type>/<id>.bin     the resources, in the game's formats (build.py)
```

| What | Where |
|---|---|
| The Oscilloscope | sheet 1903, frame 1 (its HOLD lamp dark) and 2 (lit, frozen); 148 × 100, the screen empty at `PLOT` (25, 9), 100 × 64 |
| The tray's icon | sheet 1904, frame 1 (50 × 34) |
| Knots | `WWGADGET` 1014/1903: inputs A (4, 26), B (4, 50) and freeze (4, 87), the output (143, 26) |
| Numbers | `WWGADGET` 1021/2902 (dialog, Doc texts) |
| Doc texts | `WWGADGET` 1020/2902 and the tip 4902 |
| Settings dialog | `WIDGET` 5/2902 |
| Strings | 2567 (the tray's name, block 161) |
| The scripted part | sheet 1905: 1 the body (128 × 84; the name plate empty at `PLATE` (17, 5), 94 × 13), 2 the same with a padlock (its script not allowed), 3 an input's knot and stalk, 4 an output's (both placed by their rectangles about the knot point, so the game draws them only at the knots its script has) |
| Its knots | `WWGADGET` 1014/1905: inputs (4, 30), (4, 44), (4, 58), (4, 72), then outputs at (123, …) the same heights |
| Its numbers, Doc texts | `WWGADGET` 1021/2903, 1020/2903 and the tip 4903 (no settings dialog) |
| Its tray icon, name | sheet 1904, frame 2; string 2568 |

None of these numbers is used by the English, Swedish or French CD, by
OpenWDATA or by the network parts (1900 to 1902, 2900, 2901, 2565, 2566,
20100): checked against every resource of the three CDs' exports
(`openwgt --export`) by file name, and strings 2567 and 2568 are empty in
each CD's block 161. String 2567 shares block 161 with the game's own (2560 to 2564)
and the network parts' (2565, 2566): the engine merges the families'
blocks string by string and takes a string from them only where the
game's block has it empty. The screen is drawn empty with its grid: the
engine draws the traces and the range on it
(`openparts::scope::PLOT`, which `draw.py` must match). The pictures have
no words (A and B are the same in every language), so only the dialog,
the texts and the name have Swedish and French versions.

## Building it

```sh
tools/openparts/build.py      # the resources, from src/
nix-shell -p 'python3.withPackages(p: [p.pillow])' dejavu_fonts liberation_ttf
tools/openparts/draw.py --preview DIR   # the pictures, and a look at them
```

`src/` has the sources, in English: `tables.json` (knots, numbers),
`strings.json`, `dialogs/*.json` and `texts/WWGADGET/*.txt` in the
formats of OpenWDATA's (`tools/opendata`, whose `resfmt.py` writes them);
`sv/src` and `fr/src` have the Swedish and French files over them. The
overlays get only what differs from the English.

## Adding a part

A new open part takes the next numbers (gadget id 2904, sheets from
1906, string 2569 and tray index 69, raising `port_parts::TRAY_SLOTS`),
a class in `openparts::CLASSES`, a `TrayPart` in `openparts::TRAY` with
a frame of sheet 1904, and its place in a folder in `openparts::FAMILY`.
Check the numbers against the CDs' resources (`openwgt --export=DIR
CD`) first.
