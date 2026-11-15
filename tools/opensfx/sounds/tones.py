"""The Speaker's tone samples (1006/220 and 1006/224: 220 Hz and 1760 Hz,
looped and played at other rates for other pitches,
gadgets::sound_time::tone_rate) and the default tone (1006/205), the
sound a Tape Player has before anything is recorded."""
import numpy as np

from sfx import env_adsr, sine, sound

# 2205 samples are exactly 44 cycles of 220 Hz and 352 of 1760 Hz at
# 11025 Hz: the loop has no seam.
LOOP = 2205 / 11025


def organ(f, dur):
    return 0.62 * sine(f, dur) + 0.22 * sine(2 * f, dur) + 0.1 * sine(3 * f, dur) + 0.06 * sine(4 * f, dur)


@sound(220, name="tone 220 Hz", raw=True)
def tone_220():
    return 0.8 * organ(220, LOOP)


@sound(224, name="tone 1760 Hz", raw=True)
def tone_1760():
    # Only the first harmonic fits under the 5512 Hz Nyquist with room.
    return 0.8 * (0.85 * sine(1760, LOOP) + 0.15 * sine(3520, LOOP))


@sound(205, name="default tone")
def default_tone():
    d = 0.96
    vib = 220 * (1 + 0.004 * np.sin(2 * np.pi * 5 * np.arange(int(d * 11025)) / 11025))
    return organ(vib, d) * env_adsr(d, a=0.02, d=0.1, s=0.8, r=0.15)
