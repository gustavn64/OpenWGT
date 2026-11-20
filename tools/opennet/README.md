# The network parts' own pictures and data

The Sender (`NSnd`) and the Receiver (`NRcv`) are the port's own parts
(`engine::net`): a Sender sends what is wired into it to the Receivers
with the same channel name in other players' widgets, through the relay
(`relay/`). Neither the CDs nor OpenWDATA have anything for them, so they
bring their own pictures, dialogs, texts and tables, built into the
program from `crates/engine/opennet` (and `opennet-sv`, `opennet-fr` for
their words in Swedish and French). The engine puts them behind
everything else (`engine::net::pack`): they fill only what is missing,
with a CD as without one.

```text
sheets.txt                   <sheet> <frame> <top> <left> <bottom> <right>
gadgets/<sheet>/<frame>.png  the pictures (draw.py)
<MODULE>/<type>/<id>.bin     the resources, in the game's formats (build.py)
words.txt                    the random channel names' words
```

| What | Where |
|---|---|
| The Sender, the Receiver | sheets 1900, 1901, frames 1 to 4: the lamp grey (no link), red (nobody there), amber (there, stopped), green (running); 88 × 50 |
| The tray's icons | sheet 1902, frames 1 (Sender) and 2 (Receiver) |
| Knots | `WWGADGET` 1014/1900 (an input at 4, 36), 1014/1901 (an output at 83, 36) |
| Numbers | `WWGADGET` 1021/2900, 2901 (dialog, Doc texts) |
| Doc texts | `WWGADGET` 1020/2900, 2901 and the tips 4900, 4901 |
| Settings dialogs | `WIDGET` 5/2900, 2901 |
| Strings | 2565, 2566 (the tray's names, block 161) and 20100 (BUSY, block 1257) |

None of these numbers is used by the English, Swedish or French CD.
Strings 2565 and 2566 share block 161 with the game's own (2560 to 2564),
so the engine takes a string from here only where the game's block has it
empty. The displays are drawn empty: the engine writes the channel name in
them (`net::parts::SENDER_LCD`, `RECEIVER_LCD`, which `draw.py` must match).

## Building it

```sh
tools/opennet/build.py      # the resources and words.txt, from src/
nix-shell -p 'python3.withPackages(p: [p.pillow])' dejavu_fonts liberation_ttf
tools/opennet/draw.py --preview /tmp/pv   # the pictures, and a look at them
```

`src/` has the sources, in English: `tables.json` (knots, numbers),
`strings.json`, `dialogs/*.json` and `texts/WWGADGET/*.txt` in the
formats of OpenWDATA's (`tools/opendata`, whose `resfmt.py` writes them);
`sv/src` and `fr/src` have the Swedish and French files over them. The
overlays get only what differs from the English. `draw.py` has the
pictures' words for each language.

## The words

Random channel names are three words of `src/words.txt` joined with
hyphens (`cat-sun-oak`): 2048 short English words, the same in every
language, so that 2048³ (about 8.6 billion) names make two players
unlikely to meet by chance. They are to be read out over the phone and
typed by children: 3 to 5 letters, common, spelt as they sound, none that
sounds like another or like the start of another, nothing rude.
`words.py` picks them (SCOWL's word lists, `wordfreq` for how common, the
CMU Pronouncing Dictionary for the sounds, a block list and a reading) and
checks a list:

```sh
nix-shell -p 'python3.withPackages(p: [p.wordfreq p.cmudict])' scowl
tools/opennet/words.py $SCOWL/share/dict > tools/opennet/src/words.txt
tools/opennet/words.py --check tools/opennet/src/words.txt
```
