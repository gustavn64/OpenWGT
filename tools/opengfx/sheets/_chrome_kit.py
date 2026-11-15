"""Helpers for the editor's chrome (sheets/chrome.py) and the pointers
(sheets/cursors.py): putting canvases together, a turned frame of
reference for drawing at an angle, and the cartoon gloves, scissors and
plugs the pointers and the tools show."""
from __future__ import annotations

import math

from wgfx import BRASS, BRASS_D, INK, SS, STEEL, STEEL_D, STEEL_L, WHITE, Canvas, darker, lighter, mix

GLOVE = (250, 250, 252)
GLOVE_SH = (196, 202, 214)
CUFF = (40, 110, 220)
CUFF_D = (20, 60, 150)


def place(dst: Canvas, src: Canvas, x: int, y: int):
    """Draws canvas `src` (both layers) onto `dst` at (x, y)."""
    dst.hi.alpha_composite(src.hi, (int(x * SS), int(y * SS)))
    dst.lo.alpha_composite(src.lo, (int(x), int(y)))
    dst.dither_areas += [(a + x, b + y, c + x, d + y) for a, b, c, d in src.dither_areas]


class Xf:
    """Drawing on `c` in a frame of reference moved to (ox, oy), scaled by
    `s`, turned by `rot` degrees (clockwise on the screen) and mirrored
    left-right first if `flip`."""

    def __init__(self, c: Canvas, ox, oy, s=1.0, rot=0.0, flip=False):
        self.c, self.ox, self.oy, self.s, self.flip = c, ox, oy, s, flip
        a = math.radians(rot)
        self.cs, self.sn = math.cos(a), math.sin(a)

    def p(self, x, y):
        if self.flip:
            x = -x
        return (self.ox + (x * self.cs - y * self.sn) * self.s, self.oy + (x * self.sn + y * self.cs) * self.s)

    def cap(self, a, b, w, color):
        """A capsule (a thick line with round ends) from a to b, `w` wide."""
        self.c.line([self.p(*a), self.p(*b)], color, w * self.s)

    def poly(self, pts, fill):
        self.c.polygon([self.p(*q) for q in pts], fill=fill)

    def ell(self, cx, cy, rx, ry, fill, ang=0.0, n=40):
        a = math.radians(ang)
        pts = []
        for i in range(n):
            t = 2 * math.pi * i / n
            x, y = rx * math.cos(t), ry * math.sin(t)
            pts.append((cx + x * math.cos(a) - y * math.sin(a), cy + x * math.sin(a) + y * math.cos(a)))
        self.poly(pts, fill)

    def rrect(self, x0, y0, x1, y1, r, fill, n=6):
        """A rounded rectangle (as a polygon, so it turns)."""
        pts = []
        for cx, cy, a0 in ((x1 - r, y0 + r, -90), (x1 - r, y1 - r, 0), (x0 + r, y1 - r, 90), (x0 + r, y0 + r, 180)):
            for i in range(n + 1):
                a = math.radians(a0 + 90 * i / n)
                pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
        self.poly(pts, fill)


# ---------------------------------------------------------------------------
# Shapes with an outline: each is drawn twice, first `o` wider in INK, then
# filled, part by part from the back, so the parts in front keep a line
# where they cross the ones behind.


class Shapes:
    def __init__(self, xf: Xf, o=1.0):
        self.x = xf
        self.o = o / xf.s  # outline width in local units

    def cap(self, a, b, w, fill, shade=None):
        self.x.cap(a, b, w + 2 * self.o, INK)
        self.x.cap(a, b, w, fill)
        if shade:
            # a shadow along one side: the capsule again, narrower, moved.
            sx, sy, col = shade
            self.x.cap((a[0] + sx, a[1] + sy), (b[0] + sx, b[1] + sy), w * 0.45, col)

    def ell(self, cx, cy, rx, ry, fill, ang=0.0):
        self.x.ell(cx, cy, rx + self.o, ry + self.o, INK, ang)
        self.x.ell(cx, cy, rx, ry, fill, ang)

    def rrect(self, x0, y0, x1, y1, r, fill):
        o = self.o
        self.x.rrect(x0 - o, y0 - o, x1 + o, y1 + o, r + o, INK)
        self.x.rrect(x0, y0, x1, y1, r, fill)

    def poly(self, pts, fill):
        # outline: the polygon's edges as INK capsules, then the fill.
        n = len(pts)
        for i in range(n):
            self.x.cap(pts[i], pts[(i + 1) % n], 2 * self.o, INK)
        self.x.poly(pts, fill)


# ---------------------------------------------------------------------------
# Gloves. Local units: a hand about 20 units from the cuff to the finger
# tips; fingers point up (−y) before turning.


def cuff(sh: Shapes, y0=9.0, w=5.0, col=CUFF):
    sh.rrect(-w, y0, w, y0 + 4.5, 1.5, col)
    sh.x.rrect(-w + 0.6, y0 + 0.6, w - 0.6, y0 + 1.6, 0.6, lighter(col, 0.45))


def glove_point(sh: Shapes, cuffed=True):
    """A gloved hand with the index finger out, pointing up; the tip at
    (−2.2, −13)."""
    x = sh.x
    if cuffed:
        cuff(sh, 8.0, 4.6)
    # thumb, behind on the left
    sh.cap((-4.2, 3.5), (-6.8, -1.2), 3.4, GLOVE)
    # palm
    sh.rrect(-5.0, -3.5, 5.2, 9.0, 3.6, GLOVE)
    x.ell(1.8, 4.5, 2.6, 3.2, GLOVE_SH)
    x.ell(1.0, 3.8, 2.6, 3.0, GLOVE)
    # the curled fingers: knuckles across the palm's right
    for i, yy in enumerate((-1.0, 2.0, 5.0)):
        sh.cap((0.6, yy), (5.6, yy + 0.3), 3.0, GLOVE)
    # index finger
    sh.cap((-2.4, 1.0), (-2.2, -11.4), 3.6, GLOVE)
    x.cap((-1.2, 0.5), (-1.0, -10.5), 0.9, GLOVE_SH)


def glove_open(sh: Shapes, cuffed=True):
    """An open hand, palm towards us, fingers spread, pointing up; palm
    centre at (0, 2)."""
    x = sh.x
    if cuffed:
        cuff(sh, 8.5, 4.4)
    # thumb out to the left
    sh.cap((-3.0, 5.0), (-8.6, -0.4), 3.4, GLOVE)
    # the four fingers
    for bx, tx, ty in ((-3.4, -7.0, -9.6), (-1.1, -2.6, -12.6), (1.3, 2.4, -12.6), (3.6, 6.8, -9.8)):
        sh.cap((bx, -1.0), (tx, ty), 2.8, GLOVE)
    sh.rrect(-5.2, -3.2, 5.2, 9.0, 3.4, GLOVE)
    # the palm's lines
    x.cap((-2.6, 3.0), (2.4, 2.0), 0.6, GLOVE_SH)
    x.cap((-1.6, 5.4), (2.0, 4.6), 0.6, GLOVE_SH)
    for bx in (-2.4, -0.6, 1.3, 3.0):
        x.cap((bx, -3.4), (bx * 1.05, -1.9), 0.7, GLOVE_SH)


def glove_fist(sh: Shapes, cuffed=True):
    """A closed hand gripping (knuckles up, seen from the back)."""
    x = sh.x
    if cuffed:
        cuff(sh, 7.5, 4.8)
    sh.rrect(-5.6, -4.0, 5.6, 8.5, 3.8, GLOVE)
    # four bent fingers over the top
    for i, bx in enumerate((-4.2, -1.4, 1.4, 4.2)):
        sh.cap((bx, -3.6), (bx, -1.2), 2.8, GLOVE)
    for bx in (-2.8, 0.0, 2.8):
        x.cap((bx, -0.4), (bx, 1.8), 0.6, GLOVE_SH)
    # thumb across the front
    sh.cap((-5.6, 4.0), (0.8, 1.8), 3.2, GLOVE)


def glove_hold(sh: Shapes, cuffed=True):
    """A hand holding something across its fingers (a plug): fingers wrap
    at y ≈ −2, from the right."""
    x = sh.x
    if cuffed:
        cuff(sh, 7.5, 4.6)
    sh.rrect(-4.6, -4.0, 5.8, 8.5, 3.6, GLOVE)
    for bx in (-2.6, 0.2, 3.0):
        x.cap((bx, 2.0), (bx + 0.4, 4.6), 0.6, GLOVE_SH)


def fingers_over(sh: Shapes, y0=-6.5, n=3):
    """Fingers wrapped round a thing held across the hand (drawn after it)."""
    for i in range(n):
        yy = y0 + i * 2.6
        sh.cap((-3.2, yy), (3.0, yy + 0.4), 2.6, GLOVE)


# ---------------------------------------------------------------------------
# Scissors and a plug


def scissors(c: Canvas, ox, oy, s=1.0, rot=0.0, handle=(220, 40, 40), hole=(0, 0, 0, 0)):
    """Open scissors, the pivot at (ox, oy); local: the blades point up
    (−y), the finger rings below."""
    x = Xf(c, ox, oy, s, rot)
    sh = Shapes(x, 1.0)
    for side in (-1, 1):
        cx, cy = side * 4.4, 7.4
        sh.cap((0.0, 0.0), (cx * 0.8, cy - 2.4), 2.6, handle)
    for side in (-1, 1):
        cx, cy = side * 4.4, 7.4
        x.ell(cx, cy, 3.7 + sh.o, 3.4 + sh.o, INK)
        x.ell(cx, cy, 3.7, 3.4, handle)
        x.ell(cx + side * 0.2, cy + 0.2, 1.5 + sh.o, 1.3 + sh.o, INK)
        x.ell(cx + side * 0.2, cy + 0.2, 1.5, 1.3, hole)
        x.cap((cx - 2.0, cy - 2.0), (cx - 0.6, cy - 2.8), 0.7, lighter(handle, 0.5))
    # blades, crossing at the pivot: each a long wedge
    for side in (1, -1):
        tx, ty = -side * 5.0, -12.5
        bx, by = side * 1.4, 2.4
        dx, dy = tx - bx, ty - by
        n = math.hypot(dx, dy)
        nx, ny = -dy / n * 1.5, dx / n * 1.5
        sh.poly([(bx + nx, by + ny), (bx - nx, by - ny), (tx, ty)], STEEL_L)
        x.poly([(bx - nx * side, by - ny * side), (bx, by), (tx, ty)], STEEL)
    x.ell(0, 0, 1.4, 1.4, INK)
    x.ell(0, 0, 0.7, 0.7, STEEL)


def plug(sh: Shapes, col, tip=(-12.0, 0.0)):
    """A wire plug lying across, its pin pointing left to `tip`; body from
    x −6 to 4 at y 0; the cable out to the right."""
    x = sh.x
    tx, ty = tip
    # cable
    sh.cap((3.5, 0.0), (11.0, 1.5), 2.2, darker(col, 0.25))
    # pin
    sh.cap((tx + 1.0, ty), (-5.0, 0.0), 1.6, STEEL_L)
    # body
    sh.rrect(-6.5, -2.8, 4.5, 2.8, 1.4, col)
    x.rrect(-5.8, -2.1, 3.8, -0.9, 0.5, lighter(col, 0.5))
    x.rrect(-5.8, 0.9, 3.8, 2.1, 0.5, darker(col, 0.3))


# ---------------------------------------------------------------------------
# Doc, the help character: a cartoon scientist of our own (round goggles,
# wild white side hair, a big grin). Local units: the head is about 20
# across, centred on (0, 0).

SKIN = (246, 196, 160)
SKIN_D = (214, 140, 110)
HAIR = (236, 238, 244)
HAIR_D = (180, 186, 200)
HAT = (255, 206, 30)


def doc_face(xf: Xf, o=1.0, hat=False, detail=False):
    sh = Shapes(xf, o)
    x = xf
    # wild hair at the sides and the back
    tufts = [(-10.2, -2.0, 3.6), (-11.4, 2.6, 3.2), (-9.6, 6.4, 2.6), (-8.4, -6.4, 3.4),
             (10.2, -2.0, 3.6), (11.4, 2.6, 3.2), (9.6, 6.4, 2.6), (8.4, -6.4, 3.4)]
    if not hat:
        tufts += [(-4.6, -10.0, 2.8), (0.6, -11.4, 2.6), (5.0, -10.0, 2.6)]
    for cx, cy, r in tufts:
        x.ell(cx, cy, r + sh.o, r + sh.o, INK)
    for cx, cy, r in tufts:
        x.ell(cx, cy, r, r, HAIR)
        if detail:
            x.ell(cx + r * 0.25, cy + r * 0.3, r * 0.55, r * 0.45, HAIR_D)
            x.ell(cx, cy, r * 0.6, r * 0.5, HAIR)
    # ears and head
    for s in (-1, 1):
        sh.ell(s * 9.4, 1.6, 2.0, 2.8, SKIN)
    sh.ell(0, 0.4, 9.2, 10.2, SKIN)
    x.ell(-2.5, -5.5, 3.6, 2.2, lighter(SKIN, 0.4))
    for s in (-1, 1):
        x.ell(s * 6.0, 4.2, 1.8, 1.2, (250, 150, 140))
    # eyebrows
    for s in (-1, 1):
        sh.cap((s * 7.0, -4.6), (s * 2.0, -5.8), 1.6, HAIR)
    # goggles
    x.cap((-9.0, -1.2), (9.0, -1.2), 1.4, darker(BRASS_D, 0.3))
    for s in (-1, 1):
        x.ell(s * 4.0, -1.0, 3.6 + sh.o, 3.6 + sh.o, INK)
        x.ell(s * 4.0, -1.0, 3.6, 3.6, BRASS)
        x.ell(s * 4.0, -1.0, 2.6, 2.6, (170, 236, 240))
        x.ell(s * 3.6, -0.6, 1.0, 1.0, INK)
        x.ell(s * 4.0 - 1.2, -2.2, 0.7, 0.7, WHITE)
    # nose
    sh.ell(0, 3.0, 2.3, 2.0, SKIN_D)
    x.ell(-0.7, 2.4, 0.8, 0.6, lighter(SKIN, 0.3))
    # the grin
    pts = [(4.2, 5.6)] + [(math.cos(math.radians(a)) * 4.2, 5.6 + math.sin(math.radians(a)) * 3.6) for a in range(0, 181, 15)]
    sh.poly(pts, (120, 20, 30))
    x.poly([(-3.6, 5.9), (3.6, 5.9), (3.2, 7.0), (-3.2, 7.0)], WHITE)
    x.ell(0.6, 8.2, 1.8, 0.9, (240, 110, 120))
    if hat:
        # a hard hat: the dome, a ridge and the brim
        dome = [(math.cos(math.radians(a)) * 9.6, -5.6 + math.sin(math.radians(a)) * 8.4) for a in range(180, 361, 10)]
        sh.poly(dome, HAT)
        x.ell(-3.6, -10.2, 2.4, 1.4, lighter(HAT, 0.6))
        sh.rrect(-1.4, -14.2, 1.4, -6.0, 1.2, lighter(HAT, 0.2))
        sh.rrect(-12.4, -6.6, 12.4, -4.4, 1.1, darker(HAT, 0.08))
