#!/usr/bin/env python3
"""Cut a region out of the frozen frames.

    crop.py FREEZE_DIR "x,y wxh" OUT.png

FREEZE_DIR holds one PNG per monitor and index.json with each monitor's logical box and scale, as
capture.sh wrote them. The region is in logical layout coordinates, as slurp prints it.
"""
import json, os, sys
from PIL import Image

d, geom, out = sys.argv[1], sys.argv[2], sys.argv[3]
pos, size = geom.split()
gx, gy = (int(v) for v in pos.split(","))
gw, gh = (int(v) for v in size.split("x"))
mons = json.load(open(os.path.join(d, "index.json")))
mon = next((m for m in mons if m["x"] <= gx < m["x"] + m["w"] and m["y"] <= gy < m["y"] + m["h"]), None)
if not mon:
    sys.exit(1)
s = mon["scale"]
im = Image.open(os.path.join(d, mon["file"]))
box = (round((gx - mon["x"]) * s), round((gy - mon["y"]) * s), round((gx - mon["x"] + gw) * s), round((gy - mon["y"] + gh) * s))
im.crop((max(box[0], 0), max(box[1], 0), min(box[2], im.width), min(box[3], im.height))).save(out)
