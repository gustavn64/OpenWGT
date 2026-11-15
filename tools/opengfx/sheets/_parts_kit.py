"""Helpers for sheets/parts.py: cropping and rotating drawings, gears,
pellets, keycap tiles, a heart and other small shapes. Drawn with the
kit's primitives only."""
from __future__ import annotations

import math

from PIL import Image, ImageDraw

from wgfx import INK, SS, STEEL, STEEL_D, STEEL_L, WHITE, Canvas, darker, lighter, mix

CREAM = (250, 244, 226)


def crop(c: Canvas, x0: int, y0: int, w: int, h: int) -> Canvas:
    """The part x0..x0+w × y0..y0+h of a drawing as a canvas of its own
    (pixel-exact: the same pixels come out as in the big one)."""
    n = Canvas(w, h)
    n.hi = c.hi.crop((x0 * SS, y0 * SS, (x0 + w) * SS, (y0 + h) * SS))
    n.d = ImageDraw.Draw(n.hi)
    n.lo = c.lo.crop((x0, y0, x0 + w, y0 + h))
    n.ld = ImageDraw.Draw(n.lo)
    n.dither_areas = [(a - x0, b - y0, cc - x0, d - y0) for a, b, cc, d in c.dither_areas]
    return n


def layer(c: Canvas) -> Canvas:
    """A blank canvas the size of `c`, to draw on and then `stamp`."""
    return Canvas(c.w, c.h)


def stamp(c: Canvas, l: Canvas, angle=0.0, cx=None, cy=None, dx=0.0, dy=0.0):
    """Draws layer `l`'s smooth drawing over `c`, turned `angle` degrees
    (counter-clockwise) about (cx, cy) and moved by dx, dy."""
    img = l.hi
    if angle:
        cx = c.w / 2 if cx is None else cx
        cy = c.h / 2 if cy is None else cy
        img = img.rotate(angle, resample=Image.BICUBIC, center=(cx * SS, cy * SS))
    if dx or dy:
        base = Image.new("RGBA", img.size, (0, 0, 0, 0))
        base.alpha_composite(img, (int(round(dx * SS)), int(round(dy * SS))))
        img = base
    c.hi.alpha_composite(img)
    c.lo.alpha_composite(l.lo)


def gear(c: Canvas, cx, cy, r, teeth, angle, base=STEEL, hub=None):
    """A toothed wheel turned `angle` degrees."""
    pts = []
    n = teeth * 4
    for i in range(n):
        a = math.radians(angle) + 2 * math.pi * i / n
        rr = r if (i % 4) in (0, 1) else r * 0.74
        pts.append((cx + math.cos(a) * rr, cy + math.sin(a) * rr))
    c.polygon(pts, fill=INK)
    pts2 = []
    for i in range(n):
        a = math.radians(angle) + 2 * math.pi * i / n
        rr = (r - 0.9) if (i % 4) in (0, 1) else r * 0.74 - 0.9
        pts2.append((cx + math.cos(a) * rr, cy + math.sin(a) * rr))
    c.polygon(pts2, fill=base)
    c.circle(cx, cy, r * 0.55, fill=darker(base, 0.15))
    c.circle(cx - 0.4, cy - 0.4, r * 0.48, fill=lighter(base, 0.2))
    c.circle(cx, cy, r * 0.2 + 0.4, fill=INK)
    c.circle(cx, cy, r * 0.2, fill=hub or darker(base, 0.5))
    # a spoke so the turning shows
    a = math.radians(angle + 45)
    c.line([(cx + math.cos(a) * r * 0.25, cy + math.sin(a) * r * 0.25),
            (cx + math.cos(a) * r * 0.5, cy + math.sin(a) * r * 0.5)], darker(base, 0.45), 1.0)


def pellet(c: Canvas, cx, cy, r=2.2, color=(255, 210, 40)):
    """A little glowing data bead."""
    c.circle(cx, cy, r + 0.7, fill=INK)
    c.radial(cx, cy, r, lighter(color, 0.7), darker(color, 0.25), -0.4, -0.4)


def channel(c: Canvas, x0, y0, x1, y1, fill=(16, 20, 28)):
    """A sunk groove things run along."""
    c.rect(x0, y0, x1, y1, fill=INK, radius=1.5)
    c.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, fill=fill, radius=1)
    c.rect(x0 + 1, y1 - 2, x1 - 1, y1 - 1, fill=lighter(fill, 0.18))


def bezel(c: Canvas, x0, y0, x1, y1, inner, pad=3, base=STEEL):
    """A raised steel frame round the window x0..x1 × y0..y1 (`inner` the
    window's own colour)."""
    c.rect(x0 - pad, y0 - pad, x1 + pad, y1 + pad, fill=INK, radius=2)
    c.rect(x0 - pad + 1, y0 - pad + 1, x1 + pad - 1, y1 + pad - 1, fill=STEEL_L, radius=1.5)
    c.rect(x0 - pad + 2, y0 - pad + 2, x1 + pad - 1, y1 + pad - 1, fill=STEEL_D, radius=1.5)
    c.rect(x0 - pad + 2, y0 - pad + 2, x1 + pad - 2, y1 + pad - 2, fill=base, radius=1)
    c.rect(x0 - 1, y0 - 1, x1 + 1, y1 + 1, fill=INK)
    c.rect(x0, y0, x1, y1, fill=inner)


def leds(c: Canvas, x0, y, n, lit, gap=5.0, on=(255, 70, 40), off=(80, 20, 20)):
    """A row of little lamps, the ones in `lit` on."""
    for i in range(n):
        cx = x0 + i * gap
        c.circle(cx, y, 1.9, fill=INK)
        c.circle(cx, y, 1.3, fill=on if i in lit else off)
        if i in lit:
            c.pixel(int(cx - 0.5), int(y - 0.5), lighter(on, 0.6))


def vents(c: Canvas, x0, y0, x1, n, base):
    for i in range(n):
        y = y0 + i * 2.5
        c.rect(x0, y, x1, y + 1.2, fill=darker(base, 0.55), radius=0.5)


def tile(c: Canvas, x0, y0, x1, y1, base=CREAM, radius=2.0):
    """A key cap: the background of the little icons the code blits into
    windows."""
    c.rect(x0, y0, x1, y1, fill=INK, radius=radius)
    c.gradient(x0 + 1, y0 + 1, x1 - 1, y1 - 1, [WHITE, base, darker(base, 0.12)], radius=max(0, radius - 1))
    c.gradient(x0 + 2, y1 - 2, x1 - 1, y1 - 1, [darker(base, 0.28), darker(base, 0.28)])
    c.gradient(x1 - 2, y0 + 2, x1 - 1, y1 - 1, [darker(base, 0.2), darker(base, 0.28)])


def heart_pts(cx, cy, sx, sy, n=72):
    """The classic heart outline, centred near (cx, cy), `sx`, `sy` pixels
    per unit (it's 32 units wide and about 29 tall)."""
    pts = []
    for i in range(n):
        t = 2 * math.pi * i / n
        x = 16 * math.sin(t) ** 3
        y = 13 * math.cos(t) - 5 * math.cos(2 * t) - 2 * math.cos(3 * t) - math.cos(4 * t)
        pts.append((cx + x * sx, cy - y * sy))
    return pts


def scale_pts(pts, cx, cy, k):
    return [(cx + (x - cx) * k, cy + (y - cy) * k) for x, y in pts]


def star_pts(cx, cy, r0, r1, n, rot=0.0, jitter=None):
    pts = []
    for i in range(2 * n):
        a = math.radians(rot) + math.pi * i / n
        r = r1 if i % 2 == 0 else r0
        if jitter:
            r *= jitter[i % len(jitter)]
        pts.append((cx + math.cos(a) * r, cy + math.sin(a) * r))
    return pts


def plank(c: Canvas, x0, y0, x1, y1, base=(150, 96, 52), seed=0):
    """A wooden board with grain and nails."""
    c.rect(x0, y0, x1, y1, fill=INK, radius=1.5)
    c.gradient(x0 + 1, y0 + 1, x1 - 1, y1 - 1, [lighter(base, 0.25), base, darker(base, 0.2)], radius=1)
    h = y1 - y0
    for k in range(1, 4):
        yy = y0 + h * k / 4 + ((seed + k) % 3 - 1) * 0.6
        c.line([(x0 + 3 + (seed * 7 + k * 5) % 9, yy), (x1 - 4 - (seed * 3 + k * 11) % 13, yy + 0.4)],
               darker(base, 0.3), 0.6, round_ends=False)


def nail(c: Canvas, x, y):
    c.circle(x, y, 1.1, fill=INK)
    c.circle(x - 0.2, y - 0.2, 0.6, fill=STEEL_L)


def arrow_down(c: Canvas, cx, top, bottom, shaft, head, head_len, fill, outline=INK, width=1.0):
    pts = [(cx - shaft / 2, top), (cx + shaft / 2, top), (cx + shaft / 2, bottom - head_len),
           (cx + head / 2, bottom - head_len), (cx, bottom), (cx - head / 2, bottom - head_len),
           (cx - shaft / 2, bottom - head_len)]
    c.polygon(pts, fill=fill, outline=outline, width=width)
    return pts


def grass(c: Canvas, x, y, n=5, h=8, spread=8, color=(70, 160, 50), seed=0):
    for i in range(n):
        f = (i - (n - 1) / 2) / max(1, (n - 1) / 2)
        lean = f * spread / 2 + ((seed + i * 7) % 5 - 2) * 0.5
        hh = h * (0.7 + ((seed * 3 + i * 5) % 4) * 0.12)
        x0 = x + f * spread / 3
        c.polygon([(x0 - 1.2, y), (x0 + 1.2, y), (x0 + lean, y - hh)],
                  fill=mix(color, (30, 90, 20), ((seed + i) % 3) / 3), outline=darker(color, 0.55), width=0.5)
