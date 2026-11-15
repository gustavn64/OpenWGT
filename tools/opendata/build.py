#!/usr/bin/env python3
"""Builds OpenWDATA (crates/engine/opendata, engine::opendata) from its
sources (`src/`): the game's resources in the game's own formats, in the
overrides' layout (`<MODULE>/<type>/<id>.bin`), so the engine reads them
as it reads the CD's.

    build.py [SRC] [--verify EXPORT]

The sample widgets and the puzzles are built from their recipes
(samples/, puzzles/) by the engine: `cargo run -p engine --example recipe
-- tools/opendata crates/engine/opendata`. The help (help/HELP.TXT) is
copied as it is.

--verify compares every built resource with the CD's (from an export):
before the texts are rewritten they must be byte for byte the same, which
checks the formats; after, only the texts may differ.
"""
import json
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import resfmt as R  # noqa: E402

HERE = Path(__file__).resolve().parent
OUT = HERE.parent.parent / "crates/engine/opendata"


def load(p):
    return json.loads(Path(p).read_text())


def build(src):
    src = Path(src)
    out = {}
    lay = load(src / "layout.json")

    def tail(kind, key):
        return bytes.fromhex(lay.get(kind, {}).get(str(key), ""))

    def put(module, typ, i, b):
        out[(module, typ, int(i))] = b

    pal = load(src / "palette.json")
    put("WIDGET", 1003, 101, b"".join(bytes([c[2], c[1], c[0], c[3] if len(c) > 3 else 0]) for c in pal) + tail("palette", 101))
    t = load(src / "tables.json")
    for typ, name, write in [(1010, "points", R.write_points), (1012, "rects", R.write_rects), (1014, "knots", R.write_knots)]:
        for i, v in t[name].items():
            put("WWGADGET", typ, i, write(v, tail(name, i)))
    for i, (v, h) in t["point"].items():
        put("WWGADGET", 1011, i, R.struct.pack("<hh", v, h) + tail("point", i))
    for i, r in t["rect"].items():
        put("WWGADGET", 1013, i, R.struct.pack("<hhhh", *r) + tail("rect", i))
    for i, v in t["info"].items():
        put("WWGADGET", 1021, i, R.struct.pack(f"<{len(v)}H", *v) + tail("info", i))
    eols = lay.get("text_eol", {})
    for module in ("WWGADGET", "WIDGET"):
        d = src / "texts" / module
        if d.is_dir():
            for f in d.glob("*.txt"):
                eol = eols.get(f"{module}/{f.stem}", "\r\n")
                text = f.read_text(encoding="utf-8").replace("\n", eol)
                put(module, 1020, f.stem, text.encode("cp1252"))
    for i, v in load(src / "combos.json").items():
        end = lay.get("combos", {}).get(str(i))
        put("WIDGET", 1022, i, R.write_combos(v, b"\0\0" if end is None else bytes.fromhex(end)))
    strings = {int(k): v for k, v in load(src / "strings.json").items()}
    blocks = set(lay.get("string_blocks", [])) | {k // 16 + 1 for k in strings}
    for b in sorted(blocks):
        put("WIDGET", 6, b, R.write_strings(b, strings, tail("strings", b)))
    for i, m in load(src / "menus.json").items():
        put("WIDGET", 4, i, R.write_menu(m, tail("menus", i)))
    for i, a in load(src / "accelerators.json").items():
        put("WIDGET", 9, i, R.write_accels(a, tail("accelerators", i)))
    for f in (src / "dialogs").glob("*.json"):
        put("WIDGET", 5, f.stem, R.write_dialog(load(f), tail("dialogs", f.stem)))
    return out


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    verify = argv[argv.index("--verify") + 1] if "--verify" in argv else None
    if verify:
        args.remove(verify)
    src = Path(args[0]) if args else HERE / "src"
    res = build(src)
    if verify:
        bad = same = 0
        for (m, t, i), b in sorted(res.items()):
            d = Path(verify) / m / str(t)
            orig = next((f for f in d.iterdir() if f.stem == str(i)), None) if d.is_dir() else None
            if orig is None:
                print(f"{m}/{t}/{i}: not on the CD")
                continue
            if orig.read_bytes() == b:
                same += 1
            else:
                bad += 1
                if t != 1020:
                    print(f"{m}/{t}/{i}: differs ({len(b)} bytes, CD {orig.stat().st_size})")
        print(f"{same} the same as the CD's, {bad} differ")
        return
    if OUT.exists():
        shutil.rmtree(OUT / "WIDGET", ignore_errors=True)
        shutil.rmtree(OUT / "WWGADGET", ignore_errors=True)
    for (m, t, i), b in res.items():
        p = OUT / m / str(t) / f"{i}.bin"
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(b)
    print(f"{len(res)} resources written to {OUT}")
    # The help (formats::helptext), as it is.
    help_src = src.parent / "help" / "HELP.TXT"
    if help_src.exists():
        shutil.copy(help_src, OUT / "HELP.TXT")
        print("help copied")


if __name__ == "__main__":
    main(sys.argv[1:])
