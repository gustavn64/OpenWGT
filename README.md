# OpenWGT

An open engine for **Widget Workshop: A Mad Scientist's Laboratory**
(Maxis, 1995), in Rust, for Linux, Windows, macOS and WebAssembly: a
non-matching decompilation of the game, with its own graphics
(OpenWGFX), sounds (OpenWSFX) and data (OpenWDATA), as OpenTTD has
OpenGFX, OpenSFX and OpenMSX, so it runs without the original CD.

OpenWGT is a fork of Doridian's port,
[Doridian/GadgetGenerator](https://github.com/Doridian/GadgetGenerator),
under the same licence (AGPL-3.0-or-later). This fork adds the port's
own graphics, sounds and data ([below](#without-the-cd-openwgfx-openwsfx-openwdata)),
Swedish, touch screens, fixes, a Nix flake and tools that compare it with
the original running in an emulator (`tools/compare/`).

This repository contains **none of the game's pictures, sounds, texts
or code**. It runs without the game's CD on its own pictures, sounds and
data; with your own copy of the CD it can use the original's instead.

## Play

**In the browser:** <https://gustavn.se/openwgt>, nothing to install
(on a phone or tablet too).

**Download** it from the
[latest release](https://github.com/gustavn64/OpenWGT/releases/latest):

| System | File |
|---|---|
| Windows | `openwgt-windows-x86_64.exe` (`-arm64.exe` on Arm): just run it |
| macOS | `openwgt-macos.dmg`, for Apple silicon and Intel: drag `OpenWGT.app` to Applications; the first start needs a right click, Open (it's signed ad hoc only) |
| Linux, any | `openwgt-x86_64.AppImage` (`-aarch64` on Arm): make it executable and run it |
| Debian, Ubuntu | `openwgt-x86_64.deb`: `sudo apt install ./openwgt-x86_64.deb` |
| Fedora, openSUSE | `openwgt-x86_64.rpm`: `sudo dnf install ./openwgt-x86_64.rpm` |
| Linux, by hand | `openwgt-linux-x86_64.tar.gz`: the bare program |

The Linux builds want glibc 2.35 or newer (Ubuntu 22.04 and later).

**Host the browser version yourself** with Docker; the image serves the
site, with no game data in it:

```sh
docker run -p 8080:80 ghcr.io/gustavn64/openwgt
```

and open <http://localhost:8080/>. Without Docker, the release's
`openwgt-web.zip` is the same site for any static web server
([docs/web.md](docs/web.md)).

**With Nix:** `nix run github:gustavn64/OpenWGT`.

The AUR, winget, Homebrew and Flathub packages, and how a release reaches
them: [packaging/README.md](packaging/README.md).

## The original CD

Not needed: started without one, OpenWGT plays on its own pictures,
sounds and data. With your own copy it uses the original's instead.
Options ▸ Settings ▸ Change CD... picks a CD image (an `.iso`, or a folder
with the CD's files); in the browser, the start page does. The port
remembers it (`cd-path.txt` in `~/.local/share/openwgt`,
`%APPDATA%\OpenWGT`, `~/Library/Application Support/OpenWGT`); an `.iso`
or an `orig` folder in the working directory is found on its own. On
Linux the file dialog is the desktop's (xdg-desktop-portal, else zenity;
with neither, drop the CD image on the game's window).

It reads the **Mac/PC CD** (volume `WIDGETS`). The Internet Archive has
the `.iso`: [widget-workshop](https://archive.org/details/widget-workshop).
The Swedish edition, **Uppfinnarverkstan**, works too ([Languages](#languages)).

It also reads the much smaller **OS/2 Warp CD** (`WW1_01A.ISO`, game folder
`WIDGET/`): its modules are OS/2 executables whose dialogs, menu, strings
and pointers the port turns into the Win95 build's
([docs/os2.md](docs/os2.md)).

The OS/2 CD has no Windows fonts, so the port bundles free stand-ins
(`crates/engine/fonts`): Wine's MS Sans Serif and Small Fonts, and Widget
Sans, rendered from Liberation Sans, for the gadgets' Arial. When the
Mac/PC CD's own fonts (`WIN31/SVGA`) are there, they win.

## What's in it

The port opens straight into the editor (the original's intro and main
menu are left out): the parts tray, wiring, the worksheets, undo and the
parts' own settings dialogs; the logic, number, science, sound and time,
input and display parts; puzzle mode; widgets loaded and saved in the
original's `.WGT` format; recording and WAV import; and the help viewer
for `WIDGET.HLP`. It's a non-matching decompilation of the Win95 build's
`WIDGET.EXE`: the code behaves like the original, but doesn't compile to
the same bytes. The stack is winit, wgpu and cpal, with a resizable
window, fullscreen and UI scaling.

Options ▸ Settings, the port's own, changes the UI scale on the spot
(Automatic, or 1× to 4× in quarters), the language without a CD, and
switches the port's enhancements of the original on or off, all of them
at once. They're off until the player turns them on (`original=false` in
the settings file). The enhancements: the parts tray shows a whole folder
at once, its pages one under the other, without the page arrows; and the
toolbars, dialogs and message boxes are dragged whole, not as an outline.
**Full screen** switches the window to full screen and back, as F11 and
Alt+Enter do (on a phone, where there are no such keys).
Its **Change CD...** button, after "Save changes?", picks another CD
image and starts the game again on it (in the browser, the page's start
form comes back).

On a touch screen a tap is a click, a long press a double click, and a
tap on a text field opens the device's own text box (in the browser).

Widgets travel as links too ([docs/web.md](docs/web.md#widget-links)):
File ▸ Copy Link to Widget puts one on the clipboard, which opens the
widget in the browser version for anyone with the CD, and File ▸ Open Widget Link
(Ctrl+V) opens the one on the clipboard, natively too.

## Building it

```sh
cargo run --release -p openwgt                       # the .iso here, else orig/, else no CD
cargo run --release -p openwgt -- CD.iso MY.WGT      # a CD and a widget to open
cargo run --release -p openwgt -- CD.iso 'https://gustavn.se/openwgt/play/#w=…'   # a widget link
cargo run --release -p openwgt -- --help
```

On Linux the sound needs ALSA's headers to build (`libasound2-dev`).
With Nix: `nix develop` gives a shell with the Rust tools, the libraries
the window and the sound load, and the Python tools' packages; `nix run
. -- CD.iso` builds and starts it (`flake.nix`).

Options: `--open`, `--opengfx`, `--opensfx` and `--opendata`
([below](#without-the-cd-openwgfx-openwsfx-openwdata)), `--lang=en|sv`,
`--no-audio`, `--window-size=WxH`, `--fullscreen`, `--windowed`,
`--ui-scale=N|auto` (remembered in the settings file), `--overrides=DIR`
(resources that replace or add to the CD's) and `--export=DIR` (writes
every resource there in that layout and quits).

The browser version: `tools/build-web.sh`, [docs/web.md](docs/web.md).
Every push to master builds every package
(`.github/workflows/build.yml`); a `v*` tag publishes them on its release
and the container image on ghcr.io ([packaging/README.md](packaging/README.md)).
The icon is the port's own (`tools/make-icon.py`).

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

## Languages

OpenWGT speaks English and Swedish. With a CD it plays in the CD's
language (the Swedish edition, Uppfinnarverkstan, as well: its puzzle
file is read too); the port's own words, what the original took from
Windows and what the port adds, follow it (`engine::lang`). Without a CD
the packs come in the language chosen: `--lang=sv` (`?lang=sv` in the
browser, whose start page has a list for it), else the system's.
Swedish OpenWDATA is `tools/opendata/sv` (`crates/engine/opendata-sv`),
Swedish OpenWGFX the pictures with words, `crates/engine/opengfx-sv`,
over the English ones.

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
