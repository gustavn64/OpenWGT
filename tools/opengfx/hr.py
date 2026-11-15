"""Reads the game's HR picture sheets (formats::hr), for make.py --compare:
the CD's frames shown beside the new ones while drawing. Nothing read here
goes into the pack."""
import struct


def decode_sheet(d):
    n = struct.unpack_from("<H", d, 2)[0]
    out = []
    for i in range(n):
        t, l, b, r, off = struct.unpack_from("<hhhhI", d, 4 + i * 12)
        w, h = r - l, b - t
        out.append(((t, l, b, r), _decode(d, off, w, h) if w > 0 and h > 0 else None))
    return out


def _decode(d, at, w, h):
    px = [None] * (w * h)
    row = 0
    while at < len(d):
        b0 = d[at]
        if b0 & 0xF0 == 0xF0:
            at += 2
            x = 0
            while True:
                c = d[at]
                at += 1
                n = (c & 0x3F) + 1
                k = c & 0xC0
                if k == 0:
                    for j in range(n):
                        if x + j < w and row < h:
                            px[row * w + x + j] = d[at + j]
                    at += n
                    x += n
                elif k == 0x40:
                    x += n
                elif k == 0x80:
                    v = d[at]
                    at += 1
                    for j in range(n):
                        if x + j < w and row < h:
                            px[row * w + x + j] = v
                    x += n
                else:
                    at -= 1
                    row += 1
                    break
        elif b0 & 0xF0 == 0xE0:
            row += ((b0 << 8) | d[at + 1]) & 0xFFF
            at += 2
        else:
            break
    return px
