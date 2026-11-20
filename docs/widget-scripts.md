# Scripts in widgets

The debugging tools (Settings ▸ Debugging tools, with the enhancements
on) let the player name parts and run scripts in the console. A widget
keeps both: the names, and scripts that come with it. The original game
never sees any of it.

## In the file

Everything the port keeps about a widget beside the game's own content
is an **extension chunk**: a chunk whose tag starts with `OW`, which no
part of the game's or of the port's families has
(`crates/engine/src/port_parts/ext.rs`). A widget holds them in
`Document::names`, `Document::scripts` and, for any it doesn't know,
`Document::ext`, kept byte for byte so a widget from a later version
keeps what that one put in it. They are written after the game's own
chunks and hidden as the port's own parts are (`port_parts::wgt`): each
goes into a `back` chunk right after `fver`,

```text
00 00        a background, for whoever reads it as one
"OWGT"       the port's
01           this layout
u16          the chunk's place in the file as it was (big-endian)
tag, data    the chunk
```

so the original game, and any reader that doesn't know them, reads
backgrounds before the real one (which, read last, wins) and nothing
else. The original makes a part of every chunk it doesn't know and would
probably crash on one, so they are never written unhidden. A link to a
widget carries them the same way.

| Tag | What | Data (big-endian) |
|---|---|---|
| `OWnm` | the parts' names, once | `01`, then per part: u32 id, u16 length, the name (UTF-8) |
| `OWsc` | a script, one chunk each, in order | `01`, u8 length + the language (`JavaScript`, `commands`), u16 length + the name (UTF-8), then the text (UTF-8) to the chunk's end |

Only the names of parts the widget has are written. Undo leaves the
names and the scripts as they are: they aren't edits of the worksheet.

## In the console

The console lists the widget's scripts at its top, each with ▶ to run it
(■ to stop it while it runs) and ✕ to take it out of the widget. Below
them, while there is one, is the console's last script, with Keep to put
it in the widget (under a name of its own): a `.js`, `.mjs`, `.txt` or
`.cmd` file dropped onto the window while the tools are on (it isn't run
until ▶), or the last script of several lines typed (Ctrl+Enter or
Shift+Enter starts another line; Enter runs them all). With the tools
off, a dropped file opens as a widget, as before.

A script of the widget's always runs as one that came with a widget
(`Caps::EMBEDDED`): it can look, control the run and change settings and
names, but never open, load or start other widgets or puzzles. What is
typed or dropped into the console may do everything.

## Asking first

A widget opened from a file (File ▸ Open, given on the command line, a
file dropped onto the window, an imported stand-alone) or a link that
comes with scripts asks, in the game's message box:

> This widget comes with scripts. **[Show] [Run] [Don't run]**

- **Show** (Enter): the console opens with the scripts listed, none of
  them run. If the debugging tools were off they are turned on for the
  session, not in the settings.
- **Run**: they run, and the player's trust in them is remembered.
- **Don't run** (Esc): nothing runs.

Trust is kept by the SHA-256 of each script's text
(`engine::script::trust`, a small implementation of its own), on one
line of the settings file, the newest 256:

```text
trusted_scripts=3f2a…,9b07…
```

In the browser the settings file is in the browser's storage
(`/storage`, `docs/web.md`), so the trust stays there. A widget whose
scripts are all trusted doesn't ask: they run. A script whose text
changed is another script, and asks again.

With the enhancements off nothing asks, and only trusted scripts run. A
widget that a script opens (the console's, or `--script`'s) doesn't ask
and its scripts don't run: that script is in charge. A widget's scripts
run on with the debugging tools off (saying nothing anywhere) until
another widget comes, which stops them; the console's own stop with the
tools.
