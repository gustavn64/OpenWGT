"""The parts' own sounds (1000-1151): the Pendulum's swish, the Gravity
Chamber's claw and floor, the Physics Cannon's landing, the Coin Toss,
the Random Number Generator's drum, the Tuning Fork, the Metronome, the
Switch, the Heart, the number crunchers, the Numeric Keypad's reject,
the Switcher, the Dynamite Plunger and the Win/Lose Checker. One small
cartoon lab: wood, metal and glass struck, a motor or two, brass for
the verdict."""
import numpy as np

import sfx
from sfx import bandpass, curve, env_adsr, env_points, fade, lowpass, mix, noise, normalize, sine, soft_clip, sound, square, t_of

from ._parts_kit import brass, burst, modes, scatter, tame, thump


@sound(1000, name="pendulum swish")
def pendulum():
    # Played as the bob passes the centre, every half period (half a
    # second or so for a short pendulum): a whoosh of air through a band
    # that rises and falls as the bob goes by, a faint falling whistle
    # (its Doppler) and a soft tick from the pivot.
    d = 0.6
    env = env_points(d, [(0, 0), (0.07, 0.8), (0.14, 1.0), (0.3, 0.45), (d, 0)]) ** 1.5
    band = curve(d, [(0, 600), (0.12, 1300), (d, 700)], log=True)
    air = bandpass(noise(d, "pink"), band, 2.2) * env
    whistle = sine(curve(d, [(0, 1450), (0.15, 1300), (d, 1000)], log=True), d) * env ** 1.5 * 0.06
    tick = modes(0.05, [(1700, 0.05, 0.008), (2500, 0.025, 0.005)])
    return tame(mix(air * 2.5, whistle, tick, at=[0, 0, 0.12]))


@sound(1001, name="gravity chamber: hits the floor")
def drop_floor():
    # The dropped thing lands on the chamber's metal floor: a body thud,
    # a slap of noise, and the chamber ringing on a little.
    d = 1.0
    body = thump(d, 160, 70, decay=0.09, glide=0.025)
    slap = lowpass(burst(0.12, 900, 0.8, decay=0.018), 2500)
    ring = modes(d, [(410, 0.06, 0.2), (787, 0.045, 0.16), (1130, 0.03, 0.12), (1610, 0.02, 0.08)])
    t = t_of(d)
    tail = lowpass(bandpass(noise(d, "pink"), 700, 0.7), 1800) * np.exp(-t / 0.22) * (1 - np.exp(-t / 0.01)) * 0.5
    return tame(mix(body * 0.9, slap * 1.4, ring, tail))


@sound(1101, name="gravity chamber: claw opens")
def drop_claw():
    # Two quick metal clacks as the claw's jaws spring apart.
    def clack(f):
        return modes(0.08, [(f, 0.5, 0.012), (f * 1.37, 0.3, 0.008), (f * 0.52, 0.25, 0.02)]) + burst(0.08, f, 3, decay=0.004)
    whir = bandpass(noise(0.07), 1800, 2) * env_points(0.07, [(0, 0), (0.02, 0.12), (0.07, 0)])
    return tame(mix(clack(2600), whir, clack(2900) * 1.2, at=[0, 0.01, 0.075]))


@sound(1002, name="cannon ball lands")
def cannon():
    # The ball comes down at the end of its flight (science.rs: when it
    # reaches the far side): a heavy thump into dirt, a dusty rumble, and
    # a smaller second bounce.
    d = 0.7
    hit = thump(d, 130, 50, decay=0.13, glide=0.04)
    dirt = lowpass(noise(d, "brown"), 450) * env_points(d, [(0, 0), (0.005, 1), (0.1, 0.55), (0.45, 0.3), (d, 0)]) * 0.8
    crunch = lowpass(burst(0.1, 700, 0.7, decay=0.02), 1800)
    bounce = thump(0.25, 110, 60, decay=0.05, glide=0.02) * 0.4
    return tame(mix(hit, dirt, crunch * 1.2, bounce, at=[0, 0, 0, 0.28]))


@sound(1006, name="coin toss")
def coin():
    # A flicked coin: a bright ting and its ring, two close partials
    # beating as it spins.
    d = 0.6
    ring = modes(d, [
        (1180, 0.6, 0.16), (1187, 0.22, 0.17),
        (1890, 0.32, 0.09), (1898, 0.12, 0.10),
        (2760, 0.16, 0.05), (3350, 0.08, 0.03),
    ])
    flick = burst(0.03, 3000, 2, decay=0.003) * 0.6
    return tame(mix(ring, flick))


@sound(1007, name="random number: lottery drum", raw=True)
def lottery():
    # A draw (about 0.8 s at a time, numbers.rs): balls rattling in a
    # turning drum, a low rumble of the drum, the clacks bunching as the
    # balls tumble over.
    d = 1.0
    rng = sfx.RNG
    times = []
    for c in (0.12, 0.42, 0.72):  # the tumbles
        times += list(c + rng.normal(0, 0.08, 12))
    times += list(rng.uniform(0.02, 0.9, 16))
    times = sorted(max(0.0, x) for x in times)

    def ball(i):
        f = rng.uniform(1500, 2600)
        a = rng.uniform(0.3, 1.0)
        return a * (modes(0.04, [(f, 0.6, 0.006), (f * 1.52, 0.3, 0.004)]) + burst(0.04, f * 0.8, 2, decay=0.002) * 0.6)

    clacks = scatter(d, times, ball)
    turn = 0.6 + 0.4 * np.sin(2 * np.pi * 3.3 * t_of(d))
    drum = lowpass(noise(d, "brown"), 220) * turn * 0.35
    env = env_points(d, [(0, 0), (0.05, 1), (0.85, 1), (d, 0)])
    # Raw, to keep it as quiet as the CD's (peak 0.3): it repeats while
    # the input is on, under everything else.
    return 0.45 * normalize(fade(tame((clacks + drum) * env)), 1.0)


@sound(1008, name="tuning fork")
def tuning_fork():
    # The fork struck: a steady A 440 (a fork is nearly a pure sine), a
    # little octave for small speakers, and the strike's high clang
    # partial (about 6.3 f) dying at once. Steady after the strike, as
    # the CD's. (The port's Fork plays the default tone, 205, pitched to
    # its frequency; this one is only listed among the Fork's needs.)
    d = 0.4
    t = t_of(d)
    tone = sine(440, d) + 0.12 * sine(880, d)
    tone *= env_adsr(d, a=0.004, d=0.06, s=0.85, r=0.03) * (1 - 0.15 * t / d)
    clang = modes(d, [(2770, 0.25, 0.012), (1720, 0.08, 0.02)])
    return tame(tone * 0.8 + clang)


@sound(1011, name="metronome tick")
def metronome():
    # A wood-block tick: hard and short, at every beat.
    d = 0.08
    wood = modes(d, [(1850, 0.6, 0.012), (2700, 0.35, 0.007), (950, 0.2, 0.015)])
    snap = burst(d, 3200, 2, decay=0.0015) * 0.7
    return tame(wood + snap)


@sound(1012, name="switch click")
def switch():
    # A toggle snapping over: a small pre-click as the lever goes, the
    # snap, and the plastic body's short resonance.
    d = 0.2
    pre = burst(0.03, 2200, 2.5, decay=0.003) * 0.3
    snap = burst(0.06, 1500, 1.2, decay=0.004) * 0.35
    body = modes(d, [(330, 0.9, 0.06), (760, 0.6, 0.04), (1650, 0.25, 0.015)])
    thud = thump(d, 200, 120, decay=0.035, glide=0.01) * 0.8
    return tame(mix(pre, snap, body, thud, at=[0, 0.012, 0.012, 0.012]))


@sound(1017, name="heartbeat")
def heart():
    # One beat, played on each edge of the beat (lub, then dub): a deep,
    # soft thump, rounded so it reads as flesh, not a drum.
    d = 0.17
    t = t_of(d)
    beat = thump(d, 95, 55, decay=0.085, glide=0.05, attack=0.012)
    beat = beat * (1 - np.exp(-t / 0.01))
    flesh = lowpass(noise(d, "brown"), 160) * np.exp(-t / 0.04) * 0.25
    x = soft_clip((beat + flesh) * 1.6, 1.8)  # some harmonics so small speakers hear it
    return tame(lowpass(x, 900))


@sound(1020, name="number cruncher")
def cruncher():
    # A calculation starts (the gadget takes about 1.2 s): a little
    # motor whirs up twice, then the gears ratchet to a stop.
    d = 1.2

    def whir(dur):
        f = curve(dur, [(0, 90), (dur * 0.5, 150), (dur, 120)], log=True)
        x = bandpass(square(f, dur), 700, 1.5) * 0.5 + bandpass(noise(dur), 1400, 2) * 0.25
        return x * env_points(dur, [(0, 0), (0.03, 1), (dur - 0.05, 0.8), (dur, 0)])

    def ratchet(i):
        f = 1300 + 140 * (i % 3)
        return modes(0.05, [(f, 0.5, 0.008), (f * 1.6, 0.25, 0.005), (420, 0.25, 0.012)]) + burst(0.05, 2400, 2, decay=0.002) * 0.4

    ticks = [0.68, 0.76, 0.84, 0.93, 1.03, 1.14]
    clunk = thump(0.1, 140, 90, decay=0.03) * 0.5
    return tame(mix(whir(0.28), whir(0.22), scatter(d, ticks, ratchet), clunk, at=[0, 0.36, 0, 0.62]))


@sound(1027, name="keypad reject")
def keypad_reject():
    # A wrong key: a flat, buzzy "bzzt", two close low squares beating.
    d = 0.28
    f = 196
    x = square(f, d) + 0.6 * square(f * 1.06, d)
    x = lowpass(x, 1200, 0.9)
    return tame(x * env_adsr(d, a=0.006, d=0.03, s=0.85, r=0.03))


@sound(1036, name="switcher switching")
def switcher():
    # A train-track switch thrown: the points slide over with a scrape,
    # then lock home with a heavy metal clunk.
    d = 0.5
    scrape_d = 0.16
    scrape = bandpass(noise(scrape_d), curve(scrape_d, [(0, 900), (scrape_d, 500)], log=True), 3)
    scrape *= env_points(scrape_d, [(0, 0), (0.04, 0.25), (0.13, 0.35), (scrape_d, 0)])
    clunk = thump(0.35, 150, 80, decay=0.05, glide=0.015)
    metal = modes(0.35, [(460, 0.35, 0.045), (1020, 0.22, 0.03), (1680, 0.12, 0.02), (2350, 0.06, 0.012)])
    hit = lowpass(burst(0.2, 800, 0.8, decay=0.03, color="pink"), 2500) * 1.6
    return tame(mix(scrape, clunk, metal, hit, at=[0, 0.15, 0.15, 0.15]), 3500)[: int(d * sfx.RATE)]


@sound(1048, name="dynamite explosion")
def explosion():
    # The plunger goes down: a boom. A low thump under a burst of brown
    # noise whose band closes as it rolls away, with crackles of debris
    # in the first half.
    d = 1.15
    t = t_of(d)
    boom = thump(d, 90, 40, decay=0.2, glide=0.06, attack=0.003)
    roar = 0.25 * noise(d, "brown") + 1.2 * noise(d, "pink") + 0.4 * noise(d)
    roar = lowpass(roar, curve(d, [(0, 2600), (0.15, 1200), (0.5, 500), (d, 250)], log=True), 0.8)
    roar *= env_points(d, [(0, 0), (0.008, 1), (0.12, 0.85), (0.4, 0.4), (0.8, 0.12), (d, 0)])
    roar /= np.abs(roar).max()
    rng = sfx.RNG
    crack_t = sorted(rng.uniform(0.04, 0.55, 18))
    cracks = scatter(d, crack_t, lambda i: burst(0.03, rng.uniform(700, 1800), 1.5, decay=0.004) * rng.uniform(0.3, 0.9) * (1 - crack_t[i]))
    x = boom * 0.35 + roar + cracks
    return tame(soft_clip(x * 1.3, 1.3), 3200) * (1 - np.exp(-t / 0.002))


C5, E5, G5, C6, G4, E4 = 523.25, 659.26, 783.99, 1046.5, 392.0, 329.63


@sound(1051, name="checker: win fanfare")
def win():
    # Ta-ta-taa ... taaaa, ta-DAA: a brass fanfare in C, a third below
    # in harmony on the long notes.
    def note(f, dur, vib=0.0, harm=None):
        x = brass(f, dur, vib=vib, bright=2800)
        if harm:
            x = x + 0.55 * brass(harm, dur, vib=vib, bright=2200)
        return x

    parts = [
        (note(G4, 0.13), 0.00),
        (note(C5, 0.13), 0.17),
        (note(E5, 0.13), 0.34),
        (note(G5, 0.30, harm=E5), 0.51),
        (note(E5, 0.13), 0.87),
        (note(G5, 0.80, vib=0.012, harm=E5), 1.04),
        (note(G5, 0.11), 1.90),
        (note(C6, 0.42, vib=0.01, harm=G5), 2.03),
    ]
    return tame(mix(*[p for p, _ in parts], at=[a for _, a in parts]))


@sound(1151, name="checker: lose")
def lose():
    # The sad trombone: wah, wah, wah, waaaah, down by semitones, each
    # through a mute opening and closing, the last one sagging with a
    # slow wobble.
    notes = [233.08, 220.0, 207.65]
    parts, at = [], []
    for i, f in enumerate(notes):
        d = 0.42
        wah = curve(d, [(0, 350), (0.12, 1800), (0.3, 1100), (d, 500)], log=True)
        parts.append(brass(f, d, a=0.04, r=0.08, wah=wah) * 0.9)
        at.append(i * 0.47)
    d = 1.3
    f = curve(d, [(0, 196.0), (0.5, 194.0), (d, 182.0)], log=True)
    wah = curve(d, [(0, 350), (0.15, 1700), (0.4, 1200), (d, 450)], log=True)
    t = t_of(d)
    wobble = 1 + 0.025 * np.clip((t - 0.25) / 0.3, 0, 1) * np.sin(2 * np.pi * 4.5 * t)
    last = brass(f * wobble, d, a=0.04, r=0.5, s=0.9, wah=wah * (1 + 0.15 * (wobble - 1) / 0.025))
    parts.append(last)
    at.append(3 * 0.47)
    return tame(mix(*parts, at=at), 3200)
