# OpenWGFX: the port's own graphics

OpenWGFX is a set of new pictures for Widget Workshop's picture sheets and
pointers, made for the port, that stands in for the CD's when the port is
started with `--opengfx` (`engine::opengfx`). It is to the original's
pictures what OpenGFX is to Transport Tycoon Deluxe's: every frame has the
original's size, offset and meaning, so the game's code and tables work
unchanged, but the pictures are drawn anew. Its sister sets are
OpenWSFX for the sounds (`tools/opensfx`, `--opensfx`) and OpenWDATA for
the texts, dialogs, tables, puzzles and help (`tools/opendata`,
`--opendata`); with the three the port runs with no CD at all.

The pack lives in `crates/engine/opengfx` and is built into the program:

```text
sheets.txt                   <sheet> <frame> <top> <left> <bottom> <right>
gadgets/<sheet>/<frame>.png  frame <frame> (from 1) of sheet <sheet>
cursors.txt                  <pointer> <hot x> <hot y>
cursors/<pointer>.png        the editor's pointer <pointer> (0 to 6)
```

A sheet is used only when every one of its frames has a picture; the
others are still the CD's. `openwgt --opengfx=DIR` reads a folder in the
same layout instead of the built-in copy, to try pictures without
building.

## Drawing it

The PNGs are this folder's output: `make.py` draws them with Pillow, from
one Python function per sheet (`sheets/*.py`), with the kit in `wgfx.py`.
Pictures can be edited by hand too (any colour; the port takes it to the
nearest of the palette), but then take that sheet's function out, or
`make.py` draws over them.

```sh
nix-shell -p 'python3.withPackages(p: [p.pillow])' dejavu_fonts liberation_ttf
openwgt --export=EXPORT CD.iso          # the sizes and places, from the CD
tools/opengfx/make.py EXPORT           # every sheet and the pointers
tools/opengfx/make.py EXPORT 1029 1030 --preview /tmp/pv --compare
tools/opengfx/make.py EXPORT --lang sv  # the Swedish words' sheets (below)
```

`make.py` reads the export only for numbers: the frames' rectangles, the
knots' places (type 1014), the tables of points and rectangles where the
code draws digits, needles and buttons (1010, 1012). `--compare` puts the
CD's frames under the new ones in the preview, to check that knots,
windows and buttons are where the game expects them; the CD's pixels never
go into the pack.

## Rules

- **Draw, don't trace.** No pixels of the CD's pictures, no tracing,
  recolouring or filtering of them. Look at the original to learn what a
  frame shows and where its parts are; then draw it your own way.
- **Keep the geometry.** Same frame sizes (`make.py` checks), knots
  exactly at the table's points, digit windows, needles, buttons and
  sliders where the code draws or hit-tests them. The body should cover
  about the same area, since clicks are tested against it.
- **Keep the meaning.** An animation's frames in the same order and
  phases; a lamp's off and on frames; states recognisable.
- **No logos, trademarks or likenesses.** The original's credits
  pictures (the publisher's and the developer's logos, the photo) become
  the port's own.
- Text on parts: DejaVu Sans / Liberation Sans (free), in English as the
  game is; every word through `wgfx.tr()`, so that other languages can
  have their own (below).

## Other languages: the Swedish overlay

The pictures with words (GO/STOP, ON/OFF, the logic gates, the signs,
the Zodiac's names, the Fortune Teller's answers, ...) have Swedish
versions in an overlay pack, `crates/engine/opengfx-sv`: the same layout,
but only the sheets whose words differ, and `sheets.txt` lines for those;
the engine takes every other sheet from the English pack. The words are
`wgfx.WORDS["sv"]` (English to Swedish, with `\n` where a sign breaks its
lines); `--lang sv` draws everything with them and keeps the sheets that
come out different from the English pack's:

```sh
tools/opengfx/make.py EXPORT --lang sv
# the tray's icons and the puzzles' pictures, from the Swedish parts:
cargo run --release -p engine --example render_parts -- EXPORT PARTS-SV --overlay crates/engine/opengfx-sv --data crates/engine/opendata-sv --lang sv
tools/opengfx/make.py EXPORT --lang sv --parts PARTS-SV
```

Draw the English pack first (the overlay is what differs from it). A new
language is a new `WORDS` entry and `--lang` code; words that don't fit
are shrunk by `text_fit`, and the pixel fonts (`TINY_FONT` in
`sheets/chrome.py`, `GLYPHS` in `sheets/_numbers_kit.py`, `_G` in
`sheets/_science_kit.py`) need a glyph for every letter they draw.

### French

`crates/engine/opengfx-fr`, made the same way with `--lang fr` (and
`render_parts ... --overlay crates/engine/opengfx-fr --data
crates/engine/opendata-fr --lang fr`). It follows the French edition
(Eurêka, le Laboratoire du savant fou) where that put its pictures in
French, in pictures of its own:

| English | French | the edition |
|---|---|---|
| AND, OR, XOR, NOT | ET, OU, XOU, INV | the same |
| HEADS, TAILS | FACE, PILE | the same |
| the Zodiac's names, the Fortune Teller's answers, the signs | in French | in French |
| AVERAGE, TOTAL, MAXIMUM, MINIMUM | MOYENNE, TOTAL, MAXIMUM, MINIMUM | MOYENNE, TOTAUX, ... |
| RESET (the Strip Chart) | ZÉRO | REMIS A ZERO |
| ENTER (the keypad) | ENTRÉE | |
| Ft (the Converter) | pi | Ft |
| IN, OUT (the settings' arrows) | E, S (entrée, sortie) | no words |
| for Widget Workshop | pour Widget Workshop | |
| GO/STOP, ON/OFF, AM/PM, the Stopwatch's START/STOP | kept | kept |

The texts' glossary is OpenWDATA's (`tools/opendata/README.md`).

## Style

Clean, bright cartoon lab equipment, a little toy-like: flat-shaded
plates with a 1-pixel dark outline (`INK`), a light top-left edge and a
dark bottom-right edge (`Canvas.panel`), soft gradients, screws in the
corners. Inputs are brass sockets and outputs orange lamps
(`Canvas.knot`), each on a short steel stalk into the body. Displays are
sunk glass (`Canvas.inset`) with crisp pixel-exact segments
(`Canvas.led_digit`). Small things (under ~12 pixels) are better drawn on
the pixel layer (`pixel`, `prect`, `pline`) than smooth.
