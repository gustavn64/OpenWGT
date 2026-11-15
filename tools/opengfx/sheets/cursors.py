"""The editor's pointers (WIDGET.EXE cursor groups 5007 to 5013,
editor::Pointer): 0 glove, 1 scissors, 2 plug (knot), 3 grabbing hand,
4 open hand (Move), 5 plug (wire), 6 pointing hand. 32×32, hot spot in
CURSORS."""
from wgfx import STEEL_L, Canvas, darker, lighter

from sheets._chrome_kit import GLOVE, Shapes, Xf, cuff, glove_fist, glove_open, glove_point, scissors

PLUG_KNOT = (40, 190, 70)
PLUG_WIRE = (40, 130, 235)


def at(c, local, screen, s, rot, flip=False):
    """An Xf turned by `rot` and scaled by `s` that puts local point
    `local` at `screen`."""
    x = Xf(c, 0, 0, s, rot, flip)
    px, py = x.p(*local)
    return Xf(c, screen[0] - px, screen[1] - py, s, rot, flip)


def glove():
    """0: the glove pointing up and to the left, at the hot spot (0, 0)."""
    c = Canvas(32, 32)
    glove_point(Shapes(at(c, (-2.2, -11.4), (2.3, 2.3), 0.95, -40)))
    return c


def cut():
    """1: open scissors, the blades' crossing near the hot spot (4, 4)."""
    c = Canvas(32, 32)
    scissors(c, 10.5, 10.5, 1.0, -45)
    return c


def plug_hand(col):
    """2 and 5: a hand holding a plug whose pin's tip is the hot spot
    (2, 8)."""
    c = Canvas(32, 32)
    x = Xf(c, 0, 0)
    sh = Shapes(x, 1.0)
    # the cable, out behind the hand
    sh.cap((12.0, 8.5), (21.0, 13.0), 2.2, darker(col, 0.3))
    # the hand: its back, the cuff low on the right
    cs = Shapes(Xf(c, 16.5, 13.0, 0.95, -35), 1.0)
    cuff(cs, 6.5, 4.6)
    cs.rrect(-5.0, -5.5, 5.0, 7.0, 3.6, GLOVE)
    # the plug in front of the palm
    sh.cap((2.6, 8.5), (5.0, 8.5), 1.6, STEEL_L)
    sh.rrect(4.4, 5.4, 15.0, 11.6, 1.8, col)
    x.rrect(5.2, 6.1, 14.2, 7.3, 0.5, lighter(col, 0.55))
    x.rrect(5.2, 9.8, 14.2, 10.9, 0.5, darker(col, 0.35))
    # fingers wrapped over it, the thumb along its top
    sh.cap((16.8, 6.0), (11.6, 5.0), 2.6, GLOVE)
    for fx in (12.6, 15.0, 17.4):
        sh.cap((fx + 0.6, 6.5), (fx - 0.2, 11.6), 2.5, GLOVE)
    return c


def grab():
    """3: a closed, gripping hand round the hot spot (12, 13)."""
    c = Canvas(32, 32)
    glove_fist(Shapes(Xf(c, 12.5, 12.0, 1.0, 10)))
    return c


def move():
    """4: the open hand, its palm on the hot spot (7, 7)."""
    c = Canvas(32, 32)
    glove_open(Shapes(Xf(c, 8.0, 12.5, 0.8, 0)))
    return c


def point():
    """6: the hand pointing left, the hot spot (12, 13) on its palm."""
    c = Canvas(32, 32)
    glove_point(Shapes(Xf(c, 15.0, 11.0, 0.95, -90)))
    return c


CURSORS = {
    0: (glove, (0, 0)),
    1: (cut, (4, 4)),
    2: (lambda: plug_hand(PLUG_KNOT), (2, 8)),
    3: (grab, (12, 13)),
    4: (move, (7, 7)),
    5: (lambda: plug_hand(PLUG_WIRE), (2, 8)),
    6: (point, (12, 13)),
}
