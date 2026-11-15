"""The editor's sounds, 1006/128-150 (editor `mod snd`): toolbar clicks,
wiring, picking up and dropping parts, the scissors, dialogs, the Light,
GO and STOP, Sound on and off, the tray. One family of soft "lab bench"
voices: plastic and woody ticks, plucked springs, little blips and
whooshes; pairs (pick/drop, open/close, on/off, GO/STOP) are mirror
images of each other."""
import numpy as np

from sfx import curve, env_points, highpass, lowpass, n_of, noise, sound, t_of

from ._ui_kit import (PLASTIC, WOOD, blip, buzz, fit, loop_noise, modes, place, pluck, snip, strike,
                      tail, whoosh)

# A C major family for the tonal sounds.
C5, E5, G5, C6 = 523.25, 659.26, 783.99, 1046.5


# --- toolbar and selection ------------------------------------------------


@sound(128, name="click")
def click():
    # A plastic button: a bright, very short tick.
    return fit(strike(1900, 0.06, 0.008, PLASTIC, grain=0.35), 0.06)


@sound(132, name="select", raw=True)
def select():
    # A tiny, quiet tick, softer and lower than the click (it plays a lot).
    return tail(0.3 * strike(1250, 0.045, 0.007, PLASTIC, grain=0.2), 0.006)


@sound(148, name="tray arrow")
def arrow():
    # Two plastic ticks, tick-tock, as a ratchet stepping a page.
    a = strike(1500, 0.08, 0.009, PLASTIC, grain=0.3)
    b = strike(1150, 0.08, 0.009, PLASTIC, grain=0.3)
    return place(0.3, (a, 0.0, 1.0), (b, 0.15, 0.7))


@sound(149, name="tray slide", raw=True)
def slide():
    # A gentle whoosh of the tray sliding, swelling at a fifth of a second.
    d = 0.5
    fc = curve(d, [(0, 500), (0.2, 1300), (0.5, 700)], log=True)
    env = env_points(d, [(0, 0), (0.05, 0.25), (0.2, 1.0), (0.32, 0.4), (0.5, 0)]) ** 1.3
    w = whoosh(d, fc, env, q=1.4)
    rumble = highpass(lowpass(noise(d, "brown"), 250), 60) * env
    # Raw, at about two thirds of full scale: a whoosh, under the click
    # it comes with.
    x = w + 0.35 * rumble / np.abs(rumble).max()
    return tail(0.6 * x / np.abs(x).max(), 0.02)


@sound(150, name="moving", raw=True)
def moving():
    # Looped while a part is dragged: a little ratchet of soft plastic
    # ticks over a faint scuff, eight a second. Exactly periodic (the
    # ticks die out well inside their slot), so the loop has no seam.
    d = 0.24
    n = n_of(d)
    bed = loop_noise(n, 300, 2000, tilt=1.0, seed=150)
    a = strike(1150, d / 2, 0.006, PLASTIC, grain=0.3)
    b = strike(980, d / 2, 0.006, PLASTIC, grain=0.3)
    ticks = np.concatenate([a, fit(b, d / 2)])[:n]
    return 0.12 * fit(ticks, d) + 0.025 * bed


# --- wires ------------------------------------------------------------------


def spring(f, dur, wobble, rate, decay):
    """A plucked spring that wobbles in pitch and loudness (boing)."""
    t = t_of(dur)
    fv = f * (1 + wobble * np.exp(-t / (dur / 3)) * np.sin(2 * np.pi * rate * t))
    x = pluck(fv, dur, decay=decay, bright=1.4, n=6)
    am = 0.65 + 0.35 * np.cos(2 * np.pi * rate * t)
    return x * am


@sound(129, name="wire pick")
def wire_pick():
    # Pulling a wire out of a knot: a small tick, then a springy boing
    # that wobbles and dies away.
    d = 0.5
    s = spring(curve(d, [(0, 360), (0.5, 330)], log=True), d, 0.06, 7.5, 0.16)
    return place(d, (strike(900, 0.05, 0.01, WOOD), 0, 0.6), (s, 0.005, 1.0))


@sound(130, name="wire connected")
def wire():
    # The wire's end seated in a knot: a round, short pluck that settles
    # up onto its note.
    d = 0.17
    f = curve(d, [(0, 560), (0.025, 625), (d, 625)], log=True)
    p = pluck(f, d, decay=0.05, bright=1.5, n=6)
    return place(d, (p, 0, 1.0), (strike(1300, 0.04, 0.006, WOOD), 0, 0.35))


@sound(131, name="wire snap")
def wire_snap():
    # A wire's loose end finding a knot: a tiny rising pip.
    d = 0.055
    f = curve(d, [(0, 2000), (0.03, 2600), (d, 2600)], log=True)
    return blip(f, d, decay=0.018, attack=0.002)


# --- parts --------------------------------------------------------------------


def lift(d, f0, f1):
    """A soft glide from f0 to f1 (a part rising or falling)."""
    f = curve(d, [(0, f0), (d, f1)], log=True)
    e = env_points(d, [(0, 0), (0.01, 1), (d * 0.6, 0.6), (d, 0)])
    return blip(f, d, attack=0.004, tri=0.4) * e


@sound(133, name="pick")
def pick():
    # Picking a part up: a woody tock, then a little rising "fwip".
    d = 0.2
    return place(d, (strike(240, 0.12, 0.03, WOOD, grain=0.4), 0, 0.9), (lift(0.15, 300, 750), 0.03, 0.55))


@sound(134, name="drop")
def drop():
    # Pick's mirror: a falling "fwoop", then the woody tock of landing.
    d = 0.18
    return place(d, (lift(0.12, 750, 300), 0, 0.5), (strike(240, 0.12, 0.03, WOOD, grain=0.4), 0.075, 1.0))


@sound(146, name="placed")
def placed():
    # A new part set down from the tray: drop's tock, with a second
    # little settling knock.
    d = 0.18
    return place(d, (lift(0.1, 650, 280), 0, 0.45), (strike(210, 0.12, 0.03, WOOD, grain=0.4), 0.06, 1.0),
                 (strike(330, 0.06, 0.015, WOOD, grain=0.3), 0.12, 0.35))


@sound(147, name="taken")
def taken():
    # A part taken out of the tray: a quick bright plastic "tuk" that
    # chirps up.
    d = 0.06
    f = curve(d, [(0, 900), (d, 1600)], log=True)
    return place(d, (strike(1400, d, 0.008, PLASTIC, grain=0.3), 0, 0.8), (blip(f, d, decay=0.02), 0, 0.6))


# --- the scissors ---------------------------------------------------------------


@sound(135, name="cut part")
def del_gadget():
    # Snip, and the part vanishes in a soft falling puff.
    d = 0.32
    t = t_of(d)
    fc = curve(d, [(0, 1600), (d, 350)], log=True)
    puff = whoosh(d, fc, env_points(d, [(0, 0), (0.02, 1), (d, 0)]) ** 2, q=1.2)
    bl = blip(curve(d, [(0, 700), (d, 220)], log=True), d) * np.exp(-t / 0.07)
    return place(d, (snip(0.06), 0, 1.0), (puff, 0.04, 0.6), (bl, 0.04, 0.35))


@sound(136, name="cut wire")
def del_wire():
    # Snip, and the cut wire springs away: a falling twang.
    d = 0.5
    f = curve(d, [(0, 900), (0.45, 260)], log=True)
    tw = pluck(f, d, decay=0.16, bright=1.3, n=6)
    return place(d, (snip(0.06), 0, 1.0), (tw, 0.03, 0.75))


@sound(137, name="cut joint")
def del_joint():
    # A smaller snip, and the wire straightens with a short spring.
    d = 0.3
    f = curve(d, [(0, 480), (0.25, 600)], log=True)
    t = t_of(d)
    sp = pluck(f * (1 + 0.04 * np.sin(2 * np.pi * 12 * t) * np.exp(-t / 0.1)), d, decay=0.06, bright=1.5, n=5)
    return place(d, (snip(0.05, f=2600), 0, 0.8), (sp, 0.035, 0.6))


# --- dialogs --------------------------------------------------------------------


def door(d, f0, f1, rise):
    """A whoosh from f0 to f1 Hz; `rise` swells it towards its end."""
    fc = curve(d, [(0, f0), (d, f1)], log=True)
    pts = [(0, 0), (0.6 * d, 0.7), (0.85 * d, 1.0), (d, 0)] if rise else [(0, 0), (0.05 * d, 1.0), (0.5 * d, 0.6), (d, 0)]
    return whoosh(d, fc, env_points(d, pts), q=1.5)


@sound(138, name="dialog open")
def dialog_open():
    # A rising whoosh, the panel unfolding, latched open with a tick.
    d = 0.36
    return place(d, (door(0.3, 400, 1800, True), 0, 0.7), (blip(curve(0.3, [(0, 400), (0.3, 900)], log=True), 0.3) *
                                                            env_points(0.3, [(0, 0), (0.25, 0.4), (0.3, 0)]), 0, 0.5),
                 (strike(1500, 0.06, 0.01, PLASTIC), 0.29, 0.8))


@sound(139, name="dialog close")
def dialog_close():
    # Open's mirror: a tick, then a falling whoosh as the panel folds.
    d = 0.4
    return place(d, (strike(1500, 0.06, 0.01, PLASTIC), 0, 0.8), (door(0.34, 1800, 400, False), 0.03, 0.7),
                 (blip(curve(0.34, [(0, 900), (0.34, 400)], log=True), 0.34) *
                  env_points(0.34, [(0, 0), (0.05, 0.4), (0.34, 0)]), 0.03, 0.5))


# --- the Light ------------------------------------------------------------------


@sound(140, name="light on")
def light_on():
    # A switch, and the lamp buzzes up: the hum rising and brightening,
    # then settling away.
    d = 0.22
    b = buzz(curve(d, [(0, 90), (0.06, 120), (d, 120)]), d, curve(d, [(0, 300), (0.05, 2200), (d, 900)], log=True))
    e = env_points(d, [(0, 0), (0.04, 1.0), (0.1, 0.5), (d, 0)])
    return place(d, (strike(1700, 0.04, 0.006, PLASTIC), 0, 0.7), (b * e, 0.005, 0.8))


@sound(141, name="light off")
def light_off():
    # On's mirror: the switch, and the hum drops and darkens as it dies.
    d = 0.22
    b = buzz(curve(d, [(0, 120), (d, 70)]), d, curve(d, [(0, 2200), (d, 250)], log=True))
    e = env_points(d, [(0, 0), (0.01, 1.0), (0.1, 0.5), (d, 0)])
    return place(d, (b * e, 0, 0.8), (strike(1300, 0.04, 0.006, PLASTIC), 0.0, 0.7))


# --- GO and STOP ----------------------------------------------------------------


def note(f, d, decay):
    """An arpeggio note: a soft pluck with a sine body (a toy glockenspiel)."""
    return 0.6 * pluck(f, d, decay=decay, bright=2.0, n=4) + 0.5 * modes(d, [(f, 1.0, decay), (f * 3.0, 0.15, decay / 3)])


@sound(142, name="GO")
def go():
    # A rising major arpeggio, its top note ringing: off we go.
    d = 0.72
    notes = [C5, E5, G5, C6]
    parts = [(note(f, d, 0.09 if i < 3 else 0.2), i * 0.075, 1.0 if i < 3 else 1.1) for i, f in enumerate(notes)]
    return place(d, *parts)


@sound(143, name="STOP")
def stop():
    # GO's mirror: the arpeggio falls, its bottom note sagging as it rings
    # out, a machine winding down.
    d = 0.78
    notes = [C6, G5, E5, C5]
    parts = [(note(f, d, 0.09), i * 0.075, 1.0) for i, f in enumerate(notes[:3])]
    last = curve(d, [(0, C5), (0.15, C5), (0.5, C5 * 0.84)], log=True)
    parts.append((note(last, d, 0.22), 0.225, 1.1))
    return place(d, *parts)


# --- Sound on and off -----------------------------------------------------------


def chord(fs, d, env):
    """Soft held sines with a slow tremolo, as from a little speaker."""
    t = t_of(d)
    x = sum(blip(f, d, tri=0.3) * (1 + 0.15 * np.sin(2 * np.pi * (5 + i) * t)) for i, f in enumerate(fs))
    return x * env


@sound(144, name="sound on")
def sound_on():
    # The speaker wakes: a chord that swells in while gliding up.
    d = 1.0
    s = curve(d, [(0, 0.8), (0.6, 1.0), (d, 1.0)], log=True)
    env = env_points(d, [(0, 0), (0.45, 0.8), (0.7, 1.0), (d, 0)]) ** 1.2
    return chord([C5 * s, E5 * s, G5 * s], d, env)


@sound(145, name="sound off")
def sound_off():
    # On's mirror: the chord starts full and glides down as it fades.
    d = 0.8
    s = curve(d, [(0, 1.0), (0.2, 1.0), (d, 0.75)], log=True)
    env = env_points(d, [(0, 0), (0.03, 1.0), (0.3, 0.8), (d, 0)]) ** 1.2
    return chord([C5 * s, E5 * s, G5 * s], d, env)
