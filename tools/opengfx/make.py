#!/usr/bin/env python3
"""Draws OpenWGFX, the port's own graphics, into crates/engine/opengfx
(engine::opengfx): a PNG per frame of every sheet it has a drawing for
(tools/opengfx/sheets/*.py), the editor's pointers, and sheets.txt with
every frame's rectangle.

It needs the sizes and places the game uses, from an export of the CD:

    openwgt --export=EXPORT CD.iso
    tools/opengfx/make.py EXPORT [SHEET...] [--preview DIR] [--compare] [--lang sv]

Only the given sheets are drawn again when some are named. --lang sv draws
the pictures' words in Swedish (wgfx.WORDS) into the overlay pack
crates/engine/opengfx-sv: only the sheets that come out different from
the English pack's, so the engine takes the rest from it. --preview also
writes a contact sheet per drawn sheet to DIR (with --compare, the CD's
frames beside the new ones, to check sizes and the knots' places).

The tray's icons and the puzzles' pictures are made of the parts as the
engine draws them with the pack, so they come last, in a second step:

    cargo run --release -p engine --example render_parts -- EXPORT PARTS
    tools/opengfx/make.py EXPORT 898 899 2001 ... --parts PARTS

Needs
Pillow and the DejaVu and Liberation fonts (fc-match finds them):
`nix-shell -p 'python3.withPackages(p: [p.pillow])' dejavu_fonts liberation_ttf`.
"""
from __future__ import annotations

import importlib
import os
import pkgutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import wgfx  # noqa: E402
from PIL import Image, ImageDraw  # noqa: E402

OUT = HERE.parent.parent / "crates/engine/opengfx"


def load_sheets():
    import sheets

    for m in pkgutil.iter_modules(sheets.__path__):
        importlib.import_module(f"sheets.{m.name}")


def write_rects(geo: wgfx.Geometry, overlay=False):
    lines = ["# OpenWGFX: every frame's rectangle, as the game's sheets have them.",
             "# sheet frame top left bottom right"]
    if overlay:
        lines[0] = "# OpenWGFX, the sheets with words in another language: their frames' rectangles."
    for sid in geo.sheet_ids():
        if overlay and not (OUT / "gadgets" / str(sid)).is_dir():
            continue
        for n, (t, l, b, r) in enumerate(geo.frames(sid), 1):
            lines.append(f"{sid} {n} {t} {l} {b} {r}")
    (OUT / "sheets.txt").write_text("\n".join(lines) + "\n")


def draw_sheet(geo, q, sid):
    ctx = wgfx.SheetContext(geo, sid)
    frames = wgfx.REGISTRY[sid](ctx)
    if len(frames) != ctx.count:
        raise SystemExit(f"sheet {sid}: {len(frames)} frames drawn, the game has {ctx.count}")
    out = []
    for n, c in enumerate(frames, 1):
        if (c.w, c.h) != ctx.size(n):
            raise SystemExit(f"sheet {sid} frame {n}: {c.w}×{c.h}, the game's is {ctx.size(n)[0]}×{ctx.size(n)[1]}")
        out.append(c.finish(q))
    d = OUT / "gadgets" / str(sid)
    if d.is_dir():
        for old in d.glob("*.png"):
            old.unlink()
    if OUT != wgfx.PACK and all(same(img, wgfx.PACK / "gadgets" / str(sid) / f"{n}.png")
                                for n, img in enumerate(out, 1)):
        # No words in it: the English pack's sheet does.
        if d.is_dir():
            d.rmdir()
        return out
    d.mkdir(parents=True, exist_ok=True)
    for n, img in enumerate(out, 1):
        img.save(d / f"{n}.png", optimize=True)
    return out


def same(img, path):
    """Whether `img` is the picture in the PNG at `path`."""
    if not path.exists():
        return False
    old = Image.open(path)
    return old.size == img.size and old.convert("RGBA").tobytes() == img.convert("RGBA").tobytes()


def draw_cursors(geo, q):
    from sheets import cursors

    d = OUT / "cursors"
    d.mkdir(parents=True, exist_ok=True)
    lines = ["# OpenWGFX: the editor's pointers (cursor groups 5007 to 5013).", "# pointer hot-x hot-y"]
    for i, (fn, hot) in sorted(cursors.CURSORS.items()):
        fn().finish(q).save(d / f"{i}.png", optimize=True)
        lines.append(f"{i} {hot[0]} {hot[1]}")
    (OUT / "cursors.txt").write_text("\n".join(lines) + "\n")


def contact(frames, extra=None, z=2):
    """Frames side by side (above them the CD's, with --compare)."""
    rows = [frames] + ([extra] if extra else [])
    W = max(sum(f.width + 3 for f in r) for r in rows) + 3
    H = sum(max(f.height for f in r) + 3 for r in rows) + 3
    img = Image.new("RGBA", (W, H), (64, 64, 72, 255))
    y = 3
    for r in rows:
        x = 3
        for f in r:
            img.alpha_composite(f.convert("RGBA"), (x, y))
            x += f.width + 3
        y += max(f.height for f in r) + 3
    return img.resize((W * z, H * z), Image.NEAREST)


def cd_frames(geo, sid, palette):
    """The CD's frames, for --compare only (they never go into the pack)."""
    sys.path.insert(0, str(HERE))
    from hr import decode_sheet

    data = geo._res(1004, sid)
    out = []
    for (t, l, b, r), px in decode_sheet(data):
        w, h = r - l, b - t
        im = Image.new("RGBA", (max(1, w), max(1, h)), (0, 0, 0, 0))
        if px:
            im.putdata([palette[v] + (255,) if v is not None else (0, 0, 0, 0) for v in px])
        out.append(im)
    return out


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    preview = None
    if "--preview" in argv:
        preview = Path(argv[argv.index("--preview") + 1])
        args.remove(str(preview))
    compare = "--compare" in argv
    if "--parts" in argv:
        parts = argv[argv.index("--parts") + 1]
        args.remove(parts)
        os.environ["OPENGFX_PARTS"] = parts
    global OUT
    if "--lang" in argv:
        lang = argv[argv.index("--lang") + 1]
        args.remove(lang)
        if lang != "en" and lang not in wgfx.WORDS:
            raise SystemExit(f"no words for --lang {lang} (wgfx.WORDS)")
        wgfx.LANG = lang
        OUT = wgfx.pack_dir(lang)
    if not args:
        raise SystemExit(__doc__)
    geo = wgfx.Geometry(Path(args[0]))
    q = wgfx.Quantizer(geo.palette)
    load_sheets()
    wanted = [int(a) for a in args[1:] if a.isdigit()] if args[1:] else sorted(wgfx.REGISTRY)
    # The sheets made of the others' pictures last.
    wanted.sort(key=lambda s: s in wgfx.LATE)
    OUT.mkdir(parents=True, exist_ok=True)
    overlay = OUT != wgfx.PACK
    if not overlay and (not args[1:] or "cursors" in args):
        draw_cursors(geo, q)
    for sid in wanted:
        if sid not in wgfx.REGISTRY:
            print(f"sheet {sid}: no drawing")
            continue
        try:
            frames = draw_sheet(geo, q, sid)
        except wgfx.NotReady as e:
            print(f"sheet {sid}: not drawn, {e}")
            continue
        if preview:
            preview.mkdir(parents=True, exist_ok=True)
            extra = cd_frames(geo, sid, geo.palette) if compare else None
            contact(frames, extra).save(preview / f"{sid}.png")
        print(f"sheet {sid}: {len(frames)} frames")
    write_rects(geo, overlay)
    if overlay:
        kept = sorted(int(d.name) for d in (OUT / "gadgets").iterdir() if d.is_dir())
        print(f"{wgfx.LANG}: {len(kept)} sheets with words of their own: {' '.join(map(str, kept))}")
    missing = [s for s in geo.sheet_ids() if s not in wgfx.REGISTRY]
    print(f"{len(wgfx.REGISTRY)} sheets drawn; not yet: {' '.join(map(str, missing)) or 'none'}")


if __name__ == "__main__":
    main(sys.argv[1:])
