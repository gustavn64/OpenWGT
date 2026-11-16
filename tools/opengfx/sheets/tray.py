"""The parts tray's icons (sheets 898 and 899, editor::tray::picture):
each part as fresh from the tray, drawn by the engine with OpenWGFX
(make.py --parts), made small."""
from PIL import Image, ImageFilter

from wgfx import INK, part_picture, sheet, tr


def icon(ctx, n, index):
    c = ctx.canvas(n)
    src = part_picture(index)
    bbox = src.getchannel("A").getbbox()
    if bbox is None:
        # The Text Label has no picture: a word.
        c.text_fit(1, 4, c.w - 1, c.h - 4, tr("Label"), INK, "liberation", 13)
        return c
    src = src.crop(bbox)
    scale = min((c.w - 1) / src.width, (c.h - 1) / src.height, 1.0)
    w, h = max(1, round(src.width * scale)), max(1, round(src.height * scale))
    small = src.resize((w, h), Image.LANCZOS) if scale < 1 else src
    if scale < 0.75:
        small = small.filter(ImageFilter.UnsharpMask(radius=1, percent=60, threshold=2))
    c.lo.alpha_composite(small, ((c.w - w) // 2, (c.h - h) // 2))
    return c


def tray_sheet(ctx, first):
    return [icon(ctx, n, first + n - 1) for n in range(1, ctx.count + 1)]


@sheet(898, late=True)
def tray_icons_1(ctx):
    return tray_sheet(ctx, 1)


@sheet(899, late=True)
def tray_icons_2(ctx):
    return tray_sheet(ctx, 31)

