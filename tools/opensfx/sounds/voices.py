"""The Animal Sound part's sounds (1006/200-203: dog, cat, rooster, cow)
and the Misc Sound part's (1006/300-309: buzzer, alarm, chime, clap,
breath, laugh, cuckoo, weird, spooky, chord), all synthesized: voices are
a pulse train with a pitch contour through moving formants, the rest
oscillators, FM, filtered noise and a small synthetic room."""
import numpy as np

from sfx import (
    bandpass, curve, echo, env_exp, env_points, fm, lowpass, n_of, noise, normalize, pulse_train,
    sine, soft_clip, sound, square, t_of,
)

from ._voices_kit import bell, clap, drift, place, rng, room, tame, vibrato, voice

# ---------------------------------------------------------------------------
# Animal Sound


def bark(f0, dur=0.2):
    """One "woof": a rough, breathy voice jumping up and falling, its
    first formant opening wide (w -> a -> o)."""
    f = curve(dur, [(0, f0 * 0.8), (0.025, f0 * 1.1), (0.07, f0), (dur, f0 * 0.62)], log=True) * drift(dur, 0.05, 80)
    pts = [
        (0.0, [(380, 3), (900, 4), (2300, 6)]),
        (0.03, [(800, 3), (1250, 4), (2500, 6)]),
        (0.09, [(700, 3), (1100, 4), (2450, 6)]),
        (dur, [(450, 3), (800, 4), (2300, 6)]),
    ]
    v = voice(f, dur, pts, width=0.55, breath=0.35, gains=(1.0, 0.6, 0.2), rough=0.7)
    env = env_points(dur, [(0, 0), (0.006, 1), (0.04, 0.9), (0.1, 0.4), (dur, 0)])
    return soft_clip(normalize(v * env), 2.5)


@sound(200, name="dog")
def dog():
    x = place(0.6, [(0.0, bark(470, 0.21)), (0.3, 0.85 * bark(430, 0.23))])
    return tame(room(x, size=1.0, decay=0.7, wet=0.3)[: n_of(0.86)])


@sound(201, name="cat")
def cat():
    d = 0.62
    f = curve(d, [(0, 470), (0.12, 600), (0.25, 640), (0.4, 580), (d, 430)], log=True)
    f = f * vibrato(d, 6.5, 0.012) * drift(d, 0.01, 20)
    # m -> e -> a -> o -> u: "mi-a-o-w"
    pts = [
        (0.0, [(300, 4), (1700, 8), (2700, 10)]),
        (0.08, [(450, 5), (1750, 9), (2800, 10)]),
        (0.2, [(700, 5), (1600, 9), (2800, 10)]),
        (0.33, [(950, 5), (1250, 8), (2700, 10)]),
        (0.47, [(750, 5), (1000, 8), (2500, 10)]),
        (d, [(450, 5), (800, 8), (2400, 10)]),
    ]
    v = voice(f, d, pts, width=0.4, breath=0.06, gains=(1.0, 0.8, 0.25))
    env = env_points(d, [(0, 0), (0.06, 0.2), (0.15, 1.0), (0.24, 0.6), (0.34, 1.0), (0.45, 0.45), (d, 0)])
    return tame(v * env)


@sound(202, name="rooster")
def rooster():
    d = 2.75
    # cock - a - doo - dle - doooo: pitch steps, the last one falling.
    f = curve(d, [
        (0.0, 520), (0.08, 650), (0.25, 640),
        (0.29, 700), (0.5, 690),
        (0.55, 660), (1.0, 680),
        (1.05, 690), (1.6, 660), (2.0, 640),
        (2.06, 600), (2.3, 560), (2.7, 440), (d, 420),
    ], log=True)
    f = f * drift(d, 0.012, 15) * vibrato(d, 28, 0.01)
    bright = [(1300, 3), (1950, 5), (2700, 8)]
    closed = [(600, 3), (1500, 5), (2500, 8)]
    pts = [
        (0.0, closed), (0.06, bright), (0.24, bright), (0.28, closed), (0.32, bright), (0.5, bright),
        (0.54, closed), (0.6, bright), (1.0, bright), (1.04, closed), (1.1, bright), (2.0, bright),
        (2.05, closed), (2.12, [(1200, 3), (1800, 5), (2600, 8)]), (d, [(900, 3), (1500, 5), (2500, 8)]),
    ]
    v = voice(f, d, pts, width=0.3, breath=0.06, gains=(1.0, 0.8, 0.3), rough=0.15)
    env = env_points(d, [
        (0, 0), (0.03, 0.8), (0.2, 0.9), (0.26, 0.15),
        (0.3, 0.9), (0.47, 0.7), (0.53, 0.2),
        (0.58, 1.0), (1.0, 0.85), (1.03, 0.55),
        (1.08, 0.9), (2.0, 0.95), (2.05, 0.55),
        (2.1, 0.85), (2.6, 0.85), (d, 0),
    ])
    return tame(soft_clip(normalize(v * env), 1.6))


@sound(203, name="cow")
def cow():
    d = 1.65
    f = curve(d, [(0, 95), (0.3, 108), (0.8, 118), (1.3, 120), (d, 100)], log=True)
    f = f * drift(d, 0.015, 8) * vibrato(d, 5, 0.006)
    pts = [
        (0.0, [(280, 4), (1000, 6), (2200, 8)]),  # m
        (0.22, [(300, 4), (950, 6), (2200, 8)]),
        (0.45, [(520, 5), (900, 7), (2300, 9)]),  # oo -> o
        (1.3, [(500, 5), (850, 7), (2300, 9)]),
        (d, [(380, 5), (750, 7), (2200, 9)]),  # u
    ]
    # A narrow glottal pulse: a buzzy, nasal low voice.
    v = voice(f, d, pts, width=0.08, breath=0.04, gains=(1.0, 0.9, 0.7), rough=0.15)
    v = v + 0.5 * bandpass(v, 1800, 1.0)  # the nasal buzz up top
    env = env_points(d, [(0, 0), (0.15, 0.45), (0.4, 0.9), (0.6, 1.0), (1.4, 0.95), (d, 0)])
    env = env * drift(d, 0.06, 6)
    return tame(v * env, 3000)


# ---------------------------------------------------------------------------
# Misc Sound


@sound(300, name="buzzer")
def buzzer():
    d = 0.38
    # An armature hammering a plate 120 times a second, every other
    # stroke harder (the 60 Hz rasp), ringing the plate's modes.
    raw = pulse_train(120, d, width=0.08) * (1 + 0.45 * square(60, d))
    body = bandpass(raw, 900, 6) + 0.9 * bandpass(raw, 1250, 7) + 0.8 * bandpass(raw, 1550, 8)
    body = body / np.abs(body).max() + 0.35 * lowpass(square(120, d), 500)
    return tame(soft_clip(body, 1.3) * env_points(d, [(0, 0), (0.005, 1), (d - 0.01, 1), (d, 0)]))


@sound(301, name="alarm")
def alarm():
    # A two-tone electronic alarm: hi-lo-hi-lo, buzzy and urgent.
    d = 0.84
    step = d / 6
    f = np.where((np.arange(n_of(d)) // n_of(step)) % 2 == 0, 880.0, 660.0)
    f = lowpass(f, 120)  # a hair of glide between the two
    x = 0.5 * square(f, d) + 0.5 * sine(f, d)
    x = soft_clip(x, 1.5)
    gate = 0.85 + 0.15 * np.cos(2 * np.pi * np.arange(n_of(d)) / n_of(step))
    return tame(x * gate * env_points(d, [(0, 0), (0.01, 1), (d - 0.03, 1), (d, 0)]), 3000)


@sound(302, name="chime")
def chime():
    d = 0.65
    f = 784.0
    # A tubular chime's stretched modes (with a low "hum" under them).
    parts = [(0.5, 0.5, 1.4), (1.0, 1.0, 1.0), (1.32, 0.35, 0.6), (1.71, 0.5, 0.55), (2.0, 0.25, 0.45),
             (2.74, 0.3, 0.3), (3.4, 0.12, 0.22)]
    x = bell(f, d, parts, decay=0.28)
    x = x + 0.25 * fm(f, 1.4, 1.2, d, index_env=np.exp(-t_of(d) / 0.05)) * np.exp(-t_of(d) / 0.12)
    return tame(x)


@sound(303, name="clap")
def applause():
    # A handful of people clapping, each at their own pace.
    d = 2.85
    events = []
    for p in range(6):
        rate = float(rng().uniform(3.6, 5.2))
        bright = float(rng().uniform(1000, 2200))
        lvl = float(rng().uniform(0.5, 1.0))
        t = float(rng().uniform(0, 0.25))
        while t < d - 0.05:
            events.append((t, lvl * float(rng().uniform(0.6, 1.0)) * clap(bright * float(rng().uniform(0.9, 1.1)), 1.4, float(rng().uniform(0.008, 0.016)))))
            t += (1 / rate) * float(rng().uniform(0.85, 1.15))
    x = place(d, events)
    x = room(x, size=0.6, decay=0.35, wet=0.15)[: n_of(d)]
    return tame(x * env_points(d, [(0, 0.6), (0.3, 1), (d - 0.3, 1), (d, 0.3)]))


@sound(304, name="breath")
def breath():
    # In through the nose and mouth (bright, swelling), out (warmer, quick).
    di, do = 0.85, 0.55
    n_in = noise(di, "pink") * 0.5 + noise(di) * 0.5
    inhale = 0.8 * bandpass(n_in, 2600, 1.2) + 0.5 * bandpass(n_in, 1500, 2.5) + 0.3 * bandpass(n_in, 3400, 2)
    inhale *= env_points(di, [(0, 0), (0.35, 0.8), (0.5, 1.0), (0.7, 0.35), (di, 0)])
    n_out = noise(do, "pink") * 0.7 + noise(do) * 0.3
    exhale = (bandpass(n_out, 650, 3) * 0.6 + bandpass(n_out, 1650, 3) + bandpass(n_out, 2600, 3) * 0.4)
    exhale *= env_points(do, [(0, 0), (0.06, 1.0), (0.15, 0.7), (do, 0)]) ** 1.5
    x = place(1.5, [(0.03, 0.75 * inhale), (0.95, 1.5 * exhale)])
    return tame(x, 3500)


@sound(305, name="laugh")
def laugh():
    # "HA ha ha ha ...": breathy syllables, each a little lower and softer.
    d = 1.84
    events = []
    t = 0.12
    levels = [1.0, 0.62, 0.55, 0.5, 0.42, 0.4, 0.34, 0.32, 0.22, 0.16]
    for k, lv in enumerate(levels):
        dur = 0.17 if k == 0 else 0.12
        f0 = 420 * (0.955 ** k)
        # Each "ha" bends up and back down.
        f = curve(dur, [(0, f0 * 0.85), (dur * 0.4, f0), (dur, f0 * 0.8)], log=True) * drift(dur, 0.015, 30)
        pts = [(0, [(650, 4), (1250, 6), (2700, 9)]), (dur * 0.5, [(850, 5), (1150, 7), (2800, 10)]), (dur, [(750, 5), (1150, 7), (2700, 10)])]
        v = voice(f, dur, pts, width=0.4, breath=0.05, gains=(1.0, 0.8, 0.3))
        # The "h": breath through the same mouth before the voice starts.
        h = voice(f, dur, pts, width=0.4, breath=1.0, gains=(1.0, 0.7, 0.3)) - v
        att = 0.025
        ev = env_points(dur, [(0, 0), (att, 0.1), (att + 0.012, 1), (dur * 0.6, 0.7), (dur, 0)])
        eh = env_points(dur, [(0, 0), (0.008, 1), (att + 0.01, 0.4), (dur, 0.1)])
        events.append((t, lv * (v * ev + 0.25 * h * eh)))
        t += 0.19 if k == 0 else 0.165 + 0.004 * k
    return tame(place(d, events), 3200)


@sound(306, name="cuckoo")
def cuckoo():
    # A cuckoo clock's two little pipes, a major third apart.
    def pipe(f, dur):
        g = f * (1 - 0.03 * np.exp(-t_of(dur) / 0.015))
        x = sine(g, dur) + 0.12 * sine(2 * g, dur) + 0.05 * sine(3 * g, dur)
        x += 0.6 * bandpass(noise(dur), f, 8)
        return x * env_points(dur, [(0, 0), (0.012, 1), (dur * 0.5, 0.8), (dur, 0)])

    return place(0.39, [(0.0, pipe(900, 0.13)), (0.16, 0.7 * pipe(720, 0.22))])


@sound(307, name="weird")
def weird():
    # A burble of FM chirps, then a few rubbery "boing"s.
    d = 1.5
    events = []
    t = 0.0
    while t < 0.6:
        dur = float(rng().uniform(0.04, 0.09))
        a, b = rng().uniform(200, 900, 2)
        f = curve(dur, [(0, a), (dur, b)], log=True)
        x = fm(f, float(rng().choice([0.5, 1.5, 2.0, 3.0])), 2.5, dur) * env_points(dur, [(0, 0), (0.005, 1), (dur, 0)])
        events.append((t, 0.8 * x))
        t += dur * float(rng().uniform(0.8, 1.1))
    for t0, f0, up in ((0.68, 180, 3.0), (0.9, 260, 2.2), (1.08, 150, 4.0), (1.3, 300, 1.6)):
        dur = 0.17
        f = curve(dur, [(0, f0), (dur, f0 * up)], log=True) * vibrato(dur, 30, 0.06)
        x = fm(f, 1.0, 1.5, dur, index_env=np.exp(-t_of(dur) / 0.06)) * env_exp(dur, 0.06, 0.003)
        events.append((t0, x))
    x = echo(place(d, events), delay=0.09, gain=0.3, n=2)[: n_of(d)]
    return tame(x * env_points(d, [(0, 1), (d - 0.05, 1), (d, 0)]))


@sound(308, name="spooky")
def spooky():
    # A theremin's ghostly glide over a cold wind, in a big room.
    d = 2.2
    f = curve(d, [(0, 300), (0.35, 600), (0.7, 560), (1.0, 690), (1.4, 520), (1.8, 420), (d, 300)], log=True)
    f = f * vibrato(d, 5.5, 0.025)
    tone = sine(f, d) + 0.15 * sine(2 * f, d) + 0.04 * sine(3 * f, d)
    tone *= env_points(d, [(0, 0), (0.25, 0.9), (1.4, 1.0), (d, 0)])
    wind = bandpass(noise(d, "pink"), curve(d, [(0, 300), (0.8, 700), (1.5, 400), (d, 250)]), 3)
    wind *= env_points(d, [(0, 0), (0.4, 1), (1.6, 0.8), (d, 0)])
    rms = lambda y: np.sqrt(np.mean(y * y))
    x = tone / rms(tone) + 0.45 * wind / rms(wind)
    x = echo(x, delay=0.16, gain=0.35, n=2)
    return tame(room(x, size=1.4, decay=0.7, wet=0.35)[: n_of(2.5)], 3000)


@sound(309, name="chord")
def chord():
    # A major chord on a piano-ish instrument: A2 E3 A3 C#4 E4.
    d = 1.38
    t = t_of(d)
    x = np.zeros_like(t)
    for i, midi in enumerate((45, 52, 57, 61, 64)):
        f0 = 440 * 2 ** ((midi - 69) / 12)
        for k in range(1, 12):
            fk = f0 * k * np.sqrt(1 + 0.0004 * k * k)
            if fk > 3500:
                break
            amp = (1 / k) * (0.7 if k == 1 else 1.0)
            tau = 0.55 / (1 + 0.35 * (k - 1)) * (220 / f0) ** 0.3
            # Two slightly detuned strings, beating at a different rate per note.
            for det, ph in ((-0.3 - 0.13 * i, 0.0), (0.3 + 0.11 * i, 1.7 * k + i)):
                x += 0.5 * amp * np.sin(2 * np.pi * (fk + det) * t + ph + 0.9 * k * i) * np.exp(-t / tau)
    hammer = lowpass(noise(0.03), 1500) * np.linspace(1, 0, n_of(0.03))
    x[: len(hammer)] += 0.6 * hammer * np.abs(x).max()
    x *= env_points(d, [(0, 0), (0.003, 1), (d - 0.1, 1), (d, 0)])
    return tame(x)
