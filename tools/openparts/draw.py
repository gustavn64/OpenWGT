#!/usr/bin/env python3
"""Draws the open parts' pictures (engine::openparts) into
crates/engine/openparts, with OpenWGFX's kit (tools/opengfx/wgfx.py):

    sheet 1903  the Oscilloscope, frames 1 (running or not) and 2 (frozen:
                its HOLD lamp lit)
    sheet 1904  the tray's icons: 1 the Oscilloscope

and sheets.txt with the frames' rectangles (the parts are new: there's no
CD's geometry to take them from). The screen is drawn empty, with its
grid: the game draws the traces and the range on it
(openparts::scope::PLOT, which must stay where PLOT below is), and the
knots are where the knot table says (src/tables.json).

There are no words on the pictures (A and B are the inputs' names in
every language), so there are no Swedish or French ones.

    draw.py [--preview DIR]

--preview writes every frame, big, to DIR/openparts.png, with traces on
the screen about as the game draws them, to look at.

Needs Pillow and the DejaVu fonts:
`nix-shell -p 'python3.withPackages(p: [p.pillow])' dejavu_fonts liberation_ttf`.
"""
from __future__ import annotations

import json
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(ROOT / "tools/opengfx"))

import wgfx  # noqa: E402
from PIL import Image, ImageFilter  # noqa: E402
from wgfx import INK, STEEL, STEEL_D, STEEL_L, Canvas, darker, lighter  # noqa: E402

OUT = ROOT / "crates/engine/openparts"

W, H = 148, 100
TRAY_W, TRAY_H = 50, 34
SCOPE, TRAY = 1903, 1904

BODY = (84, 88, 96)           # the instrument's grey
SCREEN = (16, 24, 22)         # the tube, nearly black
GRID = (23, 79, 87)
GRID_MID = (52, 110, 72)
# The traces' colours, as openparts::scope has them.
TRACE_A = (255, 230, 40)
TRACE_B = (60, 220, 255)
HOLD_ON, HOLD_OFF = (255, 70, 40), (80, 22, 18)

# The plot (x, y, w, h): 100 columns of 64 rows, as openparts::scope::PLOT.
PLOT = (25, 9, 100, 64)
COL_DIV, ROW_DIV = 10, 8


def knots():
    t = json.loads((HERE / "src/tables.json").read_text())["knots"][str(SCOPE)]
    out = [(h, v, k == 1) for v, h, k in t]
    assert [o for _, _, o in out] == [False, False, False, True], "A, B, freeze in; the output"
    return out


def letter(c, x, y, s, col):
    c.text(x, y, s, 9.5, col, "sans", stroke=0.8, stroke_fill=INK)


def screen(c):
    """The tube in its bezel, with the grid (pixel-exact, where the game
    draws)."""
    px, py, pw, ph = PLOT
    c.rect(px - 5, py - 5, px + pw + 5, py + ph + 5, fill=INK, radius=3.5)
    c.gradient(px - 4, py - 4, px + pw + 4, py + ph + 4, [STEEL_L, STEEL, STEEL_D], radius=3)
    c.rect(px - 2.5, py - 2.5, px + pw + 2.5, py + ph + 2.5, fill=INK, radius=2)
    c.prect(px - 1, py - 1, px + pw + 1, py + ph + 1, darker(SCREEN, 0.4))
    c.prect(px, py, px + pw, py + ph, SCREEN)
    for i in range(0, pw + 1, COL_DIV):
        x = min(px + i, px + pw - 1)
        mid = i == pw // 2
        for y in range(py, py + ph):
            if mid or (y - py) % 2 == 0:
                c.pixel(x, y, GRID_MID if mid else GRID)
    for j in range(0, ph + 1, ROW_DIV):
        y = min(py + ph - 1 - j, py + ph - 1)
        y = max(y, py)
        mid = j == ph // 2
        for x in range(px, px + pw):
            if mid or (x - px) % 2 == 0:
                c.pixel(x, y, GRID_MID if mid else GRID)
    # Ticks along the middle lines.
    cx, cy = px + pw // 2, py + ph - 1 - ph // 2
    for x in range(px, px + pw, 2):
        c.pixel(x, cy - 1, GRID_MID)
        c.pixel(x, cy + 1, GRID_MID)
    for y in range(py, py + ph, 2):
        c.pixel(cx - 1, y, GRID_MID)
        c.pixel(cx + 1, y, GRID_MID)


def knob(c, x, y, r, mark):
    c.circle(x + 0.6, y + 0.8, r, fill=darker(BODY, 0.6))
    c.circle(x, y, r, fill=INK)
    c.radial(x, y, r - 0.7, STEEL_L, STEEL_D, -0.5, -0.5)
    a = math.radians(mark)
    c.line([(x, y), (x + math.cos(a) * (r - 1.2), y + math.sin(a) * (r - 1.2))], INK, 0.9)


def scope(hold):
    c = Canvas(W, H)
    ks = knots()
    for x, y, out in ks:
        c.knot(x, y, out, "r" if out else "l", reach=6)
    c.panel(9, 1, W - 9, H - 1, BODY, radius=3.5, bevel=1.5)
    for sx, sy in ((13.5, 5.5), (W - 13.5, 5.5), (13.5, H - 5.5), (W - 13.5, H - 5.5)):
        c.screw(sx, sy, 1.3)
    screen(c)
    (ax, ay, _), (bx, by, _), (fx, fy, _), (ox, oy, _) = ks
    letter(c, 15, ay + 0.5, "A", TRACE_A)
    letter(c, 15, by + 0.5, "B", TRACE_B)
    letter(c, W - 15, oy + 0.5, "A", TRACE_A)
    # The freeze input: a pause sign and the HOLD lamp.
    for dx in (0, 3.2):
        c.rect(13.5 + dx, fy - 3, 15.5 + dx, fy + 4, fill=lighter(BODY, 0.75), outline=INK, width=0.6)
    lx = 27.5
    c.circle(lx, fy + 0.5, 3.0, fill=INK)
    if hold:
        c.radial(lx, fy + 0.5, 2.3, lighter(HOLD_ON, 0.6), darker(HOLD_ON, 0.2), -0.5, -0.5)
    else:
        c.radial(lx, fy + 0.5, 2.3, lighter(HOLD_OFF, 0.3), darker(HOLD_OFF, 0.3), -0.5, -0.5)
    # A row of knobs and a slotted grille, to look the part.
    for i, mark in enumerate((-130, -60, 200)):
        knob(c, 92 + i * 13, fy + 0.5, 4.2, mark)
    for i in range(5):
        x = 40 + i * 7
        c.rect(x, fy - 2, x + 4, fy + 3, fill=darker(BODY, 0.55), radius=1)
        c.line([(x + 0.8, fy + 2.6), (x + 3.2, fy + 2.6)], lighter(BODY, 0.3), 0.5)
    return c


def traces(img, t0=0.0, scale=1):
    """`img` with two traces on its screen, about as the game draws them
    (for the tray's icon and the preview only)."""
    img = img.copy()
    px, py, pw, ph = PLOT
    put = img.putpixel
    prev = [None, None]
    for i in range(pw):
        t = (i + t0) / pw
        a = 0.5 + 0.42 * math.sin(t * 2 * math.pi * 2.2)
        b_on = math.sin(t * 2 * math.pi * 3.0) > 0
        rows = [round(a * (ph - 1)), 24 if b_on else 8]
        for k, (r, col) in enumerate(zip(rows, (TRACE_A, TRACE_B))):
            lo, hi = (r, r) if prev[k] is None else (min(r, prev[k]), max(r, prev[k]))
            for rr in range(lo, hi + 1):
                put((px + i, py + ph - 1 - rr), col + (255,))
            prev[k] = r
    return img


def icon(q, img):
    """The part as the tray shows it: small, with traces on its screen."""
    src = traces(img)
    bbox = src.getchannel("A").getbbox()
    src = src.crop(bbox)
    scale = min(TRAY_W / src.width, TRAY_H / src.height)
    w, h = max(1, round(src.width * scale)), max(1, round(src.height * scale))
    small = src.resize((w, h), Image.LANCZOS).filter(ImageFilter.UnsharpMask(radius=1, percent=60, threshold=2))
    c = Canvas(TRAY_W, TRAY_H)
    c.lo.alpha_composite(small, ((TRAY_W - w) // 2, (TRAY_H - h) // 2))
    return c.finish(q)


def draw(q):
    """Every frame: {sheet: [image, ...]}."""
    body = [scope(hold).finish(q) for hold in (False, True)]
    return {SCOPE: body, TRAY: [icon(q, body[0])]}


def write(sheets):
    for sid, frames in sheets.items():
        d = OUT / "gadgets" / str(sid)
        if d.is_dir():
            for old in d.glob("*.png"):
                old.unlink()
        d.mkdir(parents=True, exist_ok=True)
        for n, img in enumerate(frames, 1):
            img.save(d / f"{n}.png", optimize=True)
    lines = ["# The open parts' pictures (tools/openparts/draw.py): every frame's rectangle.",
             "# sheet frame top left bottom right"]
    for sid, frames in sheets.items():
        for n, img in enumerate(frames, 1):
            lines.append(f"{sid} {n} 0 0 {img.height} {img.width}")
    (OUT / "sheets.txt").write_text("\n".join(lines) + "\n")


def preview(sheets, path):
    z, gap = 4, 6
    body = sheets[SCOPE]
    pw = 3 * (W + gap) + gap + TRAY_W + gap
    img = Image.new("RGBA", (pw, H + 2 * gap), (192, 192, 192, 255))
    for i, f in enumerate([body[0], traces(body[0]), traces(body[1], 30)]):
        img.alpha_composite(f, (gap + i * (W + gap), gap))
    img.alpha_composite(sheets[TRAY][0], (gap + 3 * (W + gap), gap))
    img.resize((img.width * z, img.height * z), Image.NEAREST).save(path)


def main(argv):
    pal = [tuple(c[:3]) for c in json.loads((ROOT / "tools/opendata/src/palette.json").read_text())]
    q = wgfx.Quantizer(pal)
    pv = Path(argv[argv.index("--preview") + 1]) if "--preview" in argv else None
    sheets = draw(q)
    write(sheets)
    if pv:
        pv.mkdir(parents=True, exist_ok=True)
        preview(sheets, pv / "openparts.png")
    print("drawn")


if __name__ == "__main__":
    main(sys.argv[1:])
