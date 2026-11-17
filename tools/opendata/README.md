# OpenWDATA: the port's own data

OpenWDATA is everything Widget Workshop reads from its CD besides the
pictures (OpenWGFX, `tools/opengfx`) and the sounds (OpenWSFX,
`tools/opensfx`): the dialogs, the menu, the messages, the parts' texts
and tables, the palette, the help, the puzzles and the sample widgets.
With the three packs OpenWGT runs with no CD at all (`openwgt --open`,
and what it does when no CD is given or found), as OpenTTD runs on
OpenGFX, OpenSFX and OpenMSX.

The pack is built into the program from `crates/engine/opendata`
(`engine::opendata`):

```text
<MODULE>/<type>/<id>.bin   resources in the game's own formats, in the
                           overrides' layout (engine::assets)
WIDGET.PZL                 the puzzles (formats::pzl)
HELP.TXT                   the help (formats::helptext)
samples/<NAME>.WGT         the sample widgets
```

`openwgt --opendata=DIR` reads a folder in the same layout instead.

## Sources

| here | what | built by |
|---|---|---|
| `src/palette.json` | the 256 colours | `build.py` |
| `src/tables.json` | the parts' tables: knots (1014), points and rects where the code draws and clicks (1010–1013), each part's numbers (1021) | `build.py` |
| `src/dialogs/<id>.json` | every dialog's layout and labels | `build.py` |
| `src/menus.json`, `src/accelerators.json` | the menu bar and its shortcuts | `build.py` |
| `src/strings.json` | the messages, prompts and the tray's help lines | `build.py` |
| `src/combos.json` | the drop-down lists' choices | `build.py` |
| `src/texts/<MODULE>/<id>.txt` | the parts' descriptions (1000–2046) and construction tips (3000–4046), the About box's credits (10000) | `build.py` |
| `help/HELP.TXT` | the help, in OpenWGT's help format | copied by `build.py` |
| `samples/*.txt`, `puzzles/*.txt` | the sample widgets and the puzzles, as recipes | `cargo run -p engine --example recipe -- tools/opendata crates/engine/opendata` |

```sh
nix develop
python3 tools/opendata/build.py                  # src/ and help/ into the pack
cargo run --release -p engine --example recipe -- tools/opendata crates/engine/opendata
cargo run --release -p engine --example recipe -- --parts   # every part's knots
```

## Swedish

The Swedish pack, `crates/engine/opendata-sv`, is built from the same
sources with `sv/` over them: `sv/src/` holds the texts that differ
(strings, menus, the drop-down lists, every dialog, the parts' texts),
`sv/help/HELP.TXT` the help, `sv/samples/` and `sv/puzzles/` the
recipes with Swedish labels. The tables, the palette and the layout are
shared.

```sh
python3 tools/opendata/build.py --lang sv
cargo run --release -p engine --example recipe -- \
    --data crates/engine/opendata-sv tools/opendata/sv crates/engine/opendata-sv
```

The words follow one glossary, shared with OpenWGFX's Swedish pictures:
widget uppfinning, part del, puzzle pussel, parts tray verktygslåda,
worksheet arbetsyta, wire sladd, knot kontakt, GO/STOP START/STOPP,
ON/OFF PÅ/AV, YES/NO JA/NEJ, AND/OR/XOR/NOT OCH/ELLER/XELLER/INTE.

## French

The French pack, `crates/engine/opendata-fr`, is built the same way from
`fr/` (`fr/src/`, `fr/help/HELP.TXT`, `fr/samples/`, `fr/puzzles/`):

```sh
python3 tools/opendata/build.py --lang fr
cargo run --release -p engine --example recipe -- \
    --data crates/engine/opendata-fr tools/opendata/fr crates/engine/opendata-fr
```

Its words follow the French edition's (Eurêka, le Laboratoire du savant
fou) where it has them, and the port's own French (`engine::lang`); the
game says *tu*. The glossary, shared with OpenWGFX's French pictures:
widget Eurêka (un Eurêka, des Eurêkas), part pièce, puzzle problème,
parts tray fenêtre à outils, worksheet zone de travail, wire connexion,
joint jonction, knot borne, input/output entrée/sortie, control box boîte
de contrôle, settings réglages, YES/NO OUI/NON, TRUE/FALSE VRAI/FAUX,
AND/OR/XOR/NOT ET/OU/XOU/INV; GO/STOP and ON/OFF stay, as in the edition.
The parts reckon in the original's units, so the texts keep them in
French words: pieds (pi), livres, mi/h, milles, pouces. The help and the
worksheet's labels can't show œ, so the pack writes *oe* throughout
(Coeur), as the edition does.

The French edition's dialogs have layouts of their own; the pack keeps
OpenWDATA's (shared by every language), with labels that fit them.

`build.py --verify EXPORT` compares the built resources with a CD's
(`openwgt --export=EXPORT CD.iso`): every format but the texts must come
out byte for byte the same, which is what keeps saved widgets, the
parts' wiring and the dialogs working as the game expects.

## Rules

- **The numbers are the game's interface.** Knot places, the code's
  points and rectangles, dialog layouts and control ids, string and
  dialog ids are what the code and every saved widget rely on; they
  stay as they are.
- **Every text is the project's own.** Descriptions, tips, messages,
  labels, the help, the puzzles and the samples are written for OpenWGT,
  in its own words; no prose of the original's. Very short generic UI
  words (OK, Cancel, Open..., a part's plain name, units) aren't anyone's.
- **Some texts are code.** A dialog's `Pic1004` names a picture sheet, an
  edit field's text like `5#` is its input filter, a preset's leading
  number is the value the part reads, `%s`/`%d` and `&` mnemonics are the
  code's: keep them as they are.
- **Labels fit their controls.** A dialog control's rectangle is in
  dialog units (about 4 to a character across in MS Sans Serif 8).

`extract.py` read the first sources from the CD; it's not run again
(it would overwrite the project's texts with the CD's).
