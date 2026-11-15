"""The synthesis kit for OpenWSFX, the port's own sounds
(crates/engine/opensfx, engine::opensfx). Every sound is made here from
oscillators, noise, filters and envelopes: nothing is recorded or taken
from the CD.

Sounds are numpy float arrays at RATE (the game's 11025 Hz), about -1..1;
`write` makes them 8-bit mono waves as the game's own are.
"""
from __future__ import annotations

import struct
from pathlib import Path

import numpy as np
from scipy import signal

RATE = 11025
RNG = np.random.default_rng(1995)


def seed(n: int):
    """Makes the noise of what follows repeatable (call once per sound)."""
    global RNG
    RNG = np.random.default_rng(n)


# ---------------------------------------------------------------------------
# Time and envelopes


def n_of(dur: float) -> int:
    return max(1, int(round(dur * RATE)))


def t_of(dur: float) -> np.ndarray:
    return np.arange(n_of(dur)) / RATE


def silence(dur: float) -> np.ndarray:
    return np.zeros(n_of(dur))


def env_adsr(dur, a=0.005, d=0.05, s=0.6, r=0.05):
    """Attack, decay to sustain level `s`, release at the end."""
    n = n_of(dur)
    e = np.full(n, s, dtype=float)
    na, nd, nr = n_of(a), n_of(d), n_of(r)
    na = min(na, n)
    e[:na] = np.linspace(0, 1, na, endpoint=False)
    k = min(nd, n - na)
    e[na:na + k] = np.linspace(1, s, k, endpoint=False)
    nr = min(nr, n)
    e[n - nr:] *= np.linspace(1, 0, nr)
    return e


def env_exp(dur, decay=0.1, attack=0.002):
    """A quick attack, then e^(-t/decay)."""
    t = t_of(dur)
    e = np.exp(-t / decay)
    na = n_of(attack)
    e[:na] *= np.linspace(0, 1, min(na, len(e)))
    return e


def env_points(dur, pts):
    """A piecewise-linear envelope through (time, level) points."""
    t = t_of(dur)
    ts, vs = zip(*pts)
    return np.interp(t, ts, vs)


def curve(dur, pts, log=False):
    """A parameter (a frequency, say) moving through (time, value)
    points; geometric between points with log=True (for pitch)."""
    t = t_of(dur)
    ts, vs = zip(*pts)
    if log:
        return np.exp(np.interp(t, ts, np.log(vs)))
    return np.interp(t, ts, vs)


# ---------------------------------------------------------------------------
# Oscillators: `f` a frequency in Hz or an array of them (a glide)


def phase(f, dur):
    n = n_of(dur)
    f = np.broadcast_to(np.asarray(f, dtype=float), (n,))
    return 2 * np.pi * np.cumsum(f) / RATE


def sine(f, dur, ph=0.0):
    return np.sin(phase(f, dur) + ph)


def saw(f, dur):
    """Band-limited enough at this rate: a sum of harmonics under Nyquist."""
    return _additive(f, dur, lambda k: 1.0 / k, range(1, 64))


def square(f, dur):
    return _additive(f, dur, lambda k: 1.0 / k, range(1, 64, 2))


def triangle(f, dur):
    return _additive(f, dur, lambda k: ((-1) ** ((k - 1) // 2)) / (k * k), range(1, 64, 2))


def _additive(f, dur, amp, ks):
    p = phase(f, dur)
    fa = np.broadcast_to(np.asarray(f, dtype=float), p.shape)
    out = np.zeros_like(p)
    for k in ks:
        alive = fa * k < RATE / 2 - 200
        if not alive.any():
            break
        out += np.where(alive, amp(k) * np.sin(k * p), 0.0)
    return out / max(1e-9, np.abs(out).max())


def pulse_train(f, dur, width=0.3):
    """A glottis-like pulse train (for voices: through `formants`)."""
    p = (phase(f, dur) / (2 * np.pi)) % 1.0
    g = np.where(p < width, np.sin(np.pi * p / width) ** 2, 0.0)
    return np.diff(g, prepend=0.0) * 8


def fm(carrier, ratio, index, dur, index_env=None):
    """Two-operator FM: bells, chimes, metallic things. `index_env`
    scales the index over time."""
    p = phase(carrier, dur)
    ie = index if index_env is None else index * index_env
    return np.sin(p + ie * np.sin(ratio * p))


# ---------------------------------------------------------------------------
# Noise and filters


def noise(dur, color="white"):
    n = n_of(dur)
    w = RNG.standard_normal(n)
    if color == "white":
        return w / 3
    if color == "pink":
        b, a = [0.049922035, -0.095993537, 0.050612699, -0.004408786], [1, -2.494956002, 2.017265875, -0.522189400]
        return signal.lfilter(b, a, w) / 2
    if color == "brown":
        x = np.cumsum(w)
        x = signal.lfilter([1, -1], [1, -0.995], x)
        return x / max(1e-9, np.abs(x).max())
    raise ValueError(color)


def lowpass(x, f, q=0.707):
    return _biquad(x, "low", f, q)


def highpass(x, f, q=0.707):
    return _biquad(x, "high", f, q)


def bandpass(x, f, q=4.0):
    return _biquad(x, "band", f, q)


def _biquad(x, kind, f, q):
    """An RBJ biquad; `f` may be an array (a sweep), done in blocks."""
    fa = np.broadcast_to(np.asarray(f, dtype=float), x.shape)
    if np.allclose(fa, fa[0]):
        b, a = _coef(kind, float(fa[0]), q)
        return signal.lfilter(b, a, x)
    out = np.empty_like(x)
    zi = np.zeros(2)
    for i in range(0, len(x), 64):
        b, a = _coef(kind, float(fa[i]), q)
        out[i:i + 64], zi = signal.lfilter(b, a, x[i:i + 64], zi=zi)
    return out


def _coef(kind, f, q):
    f = min(max(f, 20.0), RATE / 2 - 50)
    w = 2 * np.pi * f / RATE
    al = np.sin(w) / (2 * q)
    c = np.cos(w)
    if kind == "low":
        b = [(1 - c) / 2, 1 - c, (1 - c) / 2]
    elif kind == "high":
        b = [(1 + c) / 2, -(1 + c), (1 + c) / 2]
    else:
        b = [al, 0, -al]
    a = [1 + al, -2 * c, 1 - al]
    return np.array(b) / a[0], np.array(a) / a[0]


# Vowel formants (Hz, bandwidth-ish Q) for `formants`.
VOWELS = {
    "a": [(800, 6), (1150, 8), (2900, 10)],
    "e": [(400, 6), (1600, 10), (2700, 12)],
    "i": [(300, 6), (2300, 12), (3000, 12)],
    "o": [(450, 6), (800, 8), (2830, 12)],
    "u": [(325, 6), (700, 8), (2530, 12)],
    "w": [(300, 5), (600, 6), (2400, 10)],
    "m": [(280, 4), (1000, 6), (2200, 8)],
    "ow": [(550, 5), (850, 7), (2500, 10)],
}


def formants(src, vowel, gains=(1.0, 0.5, 0.25)):
    """`src` (a pulse train, noise) through a vowel's formants; `vowel` a
    key of VOWELS or a list of (f, q), each f a number or an array."""
    fs = VOWELS[vowel] if isinstance(vowel, str) else vowel
    out = np.zeros_like(src)
    for (f, q), g in zip(fs, gains):
        out += g * bandpass(src, f, q)
    return out


def morph_vowel(a, b, dur, t0=0.0, t1=None):
    """Formant tracks gliding from vowel `a` to `b` between t0 and t1."""
    t1 = dur if t1 is None else t1
    fa, fb = VOWELS[a], VOWELS[b]
    return [(curve(dur, [(0, x), (t0, x), (t1, y), (dur, y)]), (q + r) / 2) for (x, q), (y, r) in zip(fa, fb)]


# ---------------------------------------------------------------------------
# Putting together


def mix(*parts, at=None):
    """Sums sounds, each starting at its time in `at` (seconds)."""
    at = at or [0.0] * len(parts)
    n = max(n_of(t) + len(p) for p, t in zip(parts, at))
    out = np.zeros(n)
    for p, t in zip(parts, at):
        s = n_of(t) if t else 0
        out[s:s + len(p)] += p
    return out


def concat(*parts):
    return np.concatenate(parts)


def fade(x, fin=0.002, fout=0.01):
    x = x.copy()
    a, b = min(len(x), n_of(fin)), min(len(x), n_of(fout))
    x[:a] *= np.linspace(0, 1, a)
    x[len(x) - b:] *= np.linspace(1, 0, b)
    return x


def normalize(x, peak=0.9):
    m = np.abs(x).max()
    return x if m == 0 else x * (peak / m)


def soft_clip(x, drive=1.0):
    return np.tanh(x * drive) / np.tanh(drive)


def echo(x, delay=0.08, gain=0.35, n=3):
    out = np.concatenate([x, np.zeros(n_of(delay) * n)])
    for k in range(1, n + 1):
        s = n_of(delay) * k
        out[s:s + len(x)] += x * gain ** k
    return out


def write(path: Path, x, peak=0.9):
    """An 8-bit mono 11025 Hz wave, as the game's sounds are."""
    x = normalize(fade(np.asarray(x, dtype=float)), peak) if np.abs(x).max() > 0 else x
    b = np.clip(np.round(x * 127 + 128), 0, 255).astype(np.uint8).tobytes()
    hdr = b"RIFF" + struct.pack("<I", 36 + len(b)) + b"WAVEfmt " + struct.pack("<IHHIIHH", 16, 1, 1, RATE, RATE, 1, 8)
    path.write_bytes(hdr + b"data" + struct.pack("<I", len(b)) + b)


def write_raw(path: Path, x):
    """As `write`, without fading or normalizing (loops, exact levels)."""
    b = np.clip(np.round(np.asarray(x) * 127 + 128), 0, 255).astype(np.uint8).tobytes()
    hdr = b"RIFF" + struct.pack("<I", 36 + len(b)) + b"WAVEfmt " + struct.pack("<IHHIIHH", 16, 1, 1, RATE, RATE, 1, 8)
    path.write_bytes(hdr + b"data" + struct.pack("<I", len(b)) + b)


def read(path: Path) -> np.ndarray:
    """A wave's samples as floats (the CD's sounds, for --compare: every
    byte past the 44-byte header, as the game plays them)."""
    d = Path(path).read_bytes()
    return (np.frombuffer(d[44:], dtype=np.uint8).astype(float) - 128) / 127


# ---------------------------------------------------------------------------
# Sounds: a function per sound id, registered with @sound(id, name).

REGISTRY: dict = {}
# Ids written as they are (loops), not faded or normalized.
RAW: set = set()


def sound(*ids, name="", raw=False):
    def wrap(fn):
        for i in ids:
            if i in REGISTRY:
                raise SystemExit(f"sound {i} is made twice: {REGISTRY[i][0].__module__} and {fn.__module__}")
            REGISTRY[i] = (fn, name)
            if raw:
                RAW.add(i)
        return fn

    return wrap
