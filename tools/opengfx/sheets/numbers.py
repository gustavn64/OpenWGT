"""The parts that show and take numbers: the digit sheet the displays share
(100), the Three- and Ten-Digit Displays (1013, 1014), the Slider (1015),
the Numeric Keypad (1027), the Counter (1041), the Mini Constant (1063),
the Digital Clock (1009), the Totaler (1005), the Stopwatch (1004), the
Constant (1021), the Sound Transformer (1044), the Sound Switch (1045) and
the Splitters (1019).

The game draws characters over the bodies at points from the 1010 tables;
each character family is drawn by one function, used both for the
character frames and (unlit) for the body's cells under those points, so
the two meet pixel for pixel."""
from __future__ import annotations

import math

from wgfx import INK, STEEL, STEEL_D, STEEL_L, WHITE, Canvas, darker, lighter, mix, sheet

from ._numbers_kit import key, pixel_text, pixel_text_w, seg_char, seg_char_box, stalk, stamp

# ---------------------------------------------------------------------------
# Colours

# (The ghosts of unlit segments are picked from the game's palette, which
# has few dark tints.)
RED_ON, RED_CORE, RED_OFF = (255, 46, 34), (254, 135, 103), (55, 7, 0)
GREEN_ON, GREEN_OFF = (110, 255, 120), (31, 31, 31)
AMBER_ON, AMBER_OFF = (255, 178, 30), (55, 51, 43)
VFD_ON, VFD_OFF = (90, 240, 230), (31, 31, 31)
LCD_BG, LCD_ON, LCD_OFF = (155, 223, 171), (28, 38, 32), (155, 191, 171)
GLASS_D = (15, 15, 15)
NIXIE_ON, NIXIE_GLOW = (255, 170, 60), (200, 70, 10)

SLATE = (74, 86, 110)
TEAL = (36, 150, 140)
KEYPAD_BLUE = (70, 130, 205)
KEY_GREY = (222, 222, 212)
TOTAL_GREEN = (60, 140, 90)
CONST_GREY = (160, 166, 178)
SWITCH_BLUE = (50, 120, 220)


def screws(c: Canvas, x0, y0, x1, y1, r=1.4, d=3.5):
    for sx in (x0 + d, x1 - d):
        for sy in (y0 + d, y1 - d):
            c.screw(sx, sy, r)


def flat_panel(c: Canvas, x0, y0, x1, y1, face, light, dark, radius=3.0):
    """A raised plate in three flat tones (for hues whose shades the
    game's palette lacks, like red)."""
    c.rect(x0, y0, x1, y1, fill=INK, radius=radius)
    c.rect(x0 + 1, y0 + 1, x1 - 1, y1 - 1, fill=dark, radius=radius - 1)
    c.rect(x0 + 1, y0 + 1, x1 - 2.5, y1 - 2.5, fill=light, radius=radius - 1)
    c.rect(x0 + 2.5, y0 + 2.5, x1 - 2.5, y1 - 2.5, fill=face, radius=max(0, radius - 2))


RED_PANEL = ((187, 0, 0), (248, 88, 64), (119, 0, 0))


def housing(c: Canvas, x0, y0, x1, y1, base, radius=3.0, screw=True):
    c.panel(x0, y0, x1, y1, base, radius=radius, bevel=1.5)
    if screw:
        screws(c, x0, y0, x1, y1)


def window(c: Canvas, x0, y0, x1, y1, fill=GLASS_D):
    """A sunk glass window, its inside flat on the pixel layer (so the
    character frames drawn over it match it)."""
    c.inset(x0, y0, x1, y1, fill, radius=1.5, depth=1.0)
    c.prect(x0 + 2, y0 + 2, x1 - 1, y1 - 1, fill)


def arrow_glyph(up, col):
    def g(c, x, y, w, h):
        cx = x + w / 2
        a, b = h * 0.28, h * 0.72
        hw = min(w, h * 1.4) * 0.32
        pts = [(cx, y + a - 0.5), (cx + hw, y + b), (cx - hw, y + b)] if up else \
              [(cx, y + b + 0.5), (cx + hw, y + a), (cx - hw, y + a)]
        c.polygon(pts, fill=col, outline=INK, width=0.9)
    return g


def counted_knot_stalks(c: Canvas, knots, cx, cy, r):
    """Stalks from each knot towards (cx, cy), up to the circle of radius r."""
    for x, y, _ in knots:
        dx, dy = cx - x - 0.5, cy - y - 0.5
        d = math.hypot(dx, dy)
        t = (d - r + 1) / d
        stalk(c, x, y, x + dx * t, y + dy * t)


# ---------------------------------------------------------------------------
# 100: the red LED digits the displays share, "0123456789-+.E", 13 × 25.

LED_CHARS = "0123456789-+.E"


def red_led(c, x, y, ch, lit=True, cw=None):
    if lit:
        seg_char(c, x, y, 13, 25, ch, on=RED_ON, core=RED_CORE, t=3, cw=cw)
    else:
        seg_char(c, x, y, 13, 25, "8", on=RED_OFF, t=3)


@sheet(100)
def led_digits(ctx):
    out = []
    for n, ch in enumerate(LED_CHARS, 1):
        c = ctx.canvas(n)
        red_led(c, 0, 0, ch, cw=c.w)
        out.append(c)
    return out


def led_display(ctx):
    c = ctx.canvas(1)
    pts = ctx.geo.points(ctx.id)
    housing(c, 9, 1, c.w - 1, c.h - 1, SLATE, screw=False)
    for sx in (14.5, c.w - 5.5):
        for sy in (6.5, c.h - 6.5):
            c.screw(sx, sy, 1.4)
    x0 = min(x for x, _ in pts) - 4
    x1 = max(x for x, _ in pts) + 13 + 4
    y0 = min(y for _, y in pts) - 3
    window(c, x0, y0, x1, y0 + 25 + 6)
    for x, y in pts:
        red_led(c, x, y, "8", lit=False)
    c.knots(ctx.knots())
    return [c, ctx.blank(2)]


@sheet(1013)
def display3(ctx):
    return led_display(ctx)


@sheet(1014)
def display10(ctx):
    return led_display(ctx)


# ---------------------------------------------------------------------------
# 1015: the Slider. 1-14 ".-+E0123456789" (9 × 15), 15 the up button
# pressed, 16 the body, 17 the down button pressed, 18 the knob.

SLIDER_CHARS = ".-+E0123456789"


def amber(c, x, y, ch, lit=True):
    if lit:
        seg_char_box(c, x, y, 9, 15, ch, on=AMBER_ON, t=2)
    else:
        seg_char_box(c, x, y, 9, 15, "8", on=AMBER_OFF, t=2)


def slider_button(c, x, y, up, pressed):
    key(c, x, y, 15, 14, KEY_GREY, pressed, glyph=arrow_glyph(up, (40, 100, 220)))


@sheet(1015)
def slider(ctx):
    pts = ctx.geo.points(1015)
    (dx, dy), (sx, sy), down, up, t0, t1 = pts
    out = []
    for n, ch in enumerate(SLIDER_CHARS, 1):
        c = ctx.canvas(n)
        amber(c, 0, 0, ch)
        out.append(c)
    b = ctx.canvas(15)
    slider_button(b, 0, 0, True, True)
    out.append(b)

    c = ctx.canvas(16)
    bottom = 50
    housing(c, 0, 0, c.w, bottom, TEAL, screw=False)
    for x in (4.5, c.w - 4.5):
        c.screw(x, bottom - 5.5, 1.4)
    window(c, dx - 3, dy - 2, dx + 9 * sx + 9 + 3, dy + 15 + 2)
    for i in range(10):
        amber(c, dx + i * sx, dy + i * sy, "8", lit=False)
    slider_button(c, *down, False, False)
    slider_button(c, *up, True, False)
    # The track: the knob's middle runs from t0 to t1 (+ half a knob).
    kx0, kx1 = t0[0] + 9, t1[0] + 9
    gy = t0[1] + 16
    c.inset(kx0 - 5, gy - 2, kx1 + 6, gy + 3, (20, 30, 34), radius=2)
    for i in range(11):
        x = kx0 + (kx1 - kx0) * i / 10
        tall = 4 if i % 5 == 0 else 2
        c.prect(int(x), t0[1] + 2, int(x) + 1, t0[1] + 2 + tall, darker(TEAL, 0.6))
        c.prect(int(x), gy + 6, int(x) + 1, gy + 6 + tall, darker(TEAL, 0.6))
    c.knots(ctx.knots())
    out.append(c)

    b = ctx.canvas(17)
    slider_button(b, 0, 0, False, True)
    out.append(b)

    k = ctx.canvas(18)
    # A fader cap over the groove (the top ~28 pixels of the knob's box).
    k.rect(2, 2, 16, 28, fill=INK, radius=3)
    k.gradient(3, 3, 15, 27, [lighter((220, 50, 40), 0.5), (220, 50, 40), darker((220, 50, 40), 0.45)], vertical=False, radius=2)
    for y in (8, 11, 19, 22):
        k.line([(5, y), (13, y)], darker((220, 50, 40), 0.5), 0.8, round_ends=False)
    k.rect(3, 14.5, 15, 16, fill=WHITE)
    out.append(k)
    out.append(ctx.blank(19))
    return out


# ---------------------------------------------------------------------------
# 1027: the Numeric Keypad. 1-14 LCD "0123456789.-E8" (6 × 13), 15 the
# body, 16-29 the keys pressed (0-9 . - C ENTER, at the 1012 rects).

KEYPAD_CHARS = "0123456789.-E8"
KEY_LABELS = list("0123456789") + [".", "-", "C", "ENTER"]


def lcd_green(c, x, y, ch, lit=True):
    seg_char_box(c, x, y, 6, 13, ch if lit else "8", on=GREEN_ON if lit else GREEN_OFF, t=1)


def keypad_key(c, x, y, w, h, label, pressed):
    if label == ".":
        def g(c, bx, by, kw, kh):
            c.rect(bx + kw / 2 - 1.2, by + kh / 2 + 1, bx + kw / 2 + 1.2, by + kh / 2 + 3.4, fill=INK)
        key(c, x, y, w, h, KEY_GREY, pressed, glyph=g)
    elif label == "-":
        def g(c, bx, by, kw, kh):
            c.rect(bx + 5, by + kh / 2 - 0.9, bx + kw - 5, by + kh / 2 + 0.9, fill=INK)
        key(c, x, y, w, h, KEY_GREY, pressed, glyph=g)
    elif label == "ENTER":
        key(c, x, y, w, h, (250, 170, 60), pressed, label="ENTER", kind="cond", size=10)
    elif label == "C":
        key(c, x, y, w, h, KEY_GREY, pressed, label="C", label_col=(200, 30, 30), size=13)
    else:
        key(c, x, y, w, h, KEY_GREY, pressed, label=label, size=13)


@sheet(1027)
def keypad(ctx):
    out = []
    for n, ch in enumerate(KEYPAD_CHARS, 1):
        c = ctx.canvas(n)
        lcd_green(c, 0, 0, ch)
        out.append(c)
    c = ctx.canvas(15)
    (px, py), _ = ctx.geo.points(1027)[:2]
    rects = ctx.geo.rects(1027)
    right = max(r[2] for r in rects) + 9
    housing(c, 0, 0, right, c.h, KEYPAD_BLUE, radius=4, screw=False)
    for sx in (4, right - 4):
        c.screw(sx, c.h - 5, 1.4)
    window(c, px - 3, py - 3, px + 70 + 2, py + 13 + 3)
    for i in range(10):
        lcd_green(c, px + 7 * i, py, "8", lit=False)
    # A groove round the keys.
    kx0 = min(r[0] for r in rects) - 3
    ky0 = min(r[1] for r in rects) - 3
    c.rect(kx0, ky0, right - 6, max(r[3] for r in rects) + 5, fill=darker(KEYPAD_BLUE, 0.3), radius=2)
    c.rect(kx0 + 1, ky0 + 1, right - 6, max(r[3] for r in rects) + 5, fill=darker(KEYPAD_BLUE, 0.1), radius=2)
    sizes = [ctx.size(16 + i) for i in range(14)]
    for (x0, y0, _, _), (w, h), lab in zip(rects, sizes, KEY_LABELS):
        keypad_key(c, x0, y0, w, h, lab, False)
    c.knots(ctx.knots())
    out.append(c)
    for i, lab in enumerate(KEY_LABELS):
        k = ctx.canvas(16 + i)
        keypad_key(k, 0, 0, k.w, k.h, lab, True)
        out.append(k)
    out.append(ctx.blank(30))
    return out


# ---------------------------------------------------------------------------
# 1041: the Counter (a tally counter). 1 just counted, 2 waiting; 4-13 its
# digits (7 × 13). 1063: the Mini Constant, 1-10 digits, 11 the body.


def green7(c, x, y, ch, lit=True):
    seg_char_box(c, x, y, 7, 13, ch if lit else "8", on=GREEN_ON if lit else GREEN_OFF, t=2)


def counter_body(ctx, n, pressed):
    c = ctx.canvas(n)
    pts = ctx.geo.points(1041)
    x0, x1 = 10, 68
    # The finger ring underneath.
    cx = (x0 + x1) / 2
    c.circle(cx, 56, 9.5, fill=INK)
    c.circle(cx, 56, 8.5, fill=STEEL_D)
    c.arc(cx, 56, 7.6, 160, 290, STEEL_L, 1.2)
    c.circle(cx, 56, 5.5, fill=INK)
    c.circle(cx, 56, 4.6, fill=(0, 0, 0, 0))
    # The plunger.
    px = 54
    top = 10 if pressed else 3
    c.rect(px - 2, top + 3, px + 3, 18, fill=INK)
    c.gradient(px - 1, top + 3, px + 2, 18, [STEEL_L, STEEL, STEEL_D], vertical=False)
    c.rect(px - 6, top, px + 7, top + 5, fill=INK, radius=2)
    c.gradient(px - 5, top + 1, px + 6, top + 4, [STEEL_L, STEEL], radius=1)
    # The case.
    c.panel(x0, 15, x1, 50, (176, 186, 198), radius=7, bevel=1.5)
    xs = [x for x, _ in pts]
    wy = pts[0][1]
    window(c, min(xs) - 4, wy - 4, max(xs) + 7 + 4, wy + 13 + 4)
    for x, y in pts:
        green7(c, x, y, "8", lit=False)
    # The "counted" lamp.
    lc = (255, 70, 40) if pressed else (90, 20, 20)
    c.circle(17.5, 20.5, 2.6, fill=INK)
    c.circle(17.5, 20.5, 1.8, fill=lc)
    if pressed:
        c.circle(17, 20, 0.8, fill=(255, 220, 180))
        for a in (-40, 0, 40):
            r = math.radians(a - 90)
            c.line([(px + 0.5 + math.cos(r) * 9, 12 + math.sin(r) * 9), (px + 0.5 + math.cos(r) * 12, 12 + math.sin(r) * 12)], INK, 1.0)
    c.knots(ctx.knots(1041))
    return c


@sheet(1041)
def counter(ctx):
    out = [counter_body(ctx, 1, True), counter_body(ctx, 2, False), ctx.blank(3)]
    for d in range(10):
        c = ctx.canvas(4 + d)
        green7(c, 0, 0, str(d))
        out.append(c)
    return out


@sheet(1063)
def mini_constant(ctx):
    out = []
    for d in range(10):
        c = ctx.canvas(1 + d)
        green7(c, 0, 0, str(d))
        out.append(c)
    c = ctx.canvas(11)
    pts = ctx.geo.points(1063)
    housing(c, 0, 1, 55, c.h - 1, CONST_GREY, screw=False)
    xs = [x for x, _ in pts]
    wy = pts[0][1]
    window(c, min(xs) - 4, wy - 4, max(xs) + 7 + 4, wy + 13 + 4)
    for x, y in pts:
        green7(c, x, y, "8", lit=False)
    c.knots(ctx.knots())
    out += [c, ctx.blank(12)]
    return out


# ---------------------------------------------------------------------------
# 1009: the Digital Clock. 1 the body, 3 AM, 4 PM (26 × 10), 5-14 the
# digits (10 × 13), glowing like a vacuum fluorescent display.


def vfd(c, x, y, ch, lit=True):
    seg_char_box(c, x, y, 10, 13, ch if lit else "8", on=VFD_ON if lit else VFD_OFF, t=2)


@sheet(1009)
def digital_clock(ctx):
    pts = ctx.geo.points(1009)
    c = ctx.canvas(1)
    right = 117
    flat_panel(c, 1, 1, right, c.h - 1, *RED_PANEL, radius=4)
    for sy in (5.5, c.h - 5.5):
        c.screw(right - 4.5, sy, 1.4)
    digits = pts[1:]
    x0 = min(pts[0][0], digits[0][0]) - 4
    x1 = digits[-1][0] + 10 + 4
    window(c, x0, pts[0][1] - 3, x1, digits[0][1] + 13 + 4)
    for x, y in digits:
        vfd(c, x, y, "8", lit=False)
    # The colons between hours, minutes and seconds.
    y = digits[0][1]
    for a, b in ((digits[1], digits[2]), (digits[3], digits[4])):
        cx = (a[0] + 10 + b[0]) // 2 - 1
        for dy in (3, 8):
            c.prect(cx, y + dy, cx + 2, y + dy + 2, mix(VFD_ON, VFD_OFF, 0.35))
    # A speaker grille on the right.
    gx0, gx1 = x1 + 4, right - 8
    for gy in range(9, c.h - 9, 4):
        c.rect(gx0, gy, gx1, gy + 2, fill=(83, 7, 0), radius=1)
    c.knots(ctx.knots())
    out = [c, ctx.blank(2)]
    for s in ("AM", "PM"):
        k = ctx.canvas(3 + len(out) - 2)
        pixel_text(k, (k.w - pixel_text_w(s)) // 2, (k.h - 6) // 2, s, VFD_ON)
        out.append(k)
    for d in range(10):
        k = ctx.canvas(5 + d)
        vfd(k, 0, 0, str(d))
        out.append(k)
    return out


# ---------------------------------------------------------------------------
# 1005: the Totaler. 1 the body, 3-6 the labels AVERAGE TOTAL MAXIMUM
# MINIMUM (96 × 19), 7-20 ".-E+0123456789" in nixie tubes (16 × 18).

TOTALER_CHARS = ".-E+0123456789"
TOTALER_LABELS = [("AVERAGE", (30, 90, 200)), ("TOTAL", (200, 40, 40)),
                  ("MAXIMUM", (30, 140, 60)), ("MINIMUM", (150, 70, 20))]


def nixie(ch):
    c = Canvas(16, 18)
    c.rect(0, 0, 16, 18, fill=(10, 8, 8))
    c.rect(1, 0, 15, 18, fill=(34, 24, 22), radius=3)
    c.gradient(2, 1, 14, 17, [(48, 34, 30), (26, 18, 16)], radius=2.5)
    for y in range(2, 17, 2):
        for x in range(3 + (y // 2) % 2, 14, 2):
            c.pixel(x, y, (58, 42, 34))
    if ch.strip():
        s = {"-": "–"}.get(ch, ch)
        c.text(8, 9.5 if ch != "." else 9, s, 15, NIXIE_GLOW, "sans-book", stroke=1.0, stroke_fill=mix(NIXIE_GLOW, (34, 24, 22), 0.4))
        c.text(8, 9.5 if ch != "." else 9, s, 15, NIXIE_ON, "sans-book")
    c.line([(3, 2.5), (3, 7)], (110, 90, 84), 0.7, round_ends=False)
    return c


def totaler_label(text, col):
    c = Canvas(96, 19)
    c.rect(0, 0, 96, 19, fill=INK, radius=2)
    c.gradient(1, 1, 95, 18, [(252, 246, 226), (226, 216, 190)], radius=1.5)
    c.rect(1, 17, 95, 18, fill=(180, 168, 140))
    for x in (4, 92):
        c.circle(x, 9.5, 1.2, fill=(160, 140, 100))
    c.text(48, 10, " ".join(text), 11, col, "sans")
    return c


@sheet(1005)
def totaler(ctx):
    pts = ctx.geo.points(1005)
    lab, cells = pts[0], pts[1:]
    c = ctx.canvas(1)
    housing(c, 0, 9, c.w, 59, TOTAL_GREEN, radius=4, screw=False)
    for sx in (4.5, c.w - 4.5):
        for sy in (13.5, 30):
            c.screw(sx, sy, 1.4)
    c.rect(lab[0] - 2, lab[1] - 2, lab[0] + 98, lab[1] + 21, fill=darker(TOTAL_GREEN, 0.55), radius=2)
    c.rect(cells[0][0] - 3, cells[0][1] - 2, cells[-1][0] + 16 + 3, cells[0][1] + 18 + 2, fill=INK, radius=2)
    blank = nixie(" ")
    for x, y in cells:
        stamp(c, blank, x, y)
    c.knots(ctx.knots())
    out = [c, ctx.blank(2)] + [totaler_label(t, col) for t, col in TOTALER_LABELS]
    out += [nixie(ch) for ch in TOTALER_CHARS]
    return out


# ---------------------------------------------------------------------------
# 1004: the Stopwatch. 1 the body, 3-12 LCD digits (9 × 15).


def lcd(c, x, y, ch, lit=True):
    seg_char_box(c, x, y, 9, 15, ch if lit else "8", on=LCD_ON if lit else LCD_OFF, t=2)


@sheet(1004)
def stopwatch(ctx):
    pts = ctx.geo.points(1004)
    knots = ctx.knots()
    c = ctx.canvas(1)
    cx, cy, r = 43.5, 51.5, 30.0
    # The crown, under the reset knot.
    kx = knots[2][0] + 0.5
    c.rect(kx - 3, 15, kx + 3, cy - r + 3, fill=INK)
    c.gradient(kx - 2, 15, kx + 2, cy - r + 3, [STEEL_L, STEEL, STEEL_D], vertical=False)
    c.rect(kx - 6, 9, kx + 6, 16, fill=INK, radius=2)
    c.gradient(kx - 5, 10, kx + 5, 15, [STEEL_L, STEEL, STEEL_D], vertical=False, radius=1)
    for x in range(int(kx - 4), int(kx + 5), 2):
        c.prect(x, 11, x + 1, 14, STEEL_D)
    # The pushers for start and stop.
    counted_knot_stalks(c, [knots[0], knots[1]], cx, cy, r)
    # The case.
    c.circle(cx, cy, r + 0.6, fill=INK)
    c.radial(cx, cy, r - 0.4, STEEL_L, STEEL_D, -0.6, -0.6)
    c.circle(cx, cy, r - 4, fill=INK)
    c.radial(cx, cy, r - 5, WHITE, (226, 226, 214), -0.5, -0.5)
    for i in range(12):
        a = math.radians(i * 30)
        c.line([(cx + math.cos(a) * (r - 7.5), cy + math.sin(a) * (r - 7.5)),
                (cx + math.cos(a) * (r - 5.5), cy + math.sin(a) * (r - 5.5))], (120, 120, 130), 0.8, round_ends=False)
    xs = [x for x, _ in pts]
    wy = pts[0][1]
    window(c, min(xs) - 3, wy - 3, max(xs) + 9 + 3, wy + 15 + 3, LCD_BG)
    for x, y in pts:
        lcd(c, x, y, "8", lit=False)
    mx = (pts[1][0] + 9 + pts[2][0]) // 2 - 1
    for dy in (4, 9):
        c.prect(mx, wy + dy, mx + 2, wy + dy + 2, LCD_ON)
    red = (200, 30, 30)
    c.text(cx, 28.5, "R", 7, (30, 140, 60), "sans")
    c.text(cx, 37.5, "START", 8, red, "cond")
    c.text(cx, 70, "STOP", 8, red, "cond")
    c.knot(*knots[0])
    c.knot(*knots[1])
    c.knot(*knots[2], side="b", reach=6)
    c.knot(*knots[3], side="l", reach=6)
    out = [c, ctx.blank(2)]
    for d in range(10):
        k = ctx.canvas(3 + d)
        lcd(k, 0, 0, str(d))
        out.append(k)
    return out


# ---------------------------------------------------------------------------
# 1021: the Constant. 1-3 the up arrows pressed (hundreds, tens, ones), 4
# the body, 5-7 the down arrows pressed, 8-17 drum digits (18 × 20).

UP_COL, DOWN_COL = (60, 200, 90), (240, 110, 40)


def const_arrow(c, x, y, up, pressed):
    col = UP_COL if up else DOWN_COL
    key(c, x, y, 25, 14, (200, 204, 212), pressed, glyph=arrow_glyph(up, lighter(col, 0.45) if pressed else col))


def drum(ch):
    c = Canvas(18, 20)
    c.rect(0, 0, 18, 20, fill=INK)
    c.gradient(1, 0, 17, 20, [(70, 66, 60), (200, 194, 180), (252, 250, 240), (252, 250, 240), (200, 194, 180), (70, 66, 60)])
    if ch.strip():
        c.text(9, 10.5, ch, 16, INK, "sans")
    c.rect(1, 0, 17, 1, fill=(40, 38, 36))
    c.rect(1, 19, 17, 20, fill=(40, 38, 36))
    return c


@sheet(1021)
def constant(ctx):
    pts = ctx.geo.points(1021)
    ups, digs, downs = pts[0:3], pts[3:6], pts[6:9]
    out = []
    for x, y in ups:
        k = ctx.canvas(len(out) + 1)
        const_arrow(k, 0, 0, True, True)
        out.append(k)
    c = ctx.canvas(4)
    right = 86
    housing(c, 1, 10, right, 61, CONST_GREY, radius=3, screw=False)
    for sx in (5, right - 4):
        for sy in (19, 52):
            c.screw(sx, sy, 1.3)
    wx0, wx1 = digs[0][0] - 3, digs[-1][0] + 18 + 3
    window(c, wx0, digs[0][1] - 3, wx1, digs[0][1] + 20 + 3)
    blank = drum(" ")
    for x, y in digs:
        stamp(c, blank, x, y)
    for x, y in ups:
        const_arrow(c, x, y, True, False)
    for x, y in downs:
        const_arrow(c, x, y, False, False)
    c.knots(ctx.knots())
    out.append(c)
    for x, y in downs:
        k = ctx.canvas(len(out) + 1)
        const_arrow(k, 0, 0, False, True)
        out.append(k)
    out += [drum(str(d)) for d in range(10)]
    out.append(ctx.blank(18))
    return out


# ---------------------------------------------------------------------------
# 1044: the Sound Transformer. 1 the body, 3-9 a level meter lit to 1-7
# blocks (6 × 21) over its five slots.

LEVEL_COLS = [(80, 240, 90)] * 4 + [(250, 220, 40)] * 2 + [(255, 60, 40)]


def level(c, x, y, n):
    for j in range(7):
        col = LEVEL_COLS[j] if j < n else darker(LEVEL_COLS[j], 0.75)
        c.prect(x, y + 19 - 3 * j, x + 6, y + 21 - 3 * j, col)


@sheet(1044)
def sound_transformer(ctx):
    pts = ctx.geo.points(1044)
    c = ctx.canvas(1)
    housing(c, 8, 1, 91, c.h - 1, SLATE, radius=3, screw=False)
    for sx in (13, 87):
        for sy in (6, 24):
            c.screw(sx, sy, 1.3)
    for x, y in pts:
        window(c, x - 2, y - 2, x + 6 + 2, y + 21 + 2)
        level(c, x, y, 0)
    c.knots(ctx.knots())
    out = [c, ctx.blank(2)]
    for n in range(1, 8):
        k = ctx.canvas(2 + n)
        level(k, 0, 0, n)
        out.append(k)
    return out


# ---------------------------------------------------------------------------
# 1045: the Sound Switch. 1 the body (a microphone, "ON" lit), 3 "OFF"
# over its window (9 × 21).


@sheet(1045)
def sound_switch(ctx):
    (wx, wy), = ctx.geo.points(1045)
    c = ctx.canvas(1)
    housing(c, 1, 1, 60, c.h - 1, SWITCH_BLUE, radius=4, screw=False)
    for sx in (5.5, 56):
        for sy in (5.5, c.h - 5.5):
            c.screw(sx, sy, 1.3)
    mx, my, mr = 20, 22.5, 13.5
    c.circle(mx, my, mr + 1, fill=INK)
    c.radial(mx, my, mr, STEEL_L, STEEL_D, -0.6, -0.6)
    c.circle(mx, my, mr - 3, fill=INK)
    c.circle(mx, my, mr - 3.8, fill=(40, 40, 48))
    for gy in range(int(my - mr + 5), int(my + mr - 4), 2):
        for gx in range(int(mx - mr + 5), int(mx + mr - 4), 2):
            if math.hypot(gx + 0.5 - mx, gy + 0.5 - my) < mr - 5:
                c.pixel(gx, gy, (110, 112, 124))
    window(c, wx - 3, wy - 3, wx + 9 + 3, wy + 21 + 3)
    pixel_text_v(c, wx + 1, wy + 4, "ON", GREEN_ON)
    c.knots(ctx.knots())
    off = ctx.canvas(3)
    off.prect(0, 0, off.w, off.h, GLASS_D)
    pixel_text_v(off, 1, 0, "OFF", (255, 60, 40))
    return [c, ctx.blank(2), off]


def pixel_text_v(c, x, y, s, col):
    for i, ch in enumerate(s):
        pixel_text(c, x, y + i * 7 + (1 if len(s) == 3 else i * 2), ch, col)


# ---------------------------------------------------------------------------
# 1019: the Splitters, two to four ways (knot tables 1033 to 1035).


@sheet(1019)
def splitters(ctx):
    out = []
    for n, table in ((1, 1033), (2, 1034), (3, 1035)):
        c = ctx.canvas(n)
        k = ctx.knots(table)
        flat_panel(c, 9, 2, 35, c.h - 2, *RED_PANEL)
        (_, iy, _), outs = k[0], k[1:]
        bx = 21.5
        ys = [y + 0.5 for _, y, _ in outs]
        path_col, edge = (255, 226, 160), (83, 7, 0)
        for w, col in ((3.2, edge), (1.6, path_col)):
            c.line([(12, iy + 0.5), (bx, iy + 0.5)], col, w)
            c.line([(bx, ys[0]), (bx, ys[-1])], col, w)
            for y in ys:
                c.line([(bx, y), (32, y)], col, w)
        c.circle(bx, iy + 0.5, 2.2, fill=edge)
        c.circle(bx, iy + 0.5, 1.4, fill=path_col)
        c.knots(k)
        out.append(c)
    out += [ctx.blank(4), ctx.blank(5)]
    return out
