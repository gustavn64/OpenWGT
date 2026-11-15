"""Helpers for the number parts (sheets/numbers.py): pixel-exact segment
characters of any size, a tiny pixel font, keys, and stamping one drawing
onto another's pixel layer so that a digit frame and the body's cell under
it are the same pixels."""
from __future__ import annotations

from PIL import Image

from wgfx import INK, Canvas, darker, lighter

# ---------------------------------------------------------------------------
# Seven segments, as convex polygons sampled at the pixels' centres.

SEGS = {
    "0": "abcdef", "1": "bc", "2": "abged", "3": "abgcd", "4": "fgbc", "5": "afgcd",
    "6": "afgedc", "7": "abc", "8": "abcdefg", "9": "abcdfg", "-": "g", "E": "afged",
    " ": "", "+": "gp", ".": "q",
}


def _hseg(x0, x1, y, r):
    return [(x0, y), (x0 + r, y - r), (x1 - r, y - r), (x1, y), (x1 - r, y + r), (x0 + r, y + r)]


def _vseg(x, y0, y1, r):
    return [(x, y0), (x + r, y0 + r), (x + r, y1 - r), (x, y1), (x - r, y1 - r), (x - r, y0 + r)]


def seg_polys(w, h, t, gap=0.3, slant=0.0, shrink=0.0):
    """The segments of a w × h cell, thickness t: {name: polygon}. `shrink`
    narrows them all (for a lit segment's bright core)."""
    sw = w - slant * h
    r = t / 2
    # The middle segment on whole pixels: for an even t, ym on a pixel edge.
    ym = (h - t) // 2 + r
    xl, xr, yt, yb = r, sw - r, r, h - r
    rr = max(0.01, r - shrink)
    g2 = gap + shrink
    p = {
        "a": _hseg(xl + g2, xr - g2, yt, rr),
        "g": _hseg(xl + g2, xr - g2, ym, rr),
        "d": _hseg(xl + g2, xr - g2, yb, rr),
        "f": _vseg(xl, yt + g2, ym - g2, rr),
        "b": _vseg(xr, yt + g2, ym - g2, rr),
        "e": _vseg(xl, ym + g2, yb - g2, rr),
        "c": _vseg(xr, ym + g2, yb - g2, rr),
        # The '+''s upright and the '.'.
        "p": _vseg(sw / 2, ym - (yb - yt) * 0.28 + shrink, ym + (yb - yt) * 0.28 - shrink, rr),
    }
    dr = max(1.0, t * 0.6) - shrink
    cx = sw / 2
    p["q"] = [(cx - dr, yb - dr), (cx + dr, yb - dr), (cx + dr, yb + dr), (cx - dr, yb + dr)]
    if slant:
        p = {k: [(x + slant * (h - y), y) for x, y in v] for k, v in p.items()}
    return p


def _inside(poly, px, py):
    n = len(poly)
    sign = 0
    for i in range(n):
        x0, y0 = poly[i]
        x1, y1 = poly[(i + 1) % n]
        c = (x1 - x0) * (py - y0) - (y1 - y0) * (px - x0)
        if abs(c) < 1e-9:
            c = 1e-9  # on an edge: count it in
        s = 1 if c > 0 else -1
        if sign == 0:
            sign = s
        elif s != sign:
            return False
    return True


def raster(poly, w, h):
    xs = [x for x, _ in poly]
    ys = [y for _, y in poly]
    out = []
    for y in range(max(0, int(min(ys)) - 1), min(h, int(max(ys)) + 2)):
        for x in range(max(0, int(min(xs)) - 1), min(w, int(max(xs)) + 2)):
            if _inside(poly, x + 0.5, y + 0.5):
                out.append((x, y))
    return out


def box_segs(w, h, t):
    """Plain box segments for small cells, overlapping at the corners
    (rounded-looking, solid at 1x): {name: pixels}."""
    gy = (h - t) // 2
    up, lo = (1, gy + t // 2), (gy + t - t // 2, h - 1)

    def box(x0, y0, x1, y1):
        return [(x, y) for y in range(y0, y1) for x in range(x0, x1)]

    cx = w // 2 - (1 if t > 1 and w % 2 == 0 else 0)
    return {
        "a": box(1, 0, w - 1, t), "g": box(1, gy, w - 1, gy + t), "d": box(1, h - t, w - 1, h),
        "f": box(0, up[0], t, up[1]), "b": box(w - t, up[0], w, up[1]),
        "e": box(0, lo[0], t, lo[1]), "c": box(w - t, lo[0], w, lo[1]),
        "p": box(cx - (t - 1) // 2, gy - (h // 5), cx - (t - 1) // 2 + t, gy + t + (h // 5)),
        "q": box(cx - (t - 1) // 2, h - t, cx - (t - 1) // 2 + t, h),
    }


def seg_char_box(c: Canvas, x, y, w, h, ch, on=None, off=None, t=2):
    """A segment character with box segments (see box_segs)."""
    segs = box_segs(w, h, t)
    lit = SEGS.get(ch, "")
    for k, pix in segs.items():
        if k in "pq" and k not in lit:
            continue
        col = on if k in lit else off
        if col is None:
            continue
        for px, py in pix:
            c.pixel(x + px, y + py, col)


def seg_char(c: Canvas, x, y, w, h, ch, on=None, off=None, core=None, t=2, gap=0.3, slant=0.0, cw=None):
    """A segment character in the box x, y, w × h on c's pixel layer: lit
    segments `on` (with a brighter `core` inside, if given), the others
    `off` (None: left alone). `cw` clips to a narrower frame."""
    cw = w if cw is None else cw
    polys = seg_polys(w, h, t, gap, slant)
    lit = SEGS.get(ch, "")
    for k, poly in polys.items():
        if k in "pq" and k not in lit:
            continue
        col = on if k in lit else off
        if col is None:
            continue
        for px, py in raster(poly, cw, h):
            c.pixel(x + px, y + py, col)
    if core and t >= 3:
        cp = seg_polys(w, h, t, gap, slant, shrink=t / 2 - 0.55)
        for k in lit:
            for px, py in raster(cp[k], cw, h):
                c.pixel(x + px, y + py, core)


# ---------------------------------------------------------------------------
# A tiny pixel font (bold, for labels on lit indicators).

GLYPHS = {
    "A": ["0111110", "1110111", "1100011", "1111111", "1100011", "1100011"],
    "M": ["1100011", "1110111", "1111111", "1101011", "1100011", "1100011"],
    "P": ["1111110", "1100011", "1100011", "1111110", "1100000", "1100000"],
    "O": ["0111110", "1100011", "1100011", "1100011", "1100011", "0111110"],
    "N": ["1100011", "1110011", "1111011", "1101111", "1100111", "1100011"],
    "F": ["1111111", "1100000", "1111110", "1100000", "1100000", "1100000"],
}


def pixel_text(c: Canvas, x, y, s, col, gap=2):
    for ch in s:
        g = GLYPHS[ch]
        for j, row in enumerate(g):
            for i, b in enumerate(row):
                if b == "1":
                    c.pixel(x + i, y + j, col)
        x += len(g[0]) + gap


def pixel_text_w(s, gap=2):
    return sum(len(GLYPHS[ch][0]) for ch in s) + gap * (len(s) - 1)


# ---------------------------------------------------------------------------
# Stamping


def flat(c: Canvas) -> Image.Image:
    """A canvas at its own size, both layers, before quantizing."""
    small = c.hi.resize((max(1, c.w), max(1, c.h)), Image.BOX)
    small.alpha_composite(c.lo)
    return small


def stamp(dst: Canvas, src: Canvas, x: int, y: int):
    """`src` onto `dst`'s pixel layer at (x, y): the same pixels as `src`
    gives as a frame of its own."""
    dst.lo.alpha_composite(flat(src), (int(x), int(y)))


# ---------------------------------------------------------------------------
# Little parts


def bolt_row(c: Canvas, pts, r=1.4):
    for x, y in pts:
        c.screw(x, y, r)


def stalk(c: Canvas, x0, y0, x1, y1):
    """A steel stalk from a knot at (x0, y0) to (x1, y1)."""
    a, b = (x0 + 0.5, y0 + 0.5), (x1 + 0.5, y1 + 0.5)
    c.line([a, b], INK, 3.0, round_ends=False)
    c.line([a, b], (150, 160, 172), 1.4, round_ends=False)


def key(c: Canvas, x, y, w, h, base, pressed=False, label=None, label_col=INK, kind="sans", size=None, glyph=None):
    """A push button filling x..x+w × y..y+h: up, it stands 1 pixel out
    (its shadow on the right and bottom); pressed, it's pushed into that
    pixel and its edge shows the hole."""
    if pressed:
        c.rect(x, y, x + w, y + h, fill=darker(base, 0.7))
        bx, by = x + 1, y + 1
        top, bot = darker(base, 0.15), lighter(base, 0.1)
        face = darker(base, 0.12)
    else:
        c.rect(x + 1, y + 1, x + w, y + h, fill=darker(base, 0.7))
        bx, by = x, y
        top, bot = lighter(base, 0.45), darker(base, 0.35)
        face = base
    kw, kh = w - 1, h - 1
    c.rect(bx, by, bx + kw, by + kh, fill=INK, radius=1.5)
    c.rect(bx + 1, by + 1, bx + kw - 1, by + kh - 1, fill=bot, radius=1)
    c.rect(bx + 1, by + 1, bx + kw - 2, by + kh - 2, fill=top, radius=1)
    c.gradient(bx + 2, by + 2, bx + kw - 2, by + kh - 2, [lighter(face, 0.15), face, darker(face, 0.08)])
    if label:
        c.text_fit(bx + 2.5, by + 2.5, bx + kw - 2.5, by + kh - 2.5, label, label_col, kind, size or 14)
    if glyph:
        glyph(c, bx, by, kw, kh)
