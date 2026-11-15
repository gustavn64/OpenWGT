#!/usr/bin/env python3
"""Plays a comparison script (see crates/engine/examples/script.rs) into
the original in the Windows 95 VM (~/WidgetWorkshop-vm, vm.py), from the
snapshot with the editor open, and saves a screenshot of the editor's
area at each `shot`.

    run_vm.py SCRIPT OUT_DIR [--snapshot ww-editor]
"""
import os
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


def main(argv):
    script, out = argv[0], argv[1]
    snap = argv[argv.index("--snapshot") + 1] if "--snapshot" in argv else "ww-editor"
    os.makedirs(out, exist_ok=True)
    vm.hmp(f"loadvm {snap}")
    time.sleep(2)
    vm.home()
    vm.mouse_to(OX + 320, OY + 380)
    time.sleep(1)
    for line in open(script):
        w = line.split("#")[0].split()
        if not w:
            continue
        cmd, a = w[0], w[1:]
        n = []
        for v in a:
            try:
                n.append(int(v, 0))
            except ValueError:
                pass
        btn = 2 if "right" in a else 1
        if cmd in ("size", "dump"):
            continue
        if cmd == "move":
            vm.mouse_to(OX + n[0], OY + n[1])
        elif cmd in ("down", "up"):
            vm.mouse_to(OX + n[0], OY + n[1])
            vm.hmp(f"mouse_button {btn if cmd == 'down' else 0}")
            time.sleep(0.15)
        elif cmd in ("click", "dclick"):
            vm.click(OX + n[0], OY + n[1], button=btn, double=cmd == "dclick")
        elif cmd == "drag":
            vm.drag(OX + n[0], OY + n[1], OX + n[2], OY + n[3], button=btn)
        elif cmd == "key":
            vm.key(VKEYS.get(n[0], hex(n[0])))
        elif cmd == "wait":
            time.sleep(n[0] / 1000 * 2)
        elif cmd == "shot":
            p = vm.shot("_cmp")
            Image.open(p).crop((OX, OY, OX + W, OY + H)).save(os.path.join(out, a[0] + ".png"))
        else:
            raise SystemExit(f"unknown command {cmd}")


# Windows virtual keys to QEMU's key names.
VKEYS = {0x0D: "ret", 0x1B: "esc", 0x20: "spc", 0x08: "backspace", 0x09: "tab", 0x2E: "delete",
         0x25: "left", 0x26: "up", 0x27: "right", 0x28: "down", 0x70: "f1", 0x74: "f5"}
VKEYS.update({0x30 + i: str(i) for i in range(10)})
VKEYS.update({0x41 + i: chr(ord("a") + i) for i in range(26)})

if __name__ == "__main__":
    main(sys.argv[1:])
