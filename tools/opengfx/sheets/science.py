"""The science lab and its neighbours: the Pendulum (1000), the Gravity
Chamber (1001), the Cannon (1002), the Coin Toss (1006), the Random
Number Generator (1007), the Tuning Fork (1008), the Keypress (1010), the
Metronome (1011), the Solar System (1022), the Multiplexer (1036), the
Speaker (1038), the Mouse Distance Measurer (1039), the Dynamite Plunger
(1048), the Graphic Projector (1050) with its picture sets (200 to 205)
and the Win / Lose Checker (1051)."""
from __future__ import annotations

import math
import random

from wgfx import (tr, BRASS, BRASS_D, BRASS_L, COPPER, GLASS, INK, KNOT_OUT, KNOT_OUT_D, KNOT_OUT_L, STEEL,
                  STEEL_D, STEEL_L, WHITE, Canvas, darker, lighter, mix, sheet)

from ._science_kit import clip_hi, cut, mask_of, poly_mask, ptext, ptext_width, stamp

RED = (220, 40, 30)
IVORY = (240, 232, 210)
WOOD = (150, 92, 44)
GREEN = (60, 190, 80)


def blanks(ctx, frames, n0):
    """`frames` then blank (1×1) frames up to the sheet's count."""
    return frames + [ctx.blank(n) for n in range(n0, ctx.count + 1)]


def knots(c, ks, sides=None, reach=5.0):
    """The knots, each with a stalk `reach` long into the body."""
    for i, (x, y, out) in enumerate(ks):
        c.knot(x, y, out, sides[i] if sides else c.guess_side(x, y), reach)


def note(c, x, y, s=1.0, col=INK, flag=True):
    """A musical note, its head's centre at (x, y)."""
    c.ellipse(x - 2.4 * s, y - 1.7 * s, x + 2.4 * s, y + 1.7 * s, fill=col)
    c.line([(x + 2.0 * s, y), (x + 2.0 * s, y - 9 * s)], col, 0.9 * s, round_ends=False)
    if flag:
        c.line([(x + 2.0 * s, y - 9 * s), (x + 5 * s, y - 6 * s), (x + 4.5 * s, y - 3.5 * s)], col, 0.9 * s)


# ---------------------------------------------------------------------------
# 1010 Keypress: a key, up (1) and pressed (2); the game writes the key's
# letter over it, centred in (0, 0, 39, 36).


def keycap(c, down):
    c.rect(0, 1, 40, 40, fill=INK, radius=3)
    c.gradient(1, 2, 39, 39, [STEEL_D, darker(STEEL_D, 0.4)], radius=2)
    c.screw(4, 35.5, 1.3)
    c.screw(35.5, 35.5, 1.3)
    if not down:
        # sides of a tall cap, then its dished top
        c.polygon([(2, 1), (38, 1), (38, 34), (34, 37), (6, 37), (2, 34)], fill=INK)
        c.polygon([(3, 2), (37, 2), (37, 33), (33.5, 36), (6.5, 36), (3, 33)], fill=darker(IVORY, 0.32))
        c.polygon([(3, 2), (37, 2), (33, 6), (7, 6)], fill=lighter(IVORY, 0.5))
        c.polygon([(3, 2), (7, 6), (7, 29), (3, 33)], fill=darker(IVORY, 0.12))
        c.rect(7, 5, 33, 30, fill=INK, radius=3)
        c.gradient(7.5, 5.5, 32.5, 29.5, [lighter(IVORY, 0.4), IVORY, darker(IVORY, 0.08)], radius=2.5)
    else:
        c.rect(4, 5, 36, 37, fill=INK, radius=3)
        c.polygon([(5, 6), (35, 6), (35, 35), (5, 35)], fill=darker(IVORY, 0.4))
        c.polygon([(5, 6), (35, 6), (33, 8), (7, 8)], fill=darker(IVORY, 0.1))
        c.rect(7, 7, 33, 33, fill=INK, radius=3)
        c.gradient(7.5, 7.5, 32.5, 32.5, [IVORY, darker(IVORY, 0.12)], radius=2.5)


@sheet(1010)
def keypress(ctx):
    out = []
    for down in (False, True):
        c = ctx.canvas(1)
        keycap(c, down)
        knots(c, ctx.knots(), reach=4)
        out.append(c)
    return blanks(ctx, out, 3)


# ---------------------------------------------------------------------------
# 1039 Mouse Distance Measurer: a mouse, with no, one and two pairs of
# speed arrows.


def mouse_body():
    m = Canvas(24, 34)
    m.rect(0, 0, 24, 34, fill=INK, radius=11)
    m.gradient(1, 1, 23, 33, [(230, 232, 238), (178, 182, 194), (120, 124, 138)], vertical=False, radius=10)
    m.gradient(4, 2, 12, 30, [(0.0, (250, 250, 255)), (1.0, (200, 204, 214))], vertical=False, radius=4)
    m.line([(1.5, 12), (22.5, 12)], INK, 0.9)
    m.line([(12, 1), (12, 12)], INK, 0.9)
    m.rect(10.3, 4, 13.7, 9.5, fill=(70, 74, 86), radius=1.6)
    m.rect(11.2, 5, 12.8, 8.0, fill=(150, 156, 170), radius=0.8)
    return m


def arrow(c, x, y, ang, length, col=RED):
    """A fat arrow from (x, y) `length` long towards `ang` degrees (0 =
    right, 90 = up)."""
    a = math.radians(ang)
    ux, uy = math.cos(a), -math.sin(a)
    px, py = -uy, ux
    tip = (x + ux * length, y + uy * length)
    hb = (x + ux * (length - 6), y + uy * (length - 6))
    w, hw = 2.0, 4.8
    pts = [(x + px * w, y + py * w), (hb[0] + px * w, hb[1] + py * w), (hb[0] + px * hw, hb[1] + py * hw), tip,
           (hb[0] - px * hw, hb[1] - py * hw), (hb[0] - px * w, hb[1] - py * w), (x - px * w, y - py * w)]
    c.polygon(pts, fill=INK, outline=INK, width=2.0)
    c.polygon(pts, fill=col)


@sheet(1039)
def mouse_move(ctx):
    out = []
    k = ctx.knots()
    for n in range(3):
        c = ctx.canvas(1)
        cx, cy = 26, 26
        if n:
            for ang, (ox, oy) in ((135, (-9, -9)), (-45, (9, 9))):
                arrow(c, cx + ox, cy + oy, ang, 10 if n == 1 else 18)
        # the cable, from the mouse's nose to the lamp
        cable = [(cx + 10, cy - 12), (cx + 15, cy - 18), (k[0][0] - 2, k[0][1] + 3)]
        c.line(cable, INK, 2.4)
        c.line(cable, (90, 94, 110), 1.1)
        stamp(c, mouse_body(), cx, cy, -40, 1.15)
        knots(c, k, ["b"], reach=0)
        out.append(c)
    return blanks(ctx, out, 4)


# ---------------------------------------------------------------------------
# 1048 Dynamite Plunger: the handle up (1) and pushed down (2).


@sheet(1048)
def dynamite_plunger(ctx):
    out = []
    for down in (False, True):
        c = ctx.canvas(1)
        bar = 26 if down else 12
        # terminals with their wires
        for tx, wc in ((12, RED), (50, (40, 150, 60))):
            c.rect(tx - 3, 33, tx + 4, 42, fill=INK, radius=1)
            c.gradient(tx - 2, 34, tx + 3, 41, [BRASS_L, BRASS, BRASS_D], vertical=False)
            c.rect(tx - 4, 35, tx + 5, 37, fill=INK)
            c.rect(tx - 3, 35.5, tx + 4, 36.5, fill=wc)
        # the shaft
        c.rect(30, bar + 3, 35, 42, fill=INK)
        c.gradient(31, bar + 3, 34, 42, [STEEL_L, STEEL, STEEL_D], vertical=False)
        # the box
        c.panel(7, 40, 57, 69, (200, 40, 30), radius=2)
        c.rect(9, 42.5, 55, 47, fill=INK)
        for i in range(-1, 12):
            x = 9 + i * 5
            c.polygon([(x, 47), (x + 2.5, 47), (x + 7, 42.5), (x + 4.5, 42.5)], fill=(250, 210, 40))
        c.clear(0, 40, 7, 69)
        c.clear(57, 40, 65, 69)
        c.rect(7, 40, 9, 69, fill=INK)
        c.rect(55, 40, 57, 69, fill=INK)
        c.rect(26, 38, 39, 42, fill=INK, radius=1)
        c.gradient(27, 38.7, 38, 41.3, [STEEL_L, STEEL_D])
        c.text(32, 57.5, "TNT", 15, (255, 220, 60), "sans", stroke=1.3, stroke_fill=INK)
        for sx in (10.5, 53.5):
            c.screw(sx, 65.5, 1.3)
        # the T handle
        c.rect(14, bar, 51, bar + 6, fill=INK, radius=3)
        c.gradient(15, bar + 1, 50, bar + 5, [lighter(WOOD, 0.4), WOOD, darker(WOOD, 0.3)], radius=2)
        for hx in (14, 46):
            c.rect(hx, bar, hx + 5, bar + 6, fill=INK, radius=2)
            c.gradient(hx + 0.8, bar + 0.8, hx + 4.2, bar + 5.2, [BRASS_L, BRASS, BRASS_D], radius=1.5)
        c.rect(30, bar - 1, 35, bar + 7, fill=INK, radius=1)
        c.gradient(31, bar, 34, bar + 6, [STEEL_L, STEEL_D], vertical=False)
        if down:
            for sx, sy, ex, ey in ((22, 22, 19, 18), (43, 22, 46, 18), (32.5, 20, 32.5, 16)):
                c.line([(sx, sy), (ex, ey)], (255, 210, 40), 1.2)
        knots(c, ctx.knots(), ["r"], reach=4)
        out.append(c)
    return blanks(ctx, out, 3)


# ---------------------------------------------------------------------------
# 1038 Speaker: a loudspeaker with its input box.


@sheet(1038)
def speaker(ctx):
    c = ctx.canvas(1)
    cx, cy, r = 51.5, 29.5, 27
    c.circle(cx, cy, r, fill=INK)
    c.radial(cx, cy, r - 1, (120, 126, 140), (52, 56, 66), -0.4, -0.4)
    for i in range(6):
        a = math.radians(i * 60 + 30)
        c.screw(cx + math.cos(a) * (r - 3.5), cy + math.sin(a) * (r - 3.5), 1.3, i * 20)
    c.circle(cx, cy, r - 6.5, fill=INK)
    c.radial(cx, cy, r - 7.5, (40, 42, 50), (96, 100, 112), 0.0, 0.0)
    for rr in (16, 12):
        c.arc(cx, cy, rr, 0, 360, darker((60, 62, 72), 0.4), 0.8)
        c.arc(cx - 0.6, cy - 0.6, rr, 180, 300, (130, 134, 146), 0.6)
    c.circle(cx, cy, 7.5, fill=INK)
    c.radial(cx, cy, 6.8, (220, 224, 232), (90, 96, 108), -0.45, -0.45)
    # the input box, a grille
    x0, y0, x1, y1 = 10, 4, 45, 27
    c.panel(x0, y0, x1, y1, STEEL, radius=2)
    c.inset(x0 + 3, y0 + 3, x1 - 3, y1 - 3, (30, 32, 40), radius=1)
    for gy in range(y0 + 5, y1 - 4, 2):
        for gx in range(x0 + 5 + (gy % 4 == 0), x1 - 4, 2):
            c.pixel(gx, gy, (110, 116, 130))
    knots(c, ctx.knots(), ["l"], reach=6)
    return blanks(ctx, [c], 2)


# ---------------------------------------------------------------------------
# 1011 Metronome: the arm swung left (1), upright (2) and right (3); the
# game shows 3, 2, 1, 2.

WALNUT = (120, 64, 34)


@sheet(1011)
def metronome(ctx):
    out = []
    for n, ang in ((1, -22), (2, 0), (3, 22)):
        c = ctx.canvas(1)
        # the case: a pyramid on a plinth
        c.rect(2, 62, 59, 87, fill=INK, radius=2)
        c.gradient(3, 63, 58, 86, [lighter(WALNUT, 0.3), WALNUT, darker(WALNUT, 0.35)], radius=1.5)
        c.rect(3, 63, 58, 65, fill=lighter(WALNUT, 0.45))
        c.rect(6, 87, 12, 90, fill=INK, radius=1)
        c.rect(48, 87, 54, 90, fill=INK, radius=1)
        tri = [(26, 2), (35, 2), (54, 63), (7, 63)]
        c.polygon(tri, fill=INK, outline=INK, width=1.2)
        c.gradient(7, 3, 55, 63, [lighter(WALNUT, 0.35), WALNUT, darker(WALNUT, 0.3)], vertical=False, mask=poly_mask(c, [(26.6, 3), (34.4, 3), (53, 62.5), (8, 62.5)]))
        win = [(27.5, 8), (33.5, 8), (45, 61), (16, 61)]
        c.polygon(win, fill=INK)
        c.gradient(16, 9, 45, 61, [(28, 22, 30), (54, 44, 56)], mask=poly_mask(c, [(28.2, 9), (32.8, 9), (44, 60.5), (17, 60.5)]))
        # the scale
        c.rect(29, 12, 32, 58, fill=darker(IVORY, 0.15))
        for y in range(14, 57, 3):
            c.line([(29, y), (32, y)], INK, 0.5, round_ends=False)
        c.rect(26, 0, 35, 4, fill=INK, radius=1.5)
        c.gradient(27, 0.8, 34, 3.2, [BRASS_L, BRASS_D], radius=1)
        # the arm, its weight and its pivot
        px, py = 30.5, 60.0
        a = math.radians(ang)
        tx, ty = px + math.sin(a) * 52, py - math.cos(a) * 52
        c.line([(px, py), (tx, ty)], INK, 2.8)
        c.line([(px, py), (tx, ty)], STEEL_L, 1.2)
        wx, wy = px + math.sin(a) * 33, py - math.cos(a) * 33
        wt = Canvas(9, 8)
        wt.polygon([(0.5, 0.5), (8.5, 0.5), (6.5, 7.5), (2.5, 7.5)], fill=INK)
        wt.gradient(1, 1, 8, 7, [BRASS_L, BRASS, BRASS_D], vertical=False, mask=poly_mask(wt, [(1.3, 1.2), (7.7, 1.2), (6, 6.8), (3, 6.8)]))
        stamp(c, wt, wx, wy, -ang)
        c.circle(px, py, 2.6, fill=INK)
        c.circle(px, py, 1.7, fill=BRASS)
        # a brass name plate and the winding key
        c.rect(18, 70, 43, 80, fill=INK, radius=1.5)
        c.gradient(19, 71, 42, 79, [BRASS_L, BRASS, BRASS_D], radius=1)
        for i, x in enumerate(range(22, 40, 4)):
            c.line([(x, 75), (x + 2, 75)], BRASS_D, 0.8)
        for sx in (6, 55):
            c.screw(sx, 75, 1.4)
        knots(c, ctx.knots(), ["r"], reach=5)
        out.append(c)
    return blanks(ctx, out, 4)


# ---------------------------------------------------------------------------
# 1008 Tuning Fork: at rest (1), ringing (2 to 4); 5 and 6 are left over
# (drawn as the last ring dying away and at rest).

CARD = (250, 246, 232)


def fork(c, cx, top, spread, ghost=0.0):
    """A tuning fork, its prongs `spread` pixels further out than at rest."""
    for k, col in (((2.6, (214, 220, 232)), (1.4, (180, 188, 204))) if ghost else ()):
        for side in (-1, 1):
            for sgn in (-1, 1):
                x = cx + side * (6 + spread + sgn * k * ghost)
                c.line([(x, top + 1), (x, top + 26), (cx + side * 4, top + 35)], col, 2.6)
    for side in (-1, 1):
        x = cx + side * (6 + spread)
        c.line([(x, top), (x, top + 30), (cx + side * 3, top + 37)], INK, 4.4)
    c.line([(cx, top + 37), (cx, top + 56)], INK, 4.4)
    for side in (-1, 1):
        x = cx + side * (6 + spread)
        c.line([(x, top), (x, top + 30), (cx + side * 3, top + 37)], STEEL_L, 2.2)
        c.line([(x - 0.6, top + 1), (x - 0.6, top + 28)], WHITE, 0.6)
    c.arc(cx, top + 33, 3, 0, 180, STEEL_L, 2.2)
    c.line([(cx, top + 37), (cx, top + 56)], STEEL, 2.2)
    c.line([(cx - 0.5, top + 38), (cx - 0.5, top + 55)], STEEL_L, 0.7)
    c.ellipse(cx - 4, top + 54, cx + 4, top + 59, fill=INK)
    c.ellipse(cx - 3, top + 55, cx + 3, top + 58, fill=STEEL_D)


@sheet(1008)
def tuning_fork(ctx):
    out = []
    looks = [(0, 0, 0), (1.0, 1.4, 1), (0.6, 1.8, 2), (1.2, 1.2, 3), (0.3, 0.7, 1), (0, 0, 0)]
    for spread, ghost, waves in looks:
        c = ctx.canvas(1)
        c.panel(8, 0, 65, 65, BRASS, radius=3)
        c.rect(11, 3, 62, 62, fill=INK, radius=1.5)
        c.gradient(12, 4, 61, 61, [WHITE, CARD, darker(CARD, 0.08)], radius=1)
        for (nx, ny, s, fl) in ((17, 22, 1.0, True), (15, 44, 0.9, False), (54, 18, 0.9, False), (55, 42, 1.0, True)):
            note(c, nx, ny, s, (64, 70, 92), fl)
        for i in range(waves):
            r = 4 + i * 3.5
            for side, (a0, a1) in ((-1, (145, 215)), (1, (-35, 35))):
                c.arc(36.5 + side * 7, 12, r, a0, a1, (240, 110, 20), 1.1)
        fork(c, 36.5, 5, spread, ghost)
        c.rect(26, 58, 47, 62, fill=INK, radius=1)
        c.gradient(27, 58.6, 46, 61.4, [lighter(WOOD, 0.3), darker(WOOD, 0.2)])
        knots(c, ctx.knots(), ["l", "r"], reach=5)
        out.append(c)
    return blanks(ctx, out, 7)


# ---------------------------------------------------------------------------
# 1006 Coin Toss: a hand flicks a coin (1 to 5); 7 and 8 are HEADS and
# TAILS, which the game puts in the window of frame 1 at P[0].

SKIN = (236, 172, 120)


def hand(c, thumb):
    """A fist from below, knuckles to the right, the thumb "rest",
    "cocked" or "up"."""
    sk, skd = SKIN, darker(SKIN, 0.3)
    c.rect(15, 31, 47, 52, fill=INK, radius=8)
    c.gradient(16, 32, 46, 52, [lighter(sk, 0.2), sk, skd], radius=7)
    # curled fingers on the front
    for i, y in enumerate((33, 38.5, 44)):
        c.rect(36, y, 49.5, y + 6, fill=INK, radius=3)
        c.gradient(37, y + 0.8, 48.7, y + 5.2, [lighter(sk, 0.15), sk, skd], radius=2.4)
        c.line([(46.2, y + 2), (47.2, y + 3.2)], darker(sk, 0.45), 0.6)
    c.line([(25, 46), (33, 45)], skd, 0.7)
    c.line([(16, 40), (16, 52)], darker(sk, 0.15), 1)
    if thumb == "rest":
        pts = [(19, 33), (30, 30.5), (40, 31)]
    elif thumb == "cocked":
        pts = [(19, 33), (31, 33), (37, 36)]
    else:
        pts = [(19, 33), (30, 28), (38, 20)]
    c.line(pts, INK, 6.2)
    c.line(pts, sk, 4.2)
    c.line([(p[0] - 0.3, p[1] - 1.1) for p in pts[1:]], lighter(sk, 0.4), 1.0)
    tx, ty = pts[-1]
    c.circle(tx + 0.3, ty - 0.3, 1.4, fill=lighter(sk, 0.55))


def coin(c, x, y, squash):
    """A brass coin seen from `squash` (1 face on, 0 edge on)."""
    rx, ry = 4.2, max(0.9, 4.2 * squash)
    c.ellipse(x - rx - 0.8, y - ry - 0.8, x + rx + 0.8, y + ry + 0.8, fill=INK)
    c.ellipse(x - rx, y - ry, x + rx, y + ry, fill=BRASS)
    if squash > 0.4:
        c.ellipse(x - rx + 1.3, y - ry + 1.3, x + rx - 1.3, y + ry - 1.3, fill=BRASS_L)
        c.ellipse(x - rx + 2.2, y - ry + 2.0, x + rx - 1.6, y + ry - 1.6, fill=BRASS)


@sheet(1006)
def coin_toss(ctx):
    out = []
    steps = [("rest", (42, 28.5, 0.35)), ("cocked", (38, 32, 0.35)), ("up", (43, 13, 0.15)),
             ("up", (44, 5, 0.9)), ("rest", None)]
    for n, (th, cn) in enumerate(steps, 1):
        c = ctx.canvas(1)
        c.rect(9, 0, 61, 47, fill=INK, radius=2)
        c.gradient(10, 1, 60, 46, [(150, 214, 250), (210, 238, 250), (180, 214, 150)])
        c.dither(10, 1, 60, 46)
        if n == 1:
            c.rect(16, 1, 55, 17, fill=INK, radius=1.5)
            c.gradient(17, 1, 54, 16, [STEEL_L, STEEL_D], radius=1)
            c.inset(19, 3, 52, 14.5, GLASS, radius=1)
        hand(c, th)
        if cn:
            coin(c, *cn)
        if n in (3, 4):
            for dy in (5, 8):
                c.line([(cn[0] - 2, cn[1] + dy), (cn[0] - 2, cn[1] + dy + 1.5)], WHITE, 0.6)
                c.line([(cn[0] + 2, cn[1] + dy), (cn[0] + 2, cn[1] + dy + 1.5)], WHITE, 0.6)
        c.clear(0, 0, 9, 47)
        c.clear(61, 0, 69, 47)
        c.rect(9, 0, 10, 47, fill=INK)
        c.rect(60, 0, 61, 47, fill=INK)
        c.rect(9, 46, 61, 47, fill=INK)
        knots(c, ctx.knots(), ["l", "r"], reach=5)
        out.append(c)
    out.append(ctx.blank(6))
    for n, word in ((7, tr("HEADS")), (8, tr("TAILS"))):
        c = ctx.canvas(n)
        w = ptext_width(word, 1)
        ptext(c, (c.w - w) // 2, 0, word, (255, 230, 60), 1)
        out.append(c)
    return out


# ---------------------------------------------------------------------------
# 1007 Random Number Generator: a lottery drum, at rest (1) and tumbling
# (2 to 4).

BALLS = [(240, 60, 50), (250, 200, 40), (60, 140, 240), (70, 200, 90), (240, 240, 240), (230, 110, 220), (250, 140, 40)]


def ball(c, x, y, col, r=3.6):
    c.circle(x, y, r + 0.7, fill=INK)
    c.radial(x, y, r, lighter(col, 0.6), darker(col, 0.25), -0.45, -0.45)
    c.circle(x + 0.2, y + 0.3, r * 0.45, fill=WHITE)


@sheet(1007)
def random_number(ctx):
    out = []
    rng = random.Random(1007)
    cx, cy, R = 36, 26.5, 20
    rest = [(-9, 13), (-1, 15), (7, 13.5), (-13, 7), (-5, 8), (3, 9), (11, 7)]
    for n in range(4):
        c = ctx.canvas(n + 1)
        # the chute to the output
        c.rect(52, 28, 80, 37, fill=INK, radius=2)
        c.gradient(53, 29, 79, 36, [(140, 230, 140), (40, 160, 70), (20, 90, 40)])
        c.panel(8, 3, 66, 52, (60, 90, 180), radius=3)
        for sx, sy in ((11.5, 6.5), (62.5, 6.5), (11.5, 48.5), (62.5, 48.5)):
            c.screw(sx, sy, 1.3)
        c.circle(cx, cy, R + 1.5, fill=INK)
        c.circle(cx, cy, R + 0.8, fill=STEEL_L)
        c.circle(cx, cy, R, fill=(22, 30, 48))
        c.radial(cx, cy, R - 0.5, (60, 76, 110), (16, 22, 38), -0.2, 0.3)
        if n == 0:
            pos = rest
        else:
            pos = []
            for i in range(7):
                a = rng.uniform(0, 2 * math.pi)
                d = rng.uniform(4, R - 5)
                pos.append((math.cos(a) * d, math.sin(a) * d))
        for i, (bx, by) in enumerate(pos):
            ball(c, cx + bx, cy + by, BALLS[i])
        if n:
            # the paddles turn
            a = math.radians(n * 40)
            for k in range(3):
                b = a + k * 2 * math.pi / 3
                c.line([(cx, cy), (cx + math.cos(b) * (R - 2), cy + math.sin(b) * (R - 2))], (150, 170, 200), 0.8)
        else:
            c.line([(cx, cy), (cx, cy - R + 2)], (150, 170, 200), 0.8)
            c.line([(cx, cy), (cx - 15.6, cy + 9)], (150, 170, 200), 0.8)
            c.line([(cx, cy), (cx + 15.6, cy + 9)], (150, 170, 200), 0.8)
        c.circle(cx, cy, 2.4, fill=INK)
        c.circle(cx, cy, 1.6, fill=BRASS)
        # glass shine
        c.arc(cx, cy, R - 3, 200, 250, (210, 225, 255), 1.0)
        if n == 3:
            ball(c, 61, 32.5, BALLS[2], 3.2)
        knots(c, ctx.knots(), ["l", "r"], reach=4)
        out.append(c)
    return blanks(ctx, out, 5)


# ---------------------------------------------------------------------------
# 1000 Pendulum: a glass cabinet (the game draws the rod in yellow from
# P[0], 74 pixels long, swinging 36° each way) and the bob (2).

NAVY = (16, 26, 58)


def glass_case(c, x0, y0, x1, y1, deep=NAVY, frame=BRASS):
    c.rect(x0, y0, x1, y1, fill=INK, radius=2)
    c.gradient(x0 + 1, y0 + 1, x1 - 1, y1 - 1, [BRASS_L, frame, BRASS_D], vertical=False, radius=1.5)
    c.rect(x0 + 4, y0 + 4, x1 - 4, y1 - 4, fill=INK)
    c.gradient(x0 + 5, y0 + 5, x1 - 5, y1 - 5, [lighter(deep, 0.2), deep, darker(deep, 0.3)])


@sheet(1000)
def pendulum(ctx):
    c = ctx.canvas(1)
    (px, py), = ctx.geo.points(1000)
    cx = px + 0.5
    k = ctx.knots()
    # spring mounts for the knots
    for x, y, out in k:
        side = -1 if x < cx else 1
        x0, x1 = (x + 4, 24) if side < 0 else (144, x - 3)
        c.rect(x0, y - 3, x1, y + 4, fill=INK, radius=1)
        c.gradient(x0 + 0.5, y - 2, x1 - 0.5, y + 3, [STEEL_L, STEEL, STEEL_D])
        for sx in range(int(x0) + 2, int(x1) - 1, 3):
            c.line([(sx, y - 2), (sx + 1, y + 3)], STEEL_D, 0.6)
    glass_case(c, 22, 14, 147, 113)
    # glare on the glass
    for gx, gy, ln in ((34, 60, 16), (40, 46, 8), (128, 40, 14), (122, 100, 9)):
        c.line([(gx, gy), (gx + ln * 0.7, gy - ln * 0.7)], (70, 96, 150), 1.0)
    # the swing's scale: an arc below the bob's path
    r = 74 + 9
    c.arc(cx, py + 0.5, r, 90 - 40, 90 + 40, (90, 120, 170), 1.0)
    for i in range(-8, 9):
        a = math.radians(i * 5)
        ln = 4 if i % 4 == 0 else 2
        p0 = (cx + math.sin(a) * r, py + 0.5 + math.cos(a) * r)
        p1 = (cx + math.sin(a) * (r + ln), py + 0.5 + math.cos(a) * (r + ln))
        c.line([p0, p1], (150, 180, 220) if i % 4 == 0 else (90, 120, 170), 0.8)
    # the beam and the pivot
    c.rect(18, 10, 151, 18, fill=INK, radius=2)
    c.gradient(19, 11, 150, 17, [BRASS_L, BRASS, BRASS_D], radius=1.5)
    c.polygon([(cx - 15, 18), (cx - 9, 8), (cx + 9, 8), (cx + 15, 18), (cx + 7, 27), (cx - 7, 27)], fill=INK)
    c.gradient(cx - 14, 9, cx + 14, 26, [BRASS_L, BRASS, BRASS_D], mask=poly_mask(c, [(cx - 13.5, 18), (cx - 8.4, 9), (cx + 8.4, 9), (cx + 13.5, 18), (cx + 6.5, 26), (cx - 6.5, 26)]))
    c.circle(cx, py + 0.5, 3.4, fill=INK)
    c.radial(cx, py + 0.5, 2.6, STEEL_L, STEEL_D, -0.4, -0.4)
    for sx in (25.5, 143.5):
        c.screw(sx, 14, 1.4)
    # feet
    for fx in (24, 126):
        c.rect(fx, 112, fx + 20, 119, fill=INK, radius=2)
        c.gradient(fx + 1, 113, fx + 19, 118, [STEEL_L, STEEL_D], radius=1.5)
    for x, y, out in k:
        c.knot(x, y, out)
    bob = ctx.canvas(2)
    bob.circle(8.5, 9, 8.3, fill=INK)
    bob.radial(8.5, 9, 7.4, BRASS_L, BRASS_D, -0.45, -0.45)
    bob.circle(6, 6.2, 1.5, fill=(255, 250, 220))
    return blanks(ctx, [c, bob], 3)


# ---------------------------------------------------------------------------
# 1002 Cannon: a screen with the shot's flight (the game moves the ball,
# frame 3, from (19, 5) to (177, 71)).


@sheet(1002)
def cannon(ctx):
    c = ctx.canvas(1)
    k = ctx.knots()
    x0, y0, x1, y1 = 11, 0, 193, 87
    c.rect(x0, y0, x1, y1, fill=INK, radius=3)
    c.gradient(x0 + 1, y0 + 1, x1 - 1, y1 - 1, [STEEL_L, STEEL, STEEL_D], radius=2)
    c.rect(x0 + 3, y0 + 3, x1 - 3, y1 - 3, fill=INK, radius=1)
    sx0, sy0, sx1, sy1 = x0 + 4, y0 + 4, x1 - 4, y1 - 4
    c.gradient(sx0, sy0, sx1, sy1, [(10, 40, 36), (6, 24, 22)])
    for gx in range(sx0 + 12, sx1, 16):
        c.prect(gx, sy0, gx + 1, sy1, (24, 76, 60))
    for gy in range(sy1 - 8, sy0, -12):
        c.prect(sx0, gy, sx1, gy + 1, (24, 76, 60))
    # the ground, the cliff it is fired from and the landing mark
    c.rect(sx0, 79, sx1, sy1, fill=(110, 70, 36))
    c.rect(sx0, 79, sx1, 80, fill=(120, 190, 70))
    c.polygon([(sx0, 16), (32, 16), (34, 30), (31, 50), (35, 79), (sx0, 79)], fill=(120, 76, 40))
    c.polygon([(sx0, 16), (32, 16), (33, 18), (sx0, 18)], fill=(120, 190, 70))
    for yy in (28, 42, 60):
        c.line([(sx0 + 3, yy), (sx0 + 12, yy + 2)], (90, 56, 30), 0.8)
    c.line([(176, 81), (190, 81)], (255, 200, 40), 1.0)
    c.line([(183, 76), (183, 86)], (255, 200, 40), 1.0)
    knots(c, k, reach=6)
    b = ctx.canvas(3)
    b.circle(6, 5.5, 5.4, fill=INK)
    b.radial(6, 5.5, 4.6, (255, 236, 150), (220, 90, 20), -0.45, -0.45)
    return [c, ctx.blank(2), b]


# ---------------------------------------------------------------------------
# 1001 Gravity Chamber: a glass tube (1); the open claw (3); for the
# feather (4 to 9), the elephant (10 to 15) and the baseball (16 to 21):
# held while the claw opens (4 frames), falling, landed. The game places
# each at its point (1010/1001), so everything is drawn in the tube's
# space and cut out at the frame's point.

TEAL = (18, 58, 70)
CLAW_X, CLAW_Y = 29.0, 43.0


def claw(c, t):
    """The claw hanging at CLAW, `t` 0 closed to 1 open."""
    cx, top = CLAW_X, CLAW_Y
    hub = (cx, top + 4.5)
    for s in (-1, 1):
        el = (cx + s * (11.5 + 4 * t), top + 13 - 3 * t)
        tip = (cx + s * (8.5 + 9 * t), top + 23.5 - 1.5 * t)
        for col, w in ((INK, 4.4), (BRASS, 2.6)):
            c.line([hub, el, tip], col, w)
        c.line([(hub[0] + s * 0.5, hub[1] - 0.6), (el[0] - s * 0.4, el[1] - 0.8)], BRASS_L, 0.7)
        c.circle(*el, 1.9, fill=INK)
        c.circle(*el, 1.1, fill=BRASS_L)
        c.circle(tip[0] - s * 0.6, tip[1], 2.3, fill=INK)
        c.circle(tip[0] - s * 0.6, tip[1], 1.4, fill=RED)
    c.rect(cx - 6, top, cx + 6, top + 4, fill=INK, radius=1)
    c.gradient(cx - 5, top + 0.5, cx + 5, top + 3.5, [BRASS_L, BRASS_D])
    c.circle(*hub, 2.6, fill=INK)
    c.circle(*hub, 1.6, fill=STEEL_L)


def stamp_xy(dst, src, cx, cy, angle=0.0, sx=1.0, sy=1.0):
    from PIL import Image as _I
    from wgfx import SS

    im = src.hi
    if sx != 1.0 or sy != 1.0:
        im = im.resize((max(1, int(im.width * sx)), max(1, int(im.height * sy))), _I.LANCZOS)
    if angle:
        im = im.rotate(angle, resample=_I.BICUBIC, expand=True)
    layer = _I.new("RGBA", dst.hi.size, (0, 0, 0, 0))
    from ._science_kit import _paste_any

    _paste_any(layer, im, int(round(cx * SS - im.width / 2)), int(round(cy * SS - im.height / 2)))
    dst.hi.alpha_composite(layer)


def feather_img():
    f = Canvas(46, 18)
    spine = [(3, 12), (14, 9.5), (28, 8.2), (44, 8.8)]
    top = [(8, 10), (13, 3), (24, 0.8), (36, 1.5), (45, 8)]
    bot = [(8, 11), (15, 17), (28, 16.8), (38, 14), (45, 9.5)]
    shape = top + list(reversed(bot))
    f.polygon(shape, fill=INK, outline=INK, width=1.6)
    f.polygon(shape, fill=(236, 238, 246))
    f.gradient(0, 0, 46, 16, [(255, 255, 255), (226, 230, 240), (176, 182, 200)], mask=poly_mask(f, shape))
    for i in range(10, 44, 3):
        f.line([(i, 8.6), (i + 3, 3.0 + abs(i - 27) * 0.08)], (190, 196, 214), 0.5)
        f.line([(i, 9.2), (i + 3, 15.0 - abs(i - 27) * 0.06)], (170, 176, 196), 0.5)
    for gx in (20, 31):
        f.line([(gx, 2), (gx - 1.6, 6.6)], INK, 0.7)
    f.line(spine, (150, 120, 90), 1.0)
    f.line(spine[:2], (120, 90, 60), 1.2)
    return f


def elephant_img(scared):
    e = Canvas(34, 36)
    g, gd, gl = (172, 176, 190), (120, 124, 140), (214, 218, 228)
    # body and legs
    e.ellipse(7, 17, 27, 33, fill=INK)
    e.ellipse(8, 18, 26, 32, fill=g)
    for lx in (8.5, 14, 20, 25.5):
        e.rect(lx - 3, 26, lx + 3, 35.5, fill=INK, radius=2)
        e.rect(lx - 2, 26, lx + 2, 34.5, fill=g, radius=1.5)
        e.line([(lx - 1.4, 34.2), (lx + 1.4, 34.2)], gl, 0.6)
    # ears
    for s in (-1, 1):
        ex = 17 + s * 10
        e.ellipse(ex - 7.5, 3, ex + 7.5, 20, fill=INK)
        e.ellipse(ex - 6.5, 4, ex + 6.5, 19, fill=gd)
        e.ellipse(ex - 4.8 + s * 0.8, 6, ex + 4.8 + s * 0.8, 17, fill=(232, 170, 176))
    # head
    e.ellipse(8, 1, 26, 21, fill=INK)
    e.ellipse(9, 2, 25, 20, fill=g)
    e.ellipse(11, 3, 19, 10, fill=gl)
    # trunk
    if scared:
        e.line([(17, 16), (17.5, 22), (21, 26), (24.5, 24)], INK, 4.4)
        e.line([(17, 16), (17.5, 22), (21, 26), (24.5, 24)], g, 2.6)
    else:
        e.line([(17, 16), (16.5, 23), (14, 27), (11.5, 26)], INK, 4.4)
        e.line([(17, 16), (16.5, 23), (14, 27), (11.5, 26)], g, 2.6)
    # eyes
    for s in (-1, 1):
        ex = 17 + s * 3.6
        if scared:
            e.circle(ex, 9.5, 2.6, fill=INK)
            e.circle(ex, 9.5, 1.9, fill=WHITE)
            e.circle(ex, 9.5, 0.8, fill=INK)
        else:
            e.ellipse(ex - 1.2, 8, ex + 1.2, 11.5, fill=INK)
            e.circle(ex - 0.3, 8.8, 0.45, fill=WHITE)
    if scared:
        e.ellipse(12.5, 15.5, 16, 19.5, fill=INK)
        e.ellipse(13.2, 16.5, 15.3, 19, fill=(220, 60, 70))
    else:
        e.arc(13.5, 15.5, 2.0, 30, 150, INK, 0.7)
    return e


def baseball_img(r=9.0):
    b = Canvas(int(2 * r + 2), int(2 * r + 2))
    c0 = r + 1
    b.circle(c0, c0, r + 0.5, fill=INK)
    b.radial(c0, c0, r - 0.3, WHITE, (196, 196, 204), -0.4, -0.4)
    for s in (-1, 1):
        b.arc(c0 + s * r * 1.25, c0, r * 0.82, 90 - 52 + (90 + 52 - 90 + 52) * 0 if s < 0 else 0, 360, (0, 0, 0, 0), 0.1)
    for s in (-1, 1):
        cx = c0 + s * r * 1.22
        a0, a1 = (-48, 48) if s < 0 else (132, 228)
        b.arc(cx, c0, r * 0.85, a0, a1, (210, 40, 40), 0.8)
        for k in range(-3, 4):
            a = math.radians((a0 + a1) / 2 + k * 13)
            px, py = cx + math.cos(a) * r * 0.85, c0 + math.sin(a) * r * 0.85
            b.line([(px - math.sin(a) * 1.1, py + math.cos(a) * 1.1 * 0 - 0.9), (px + math.sin(a) * 1.1, py + 0.9)], (210, 40, 40), 0.5)
    return b


# Each object: its frames' sizes come from the sheet; (held centre, its
# drop in the last claw phase), and how it is drawn.
def draw_thing(c, th, x, y, phase):
    """Thing `th` (0 feather, 1 elephant, 2 ball) centred at (x, y);
    `phase` "held", "fall" or "land"."""
    if th == 0:
        if phase == "land":
            stamp_xy(c, feather_img(), x, y, 3, 1.12, 0.9)
        elif phase == "fall":
            stamp_xy(c, feather_img(), x, y, 14)
        else:
            stamp_xy(c, feather_img(), x, y, 4)
    elif th == 1:
        if phase == "land":
            stamp_xy(c, elephant_img(False), x, y, 0, 1.2, 0.82)
        else:
            stamp_xy(c, elephant_img(phase in ("fall", "scared")), x, y)
    else:
        if phase == "land":
            stamp_xy(c, baseball_img(), x, y, 0, 1.0, 0.82)
        else:
            stamp_xy(c, baseball_img(), x, y)


@sheet(1001)
def drop_chamber(ctx):
    pts = ctx.geo.points(1001)
    out = [drop_tube(ctx), ctx.blank(2)]
    big = Canvas(60, 201)
    claw(big, 1.0)
    out.append(cut(big, *pts[0], *ctx.size(3)))
    # the held centres (gadget space) for each thing, and how far each
    # claw phase lets it sink
    held = {0: (29, 66), 1: (28.5, 70), 2: (29.5, 64)}
    sink = [0, 0, 1, 3]
    opening = [0.0, 0.15, 0.7, 1.0]
    for th, (f0, p0) in enumerate(((4, 1), (10, 7), (16, 13))):
        for ph in range(4):
            big = Canvas(60, 201)
            hx, hy = held[th]
            if th == 1:
                draw_thing(big, th, hx, hy + sink[ph] * 1.3, "held" if ph < 2 else "scared")
                claw(big, opening[ph])
            else:
                claw(big, opening[ph]) if th == 2 else None
                draw_thing(big, th, hx, hy + sink[ph] * 1.3, "held")
                if th == 0:
                    claw(big, opening[ph])
            x, y = pts[p0 + ph]
            out.append(cut(big, x, y, *ctx.size(f0 + ph)))
        for k, kind in ((4, "fall"), (5, "land")):
            w, h = ctx.size(f0 + k)
            c = ctx.canvas(f0 + k)
            draw_thing(c, th, w / 2, h / 2 if kind == "fall" else h - (8.5, 15.5, 8.2)[th], kind)
            out.append(c)
    return out


def drop_tube(ctx):
    c = ctx.canvas(1)
    k = ctx.knots()
    gx0, gx1, gy0, gy1 = 3, 57, 27, 182
    # glass
    c.rect(gx0, gy0, gx1, gy1, fill=INK)
    c.gradient(gx0 + 1, gy0, gx1 - 1, gy1, [darker(TEAL, 0.2), lighter(TEAL, 0.25), TEAL, darker(TEAL, 0.4)], vertical=False)
    c.rect(gx0 + 5, gy0 + 6, gx0 + 8, gy1 - 14, fill=lighter(TEAL, 0.35))
    c.rect(gx0 + 10, gy0 + 6, gx0 + 11, gy1 - 14, fill=lighter(TEAL, 0.25))
    c.rect(gx1 - 7, gy0 + 6, gx1 - 6, gy1 - 14, fill=lighter(TEAL, 0.15))
    # height marks
    for i, y in enumerate(range(60, 168, 9)):
        c.prect(gx1 - 5, y, gx1 - (1 if i % 2 else 3) - 1, y + 1, (110, 170, 170))
    # the floor: a green cushion
    c.rect(gx0 + 1, 171, gx1 - 1, 179, fill=INK)
    c.gradient(gx0 + 1, 172, gx1 - 1, 179, [(140, 240, 120), (40, 160, 60), (20, 90, 40)])
    # top cap with the claw's housing
    c.rect(1, 19, 59, 31, fill=INK, radius=3)
    c.gradient(2, 20, 58, 30, [BRASS_L, BRASS, BRASS_D], radius=2)
    c.rect(24, 6, 36, 20, fill=INK, radius=1)
    c.gradient(25, 6, 35, 20, [BRASS_D, BRASS_L, BRASS, BRASS_D], vertical=False)
    c.rect(19, 30, 39, 37, fill=INK, radius=1)
    c.gradient(20, 30, 38, 36, [STEEL_L, STEEL_D])
    c.rect(26, 36, 32, 44, fill=INK)
    c.gradient(27, 36, 31, 44, [STEEL_L, STEEL_D], vertical=False)
    for sx in (5.5, 54.5):
        c.screw(sx, 25, 1.4)
    # bottom cap
    c.rect(1, 179, 59, 189, fill=INK, radius=3)
    c.gradient(2, 180, 58, 188, [BRASS_L, BRASS, BRASS_D], radius=2)
    for sx in (5.5, 54.5):
        c.screw(sx, 184, 1.4)
    for x, y, out in k:
        c.knot(x, y, out, "t" if y < 100 else "b", 4)
    return c


# ---------------------------------------------------------------------------
# 1036 Multiplexer: a railway switch. 1 both tracks (editing); 3 to 7 the
# track from input 1 with a train going through, 8 to 12 the same from
# input 2.

SAND = (214, 168, 92)


def bezier(p0, p1, p2, p3, n=24):
    out = []
    for i in range(n + 1):
        t = i / n
        a, b, c_, d = (1 - t) ** 3, 3 * (1 - t) ** 2 * t, 3 * (1 - t) * t * t, t ** 3
        out.append((a * p0[0] + b * p1[0] + c_ * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + c_ * p2[1] + d * p3[1]))
    return out


def track_path(y_in, y_out=45.0):
    pts = [(2.0, y_in), (16.0, y_in)]
    pts += bezier((16, y_in), (34, y_in), (38, y_out), (56, y_out))[1:]
    pts += [(84.0, y_out)]
    # resample evenly by length
    out, acc = [pts[0]], 0.0
    seg = []
    for a, b in zip(pts, pts[1:]):
        seg.append((a, b, math.dist(a, b)))
    total = sum(d for _, _, d in seg)
    step, s = 0.5, 0.5
    for a, b, d in seg:
        while s <= acc + d and d > 0:
            t = (s - acc) / d
            out.append((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t))
            s += step
        acc += d
    return out, step


def along(path, step, dist):
    """Point and angle (degrees, anticlockwise) at `dist` along `path`."""
    i = max(1, min(len(path) - 2, int(dist / step)))
    (x0, y0), (x1, y1) = path[i - 1], path[i + 1]
    return path[i], math.degrees(math.atan2(-(y1 - y0), x1 - x0))


def offset(path, d):
    out = []
    for i, (x, y) in enumerate(path):
        a, b = path[max(0, i - 1)], path[min(len(path) - 1, i + 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        n = math.hypot(dx, dy) or 1
        out.append((x - dy / n * d, y + dx / n * d))
    return out


def ties(c, path, step):
    for k in range(0, len(path), int(5 / step)):
        (x, y), _ = path[k], 0
        a, b = path[max(0, k - 1)], path[min(len(path) - 1, k + 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        n = math.hypot(dx, dy) or 1
        nx, ny = -dy / n, dx / n
        c.line([(x - nx * 6, y - ny * 6), (x + nx * 6, y + ny * 6)], INK, 2.6, round_ends=False)
        c.line([(x - nx * 5.4, y - ny * 5.4), (x + nx * 5.4, y + ny * 5.4)], (110, 70, 40), 1.4, round_ends=False)


def rails(c, path):
    for d in (-3.5, 3.5):
        r = offset(path, d)
        c.line(r, INK, 2.2)
    for d in (-3.5, 3.5):
        r = offset(path, d)
        c.line(r, STEEL_L, 1.0)


def dirt(c):
    rng = random.Random(1036)
    pts = []
    for i in range(28):
        a = i / 28 * 2 * math.pi
        rx, ry = 38 + rng.uniform(-3, 3), 29 + rng.uniform(-3, 3)
        pts.append((45 + math.cos(a) * rx, 40 + math.sin(a) * ry))
    c.polygon(pts, fill=darker(SAND, 0.45), outline=darker(SAND, 0.45), width=1.4)
    c.polygon(pts, fill=SAND)
    for _ in range(160):
        x, y = rng.uniform(10, 80), rng.uniform(14, 68)
        if ((x - 45) / 36) ** 2 + ((y - 40) / 27) ** 2 < 1:
            c.pixel(int(x), int(y), rng.choice([darker(SAND, 0.2), lighter(SAND, 0.25), darker(SAND, 0.35)]))
    for gx, gy in ((22, 18), (70, 22), (28, 66), (68, 62), (40, 15)):
        c.line([(gx, gy), (gx - 1.5, gy - 3)], (70, 150, 60), 0.8)
        c.line([(gx + 1, gy), (gx + 2, gy - 3.5)], (90, 180, 70), 0.8)


def engine_img():
    e = Canvas(20, 13)
    e.rect(0.5, 0.5, 19, 12.5, fill=INK, radius=2)
    e.gradient(1.5, 1.5, 18, 11.5, [(140, 180, 255), (60, 100, 230), (30, 50, 150)], radius=1.5)
    e.rect(2, 2.5, 8.5, 10.5, fill=INK, radius=1)
    e.rect(2.8, 3.3, 7.7, 9.7, fill=(170, 215, 255))
    e.circle(14, 6.5, 3.6, fill=INK)
    e.circle(14, 6.5, 2.6, fill=(40, 40, 50))
    e.circle(13.3, 5.8, 0.8, fill=(110, 110, 130))
    e.rect(18.5, 4.5, 20, 8.5, fill=(255, 220, 80))
    return e


def car_img():
    w = Canvas(15, 12)
    w.rect(0.5, 0.5, 14.5, 11.5, fill=INK, radius=1.5)
    w.gradient(1.5, 1.5, 13.5, 10.5, [(130, 230, 150), (40, 160, 80)], radius=1)
    w.rect(3.5, 3, 11.5, 9, fill=(214, 150, 70))
    w.rect(3.5, 3, 11.5, 4.5, fill=(250, 200, 120))
    return w


def switch_stand(c, sel):
    x = 52.5
    c.rect(x - 1.5, 6, x + 1.5, 34, fill=INK)
    c.rect(x - 0.7, 6, x + 0.7, 34, fill=STEEL_L)
    c.rect(x - 4, 31, x + 5, 36, fill=INK, radius=1)
    c.gradient(x - 3, 31.7, x + 4, 35.3, [STEEL_L, STEEL_D])
    f = Canvas(8, 20)
    f.ellipse(0.5, 0.5, 7.5, 19.5, fill=INK)
    f.gradient(1.3, 1.3, 6.7, 18.7, [(150, 240, 120), (50, 170, 60), (30, 110, 40)], vertical=False, mask=mask_of(f, 1.3, 1.3, 6.7, 18.7))
    ang = {0: 0, 1: -35, 2: 35}[sel]
    a = math.radians(ang)
    stamp(c, f, x + math.sin(-a) * 9, 22 - math.cos(a) * 9 + 9, ang)
    c.circle(x, 22, 1.8, fill=INK)
    c.circle(x, 22, 1.1, fill=BRASS_L)


@sheet(1036)
def multiplexer(ctx):
    k = ctx.knots()
    p1, step = track_path(k[1][1] + 0.5)
    p2, _ = track_path(k[2][1] + 0.5)

    def frame(sel, train):
        c = ctx.canvas(1)
        dirt(c)
        paths = [p1, p2] if sel == 0 else [(p1, p2)[sel - 1]]
        for p in paths:
            ties(c, p, step)
        for p in paths:
            rails(c, p)
        path = paths[0]
        L = len(path) * step
        for img, dist in train:
            if 0 < dist < L:
                (x, y), ang = along(path, step, dist)
                stamp(c, img, x, y, ang)
        switch_stand(c, sel)
        knots(c, k, ["t", "l", "l", "r"], reach=3)
        return c

    out = [frame(0, []), ctx.blank(2)]
    runs = [[], [(engine_img(), 15)], [(engine_img(), 47), (car_img(), 29)], [(engine_img(), 76), (car_img(), 58)], [(car_img(), 76)]]
    for sel in (1, 2):
        for tr in runs:
            out.append(frame(sel, tr))
    return out


# ---------------------------------------------------------------------------
# 1051 Win / Lose Checker: a black box (1); on it at P[0] a question mark
# (2, waiting) that turns (3, 4) into an exclamation mark with a spark
# running through it (5 to 9).


def bubbly(g, strokes, dots, width, body, light, dark):
    """Fat round strokes (polylines) and dots, outlined in INK, with a
    shine along the top left."""
    for pts in strokes:
        g.line(pts, INK, width + 3)
    for x, y, r in dots:
        g.circle(x, y, r + 1.5, fill=INK)
    for pts in strokes:
        g.line(pts, dark, width)
        g.line([(x - 0.8, y - 0.8) for x, y in pts], body, width - 2.6)
        g.line([(x - width * 0.22, y - width * 0.22) for x, y in pts], light, width * 0.28)
    for x, y, r in dots:
        g.circle(x, y, r, fill=dark)
        g.circle(x - 0.8, y - 0.8, r - 1.3, fill=body)
        g.circle(x - r * 0.35, y - r * 0.35, r * 0.32, fill=light)


def question(w, h, body, light, dark, angle=0.0, sx=1.0):
    g = Canvas(w, h)
    cx, cy, r = w / 2, 26.0, 17.0
    pts = [(cx + r * math.cos(math.radians(a)), cy - r * math.sin(math.radians(a))) for a in range(205, -45, -8)]
    pts += [(cx + 4, cy + 22), (cx + 0.5, cy + 29), (cx, cy + 36)]
    bubbly(g, [pts], [(cx, 80, 9)], 15, body, light, dark)
    out = Canvas(w, h)
    stamp_xy(out, g, w / 2, h / 2, angle, sx, 1.0)
    return out


def exclaim(w, h, spark_col, phase):
    g = Canvas(w, h)
    cx = w / 2
    body, light, dark = (240, 50, 30), (255, 170, 140), (150, 14, 10)
    T, B = 13.0, 8.0
    bar = [(cx - T, 13), (cx + T, 13), (cx + B, 63), (cx - B, 63)]
    g.polygon(bar, fill=INK, outline=INK, width=3.0)
    g.circle(cx, 13, T + 1.5, fill=INK)
    g.circle(cx, 63, B + 1.5, fill=INK)
    g.circle(cx, 81, 10.5, fill=INK)
    g.circle(cx, 13, T, fill=dark)
    g.polygon([(cx - T, 13), (cx + T, 13), (cx + B, 63), (cx - B, 63)], fill=dark)
    g.circle(cx, 63, B, fill=dark)
    g.circle(cx - 1, 12.5, T - 1.6, fill=body)
    g.polygon([(cx - T + 0.4, 12.5), (cx + T - 2.6, 12.5), (cx + B - 2.4, 62), (cx - B + 0.4, 62)], fill=body)
    g.circle(cx - 1, 62, B - 1.6, fill=body)
    g.line([(cx - 7.5, 6), (cx - 8.5, 16), (cx - 5, 52)], light, 2.6)
    g.circle(cx, 81, 9, fill=dark)
    g.circle(cx - 0.8, 80.2, 7.6, fill=body)
    g.circle(cx - 3.4, 77.6, 2.6, fill=light)
    pts = []
    for j in range(10):
        y = 7 + j * 6.0
        amp = 7.0 - j * 0.45
        x = cx + amp * math.sin(j * 1.9 + phase)
        pts.append((x, y))
    g.line(pts, darker(spark_col, 0.45), 4.2)
    g.line(pts, spark_col, 2.6)
    g.line(pts, lighter(spark_col, 0.7), 0.8)
    return g


@sheet(1051)
def stopper(ctx):
    k = ctx.knots()
    c = ctx.canvas(1)
    base = (34, 46, 130)
    front = [(6, 21), (88, 21), (88, 111), (6, 111)]
    top = [(6, 21), (28, 3), (110, 3), (88, 21)]
    side = [(88, 21), (110, 3), (110, 92), (88, 111)]
    for poly, cols in ((front, [lighter(base, 0.15), darker(base, 0.25)]), (top, [lighter(base, 0.45), lighter(base, 0.2)]),
                       (side, [darker(base, 0.35), darker(base, 0.55)])):
        c.polygon(poly, fill=INK, outline=INK, width=2.0)
        c.gradient(0, 0, 112, 113, cols, mask=poly_mask(c, poly))
    # rivets along the edges, a light edge on top
    c.line([(7, 22), (87.5, 22)], lighter(base, 0.6), 0.8)
    c.line([(88.5, 21), (109.5, 3.5)], lighter(base, 0.4), 0.8)
    for x in range(12, 86, 12):
        c.circle(x, 106, 0.9, fill=lighter(base, 0.4))
    for y in range(28, 108, 12):
        c.circle(10.5, y, 0.9, fill=lighter(base, 0.4))
        c.circle(84, y, 0.9, fill=lighter(base, 0.4))
    c.dither(0, 0, 112, 113)
    knots(c, k, ["l"], reach=3)
    out = [c]
    w, h = ctx.size(2)
    out.append(question(w, h, (255, 170, 30), (255, 245, 170), (220, 70, 20)))
    out.append(question(w, h, (255, 130, 30), (255, 220, 140), (210, 50, 20), angle=-12))
    out.append(question(w, h, (250, 90, 30), (255, 190, 120), (190, 30, 20), sx=0.5))
    sparks = [(255, 240, 60), (200, 255, 60), (255, 255, 120), (60, 240, 220), (120, 255, 200)]
    for i, col in enumerate(sparks):
        out.append(exclaim(w, h, col, i * 2.1))
    return out


# ---------------------------------------------------------------------------
# 1050 Graphic Projector: a screen in a frame, black (1), a blue stage (2,
# for the rock star) and a test card (3, for the numbers). The pictures
# (sheets 200 to 205) are centred on the screen (1012/1050 R0) by their
# first frame's centre.

ROSEWOOD = (128, 58, 36)


def screen_rect(ctx):
    return ctx.geo.rects(1050)[0]


def screen_at(ctx, sheet_id):
    """Where frame 1 of picture sheet `sheet_id` goes on the projector."""
    x0, y0, x1, y1 = screen_rect(ctx)
    t, l, b, r = ctx.geo.frames(sheet_id)[0]
    return x0 + (x1 - x0) // 2 - (l + (r - l) // 2), y0 + (y1 - y0) // 2 - (t + (b - t) // 2)


def stage(c, x0, y0, x1, y1):
    c.gradient(x0, y0, x1, y1, [(4, 8, 40), (16, 30, 110), (30, 60, 170)])
    cx = (x0 + x1) / 2
    for dx in (-30, 30, 0):
        pts = [(cx + dx - 3, y0), (cx + dx + 3, y0), (cx + dx * 0.4 + 16, y1 - 8), (cx + dx * 0.4 - 16, y1 - 8)]
        c.gradient(x0, y0, x1, y1, [(60, 90, 210), (40, 70, 190)], mask=poly_mask(c, pts).point(lambda v: v * 45 // 100))
    floor = y1 - 8
    c.rect(x0, floor, x1, y1, fill=(40, 90, 220))
    c.rect(x0, floor, x1, floor + 1, fill=(140, 190, 255))
    c.rect(x0, floor + 4, x1, floor + 5, fill=(30, 70, 190))
    c.dither(x0, y0, x1, floor)


def test_card(c, x0, y0, x1, y1):
    c.rect(x0, y0, x1, y1, fill=(120, 124, 132))
    for gx in range(x0 + 4, x1, 8):
        c.prect(gx, y0, gx + 1, y1, (160, 164, 172))
    for gy in range(y0 + 3, y1, 8):
        c.prect(x0, gy, x1, gy + 1, (160, 164, 172))
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    c.circle(cx, cy, 31, fill=INK)
    c.circle(cx, cy, 30, fill=(214, 216, 222))
    # grey steps across the middle, colour bars above and below
    bars = [(230, 230, 230), (230, 220, 60), (60, 210, 220), (70, 200, 80), (210, 70, 200), (220, 60, 50), (60, 70, 220)]
    m = mask_of(c, cx - 29.5, cy - 29.5, cx + 29.5, cy + 29.5)
    bw = 60 / len(bars)
    for i, col in enumerate(bars):
        c.gradient(cx - 30 + i * bw, cy - 30, cx - 30 + (i + 1) * bw + 0.3, cy - 9, [col, col], mask=m)
    for i in range(6):
        g = int(20 + i * 45)
        c.gradient(cx - 30 + i * 10, cy + 9, cx - 20 + i * 10 + 0.3, cy + 30, [(g, g, g), (g, g, g)], mask=m)
    c.rect(cx - 30, cy - 9, cx + 30, cy + 9, fill=(40, 42, 48))
    c.circle(cx, cy, 10, fill=WHITE)
    c.circle(cx, cy, 8.6, fill=(40, 42, 48))
    c.line([(cx - 30, cy), (cx + 30, cy)], WHITE, 0.8)
    c.line([(cx, cy - 30), (cx, cy + 30)], WHITE, 0.8)
    for sx, sy in ((x0 + 6, y0 + 6), (x1 - 6, y0 + 6), (x0 + 6, y1 - 6), (x1 - 6, y1 - 6)):
        c.circle(sx, sy, 3.5, fill=INK)
        c.circle(sx, sy, 2.5, fill=WHITE)


def projector_body(ctx, n):
    c = ctx.canvas(n)
    sx0, sy0, sx1, sy1 = screen_rect(ctx)
    c.panel(13, 0, 131, 98, ROSEWOOD, radius=5, bevel=2, gloss=False)
    c.rect(sx0 - 5, sy0 - 5, sx1 + 5, sy1 + 5, fill=INK, radius=4)
    c.gradient(sx0 - 4, sy0 - 4, sx1 + 4, sy1 + 4, [BRASS_L, BRASS, BRASS_D], radius=3)
    c.rect(sx0 - 1, sy0 - 1, sx1 + 1, sy1 + 1, fill=INK, radius=1)
    if n == 1:
        c.rect(sx0, sy0, sx1, sy1, fill=(4, 4, 8))
        c.line([(sx0 + 4, sy0 + 12), (sx0 + 12, sy0 + 4)], (40, 44, 60), 1.0)
    elif n == 2:
        stage(c, sx0, sy0, sx1, sy1)
    else:
        test_card(c, sx0, sy0, sx1, sy1)
    for x, y in ((18, 5), (126, 5), (126, 92), (34, 92)):
        c.screw(x + 0.5, y + 0.5, 1.5)
    # a lamp housing on the bottom rail
    c.rect(60, 86, 84, 95, fill=INK, radius=2)
    c.gradient(61, 87, 83, 94, [STEEL_L, STEEL_D], radius=1.5)
    c.circle(72, 90.5, 2.5, fill=(255, 230, 120))
    for a in ((13, 0, 131, sy0 - 5), (13, sy1 + 5, 131, 98), (13, 0, sx0 - 5, 98), (sx1 + 5, 0, 131, 98)):
        c.dither(*a)
    knots(c, ctx.knots(), ["l"], reach=10)
    return c


@sheet(1050)
def graphic_projector(ctx):
    return [projector_body(ctx, n) for n in (1, 2, 3)] + [ctx.blank(4)]


# 200 Phases of the Moon: new, waxing crescent, first quarter, waxing
# gibbous, full, waning gibbous, last quarter, waning crescent.


def moon(w, h, phi, r=None):
    from PIL import Image as _I
    from wgfx import SS

    c = Canvas(w, h)
    cx, cy = w / 2, h / 2
    r = r or min(w, h) / 2 - 1
    c.circle(cx, cy, r, fill=(190, 190, 186))
    c.radial(cx, cy, r, (236, 234, 226), (150, 150, 148), -0.35, -0.35)
    rng = random.Random(200)
    for (mx, my, mr) in ((-0.3, -0.25, 0.28), (0.2, -0.35, 0.18), (0.25, 0.2, 0.3), (-0.35, 0.3, 0.16), (0.0, 0.05, 0.14)):
        c.ellipse(cx + (mx - mr) * r, cy + (my - mr * 0.9) * r, cx + (mx + mr) * r, cy + (my + mr * 0.9) * r, fill=(150, 150, 150))
    for _ in range(14):
        a, d = rng.uniform(0, 2 * math.pi), rng.uniform(0, 0.85)
        x, y, cr = cx + math.cos(a) * d * r, cy + math.sin(a) * d * r, rng.uniform(0.04, 0.09) * r
        c.circle(x, y, cr, fill=(120, 120, 122))
        c.circle(x - cr * 0.25, y - cr * 0.25, cr * 0.7, fill=(204, 204, 198))
    # the night side
    if phi is not None:
        dark = _I.new("L", c.hi.size, 0)
        px = dark.load()
        R = r * SS
        for y in range(dark.height):
            v = (y + 0.5 - cy * SS) / R
            if abs(v) >= 1:
                continue
            t = math.sqrt(1 - v * v)
            for x in range(dark.width):
                u = (x + 0.5 - cx * SS) / R
                if abs(u) >= t:
                    continue
                lit = u > t * math.cos(phi) if phi <= math.pi else u < -t * math.cos(phi)
                if not lit:
                    px[x, y] = 255
        shade = _I.new("RGBA", c.hi.size, (34, 34, 42, 255))
        a = c.hi.getchannel("A")
        from PIL import ImageChops, ImageFilter

        dark = dark.filter(ImageFilter.GaussianBlur(SS * 0.6))
        dark = ImageChops.multiply(dark, a)
        tex = c.hi.copy()
        c.hi = _I.composite(_I.blend(shade, tex, 0.18), tex, dark)
        from PIL import ImageDraw

        c.d = ImageDraw.Draw(c.hi)
    c.circle(cx, cy, r + 0.6, outline=(20, 20, 26), width=0.9)
    return c


@sheet(200)
def moon_phases(ctx):
    w, h = ctx.size(1)
    return [moon(w, h, i * math.pi / 4 if i else 0.0) for i in range(8)]


# 201 ESP cards: waves, square, star, circle, cross.


@sheet(201)
def esp_cards(ctx):
    w, h = ctx.size(1)
    cx, cy = w / 2, h / 2
    col, lw = (250, 250, 255), 5.0
    out = []
    for n in range(5):
        c = ctx.canvas(n + 1)
        if n == 0:
            for k in (-1, 0, 1):
                pts = [(cx - 22 + i / 2, cy + k * 10 + 3.0 * math.sin(i / 88 * 3 * 2 * math.pi)) for i in range(0, 89)]
                c.line(pts, col, 3.4)
        elif n == 1:
            c.rect(cx - 17, cy - 18, cx + 17, cy + 18, outline=col, width=lw)
        elif n == 2:
            pts = []
            for i in range(10):
                a = math.radians(90 + i * 36)
                rr = 21 if i % 2 == 0 else 9
                pts.append((cx + math.cos(a) * rr, cy + 1.5 - math.sin(a) * rr))
            c.polygon(pts, outline=col, width=3.6)
        elif n == 3:
            c.circle(cx, cy, 17, outline=col, width=lw)
        else:
            c.rect(cx - 3, cy - 19, cx + 3, cy + 19, fill=col)
            c.rect(cx - 19, cy - 3, cx + 19, cy + 3, fill=col)
        out.append(c)
    return out


# 202 Astrological signs: the sign in a box, the name under it.

SIGNS = [("Aquarius", "♒", (60, 140, 200)), ("Pisces", "♓", (160, 60, 220)), ("Aries", "♈", (220, 60, 50)),
         ("Taurus", "♉", (140, 80, 30)), ("Gemini", "♊", (230, 160, 30)), ("Cancer", "♋", (60, 120, 210)),
         ("Leo", "♌", (230, 120, 20)), ("Virgo", "♍", (130, 40, 160)), ("Libra", "♎", (220, 60, 160)),
         ("Scorpio", "♏", (170, 20, 40)), ("Sagittarius", "♐", (30, 140, 150)), ("Capricorn", "♑", (50, 140, 50))]


@sheet(202)
def zodiac(ctx):
    out = []
    for n, (name, glyph, col) in enumerate(SIGNS, 1):
        c = ctx.canvas(n)
        cx = c.w / 2
        c.rect(cx - 14, 0, cx + 14, 28, fill=(220, 250, 60), radius=1)
        c.rect(cx - 12.5, 1.5, cx + 12.5, 26.5, fill=lighter(col, 0.85))
        c.text(cx, 14, glyph, 25, col, "sans-book", stroke=0.7, stroke_fill=col)
        c.text_fit(0, 31, c.w, 50, tr(name), (220, 250, 60), "sans", 13, stroke=0.9, stroke_fill=(20, 40, 0))
        out.append(c)
    return out


# 203 Fortune teller: the eight answers.

ANSWERS = ["No", "Yes", "NEVER!", "Ask\nagain\nlater", "Maybe", "For sure", "Of\nCourse!", "Not Today"]


@sheet(203)
def fortunes(ctx):
    out = []
    for n, s in enumerate(ANSWERS, 1):
        c = ctx.canvas(n)
        c.text_fit(2, 6, c.w - 2, c.h - 6, tr(s), WHITE, "sans", 17, stroke=0.6, stroke_fill=(60, 60, 90))
        out.append(c)
    return out


# 204 Numbers: big digits on the test card (frame 3 of 1050) behind them.


@sheet(204)
def numbers(ctx):
    bx, by = screen_at(ctx, 204)
    from wgfx import SheetContext

    card = projector_body(SheetContext(ctx.geo, 1050), 3)
    out = []
    for n in range(10):
        w, h = ctx.size(n + 1)
        c = cut(card, bx, by, w, h)
        c.text(w / 2, h / 2 + 1, str(n), 58, (240, 30, 20), "cond", stroke=2.0, stroke_fill=INK)
        out.append(c)
    return out


# 205 Rock star: a guitarist head-banging: upright (1), bending (2, 3),
# crouched with the hair thrown forward (4, 5), coming up (6, 7).

HAIR = (255, 220, 60)
SHIRT = (220, 40, 40)
JEANS = (40, 50, 120)


def limb(c, pts, col, w):
    c.line(pts, INK, w + 1.8)
    c.line(pts, col, w)


def rot(p, a, o):
    """Point `p` (relative) turned `a` radians clockwise, then moved to `o`."""
    x, y = p
    return (o[0] + x * math.cos(a) - y * math.sin(a), o[1] + x * math.sin(a) + y * math.cos(a))


def rocker(w, h, crouch, bang, flip):
    c = Canvas(w, h)
    fy = h - 2.5
    fl, fr = (16.0, fy), (31.0, fy)
    hip = (21.0 + 2 * crouch, 35.0 + 11 * crouch)
    knee_l = (12.0 + 3 * crouch, (fy + hip[1]) / 2 - 1 + 3 * crouch)
    knee_r = (30.0 + 6 * crouch, (fy + hip[1]) / 2 - 2 - 1 * crouch)
    lean = math.radians(6 + 45 * crouch)
    sh = rot((0, -16), lean, hip)
    nod = lean + math.radians(55 * bang)
    head = rot((0, -8), nod, sh)
    # legs and boots
    limb(c, [hip, knee_l, fl], JEANS, 4.6)
    limb(c, [(hip[0] + 2, hip[1]), knee_r, fr], JEANS, 4.6)
    for f in (fl, fr):
        c.rect(f[0] - 3.5, f[1] - 3.5, f[0] + 5, f[1] + 2, fill=INK, radius=1.5)
        c.rect(f[0] - 2.7, f[1] - 2.7, f[0] + 4.2, f[1] + 1.2, fill=(240, 240, 250), radius=1)
    # torso
    c.line([hip, sh], INK, 11)
    c.line([hip, sh], SHIRT, 9)
    c.line([rot((-2.5, -2), lean, hip), rot((-2.5, 1), lean, sh)], lighter(SHIRT, 0.35), 1.4)
    # the mane, in the head's frame: up is (0, -1), the back is to the left
    mane = []
    for i in range(17):
        th = math.radians(-35 + i * 14)
        r = (12.5 if i % 2 == 0 else 8.0) + (3.5 if th > math.radians(90) else 0)
        mane.append(rot((-math.sin(th) * r, -math.cos(th) * r), nod + math.radians(flip), head))
    mane.append(rot((-1, 6), nod, head))

    def draw_mane():
        c.polygon(mane, fill=INK, outline=INK, width=1.8)
        c.polygon(mane, fill=HAIR)
        for i in range(2, 16, 3):
            c.line([rot((-1.5, -1), nod, head), mane[i]], darker(HAIR, 0.2), 0.6)

    if bang < 0.6:
        draw_mane()
    c.circle(*head, 5.6, fill=INK)
    c.circle(*head, 4.6, fill=SKIN)
    if bang < 0.6:
        eye = rot((2.4, -1), nod, head)
        c.circle(*eye, 0.75, fill=INK)
        c.line([rot((1.5, 2.2), nod, head), rot((3.6, 1.8), nod, head)], INK, 0.7)
        fringe = [rot(p, nod, head) for p in ((-4, -5), (1, -8.5), (2, -5), (6, -6.5), (4.5, -3), (-1, -2.5))]
        c.polygon(fringe, fill=INK, outline=INK, width=1.4)
        c.polygon(fringe, fill=HAIR)
    else:
        draw_mane()
    # the guitar across the hips, its neck up to the right
    gb = rot((3.5, -4), lean * 0.5, hip)
    ga = math.radians(20 + 12 * crouch)
    neck_end = (gb[0] + math.cos(ga) * 27, gb[1] - math.sin(ga) * 27)
    c.line([gb, neck_end], INK, 3.2)
    c.line([gb, neck_end], (130, 80, 40), 1.6)
    c.rect(neck_end[0] - 2.5, neck_end[1] - 2.5, neck_end[0] + 3, neck_end[1] + 2.5, fill=INK, radius=1)
    c.ellipse(gb[0] - 8, gb[1] - 5.5, gb[0] + 6, gb[1] + 6.5, fill=INK)
    c.ellipse(gb[0] - 7, gb[1] - 4.5, gb[0] + 5, gb[1] + 5.5, fill=(60, 120, 240))
    c.ellipse(gb[0] - 5.5, gb[1] - 3.5, gb[0], gb[1] + 1.5, fill=(150, 200, 255))
    c.circle(gb[0] - 0.5, gb[1] + 0.5, 1.3, fill=INK)
    # arms: one strums, one holds the neck
    elbow1 = rot((-5, 7), lean, sh)
    limb(c, [sh, elbow1, (gb[0] - 0.5, gb[1] - 1)], SKIN, 2.8)
    hand2 = (gb[0] + math.cos(ga) * 19, gb[1] - math.sin(ga) * 19)
    elbow2 = ((sh[0] + hand2[0]) / 2 + 1, (sh[1] + hand2[1]) / 2 + 4)
    limb(c, [sh, elbow2, hand2], SKIN, 2.8)
    c.circle(*hand2, 2.0, fill=INK)
    c.circle(*hand2, 1.3, fill=SKIN)
    return c


@sheet(205)
def rock_star(ctx):
    poses = [(0.0, 0.0, 0), (0.25, 0.15, -10), (0.5, 0.35, -15), (0.85, 0.9, 10), (1.0, 1.0, 25), (0.55, 0.45, -25), (0.15, 0.1, -5)]
    out = []
    for n, (cr, bang, flip) in enumerate(poses, 1):
        w, h = ctx.size(n)
        out.append(rocker(w, h, cr, bang, flip))
    return out


# ---------------------------------------------------------------------------
# 1022 Solar System: the planets, Mercury to Pluto (1 to 9), the Moon (10),
# the Sun (11), a nebula (12), a galaxy (13) and the viewer (14) they are
# shown in, with the name under them (the game writes it, in green, at
# the bottom).

def _hash(ix, iy, iz, seed):
    h = (ix * 374761393 + iy * 668265263 + iz * 2147483647 + seed * 144269504) & 0xFFFFFFFF
    h = ((h ^ (h >> 13)) * 1274126177) & 0xFFFFFFFF
    return ((h ^ (h >> 16)) & 0xFFFF) / 65535.0


def vnoise(x, y, z, seed=0):
    ix, iy, iz = math.floor(x), math.floor(y), math.floor(z)
    fx, fy, fz = x - ix, y - iy, z - iz
    fx, fy, fz = fx * fx * (3 - 2 * fx), fy * fy * (3 - 2 * fy), fz * fz * (3 - 2 * fz)
    v = 0.0
    for dx in (0, 1):
        wx = fx if dx else 1 - fx
        for dy in (0, 1):
            wy = fy if dy else 1 - fy
            for dz in (0, 1):
                wz = fz if dz else 1 - fz
                v += wx * wy * wz * _hash(ix + dx, iy + dy, iz + dz, seed)
    return v


def fbm(x, y, z, seed=0, octaves=4):
    v, a, f = 0.0, 0.5, 1.0
    for o in range(octaves):
        v += a * vnoise(x * f, y * f, z * f, seed + o)
        a *= 0.5
        f *= 2.0
    return v / (1 - 0.5 ** octaves)


def sphere(c, cx, cy, r, tex, light=(-0.55, -0.5, 0.67), ambient=0.1, tilt=0.0):
    """A shaded globe: `tex(x, y, z)` gives the colour at a point of the
    unit sphere (y up the axis, z towards the viewer)."""
    from PIL import Image as _I
    from wgfx import SS

    Z = 2
    size = int(math.ceil(2 * r * Z)) + 2
    img = _I.new("RGBA", (size, size), (0, 0, 0, 0))
    px = img.load()
    ln = math.sqrt(sum(v * v for v in light))
    L = [v / ln for v in light]
    ct, st = math.cos(tilt), math.sin(tilt)
    o = size / 2
    for j in range(size):
        v = (j + 0.5 - o) / (r * Z)
        for i in range(size):
            u = (i + 0.5 - o) / (r * Z)
            d = u * u + v * v
            if d >= 1:
                continue
            z = math.sqrt(1 - d)
            nx, ny, nz = u, -v, z
            # the texture's frame, tilted
            tx, ty = nx * ct - ny * st, nx * st + ny * ct
            col = tex(tx, ty, nz)
            lam = max(0.0, nx * L[0] + ny * -L[1] + nz * L[2])
            k = ambient + (1 - ambient) * lam ** 0.8
            k *= 0.85 + 0.15 * z
            a = 255 if d < 1 - 2 / (r * Z) else 200
            px[i, j] = (min(255, int(col[0] * k)), min(255, int(col[1] * k)), min(255, int(col[2] * k)), a)
    big = img.resize((int(size * SS / Z), int(size * SS / Z)), _I.BICUBIC)
    from ._science_kit import _paste_any

    layer = _I.new("RGBA", c.hi.size, (0, 0, 0, 0))
    _paste_any(layer, big, int(round((cx - size / Z / 2) * SS)), int(round((cy - size / Z / 2) * SS)))
    c.hi.alpha_composite(layer)


def lon_lat(x, y, z):
    return math.atan2(x, z), math.asin(max(-1, min(1, y)))


def tex_mercury(x, y, z):
    n = fbm(x * 3, y * 3, z * 3, 11)
    base = mix((120, 112, 104), (196, 186, 172), n)
    for (cx, cy, cz, rr) in CRATERS:
        d = (x - cx) ** 2 + (y - cy) ** 2 + (z - cz) ** 2
        if d < rr * rr:
            t = d / (rr * rr)
            base = mix(base, (90, 84, 80), 0.5) if t < 0.6 else mix(base, (230, 222, 210), 0.5)
    return base


_rng = random.Random(1022)
CRATERS = []
for _ in range(26):
    a, b = _rng.uniform(0, 2 * math.pi), _rng.uniform(-1, 1)
    q = math.sqrt(1 - b * b)
    CRATERS.append((q * math.sin(a), b, q * math.cos(a), _rng.uniform(0.05, 0.16)))


def tex_venus(x, y, z):
    n = fbm(x * 2 + y * 1.5, y * 4, z * 2, 21)
    return mix((200, 170, 110), (250, 236, 200), n)


def tex_earth(x, y, z):
    land = fbm(x * 1.8, y * 1.8, z * 1.8, 31)
    if land > 0.56:
        col = mix((70, 150, 60), (170, 140, 80), min(1, (land - 0.56) * 5))
    else:
        col = mix((20, 60, 160), (40, 110, 210), land / 0.56)
    if abs(y) > 0.86:
        col = (240, 244, 250)
    cl = fbm(x * 3 + 7, y * 6, z * 3, 37)
    if cl > 0.58:
        col = mix(col, (250, 250, 255), min(1, (cl - 0.58) * 5))
    return col


def tex_mars(x, y, z):
    n = fbm(x * 2.5, y * 2.5, z * 2.5, 41)
    col = mix((230, 120, 50), (180, 70, 30), n)
    if n > 0.62:
        col = mix(col, (110, 50, 30), 0.6)
    if y > 0.88:
        col = (250, 240, 230)
    return col


def tex_jupiter(x, y, z):
    w = fbm(x * 2, y * 2, z * 2, 51) * 0.18
    t = (y + w) * 7
    bands = [(240, 226, 200), (196, 150, 110), (236, 210, 170), (160, 110, 80), (230, 220, 200), (200, 160, 120)]
    i = int(math.floor(t)) % len(bands)
    col = mix(bands[i], bands[(i + 1) % len(bands)], (t - math.floor(t)) ** 3)
    # the great red spot
    d = ((x + 0.25) / 0.22) ** 2 + ((y + 0.33) / 0.11) ** 2
    if z > 0 and d < 1:
        col = mix((200, 90, 60), col, d ** 2)
    return col


def tex_saturn(x, y, z):
    t = y * 6 + fbm(x * 2, y * 2, z * 2, 61) * 0.6
    return mix((230, 210, 160), (196, 170, 120), 0.5 + 0.5 * math.sin(t * math.pi))


def tex_uranus(x, y, z):
    return mix((150, 220, 226), (190, 236, 236), 0.5 + 0.3 * math.sin(y * 5) + 0.2 * fbm(x, y * 3, z, 71))


def tex_neptune(x, y, z):
    n = fbm(x * 2, y * 5, z * 2, 81)
    col = mix((30, 70, 200), (70, 130, 240), n)
    d = ((x - 0.15) / 0.2) ** 2 + ((y + 0.15) / 0.1) ** 2
    if z > 0 and d < 1:
        col = mix((20, 30, 110), col, d)
    if 0.22 < y < 0.28 and x > -0.3:
        col = mix(col, (240, 250, 255), 0.7)
    return col


def tex_pluto(x, y, z):
    n = fbm(x * 3, y * 3, z * 3, 91)
    col = mix((150, 130, 110), (220, 200, 176), n)
    d = ((x - 0.1) / 0.35) ** 2 + ((y + 0.15) / 0.3) ** 2
    if z > 0 and d < 1:
        col = mix((246, 236, 220), col, d)
    return col


def tex_moon(x, y, z):
    n = fbm(x * 2, y * 2, z * 2, 101)
    col = mix((210, 210, 206), (160, 160, 158), n)
    if n > 0.58:
        col = mix(col, (110, 110, 112), 0.6)
    for (cx, cy, cz, rr) in CRATERS[:14]:
        d = (x - cx) ** 2 + (y - cy) ** 2 + (z - cz) ** 2
        if d < rr * rr * 0.5:
            col = mix(col, (240, 240, 236), 0.5)
    return col


def tex_sun(x, y, z):
    n = fbm(x * 8, y * 8, z * 8, 111, 2)
    return mix((255, 200, 40), (255, 250, 150), n * 0.8 + 0.2 * z)


def planet_frame(ctx, n, tex, light=(-0.55, -0.5, 0.67), ambient=0.1, tilt=0.0, pad=1.0):
    w, h = ctx.size(n)
    c = ctx.canvas(n)
    sphere(c, w / 2, h / 2, min(w, h) / 2 - pad, tex, light, ambient, tilt)
    return c


def saturn(ctx, n):
    w, h = ctx.size(n)
    c = ctx.canvas(n)
    cx, cy = w / 2, h / 2
    tilt = math.radians(-12)

    def ring(front):
        from PIL import Image as _I, ImageDraw as _D
        from wgfx import SS

        layer = _I.new("RGBA", c.hi.size, (0, 0, 0, 0))
        d = _D.Draw(layer)
        for rr, col, wd in ((42, (150, 130, 100), 2.0), (39, (220, 200, 160), 3.2), (35, (196, 176, 136), 2.4), (31.5, (160, 140, 110), 1.6)):
            box = [(cx - rr) * SS, (cy - rr * 0.32) * SS, (cx + rr) * SS, (cy + rr * 0.32) * SS]
            d.arc(box, 0 if front else 180, 180 if front else 360, fill=col + (255,), width=int(wd * SS))
        layer = layer.rotate(math.degrees(-tilt), resample=_I.BICUBIC, center=(cx * SS, cy * SS))
        c.hi.alpha_composite(layer)

    ring(False)
    sphere(c, cx, cy, 15.5, tex_saturn, tilt=tilt)
    ring(True)
    return c


def nebula(ctx, n):
    w, h = ctx.size(n)
    c = ctx.canvas(n)
    cx, cy = w / 2, h / 2
    rng = random.Random(1212)
    for s in (-1, 1):
        for k in range(26):
            t = k / 25
            y = cy + s * (6 + t * 36)
            rr = 5 + 16 * math.sin(t * math.pi * 0.9) + rng.uniform(-2, 2)
            x = cx + rng.uniform(-3, 3) + s * t * 4
            col = mix((60, 200, 230), (20, 50, 140), t)
            c.ellipse(x - rr, y - rr * 0.6, x + rr, y + rr * 0.6, fill=col)
        for k in range(30):
            t = rng.uniform(0.1, 0.9)
            y = cy + s * (6 + t * 36)
            x = cx + rng.uniform(-12, 12) * math.sin(t * math.pi)
            c.circle(x, y, rng.uniform(1, 2.5), fill=mix((150, 240, 255), (60, 120, 220), t))
    c.ellipse(cx - 6, cy - 10, cx + 6, cy + 10, fill=(250, 150, 50))
    c.ellipse(cx - 3, cy - 6, cx + 3, cy + 6, fill=(255, 220, 120))
    c.circle(cx, cy, 2.2, fill=WHITE)
    return c


def galaxy(ctx, n):
    w, h = ctx.size(n)
    c = ctx.canvas(n)
    cx, cy = w / 2, h / 2
    rng = random.Random(1313)
    for _ in range(140):
        c.pixel(rng.randrange(w), rng.randrange(h), rng.choice([(200, 200, 255), (255, 255, 255), (150, 150, 200)]))
    for arm in range(2):
        for k in range(320):
            t = k / 320
            a = arm * math.pi + t * 3.6 * math.pi * 0.9
            rr = 4 + t * 40
            x = cx + math.cos(a) * rr + rng.gauss(0, 1.5 + 3 * t)
            y = cy + math.sin(a) * rr * 0.86 + rng.gauss(0, 1.5 + 3 * t)
            col = mix((255, 200, 140), (110, 60, 200), min(1, t * 1.4))
            c.circle(x, y, 1.6 - t * 0.9, fill=col)
    c.circle(cx, cy, 9, fill=(255, 150, 60))
    c.circle(cx, cy, 6, fill=(255, 210, 120))
    c.circle(cx, cy, 3.2, fill=(255, 250, 220))
    return c


def sun(ctx, n):
    w, h = ctx.size(n)
    c = ctx.canvas(n)
    cx, cy = w / 2, h / 2
    r = min(w, h) / 2 - 1
    rng = random.Random(1111)
    for k in range(22):
        a = k / 22 * 2 * math.pi + rng.uniform(-0.1, 0.1)
        ln = rng.uniform(1.5, 3.5)
        c.line([(cx + math.cos(a) * (r - 3), cy + math.sin(a) * (r - 3)), (cx + math.cos(a) * (r - 3 + ln + 2), cy + math.sin(a) * (r - 3 + ln + 2))], (240, 90, 20), 2.2)
    sphere(c, cx, cy, r - 2.5, tex_sun, (0, 0, 1), 0.75)
    for _ in range(9):
        a, d = rng.uniform(0, 2 * math.pi), rng.uniform(0, r * 0.7)
        c.circle(cx + math.cos(a) * d, cy + math.sin(a) * d, rng.uniform(0.7, 1.4), fill=(210, 120, 30))
    return c


def viewer(ctx):
    c = ctx.canvas(14)
    c.panel(0, 0, 108, 109, STEEL, radius=4, bevel=1.5)
    c.inset(5, 5, 103, 96, (0, 0, 0), radius=2, depth=1.5)
    c.inset(7, 96, 101, 108, (0, 14, 0), radius=1, depth=1)
    for x, y in ((2.5, 2.5), (105.5, 2.5)):
        c.screw(x, y, 1.2)
    knots(c, ctx.knots(), ["r"], reach=5)
    return c


@sheet(1022)
def solar_system(ctx):
    side = (0.7, -0.35, 0.6)
    return [
        planet_frame(ctx, 1, tex_mercury, side),
        planet_frame(ctx, 2, tex_venus),
        planet_frame(ctx, 3, tex_earth, tilt=math.radians(-20)),
        planet_frame(ctx, 4, tex_mars),
        planet_frame(ctx, 5, tex_jupiter),
        saturn(ctx, 6),
        planet_frame(ctx, 7, tex_uranus),
        planet_frame(ctx, 8, tex_neptune),
        planet_frame(ctx, 9, tex_pluto),
        planet_frame(ctx, 10, tex_moon, ambient=0.2),
        sun(ctx, 11),
        nebula(ctx, 12),
        galaxy(ctx, 13),
        viewer(ctx),
        ctx.blank(15),
    ]
