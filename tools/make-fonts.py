#!/usr/bin/env python3
"""Renders the port's own TrueType stand-ins as Windows bitmap fonts,
hinted and without smoothing, as GDI drew them, written as bare `FNT` 3.0
files into crates/engine/fonts:

- "Widget Sans", for the gadgets' "Helvetica" (Arial under Windows):
  Liberation Sans, which has Arial's metrics, at the cell heights the game
  asks for (16, 16 bold and 24);
- "Widget Mono", for the debug console: Liberation Mono, which has Courier
  New's metrics, at 13 (as MS Sans Serif 8 pt) and 13 bold (struck
  twice).

Needs Pillow and Liberation 2.x (SIL OFL 1.1, whose Reserved Font Names
"Liberation", "Arimo" and "Cousine" a derivative can't carry).

Usage: tools/make-fonts.py [LIBERATION_DIR]
"""
import struct
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

OUT = Path(__file__).resolve().parent.parent / "crates/engine/fonts"
EM = 2048
FIRST, LAST, DEFAULT = 32, 255, ord("?")


class Face:
    def __init__(self, name, file, ascent, descent, mono, copyright):
        self.name, self.file, self.mono = name, file, mono
        # usWinAscent and usWinDescent, per EM.
        self.ascent, self.descent = ascent, descent
        self.copyright = copyright


SANS = Face("Widget Sans", "LiberationSans", 1854, 434, False,
            b"Liberation Sans: (c) 2012 Red Hat, (c) 2010 Google; SIL OFL 1.1")
MONO = Face("Widget Mono", "LiberationMono", 1705, 615, True,
            b"Liberation Mono: (c) 2012 Red Hat, (c) 2010 Google; SIL OFL 1.1")
FONTS = [(SANS, 16, False, "WSANS16"), (SANS, 16, True, "WSANS16B"),
         (SANS, 24, False, "WSANS24"), (MONO, 13, False, "WMONO13"),
         (MONO, 13, True, "WMONO13B")]


def chars() -> list[str]:
    out = []
    for c in range(FIRST, LAST + 1):
        try:
            out.append(bytes([c]).decode("cp1252"))
        except UnicodeDecodeError:
            out.append(None)
    return out


def render(face: Face, ttf: Path, height: int, bold: bool) -> bytes:
    # CreateFont with a positive height asks for the cell height: GDI
    # scales the em to fit the Windows ascent and descent into it.
    ppem = round(height * EM / (face.ascent + face.descent))
    font = ImageFont.truetype(str(ttf), ppem)
    ascent = round(face.ascent * ppem / EM)
    # Every cell as wide as an M, in a fixed-pitch face.
    cell = round(font.getlength("M", mode="1")) if face.mono else None
    glyphs = []
    for ch in chars():
        if ch is None:
            glyphs.append(None)
            continue
        w = cell or max(round(font.getlength(ch, mode="1")), 1 if ch != " " else 0)
        img = Image.new("1", (max(w, 1), height), 0)
        d = ImageDraw.Draw(img)
        d.fontmode = "1"
        d.text((0, ascent), ch, font=font, fill=1, anchor="ls")
        if face.mono and bold:
            # Its own bold is hardly bolder this small: struck twice, a
            # pixel apart, as GDI made a raster font bold.
            d.text((1, ascent), ch, font=font, fill=1, anchor="ls")
        glyphs.append((w, img))
    glyphs = [g if g is not None else glyphs[DEFAULT - FIRST] for g in glyphs]

    name = face.name.encode() + b"\0"
    count = LAST - FIRST + 2  # and the sentinel
    table_at = 148
    bits_at = table_at + count * 6
    bits = bytearray()
    table = bytearray()
    for w, img in glyphs:
        table += struct.pack("<HI", w, bits_at + len(bits))
        px = img.load()
        for strip in range((w + 7) // 8):
            for y in range(height):
                b = 0
                for bit in range(8):
                    x = strip * 8 + bit
                    if x < w and px[x, y]:
                        b |= 0x80 >> bit
                bits.append(b)
    table += struct.pack("<HI", 0, bits_at + len(bits))
    bits += b"\0" * height
    face_at = bits_at + len(bits)
    size = face_at + len(name)
    widths = [w for w, _ in glyphs]
    head = struct.pack(
        "<HI60sHHHHHHHBBBHBHHBHHBBBBHIIIIBIHHHI16s",
        0x300, size, face.copyright.ljust(60, b"\0"),
        0,  # raster
        round(ppem * 72 / 96), 96, 96,
        ascent, height - ppem, 0,
        0, 0, 0,
        700 if bold else 400,
        0,  # ANSI
        cell or 0, height,
        0x30 if face.mono else 0x21,  # FF_MODERN, fixed; FF_SWISS, variable
        cell or round(sum(widths) / len(widths)), max(widths),
        FIRST, LAST, DEFAULT - FIRST, ord(" ") - FIRST,
        0, 0, face_at, 0, bits_at, 0,
        1 if face.mono else 2,  # DFF_FIXED, DFF_PROPORTIONAL
        0, 0, 0, 0, b"\0" * 16,
    )
    assert len(head) == table_at, len(head)
    return head + table + bits + name


def main() -> None:
    src = Path(sys.argv[1] if len(sys.argv) > 1 else "/usr/share/fonts/liberation")
    OUT.mkdir(parents=True, exist_ok=True)
    for face, height, bold, file in FONTS:
        heavy = bold and not face.mono
        ttf = src / f"{face.file}-{'Bold' if heavy else 'Regular'}.ttf"
        (OUT / f"{file}.FNT").write_bytes(render(face, ttf, height, bold))
        print(OUT / f"{file}.FNT")


if __name__ == "__main__":
    main()
