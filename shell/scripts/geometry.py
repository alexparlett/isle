#!/usr/bin/env python3
"""Prints capture geometry in slurp's "x,y wxh" form.

    geometry.py screen     the focused monitor
    geometry.py windows    every mapped window, one per line, for `slurp -r`
"""
import json, subprocess, sys

def hyprctl(*args):
    return json.loads(subprocess.run(["hyprctl", "-j", *args], capture_output=True, text=True, check=True).stdout)

mode = sys.argv[1] if len(sys.argv) > 1 else "screen"
if mode == "screen":
    m = next(m for m in hyprctl("monitors") if m["focused"])
    print(f'{m["x"]},{m["y"]} {int(m["width"] / m["scale"])}x{int(m["height"] / m["scale"])}')
else:
    for c in hyprctl("clients"):
        if c["mapped"] and c["workspace"]["id"] > 0:
            print(f'{c["at"][0]},{c["at"][1]} {c["size"][0]}x{c["size"][1]}')
