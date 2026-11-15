"""The resource formats OpenWDATA's sources compile to, each with a reader
(for extract.py) and a writer (for build.py): the game's own formats,
so the engine reads the pack exactly as it reads the CD.

Windows resources (WIDGET.EXE): RT_MENU, RT_DIALOG (DLGTEMPLATE),
RT_STRING, RT_ACCELERATOR, and the palette (type 1003, RGBQUADs).
The game's own (WWGADGET.DLL, little-endian, Mac-shaped: points are
(v, h), rects (top, left, bottom, right)): 1010 point lists, 1011 a point,
1012 rect lists, 1013 a rect, 1014 knot tables, 1020 texts, 1021 a
gadget's numbers, 1022 combo-box strings.
"""
from __future__ import annotations

import struct

# ---------------------------------------------------------------------------
# Little helpers


def u16(d, at):
    return struct.unpack_from("<H", d, at)[0]


def i16(d, at):
    return struct.unpack_from("<h", d, at)[0]


def u32(d, at):
    return struct.unpack_from("<I", d, at)[0]


def utf16z(d, at):
    """A NUL-ended UTF-16 string at `at`: (text, the offset after it)."""
    out = []
    while True:
        c = u16(d, at)
        at += 2
        if c == 0:
            return "".join(map(chr, out)), at
        out.append(c)


def put_utf16z(s: str) -> bytes:
    return s.encode("utf-16-le") + b"\0\0"


# ---------------------------------------------------------------------------
# The game's tables


def read_points(d):
    n = u16(d, 0)
    return [[i16(d, 2 + i * 4), i16(d, 4 + i * 4)] for i in range(n)], d[2 + n * 4:]


def write_points(pts, tail=b""):
    return struct.pack("<H", len(pts)) + b"".join(struct.pack("<hh", v, h) for v, h in pts) + tail


def read_rects(d):
    n = u16(d, 0)
    return [list(struct.unpack_from("<hhhh", d, 2 + i * 8)) for i in range(n)], d[2 + n * 8:]


def write_rects(rs, tail=b""):
    return struct.pack("<H", len(rs)) + b"".join(struct.pack("<hhhh", *r) for r in rs) + tail


def read_knots(d):
    n = u16(d, 0)
    return [list(struct.unpack_from("<hhH", d, 2 + i * 6)) for i in range(n)], d[2 + n * 6:]


def write_knots(ks, tail=b""):
    return struct.pack("<H", len(ks)) + b"".join(struct.pack("<hhH", *k) for k in ks) + tail


def read_combos(d):
    """1022: entries of (control id, 0x0403, length with the NUL, text);
    a zero id ends them. Returns (entries, the bytes after the end mark)."""
    out, at = [], 0
    while at + 2 <= len(d):
        cid = u16(d, at)
        if cid == 0:
            return out, d[at:]
        tag, n = u16(d, at + 2), u32(d, at + 4)
        text = d[at + 8:at + 8 + n - 1].decode("cp1252")
        out.append([cid, text] if tag == 0x0403 else [cid, text, tag])
        at += 8 + n
    return out, d[at:]


def write_combos(entries, tail=b"\0\0"):
    out = b""
    for e in entries:
        cid, text = e[0], e[1]
        tag = e[2] if len(e) > 2 else 0x0403
        b = text.encode("cp1252") + b"\0"
        out += struct.pack("<HHI", cid, tag, len(b)) + b
    return out + tail


# ---------------------------------------------------------------------------
# String tables: blocks of 16 counted UTF-16 strings, block n holding ids
# (n − 1) × 16 to n × 16 − 1.


def read_strings(block_id, d):
    out, at = {}, 0
    for i in range(16):
        n = u16(d, at)
        at += 2
        if n:
            out[(block_id - 1) * 16 + i] = "".join(map(chr, struct.unpack_from(f"<{n}H", d, at)))
        at += 2 * n
    return out, d[at:]


def write_strings(block_id, strings, tail=b""):
    out = b""
    for i in range(16):
        s = strings.get((block_id - 1) * 16 + i, "")
        out += struct.pack("<H", len(s)) + s.encode("utf-16-le")
    return out + tail


# ---------------------------------------------------------------------------
# Accelerators: 8-byte entries (flags, key, command, padding); 0x80 marks
# the last.


def read_accels(d):
    out = []
    for at in range(0, len(d) - 7, 8):
        flags, key, cmd, pad = struct.unpack_from("<HHHH", d, at)
        out.append({"flags": flags & ~0x80, "key": key, "command": cmd} | ({"pad": pad} if pad else {}))
        if flags & 0x80:
            return out, d[at + 8:]
    return out, b""


def write_accels(acc, tail=b""):
    out = b""
    for i, a in enumerate(acc):
        flags = a["flags"] | (0x80 if i == len(acc) - 1 else 0)
        out += struct.pack("<HHHH", flags, a["key"], a["command"], a.get("pad", 0))
    return out + tail


# ---------------------------------------------------------------------------
# Menus: a 4-byte header, then MENUITEMTEMPLATEs (MF_POPUP 0x10, MF_END 0x80)


def read_menu(d):
    header = list(struct.unpack_from("<HH", d, 0))

    def items(at):
        out = []
        while True:
            flags = u16(d, at)
            at += 2
            if flags & 0x10:
                text, at = utf16z(d, at)
                sub, at = items(at)
                out.append({"popup": text, "flags": flags & ~0x90, "items": sub})
            else:
                cid = u16(d, at)
                text, at = utf16z(d, at + 2)
                if cid == 0 and text == "" and flags & ~0x80 == 0:
                    out.append("-")
                else:
                    out.append({"id": cid, "text": text, "flags": flags & ~0x80})
            if flags & 0x80:
                return out, at

    tree, at = items(4)
    return {"header": header, "items": tree}, d[at:]


def write_menu(m, tail=b""):
    def items(lst):
        out = b""
        for i, it in enumerate(lst):
            end = 0x80 if i == len(lst) - 1 else 0
            if it == "-":
                out += struct.pack("<HH", end, 0) + b"\0\0"
            elif "popup" in it:
                out += struct.pack("<H", it["flags"] | 0x10 | end) + put_utf16z(it["popup"]) + items(it["items"])
            else:
                out += struct.pack("<HH", it["flags"] | end, it["id"]) + put_utf16z(it["text"])
        return out

    return struct.pack("<HH", *m["header"]) + items(m["items"]) + tail


# ---------------------------------------------------------------------------
# Dialogs: DLGTEMPLATE, then DLGITEMTEMPLATEs, each DWORD-aligned.


def _field(d, at):
    c = u16(d, at)
    if c == 0:
        return None, at + 2
    if c == 0xFFFF:
        return {"ordinal": u16(d, at + 2)}, at + 4
    s, at = utf16z(d, at)
    return s, at


def _put_field(f):
    if f is None:
        return b"\0\0"
    if isinstance(f, dict):
        return struct.pack("<HH", 0xFFFF, f["ordinal"])
    return put_utf16z(f)


CLASSES = {0x80: "button", 0x81: "edit", 0x82: "static", 0x83: "listbox", 0x84: "scrollbar", 0x85: "combobox"}
CLASS_IDS = {v: k for k, v in CLASSES.items()}


def read_dialog(d):
    style, ex, n, x, y, cx, cy = struct.unpack_from("<IIHhhhh", d, 0)
    at = 18
    menu, at = _field(d, at)
    cls, at = _field(d, at)
    title, at = utf16z(d, at)
    font = None
    if style & 0x40:
        size = u16(d, at)
        face, at = utf16z(d, at + 2)
        font = [size, face]
    dlg = {"style": f"0x{style:08x}", "ex_style": f"0x{ex:08x}", "rect": [x, y, cx, cy], "menu": menu,
           "class": cls, "title": title, "font": font, "items": []}
    for _ in range(n):
        at = (at + 3) & ~3
        st, ix, x, y, cx, cy, cid = struct.unpack_from("<IIhhhhH", d, at)
        at += 18
        icls, at = _field(d, at)
        text, at = _field(d, at)
        extra = u16(d, at)
        at += 2
        data = d[at:at + extra].hex() if extra else None
        at += extra
        if isinstance(icls, dict) and icls["ordinal"] in CLASSES:
            icls = CLASSES[icls["ordinal"]]
        item = {"id": cid, "class": icls, "text": text, "rect": [x, y, cx, cy], "style": f"0x{st:08x}"}
        if ix:
            item["ex_style"] = f"0x{ix:08x}"
        if data:
            item["data"] = data
        dlg["items"].append(item)
    return dlg, d[at:]


def write_dialog(dlg, tail=b""):
    style = int(dlg["style"], 16)
    out = struct.pack("<IIHhhhh", style, int(dlg["ex_style"], 16), len(dlg["items"]), *dlg["rect"])
    out += _put_field(dlg["menu"]) + _put_field(dlg["class"]) + put_utf16z(dlg["title"])
    if style & 0x40:
        out += struct.pack("<H", dlg["font"][0]) + put_utf16z(dlg["font"][1])
    for it in dlg["items"]:
        out += b"\0" * (-len(out) % 4)
        x, y, cx, cy = it["rect"]
        out += struct.pack("<IIhhhhH", int(it["style"], 16), int(it.get("ex_style", "0x0"), 16), x, y, cx, cy, it["id"])
        cls = it["class"]
        out += _put_field({"ordinal": CLASS_IDS[cls]} if isinstance(cls, str) and cls in CLASS_IDS else cls)
        out += _put_field(it["text"])
        data = bytes.fromhex(it["data"]) if it.get("data") else b""
        out += struct.pack("<H", len(data)) + data
    return out + tail
