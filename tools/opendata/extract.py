#!/usr/bin/env python3
"""Reads the game's data from an export of the CD (`openwgt --export=DIR
CD.iso`) into OpenWDATA's editable sources (`src/`): the resources the
port reads besides pictures and sounds, as JSON and text.

This is the starting point, run once: the numbers (tables, layouts) are
the game's interface and stay; every text is then rewritten in the
project's own words before it is shared (README.md). Never run it over
sources that have been rewritten.

    extract.py EXPORT [SRC]
"""
import json
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import resfmt as R  # noqa: E402

HERE = Path(__file__).resolve().parent


def res(export, module, typ):
    d = Path(export) / module / str(typ)
    if not d.is_dir():
        return {}
    return {int(f.stem): f.read_bytes() for f in sorted(d.iterdir(), key=lambda f: int(f.stem))}


def dump(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=1, ensure_ascii=False) + "\n")


def tails(store, kind, key, tail):
    if tail:
        store.setdefault(kind, {})[str(key)] = tail.hex()


def main(export, src):
    src = Path(src)
    extra = {}
    # The palette: 256 RGBQUADs.
    pal = res(export, "WIDGET", 1003)[101]
    # Each colour's fourth byte (Windows' palette flags) after it, when not 0.
    dump(src / "palette.json", [[pal[i * 4 + 2], pal[i * 4 + 1], pal[i * 4]] + ([pal[i * 4 + 3]] if pal[i * 4 + 3] else [])
                                for i in range(256)])
    tails(extra, "palette", 101, pal[1024:])
    # The game's tables.
    tables = {}
    for typ, name, read in [(1010, "points", R.read_points), (1012, "rects", R.read_rects), (1014, "knots", R.read_knots)]:
        tables[name] = {}
        for i, d in res(export, "WWGADGET", typ).items():
            tables[name][str(i)], t = read(d)
            tails(extra, name, i, t)
    tables["point"] = {str(i): [R.i16(d, 0), R.i16(d, 2)] for i, d in res(export, "WWGADGET", 1011).items()}
    tables["rect"] = {str(i): list(R.struct.unpack_from("<hhhh", d, 0)) for i, d in res(export, "WWGADGET", 1013).items()}
    tables["info"] = {str(i): list(R.struct.unpack_from(f"<{len(d) // 2}H", d, 0)) for i, d in res(export, "WWGADGET", 1021).items()}
    for i, d in res(export, "WWGADGET", 1011).items():
        tails(extra, "point", i, d[4:])
    for i, d in res(export, "WWGADGET", 1013).items():
        tails(extra, "rect", i, d[8:])
    for i, d in res(export, "WWGADGET", 1021).items():
        tails(extra, "info", i, d[len(d) // 2 * 2:])
    dump(src / "tables.json", tables)
    # Texts (1020), one file each, lines ended by \n here.
    eols = {}
    for module in ("WWGADGET", "WIDGET"):
        for i, d in res(export, module, 1020).items():
            t = d.decode("cp1252")
            eol = "\r\n" if "\r\n" in t else "\r"
            eols[f"{module}/{i}"] = eol
            p = src / "texts" / module / f"{i}.txt"
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(t.replace("\r\n", "\n").replace("\r", "\n"), encoding="utf-8")
    extra["text_eol"] = eols
    # Combo boxes' strings.
    combos = {}
    for i, d in res(export, "WIDGET", 1022).items():
        combos[str(i)], t = R.read_combos(d)
        # Even an empty end: some lists have no end mark.
        extra.setdefault("combos", {})[str(i)] = t.hex()
    dump(src / "combos.json", combos)
    # String tables.
    strings = {}
    for b, d in res(export, "WIDGET", 6).items():
        s, t = R.read_strings(b, d)
        strings.update({str(k): v for k, v in s.items()})
        tails(extra, "strings", b, t)
    extra["string_blocks"] = sorted(res(export, "WIDGET", 6))
    dump(src / "strings.json", strings)
    # Menus, accelerators, dialogs.
    menus = {}
    for i, d in res(export, "WIDGET", 4).items():
        menus[str(i)], t = R.read_menu(d)
        tails(extra, "menus", i, t)
    dump(src / "menus.json", menus)
    accels = {}
    for i, d in res(export, "WIDGET", 9).items():
        accels[str(i)], t = R.read_accels(d)
        tails(extra, "accelerators", i, t)
    dump(src / "accelerators.json", accels)
    for i, d in res(export, "WIDGET", 5).items():
        dlg, t = R.read_dialog(d)
        dump(src / "dialogs" / f"{i}.json", dlg)
        tails(extra, "dialogs", i, t)
    dump(src / "layout.json", extra)
    print(f"sources written to {src}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else HERE / "src")
