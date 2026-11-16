"""Switch, bulb, heart, colour mixer, the number crunchers (+ − × ÷, the
one-number functions, area/volume, conversions), the comparer, tape
player, mouse button, animal and assorted sounds, the message list,
billboard, strip chart and the stickers (sheets 1012 to 1061, all in
WWGADGET)."""
from __future__ import annotations

import math

from wgfx import (tr, BRASS, BRASS_D, BRASS_L, GLASS, INK, LED_GREEN, LED_RED, LED_RED_OFF,
                  PANEL_BLUE, PANEL_TEAL, STEEL, STEEL_D, STEEL_L, WHITE, darker, lighter, sheet)

from ._numbers_kit import pixel_text, pixel_text_w
from ._parts_kit import (CREAM, leds, vents, arrow_down, bezel, channel, crop, gear, grass, heart_pts, layer, nail,
                         pellet, plank, scale_pts, stamp, star_pts, tile)


def pixel_word(c, cx, top, word, col):
    """`word` in the bold pixel font, centred on `cx`, its letters' tops at
    `top` (an Å's ring a row above): for the small ON/OFF words in a
    language whose word has a ring or dots too small for a smooth font
    (PÅ)."""
    pixel_text(c, round(cx - pixel_text_w(word) / 2), top, word, col)

ORANGE = (240, 150, 36)
MAGENTA = (196, 74, 150)
BEAD = (255, 214, 40)


def screws(c, x0, y0, x1, y1, inset=3.5, r=1.4):
    for sx in (x0 + inset, x1 - inset):
        for sy in (y0 + inset, y1 - inset):
            c.screw(sx, sy, r)


# ---------------------------------------------------------------------------
# The number crunchers: a plate with a groove from the inputs to a window
# (where the code puts the operation's icon), beads running along it and
# gears turning while it crunches (8 steps, 0 at rest).


def crunch_body(c, base, box, groove_y, win, phase, top_x=None, out_x=None, gears_x=None):
    x0, y0, x1, y1 = box
    wx0, wy0, wx1, wy1 = win
    c.panel(x0, y0, x1, y1, base, radius=3, bevel=1.5, gloss=False)
    screws(c, x0, y0, x1, y1)
    gy0, gy1 = groove_y - 4, groove_y + 4
    # gears above and below the groove, meshing with the beads
    gx = gears_x if gears_x is not None else (x0 + wx0) / 2 - 2
    turn = phase * 22.5
    gear(c, gx, gy0 - 7.5, 7, 8, turn + 10)
    gear(c, gx, gy1 + 7.5, 7, 8, -turn + 10)
    channel(c, x0 - 1, gy0, wx0 - 2, gy1)
    if top_x is not None:
        channel(c, top_x - 4, y0 - 1, top_x + 4, gy1)
        c.rect(top_x - 3, gy0 + 1, top_x + 3, gy1 - 1, fill=(16, 20, 28))
    if out_x is not None:
        channel(c, wx1 + 2, gy0, x1 + 1, gy1)
    bezel(c, wx0, wy0, wx1, wy1, GLASS, pad=3)
    mx = (wx0 + wx1) / 2
    lit = set() if phase == 0 else {(phase - 1) % 4}
    if wy0 - 3 - y0 >= 8:
        leds(c, mx - 7.5, (y0 + wy0 - 3) / 2, 4, lit)
    if y1 - (wy1 + 3) >= 8:
        vents(c, wx0, wy1 + 5, wx1, min(3, int((y1 - wy1 - 8) / 2.5)), base)
    # beads: at rest, two wait in the groove; crunching, they run in
    left = x0 + 2
    span = wx0 - 2 - left
    if phase == 0:
        pellet(c, left + span * 0.3, groove_y)
        if top_x is not None:
            pellet(c, top_x, gy0 - 8)
    else:
        t = phase / 7
        for k in range(3):
            f = (t + k / 3) % 1.0
            pellet(c, left + 3 + (span - 6) * f, groove_y)
        if top_x is not None:
            for k in range(2):
                f = (t + k / 2) % 1.0
                pellet(c, top_x, y0 + 2 + (gy0 - y0 - 2) * f)
    # the result leaves at the end
    if out_x is not None:
        lit = phase >= 5
        c.circle(out_x, groove_y, 1.8, fill=INK)
        c.circle(out_x, groove_y, 1.2, fill=(255, 170, 40) if lit else (90, 50, 20))


def icon_tile(c, base=CREAM):
    tile(c, 0, 0, c.w, c.h, base)


def op_icon(c, op, thick=3.0):
    """+ − × ÷ on a key cap."""
    icon_tile(c)
    w, h = c.w, c.h
    cx, cy = w / 2 - 0.5, h / 2 - 0.5
    r = min(w, h) * 0.3
    col = INK
    if op == 0:
        c.line([(cx - r, cy), (cx + r, cy)], col, thick)
        c.line([(cx, cy - r), (cx, cy + r)], col, thick)
    elif op == 1:
        c.line([(cx - r, cy), (cx + r, cy)], col, thick)
    elif op == 2:
        k = r * 0.8
        c.line([(cx - k, cy - k), (cx + k, cy + k)], col, thick)
        c.line([(cx - k, cy + k), (cx + k, cy - k)], col, thick)
    else:
        c.line([(cx - r, cy), (cx + r, cy)], col, thick * 0.8)
        d = r * 0.68
        c.circle(cx, cy - d, thick * 0.55, fill=col)
        c.circle(cx, cy + d, thick * 0.55, fill=col)


@sheet(1020)
def number_cruncher(ctx):
    k = ctx.knots()
    w, h = ctx.size(5)
    win = (53, 24, 79, 48)
    steps = [10, 11, 12, 13, 14, 5, 6, 7]  # CRUNCH_FRAMES by step
    frames = {}
    for phase, n in enumerate(steps):
        c = ctx.canvas(n)
        crunch_body(c, ORANGE, (9, 9, 91, 64), 36, win, phase, top_x=49, out_x=86, gears_x=24)
        c.knots(k)
        frames[n] = c
    out = []
    for n in range(1, ctx.count + 1):
        if n in frames:
            out.append(frames[n])
        elif n <= 4 or 15 <= n <= 18:
            c = ctx.canvas(n)
            op_icon(c, (n - 1) % 4 if n <= 4 else n - 15, 3.0 if n <= 4 else 2.2)
            out.append(c)
        else:
            out.append(ctx.blank(n))
    return out


def one_cruncher(ctx, base, icons, knots_table=None):
    k = ctx.knots(knots_table)
    win = (55, 15, 81, 40)
    out = []
    for phase in range(8):
        c = ctx.canvas(phase + 1)
        crunch_body(c, base, (11, 0, 92, 55), 27, win, phase, out_x=87, gears_x=27)
        c.knots(k)
        out.append(c)
    for i, fn in enumerate(icons):
        c = ctx.canvas(9 + i)
        if fn is None:
            out.append(c)
            continue
        icon_tile(c)
        fn(c)
        out.append(c)
    while len(out) < ctx.count:
        out.append(ctx.blank(len(out) + 1))
    return out


def label(text, kind="sans", size=13.0):
    def draw(c):
        c.text_fit(2, 2, c.w - 3, c.h - 3, text, INK, kind, size)
    return draw


def bang(c):
    cx = c.w / 2 - 0.5
    c.line([(cx, 5), (cx, 14.5)], INK, 3.2)
    c.circle(cx, 19, 1.8, fill=INK)


def root(c):
    c.line([(4, 13), (7, 11.5), (10.5, 19), (14.5, 5), (21.5, 5)], INK, 2.2, round_ends=False)


def square(c):
    c.text(8.5, 13.5, "x", 15, INK, "sans")
    c.text(18.5, 7.5, "2", 8.5, INK, "sans")


@sheet(1024)
def one_number_cruncher(ctx):
    # frames 9.. : !, √, x², Cos, Sin, Tan, Int
    return one_cruncher(ctx, PANEL_BLUE, [bang, root, square, label("Cos"), label("Sin"),
                                          label("Tan"), label("Int")])


def shape_square(c):
    c.rect(5, 5, 20, 20, fill=INK)
    c.gradient(6, 6, 19, 19, [(150, 190, 240), (70, 120, 200)])


def shape_circle(c):
    c.circle(12.5, 12.5, 8, fill=INK)
    c.circle(12.5, 12.5, 7, fill=(70, 160, 110))
    c.circle(12.5, 12.5, 6, fill=(110, 200, 140))


def shape_cube(c):
    s, cx, cy = 7.0, 12.5, 12.5
    top = [(cx, cy - s), (cx + s, cy - s / 2), (cx, cy), (cx - s, cy - s / 2)]
    left = [(cx - s, cy - s / 2), (cx, cy), (cx, cy + s), (cx - s, cy + s / 2)]
    right = [(cx, cy), (cx + s, cy - s / 2), (cx + s, cy + s / 2), (cx, cy + s)]
    c.polygon([(cx, cy - s - 1), (cx + s + 1, cy - s / 2 - 0.5), (cx + s + 1, cy + s / 2 + 0.5),
               (cx, cy + s + 1), (cx - s - 1, cy + s / 2 + 0.5), (cx - s - 1, cy - s / 2 - 0.5)], fill=INK)
    c.polygon(top, fill=(250, 200, 120))
    c.polygon(left, fill=(226, 140, 50))
    c.polygon(right, fill=(170, 90, 30))
    c.line([top[3], top[2], top[1]], INK, 0.5)
    c.line([top[2], left[2]], INK, 0.5)


def shape_sphere(c):
    c.circle(12.5, 12.5, 8.2, fill=INK)
    c.radial(12.5, 12.5, 7.4, (255, 220, 240), (150, 40, 110), -0.45, -0.45)


@sheet(1025)
def area_volume(ctx):
    return one_cruncher(ctx, PANEL_TEAL, [shape_square, shape_circle, shape_cube, shape_sphere, None], 1024)


@sheet(1026)
def conversion(ctx):
    k = ctx.knots()
    left = (27, 22, 47, 33)
    right = (58, 23, 78, 34)
    out = []
    for phase in range(8):
        c = ctx.canvas(phase + 1)
        x0, y0, x1, y1 = 11, 0, 92, 55
        c.panel(x0, y0, x1, y1, MAGENTA, radius=3, bevel=1.5)
        screws(c, x0, y0, x1, y1)
        gy = 27
        turn = phase * 22.5
        # rollers squeezing the number through the converter
        gear(c, 37, 12, 6.5, 8, turn)
        gear(c, 37, 42.5, 6.5, 8, -turn)
        channel(c, x0 - 1, gy - 4, 53, gy + 4)
        channel(c, 82, gy - 4, x1 + 1, gy + 4)
        # the "from" unit rides on a little sled in the groove
        c.rect(left[0] - 2, left[1] - 2, left[2] + 2, left[3] + 2, fill=INK, radius=1.5)
        c.rect(left[0] - 1, left[1] - 1, left[2] + 1, left[3] + 1, fill=STEEL_L, radius=1)
        bezel(c, right[0], right[1], right[2], right[3], GLASS, pad=4)
        leds(c, 60.5, 10, 4, set() if phase == 0 else {(phase - 1) % 4})
        vents(c, right[0], 44, right[2], 3, MAGENTA)
        if phase:
            t = phase / 7
            for kk in range(2):
                f = (t + kk / 2) % 1.0
                pellet(c, 13 + 9 * f, gy, 1.8)
            # the arrow from one unit to the other glows
        c.polygon([(49, gy - 3), (53.5, gy), (49, gy + 3)], fill=(255, 200, 60) if phase else (110, 60, 40), outline=INK, width=0.6)
        c.circle(87, gy, 1.8, fill=INK)
        c.circle(87, gy, 1.2, fill=(255, 170, 40) if phase >= 5 else (90, 50, 20))
        c.knots(k)
        out.append(c)
    out.append(ctx.blank(9))
    for i, u in enumerate(["°C", "M", "Kg", "°F", "Ft", "Lb"]):
        c = ctx.canvas(10 + i)
        tile(c, 0, 0, c.w, c.h, (255, 214, 190), radius=1.5)
        c.text_fit(1.5, 1.5, c.w - 2, c.h - 2, tr(u), INK, "sans", 9)
        out.append(c)
    return out


# ---------------------------------------------------------------------------
# The Comparer: a screen where a little robot thinks of the comparison.


def compare_sign(c, mode, cx, cy, s, col=INK, w=2.2):
    """mode 0 >, 1 <, 2 ≠, 3 =, 4 ≥, 5 ≤ (the sheet's frame order)."""
    if mode in (0, 1, 4, 5):
        d = -1 if mode in (0, 4) else 1
        lift = -1.6 if mode in (4, 5) else 0
        c.line([(cx + d * s, cy - s * 0.85 + lift), (cx - d * s, cy + lift), (cx + d * s, cy + s * 0.85 + lift)], col, w)
        if mode in (4, 5):
            c.line([(cx - s, cy + s * 0.85 + 2.6), (cx + s, cy + s * 0.85 + 2.6)], col, w)
    else:
        c.line([(cx - s, cy - s * 0.4), (cx + s, cy - s * 0.4)], col, w)
        c.line([(cx - s, cy + s * 0.4), (cx + s, cy + s * 0.4)], col, w)
        if mode == 2:
            c.line([(cx + s * 0.5, cy - s), (cx - s * 0.5, cy + s)], col, w)


def robot_head(c, x, y, s=1.0):
    """A little thinking robot's head, top left at (x, y), 12 wide."""
    c.line([(x + 9 * s, y + 3), (x + 9 * s, y)], INK, 1.0)
    c.circle(x + 9 * s, y, 1.4, fill=LED_RED)
    c.rect(x, y + 2.5, x + 12 * s, y + 12 * s, fill=INK, radius=2)
    c.gradient(x + 1, y + 3.5, x + 12 * s - 1, y + 12 * s - 1, [STEEL_L, STEEL], radius=1.5)
    c.rect(x + 2, y + 5, x + 10 * s, y + 8.5, fill=GLASS, radius=1)
    c.circle(x + 4.3, y + 6.8, 1.1, fill=(120, 230, 255))
    c.circle(x + 7.8 * s, y + 6.8, 1.1, fill=(120, 230, 255))
    c.line([(x + 4, y + 10.2), (x + 8 * s, y + 10.2)], STEEL_D, 0.8)


@sheet(1028)
def comparer(ctx):
    k = ctx.knots()
    out = []
    for mode in range(6):
        c = ctx.canvas(mode + 1)
        x0, y0, x1, y1 = 13, 11, 66, 62
        c.panel(x0, y0, x1, y1, (120, 90, 200), radius=3, bevel=1.5, gloss=False)
        c.inset(x0 + 4, y0 + 4, x1 - 4, y1 - 4, (30, 40, 110), radius=1.5)
        c.gradient(x0 + 5, y0 + 5, x1 - 5, y1 - 5, [(20, 30, 90), (50, 90, 190)])
        # thought cloud
        cl = (250, 250, 255)
        for cx, cy, r in ((26, 27, 7), (33, 23, 8), (42, 25, 7.5), (47, 31, 5.5), (37, 33, 7), (27, 33, 5.5)):
            c.circle(cx, cy, r + 0.9, fill=INK)
        for cx, cy, r in ((26, 27, 7), (33, 23, 8), (42, 25, 7.5), (47, 31, 5.5), (37, 33, 7), (27, 33, 5.5)):
            c.circle(cx, cy, r, fill=cl)
        for cx, cy, r in ((43.5, 40.5, 1.7), (46, 44, 1.1)):
            c.circle(cx, cy, r + 0.8, fill=INK)
            c.circle(cx, cy, r, fill=cl)
        compare_sign(c, mode, 36, 28.5 if mode < 4 else 27.5, 5.5 if mode < 4 else 4.8, INK, 2.3)
        robot_head(c, 46, 45)
        c.text(27, 49, "?", 11, (255, 220, 60), "sans", stroke=0.8, stroke_fill=INK)
        c.knots(k)
        out.append(c)
    out.append(ctx.blank(7))
    return out


# ---------------------------------------------------------------------------
# Switch, light bulb, heart


@sheet(1012)
def switch(ctx):
    k = ctx.knots()
    out = []
    for on in (True, False):
        c = ctx.canvas(1 if on else 2)
        c.knots(k)
        c.panel(0, 0, 28, 42, PANEL_BLUE, radius=3, bevel=1.5, gloss=False)
        lit, dim = (255, 240, 120), lighter(PANEL_BLUE, 0.35)
        if tr("ON") == "ON":
            c.text(14, 5.8, tr("ON"), 8.5, lit if on else dim, "sans", stroke=0.7 if on else 0, stroke_fill=INK)
            c.text(14, 36.2, tr("OFF"), 8.5, dim if on else lit, "sans", stroke=0 if on else 0.7, stroke_fill=INK)
        else:
            pixel_word(c, 14, 3, tr("ON"), lit if on else dim)
            pixel_word(c, 14, 33, tr("OFF"), dim if on else lit)
        # the toggle's plate and slot
        c.rect(7, 12, 21, 30, fill=INK, radius=3)
        c.gradient(8, 13, 20, 29, [STEEL_L, STEEL, STEEL_D], radius=2)
        c.rect(11, 15, 17, 27, fill=INK, radius=2)
        c.rect(12, 16, 16, 26, fill=(40, 44, 52), radius=1.5)
        # the lever, from the pivot to its knob
        tip = 12.5 if on else 29.5
        c.polygon([(12.2, 21), (15.8, 21), (16.2 if on else 16.6, tip), (11.8 if on else 11.4, tip)], fill=INK)
        c.polygon([(12.9, 21), (15.1, 21), (15.4, tip), (12.6, tip)], fill=STEEL_L)
        c.circle(14, tip, 3.3, fill=INK)
        c.radial(14, tip, 2.6, WHITE, STEEL_D, -0.5, -0.5)
        c.circle(14, 21, 2.2, fill=INK)
        c.circle(14, 21, 1.4, fill=STEEL)
        out.append(c)
    out.append(ctx.blank(3))
    return out


@sheet(1016)
def light_bulb(ctx):
    k = ctx.knots()
    out = []
    for on in (False, True):
        c = ctx.canvas(2 if on else 1)
        c.knots(k, ["b"])
        cx = 20.5
        glass_pts = [(14, 22), (27, 22), (25, 31), (16, 31)]
        if on:
            for i in range(12):
                a = math.radians(i * 30 + 15)
                if math.sin(a) > 0.55:
                    continue
                c.line([(cx + math.cos(a) * 15, 14 + math.sin(a) * 15), (cx + math.cos(a) * 19, 14 + math.sin(a) * 19)],
                       (255, 230, 80), 1.3)
        c.circle(cx, 14, 13.2, fill=INK)
        c.polygon([(13.4, 21), (27.6, 21), (25.8, 32), (15.2, 32)], fill=INK)
        if on:
            c.polygon(glass_pts, fill=(255, 230, 100))
            c.radial(cx, 14, 12.2, (255, 255, 235), (255, 196, 40), 0, 0)
        else:
            c.polygon(glass_pts, fill=(200, 210, 222))
            c.radial(cx, 14, 12.2, (246, 248, 252), (176, 188, 204), -0.4, -0.4)
        # filament on its posts
        post = (120, 110, 100) if not on else (255, 200, 120)
        c.line([(18, 30), (18, 17)], post, 0.8)
        c.line([(23, 30), (23, 17)], post, 0.8)
        fil = [(18, 17), (19, 14), (20, 17), (21, 14), (22, 17), (23, 14)]
        c.line(fil, WHITE if on else (90, 80, 70), 1.0 if on else 0.8)
        if not on:
            c.arc(cx - 3, 10, 5, 190, 260, WHITE, 1.4)
        # the screw base
        c.rect(15, 30.5, 26, 37, fill=INK, radius=1)
        for i, yy in enumerate((31.5, 33.5, 35.5)):
            c.gradient(16, yy, 25, yy + 1.5, [BRASS_L, BRASS, BRASS_D], vertical=False)
        # the plate with the label
        c.panel(1, 43, 41, 49, STEEL, radius=2, bevel=1, gloss=False)
        c.rect(7, 36, 35, 45.5, fill=INK, radius=2)
        c.rect(8, 37, 34, 44.5, fill=(30, 34, 46), radius=1.5)
        word, col = (tr("ON"), (255, 90, 60)) if on else (tr("OFF"), (170, 180, 200))
        if tr("ON") == "ON":
            c.text(21, 40.8, word, 8, col, "sans")
        else:
            pixel_word(c, 21, 38, word, col)
        out.append(c)
    out.append(ctx.blank(3))
    return out


def heart_canvas(ctx, beat):
    from wgfx import Canvas
    c = Canvas(*ctx.size(1))
    kx, ky, ko = ctx.knots()[0]
    k = 0.9 if beat else 1.0
    pts = scale_pts(heart_pts(17, 27, 1.06, 1.41), 17, 27, k)
    red = (255, 34, 24) if beat else (200, 24, 30)
    # the vessel to the output lamp
    c.knot(kx, ky, ko, "l", reach=5)
    c.line([(21, 15), (24, 9.5), (31, 9.5 + 0.5)], INK, 4.0)
    c.line([(21, 15), (24, 9.5), (31, 9.5 + 0.5)], (70, 140, 230), 2.2)
    c.polygon(pts, fill=INK)
    c.polygon(scale_pts(pts, 17, 28, 0.94), fill=darker(red, 0.3))
    c.polygon(scale_pts(pts, 15.5, 25, 0.86), fill=red)
    c.ellipse(7 if not beat else 8, 14, 14, 20, fill=lighter(red, 0.55))
    c.circle(9.5 if not beat else 10.5, 15.5, 1.3, fill=WHITE)
    return c


@sheet(1017)
def heart(ctx):
    rest = heart_canvas(ctx, False)
    beat = heart_canvas(ctx, True)
    t, l, b, r = ctx.rects[1]
    return [rest, crop(beat, l, t, r - l, b - t), ctx.blank(3)]


# ---------------------------------------------------------------------------
# Colour mixer, message list, billboard, strip chart: windows the code
# draws in.


@sheet(1018)
def rgb_display(ctx):
    c = ctx.canvas(1)
    k = ctx.knots()
    fill = (17, 6, 113, 77)  # where the code paints the colour
    c.panel(8, 0, 117, 82, STEEL, radius=3, bevel=1.5, gloss=False)
    c.rect(fill[0] - 2, fill[1] - 2, fill[2] + 2, fill[3] + 2, fill=STEEL_D, radius=1)
    c.rect(fill[0] - 1, fill[1] - 1, fill[2] + 1, fill[3] + 1, fill=INK)
    c.rect(*fill, fill=(14, 15, 20))
    c.polygon([(fill[0], fill[1] + 26), (fill[0] + 26, fill[1]), (fill[0] + 36, fill[1]), (fill[0], fill[1] + 36)], fill=(24, 26, 34))
    for (x, y, out), col, ch in zip(k, (LED_RED, LED_GREEN, (60, 120, 255)), "RGB"):
        c.knot(x, y, out, "l", reach=6)
        c.rect(9, y - 7, 15.5, y + 8, fill=INK, radius=1.5)
        c.gradient(10, y - 6, 14.5, y + 7, [lighter(col, 0.5), col, darker(col, 0.3)], radius=1)
    c.circle(108, 79.5, 1.0, fill=LED_GREEN)
    return [c, ctx.blank(2)]


@sheet(1046)
def message_list(ctx):
    c = ctx.canvas(1)
    k = ctx.knots()
    c.knots(k)
    c.panel(10, 0, 280, 24, PANEL_TEAL, radius=3, bevel=1.5, gloss=False)
    c.rect(14, 3, 277, 21, fill=INK, radius=1.5)
    c.gradient(15, 4, 276, 20, [(14, 30, 22), (4, 12, 8)])
    c.line([(16, 5), (275, 5)], (30, 60, 44), 0.6)
    return [c, ctx.blank(2)]


@sheet(1047)
def billboard(ctx):
    c = ctx.canvas(1)
    k = ctx.knots()
    c.knots(k)
    wood = (176, 116, 60)
    c.rect(10, 0, 183, 85, fill=INK, radius=2)
    c.gradient(11, 1, 182, 84, [lighter(wood, 0.25), wood, darker(wood, 0.2)], vertical=False, radius=1.5)
    c.rect(13, 3, 180, 82, fill=darker(wood, 0.5))
    c.rect(14, 4, 180, 82, fill=WHITE)
    for x, y in ((12, 2), (181, 2), (12, 83), (181, 83)):
        nail(c, x, y)
    return [c, ctx.blank(2)]


@sheet(1049)
def strip_chart(ctx):
    c = ctx.canvas(1)
    k = ctx.knots()
    ox, oy = ctx.geo.points(1049)[0]
    c.knots(k, ["l", "t"])
    c.panel(10, 10, 148, 103, PANEL_BLUE, radius=3, bevel=1.5, gloss=False)
    # the chart paper: 120 columns from ox, 80 rows up from oy
    px0, py0, px1, py1 = ox - 2, oy - 82, ox + 122, oy + 3
    c.rect(px0 - 1, py0 - 1, px1 + 1, py1 + 1, fill=INK)
    c.prect(px0, py0, px1, py1, (250, 250, 244))
    grid = (200, 214, 230)
    for i in range(0, 121, 10):
        c.prect(ox + i, oy - 79, ox + i + 1, oy + 1, grid)
    for j in range(0, 80, 10):
        c.prect(ox, oy - j, ox + 120, oy - j + 1, grid)
    c.prect(ox, oy - 79, ox + 120, oy - 78, grid)
    c.prect(ox - 1, oy - 79, ox, oy + 1, (90, 110, 140))
    c.prect(ox - 1, oy + 1, ox + 121, oy + 2, (90, 110, 140))
    # the reset label down the side, under its socket
    for i, ch in enumerate(tr("RESET")):
        c.text(16.5, 22 + i * 9, ch, 7.5, WHITE, "sans", stroke=0.5, stroke_fill=INK)
    pen = ctx.canvas(3)
    pen.prect(0, 0, 4, 3, INK)
    pen.prect(1, 0, 3, 2, LED_RED)
    pen.prect(1, 3, 3, 4, INK)
    pen.prect(1, 4, 2, 5, INK)
    return [c, ctx.blank(2), pen]


# ---------------------------------------------------------------------------
# Tape player, mouse button


def tape_deck(ctx, playing, turn):
    c = ctx.canvas(1)
    c.knots(ctx.knots())
    deck = (78, 84, 98)
    c.panel(11, 0, 58, 40, deck, radius=3, bevel=1.5, gloss=False)
    # the play lamp
    c.rect(15, 2, 54, 8, fill=INK, radius=1.5)
    if playing:
        c.gradient(16, 3, 53, 7, [(255, 220, 140), (255, 70, 30), (190, 20, 10)], radius=1)
    else:
        c.rect(16, 3, 53, 7, fill=LED_RED_OFF, radius=1)
    # the cassette
    c.rect(15, 9, 54, 34, fill=INK, radius=2)
    c.gradient(16, 10, 53, 33, [(70, 120, 200), (40, 80, 160)], radius=1.5)
    c.rect(18, 11, 51, 15, fill=CREAM, radius=1)
    c.line([(20, 13), (40, 13)], (200, 80, 60), 0.6)
    c.rect(21, 16.5, 48, 27.5, fill=INK, radius=4)
    c.rect(22, 17.5, 47, 26.5, fill=(26, 28, 36), radius=3.5)
    c.rect(32, 19.5, 37, 24.5, fill=(110, 70, 40))
    for rx in (27.5, 41.5):
        c.circle(rx, 22, 4.2, fill=(230, 232, 236))
        c.circle(rx, 22, 2.3, fill=INK)
        for i in range(6):
            a = math.radians(turn + i * 60)
            c.line([(rx + math.cos(a) * 1.6, 22 + math.sin(a) * 1.6), (rx + math.cos(a) * 3.2, 22 + math.sin(a) * 3.2)],
                   STEEL_D, 0.9, round_ends=False)
    c.polygon([(21, 33.5), (24, 29), (45, 29), (48, 33.5)], fill=darker((40, 80, 160), 0.3), outline=INK, width=0.6)
    # keys
    for i in range(4):
        x = 17 + i * 9
        c.rect(x, 35, x + 7, 39, fill=INK, radius=1)
        c.rect(x + 1, 35.5, x + 6, 38, fill=(220, 60, 50) if i == 0 else STEEL_L, radius=0.8)
    return c


@sheet(1037)
def tape_player(ctx):
    still = tape_deck(ctx, False, 0)
    play = tape_deck(ctx, True, 0)
    turned = tape_deck(ctx, True, 30)
    x, y = ctx.geo.points(1037)[0]
    w, h = ctx.size(4)
    return [still, play, ctx.blank(3), crop(turned, x, y, w, h)]


@sheet(1040)
def mouse_button(ctx):
    k = ctx.knots()
    out = []
    for down in (True, False):
        c = ctx.canvas(1 if down else 2)
        c.knots(k)
        kx, ky, _ = k[0]
        # the cord
        c.line([(22, 14), (23, 5), (34, 3.5), (44, ky + 0.5)], INK, 2.6)
        c.line([(22, 14), (23, 5), (34, 3.5), (44, ky + 0.5)], (60, 64, 76), 1.2)
        # the mouse
        body = (226, 228, 234)
        c.rect(7, 11, 38, 46, fill=INK, radius=13)
        c.gradient(8, 12, 37, 45, [WHITE, body, (170, 174, 186)], radius=12)
        c.line([(22.5, 12), (22.5, 26)], INK, 0.8)
        c.line([(8, 26), (37, 26)], INK, 0.8)
        btn = (150, 200, 255) if down else body
        c.polygon([(9, 25.5), (9, 22), (11, 15), (16, 12.5), (22, 12), (22, 25.5)], fill=btn)
        if down:
            c.polygon([(9, 25.5), (9, 22), (11, 15), (16, 12.5), (22, 12), (22, 13.2), (16.5, 13.8), (12, 16), (10.2, 22), (10.2, 25.5)],
                      fill=darker(btn, 0.3))
            for a in (200, 240, 280):
                r = math.radians(a)
                c.line([(15 + math.cos(r) * 7, 18 + math.sin(r) * 7), (15 + math.cos(r) * 10.5, 18 + math.sin(r) * 10.5)],
                       (255, 230, 60), 1.4)
        c.rect(21, 15, 24, 21, fill=INK, radius=1.5)
        c.rect(21.8, 15.8, 23.2, 20.2, fill=STEEL, radius=0.7)
        # the pointer, clicking the left button
        tx, ty = 13, 17 + (1 if down else 0)
        arrow = [(0, 0), (0, 16), (4, 12.5), (7, 19), (10, 17.6), (7, 11.4), (12, 11.4)]
        pts = [(tx + x, ty + y) for x, y in arrow]
        c.polygon(pts, fill=INK, outline=INK, width=2.2)
        c.polygon(pts, fill=WHITE)
        out.append(c)
    out.append(ctx.blank(3))
    return out


# ---------------------------------------------------------------------------
# Animal sounds and assorted sounds: framed pictures.


def picture_frame(c, x0, y0, x1, y1):
    c.rect(x0, y0, x1, y1, fill=INK, radius=1.5)
    c.gradient(x0 + 1, y0 + 1, x1 - 1, y1 - 1, [BRASS_L, BRASS, BRASS_D], radius=1)
    c.rect(x0 + 2.5, y0 + 2.5, x1 - 2.5, y1 - 2.5, fill=INK)


def clip(l, x0, y0, x1, y1):
    l.clear(0, 0, l.w, y0)
    l.clear(0, y1, l.w, l.h)
    l.clear(0, 0, x0, l.h)
    l.clear(x1, 0, l.w, l.h)


def draw_cat(p, x0, y0, x1, y1):
    p.rect(x0, y0, x1, y1, fill=(90, 160, 90))
    cx = (x0 + x1) / 2
    fur, fur_d = (240, 150, 60), (190, 96, 30)
    p.ellipse(cx - 14, 44, cx + 14, 64, fill=INK)
    p.ellipse(cx - 13, 45, cx + 13, 64, fill=WHITE)
    for s in (-1, 1):
        ear = [(cx + s * 14, 26), (cx + s * 15.5, 6), (cx + s * 4, 18)]
        p.polygon(ear, fill=fur, outline=INK, width=0.9)
        p.polygon([(cx + s * 12.5, 22), (cx + s * 14, 10), (cx + s * 7, 18)], fill=(250, 170, 170))
    p.ellipse(cx - 17, 14, cx + 17, 47, fill=INK)
    p.ellipse(cx - 16, 15, cx + 16, 46, fill=fur)
    for dx in (-4, 0, 4):
        p.line([(cx + dx, 16.5), (cx + dx * 0.7, 22)], fur_d, 1.2)
    p.ellipse(cx - 9, 33, cx + 9, 45, fill=(255, 236, 214))
    for s in (-1, 1):
        ex = cx + s * 6.5
        p.ellipse(ex - 3.8, 25, ex + 3.8, 32, fill=INK)
        p.ellipse(ex - 3, 25.8, ex + 3, 31.2, fill=(150, 220, 80))
        p.ellipse(ex - 0.9, 25.8, ex + 0.9, 31.2, fill=INK)
        p.circle(ex + 1.3 * s - 0.3, 27, 0.8, fill=WHITE)
        for dy in (-1.5, 1.2):
            p.line([(cx + s * 7, 38 + dy), (cx + s * 18, 37 + dy * 2.2)], WHITE, 0.6)
    p.polygon([(cx - 2.2, 34), (cx + 2.2, 34), (cx, 36.4)], fill=(240, 110, 130), outline=INK, width=0.5)
    p.line([(cx, 36.4), (cx, 38.2), (cx - 2.4, 39.6)], INK, 0.7)
    p.line([(cx, 38.2), (cx + 2.4, 39.6)], INK, 0.7)


def draw_dog(p, x0, y0, x1, y1):
    p.rect(x0, y0, x1, y1, fill=(120, 176, 236))
    cx = (x0 + x1) / 2
    tan, brown = (200, 120, 50), (110, 60, 26)
    p.ellipse(cx - 18, 46, cx + 18, 70, fill=INK)
    p.ellipse(cx - 17, 47, cx + 17, 70, fill=WHITE)
    p.ellipse(cx - 14, 9, cx + 14, 46, fill=INK)
    p.ellipse(cx - 13, 10, cx + 13, 45, fill=tan)
    p.polygon([(cx - 2.5, 11), (cx + 2.5, 11), (cx + 6, 36), (cx - 6, 36)], fill=WHITE)
    p.ellipse(cx - 10, 31, cx + 10, 49, fill=INK)
    p.ellipse(cx - 9, 32, cx + 9, 48, fill=WHITE)
    for s in (-1, 1):
        ear = [(cx + s * 9, 13), (cx + s * 17, 15), (cx + s * 20, 36), (cx + s * 16, 46), (cx + s * 11, 40), (cx + s * 10, 22)]
        p.polygon(ear, fill=brown, outline=INK, width=0.9)
        ex = cx + s * 6
        p.circle(ex, 25, 2.8, fill=INK)
        p.circle(ex, 25, 2.2, fill=(70, 36, 16))
        p.circle(ex - 0.7, 24.2, 0.8, fill=WHITE)
        p.line([(ex - 2.5, 21), (ex + 2.5, 21.4 - s * 0.6)], darker(tan, 0.45), 0.8)
    p.ellipse(cx - 4.5, 34, cx + 4.5, 40, fill=INK)
    p.ellipse(cx - 2.5, 34.8, cx - 0.5, 36.3, fill=(110, 110, 120))
    p.line([(cx, 40), (cx, 42.5)], INK, 0.7)
    p.arc(cx - 3, 42.5, 3, 20, 160, INK, 0.7)
    p.arc(cx + 3, 42.5, 3, 20, 160, INK, 0.7)
    p.ellipse(cx - 2.6, 44, cx + 2.6, 49.5, fill=(240, 110, 130), outline=INK, width=0.6)


def draw_cow(p, x0, y0, x1, y1):
    p.rect(x0, y0, x1, y0 + 34, fill=(170, 214, 250))
    p.rect(x0, y0 + 32, x1, y1, fill=(100, 170, 60))
    blk = (30, 30, 34)
    # legs
    for lx in (27, 32, 46, 51):
        p.rect(lx - 1.8, 36, lx + 1.8, 50, fill=INK)
        p.rect(lx - 1, 36, lx + 1, 48.5, fill=WHITE)
        p.rect(lx - 1.2, 48, lx + 1.2, 50, fill=blk)
    p.line([(56, 26), (59.5, 36)], INK, 1.0)
    p.circle(59.5, 37, 1.3, fill=blk)
    # body with patches
    p.rect(23, 21, 57, 40, fill=INK, radius=7)
    body = layer(p)
    body.rect(24, 22, 56, 39, fill=WHITE, radius=6)
    for ex, ey, rx, ry in ((32, 26, 6, 5), (47, 33, 7, 5), (52, 23, 4, 3)):
        body.ellipse(ex - rx, ey - ry, ex + rx, ey + ry, fill=blk)
    clip_mask_round(body, 24, 22, 56, 39, 6)
    stamp(p, body)
    p.ellipse(42, 36.5, 48, 41.5, fill=(250, 170, 180), outline=INK, width=0.6)
    # head
    p.polygon([(16, 16), (13.5, 12.5), (17.5, 15.3)], fill=CREAM, outline=INK, width=0.6)
    p.polygon([(24, 16), (26.5, 12.5), (22.5, 15.3)], fill=CREAM, outline=INK, width=0.6)
    p.ellipse(11, 17, 15.5, 21, fill=blk, outline=INK)
    p.ellipse(24.5, 17, 29, 21, fill=blk, outline=INK)
    p.rect(14, 14.5, 26, 34, fill=INK, radius=5)
    p.rect(15, 15.5, 25, 33, fill=WHITE, radius=4.5)
    p.ellipse(15, 15.5, 21, 23, fill=blk)
    p.rect(15, 27, 25, 33, fill=(250, 180, 180), radius=3)
    p.circle(18, 30, 0.8, fill=INK)
    p.circle(22, 30, 0.8, fill=INK)
    p.circle(17.6, 22.5, 1.2, fill=INK)
    p.circle(22.6, 22.5, 1.2, fill=INK)
    for gx in range(x0 + 2, x1, 5):
        p.line([(gx, y1 - 1), (gx + 1, y1 - 4)], (50, 110, 30), 0.7)


def clip_mask_round(l, x0, y0, x1, y1, r):
    """Keep only what's inside the rounded box on `l`'s smooth layer."""
    from PIL import Image, ImageChops, ImageDraw
    from wgfx import SS
    m = Image.new("L", l.hi.size, 0)
    ImageDraw.Draw(m).rounded_rectangle([x0 * SS, y0 * SS, x1 * SS - 1, y1 * SS - 1], radius=r * SS, fill=255)
    a = ImageChops.multiply(l.hi.getchannel("A"), m)
    l.hi.putalpha(a)


def draw_rooster(p, x0, y0, x1, y1):
    p.rect(x0, y0, x1, y1, fill=(255, 220, 110))
    p.rect(x0, y1 - 5, x1, y1, fill=(190, 130, 60))
    p.line([(x0, y1 - 5), (x1, y1 - 5)], INK, 0.6)
    # tail feathers
    for i, (ang, col) in enumerate(((200, (30, 90, 70)), (225, (20, 40, 40)), (250, (40, 120, 90)), (275, (20, 50, 50)))):
        a = math.radians(ang)
        tip = (33 + math.cos(a) * 20, 33 + math.sin(a) * 22)
        mid = (33 + math.cos(a + 0.3) * 13, 33 + math.sin(a + 0.3) * 14)
        p.line([(32, 34), mid, tip], INK, 4.2)
        p.line([(32, 34), mid, tip], col, 2.6)
    # legs
    for lx in (34, 40):
        p.line([(lx, 40), (lx, 51)], INK, 2.2)
        p.line([(lx, 40), (lx, 51)], (250, 190, 40), 1.2)
        p.line([(lx - 3, 52), (lx, 51), (lx + 3.5, 52)], INK, 1.2)
    # body
    p.ellipse(24, 26, 49, 44, fill=INK)
    p.ellipse(25, 27, 48, 43, fill=(200, 90, 30))
    p.ellipse(28, 31, 42, 40, fill=(140, 60, 24), outline=INK, width=0.6)
    p.line([(31, 35), (40, 34)], darker((140, 60, 24), 0.4), 0.6)
    # neck and head
    p.polygon([(40, 30), (44, 15), (51, 15), (49, 31)], fill=INK)
    p.polygon([(41, 30), (44.6, 16), (50.2, 16), (48, 31)], fill=(230, 130, 40))
    for bx, by, r in ((44.5, 10.5, 2.2), (47.5, 9.2, 2.4), (50.3, 10.5, 2.0)):
        p.circle(bx, by, r + 0.7, fill=INK)
    for bx, by, r in ((44.5, 10.5, 2.2), (47.5, 9.2, 2.4), (50.3, 10.5, 2.0)):
        p.circle(bx, by, r, fill=(230, 30, 40))
    p.circle(47.5, 15, 5, fill=INK)
    p.circle(47.5, 15, 4.2, fill=(230, 130, 40))
    p.polygon([(51.5, 13.5), (56, 15.3), (51.5, 16.8)], fill=(250, 200, 40), outline=INK, width=0.6)
    p.ellipse(49.5, 17, 53, 22.5, fill=(230, 30, 40), outline=INK, width=0.6)
    p.circle(48.8, 13.8, 1.0, fill=INK)


def framed(ctx, n, art, knots_table=None):
    c = ctx.canvas(n)
    k = ctx.knots(knots_table)
    c.knots(k)
    x0, y0, x1, y1 = 11, 0, 63, c.h
    picture_frame(c, x0, y0, x1, y1)
    l = layer(c)
    art(l, x0 + 3, y0 + 3, x1 - 3, y1 - 3)
    clip(l, x0 + 3, y0 + 3, x1 - 3, y1 - 3)
    stamp(c, l)
    return c


@sheet(1042)
def animal_sounds(ctx):
    # 1 cat, 2 dog, 3 cow, 4 rooster
    return [framed(ctx, i + 1, a) for i, a in enumerate((draw_cat, draw_dog, draw_cow, draw_rooster))] + [ctx.blank(5)]


def draw_music(p, x0, y0, x1, y1):
    p.rect(x0, y0, x1, y1, fill=(250, 238, 206))
    # a stave with notes
    for i in range(5):
        y = 7 + i * 2.5
        p.line([(x0 + 1, y), (x1 - 1, y)], (110, 90, 70), 0.5)
    for nx, ny, up in ((22, 14.5, True), (30, 12, True), (38, 9.5, True)):
        p.ellipse(nx - 2.2, ny - 1.6, nx + 2.2, ny + 1.6, fill=INK)
        p.line([(nx + 1.9, ny), (nx + 1.9, ny - 8)], INK, 0.8)
    p.line([(22 + 1.9, 6.5), (30 + 1.9, 4), (38 + 1.9, 1.5)], INK, 1.6, round_ends=False)
    p.ellipse(46, 12, 50.4, 15.2, fill=INK)
    p.line([(50, 13.6), (50, 5.5)], INK, 0.8)
    p.line([(50, 5.5), (53, 8.5), (52.5, 10.5)], INK, 0.9)
    # keys (behind the drum)
    p.rect(34, 30, 59, 54, fill=INK)
    for i in range(6):
        kx = 35 + i * 4
        p.rect(kx, 31, kx + 3.3, 53, fill=WHITE)
    for i in (0, 1, 3, 4):
        kx = 37.6 + i * 4
        p.rect(kx, 31, kx + 2.4, 43, fill=INK)
    # a drum with sticks
    red = (220, 40, 50)
    p.line([(14, 24), (32, 38)], INK, 2.4)
    p.line([(14, 24), (32, 38)], (230, 190, 120), 1.2)
    p.rect(13, 34, 37, 51, fill=INK, radius=2)
    p.gradient(14, 35, 36, 50, [lighter(red, 0.3), red, darker(red, 0.35)], vertical=False)
    for i in range(5):
        x = 15 + i * 5
        p.line([(x, 36), (x + 3, 49)], (250, 220, 120), 0.8)
    p.rect(13, 47.5, 37, 51, fill=BRASS, outline=INK, width=0.6)
    p.ellipse(13, 30, 37, 38.5, fill=INK)
    p.ellipse(14, 31, 36, 37.5, fill=(245, 245, 240))
    p.line([(38, 24), (22, 34)], INK, 2.4)
    p.line([(38, 24), (22, 34)], (230, 190, 120), 1.2)
    p.circle(22, 34, 1.4, fill=(250, 230, 180), outline=INK, width=0.5)


@sheet(1043)
def assorted_sounds(ctx):
    return [framed(ctx, 1, draw_music), ctx.blank(2)]


# ---------------------------------------------------------------------------
# The stickers: signs to put on a machine.

ITALIC = "DejaVu Sans:bold:oblique"
CONDENSED = "DejaVu Sans:condensed:bold"


def ground(c, cx, y, rx, seed=0):
    c.ellipse(cx - rx, y - 3, cx + rx, y + 3, fill=(120, 84, 50))
    c.ellipse(cx - rx + 2, y - 2.2, cx + rx - 2, y + 1.6, fill=(160, 116, 70))
    for i, dx in enumerate((-rx * 0.6, -rx * 0.15, rx * 0.35, rx * 0.75)):
        grass(c, cx + dx, y + 0.5, n=4, h=7 + (i + seed) % 3 * 2, spread=7, seed=seed + i)


def post(c, x0, y0, x1, y1, wood=(130, 84, 44)):
    c.rect(x0, y0, x1, y1, fill=INK, radius=1)
    c.gradient(x0 + 1, y0, x1 - 1, y1, [lighter(wood, 0.25), wood, darker(wood, 0.3)], vertical=False)


def board_text(l, x0, y0, x1, y1, text, size, kind=CONDENSED):
    l.text_fit(x0, y0, x1, y1, text, (255, 250, 236), kind, size, stroke=1.2, stroke_fill=(60, 34, 14))


@sheet(1052)
def keep_off(ctx):
    c = ctx.canvas(1)
    ground(c, 41, 79, 26, 1)
    post(c, 35, 40, 46, 80)
    l = layer(c)
    plank(l, 3, 6, 79, 28, seed=1)
    plank(l, 4, 27, 78, 50, seed=2)
    for x, y in ((8, 11), (73, 11), (9, 44), (72, 44)):
        nail(l, x, y)
    board_text(l, 10, 8, 72, 48, tr("KEEP\nOFF!"), 22)
    stamp(c, l, 4, 41, 28)
    return [c, ctx.blank(2)]


@sheet(1053)
def stay_away(ctx):
    c = ctx.canvas(1)
    ground(c, 55, 69, 30, 3)
    post(c, 49, 34, 60, 70)
    l = layer(c)
    plank(l, 3, 12, 107, 29, seed=3)
    plank(l, 6, 28, 104, 44, seed=4)
    for x, y in ((8, 17), (102, 17), (11, 39), (99, 39)):
        nail(l, x, y)
    board_text(l, 12, 14, 99, 42, tr("STAY AWAY!"), 22)
    stamp(c, l, -3, 55, 28)
    return [c, ctx.blank(2)]


def placard(c, x0, y0, x1, y1, base, radius=5):
    c.rect(x0, y0, x1, y1, fill=INK, radius=radius)
    c.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, fill=WHITE, radius=radius - 1)
    c.rect(x0 + 2.5, y0 + 2.5, x1 - 2.5, y1 - 2.5, fill=base, radius=radius - 2)
    c.rect(x0 + 4, y0 + 3.5, x1 - 4, y0 + 5.5, fill=lighter(base, 0.35), radius=1)


def push_sign(ctx, text, base):
    c = ctx.canvas(1)
    w = c.w
    pts = arrow_down(c, w / 2, 40, 79, 22, 50, 22, INK)
    c.polygon(pts, fill=INK, outline=INK, width=2.2)
    arrow_down(c, w / 2, 40, 77.2, 18.5, 45, 20, base, outline=None)
    c.polygon([(w / 2 - 7, 42), (w / 2 - 4, 42), (w / 2 - 4, 56), (w / 2 - 7, 56)], fill=lighter(base, 0.4))
    placard(c, 1, 1, w - 1, 45, base)
    c.text_fit(6, 5, w - 6, 41, text, WHITE, "sans", 20, stroke=1.2, stroke_fill=darker(base, 0.65))
    return [c, ctx.blank(2)]


@sheet(1054)
def push_to_start(ctx):
    return push_sign(ctx, tr("PUSH\nTO START"), (40, 180, 60))


@sheet(1055)
def push_to_stop(ctx):
    return push_sign(ctx, tr("PUSH\nTO STOP"), (220, 40, 40))


@sheet(1056)
def push_here(ctx):
    c = ctx.canvas(1)
    base = (255, 196, 30)
    w = c.w
    pts = [(14, 1), (w - 14, 1), (w - 14, 30), (w - 1.5, 30), (w / 2, 59), (1.5, 30), (14, 30)]
    c.polygon(pts, fill=INK)
    c.polygon([(15.5, 2.5), (w - 15.5, 2.5), (w - 15.5, 31.5), (w - 5.5, 31.5), (w / 2, 56.5), (5.5, 31.5), (15.5, 31.5)], fill=base)
    c.polygon([(17, 4), (w - 17, 4), (w - 17, 7), (17, 7)], fill=lighter(base, 0.5))
    c.text_fit(18, 4, w - 18, 30, tr("PUSH"), INK, "sans", 18)
    c.text_fit(23, 30, w - 23, 44, tr("HERE"), INK, "sans", 13)
    return [c, ctx.blank(2)]


@sheet(1057)
def enter_here(ctx):
    c = ctx.canvas(1)
    base = (60, 120, 230)
    # a bent arrow from under the plate, down to the entrance
    path = [(18, 36), (17, 50), (24, 60), (38, 64)]
    c.line(path, INK, 11)
    c.line(path, base, 8)
    c.polygon([(36, 54.5), (56, 65), (36, 75.5)], fill=INK)
    c.polygon([(37.5, 57), (53, 65), (37.5, 73)], fill=base)
    c.line([(15.8, 40), (15.5, 49)], lighter(base, 0.4), 1.4)
    placard(c, 1, 1, 65, 40, base)
    c.text_fit(5, 4, 61, 37, tr("ENTER\nHERE"), WHITE, "sans", 18, stroke=1.2, stroke_fill=darker(base, 0.65))
    return [c, ctx.blank(2)]


@sheet(1058)
def the_answer_is(ctx):
    c = ctx.canvas(1)
    gold, gold_d = (240, 190, 60), (170, 110, 30)

    def wave(x, amp=2.2):
        return amp * math.sin((x - 10) / 118 * 2 * math.pi)
    # the ribbon's tail at the left, folded behind
    c.polygon([(1, 14), (16, 14), (16, 38), (1, 38), (7, 26)], fill=INK)
    c.polygon([(3.2, 15.2), (16, 15.2), (16, 36.8), (3.2, 36.8), (8.8, 26)], fill=gold_d)
    top = [(x, 9 + wave(x)) for x in range(10, 133, 4)]
    bot = [(x, 33 + wave(x)) for x in range(132, 9, -4)]
    c.polygon(top + bot, fill=INK)
    top_i = [(x, 10.2 + wave(x)) for x in range(11, 132, 4)]
    bot_i = [(x, 31.8 + wave(x)) for x in range(131, 10, -4)]
    c.polygon(top_i + bot_i, fill=gold)
    c.polygon([(x, 11.5 + wave(x)) for x in range(11, 132, 4)] + [(x, 15 + wave(x)) for x in range(131, 10, -4)],
              fill=(255, 232, 140))
    c.polygon([(x, 28 + wave(x)) for x in range(11, 132, 4)] + [(x, 31.8 + wave(x)) for x in range(131, 10, -4)],
              fill=(214, 150, 40))
    c.text_fit(16, 11, 128, 31, tr("THE ANSWER IS"), (90, 40, 10), "DejaVu Serif:condensed:bold", 15)
    # the arrow pointing at the answer
    c.polygon([(133, 13), (143, 13), (143, 5), (156, 21.5), (143, 38), (143, 30), (133, 30)], fill=INK)
    c.polygon([(134.5, 14.5), (144.5, 14.5), (144.5, 9), (154, 21.5), (144.5, 34), (144.5, 28.5), (134.5, 28.5)],
              fill=(60, 190, 70))
    c.line([(136, 16.5), (145, 16.5)], (160, 240, 160), 1.0)
    return [c, ctx.blank(2)]


def microphone(c, cx, top):
    """A studio microphone, 16 wide, 46 tall."""
    c.arc(cx, top + 15, 9.5, 0, 180, INK, 3.0)
    c.arc(cx, top + 15, 9.5, 0, 180, STEEL_L, 1.2)
    c.line([(cx, top + 25), (cx, top + 41)], INK, 3.4)
    c.line([(cx, top + 25), (cx, top + 41)], STEEL, 1.6)
    c.rect(cx - 9, top + 40, cx + 9, top + 45, fill=INK, radius=2)
    c.rect(cx - 8, top + 41, cx + 8, top + 44, fill=STEEL, radius=1.5)
    c.rect(cx - 7, top, cx + 7, top + 24, fill=INK, radius=7)
    c.gradient(cx - 6, top + 1, cx + 6, top + 23, [STEEL_L, STEEL, STEEL_D], vertical=False, radius=6)
    for i in range(5):
        y = top + 4.5 + i * 3.5
        c.line([(cx - 4.5, y), (cx + 4.5, y)], STEEL_D, 0.8)


@sheet(1059)
def voice_activated(ctx):
    c = ctx.canvas(1)
    microphone(c, 13, 3)
    pink, cyan = (230, 50, 170), (40, 190, 230)
    c.rect(28, 2, 116, 24, fill=INK, radius=3)
    c.rect(29, 3, 115, 23, fill=pink, radius=2)
    c.text_fit(32, 4, 112, 22, tr("VOICE"), WHITE, ITALIC, 18, stroke=1, stroke_fill=darker(pink, 0.6))
    c.rect(28, 27, 116, 49, fill=INK, radius=3)
    c.rect(29, 28, 115, 48, fill=cyan, radius=2)
    c.text_fit(32, 30, 112, 47, tr("ACTIVATED"), INK, "sans", 14)
    return [c, ctx.blank(2)]


@sheet(1060)
def mad_scientist(ctx):
    c = ctx.canvas(1)
    w = c.w
    # the barricade: two legs each side and a striped rail
    for x in (24, 102):
        c.line([(x, 50), (x - 7, 79)], INK, 4.4)
        c.line([(x, 50), (x - 7, 79)], (200, 200, 206), 2.4)
        c.line([(x, 50), (x + 7, 79)], INK, 4.4)
        c.line([(x, 50), (x + 7, 79)], (200, 200, 206), 2.4)
    for x in (30, 96):
        c.rect(x - 1.8, 36, x + 1.8, 52, fill=INK)
        c.rect(x - 0.8, 36, x + 0.8, 52, fill=STEEL)
    c.rect(12, 46, 114, 57, fill=INK, radius=1)
    rail = layer(c)
    rail.rect(13, 47, 113, 56, fill=WHITE)
    for i in range(-2, 16):
        x = 13 + i * 8
        rail.polygon([(x, 56), (x + 4, 56), (x + 13, 47), (x + 9, 47)], fill=(230, 50, 30))
    rail.clear(0, 0, 13, rail.h)
    rail.clear(113, 0, rail.w, rail.h)
    rail.clear(0, 0, rail.w, 47)
    rail.clear(0, 56, rail.w, rail.h)
    stamp(c, rail)
    # the sign
    base = (255, 130, 30)
    c.rect(1, 2, w - 1, 39, fill=INK, radius=3)
    c.rect(2, 3, w - 2, 38, fill=(255, 236, 200), radius=2)
    c.rect(3.5, 4.5, w - 3.5, 36.5, fill=base, radius=1.5)
    c.text_fit(14, 5, w - 14, 36, tr("Mad Scientist\nat Work!"), INK, "sans", 16)
    for x in (8.5, w - 8.5):
        c.circle(x, 28, 4.2, fill=INK)
        c.radial(x, 28, 3.4, (255, 255, 200), (250, 200, 0), -0.4, -0.4)
    return [c, ctx.blank(2)]


def sparkle(c, cx, cy, r, col=(255, 255, 220)):
    c.polygon([(cx, cy - r), (cx + r * 0.25, cy - r * 0.25), (cx + r, cy), (cx + r * 0.25, cy + r * 0.25),
               (cx, cy + r), (cx - r * 0.25, cy + r * 0.25), (cx - r, cy), (cx - r * 0.25, cy - r * 0.25)], fill=col)


@sheet(1061)
def its_magic(ctx):
    c = ctx.canvas(1)
    cx, cy = 72, 43
    jit = [1.0, 0.8, 0.95, 0.7, 1.0, 0.85, 0.9, 0.75]
    outer = [(cx + (x - cx) * 1.7, cy + (y - cy) * 1.0) for x, y in star_pts(cx, cy, 20, 42, 14, 8, jit)]
    c.polygon(outer, fill=INK)
    c.polygon(scale_pts(outer, cx, cy, 0.95), fill=(70, 50, 200))
    c.polygon(scale_pts([(cx + (x - cx) * 1.7, cy + (y - cy)) for x, y in star_pts(cx, cy, 16, 32, 14, 21, jit[::-1])], cx, cy, 0.95),
              fill=(120, 90, 240))
    c.ellipse(cx - 48, cy - 22, cx + 48, cy + 22, fill=(160, 120, 255))
    c.ellipse(cx - 40, cy - 17, cx + 40, cy + 17, fill=(190, 160, 255))
    for x, y, r in ((14, 14, 5), (128, 12, 6), (8, 64, 4), (134, 70, 5), (40, 78, 3.5), (104, 4, 3.5), (30, 4, 3)):
        sparkle(c, x, y, r + 0.9, INK)
        sparkle(c, x, y, r)
    c.text_fit(26, 6, 118, 80, tr("It's\nMagic!"), (255, 200, 40), ITALIC, 30, stroke=1.6, stroke_fill=(120, 30, 0),
               shadow=(1.2, 1.2, INK))
    return [c, ctx.blank(2)]
