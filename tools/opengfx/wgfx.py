"""The drawing kit for OpenWGFX, the port's own graphics
(crates/engine/opengfx, engine::opengfx). Every frame is drawn anew: shapes
are drawn smooth at SS times the size, scaled down, then taken to the
game's palette; pixel-exact things (LED segments, 1-pixel lines) go on a
layer drawn at the frame's own size, over the smooth one.

Coordinates are the frame's pixels, floats allowed; (0, 0) is the frame's
top left, whatever its rectangle's offset in the sheet.

The geometry the game needs (frame sizes, the knots' places, the tables of
points and rectangles where the code draws digits and needles) comes from
an export of the player's CD (`openwgt --export DIR`); only sizes and
places, never pixels: nothing here looks at the CD's pictures.
"""
from __future__ import annotations

import math
import os
import shutil
import struct
import subprocess
from functools import lru_cache
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

SS = 4  # supersampling

# ---------------------------------------------------------------------------
# Words. The pictures' words are drawn in LANG: "en", or "sv" for the
# Swedish overlay pack (crates/engine/opengfx-sv, make.py --lang sv). Each
# goes through tr(), which takes the English word to LANG's.

LANG = "en"

WORDS = {
    "sv": {
        # the main toolbar, the Stopwatch
        "GO": "START", "STOP": "STOPP",
        # the settings dialogs' arrows by the knots
        "IN": "IN", "OUT": "UT",
        # the logic gates
        "AND": "OCH", "OR": "ELLER", "XOR": "XELLER", "NOT": "INTE",
        # switches, the light bulb, the Sound Switch
        "ON": "PÅ", "OFF": "AV",
        # the tray's Text Label, the Converter's units, the Plotter's reset
        "Label": "Etikett", "Ft": "Fot", "RESET": "NOLLA",
        # the Digital Clock, the Totaler, the Numeric Keypad
        "AM": "FM", "PM": "EM",
        "AVERAGE": "MEDEL", "TOTAL": "SUMMA", "MAXIMUM": "STÖRST", "MINIMUM": "MINST",
        "ENTER": "RETUR",
        # the signs
        "KEEP\nOFF!": "BETRÄD\nEJ!", "STAY AWAY!": "HÅLL AVSTÅND!",
        "PUSH\nTO START": "TRYCK\nFÖR START", "PUSH\nTO STOP": "TRYCK\nFÖR STOPP",
        "PUSH": "TRYCK", "HERE": "HÄR", "ENTER\nHERE": "GÅ IN\nHÄR",
        "THE ANSWER IS": "SVARET ÄR", "VOICE": "RÖST", "ACTIVATED": "STYRD",
        "Mad Scientist\nat Work!": "Galen forskare\ni arbete!", "It's\nMagic!": "Det är\nmagi!",
        # the Coin Toss
        "HEADS": "KRONA", "TAILS": "KLAVE",
        # the Zodiac
        "Aquarius": "Vattumannen", "Pisces": "Fiskarna", "Aries": "Väduren", "Taurus": "Oxen",
        "Gemini": "Tvillingarna", "Cancer": "Kräftan", "Leo": "Lejonet", "Virgo": "Jungfrun",
        "Libra": "Vågen", "Scorpio": "Skorpionen", "Sagittarius": "Skytten", "Capricorn": "Stenbocken",
        # the Fortune Teller
        "No": "Nej", "Yes": "Ja", "NEVER!": "ALDRIG!", "Ask\nagain\nlater": "Fråga\nigen\nsenare",
        "Maybe": "Kanske", "For sure": "Absolut", "Of\nCourse!": "Så\nklart!", "Not Today": "Inte\nidag",
        # the About box
        "for Widget Workshop": "för Widget Workshop",
    },
}


def tr(word: str) -> str:
    """`word` (English) in LANG."""
    return WORDS.get(LANG, {}).get(word, word)

# ---------------------------------------------------------------------------
# Colours: the kit's own names, taken to the palette in the end.

BLACK = (0, 0, 0)
WHITE = (255, 255, 255)
INK = (24, 22, 30)            # outlines
STEEL = (150, 160, 172)
STEEL_D = (88, 96, 108)
STEEL_L = (214, 220, 228)
BRASS = (214, 168, 64)
BRASS_D = (140, 98, 30)
BRASS_L = (250, 222, 140)
COPPER = (196, 104, 52)
LED_RED = (255, 40, 30)
LED_RED_OFF = (70, 12, 12)
LED_GREEN = (60, 255, 90)
LED_GREEN_OFF = (10, 60, 20)
GLASS = (20, 26, 34)
PANEL_BLUE = (40, 110, 220)
PANEL_TEAL = (30, 150, 150)

# Knots (where wires plug in): an input is a socket, an output a lamp-ball.
KNOT_IN_RIM = (232, 186, 70)
KNOT_IN_HOLE = (36, 20, 10)
KNOT_OUT = (255, 120, 20)
KNOT_OUT_D = (150, 40, 0)
KNOT_OUT_L = (255, 236, 150)


def mix(a, b, t: float):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(3))


def lighter(c, t=0.35):
    return mix(c, WHITE, t)


def darker(c, t=0.35):
    return mix(c, BLACK, t)


# ---------------------------------------------------------------------------
# Fonts

FONT_NAMES = {
    "sans": "DejaVu Sans:bold",
    "sans-book": "DejaVu Sans",
    "cond": "DejaVu Sans:condensed:bold",
    "serif": "DejaVu Serif:bold",
    "mono": "DejaVu Sans Mono:bold",
    "liberation": "Liberation Sans:bold",
    "liberation-book": "Liberation Sans",
    "narrow": "Liberation Sans Narrow:bold",
}


@lru_cache(maxsize=None)
def font_file(kind: str) -> str:
    env = os.environ.get("OPENGFX_FONT_" + kind.upper().replace("-", "_"))
    if env:
        return env
    pattern = FONT_NAMES.get(kind, kind)
    if shutil.which("fc-match"):
        out = subprocess.run(["fc-match", "-f", "%{file}", pattern], capture_output=True, text=True)
        if out.returncode == 0 and out.stdout:
            return out.stdout
    raise SystemExit(f"no font for {kind!r} ({pattern}); install DejaVu and Liberation fonts")


@lru_cache(maxsize=None)
def font(kind: str, px: float) -> ImageFont.FreeTypeFont:
    return ImageFont.truetype(font_file(kind), max(1, int(round(px))))


# ---------------------------------------------------------------------------
# The game's palette and the CD's geometry


class Geometry:
    """Sizes and places from an export of the CD (`openwgt --export`)."""

    def __init__(self, export: Path):
        self.export = Path(export)
        d = (self.export / "WIDGET/1003/101.bin").read_bytes()
        self.palette = [(d[i * 4 + 2], d[i * 4 + 1], d[i * 4]) for i in range(256)]

    def _res(self, typ: int, id: int, module="WWGADGET"):
        base = self.export / module / str(typ)
        if not base.is_dir():
            return None
        for f in base.iterdir():
            if f.stem == str(id):
                return f.read_bytes()
        return None

    def sheet_ids(self) -> list[int]:
        return sorted(int(f.stem) for f in (self.export / "WWGADGET/1004").iterdir())

    def frames(self, sheet: int) -> list[tuple[int, int, int, int]]:
        """The frames' rectangles: (top, left, bottom, right)."""
        d = self._res(1004, sheet)
        n = struct.unpack_from("<H", d, 2)[0]
        return [struct.unpack_from("<hhhh", d, 4 + i * 12) for i in range(n)]

    def knots(self, table: int) -> list[tuple[int, int, bool]]:
        """(x, y, is_output) of knot table `table` (type 1014)."""
        d = self._res(1014, table)
        if not d:
            return []
        n = struct.unpack_from("<H", d, 0)[0]
        out = []
        for i in range(n):
            v, h, k = struct.unpack_from("<hhH", d, 2 + i * 6)
            out.append((h, v, k != 0))
        return out

    def points(self, id: int) -> list[tuple[int, int]]:
        """Point table `id` (type 1010) as (x, y)."""
        d = self._res(1010, id)
        if not d:
            return []
        n = struct.unpack_from("<H", d, 0)[0]
        return [(struct.unpack_from("<h", d, 4 + i * 4)[0], struct.unpack_from("<h", d, 2 + i * 4)[0]) for i in range(n)]

    def rects(self, id: int) -> list[tuple[int, int, int, int]]:
        """Rectangle table `id` (type 1012) as (x0, y0, x1, y1)."""
        d = self._res(1012, id)
        if not d:
            return []
        n = struct.unpack_from("<H", d, 0)[0]
        out = []
        for i in range(n):
            t, l, b, r = struct.unpack_from("<hhhh", d, 2 + i * 8)
            out.append((l, t, r, b))
        return out


# ---------------------------------------------------------------------------
# Quantizing


class Quantizer:
    def __init__(self, palette):
        self.palette = palette
        self.cache: dict = {}

    def nearest(self, c):
        c = c[:3]
        hit = self.cache.get(c)
        if hit is not None:
            return hit
        best, bd = 0, 1 << 60
        r, g, b = c
        for i, (pr, pg, pb) in enumerate(self.palette):
            rm = (r + pr) / 2
            d = (2 + rm / 256) * (r - pr) ** 2 + 4 * (g - pg) ** 2 + (2 + (255 - rm) / 256) * (b - pb) ** 2
            if d < bd:
                best, bd = i, d
        self.cache[c] = self.palette[best]
        return self.palette[best]


BAYER4 = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]

# ---------------------------------------------------------------------------
# A frame


class Canvas:
    """One frame, `w` × `h` pixels."""

    def __init__(self, w: int, h: int):
        self.w, self.h = w, h
        self.hi = Image.new("RGBA", (max(1, w * SS), max(1, h * SS)), (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.hi)
        self.lo = Image.new("RGBA", (max(1, w), max(1, h)), (0, 0, 0, 0))
        self.ld = ImageDraw.Draw(self.lo)
        # Areas to dither instead of rounding: (x0, y0, x1, y1).
        self.dither_areas: list = []

    # -- smooth layer -------------------------------------------------------

    @staticmethod
    def _s(v):
        return v * SS

    def _box(self, x0, y0, x1, y1):
        return [x0 * SS, y0 * SS, x1 * SS - 1, y1 * SS - 1]

    def rect(self, x0, y0, x1, y1, fill=None, outline=None, width=1.0, radius=0.0):
        """A rectangle covering x0..x1 × y0..y1 (x1, y1 exclusive)."""
        box = self._box(x0, y0, x1, y1)
        wd = max(1, int(round(width * SS))) if outline else 0
        if radius > 0:
            self.d.rounded_rectangle(box, radius=radius * SS, fill=_c(fill), outline=_c(outline), width=wd)
        else:
            self.d.rectangle(box, fill=_c(fill), outline=_c(outline), width=wd)

    def ellipse(self, x0, y0, x1, y1, fill=None, outline=None, width=1.0):
        wd = max(1, int(round(width * SS))) if outline else 0
        self.d.ellipse(self._box(x0, y0, x1, y1), fill=_c(fill), outline=_c(outline), width=wd)

    def circle(self, cx, cy, r, fill=None, outline=None, width=1.0):
        self.ellipse(cx - r, cy - r, cx + r, cy + r, fill, outline, width)

    def polygon(self, pts, fill=None, outline=None, width=1.0):
        p = [(x * SS, y * SS) for x, y in pts]
        self.d.polygon(p, fill=_c(fill))
        if outline:
            self.d.line(p + [p[0]], fill=_c(outline), width=max(1, int(round(width * SS))), joint="curve")

    def line(self, pts, color, width=1.0, round_ends=True):
        p = [(x * SS, y * SS) for x, y in pts]
        w = max(1, int(round(width * SS)))
        self.d.line(p, fill=_c(color), width=w, joint="curve")
        if round_ends:
            for x, y in (p[0], p[-1]):
                self.d.ellipse([x - w / 2, y - w / 2, x + w / 2, y + w / 2], fill=_c(color))

    def arc(self, cx, cy, r, start, end, color, width=1.0):
        self.d.arc(self._box(cx - r, cy - r, cx + r, cy + r), start, end, fill=_c(color), width=max(1, int(round(width * SS))))

    def pieslice(self, cx, cy, r, start, end, fill):
        self.d.pieslice(self._box(cx - r, cy - r, cx + r, cy + r), start, end, fill=_c(fill))

    def gradient(self, x0, y0, x1, y1, stops, vertical=True, radius=0.0, mask=None):
        """A gradient over the box, `stops` a list of colours (evenly spaced)
        or of (t, colour); clipped to a rounded box, or to `mask` (an "L"
        image the size of the smooth layer)."""
        st = _stops(stops)
        bx = [int(v) for v in self._box(x0, y0, x1, y1)]
        W, H = bx[2] - bx[0] + 1, bx[3] - bx[1] + 1
        if W <= 0 or H <= 0:
            return
        g = Image.new("RGBA", (W, H))
        n = H if vertical else W
        for i in range(n):
            c = _at(st, i / max(1, n - 1))
            if vertical:
                g.paste(c + (255,), (0, i, W, i + 1))
            else:
                g.paste(c + (255,), (i, 0, i + 1, H))
        m = Image.new("L", (W, H), 0)
        ImageDraw.Draw(m).rounded_rectangle([0, 0, W - 1, H - 1], radius=radius * SS, fill=255) if radius else m.paste(255, (0, 0, W, H))
        if mask is not None:
            m = ImageChops.multiply(m, mask.crop((bx[0], bx[1], bx[0] + W, bx[1] + H)))
        self.hi.paste(g, (bx[0], bx[1]), m)

    def radial(self, cx, cy, r, inner, outer, hx=0.0, hy=0.0):
        """A ball: a radial gradient from `inner` (at the highlight, offset
        hx, hy as fractions of r) to `outer` at the rim."""
        steps = max(4, int(r * SS))
        for i in range(steps, 0, -1):
            t = i / steps
            rr = r * t
            c = mix(inner, outer, t ** 1.4)
            ox, oy = cx + hx * r * (1 - t), cy + hy * r * (1 - t)
            self.ellipse(ox - rr, oy - rr, ox + rr, oy + rr, fill=c)

    def text(self, x, y, s, size, fill, kind="sans", anchor="mm", stroke=0.0, stroke_fill=None, spacing=0.0, shadow=None):
        """Text centred (anchor "mm") or placed at x, y; `size` in pixels of
        the frame (the em)."""
        f = font(kind, size * SS)
        if shadow:
            sx, sy, sc = shadow
            self._text(x + sx, y + sy, s, f, sc, anchor, stroke, sc, spacing)
        self._text(x, y, s, f, fill, anchor, stroke, stroke_fill, spacing)

    def _text(self, x, y, s, f, fill, anchor, stroke, stroke_fill, spacing):
        kw = {}
        if stroke:
            kw = {"stroke_width": int(round(stroke * SS)), "stroke_fill": _c(stroke_fill or INK)}
        if "\n" in s:
            self.d.multiline_text((x * SS, y * SS), s, font=f, fill=_c(fill), anchor=anchor, align="center", spacing=spacing * SS, **kw)
        else:
            self.d.text((x * SS, y * SS), s, font=f, fill=_c(fill), anchor=anchor, **kw)

    def text_fit(self, x0, y0, x1, y1, s, fill, kind="sans", max_size=40.0, stroke=0.0, stroke_fill=None, shadow=None):
        """The largest text that fits the box, centred in it."""
        size = max_size
        while size > 4:
            f = font(kind, size * SS)
            if "\n" in s:
                bb = self.d.multiline_textbbox((0, 0), s, font=f, align="center", stroke_width=int(stroke * SS))
            else:
                bb = self.d.textbbox((0, 0), s, font=f, stroke_width=int(stroke * SS))
            if bb[2] - bb[0] <= (x1 - x0) * SS and bb[3] - bb[1] <= (y1 - y0) * SS:
                break
            size -= 0.5
        self.text((x0 + x1) / 2, (y0 + y1) / 2, s, size, fill, kind, "mm", stroke, stroke_fill, shadow=shadow)
        return size

    def paste(self, img: Image.Image, x, y):
        """A picture drawn at SS times the size (from `sub`)."""
        self.hi.alpha_composite(img, (int(x * SS), int(y * SS)))

    def sub(self, w, h) -> "Canvas":
        return Canvas(w, h)

    def blur_shadow(self, mask_shape, dx=1, dy=1, color=(0, 0, 0), alpha=90, radius=1.0):
        """A soft shadow under what's been drawn so far (drawn beneath)."""
        a = self.hi.getchannel("A")
        sh = Image.new("RGBA", self.hi.size, color + (0,))
        sh.putalpha(a.point(lambda v: alpha if v > 0 else 0).filter(ImageFilter.GaussianBlur(radius * SS)))
        base = Image.new("RGBA", self.hi.size, (0, 0, 0, 0))
        base.alpha_composite(sh, (int(dx * SS), int(dy * SS)))
        base.alpha_composite(self.hi)
        self.hi = base
        self.d = ImageDraw.Draw(self.hi)

    def dither(self, x0=0, y0=0, x1=None, y1=None):
        """Dither the area instead of rounding each colour: for big smooth
        gradients (glass, sky, metal)."""
        self.dither_areas.append((x0, y0, self.w if x1 is None else x1, self.h if y1 is None else y1))

    # -- composed parts ------------------------------------------------------

    def panel(self, x0, y0, x1, y1, base, radius=2.0, bevel=1.5, outline=INK, gloss=True):
        """A raised plate: outline, a top-left light edge, a bottom-right
        dark edge and a gentle top-to-bottom shading."""
        self.rect(x0, y0, x1, y1, fill=outline, radius=radius)
        self.gradient(x0 + 1, y0 + 1, x1 - 1, y1 - 1, [lighter(base, 0.55), lighter(base, 0.2)], radius=max(0, radius - 1))
        self.gradient(x0 + 1 + bevel, y0 + 1 + bevel, x1 - 1, y1 - 1, [darker(base, 0.25), darker(base, 0.55)], radius=max(0, radius - 1))
        self.gradient(x0 + 1 + bevel, y0 + 1 + bevel, x1 - 1 - bevel, y1 - 1 - bevel,
                      [lighter(base, 0.12), base, darker(base, 0.12)] if gloss else [base, base], radius=max(0, radius - 1.5))

    def inset(self, x0, y0, x1, y1, fill, radius=1.0, depth=1.0, rim=STEEL_D):
        """A sunken window (a display's glass)."""
        self.rect(x0, y0, x1, y1, fill=darker(rim, 0.5), radius=radius)
        self.rect(x0 + depth, y0 + depth, x1, y1, fill=lighter(rim, 0.5), radius=radius)
        self.rect(x0 + depth, y0 + depth, x1 - depth, y1 - depth, fill=fill, radius=max(0, radius - depth))

    def screw(self, cx, cy, r=1.6, angle=35):
        self.circle(cx, cy, r, fill=STEEL_D)
        self.circle(cx - 0.3, cy - 0.3, r - 0.5, fill=STEEL_L)
        a = math.radians(angle)
        dx, dy = math.cos(a) * (r - 0.5), math.sin(a) * (r - 0.5)
        self.line([(cx - dx, cy - dy), (cx + dx, cy + dy)], STEEL_D, 0.6, round_ends=False)

    def knot(self, x, y, out: bool, side: str | None = None, reach: float = 7.0):
        """A knot at (x, y), the point the game takes as the wire's end: an
        input is a brass socket, an output an orange lamp. `side` ("l", "r",
        "t", "b": the edge of the part the knot sits on) draws a stalk
        `reach` pixels long from the knot in towards the body."""
        cx, cy = x + 0.5, y + 0.5
        if side:
            dx, dy = {"l": (1, 0), "r": (-1, 0), "t": (0, 1), "b": (0, -1)}[side]
            end = (cx + dx * reach, cy + dy * reach)
            self.line([(cx, cy), end], INK, 3.0, round_ends=False)
            self.line([(cx, cy), end], STEEL, 1.4, round_ends=False)
        if out:
            self.circle(cx, cy, 3.9, fill=INK)
            self.radial(cx, cy, 3.2, KNOT_OUT_L, KNOT_OUT_D, -0.5, -0.5)
        else:
            self.circle(cx, cy, 3.9, fill=INK)
            self.circle(cx, cy, 3.2, fill=KNOT_IN_RIM)
            self.arc(cx, cy, 2.6, 180, 270, lighter(KNOT_IN_RIM, 0.6), 0.8)
            self.circle(cx, cy, 1.6, fill=KNOT_IN_HOLE)

    def knots(self, knots, sides=None):
        """Every knot of a table (Geometry.knots), stubs towards `sides`
        (one letter each, or None to guess from the frame's edges)."""
        for i, (x, y, out) in enumerate(knots):
            side = sides[i] if sides else self.guess_side(x, y)
            self.knot(x, y, out, side)

    def guess_side(self, x, y):
        """Which way the body lies from a knot near an edge: away from the
        nearest edge ("l" for a knot on the left edge, body to its right)."""
        d = {"l": x, "r": self.w - 1 - x, "t": y, "b": self.h - 1 - y}
        return min(d, key=d.get)

    def led_digit(self, x, y, w, h, ch, on=LED_RED, off=None, thick=2):
        """A seven-segment character on the pixel layer: 0-9, '-', 'E',
        ' ', '.', '+'."""
        segs = {
            "0": "abcdef", "1": "bc", "2": "abged", "3": "abgcd", "4": "fgbc", "5": "afgcd",
            "6": "afgedc", "7": "abc", "8": "abcdefg", "9": "abcdfg", "-": "g", "E": "afged",
            " ": "", "r": "eg", "o": "cdeg", "F": "afge", "H": "bcefg", "L": "def", "P": "abefg",
        }
        t = thick
        m = (h - 1) // 2
        boxes = {
            "a": (x + 1, y, x + w - 1, y + t), "g": (x + 1, y + m - t // 2, x + w - 1, y + m - t // 2 + t),
            "d": (x + 1, y + h - t, x + w - 1, y + h),
            "f": (x, y + 1, x + t, y + m), "b": (x + w - t, y + 1, x + w, y + m),
            "e": (x, y + m + 1, x + t, y + h - 1), "c": (x + w - t, y + m + 1, x + w, y + h - 1),
        }
        lit = segs.get(ch, "")
        for k, (a, b, c, d) in boxes.items():
            col = on if k in lit else off
            if col:
                self.ld.rectangle([a, b, c - 1, d - 1], fill=col + (255,))
        if ch == "+":
            cx = x + w // 2
            self.ld.rectangle([x + 1, y + m - t // 2, x + w - 2, y + m - t // 2 + t - 1], fill=on + (255,))
            self.ld.rectangle([cx - t // 2, y + 2, cx - t // 2 + t - 1, y + h - 3], fill=on + (255,))
        if ch == ".":
            self.ld.rectangle([x + w // 2 - t // 2, y + h - t, x + w // 2 - t // 2 + t - 1, y + h - 1], fill=on + (255,))

    # -- pixel layer -------------------------------------------------------------

    def pixel(self, x, y, c):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.lo.putpixel((int(x), int(y)), _c(c))

    def prect(self, x0, y0, x1, y1, c):
        """Pixel-exact rectangle x0..x1 × y0..y1 (exclusive ends)."""
        if x1 > x0 and y1 > y0:
            self.ld.rectangle([x0, y0, x1 - 1, y1 - 1], fill=_c(c))

    def pline(self, pts, c):
        self.ld.line(pts, fill=_c(c))

    def clear(self, x0, y0, x1, y1):
        """Make an area transparent again on both layers."""
        self.d.rectangle(self._box(x0, y0, x1, y1), fill=(0, 0, 0, 0))
        self.ld.rectangle([x0, y0, x1 - 1, y1 - 1], fill=(0, 0, 0, 0))

    # -- the end -----------------------------------------------------------------

    def finish(self, q: Quantizer) -> Image.Image:
        small = self.hi.resize((max(1, self.w), max(1, self.h)), Image.BOX)
        small.alpha_composite(self.lo)
        px = small.load()
        out = Image.new("RGBA", small.size, (0, 0, 0, 0))
        po = out.load()
        for y in range(self.h):
            for x in range(self.w):
                r, g, b, a = px[x, y]
                if a < 128:
                    continue
                c = (r, g, b)
                if any(x0 <= x < x1 and y0 <= y < y1 for x0, y0, x1, y1 in self.dither_areas):
                    t = (BAYER4[y % 4][x % 4] + 0.5) / 16 - 0.5
                    c = tuple(max(0, min(255, int(v + t * 40))) for v in c)
                po[x, y] = q.nearest(c) + (255,)
        return out


def _c(c):
    if c is None:
        return None
    if len(c) == 3:
        return tuple(c) + (255,)
    return tuple(c)


def _stops(stops):
    if isinstance(stops[0], tuple) and len(stops[0]) == 2 and isinstance(stops[0][1], tuple):
        return list(stops)
    n = len(stops)
    return [(i / max(1, n - 1), s) for i, s in enumerate(stops)]


def _at(st, t):
    for (t0, c0), (t1, c1) in zip(st, st[1:]):
        if t <= t1:
            return mix(c0, c1, 0 if t1 == t0 else (t - t0) / (t1 - t0))
    return st[-1][1]


# ---------------------------------------------------------------------------
# Sheets: a drawing function per sheet, registered with @sheet(id).

REGISTRY: dict = {}
# Sheets drawn from the pack's other pictures (the tray's icons, the
# puzzles' pictures): drawn after the rest.
LATE: set = set()


def sheet(*ids, late=False):
    """Registers `fn(ctx)` as the drawing of sheets `ids`; it returns the
    frames, one Canvas per frame (ctx.canvas(n) gives frame n's). A `late`
    sheet is drawn after the others, from their finished pictures
    (`pack_frame`)."""

    def wrap(fn):
        for i in ids:
            if i in REGISTRY:
                raise SystemExit(f"sheet {i} is drawn twice: {REGISTRY[i].__module__} and {fn.__module__}")
            REGISTRY[i] = fn
            if late:
                LATE.add(i)
        return fn

    return wrap


PACK = Path(__file__).resolve().parent.parent.parent / "crates/engine/opengfx"


class NotReady(Exception):
    """A late sheet can't be drawn yet (what it's made from is missing)."""


def parts_dir() -> Path:
    """The parts drawn whole by the engine with the pack (make.py --parts,
    from `cargo run -p engine --example render_parts -- EXPORT DIR`)."""
    d = os.environ.get("OPENGFX_PARTS")
    if not d:
        raise NotReady("needs --parts DIR (cargo run -p engine --example render_parts -- EXPORT DIR)")
    return Path(d)


def part_picture(tray_index: int) -> Image.Image:
    """Part `tray_index` (from 1, editor::tray::TAGS) as fresh from the
    tray, knots and all, drawn by the engine with the pack's pictures."""
    p = parts_dir() / f"{tray_index}.png"
    if not p.exists():
        raise NotReady(f"no {p}")
    return Image.open(p).convert("RGBA")


def pack_frame(sheet_id: int, n: int = 1) -> Image.Image | None:
    """Frame `n` of the pack's sheet `sheet_id` as drawn (RGBA), or None."""
    for pack in (pack_dir(LANG), PACK):
        p = pack / "gadgets" / str(sheet_id) / f"{n}.png"
        if p.exists():
            return Image.open(p).convert("RGBA")
    return None


def pack_dir(lang: str) -> Path:
    """The pack drawn in `lang`: the English one, or an overlay of the
    sheets whose words differ (crates/engine/opengfx-<lang>)."""
    return PACK if lang == "en" else PACK.with_name(f"opengfx-{lang}")


class SheetContext:
    def __init__(self, geo: Geometry, id: int):
        self.geo = geo
        self.id = id
        self.rects = geo.frames(id)

    def size(self, n: int):
        t, l, b, r = self.rects[n - 1]
        return r - l, b - t

    def canvas(self, n: int) -> Canvas:
        """A blank canvas for frame `n` (from 1)."""
        return Canvas(*self.size(n))

    def knots(self, table: int | None = None):
        return self.geo.knots(self.id if table is None else table)

    def blank(self, n: int) -> Canvas:
        """The game's 1×1 (or any unused) frame: left transparent."""
        return self.canvas(n)

    @property
    def count(self):
        return len(self.rects)
