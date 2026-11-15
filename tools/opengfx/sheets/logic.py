"""The logic gates (AND, OR, XOR, NOT: sheets 1029 to 1032)."""
from wgfx import INK, PANEL_BLUE, WHITE, darker, lighter, sheet


def gate(ctx, label, base):
    c = ctx.canvas(1)
    k = ctx.knots()
    left = min(x for x, _, o in k if not o) + 4
    right = max(x for x, _, o in k if o) - 4
    c.panel(left, 1, right, c.h - 1, base, radius=3, bevel=1.5)
    for sx in (left + 3.5, right - 3.5):
        for sy in (4.5, c.h - 4.5):
            c.screw(sx, sy, 1.4)
    c.text_fit(left + 6, 4, right - 6, c.h - 4, label, WHITE, "sans", 15, stroke=1.2, stroke_fill=darker(base, 0.6))
    c.knots(k)
    return [c, ctx.blank(2)]


@sheet(1029)
def and_gate(ctx):
    return gate(ctx, "AND", PANEL_BLUE)


@sheet(1030)
def or_gate(ctx):
    return gate(ctx, "OR", (30, 150, 90))


@sheet(1031)
def xor_gate(ctx):
    return gate(ctx, "XOR", (150, 70, 200))


@sheet(1032)
def not_gate(ctx):
    return gate(ctx, "NOT", (210, 70, 50))
