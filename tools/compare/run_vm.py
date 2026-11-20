#!/usr/bin/env python3
"""Plays a comparison script (see crates/engine/examples/script.rs) into
the original in the Windows 95 VM (~/WidgetWorkshop-vm, vm.py), from the
snapshot with the editor open, and saves a screenshot of the editor's
area at each `shot`.

    run_vm.py SCRIPT OUT_DIR [--snapshot ww-editor]
"""
import os
import re
import sys
import time

VM = os.path.expanduser("~/WidgetWorkshop-vm")
sys.path.insert(0, VM)
import vm  # noqa: E402
from PIL import Image  # noqa: E402

# The editor's top left on the VM's screen: the maximized window's client
# area, under the title bar.
OX, OY = 2, 21
W, H = 640, 433


# QEMU's `mouse_button` takes the buttons held, as a mask.
LEFT, RIGHT = 1, 4

# The script's commands the VM can't play: the port's `open` drops a file
# on the window and `puzzle` loads one directly; here, start from a
# snapshot with the widget or puzzle already open instead. The port's
# commands that work with parts by name and with the run
# (engine::script::cmd) change the widget in ways the VM can't follow.
UNSUPPORTED = {"open", "puzzle", "new", "run", "stop", "step", "until", "press", "set", "poke",
               "setting", "seed"}
# The port's commands that only look (or name parts to look with):
# skipped here.
SKIPPED = {"name", "expect", "assert", "log", "print", "parts", "wires", "help"}
KNOWN = {"size", "dump", "move", "down", "up", "click", "dclick", "drag", "key", "wait", "shot"} | SKIPPED

# `#` starts a comment, unless a digit follows (`#7` is a part).
COMMENT = re.compile(r"#(?!\d)")


def parse(script):
    """The script's lines as (line number, command, words, numbers),
    checked before the VM is touched."""
    lines = []
    for i, line in enumerate(open(script), 1):
        w = COMMENT.split(line)[0].split()
        if not w:
            continue
        cmd, a = w[0], w[1:]
        if cmd in UNSUPPORTED:
            raise SystemExit(f"line {i}: `{cmd}` can't be played in the VM (start from a snapshot with it open)")
        if cmd not in KNOWN:
            raise SystemExit(f"line {i}: unknown command {cmd}")
        n = []
        for v in a:
            try:
                n.append(int(v, 0))
            except ValueError:
                pass
        lines.append((i, cmd, a, n))
    return lines


def press(mask):
    vm.hmp(f"mouse_button {mask}")


def move(x, y, frm=None):
    """The mouse to (x, y), in 8-pixel steps from `frm` as the port moves it
    (a drag sees the way)."""
    if frm is None:
        vm.mouse_to(OX + x, OY + y)
        return
    x0, y0 = frm
    k = max(1, (max(abs(x - x0), abs(y - y0)) + 7) // 8)
    for j in range(1, k + 1):
        vm.mouse_to(OX + x0 + (x - x0) * j // k, OY + y0 + (y - y0) * j // k)
        time.sleep(0.02)


def main(argv):
    script, out = argv[0], argv[1]
    snap = argv[argv.index("--snapshot") + 1] if "--snapshot" in argv else "ww-editor"
    lines = parse(script)
    os.makedirs(out, exist_ok=True)
    vm.hmp(f"loadvm {snap}")
    time.sleep(2)
    vm.home()
    vm.mouse_to(OX + 320, OY + 380)
    time.sleep(1)
    for _, cmd, a, n in lines:
        btn = RIGHT if "right" in a else LEFT
        if cmd in ("size", "dump") or cmd in SKIPPED:
            continue
        if cmd == "move":
            move(n[0], n[1])
        elif cmd in ("down", "up"):
            move(n[0], n[1])
            press(btn if cmd == "down" else 0)
            time.sleep(0.15)
        elif cmd in ("click", "dclick"):
            move(n[0], n[1])
            for _ in range(2 if cmd == "dclick" else 1):
                press(btn)
                time.sleep(0.08)
                press(0)
                time.sleep(0.12)
        elif cmd == "drag":
            move(n[0], n[1])
            press(btn)
            time.sleep(0.2)
            move(n[2], n[3], frm=(n[0], n[1]))
            time.sleep(0.2)
            press(0)
            time.sleep(0.2)
        elif cmd == "key":
            vm.key(VKEYS.get(n[0], hex(n[0])))
        elif cmd == "wait":
            time.sleep(n[0] / 1000 * 2)
        elif cmd == "shot":
            # (no name: "shot", as the port names it)
            name = a[0] if a else "shot"
            p = vm.shot("_cmp")
            Image.open(p).crop((OX, OY, OX + W, OY + H)).save(os.path.join(out, name + ".png"))


# Windows virtual keys to QEMU's key names.
VKEYS = {0x0D: "ret", 0x1B: "esc", 0x20: "spc", 0x08: "backspace", 0x09: "tab", 0x2E: "delete",
         0x25: "left", 0x26: "up", 0x27: "right", 0x28: "down", 0x70: "f1", 0x74: "f5"}
VKEYS.update({0x30 + i: str(i) for i in range(10)})
VKEYS.update({0x41 + i: chr(ord("a") + i) for i in range(26)})

if __name__ == "__main__":
    main(sys.argv[1:])
