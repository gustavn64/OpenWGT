"""Helpers for sheets/science.py: drawing in a bigger space and cutting
frames out of it, rotated stamps, and a small pixel font."""
from __future__ import annotations

from PIL import Image

from wgfx import SS, Canvas

# ---------------------------------------------------------------------------
# Cutting a frame out of a bigger drawing


def cut(big: Canvas, x: int, y: int, w: int, h: int) -> Canvas:
    """Frame `w`×`h` taken from `big` at (x, y): both layers, the dither
    areas moved along."""
    from PIL import ImageDraw

    c = Canvas(w, h)
    _paste_any(c.hi, big.hi, -x * SS, -y * SS)
    _paste_any(c.lo, big.lo, -x, -y)
    c.d = ImageDraw.Draw(c.hi)
    c.ld = ImageDraw.Draw(c.lo)
    c.dither_areas = [(a - x, b - y, cc - x, d - y) for a, b, cc, d in big.dither_areas]
    return c


def _paste_any(dst, src, ox, oy):
    """`src` composited onto `dst` with its top left at (ox, oy), clipped."""
    sx0, sy0 = max(0, -ox), max(0, -oy)
    sx1, sy1 = min(src.width, dst.width - ox), min(src.height, dst.height - oy)
    if sx1 > sx0 and sy1 > sy0:
        dst.alpha_composite(src.crop((sx0, sy0, sx1, sy1)), (ox + sx0, oy + sy0))


def stamp(dst: Canvas, src: Canvas, cx: float, cy: float, angle: float = 0.0, scale: float = 1.0):
    """`src`'s smooth layer drawn onto `dst` with its centre at (cx, cy),
    turned `angle` degrees anticlockwise (and scaled)."""
    im = src.hi
    if scale != 1.0:
        im = im.resize((max(1, int(im.width * scale)), max(1, int(im.height * scale))), Image.LANCZOS)
    if angle:
        im = im.rotate(angle, resample=Image.BICUBIC, expand=True)
    x = int(round(cx * SS - im.width / 2))
    y = int(round(cy * SS - im.height / 2))
    layer = Image.new("RGBA", dst.hi.size, (0, 0, 0, 0))
    _paste_any(layer, im, x, y)
    dst.hi.alpha_composite(layer)


def mask_of(c: Canvas, x0, y0, x1, y1, shape="ellipse"):
    """An "L" mask the size of the smooth layer (for Canvas.gradient)."""
    from PIL import ImageDraw

    m = Image.new("L", c.hi.size, 0)
    box = [x0 * SS, y0 * SS, x1 * SS - 1, y1 * SS - 1]
    getattr(ImageDraw.Draw(m), shape)(box, fill=255)
    return m


def poly_mask(c: Canvas, pts):
    from PIL import ImageDraw

    m = Image.new("L", c.hi.size, 0)
    ImageDraw.Draw(m).polygon([(x * SS, y * SS) for x, y in pts], fill=255)
    return m


def clip_hi(c: Canvas, mask):
    """Keep the smooth layer only inside `mask`."""
    from PIL import ImageChops, ImageDraw

    a = ImageChops.multiply(c.hi.getchannel("A"), mask)
    c.hi.putalpha(a)
    c.d = ImageDraw.Draw(c.hi)


# ---------------------------------------------------------------------------
# A 5×7 pixel font (capitals, digits), drawn on the pixel layer.

_G = {
    "A": ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
    "D": ["11110", "10001", "10001", "10001", "10001", "10001", "11110"],
    "E": ["1111", "1000", "1000", "1110", "1000", "1000", "1111"],
    "H": ["10001", "10001", "10001", "11111", "10001", "10001", "10001"],
    "I": ["111", "010", "010", "010", "010", "010", "111"],
    "L": ["1000", "1000", "1000", "1000", "1000", "1000", "1111"],
    "S": ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
    "T": ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
    "N": ["10001", "11001", "10101", "10011", "10001", "10001", "10001"],
    " ": ["00", "00", "00", "00", "00", "00", "00"],
    # the Swedish pack's (make.py --lang sv): KRONA, KLAVE (a narrow K, so
    # that they fit the windows HEADS and TAILS fill)
    "K": ["1001", "1010", "1100", "1000", "1100", "1010", "1001"],
    "R": ["11110", "10001", "10001", "11110", "10100", "10010", "10001"],
    "O": ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
    "V": ["10001", "10001", "10001", "10001", "10001", "01010", "00100"],
}


def ptext_width(s: str, gap=1) -> int:
    return sum(len(_G[ch][0]) for ch in s) + gap * (len(s) - 1)


def ptext(c: Canvas, x: int, y: int, s: str, color, gap=1, shadow=None):
    for dx, col in ([(1, shadow)] if shadow else []) + [(0, color)]:
        xx = x
        for ch in s:
            g = _G[ch]
            for j, row in enumerate(g):
                for i, b in enumerate(row):
                    if b == "1":
                        c.pixel(xx + i + dx, y + j + dx, col)
            xx += len(g[0]) + gap
