"""Helpers for the editor's sounds (sounds/ui.py): small modal "lab
bench" voices built on the kit in sfx.py, woody and plastic ticks,
plucked springs, blips, whooshes, a lamp's buzz and a seamless noise
loop. Everything is synthesized; nothing is taken from the CD."""
import numpy as np

import sfx
from sfx import RATE, bandpass, lowpass, highpass, n_of, noise, phase, t_of

# Partial ratios, amplitudes and relative decays of struck little objects.
WOOD = [(1.0, 1.0, 1.0), (2.76, 0.35, 0.45), (5.40, 0.12, 0.25)]
PLASTIC = [(1.0, 1.0, 1.0), (1.71, 0.5, 0.7), (2.83, 0.25, 0.45), (4.1, 0.1, 0.3)]
METAL = [(1.0, 1.0, 1.0), (2.32, 0.6, 0.9), (4.25, 0.35, 0.8), (6.63, 0.2, 0.6)]

TOP = 4300.0  # nothing louder than a whisper above this (8-bit, 11025 Hz)


def fit(x, dur):
    """`x` cut or padded with silence to `dur` seconds."""
    n = n_of(dur)
    return x[:n] if len(x) >= n else np.concatenate([x, np.zeros(n - len(x))])


def modes(dur, partials, attack=0.0008):
    """A sum of exponentially decaying sines, (f, amp, decay) each; an
    `f` may be an array (a glide). Partials past TOP are left out."""
    t = t_of(dur)
    out = np.zeros_like(t)
    for f, a, d in partials:
        if np.max(f) > TOP:
            continue
        out += a * np.sin(phase(f, dur)) * np.exp(-t / d)
    na = n_of(attack)
    out[:na] *= np.linspace(0, 1, min(na, len(out)))
    return out


def strike(f, dur=0.12, decay=0.03, kind=WOOD, grain=0.25):
    """A struck object at `f` Hz: modal ring plus a short noise scuff
    through the same pitch, the "contact" of the strike."""
    body = modes(dur, [(f * r, a, decay * d) for r, a, d in kind])
    sc = bandpass(noise(dur), min(f * 1.8, TOP), 1.5) * np.exp(-t_of(dur) / 0.004)
    x = body + grain * sc / max(1e-9, np.abs(sc).max())
    return x / max(1e-9, np.abs(x).max())


def pluck(f, dur, decay=0.1, bright=1.2, n=8):
    """A plucked spring or string: harmonics of `f` (a number or a glide),
    the higher ones softer and shorter."""
    t = t_of(dur)
    p = phase(f, dur)
    fa = np.broadcast_to(np.asarray(f, dtype=float), p.shape)
    out = np.zeros_like(t)
    for k in range(1, n + 1):
        alive = fa * k < TOP
        if not alive.any():
            break
        out += np.where(alive, np.sin(k * p) / k ** bright * np.exp(-t * k ** 0.6 / decay), 0.0)
    out[: n_of(0.001)] *= np.linspace(0, 1, n_of(0.001))
    return out / max(1e-9, np.abs(out).max())


def blip(f, dur, decay=None, attack=0.003, tri=0.25):
    """A soft little tone (a sine with a touch of third harmonic), `f`
    a number or a glide; held, or decaying with `decay`."""
    p = phase(f, dur)
    x = np.sin(p) + tri * np.sin(3 * p) / 3
    t = t_of(dur)
    e = np.ones_like(t) if decay is None else np.exp(-t / decay)
    na = n_of(attack)
    e[:na] *= np.linspace(0, 1, na)
    nr = n_of(min(0.01, dur / 4))
    e[-nr:] *= np.linspace(1, 0, nr)
    return x * e


def whoosh(dur, fc, env, q=1.6, color="pink"):
    """Air: noise through a band sweeping along `fc` (a curve), shaped
    by `env` (an envelope array)."""
    x = noise(dur, color)
    y = bandpass(x, fc, q) + 0.5 * bandpass(x, np.minimum(fc * 1.7, TOP), q * 1.3)
    y = lowpass(y, TOP, 0.7)
    y = y / max(1e-9, np.abs(y).max())
    return y * env


def buzz(f, dur, bright):
    """A lamp's mains buzz: a sawtooth at `f` (number or glide) through a
    lowpass at `bright` (number or curve), with a little crackle."""
    x = sfx.saw(f, dur)
    y = lowpass(x, bright, 1.2)
    cr = highpass(noise(dur), 1500) * (sfx.RNG.random(n_of(dur)) < 0.02)
    y = y + 0.4 * lowpass(cr, 3500)
    return y / max(1e-9, np.abs(y).max())


def snip(dur=0.06, f=2300.0, level=1.0):
    """Scissor blades meeting: a bright scrape that ends in a small
    metallic tick."""
    t = t_of(dur)
    sc = bandpass(noise(dur), f, 2.0) * np.exp(-t / 0.012)
    tick = modes(dur, [(f * 0.62 * r, a, 0.015 * d) for r, a, d in METAL])
    x = 0.8 * sc / max(1e-9, np.abs(sc).max()) + tick / max(1e-9, np.abs(tick).max())
    return level * x / np.abs(x).max()


def loop_noise(n, lo, hi, tilt=1.0, seed=0):
    """Exactly periodic band-limited noise of `n` samples: it loops
    without a seam (random phases on whole-cycle frequencies)."""
    rng = np.random.default_rng(seed)
    f = np.fft.rfftfreq(n, 1 / RATE)
    mag = np.where((f >= lo) & (f <= hi), 1.0 / np.maximum(f, 1.0) ** (tilt / 2), 0.0)
    # Soft band edges.
    mag *= np.clip((f - lo) / (0.3 * lo + 1), 0, 1) * np.clip((hi - f) / (0.3 * hi + 1), 0, 1)
    spec = mag * np.exp(2j * np.pi * rng.random(len(f)))
    x = np.fft.irfft(spec, n)
    return x / max(1e-9, np.abs(x).max())


def place(n_dur, *parts):
    """`parts` as (sound, start seconds, gain) summed into `n_dur` seconds."""
    out = np.zeros(n_of(n_dur))
    for x, at, g in parts:
        s = n_of(at) if at else 0
        k = min(len(x), len(out) - s)
        if k > 0:
            out[s:s + k] += g * x[:k]
    return out


def tail(x, dur=0.01):
    """A short fade-out at the end (for raw sounds)."""
    x = x.copy()
    k = n_of(dur)
    x[-k:] *= np.linspace(1, 0, k)
    k = n_of(0.001)
    x[:k] *= np.linspace(0, 1, k)
    return x
