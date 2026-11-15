#!/usr/bin/env python3
"""Makes OpenWSFX, the port's own sounds, into crates/engine/opensfx
(engine::opensfx): a wave per sound the game plays, from one Python
function each (tools/opensfx/sounds/*.py), with the kit in sfx.py.

    tools/opensfx/make.py [ID...] [--preview DIR [--compare EXPORT]]

Only the given sounds are made again when some are named. --preview
writes a picture per sound to DIR: its waveform and spectrogram, with
--compare the CD's sound's under it (from an export of the CD, `widget
--export=EXPORT CD.iso`), to match length, loudness and pitch by eye.
The CD's sounds are only looked at, never used. Needs numpy, scipy and
Pillow: `nix-shell -p 'python3.withPackages(p: [p.numpy p.scipy p.pillow])'`.
"""
from __future__ import annotations

import importlib
import pkgutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import numpy as np  # noqa: E402
import sfx  # noqa: E402

OUT = HERE.parent.parent / "crates/engine/opensfx/sounds"


def load_sounds():
    import sounds

    for m in pkgutil.iter_modules(sounds.__path__):
        importlib.import_module(f"sounds.{m.name}")


def picture(x, title, w=600):
    """A waveform over a spectrogram, as a Pillow image."""
    from PIL import Image, ImageDraw
    from scipy import signal

    h_wave, h_spec = 60, 110
    img = Image.new("RGB", (w, h_wave + h_spec + 14), (20, 20, 26))
    d = ImageDraw.Draw(img)
    dur = len(x) / sfx.RATE
    rms = float(np.sqrt(np.mean(x * x))) if len(x) else 0.0
    d.text((3, 1), f"{title}  {dur:.2f}s  rms {rms:.2f}  peak {np.abs(x).max() if len(x) else 0:.2f}", fill=(230, 230, 120))
    # 1 second of sound = w/3 pixels, so lengths compare across pictures.
    px = w / 3
    for i in range(min(w, int(dur * px) + 1)):
        seg = x[int(i / px * sfx.RATE): int((i + 1) / px * sfx.RATE)]
        if len(seg):
            lo, hi = seg.min(), seg.max()
            y0 = 14 + h_wave / 2
            d.line([(i, y0 - hi * h_wave / 2), (i, y0 - lo * h_wave / 2)], fill=(120, 200, 255))
    if len(x) > 256:
        f, t, s = signal.spectrogram(x, sfx.RATE, nperseg=256, noverlap=192)
        s = 10 * np.log10(s + 1e-10)
        s = np.clip((s - s.max() + 70) / 70, 0, 1)
        for j, tt in enumerate(t):
            xi = int(tt * px)
            if xi >= w:
                break
            for k in range(len(f)):
                v = s[k, j]
                y = 14 + h_wave + h_spec - 1 - int(k / len(f) * h_spec)
                c = (int(255 * v), int(255 * v * v), int(80 * (1 - v) * v * 4))
                d.rectangle([xi, y, xi + 2, y], fill=c)
    return img


def main(argv):
    args = [a for a in argv if not a.startswith("--")]
    preview = compare = None
    if "--preview" in argv:
        preview = Path(argv[argv.index("--preview") + 1])
        args.remove(str(preview))
    if "--compare" in argv:
        compare = Path(argv[argv.index("--compare") + 1])
        args.remove(str(compare))
    load_sounds()
    wanted = [int(a) for a in args] or sorted(sfx.REGISTRY)
    OUT.mkdir(parents=True, exist_ok=True)
    for sid in wanted:
        if sid not in sfx.REGISTRY:
            print(f"sound {sid}: nothing makes it")
            continue
        fn, name = sfx.REGISTRY[sid]
        sfx.seed(sid)
        x = np.asarray(fn(), dtype=float)
        path = OUT / f"{sid}.wav"
        (sfx.write_raw if sid in sfx.RAW else sfx.write)(path, x)
        made = sfx.read(path)
        if preview:
            from PIL import Image

            preview.mkdir(parents=True, exist_ok=True)
            pics = [picture(made, f"{sid} {name} (new)")]
            orig = compare / "WWSOUND/1006" / f"{sid}.wav" if compare else None
            if orig and orig.exists():
                pics.append(picture(sfx.read(orig), f"{sid} {name} (CD)"))
            img = Image.new("RGB", (pics[0].width, sum(p.height + 4 for p in pics)))
            y = 0
            for p in pics:
                img.paste(p, (0, y))
                y += p.height + 4
            img.save(preview / f"{sid}.png")
        print(f"sound {sid} {name}: {len(made) / sfx.RATE:.2f} s")
    print(f"{len(sfx.REGISTRY)} sounds made")


if __name__ == "__main__":
    main(sys.argv[1:])
