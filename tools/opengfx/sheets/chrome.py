"""The editor's own pictures: the main toolbar (sheet 1, GO/STOP, light,
sound), the parts tray (2), the broken part (101), the dialogs' knot
arrows (504), Doc's button (505), the worksheet's backgrounds (700 to
702) and their pipe frame (704), the lock badge (800), the tray's
stipple and count digits (801), the scientist (901), the dynamite
plunger (1048) and the About box (10001 to 10004)."""
from __future__ import annotations

import math
import random

from wgfx import (tr, BRASS, BRASS_D, BRASS_L, GLASS, INK, KNOT_OUT, LED_GREEN, PANEL_BLUE, STEEL, STEEL_D, STEEL_L, WHITE,
                  Canvas, darker, lighter, mix, sheet)

from sheets._chrome_kit import GLOVE, Shapes, Xf, cuff, doc_face, glove_fist, glove_open, glove_point, place, scissors

BODY = (178, 184, 196)
BODY_L = (228, 232, 238)
BODY_D = (108, 114, 128)
NAVY = (18, 34, 84)
GO_GREEN = (30, 170, 60)
STOP_RED = (215, 40, 30)
TOOL_BLUE = (38, 104, 214)
CARD = (196, 200, 210)
SKIN = (242, 178, 140)
DIGIT_BLUE = (120, 200, 255)


# ---------------------------------------------------------------------------
# Common pieces


def frame_box(c: Canvas, x0, y0, x1, y1, body=BODY):
    """A window's flat body with a pixel-exact border: INK outside, a light
    top and left edge, a dark bottom and right one (all one pixel)."""
    c.rect(x0, y0, x1, y1, fill=INK)
    c.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, fill=lighter(body, 0.55))
    c.rect(x0 + 2, y0 + 2, x1 - 1, y1 - 1, fill=darker(body, 0.4))
    c.rect(x0 + 2, y0 + 2, x1 - 2, y1 - 2, fill=body)


def well(c: Canvas, x0, y0, x1, y1, fill=None, body=BODY):
    """A sunk well: dark top and left edge, light bottom and right."""
    c.rect(x0, y0, x1, y1, fill=lighter(body, 0.55))
    c.rect(x0, y0, x1 - 1, y1 - 1, fill=darker(body, 0.45))
    c.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, fill=fill or darker(body, 0.25))


def key(c: Canvas, x0, y0, x1, y1, base, down=False, radius=1.5):
    """A button face: raised, or pressed in (`down`)."""
    if not down:
        c.rect(x0, y0, x1, y1, fill=INK, radius=radius)
        c.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, fill=lighter(base, 0.5), radius=max(0, radius - 1))
        c.rect(x0 + 2, y0 + 2, x1 - 1, y1 - 1, fill=darker(base, 0.45), radius=max(0, radius - 1))
        c.gradient(x0 + 2, y0 + 2, x1 - 2, y1 - 2, [lighter(base, 0.15), base, darker(base, 0.1)])
    else:
        c.rect(x0, y0, x1, y1, fill=INK, radius=radius)
        c.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, fill=darker(base, 0.5), radius=max(0, radius - 1))
        c.rect(x0 + 2, y0 + 2, x1 - 1, y1 - 1, fill=lighter(base, 0.35), radius=max(0, radius - 1))
        c.gradient(x0 + 2, y0 + 2, x1 - 1, y1 - 1, [darker(base, 0.06), base])


def grip(c: Canvas, x0, y0, x1, y1):
    """A dark move bar with a row of rivets."""
    c.rect(x0, y0, x1, y1, fill=INK)
    c.gradient(x0 + 1, y0 + 1, x1 - 1, y1 - 1, [lighter(NAVY, 0.25), NAVY, darker(NAVY, 0.3)])
    cy = (y0 + y1) / 2
    n = int((x1 - x0 - 8) // 7)
    sx = (x0 + x1) / 2 - (n - 1) * 3.5
    for i in range(n):
        c.circle(sx + i * 7, cy, 1.5, fill=darker(NAVY, 0.5))
        c.circle(sx + i * 7 - 0.3, cy - 0.3, 1.0, fill=lighter(NAVY, 0.45))


# ---------------------------------------------------------------------------
# Sheet 1: the main toolbar, 53×93, the "on" art 53 pixels to the right.


def lamp_word(c, x0, y0, x1, y1, word, base, lit, down):
    key(c, x0, y0, x1, y1, base if lit else darker(base, 0.45), down=down, radius=2)
    d = 1 if down else 0
    if lit:
        c.text_fit(x0 + 3 + d, y0 + 2 + d, x1 - 3 + d, y1 - 2 + d, word, WHITE, "sans", 15, stroke=1.2,
                   stroke_fill=darker(base, 0.65))
    else:
        c.text_fit(x0 + 3, y0 + 2, x1 - 3, y1 - 2, word, darker(base, 0.1), "sans", 15, stroke=1.2,
                   stroke_fill=darker(base, 0.75))


def bulb(c, cx, cy, lit):
    """A light bulb: its glass centred at (cx, cy)."""
    if lit:
        for a in range(0, 360, 45):
            r = math.radians(a)
            c.line([(cx + math.cos(r) * 7.5, cy + math.sin(r) * 7.5), (cx + math.cos(r) * 9.5, cy + math.sin(r) * 9.5)],
                   (255, 230, 60), 1.2)
    glass = (255, 236, 80) if lit else (150, 156, 166)
    c.circle(cx, cy, 6.2, fill=INK)
    c.polygon([(cx - 3.6, cy + 2), (cx + 3.6, cy + 2), (cx + 3.0, cy + 7.2), (cx - 3.0, cy + 7.2)], fill=INK)
    c.radial(cx, cy, 5.2, lighter(glass, 0.8 if lit else 0.5), glass if lit else darker(glass, 0.3), -0.45, -0.45)
    c.polygon([(cx - 2.6, cy + 3), (cx + 2.6, cy + 3), (cx + 2.0, cy + 6.4), (cx - 2.0, cy + 6.4)],
              fill=glass if lit else darker(glass, 0.3))
    # filament
    c.line([(cx - 1.6, cy + 4), (cx - 1.2, cy), (cx, cy + 1.2), (cx + 1.2, cy), (cx + 1.6, cy + 4)],
           (255, 140, 0) if lit else (80, 80, 90), 0.7)
    # screw base
    c.rect(cx - 3.2, cy + 7, cx + 3.2, cy + 11.5, fill=INK, radius=1)
    for i in range(3):
        c.rect(cx - 2.4, cy + 7.6 + i * 1.3, cx + 2.4, cy + 8.4 + i * 1.3, fill=STEEL_L if i % 2 == 0 else STEEL)
    c.rect(cx - 1.2, cy + 11, cx + 1.2, cy + 12.5, fill=INK, radius=0.6)


def speaker(c, cx, cy, on):
    """A loudspeaker facing right, with sound waves when `on`."""
    sx = cx - 4.5
    c.rect(sx - 4.5, cy - 3.2, sx + 0.5, cy + 3.2, fill=INK)
    c.rect(sx - 3.5, cy - 2.2, sx + 0.5, cy + 2.2, fill=STEEL)
    c.polygon([(sx - 0.5, cy - 3.2), (sx + 5.5, cy - 8), (sx + 5.5, cy + 8), (sx - 0.5, cy + 3.2)], fill=INK)
    c.polygon([(sx + 0.5, cy - 2.4), (sx + 4.5, cy - 6), (sx + 4.5, cy + 6), (sx + 0.5, cy + 2.4)],
              fill=STEEL_L if on else STEEL)
    if on:
        for r in (3.0, 6.0):
            c.arc(sx + 5.5, cy, r, -55, 55, INK, 1.6)
    else:
        x0 = sx + 7.5
        c.line([(x0, cy - 2.5), (x0 + 4.5, cy + 2.5)], INK, 1.8)
        c.line([(x0, cy + 2.5), (x0 + 4.5, cy - 2.5)], INK, 1.8)
        c.line([(x0, cy - 2.5), (x0 + 4.5, cy + 2.5)], (220, 60, 50), 0.8)
        c.line([(x0, cy + 2.5), (x0 + 4.5, cy - 2.5)], (220, 60, 50), 0.8)


def main_face(on: bool) -> Canvas:
    c = Canvas(53, 93)
    frame_box(c, 0, 0, 53, 93)
    grip(c, 3, 3, 50, 15)
    well(c, 3, 17, 50, 62)
    well(c, 3, 62, 50, 91)
    lamp_word(c, 5, 19, 47, 38, tr("GO"), GO_GREEN, on, on)
    lamp_word(c, 5, 40, 47, 59, tr("STOP"), STOP_RED, on, on)
    lit = (255, 214, 90)
    for (x0, x1), draw in (((5, 25), lambda cx, cy: bulb(c, cx, cy - 2, on)), ((27, 47), lambda cx, cy: speaker(c, cx, cy, on))):
        key(c, x0, 64, x1, 88, lit if on else BODY, down=on, radius=1.5)
        d = 1 if on else 0
        draw((x0 + x1) / 2 + d, 76 + d)
    return c


@sheet(1)
def main_toolbar(ctx):
    c = ctx.canvas(1)
    place(c, main_face(False), 0, 0)
    place(c, main_face(True), 53, 0)
    return [c]


# ---------------------------------------------------------------------------
# Sheet 2: the parts tray. x 0..449 the face, beyond it the tools' "on"
# art, the page arrows' pressed and disabled art and the page digits; the
# tabs' "on" art lies in the face under the slot area (y 53..84).


def monitor_icon(c, cx, cy):
    c.rect(cx - 3, cy + 6, cx + 3, cy + 10, fill=INK)
    c.rect(cx - 2, cy + 6, cx + 2, cy + 9, fill=STEEL_D)
    c.rect(cx - 7, cy + 8.5, cx + 7, cy + 11, fill=INK, radius=1)
    c.rect(cx - 6, cy + 9, cx + 6, cy + 10.2, fill=STEEL)
    c.rect(cx - 11, cy - 9, cx + 11, cy + 7.5, fill=INK, radius=2)
    c.rect(cx - 10, cy - 8, cx + 10, cy + 6.5, fill=PANEL_BLUE, radius=1.5)
    c.rect(cx - 10, cy - 8, cx + 10, cy - 6.5, fill=lighter(PANEL_BLUE, 0.4), radius=1)
    c.rect(cx - 8, cy - 6, cx + 8, cy + 4.5, fill=INK)
    c.gradient(cx - 7, cy - 5, cx + 7, cy + 3.5, [(30, 60, 50), (10, 30, 25)])
    c.line([(cx - 6, cy), (cx - 3, cy), (cx - 1.5, cy - 3.5), (cx + 0.5, cy + 2.5), (cx + 2, cy), (cx + 6, cy)],
           LED_GREEN, 1.0, round_ends=False)
    c.circle(cx + 8, cy + 5.4, 0.7, fill=(255, 80, 40))


def stopwatch_icon(c, cx, cy):
    cy += 1.5
    c.rect(cx - 2, cy - 13, cx + 2, cy - 9, fill=INK, radius=0.8)
    c.rect(cx - 1.2, cy - 12.3, cx + 1.2, cy - 9, fill=STEEL_L)
    c.line([(cx + 7, cy - 7), (cx + 9, cy - 9)], INK, 3)
    c.line([(cx + 7, cy - 7), (cx + 9, cy - 9)], STEEL, 1.4)
    c.circle(cx, cy, 10, fill=INK)
    c.radial(cx, cy, 9, STEEL_L, STEEL_D, -0.5, -0.5)
    c.circle(cx, cy, 7.4, fill=INK)
    c.circle(cx, cy, 6.6, fill=WHITE)
    for a in range(0, 360, 30):
        r = math.radians(a)
        c.line([(cx + math.cos(r) * 5.0, cy + math.sin(r) * 5.0), (cx + math.cos(r) * 6.2, cy + math.sin(r) * 6.2)], INK,
               0.6, round_ends=False)
    c.line([(cx, cy), (cx + 3.6, cy - 3.6)], (220, 30, 30), 1.0)
    c.line([(cx, cy), (cx - 0.5, cy + 4.5)], INK, 0.8)
    c.circle(cx, cy, 1.0, fill=INK)


def ear_icon(c, cx, cy):
    # the outer ear, a lobe below
    o = INK
    c.ellipse(cx - 8, cy - 11, cx + 7.5, cy + 8, fill=o)
    c.ellipse(cx - 6, cy + 2, cx + 3.5, cy + 12, fill=o)
    c.ellipse(cx - 7, cy - 10, cx + 6.5, cy + 7, fill=SKIN)
    c.ellipse(cx - 5, cy + 3, cx + 2.5, cy + 11, fill=SKIN)
    # the rim's fold and the hollow
    c.arc(cx - 0.5, cy - 1.5, 5.6, 200, 430, darker(SKIN, 0.35), 1.6)
    c.ellipse(cx - 2.8, cy - 1.5, cx + 2.6, cy + 5, fill=darker(SKIN, 0.25))
    c.ellipse(cx - 2.6, cy + 0.5, cx + 0.8, cy + 4.5, fill=darker(SKIN, 0.5))
    c.arc(cx - 0.5, cy - 2.0, 7.0, 230, 330, lighter(SKIN, 0.5), 0.8)


def numbers_icon(c, cx, cy):
    for ch, dx, dy, col in (("1", -8, 2, (225, 40, 40)), ("2", 0, -3, (40, 100, 230)), ("3", 8, 3, (40, 170, 70))):
        c.text(cx + dx, cy + dy, ch, 17, col, "sans", stroke=1.3, stroke_fill=INK)


def switch_icon(c, cx, cy):
    c.rect(cx - 7, cy - 12, cx + 7, cy + 12, fill=INK, radius=2)
    c.gradient(cx - 6, cy - 11, cx + 6, cy + 11, [(255, 214, 120), (236, 150, 40)], radius=1.5)
    c.screw(cx, cy - 9, 1.3)
    c.screw(cx, cy + 9, 1.3)
    c.rect(cx - 3.2, cy - 6, cx + 3.2, cy + 6, fill=INK, radius=1)
    c.rect(cx - 2.4, cy - 5.2, cx + 2.4, cy + 5.2, fill=(60, 40, 30))
    c.rect(cx - 2.4, cy - 5.2, cx + 2.4, cy + 0.6, fill=INK, radius=0.6)
    c.gradient(cx - 1.8, cy - 4.8, cx + 1.8, cy - 0.1, [WHITE, (200, 200, 205)], radius=0.5)


def letter_icon(c, cx, cy):
    c.text(cx - 1, cy + 1, "A", 26, (150, 60, 200), "serif", stroke=1.4, stroke_fill=INK)
    c.line([(cx + 6, cy + 8), (cx + 13, cy + 1)], INK, 2.8)
    c.line([(cx + 6, cy + 8), (cx + 13, cy + 1)], (255, 200, 60), 1.4)
    c.line([(cx + 5.2, cy + 8.8), (cx + 6.4, cy + 7.6)], INK, 1.4)


def planet_icon(c, cx, cy):
    def ring(front):
        lo, hi = (0, 180) if front else (180, 360)
        c.arc(cx, cy + 0.5, 0, 0, 0, INK, 0) if False else None
        x = Xf(c, cx, cy, 1, -18)
        pts = [(math.cos(math.radians(a)) * 13, math.sin(math.radians(a)) * 4) for a in range(lo, hi + 1, 10)]
        for p, q in zip(pts, pts[1:]):
            x.cap(p, q, 3.2, INK)
        for p, q in zip(pts, pts[1:]):
            x.cap(p, q, 1.6, (250, 200, 90))

    ring(False)
    c.circle(cx, cy, 7.4, fill=INK)
    c.radial(cx, cy, 6.6, (255, 190, 130), (200, 80, 40), -0.5, -0.5)
    c.arc(cx, cy, 5.0, 200, 250, (255, 230, 200), 0.8)
    ring(True)
    for sx, sy in ((cx - 11, cy - 8), (cx + 11, cy + 8), (cx + 10, cy - 10)):
        c.line([(sx - 1.6, sy), (sx + 1.6, sy)], (255, 240, 120), 0.8)
        c.line([(sx, sy - 1.6), (sx, sy + 1.6)], (255, 240, 120), 0.8)


TAB_ICONS = [monitor_icon, stopwatch_icon, ear_icon, numbers_icon, switch_icon, letter_icon, planet_icon]


def tab(icon, on: bool) -> Canvas:
    """A folder tab, 49×31: a grey card, or the open (white) one."""
    c = Canvas(49, 31)
    if on:
        c.rect(0, 0, 49, 31, fill=INK, radius=3)
        c.rect(1, 1, 48, 31, fill=darker(BODY, 0.2), radius=2.5)
        c.rect(2, 2, 48, 31, fill=WHITE, radius=2)
        c.gradient(2, 2, 47, 31, [(255, 255, 255), (238, 242, 248)], radius=2)
    else:
        c.rect(0, 0, 49, 31, fill=INK, radius=3)
        c.rect(1, 1, 48, 30, fill=lighter(CARD, 0.6), radius=2.5)
        c.rect(2, 2, 48, 30, fill=darker(CARD, 0.4), radius=2)
        c.gradient(2, 2, 47, 29, [lighter(CARD, 0.15), CARD, darker(CARD, 0.08)], radius=2)
    d = 1 if on else 0
    icon(c, 24.5 + d, 15.5 + d)
    return c


def tool_key(w, h, on, draw) -> Canvas:
    c = Canvas(w, h)
    key(c, 0, 0, w, h, lighter(TOOL_BLUE, 0.45) if on else TOOL_BLUE, down=on, radius=2)
    draw(c, 1 if on else 0)
    return c


def pointer_tool(c, d):
    glove_point(Shapes(Xf(c, 31 + d, 19 + d, 1.25, -45), 1.0))


def cut_tool(c, d):
    scissors(c, 14.5 + d, 18.5 + d, 1.05, -30, hole=TOOL_BLUE if not d else lighter(TOOL_BLUE, 0.45))


def move_tool(c, d):
    glove_open(Shapes(Xf(c, 14 + d, 19 + d, 0.95, 0), 1.0))


def arrow_key(up: bool, state: str) -> Canvas:
    """A page arrow, 21×16: `state` "normal", "down" or "off"."""
    c = Canvas(21, 16)
    base = BODY if state == "off" else TOOL_BLUE
    key(c, 0, 0, 21, 16, base, down=state == "down", radius=1.5)
    d = 1 if state == "down" else 0
    col = darker(BODY, 0.2) if state == "off" else (255, 120, 40)
    if up:
        pts = [(10.5 + d, 3.5 + d), (16 + d, 11 + d), (5 + d, 11 + d)]
    else:
        pts = [(10.5 + d, 12.5 + d), (16 + d, 5 + d), (5 + d, 5 + d)]
    c.polygon(pts, fill=INK if state != "off" else darker(BODY, 0.35), outline=INK if state != "off" else darker(BODY, 0.35), width=1.6)
    c.polygon(pts, fill=col)
    return c


# Pixel fonts of our own: the page digits (6×9) and the counts (6×7).
PAGE_FONT = {
    "1": ["..##..", ".###..", "..##..", "..##..", "..##..", "..##..", "..##..", "..##..", ".####."],
    "2": [".####.", "##..##", "....##", "...##.", "..##..", ".##...", "##....", "##....", "######"],
    "3": [".####.", "##..##", "....##", "..###.", "....##", "....##", "....##", "##..##", ".####."],
    "4": ["...##.", "..###.", ".####.", "##.##.", "##.##.", "######", "...##.", "...##.", "...##."],
    "5": ["######", "##....", "##....", "#####.", "....##", "....##", "....##", "##..##", ".####."],
    "6": [".####.", "##..##", "##....", "#####.", "##..##", "##..##", "##..##", "##..##", ".####."],
}
COUNT_FONT = {
    "0": [".####.", "##..##", "##..##", "##..##", "##..##", "##..##", ".####."],
    "1": ["..##..", ".###..", "..##..", "..##..", "..##..", "..##..", ".####."],
    "2": [".####.", "##..##", "....##", "..###.", ".##...", "##....", "######"],
    "3": ["#####.", "....##", "....##", ".####.", "....##", "....##", "#####."],
    "4": ["...##.", "..###.", ".#.##.", "##.##.", "######", "...##.", "...##."],
    "5": ["######", "##....", "#####.", "....##", "....##", "##..##", ".####."],
    "6": [".####.", "##....", "##....", "#####.", "##..##", "##..##", ".####."],
    "7": ["######", "....##", "...##.", "..##..", "..##..", ".##...", ".##..."],
    "8": [".####.", "##..##", "##..##", ".####.", "##..##", "##..##", ".####."],
    "9": [".####.", "##..##", "##..##", ".#####", "....##", "....##", ".####."],
}
# 3×5 capitals for the knot arrows.
TINY_FONT = {
    "I": ["###", ".#.", ".#.", ".#.", "###"],
    "N": ["#.#", "###", "###", "#.#", "#.#"],
    "O": [".#.", "#.#", "#.#", "#.#", ".#."],
    "U": ["#.#", "#.#", "#.#", "#.#", "###"],
    "T": ["###", ".#.", ".#.", ".#.", ".#."],
}


def glyph(c: Canvas, x, y, rows, col):
    for j, row in enumerate(rows):
        for i, ch in enumerate(row):
            if ch == "#":
                c.pixel(x + i, y + j, col)


def page_digit(c: Canvas, x, y, w, h, n):
    """The page number `n` in light blue on the number window's glass."""
    c.prect(x, y, x + w, y + h, GLASS)
    glyph(c, x + 3, y + 1, PAGE_FONT[str(n)], DIGIT_BLUE)


@sheet(2)
def tray(ctx):
    c = ctx.canvas(1)
    # the window
    frame_box(c, 0, 0, 449, 93)
    # the help line (the code writes white text in (4,3)-(445,15))
    c.rect(3, 2, 446, 16, fill=darker(BODY, 0.45))
    c.rect(4, 3, 446, 16, fill=lighter(BODY, 0.55))
    c.rect(4, 3, 445, 15, fill=NAVY)
    # the tools
    well(c, 3, 17, 64, 90)
    place(c, tool_key(58, 34, False, pointer_tool), 5, 18)
    place(c, tool_key(28, 34, False, cut_tool), 5, 54)
    place(c, tool_key(28, 34, False, move_tool), 35, 54)
    # the slot area's frame (the area itself, (67,51)-(417,91), is the
    # code's); the tabs' "on" art lies under it
    c.rect(64, 48, 420, 91, fill=darker(BODY, 0.45))
    c.rect(65, 49, 420, 91, fill=INK)
    c.rect(66, 50, 420, 91, fill=darker(BODY, 0.2))
    c.rect(67, 51, 420, 91, fill=lighter(BODY, 0.55))
    c.rect(67, 51, 419, 91, fill=BODY)
    c.rect(67, 51, 417, 91, fill=WHITE)
    for i, icon in enumerate(TAB_ICONS):
        x = 68 + 50 * i
        place(c, tab(icon, False), x, 18)
        place(c, tab(icon, True), x, 53)
    # the page arrows and number
    well(c, 421, 24, 446, 85)
    place(c, arrow_key(True, "normal"), 423, 27)
    place(c, arrow_key(False, "normal"), 423, 65)
    c.rect(423, 44, 444, 64, fill=INK)
    c.rect(424, 45, 443, 63, fill=GLASS)
    # the tools' "on" art
    place(c, tool_key(58, 34, True, pointer_tool), 450, 0)
    place(c, tool_key(28, 34, True, cut_tool), 450, 36)
    place(c, tool_key(28, 34, True, move_tool), 480, 36)
    # the arrows pressed and disabled
    place(c, arrow_key(True, "down"), 450, 74)
    place(c, arrow_key(True, "off"), 471, 74)
    place(c, arrow_key(False, "down"), 450, 90)
    place(c, arrow_key(False, "off"), 471, 90)
    # the digits 1..6 at (511, 0..66)
    for n, (y0, y1) in enumerate(((0, 10), (11, 22), (22, 33), (33, 44), (44, 55), (55, 66)), 1):
        page_digit(c, 511, y0, 12, y1 - y0, n)
    return [c]


# ---------------------------------------------------------------------------
# Sheet 101: a part that can't work here (drawn over it): a red cross.


def cross(n):
    c = Canvas(n, n)
    m = n * 0.12
    for col, w in ((INK, n * 0.2 + 2), ((225, 30, 30), n * 0.2)):
        c.line([(m, m), (n - m, n - m)], col, w)
        c.line([(m, n - m), (n - m, m)], col, w)
    c.line([(m + 1, m + 0.6), (n / 2 - 1, n / 2 - 1.4)], (255, 130, 110), max(0.8, n * 0.04))
    return c


@sheet(101)
def broken(ctx):
    return [cross(41), cross(23)]


# ---------------------------------------------------------------------------
# Sheet 504: the arrows a settings dialog puts by the part's knots: into
# an input from the left (1) and from above (2), out of an output (3).


def knot_arrow(w, h, tip, d, label, col):
    """An arrow pointing in direction `d` ("r" or "d") with its tip at
    `tip`, the label in tiny capitals on its shaft, a shadow below."""
    c = Canvas(w, h)
    tx, ty = tip
    if d == "r":
        sx0 = max(0.5, tx - 11 - len(label) * 4)
        pts = [(sx0, ty - 4), (tx - 8, ty - 4), (tx - 8, ty - 8.5), (tx + 0.5, ty + 0.5), (tx - 8, ty + 9.5),
               (tx - 8, ty + 5), (sx0, ty + 5)]
    else:
        sy0 = 0.5
        pts = [(tx - 4, sy0), (tx - 4, ty - 8), (tx - 8.5, ty - 8), (tx + 0.5, ty + 0.5), (tx + 9.5, ty - 8),
               (tx + 5, ty - 8), (tx + 5, sy0)]
    c.polygon(pts, fill=INK, outline=INK, width=1.6)
    c.polygon(pts, fill=col)
    lab = INK
    if d == "r":
        for i, ch in enumerate(label):
            glyph(c, int(sx0) + 2 + i * 4, ty - 2, TINY_FONT[ch], lab)
    else:
        for i, ch in enumerate(label):
            glyph(c, tx - 3 + i * 4, 2, TINY_FONT[ch], lab)
    return c


@sheet(504)
def knot_arrows(ctx):
    # frame offsets (-18,-10), (-10,-18), (1,-10): the knot is at
    # (18,10), (10,18) and (-1,10) of the frames
    gold, orange = (255, 214, 40), (255, 130, 30)
    return [knot_arrow(19, 22, (18, 10), "r", tr("IN"), gold),
            knot_arrow(21, 19, (10, 18), "d", tr("IN"), gold),
            knot_arrow(24, 25, (23, 10), "r", tr("OUT"), orange)]


# ---------------------------------------------------------------------------
# Sheet 800: the lock badge on locked parts (puzzle authors' mode).


@sheet(800)
def lock(ctx):
    c = ctx.canvas(1)
    c.arc(7, 7, 4.6, 180, 360, INK, 3.4)
    c.rect(2.4, 6.6, 4.6, 9, fill=INK)
    c.rect(9.4, 6.6, 11.6, 9, fill=INK)
    c.arc(7, 7, 4.6, 180, 360, STEEL_L, 1.4)
    c.rect(3, 6.8, 4.1, 9, fill=STEEL_L)
    c.rect(9.9, 6.8, 11, 9, fill=STEEL_L)
    c.rect(0, 8, 14, 17, fill=INK, radius=1.5)
    c.gradient(1, 9, 13, 16, [BRASS_L, BRASS, BRASS_D], radius=1)
    c.prect(6, 11, 8, 13, INK)
    c.prect(7, 13, 8, 15, INK)
    return [c]


# ---------------------------------------------------------------------------
# Sheet 801: the stipple that greys out a part none are left of, and the
# count digits 0..9 (8×9) put at a limited part's corner.


def count_digit(ch):
    """A digit of a little LED counter: amber on dark glass; the digits of
    a number sit side by side as one strip."""
    c = Canvas(8, 9)
    c.prect(0, 0, 8, 9, GLASS)
    glyph(c, 1, 1, COUNT_FONT[ch], (255, 200, 40))
    return c


@sheet(801)
def tray_marks(ctx):
    c = ctx.canvas(1)
    for y in range(c.h):
        for x in range((y + 1) % 2, c.w, 2):
            c.pixel(x, y, WHITE)
    return [c] + [count_digit(str(d)) for d in range(10)]


# ---------------------------------------------------------------------------
# Sheets 700, 701, 702: the worksheet's backgrounds, tiles that repeat
# without a seam; frame 1 with the light on, 2 with it off.


def texture(w, h, fn, dither=True):
    c = Canvas(w, h)
    for y in range(h):
        for x in range(w):
            c.pixel(x, y, fn(x, y))
    if dither:
        c.dither()
    return c


def clamp(v):
    return max(0, min(255, int(round(v))))


def shade(col, k):
    """`col` lighter (k > 0) or darker (k < 0), k in −1..1."""
    return lighter(col, k) if k >= 0 else darker(col, -k)


@sheet(700)
def speckle(ctx):
    """"Noise": a speckled lab bench top, flecks of a few tones."""
    w, h = ctx.size(1)

    def make(base, flecks, seed):
        rnd = random.Random(seed)
        grid = [[base] * w for _ in range(h)]
        for _ in range(int(w * h * 0.075)):
            x, y = rnd.randrange(w), rnd.randrange(h)
            col = rnd.choice(flecks)
            grid[y][x] = col
            if rnd.random() < 0.35:
                grid[y][(x + 1) % w] = col
            if rnd.random() < 0.2:
                grid[(y + 1) % h][x] = col
        return texture(w, h, lambda x, y: grid[y][x], dither=False)

    return [make((244, 244, 246), [(206, 208, 214), (184, 188, 196), (222, 226, 236)], 7),
            make((40, 40, 46), [(70, 70, 78), (24, 24, 28), (90, 92, 104)], 7)]


@sheet(701)
def waves(ctx):
    """"Metal Waves": a corrugated sheet, its ridges waving across."""
    w, h = ctx.size(1)

    def fn(base, amp):
        def f(x, y):
            ph = 2 * math.pi * ((y + 0.5) / 16 + 0.18 * math.sin(2 * math.pi * (x + 0.5) / 48))
            k = math.sin(ph) * amp + math.cos(ph) * amp * 0.35
            return shade(base, k)
        return f

    return [texture(w, h, fn((214, 216, 222), 0.2)), texture(w, h, fn((50, 52, 60), 0.3))]


@sheet(702)
def plates(ctx):
    """"Steel": brushed steel plates, riveted along their edges."""
    w, h = ctx.size(1)
    P = 64
    along = (4, 18, 32, 45, 59)
    RIVETS = sorted({(e, a) for e in (4, 59) for a in along} | {(a, e) for e in (4, 59) for a in along})

    def make(base, seed):
        rnd = random.Random(seed)
        streak = [rnd.uniform(-1, 1) for _ in range(h)]
        tone = {(i, j): rnd.uniform(-0.06, 0.06) for i in range(w // P) for j in range(h // P)}
        noise = [[rnd.uniform(-1, 1) for _ in range(w)] for _ in range(h)]

        def f(x, y):
            px, py = x % P, y % P
            k = tone[(x // P, y // P)] + streak[y] * 0.06 + noise[y][x] * 0.05
            # the seams: a dark line, a light one
            if px == P - 1 or py == P - 1:
                return shade(base, -0.45)
            if px == 0 or py == 0:
                return shade(base, 0.35)
            # rivets along the edges, 4 in from them
            for ax, ay in RIVETS:
                dx, dy = px - ax, py - ay
                if dx * dx + dy * dy <= 2:
                    return shade(base, 0.5 if dx + dy < 0 else (0.0 if dx + dy == 0 else -0.4))
                if (dx, dy) in ((2, 1), (1, 2), (2, 0), (0, 2), (2, 2)):
                    return shade(base, -0.25)
            return shade(base, k)
        return f

    return [texture(w, h, make((200, 202, 208), 3), dither=False), texture(w, h, make((62, 66, 76), 3), dither=False)]


# ---------------------------------------------------------------------------
# Sheet 704: the pipes framing a textured worksheet: corners (84×54) 1 top
# left, 3 top right, 5 bottom right, 7 bottom left; 2 and 6 the top and
# bottom (192×11), 4 and 8 the right and left (11×158), all repeated.

PIPE = [INK, STEEL_D, STEEL, STEEL_L, lighter(STEEL_L, 0.5), WHITE, lighter(STEEL_L, 0.5), STEEL_L, STEEL, STEEL_D, INK]
COLLAR = [INK, BRASS_D, BRASS, BRASS_L, lighter(BRASS_L, 0.5), BRASS_L, BRASS, BRASS, BRASS_D, darker(BRASS_D, 0.3), INK]


def pipe_run(c: Canvas, x0, x1, y0=0, prof=PIPE):
    """A run of pipe along x (on the smooth layer, so fittings go on it)."""
    for j in range(11):
        c.rect(x0, y0 + j, x1, y0 + j + 1, fill=prof[j])


def collar(c: Canvas, x, y0=0):
    """A brass joint ring, 4 wide."""
    c.rect(x, y0, x + 4, y0 + 11, fill=INK)
    for j in range(1, 10):
        c.rect(x + 1, y0 + j, x + 3, y0 + j + 1, fill=COLLAR[j])


def rot90(src: Canvas, k: int) -> Canvas:
    """`src` turned k quarter turns anticlockwise."""
    from PIL import Image, ImageDraw
    op = {1: Image.ROTATE_90, 2: Image.ROTATE_180, 3: Image.ROTATE_270}[k % 4]
    w, h = (src.h, src.w) if k % 2 else (src.w, src.h)
    c = Canvas(w, h)
    c.hi = src.hi.transpose(op)
    c.d = ImageDraw.Draw(c.hi)
    c.lo = src.lo.transpose(op)
    c.ld = ImageDraw.Draw(c.lo)
    return c


def pipe_corner() -> Canvas:
    """The top left corner: an elbow (radii 3 to 14 round (14, 14)), the
    pipes on along the top and down the left."""
    c = Canvas(84, 54)
    R = 14
    for y in range(R):
        for x in range(R):
            d = math.hypot(x + 0.5 - R, y + 0.5 - R)
            if R - 11 <= d <= R:
                c.rect(x, y, x + 1, y + 1, fill=PIPE[min(10, int(R - d))])
    pipe_run(c, R, 84)
    v = Canvas(54 - R, 11)
    pipe_run(v, 0, 54 - R)
    collar(v, 2)
    collar(v, 30)
    place(c, rot90(v, 3), 0, R)
    collar(c, R + 2)
    collar(c, 70)
    # a pressure gauge on the elbow
    c.circle(6.5, 6.5, 5.6, fill=INK)
    c.circle(6.5, 6.5, 4.6, fill=WHITE)
    c.line([(6.5, 6.5), (8.6, 4.0)], (220, 30, 30), 1.0)
    c.circle(6.5, 6.5, 0.9, fill=INK)
    return c


def sight_glass(c, x0, x1, liquid):
    c.prect(x0, 1, x1, 10, INK)
    c.prect(x0 + 1, 2, x1 - 1, 9, (200, 230, 240))
    c.prect(x0 + 1, 5, x1 - 1, 9, liquid)
    c.prect(x0 + 1, 5, x1 - 1, 6, lighter(liquid, 0.4))
    for bx, by in ((x0 + 4, 7), (x0 + 9, 6), (x0 + 15, 7), (x0 + 21, 6), (x0 + 12, 3)):
        if bx < x1 - 2:
            c.pixel(bx, by, WHITE)
    c.prect(x0 + 1, 2, x1 - 1, 3, WHITE)


def valve(c, cx, col=(220, 40, 40)):
    c.circle(cx, 5.5, 5.5, fill=INK)
    c.circle(cx, 5.5, 4.6, fill=col)
    c.circle(cx, 5.5, 3.0, fill=INK)
    c.circle(cx, 5.5, 2.2, fill=(0, 0, 0, 0))
    for a in (0, 60, 120):
        r = math.radians(a)
        c.line([(cx - math.cos(r) * 4, 5.5 - math.sin(r) * 4), (cx + math.cos(r) * 4, 5.5 + math.sin(r) * 4)], col, 1.0)
    c.circle(cx, 5.5, 1.3, fill=lighter(col, 0.4))


def gauge(c, cx, needle=40):
    c.circle(cx, 5.5, 5.5, fill=INK)
    c.circle(cx, 5.5, 4.6, fill=BRASS)
    c.circle(cx, 5.5, 3.8, fill=WHITE)
    r = math.radians(needle - 90)
    c.line([(cx, 5.5), (cx + math.cos(r) * 3.2, 5.5 + math.sin(r) * 3.2)], (220, 30, 30), 0.9)
    c.circle(cx, 5.5, 0.7, fill=INK)


def hazard(c, x0, x1):
    c.prect(x0, 0, x1, 11, INK)
    for x in range(x0, x1):
        for y in range(1, 10):
            c.pixel(x, y, (255, 210, 30) if ((x + y) // 3) % 2 == 0 else (30, 30, 30))


def pipe_straight(n) -> Canvas:
    """A straight run `n` long (along x), its fittings between the joints."""
    c = Canvas(n, 11)
    pipe_run(c, 0, n)
    collar(c, 0)
    if n == 192:
        sight_glass(c, 20, 46, (60, 220, 90))
        valve(c, 62)
        collar(c, 96)
        gauge(c, 112, 30)
        hazard(c, 126, 142)
        sight_glass(c, 156, 178, (90, 140, 255))
    else:
        valve(c, 22, (40, 120, 220))
        sight_glass(c, 40, 66, (255, 140, 40))
        collar(c, 80)
        gauge(c, 98, -20)
        hazard(c, 112, 126)
        sight_glass(c, 134, 150, (60, 220, 90))
    return c


@sheet(704)
def pipes(ctx):
    corner = pipe_corner()
    h = pipe_straight(192)
    v = pipe_straight(158)
    tl = corner

    def flip_x(src):
        from PIL import Image, ImageDraw
        c = Canvas(src.w, src.h)
        c.hi = src.hi.transpose(Image.FLIP_LEFT_RIGHT)
        c.d = ImageDraw.Draw(c.hi)
        c.lo = src.lo.transpose(Image.FLIP_LEFT_RIGHT)
        c.ld = ImageDraw.Draw(c.lo)
        return c

    def flip_y(src):
        from PIL import Image, ImageDraw
        c = Canvas(src.w, src.h)
        c.hi = src.hi.transpose(Image.FLIP_TOP_BOTTOM)
        c.d = ImageDraw.Draw(c.hi)
        c.lo = src.lo.transpose(Image.FLIP_TOP_BOTTOM)
        c.ld = ImageDraw.Draw(c.lo)
        return c

    return [tl, h, flip_x(tl), rot90(v, 1), flip_x(flip_y(tl)), h, flip_y(tl), rot90(v, 3)]


# ---------------------------------------------------------------------------
# Sheet 505: Doc's button in the settings dialogs, 32×30: 1 up and 2 down
# by the part's description, 3 and 4 (in a hard hat) by the building tip.


def doc_button(hat, down):
    c = Canvas(32, 30)
    key(c, 0, 0, 32, 30, (150, 200, 240), down=down, radius=2)
    face = Canvas(28, 26)
    d = 1 if down else 0
    doc_face(Xf(face, 14 + d, 14.6 + d, 1.12), 1.0, hat=hat)
    place(c, face, 2, 2)
    return c


@sheet(505)
def doc_buttons(ctx):
    return [doc_button(False, False), doc_button(False, True), doc_button(True, False), doc_button(True, True)]


# ---------------------------------------------------------------------------
# Sheet 901: the About box's hidden picture: Doc in his lab coat, holding up
# a bubbling flask, before a chalkboard.


def flask(c: Canvas, cx, top, h, liquid, o=2.0):
    """An Erlenmeyer flask, its neck's top centred at (cx, top), `h` tall."""
    w = h * 0.42
    neck = h * 0.11
    pts = [(cx - neck, top), (cx + neck, top), (cx + neck, top + h * 0.38), (cx + w, top + h - 3),
           (cx + w - 3, top + h), (cx - w + 3, top + h), (cx - w, top + h - 3), (cx - neck, top + h * 0.38)]
    c.polygon(pts, fill=INK, outline=INK, width=o * 2)
    c.polygon(pts, fill=(210, 236, 246))
    ly = top + h * 0.55
    k = (ly - (top + h * 0.38)) / (h * 0.62 - 3)
    lw = neck + (w - neck) * k
    c.polygon([(cx - lw, ly), (cx + lw, ly), (cx + w, top + h - 3), (cx + w - 3, top + h), (cx - w + 3, top + h),
               (cx - w, top + h - 3)], fill=liquid)
    c.line([(cx - lw, ly), (cx + lw, ly)], lighter(liquid, 0.5), 1.4)
    for bx, by, r in ((-0.3, 0.8, 3.0), (0.25, 0.68, 2.2), (0.0, 0.9, 1.6), (0.4, 0.86, 1.4)):
        c.circle(cx + bx * w, top + by * h, r, fill=lighter(liquid, 0.6))
    # bubbles out of the top
    for bx, by, r in ((-2, -6, 3.2), (4, -14, 2.4), (-3, -21, 1.8), (2, -27, 1.4)):
        c.circle(cx + bx, top + by, r + 1, fill=INK)
        c.circle(cx + bx, top + by, r, fill=lighter(liquid, 0.55))
    c.line([(cx - neck + 2.5, top + 4), (cx - neck + 2.5, top + h * 0.36), (cx - w + 7, top + h - 8)], WHITE, 1.6)
    c.rect(cx - neck - 2, top - 2, cx + neck + 2, top + 3, fill=INK, radius=1.5)
    c.rect(cx - neck - 1, top - 1, cx + neck + 1, top + 2, fill=(210, 236, 246), radius=1)


@sheet(901)
def scientist(ctx):
    c = ctx.canvas(1)
    W, H = c.w, c.h
    c.gradient(0, 0, W, H, [(40, 70, 120), (20, 34, 70)])
    c.dither()
    # the chalkboard
    c.rect(6, 8, W - 6, 98, fill=INK, radius=2)
    c.rect(8, 10, W - 8, 96, fill=(150, 96, 50), radius=1.5)
    c.rect(12, 14, W - 12, 92, fill=(28, 74, 54))
    chalk = (226, 236, 226)
    c.ellipse(18, 22, 54, 40, outline=chalk, width=1.2)
    for a in (60, -60):
        x = Xf(c, 36, 31, 1, a)
        pts = [x.p(math.cos(math.radians(t)) * 18, math.sin(math.radians(t)) * 9) for t in range(0, 361, 15)]
        c.line(pts, chalk, 1.2)
    c.circle(36, 31, 2.4, fill=(255, 150, 120))
    c.line([(W - 60 + i, 30 - 9 * math.sin(i / 6)) for i in range(0, 44)], chalk, 1.2)
    c.line([(W - 62, 44), (W - 14, 44)], chalk, 1.0)
    c.line([(W - 60, 18), (W - 60, 46)], chalk, 1.0)
    # the coat
    coat = [(14, H), (24, 146), (48, 128), (126, 128), (150, 146), (160, H)]
    c.polygon(coat, fill=INK, outline=INK, width=4)
    c.polygon(coat, fill=(236, 240, 246))
    c.polygon([(87, 132), (60, 132), (87, H)], fill=(208, 214, 226))
    c.polygon([(87, 132), (114, 132), (87, H)], fill=(208, 214, 226))
    c.polygon([(70, 128), (104, 128), (87, 168)], fill=INK)
    c.polygon([(73, 130), (101, 130), (87, 162)], fill=(130, 180, 230))
    for s in (-1, 1):
        c.line([(87 + s * 15, 128), (87 + s * 4, 172), (87 + s * 22, 150), (87 + s * 25, 136)], INK, 2.0)
    # bow tie
    for s in (-1, 1):
        c.polygon([(87, 138), (87 + s * 12, 131), (87 + s * 12, 146)], fill=INK, outline=INK, width=2)
        c.polygon([(87, 138), (87 + s * 11, 132.5), (87 + s * 11, 144.5)], fill=(220, 40, 40))
    c.circle(87, 138, 3.4, fill=INK)
    c.circle(87, 138, 2.4, fill=(240, 70, 60))
    # the pocket and pens
    c.rect(34, 158, 56, 162, fill=INK)
    for px, col in ((38, (40, 110, 220)), (44, (220, 40, 40)), (50, (40, 170, 70))):
        c.rect(px - 1.6, 148, px + 1.6, 160, fill=INK, radius=1)
        c.rect(px - 0.8, 149, px + 0.8, 160, fill=col, radius=0.6)
    c.rect(32, 160, 58, H, fill=INK)
    c.rect(34, 162, 56, H, fill=(224, 230, 240))
    # the head
    doc_face(Xf(c, 87, 74, 4.4), 2.0, detail=True)
    # the flask, held up in a glove
    flask(c, 142, 98, 50, (90, 230, 110))
    sh = Shapes(Xf(c, 146, 160, 2.4, 0), 1.6)
    cuff(sh, 8.0, 5.2)
    sh.rrect(-6.0, -4.0, 6.0, 9.0, 3.6, GLOVE)
    for i, yy in enumerate((-6.0, -3.0, 0.0)):
        sh.cap((-8.4, yy - 4.5), (1.4, yy - 4.5), 3.0, GLOVE)
    sh.cap((-2.0, 5.0), (-8.6, -6.0), 3.2, GLOVE)
    return [c]


# ---------------------------------------------------------------------------
# Sheet 1048: the dynamite plunger: 1 the handle up, 2 pushed down (on);
# 3 unused. Its output knot at (55, 55).


def plunger(down: bool, knot) -> Canvas:
    c = Canvas(65, 69)
    kx, ky, _ = knot
    bx0, bx1, by0, by1 = 5, 51, 38, 68
    # wires from the terminals
    for tx, col in ((12, (220, 40, 40)), (44, (40, 160, 60))):
        c.rect(tx - 3, by0 - 5, tx + 3, by0 + 1, fill=INK, radius=1)
        c.rect(tx - 2, by0 - 4, tx + 2, by0, fill=STEEL_L, radius=0.6)
        c.line([(tx, by0 - 3), (tx - 4 if tx < 30 else tx + 4, by0 - 6), (tx - 6 if tx < 30 else tx + 7, by0 - 3)], INK, 2.6)
        c.line([(tx, by0 - 3), (tx - 4 if tx < 30 else tx + 4, by0 - 6), (tx - 6 if tx < 30 else tx + 7, by0 - 3)], col, 1.2)
    # the rod and the T-handle
    top = 24 if down else 6
    c.rect(25, top, 31, by0 + 1, fill=INK)
    c.gradient(26, top, 30, by0, [STEEL_L, STEEL, STEEL_D], vertical=False)
    c.rect(9, top - 4, 47, top + 3, fill=INK, radius=3)
    c.gradient(10, top - 3, 46, top + 2, [(250, 150, 90), (200, 80, 40), (150, 50, 20)], radius=2.5)
    c.rect(10, top - 3, 46, top - 2, fill=(255, 200, 150))
    c.rect(24, top - 5, 32, top + 4, fill=INK, radius=1.5)
    c.rect(25, top - 4, 31, top + 3, fill=BRASS, radius=1)
    # the box
    c.panel(bx0, by0, bx1, by1, (200, 60, 40), radius=2, bevel=1.5)
    c.rect(bx0 + 3, by0 + 5, bx1 - 3, by1 - 5, fill=INK)
    for x in range(bx0 + 4, bx1 - 3):
        for y in range(by0 + 6, by1 - 5):
            c.rect(x, y, x + 1, y + 1, fill=(255, 210, 30) if ((x - y) // 3) % 2 == 0 else (40, 34, 30))
    c.rect(bx0 + 12, by0 + 9, bx1 - 12, by1 - 9, fill=INK, radius=1.5)
    c.rect(bx0 + 13, by0 + 10, bx1 - 13, by1 - 10, fill=(255, 236, 200), radius=1)
    cx, cy = (bx0 + bx1) / 2, (by0 + by1) / 2
    bolt = [(cx + 2.5, cy - 6), (cx - 3, cy + 0.8), (cx - 0.2, cy + 0.8), (cx - 2.5, cy + 6), (cx + 3, cy - 0.8),
            (cx + 0.2, cy - 0.8)]
    c.polygon(bolt, fill=INK, outline=INK, width=1.2)
    c.polygon(bolt, fill=(240, 40, 30))
    for sx in (bx0 + 3, bx1 - 3):
        for sy in (by0 + 2.6, by1 - 2.6):
            c.screw(sx, sy, 1.3)
    if down:
        for dx in (-1, 1):
            c.line([(28 + dx * 22, top - 6), (28 + dx * 25, top - 10)], WHITE, 1.2)
            c.line([(28 + dx * 21, top + 1), (28 + dx * 26, top + 1)], WHITE, 1.2)
    c.knot(kx, ky, True, "r", reach=6)
    return c


# Sheet 1048 is drawn in science.py; this one is a spare.
def dynamite_plunger(ctx):
    k = ctx.knots()[0]
    return [plunger(False, k), plunger(True, k), ctx.blank(3)]


# ---------------------------------------------------------------------------
# The About box: 10001 the machine round the black scrolling window
# (36,97)-(310,210); 10004 the title that scrolls up first (the version is
# written over it at y 66..84); 10002 and 10003, the original's credits
# pictures, two lab pictures of our own.

SCROLL = (36, 97, 310, 210)
GLOW = (143, 199, 247)


def gear(c: Canvas, cx, cy, r, teeth, col, rot=0.0, hole=True, o=1.5):
    pts = []
    for i in range(teeth * 4):
        a = math.radians(rot + 360 * i / (teeth * 4))
        rr = r if (i % 4) in (1, 2) else r * 0.8
        pts.append((cx + math.cos(a) * rr, cy + math.sin(a) * rr))
    c.polygon(pts, fill=INK, outline=INK, width=o * 2)
    c.polygon(pts, fill=col)
    c.circle(cx, cy, r * 0.62, fill=darker(col, 0.2))
    c.circle(cx - r * 0.06, cy - r * 0.06, r * 0.55, fill=lighter(col, 0.15))
    if hole:
        c.circle(cx, cy, r * 0.22 + o, fill=INK)
        c.circle(cx, cy, r * 0.22, fill=darker(col, 0.55))


def bolt_path(x0, y0, x1, y1, n, amp, seed):
    rnd = random.Random(seed)
    pts = [(x0, y0)]
    for i in range(1, n):
        t = i / n
        pts.append((x0 + (x1 - x0) * t + rnd.uniform(-amp, amp) * (y1 - y0 != 0),
                    y0 + (y1 - y0) * t + rnd.uniform(-amp, amp) * (x1 - x0 != 0)))
    pts.append((x1, y1))
    return pts


def lightning(c, pts, w=1.0):
    c.line(pts, (60, 110, 255), 4.5 * w)
    c.line(pts, (150, 200, 255), 2.4 * w)
    c.line(pts, WHITE, 1.0 * w)


def tesla(c: Canvas, cx, top, bottom, r=10):
    """A Tesla coil: a base, the wound column and the ball on top."""
    c.rect(cx - 16, bottom - 12, cx + 16, bottom, fill=INK, radius=2)
    c.gradient(cx - 15, bottom - 11, cx + 15, bottom - 1, [(120, 80, 50), (70, 44, 26)], radius=1.5)
    c.rect(cx - 8, top + r - 2, cx + 8, bottom - 11, fill=INK)
    c.gradient(cx - 7, top + r - 2, cx + 7, bottom - 11, [(150, 70, 30), (250, 160, 90), (150, 70, 30)], vertical=False)
    for y in range(int(top + r), int(bottom - 12), 2):
        c.line([(cx - 7, y), (cx + 7, y)], darker(COPPER_C, 0.4), 0.5, round_ends=False)
    c.circle(cx, top, r + 1.4, fill=INK)
    c.radial(cx, top, r, (250, 252, 255), STEEL_D, -0.45, -0.45)


COPPER_C = (196, 104, 52)


def dial(c, cx, cy, r, needle, face=WHITE):
    c.circle(cx, cy, r + 1.5, fill=INK)
    c.radial(cx, cy, r, BRASS_L, BRASS_D, -0.5, -0.5)
    c.circle(cx, cy, r * 0.78, fill=INK)
    c.circle(cx, cy, r * 0.72, fill=face)
    for a in range(-135, 136, 27):
        t = math.radians(a - 90)
        c.line([(cx + math.cos(t) * r * 0.55, cy + math.sin(t) * r * 0.55), (cx + math.cos(t) * r * 0.66, cy + math.sin(t) * r * 0.66)],
               INK, 0.8, round_ends=False)
    c.arc(cx, cy, r * 0.6, 30, 45, (220, 40, 40), 1.6)
    t = math.radians(needle - 90)
    c.line([(cx, cy), (cx + math.cos(t) * r * 0.62, cy + math.sin(t) * r * 0.62)], (220, 30, 30), 1.2)
    c.circle(cx, cy, 1.6, fill=INK)


def lamp(c, cx, cy, r, col, lit=True):
    c.circle(cx, cy, r + 1.2, fill=INK)
    c.radial(cx, cy, r, lighter(col, 0.8) if lit else lighter(col, 0.2), col if lit else darker(col, 0.5), -0.4, -0.4)


@sheet(10001)
def about_frame(ctx):
    c = ctx.canvas(1)
    W, H = c.w, c.h
    x0, y0, x1, y1 = SCROLL
    base = (58, 92, 138)
    c.panel(0, 0, W, H, base, radius=6, bevel=2)
    # rivets round the edge
    for x in range(14, W - 8, 24):
        for y in (6, H - 6):
            c.screw(x, y, 1.5)
    for y in range(30, H - 20, 24):
        for x in (6, W - 6):
            c.screw(x, y, 1.5)
    # the screen's bezel
    c.rect(x0 - 10, y0 - 10, x1 + 10, y1 + 10, fill=INK, radius=6)
    c.gradient(x0 - 9, y0 - 9, x1 + 9, y1 + 9, [STEEL_L, STEEL, STEEL_D], radius=5)
    c.rect(x0 - 5, y0 - 5, x1 + 5, y1 + 5, fill=darker(STEEL_D, 0.3), radius=3)
    c.rect(x0 - 4, y0 - 4, x1 + 5, y1 + 5, fill=STEEL_L, radius=3)
    c.rect(x0 - 4, y0 - 4, x1 + 4, y1 + 4, fill=INK, radius=3)
    c.rect(x0, y0, x1, y1, fill=(0, 0, 0))
    for sx in (x0 - 6, x1 + 6):
        for sy in (y0 - 6, y1 + 6):
            c.screw(sx, sy, 1.8)
    # the Tesla coils and the spark between them
    tesla(c, 32, 30, 84)
    tesla(c, W - 32, 30, 84)
    lightning(c, bolt_path(42, 28, W - 42, 28, 14, 7, 5))
    lightning(c, bolt_path(80, 30, 140, 46, 4, 4, 9), 0.6)
    # dials and a row of lamps on a brass plate
    dial(c, 86, 62, 16, 25)
    dial(c, W - 86, 62, 16, -40)
    c.rect(116, 50, W - 116, 76, fill=INK, radius=4)
    c.gradient(117, 51, W - 117, 75, [BRASS_L, BRASS, BRASS_D], radius=3.5)
    for i, col in enumerate(((240, 50, 40), (255, 200, 30), (60, 220, 90), (60, 150, 255), (255, 120, 30))):
        lamp(c, 136 + i * 18, 63, 5, col, lit=i != 2)
    # pipes down the sides
    for px in (10, W - 21):
        c.rect(px, 92, px + 11, 216, fill=INK)
        c.gradient(px + 1, 92, px + 10, 216, [STEEL_D, STEEL_L, WHITE, STEEL_L, STEEL_D], vertical=False)
        for py in (96, 150, 206):
            c.rect(px - 1, py, px + 12, py + 4, fill=INK)
            c.gradient(px, py + 1, px + 11, py + 3, [BRASS_D, BRASS_L, BRASS_D], vertical=False)
        c.rect(px + 1, 112, px + 10, 140, fill=INK)
        c.rect(px + 2, 113, px + 9, 139, fill=(200, 230, 240))
        c.rect(px + 2, 122, px + 9, 139, fill=(80, 220, 100))
        c.circle(px + 4.5, 132, 1.3, fill=WHITE)
        c.circle(px + 6.5, 126, 1.0, fill=WHITE)
    # the desk: knobs, a big red button, levers and sliders
    c.rect(24, 226, W - 24, H - 12, fill=INK, radius=4)
    c.gradient(25, 227, W - 25, H - 13, [lighter(base, 0.3), base, darker(base, 0.3)], radius=3.5)
    for i, kx in enumerate((50, 80)):
        c.circle(kx, 244, 10, fill=INK)
        c.radial(kx, 244, 9, STEEL_L, STEEL_D, -0.4, -0.4)
        a = math.radians((-60, 40)[i] - 90)
        c.line([(kx, 244), (kx + math.cos(a) * 7, 244 + math.sin(a) * 7)], INK, 1.6)
    c.circle(W / 2, 246, 15, fill=INK)
    c.circle(W / 2, 246, 13.5, fill=(160, 160, 170))
    c.circle(W / 2, 244, 11, fill=INK)
    c.radial(W / 2, 244, 10, (255, 150, 140), (180, 20, 20), -0.4, -0.5)
    for i in range(4):
        sx = 220 + i * 18
        c.rect(sx - 2, 232, sx + 2, 256, fill=INK, radius=1.5)
        c.rect(sx - 1, 233, sx + 1, 255, fill=(20, 26, 34))
        ky = 236 + (i * 7) % 16
        c.rect(sx - 5, ky, sx + 5, ky + 5, fill=INK, radius=1)
        c.rect(sx - 4, ky + 1, sx + 4, ky + 4, fill=(230, 232, 240), radius=0.8)
    for i, (lx, up) in enumerate(((110, True), (126, False))):
        c.rect(lx - 5, 248, lx + 5, 256, fill=INK, radius=1.5)
        c.rect(lx - 4, 249, lx + 4, 255, fill=STEEL_D, radius=1)
        tip = (lx + (6 if up else -6), 232)
        c.line([(lx, 252), tip], INK, 3)
        c.line([(lx, 252), tip], STEEL_L, 1.4)
        c.circle(tip[0], tip[1], 3.4, fill=INK)
        c.circle(tip[0], tip[1], 2.6, fill=(220, 40, 40) if up else (40, 120, 230))
    c.dither(0, 0, W, 90)
    return [c]


@sheet(10004)
def about_title(ctx):
    c = ctx.canvas(1)
    W, H = c.w, c.h
    c.rect(0, 0, W, H, fill=(0, 0, 0))
    # a little emblem: a gear and a flask
    gear(c, W / 2 - 7, 22, 12, 9, BRASS, rot=8)
    flask(c, W / 2 + 9, 8, 26, (90, 230, 110), o=1.2)
    c.text(W / 2, 50, "OpenWGT", 24, (255, 214, 70), "serif", shadow=(1, 1, (150, 80, 0)))
    # (y 66..84: the version, the code's)
    c.text(W / 2, 93, tr("for Widget Workshop"), 11, GLOW, "sans-book")
    c.text(W / 2, 106, "OpenWGFX + OpenWSFX", 10, GLOW, "sans-book")
    # a contraption: three gears turning a crank that lights a bulb
    gear(c, 70, 172, 26, 12, BRASS, rot=4)
    gear(c, 114, 196, 16, 9, STEEL, rot=12)
    gear(c, 148, 168, 20, 10, COPPER_C, rot=0)
    c.line([(148, 168), (186, 150)], INK, 4)
    c.line([(148, 168), (186, 150)], STEEL_L, 2)
    c.line([(186, 150), (212, 156)], INK, 3)
    c.line([(186, 150), (212, 156)], (220, 40, 40), 1.4)
    c.line([(212, 156), (226, 140)], INK, 3)
    c.line([(212, 156), (226, 140)], (220, 40, 40), 1.4)
    c.circle(226, 128, 13, fill=(80, 70, 10))
    c.circle(226, 128, 11, fill=INK)
    c.radial(226, 128, 10, (255, 255, 220), (255, 200, 30), -0.4, -0.4)
    c.rect(220, 138, 232, 146, fill=INK, radius=1)
    c.rect(221, 139, 231, 145, fill=STEEL_L)
    for a in range(-150, 31, 30):
        t = math.radians(a)
        c.line([(226 + math.cos(t) * 15, 128 + math.sin(t) * 15), (226 + math.cos(t) * 20, 128 + math.sin(t) * 20)],
               (255, 220, 60), 1.4)
    c.rect(30, 214, W - 30, 218, fill=STEEL_D, radius=1)
    return [c]


@sheet(10002)
def about_coil(ctx):
    c = ctx.canvas(1)
    W, H = c.w, c.h
    c.rect(0, 0, W, H, fill=(0, 0, 0))
    cx = W / 2
    # a big Tesla coil throwing sparks
    for i, (ex, ey) in enumerate(((20, 40), (W - 18, 70), (30, 150), (W - 30, 20), (W - 24, 170))):
        lightning(c, bolt_path(cx, 92, ex, ey, 7, 9, i + 1), 0.9)
    c.rect(cx - 58, H - 46, cx + 58, H - 14, fill=INK, radius=4)
    c.gradient(cx - 57, H - 45, cx + 57, H - 15, [(130, 86, 50), (70, 44, 26)], radius=3.5)
    c.rect(cx - 26, 108, cx + 26, H - 44, fill=INK)
    c.gradient(cx - 25, 108, cx + 25, H - 44, [(130, 60, 24), (250, 170, 100), (130, 60, 24)], vertical=False)
    for y in range(112, H - 46, 3):
        c.line([(cx - 25, y), (cx + 25, y)], darker(COPPER_C, 0.45), 0.7, round_ends=False)
    c.ellipse(cx - 70, 70, cx + 70, 120, fill=INK)
    c.gradient(cx - 68, 72, cx + 68, 118, [WHITE, STEEL_L, STEEL, STEEL_D])
    c.ellipse(cx - 70, 70, cx + 70, 120, outline=INK, width=2)
    c.ellipse(cx - 40, 76, cx + 10, 90, fill=WHITE)
    for sx in (cx - 44, cx + 44):
        c.screw(sx, H - 30, 3)
    c.dither()
    return [c]


@sheet(10003)
def about_chemistry(ctx):
    c = ctx.canvas(1)
    W, H = c.w, c.h
    c.rect(0, 0, W, H, fill=(0, 0, 0))
    # a stand with a burner, a flask boiling and a coil of glass tubing
    c.rect(30, H - 40, W - 30, H - 26, fill=INK, radius=3)
    c.gradient(31, H - 39, W - 31, H - 27, [STEEL_L, STEEL_D], radius=2.5)
    c.rect(60, 60, 68, H - 38, fill=INK)
    c.gradient(61, 60, 67, H - 38, [STEEL_L, STEEL_D], vertical=False)
    c.rect(64, 196, 200, 204, fill=INK)
    c.gradient(64, 197, 200, 203, [STEEL_L, STEEL_D])
    c.rect(118, 300, 166, H - 38, fill=INK, radius=3)
    c.gradient(119, 301, 165, H - 39, [STEEL_L, STEEL, STEEL_D], vertical=False, radius=2.5)
    for i, (fx, fy, r) in enumerate(((142, 296, 16), (136, 284, 10), (148, 280, 8))):
        c.ellipse(fx - r * 0.6, fy - r * 1.5, fx + r * 0.6, fy, fill=(255, 140, 30) if i == 0 else (255, 230, 80))
    flask(c, 142, 120, 150, (180, 60, 230), o=2.5)
    # tubing out of the flask's neck, round a coil, into a beaker
    c.line([(142, 114), (142, 70), (220, 70), (220, 120)], INK, 7)
    c.line([(142, 114), (142, 70), (220, 70), (220, 120)], (200, 236, 246), 4)
    for i in range(5):
        y = 128 + i * 22
        c.ellipse(196, y - 7, 244, y + 7, outline=INK, width=6)
        c.ellipse(196, y - 7, 244, y + 7, outline=(200, 236, 246), width=3)
    c.rect(198, 270, 242, H - 40, fill=INK, radius=3)
    c.rect(201, 270, 239, H - 43, fill=(200, 236, 246), radius=2)
    c.rect(201, 316, 239, H - 43, fill=(180, 60, 230), radius=2)
    for bx, by in ((210, 330), (226, 345), (218, 360)):
        c.circle(bx, by, 3, fill=lighter((180, 60, 230), 0.6))
    # gears on the wall
    gear(c, 40, 30, 22, 11, BRASS)
    gear(c, 76, 48, 13, 8, COPPER_C, rot=10)
    return [c]
