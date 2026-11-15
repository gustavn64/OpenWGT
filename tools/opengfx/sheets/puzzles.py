"""The puzzles' pictures of a part with its IN and OUT arrows (sheets
2001 to 4037: the puzzle's tested part, docs/puzzles.md): the part as
the engine draws it with OpenWGFX (make.py --parts), the pack's own
arrows (sheet 504) at its knots, and wires to the OUT arrows."""
from PIL import Image, ImageDraw

from wgfx import NotReady, pack_frame, part_picture, parts_dir, sheet

WIRE = (205, 30, 20)

# picture: (tray index of the part, where it goes, the inputs' arrows, the
# outputs' arrows). Where: ("box", x0, y0, x1, y1), the part made to fit
# and centred in it; or ("at", x, y), the part as it is with its top left
# there (cut by the picture's edge). An input's arrow points at the knot
# from the left ("r"), from above ("d") or from below ("u"); "auto" picks
# by the knot's side. An output's arrow is at (x, y), the wire running to it.
PICTURES = {
    2001: (5, ("box", 5, 32, 134, 88), ["d"], [(126, 20)]),
    2006: (6, ("box", 35, 39, 103, 86), ["r"], [(126, 20)]),
    2007: (31, ("box", 9, 9, 87, 120), [], [(126, 20)]),
    2027: (48, ("box", 28, 34, 128, 91), ["r"], []),
    2028: (47, ("at", 27, 52), ["r"], []),
    2043: (52, ("box", 40, 30, 118, 103), ["r"], []),
    2044: (50, ("box", 38, 30, 138, 113), ["auto", "auto"], []),
    4000: (1, ("box", 20, 23, 134, 100), ["r"], [(126, 20)]),
    4001: (2, ("box", 53, 10, 83, 118), ["r"], [(126, 20)]),
    4002: (3, ("box", 25, 40, 130, 78), ["r"], [(126, 20)]),
    4014: (14, ("at", 42, 40), ["r"], []),
    4015: (15, ("box", 13, 45, 98, 88), [], [(126, 20)]),
    4035: (51, ("box", 22, 21, 128, 97), ["r"], []),
    4036: (16, ("box", 49, 28, 91, 88), ["u"], []),
    4037: (18, ("box", 28, 26, 133, 99), ["r", "r", "r"], []),
}

# Sheet 504's frames: IN pointing right, IN pointing down, OUT; each
# placed by its rectangle's offset from the point it marks.
IN_RIGHT, IN_DOWN, OUT = 1, 2, 3


def knots_of(index):
    lines = (parts_dir() / f"{index}.txt").read_text().split("\n")
    return [(int(a), int(b), c == "1") for a, b, c in (l.split() for l in lines if l.strip())]


def arrow(img, n, x, y, offsets, flip=False):
    pic = pack_frame(504, n)
    if pic is None:
        raise NotReady("sheet 504 (the arrows) isn't drawn yet")
    t, l, b, r = offsets[n - 1]
    if flip:
        pic = pic.transpose(Image.FLIP_TOP_BOTTOM)
        t, b = -b, -t
    img.alpha_composite(pic, (x + l, y + t))


def picture(ctx, spec):
    index, where, ins, outs = spec
    c = ctx.canvas(1)
    part = part_picture(index)
    knots = knots_of(index)
    if where[0] == "box":
        _, x0, y0, x1, y1 = where
        s = min((x1 - x0) / part.width, (y1 - y0) / part.height, 1.0)
        w, h = max(1, round(part.width * s)), max(1, round(part.height * s))
        if s < 1:
            part = part.resize((w, h), Image.LANCZOS)
        px, py = x0 + (x1 - x0 - w) // 2, y0 + (y1 - y0 - h) // 2
    else:
        s, (px, py) = 1.0, where[1:]
    img = Image.new("RGBA", (c.w, c.h), (0, 0, 0, 0))
    img.paste(part, (px, py), part)
    at = [(round(px + x * s), round(py + y * s), out) for x, y, out in knots]
    offsets = ctx.geo.frames(504)
    d = ImageDraw.Draw(img)
    for (kx, ky), (ax, ay) in zip([(x, y) for x, y, o in at if o], outs):
        # A wire across, then up or down to the arrow, then across.
        mid = kx + max(4, (ax - kx) // 2) if ax > kx + 8 else kx + 4
        d.line([(kx + 4, ky), (mid, ky), (mid, ay), (ax, ay)], fill=WIRE + (255,))
        arrow(img, OUT, ax, ay, offsets)
    inputs = [(x, y) for x, y, o in at if not o]
    for (kx, ky), way in zip(inputs, ins):
        if way == "auto":
            way = "d" if ky - py < 8 * s else "r"
        if way == "r":
            arrow(img, IN_RIGHT, kx - 5, ky, offsets)
        elif way == "d":
            arrow(img, IN_DOWN, kx, ky - 5, offsets)
        else:
            arrow(img, IN_DOWN, kx, ky + 5, offsets, flip=True)
    c.lo.alpha_composite(img)
    return [c]


for _id, _spec in PICTURES.items():
    sheet(_id, late=True)(lambda ctx, _spec=_spec: picture(ctx, _spec))
