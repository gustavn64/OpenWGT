#!/usr/bin/env python3
"""Builds the open parts' data (engine::openparts) from its sources
(`src/`) into crates/engine/openparts, in the game's own formats and the
overrides' layout (`<MODULE>/<type>/<id>.bin`), as OpenWDATA's
(tools/opendata/build.py, whose resfmt.py writes them):

    WIDGET/5/2902               the Oscilloscope's settings dialog
                                (src/dialogs)
    WIDGET/6/161                string 2567 (the tray's name); the rest of
                                the block empty
    WWGADGET/1014/1903          the knots (src/tables.json)
    WWGADGET/1021/2902          the part's numbers (src/tables.json)
    WWGADGET/1020/2902, 4902    the Doc texts (src/texts)

and the Swedish and French ones (sv/src, fr/src over src/, as OpenWDATA
does it) into openparts-sv and openparts-fr, only what differs from the
English. draw.py draws the pictures beside them.

    build.py
"""
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(ROOT / "tools/opendata"))
import resfmt as R  # noqa: E402

OUT = ROOT / "crates/engine/openparts"
LANGS = ["sv", "fr"]


def load(p):
    return json.loads(Path(p).read_text())


def source(lang, name):
    """`name` under src/, or the language's own over it."""
    own = HERE / lang / "src" / name if lang != "en" else None
    return own if own and own.exists() else HERE / "src" / name


def build(lang):
    out = {}
    t = load(HERE / "src/tables.json")
    for i, v in t["knots"].items():
        out[("WWGADGET", 1014, int(i))] = R.write_knots(v)
    for i, v in t["info"].items():
        out[("WWGADGET", 1021, int(i))] = R.struct.pack(f"<{len(v)}H", *v)
    strings = load(HERE / "src/strings.json")
    if lang != "en" and (HERE / lang / "src/strings.json").exists():
        strings.update(load(HERE / lang / "src/strings.json"))
    strings = {int(k): v for k, v in strings.items()}
    for b in sorted({k // 16 + 1 for k in strings}):
        out[("WIDGET", 6, b)] = R.write_strings(b, strings)
    for f in sorted((HERE / "src/dialogs").glob("*.json")):
        out[("WIDGET", 5, int(f.stem))] = R.write_dialog(load(source(lang, f"dialogs/{f.name}")))
    for f in sorted((HERE / "src/texts/WWGADGET").glob("*.txt")):
        text = source(lang, f"texts/WWGADGET/{f.name}").read_text(encoding="utf-8")
        out[("WWGADGET", 1020, int(f.stem))] = text.replace("\n", "\r\n").encode("cp1252")
    return out


def write(dir, res, base=None):
    shutil.rmtree(dir / "WIDGET", ignore_errors=True)
    shutil.rmtree(dir / "WWGADGET", ignore_errors=True)
    n = 0
    for (m, t, i), b in sorted(res.items()):
        if base is not None and base.get((m, t, i)) == b:
            continue
        p = dir / m / str(t) / f"{i}.bin"
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(b)
        n += 1
    print(f"{n} resources written to {dir}")


def main():
    en = build("en")
    write(OUT, en)
    for lang in LANGS:
        write(OUT.with_name(f"openparts-{lang}"), build(lang), en)


if __name__ == "__main__":
    main()
