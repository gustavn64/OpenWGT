# OpenWGT

An open engine for **Widget Workshop: A Mad Scientist's Laboratory**
(Maxis, 1995), in Rust, for Linux, Windows, macOS and WebAssembly: a
non-matching decompilation of the game, with its own graphics
(OpenWGFX), sounds (OpenWSFX) and data (OpenWDATA), as OpenTTD has
OpenGFX, OpenSFX and OpenMSX, so it runs without the original CD.

**Play it in your browser:** <https://gustavn64.github.io/OpenWGT/>
([docs/web.md](docs/web.md)), or download it for Linux, Windows or macOS
from the [releases](https://github.com/gustavn64/OpenWGT/releases)
([Packages](#packages)).

OpenWGT is a fork of Doridian's port,
[Doridian/WidgetWorkshop](https://github.com/Doridian/WidgetWorkshop),
under the same licence (AGPL-3.0-or-later). This fork adds the port's
own graphics, sounds and data ([below](#without-the-cd-openwgfx-openwsfx-openwdata)),
fixes, a Nix flake and tools that compare it with the original running
in an emulator (`tools/compare/`).

The port opens straight into the editor (the original's intro and main
menu are left out): the parts tray, wiring, the worksheets, undo and the
parts' own settings dialogs; the logic, number, science, sound and time,
input and display parts; puzzle mode; widgets loaded and saved in the
original's `.WGT` format; recording and WAV import; and the help viewer
for `WIDGET.HLP`. It's a non-matching decompilation of the Win95 build's
`WIDGET.EXE`: the code behaves like the original, but doesn't compile to
the same bytes. The stack is winit, wgpu and cpal, with a resizable
window, fullscreen and UI scaling.

This repository contains **none of the game's pictures, sounds, texts
or code**. It runs without the game's CD on its own pictures, sounds and
data; with your own copy of the CD it can use the original's instead.

## The original CD

Not needed, but the port can use it. It reads the **Mac/PC CD** (volume `WIDGETS`), as an `.iso` or as
a folder with its files. The Internet
Archive has the `.iso`:
[widget-workshop](https://archive.org/details/widget-workshop).

It also reads the much smaller **OS/2 Warp CD** (`WW1_01A.ISO`, game folder
`WIDGET/`): its modules are OS/2 executables whose dialogs, menu, strings
and pointers the port turns into the Win95 build's
([docs/os2.md](docs/os2.md)).

The OS/2 CD has no Windows fonts, so the port bundles free stand-ins
(`crates/engine/fonts`): Wine's MS Sans Serif and Small Fonts, and Widget
Sans, rendered from Liberation Sans, for the gadgets' Arial. When the
Mac/PC CD's own fonts (`WIN31/SVGA`) are there, they win.

## Packages

Every push builds the port as one file per platform
(`.github/workflows/build.yml`); a `v*` tag publishes them on its release:

- `openwgt-x86_64.AppImage`, `openwgt-aarch64.AppImage` (built on
  Ubuntu 22.04: glibc 2.35 or newer)
- `openwgt-windows-x86_64.exe` (the C runtime linked in)
- `openwgt-macos.dmg`: `OpenWGT.app`, for Apple silicon and
  Intel (signed ad hoc only: the first start needs a right click, Open)
- `openwgt-web.zip`: the browser build

None of them carries game data. Started without a CD (from its icon), it
uses an `.iso` or `orig` in the working directory, else the CD picked
before; with none, it runs on its own pictures, sounds and data. Settings
▸ Change CD... picks a CD image with the system's file dialog (on Linux
the desktop's, xdg-desktop-portal, else zenity; with neither, the CD image
is dropped on the game's window) and keeps its path in the data directory
(`cd-path.txt` in `~/.local/share/openwgt`, `%APPDATA%\OpenWGT`,
`~/Library/Application Support/OpenWGT`). The icon is the port's own
(`tools/make-icon.py`).

## Running

```sh
cargo run --release -p openwgt                       # the .iso here, else orig/, else no CD
cargo run --release -p openwgt -- CD.iso MY.WGT      # a CD and a widget to open
cargo run --release -p openwgt -- CD.iso 'https://…/widget.html#w=…'   # a widget link
cargo run --release -p openwgt -- --help
```

With Nix: `nix develop` gives a shell with the Rust tools, the libraries
the window and the sound load, and the Python tools' packages; `nix run
. -- CD.iso` builds and starts it (`flake.nix`).

Options: `--open`, `--opengfx`, `--opensfx` and `--opendata`
([below](#without-the-cd-openwgfx-openwsfx-openwdata)), `--no-audio`,
`--window-size=WxH`, `--fullscreen`, `--windowed`,
`--ui-scale=N|auto` (remembered in the settings file), `--overrides=DIR`
(resources that replace or add to the CD's) and `--export=DIR` (writes
every resource there in that layout and quits). On Linux the sound needs
ALSA's headers to build (`libasound2-dev`).

Options ▸ Settings, the port's own, changes the UI scale on the spot
(Automatic, or 1× to 4× in quarters) and switches the port's
enhancements of the original on or off, all of them at once
(`original=true` in the settings file turns them off). The
enhancements: the parts tray shows a whole folder at once, its pages one
under the other, without the page arrows; and the toolbars, dialogs and
message boxes are dragged whole, not as an outline.
Its **Change CD...** button, after "Save changes?", picks another CD
image and starts the game again on it (in the browser, the page's start
form comes back).

Widgets travel as links too ([docs/web.md](docs/web.md#widget-links)):
File ▸ Copy Link to Widget puts one on the clipboard, which opens the
widget in the browser build for anyone with the CD, and File ▸ Open
Widget Link (Ctrl+V) opens the one on the clipboard, natively too.

The browser build: `tools/build-web.sh`, [docs/web.md](docs/web.md).

## Without the CD: OpenWGFX, OpenWSFX, OpenWDATA

OpenWGT has its own pictures (OpenWGFX), sounds (OpenWSFX) and data
(OpenWDATA: the dialogs, menus, messages, the parts' texts and tables,
the help, puzzles and sample widgets), built into the program, as OpenTTD
has OpenGFX, OpenSFX and OpenMSX. Started with no CD given or found (or
with `--open`) it runs on them alone; with a CD, `--opengfx`, `--opensfx`
and `--opendata` use them instead of the CD's parts one at a time (in the
browser `?open`, `?opengfx`, ...). Every part, toolbar, pointer, dialog
and sound keeps the original's size, number and job (the knots where the
wires plug in, the places the digits are drawn, the dialogs' controls),
so widgets made with the original open and work the same; the pictures
are drawn, the sounds synthesized and the texts, help, puzzles and
samples written anew. The game has no music besides its intro, which the
port leaves out, so there is no music set.

```sh
cargo run --release -p openwgt                         # no CD here: the packs alone
cargo run --release -p openwgt -- --open               # the packs even with a CD here
cargo run --release -p openwgt -- --opengfx --opensfx CD.iso   # the CD's data, own pictures and sounds
cargo run --release -p openwgt -- --opengfx=DIR CD.iso # a pack from a folder
```

The packs are in `crates/engine/opengfx`, `crates/engine/opensfx` and
`crates/engine/opendata`; how they're made, their layout and rules:
[tools/opengfx/README.md](tools/opengfx/README.md),
[tools/opensfx/README.md](tools/opensfx/README.md),
[tools/opendata/README.md](tools/opendata/README.md).

## Tools

- `tools/gh`, `tools/ghidra/`: headless Ghidra setup and CLI for the decompilation.
- `tools/build-web.sh`: builds the browser version into `web/`.
- `tools/make-icon.py`: draws the port's icon.
- `tools/opengfx/`: draws OpenWGFX, the port's own graphics.
- `tools/opensfx/`: synthesizes OpenWSFX, the port's own sounds.
- `tools/opendata/`: builds OpenWDATA, the port's own data.
- `tools/compare/`: plays the same scripts into the original (in a Windows
  95 virtual machine) and the port, and compares the screens.

## Docs

- [docs/web.md](docs/web.md): the browser build
- [docs/gadget-model.md](docs/gadget-model.md): the gadget model
- [docs/editor-layout.md](docs/editor-layout.md): the editor: main window, toolbars, parts tray, worksheet frame
- [docs/editor-interaction.md](docs/editor-interaction.md): editor interaction: the worksheet
- [docs/dialogs.md](docs/dialogs.md): the parts' settings dialogs
- [docs/puzzles.md](docs/puzzles.md): puzzles
- [docs/help.md](docs/help.md): `WIDGET.HLP` and the Help menu
- [docs/os2.md](docs/os2.md): the OS/2 Warp CD and how its modules differ
- [docs/gadgets/logic.md](docs/gadgets/logic.md): the logic and number parts
- [docs/gadgets/science.md](docs/gadgets/science.md): the science lab parts
- [docs/gadgets/sound-time.md](docs/gadgets/sound-time.md): time and sound parts
- [docs/gadgets/io.md](docs/gadgets/io.md): input and display parts

## Legal

The port's code is under the AGPL-3.0-or-later ([LICENSE](LICENSE)):
Doridian's original port and this fork's changes alike; the fork keeps the
original authors' notices. OpenWGFX, OpenWSFX and OpenWDATA, and the
tools that make them, are made for this project and under the same
licence. The bundled fonts have their own
([crates/engine/fonts](crates/engine/fonts/README.md)).

None of the game's pictures, sounds, texts or code is included. OpenWDATA
keeps the numbers the code and saved widgets rely on (where the knots
are, the dialogs' layouts and ids, the palette), so that widgets made
with the original work the same. Widget Workshop is © Maxis and a
trademark of its owners; OpenWGT is an unofficial project, not
affiliated with them.
