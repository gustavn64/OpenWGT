# The open parts' own pictures and data

The open parts are the port's own parts for everyone, with nothing but
the game's wires in and out (`engine::openparts`): so far the
**Oscilloscope** (`Scop`, [docs/oscilloscope.md](../../docs/oscilloscope.md)).
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

None of these numbers is used by the English, Swedish or French CD, by
OpenWDATA or by the network parts (1900 to 1902, 2900, 2901, 2565, 2566,
20100). String 2567 shares block 161 with the game's own (2560 to 2564)
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

A new open part takes the next numbers (gadget id 2903, sheets from
1905, string 2568 and tray index 68, raising `port_parts::TRAY_SLOTS`),
a class in `openparts::CLASSES`, a `TrayPart` in `openparts::TRAY` with
a frame of sheet 1904, and its place in a folder in `openparts::FAMILY`.
Check the numbers against the CDs' resources (`openwgt --export=DIR
CD`) first.
