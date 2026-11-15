#!/usr/bin/env python3
"""Puts the original's and the port's shots of a script side by side,
with the pixels that differ in magenta, and says how many differ.

    compare.py VM_DIR PORT_DIR OUT_DIR
"""
import os
import sys

from PIL import Image, ImageChops


def main(a, b, out):
    os.makedirs(out, exist_ok=True)
    for name in sorted(os.listdir(a)):
        if not name.endswith(".png") or not os.path.exists(os.path.join(b, name)):
            continue
        x = Image.open(os.path.join(a, name)).convert("RGB")
        y = Image.open(os.path.join(b, name)).convert("RGB")
        w, h = min(x.width, y.width), min(x.height, y.height)
        x, y = x.crop((0, 0, w, h)), y.crop((0, 0, w, h))
        d = ImageChops.difference(x, y).convert("L").point(lambda v: 255 if v > 24 else 0)
        n = sum(1 for v in d.getdata() if v)
        mark = y.copy()
        mark.paste((255, 0, 255), mask=d)
        o = Image.new("RGB", (w * 3 + 8, h), (40, 40, 40))
        o.paste(x, (0, 0)); o.paste(y, (w + 4, 0)); o.paste(mark, (2 * w + 8, 0))
        o.save(os.path.join(out, name))
        print(f"{name}: {n} pixels differ ({100 * n / (w * h):.1f}%)")


if __name__ == "__main__":
    main(*sys.argv[1:4])
