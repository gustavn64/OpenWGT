#!/usr/bin/env python3
"""Draws the network parts' pictures (engine::net::pack) into
crates/engine/opennet, with OpenWGFX's kit (tools/opengfx/wgfx.py), and
their words in Swedish and French into the overlays opennet-sv and
opennet-fr (only the frames that differ):

    sheet 1900  the Sender, frames 1 to 4: its lamp grey (no link), red
                (nobody there), amber (there, stopped), green (running)
    sheet 1901  the Receiver, the same
    sheet 1902  the tray's icons: 1 the Sender, 2 the Receiver

and sheets.txt with the frames' rectangles (the parts are new: there's no
CD's geometry to take them from). The displays are left empty: the game
writes the channel name in them (net::parts::SENDER_LCD, RECEIVER_LCD,
which must stay where LCD_* below are), and the knots are where the knot
tables say (src/tables.json).

    draw.py [--preview DIR]

--preview writes every frame, big, to DIR/opennet-<lang>.png, with a
channel name in the displays as the game shows it, to look at.

Needs Pillow and the DejaVu fonts:
`nix-shell -p 'python3.withPackages(p: [p.pillow])' dejavu_fonts liberation_ttf`.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(ROOT / "tools/opengfx"))

import wgfx  # noqa: E402
from PIL import Image, ImageDraw, ImageFilter  # noqa: E402
from wgfx import INK, STEEL_D, STEEL_L, WHITE, Canvas, darker, lighter  # noqa: E402

OUT = ROOT / "crates/engine/opennet"
LANGS = ["en", "sv", "fr"]

# The words on the parts, short and loud.
WORDS = {
    "en": ("SEND", "RECEIVE"),
    "sv": ("SÄND", "TA EMOT"),
    "fr": ("ENVOYER", "RECEVOIR"),
}

W, H = 88, 50
TRAY_W, TRAY_H = 44, 25
SENDER, RECEIVER, TRAY = 1900, 1901, 1902

SEND = (200, 110, 40)     # copper: sends out
RECV = (40, 130, 170)     # teal: takes in
GLASS = (18, 34, 26)
LCD = (120, 255, 150)
WAVES_OUT = (255, 230, 120)
WAVES_IN = (190, 240, 255)
TIP = (255, 90, 60)

GREY, RED, AMBER, GREEN = (120, 120, 120), (220, 50, 40), (255, 190, 40), (60, 200, 80)
LAMPS = [GREY, RED, AMBER, GREEN]

# The displays (x, y, w, h), as net::parts has them.
LCD_SENDER = (14, 24, 66, 22)
LCD_RECEIVER = (8, 24, 66, 22)


def knots():
    t = json.loads((HERE / "src/tables.json").read_text())["knots"]
    (sv, sh, sk), = t[str(SENDER)]
    (rv, rh, rk), = t[str(RECEIVER)]
    assert sk == 0 and rk == 1, "the Sender's knot is an input, the Receiver's an output"
    return (sh, sv), (rh, rv)


def display(c, box):
    x, y, w, h = box
    c.inset(x, y, x + w, y + h, GLASS, radius=1.5)


def lamp(c, x, y, col):
    c.circle(x, y, 2.6, fill=INK)
    c.radial(x, y, 2.0, lighter(col, 0.5), darker(col, 0.3), -0.5, -0.5)


def mast(c, x, y_top, y_base):
    c.line([(x, y_base), (x, y_top)], INK, 3.0)
    c.line([(x, y_base), (x, y_top)], STEEL_L, 1.3)
    c.line([(x - 4, y_base), (x, y_top + 6), (x + 4, y_base)], STEEL_D, 1.0)
    c.circle(x, y_top, 2.4, fill=INK)
    c.circle(x - 0.3, y_top - 0.3, 1.6, fill=TIP)
    for i in range(3):
        c.arc(x, y_top, 4 + i * 3.2, -60, 60, WAVES_OUT, 1.3)


def dish(c, cx, cy):
    c.line([(cx + 1, cy + 1), (cx + 3, cy + 7)], INK, 2.6)
    c.line([(cx + 1, cy + 1), (cx + 3, cy + 7)], STEEL_L, 1.1)
    c.pieslice(cx, cy, 7, 135, 315, INK)
    c.pieslice(cx + 0.6, cy + 0.6, 6, 135, 315, STEEL_L)
    c.line([(cx - 1, cy - 1), (cx - 5, cy - 5)], STEEL_D, 1.0)
    c.circle(cx - 5, cy - 5, 1.3, fill=TIP)
    for i in range(3):
        c.arc(cx - 6, cy - 6, 4 + i * 3.2, 130, 230, WAVES_IN, 1.3)


def title(c, x0, x1, word, base):
    c.text_fit(x0, 13, x1, 23.5, word, WHITE, "sans", 9, stroke=1.0, stroke_fill=darker(base, 0.6))


def sender(word, col, knot):
    c = Canvas(W, H)
    c.panel(10, 12, W - 4, H - 2, SEND, radius=3, bevel=1.5)
    for sx in (13.5, W - 7.5):
        c.screw(sx, 15.5, 1.3)
    mast(c, W - 18, 2.5, 13)
    title(c, 16, W - 22, word, SEND)
    display(c, LCD_SENDER)
    lamp(c, W - 11.5, 18.5, col)
    c.knot(*knot, False, "l", reach=10)
    return c


def receiver(word, col, knot):
    c = Canvas(W, H)
    c.panel(4, 12, W - 10, H - 2, RECV, radius=3, bevel=1.5)
    for sx in (7.5, W - 13.5):
        c.screw(sx, 15.5, 1.3)
    dish(c, 20, 7)
    title(c, 17, W - 17, word, RECV)
    display(c, LCD_RECEIVER)
    lamp(c, 11.5, 18.5, col)
    c.knot(*knot, True, "r", reach=10)
    return c


def with_name(img, box, lines):
    """`img` with a name in its display, about as the game writes it (for
    the tray's icons and the preview only)."""
    img = img.copy()
    d = ImageDraw.Draw(img)
    x, y, w, h = box
    f = wgfx.font("sans-book", 8)
    pitch = 9
    top = y + (h - pitch * len(lines)) / 2 + pitch / 2
    for i, line in enumerate(lines):
        d.text((x + w / 2, top + i * pitch), line, font=f, fill=LCD + (255,), anchor="mm")
    return img


def icon(q, img, box):
    """A part as the tray shows it: small, a name in its display."""
    src = with_name(img, box, ["cat-sun-oak"])
    bbox = src.getchannel("A").getbbox()
    src = src.crop(bbox)
    scale = min((TRAY_W - 1) / src.width, (TRAY_H - 1) / src.height)
    w, h = max(1, round(src.width * scale)), max(1, round(src.height * scale))
    small = src.resize((w, h), Image.LANCZOS).filter(ImageFilter.UnsharpMask(radius=1, percent=60, threshold=2))
    c = Canvas(TRAY_W, TRAY_H)
    c.lo.alpha_composite(small, ((TRAY_W - w) // 2, (TRAY_H - h) // 2))
    return c.finish(q)


def draw(lang, q):
    """Every frame in `lang`: {sheet: [image, ...]}."""
    send_word, recv_word = WORDS[lang]
    sk, rk = knots()
    s = [sender(send_word, col, sk).finish(q) for col in LAMPS]
    r = [receiver(recv_word, col, rk).finish(q) for col in LAMPS]
    return {
        SENDER: s,
        RECEIVER: r,
        TRAY: [icon(q, s[3], LCD_SENDER), icon(q, r[3], LCD_RECEIVER)],
    }


def same(img, path):
    if not path.exists():
        return False
    old = Image.open(path).convert("RGBA")
    return old.size == img.size and old.tobytes() == img.convert("RGBA").tobytes()


def write(lang, sheets):
    out = OUT if lang == "en" else OUT.with_name(f"opennet-{lang}")
    for sid, frames in sheets.items():
        d = out / "gadgets" / str(sid)
        if d.is_dir():
            for old in d.glob("*.png"):
                old.unlink()
        for n, img in enumerate(frames, 1):
            if lang != "en" and same(img, OUT / "gadgets" / str(sid) / f"{n}.png"):
                continue  # no words in it: the English one does
            d.mkdir(parents=True, exist_ok=True)
            img.save(d / f"{n}.png", optimize=True)
        if d.is_dir() and not any(d.iterdir()):
            d.rmdir()
    if lang == "en":
        lines = ["# The network parts' pictures (tools/opennet/draw.py): every frame's rectangle.",
                 "# sheet frame top left bottom right"]
        for sid, frames in sheets.items():
            for n, img in enumerate(frames, 1):
                lines.append(f"{sid} {n} 0 0 {img.height} {img.width}")
        (out / "sheets.txt").write_text("\n".join(lines) + "\n")


def preview(lang, sheets, path):
    names = [["cat-sun-oak"], ["swamp-", "quilt-zebra"], ["kiwi42"], ["BUSY"]]
    z, gap = 4, 6
    rows = [(sheets[SENDER], LCD_SENDER), (sheets[RECEIVER], LCD_RECEIVER)]
    pw = 4 * (W + gap) + gap + TRAY_W * 2 + gap * 2
    img = Image.new("RGBA", (pw, 2 * (H + gap) + gap), (192, 192, 192, 255))
    for row, (frames, box) in enumerate(rows):
        for i, f in enumerate(frames):
            shown = with_name(f, box, names[i]) if not (row == 1 and i == 3) else with_name(f, box, names[0])
            img.alpha_composite(shown, (gap + i * (W + gap), gap + row * (H + gap)))
        img.alpha_composite(sheets[TRAY][row], (gap + 4 * (W + gap), gap + row * (H + gap)))
    img.resize((img.width * z, img.height * z), Image.NEAREST).save(path)


def main(argv):
    pal = [tuple(c[:3]) for c in json.loads((ROOT / "tools/opendata/src/palette.json").read_text())]
    q = wgfx.Quantizer(pal)
    pv = Path(argv[argv.index("--preview") + 1]) if "--preview" in argv else None
    # English first: the others keep only what differs from it.
    for lang in LANGS:
        wgfx.LANG = lang
        sheets = draw(lang, q)
        write(lang, sheets)
        if pv:
            pv.mkdir(parents=True, exist_ok=True)
            preview(lang, sheets, pv / f"opennet-{lang}.png")
    print("drawn")


if __name__ == "__main__":
    main(sys.argv[1:])
