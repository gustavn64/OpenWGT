"""Helpers for sounds/voices.py (the Animal Sound and Misc Sound parts):
voices from a pulse train through moving formants, small rooms, bells,
claps. All synthesis; nothing here reads a recorded sound."""
from __future__ import annotations

import numpy as np
from scipy import signal

import sfx
from sfx import RATE, bandpass, curve, formants, lowpass, n_of, noise, phase, pulse_train


def rng():
    """The kit's noise generator as `sfx.seed` last set it."""
    return sfx.RNG


def drift(dur, amount, smooth=25.0):
    """1 + a smooth random wander of about `amount` (relative), changing
    at up to `smooth` Hz: jitter for pitches and levels."""
    n = n_of(dur)
    w = rng().standard_normal(n + 512)
    b, a = signal.butter(2, min(smooth, RATE / 2 - 1) / (RATE / 2))
    w = signal.lfilter(b, a, w)[512:]
    s = w.std()
    return 1 + amount * (w / s if s > 0 else w)


def vibrato(dur, rate, depth, ph=0.0):
    """1 + depth * sin: a pitch multiplier."""
    return 1 + depth * np.sin(phase(rate, dur) + ph)


def tracks(dur, pts):
    """Formant tracks through (time, vowel) points; a vowel is a key of
    sfx.VOWELS or a list of (f, q). Gives a list of (f array, q)."""
    vs = [sfx.VOWELS[v] if isinstance(v, str) else v for _, v in pts]
    ts = [t for t, _ in pts]
    out = []
    for k in range(min(len(v) for v in vs)):
        f = curve(dur, list(zip(ts, [v[k][0] for v in vs])), log=True)
        q = float(np.mean([v[k][1] for v in vs]))
        out.append((f, q))
    return out


def voice(f0, dur, pts, width=0.35, breath=0.08, gains=(1.0, 0.5, 0.25), rough=0.0):
    """A voiced sound: a pulse train at pitch `f0` (a number or a curve)
    with some breath noise, through formants moving along `pts`.
    `rough` adds period-doubling (a growl)."""
    n = n_of(dur)
    f = np.broadcast_to(np.asarray(f0, dtype=float), (n,))
    v = pulse_train(f, dur, width)
    v = v / max(1e-9, np.abs(v).max())
    if rough:
        v = v * (1 + rough * np.sin(phase(f / 2, dur)))
    src = v + breath * noise(dur) * 3
    return formants(src, tracks(dur, pts), gains)


def tame(x, f=3600.0):
    """Keeps the top end down for the 8-bit, 11025 Hz output."""
    return lowpass(lowpass(x, f), f)


def room(x, size=1.0, decay=0.5, wet=0.3):
    """A small Schroeder reverb: four combs and two all-passes. `decay`
    is the comb feedback (0..0.9), the tail grows with it."""
    combs = [0.0297, 0.0371, 0.0411, 0.0437]
    tail = int(RATE * 0.05 * size / max(1e-3, 1 - decay) + RATE * 0.05)
    xin = np.concatenate([x, np.zeros(tail)])
    acc = np.zeros_like(xin)
    for d in combs:
        k = max(1, int(d * size * RATE))
        a = np.zeros(k + 1)
        a[0], a[k] = 1, -decay
        b = np.zeros(k + 1)
        b[k] = 1
        acc += signal.lfilter(b, a, xin)
    acc /= len(combs)
    for d, g in ((0.005, 0.7), (0.0017, 0.7)):
        k = max(1, int(d * RATE))
        b = np.zeros(k + 1)
        a = np.zeros(k + 1)
        b[0], b[k] = -g, 1
        a[0], a[k] = 1, -g
        acc = signal.lfilter(b, a, acc)
    acc = lowpass(acc, 2500)
    return (1 - wet) * xin + wet * acc


def place(dur, events):
    """Lays (time, sound) pairs into `dur` seconds (cut at the end)."""
    out = np.zeros(n_of(dur))
    for t, s in events:
        i = n_of(t) if t > 0 else 0
        if i >= len(out):
            continue
        m = min(len(s), len(out) - i)
        out[i:i + m] += s[:m]
    return out


def bell(f, dur, partials, decay=0.4):
    """An additive bell: `partials` is (ratio, level, decay factor) for
    each mode; higher modes usually die faster."""
    t = np.arange(n_of(dur)) / RATE
    out = np.zeros_like(t)
    for r, lv, dk in partials:
        if f * r < RATE / 2 - 300:
            out += lv * np.sin(2 * np.pi * f * r * t + r) * np.exp(-t / (decay * dk))
    a = n_of(0.002)
    out[:a] *= np.linspace(0, 1, a)
    return out


def clap(bright=1500.0, q=1.5, decay=0.012):
    """One hand clap: a few very close noise bursts through a band-pass."""
    dur = decay * 8 + 0.01
    x = np.zeros(n_of(dur))
    # A clap is two or three hand-surface contacts within ~10 ms.
    for k in range(int(rng().integers(1, 4))):
        t0 = k * float(rng().uniform(0.002, 0.005))
        seg = noise(dur - t0) * np.exp(-np.arange(n_of(dur - t0)) / (RATE * decay)) * (0.6 if k else 1.0)
        x[n_of(t0):n_of(t0) + len(seg)] += seg[: len(x) - n_of(t0)]
    return bandpass(x, bright, q) + 0.15 * lowpass(x, 600)
