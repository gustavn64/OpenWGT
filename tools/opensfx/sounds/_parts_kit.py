"""Helpers for the parts' own sounds (sounds/parts.py): struck modes,
noise bursts, pitch-dropping thumps, a brass voice. All made from the
kit's oscillators, noise and filters."""
import numpy as np

import sfx
from sfx import RATE, bandpass, curve, env_adsr, lowpass, n_of, noise, saw, t_of


def modes(dur, parts, attack=0.0005):
    """A struck object: decaying sines, `parts` (freq Hz, amplitude,
    decay s) each. A random start phase per mode keeps the strike
    from being one big spike."""
    t = t_of(dur)
    out = np.zeros_like(t)
    for f, a, d in parts:
        out += a * np.sin(2 * np.pi * f * t + sfx.RNG.uniform(0, 2 * np.pi)) * np.exp(-t / d)
    na = min(len(out), n_of(attack))
    out[:na] *= np.linspace(0, 1, na)
    return out


def burst(dur, f, q=3.0, decay=0.01, color="white", attack=0.0005):
    """A noise burst through a band: a click, a scrape, a clack."""
    t = t_of(dur)
    x = bandpass(noise(dur, color), f, q) * np.exp(-t / decay)
    na = min(len(x), n_of(attack))
    x[:na] *= np.linspace(0, 1, na)
    return x


def thump(dur, f0, f1, decay=0.08, glide=0.03, attack=0.002):
    """A low sine falling from f0 to f1 Hz (in `glide` s) as it dies:
    a kick, a heartbeat, a body hitting the floor."""
    t = t_of(dur)
    f = f1 + (f0 - f1) * np.exp(-t / glide)
    x = np.sin(2 * np.pi * np.cumsum(f) / RATE)
    e = np.exp(-t / decay)
    na = min(len(x), n_of(attack))
    e[:na] *= np.linspace(0, 1, na)
    return x * e


def brass(f, dur, a=0.025, r=0.06, s=0.85, bright=2400, vib=0.0, vib_rate=5.5, vib_delay=0.15, wah=None):
    """A brassy note: a saw through a lowpass that opens on the attack
    (the 'blat') and settles; `vib` the vibrato depth (fraction of f),
    coming in after `vib_delay`; `wah` an array of cutoffs instead (a
    mute opening and closing)."""
    t = t_of(dur)
    depth = vib * np.clip((t - vib_delay) / 0.15, 0, 1)
    fr = np.broadcast_to(np.asarray(f, dtype=float), t.shape) * (1 + depth * np.sin(2 * np.pi * vib_rate * t))
    x = saw(fr, dur)
    if wah is None:
        cut = curve(dur, [(0, 500), (min(a * 1.5, dur), bright), (min(a * 4 + 0.05, dur), bright * 0.7), (dur, bright * 0.6)])
    else:
        cut = wah
    x = lowpass(x, np.minimum(cut, 3600), 1.3)
    return x * env_adsr(dur, a=a, d=0.08, s=s, r=r)


def tame(x, f=3800):
    """Takes off what's above `f` Hz (an 11025 Hz 8-bit wave has little
    room there)."""
    return lowpass(lowpass(x, f), f)


def scatter(dur, times, make):
    """`make(i)` placed at each of `times` (s), summed into `dur` s."""
    out = np.zeros(n_of(dur))
    for i, tt in enumerate(times):
        p = make(i)
        s = n_of(tt) if tt else 0
        if s >= len(out):
            continue
        k = min(len(p), len(out) - s)
        out[s:s + k] += p[:k]
    return out
