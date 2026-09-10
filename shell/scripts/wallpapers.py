#!/usr/bin/env python3
"""Images in the wallpaper folders as JSON: [{ path, name, folder }]. Extra folders come as arguments."""
import json, os, sys

HOME = os.path.expanduser("~")
folders = [HOME + "/Pictures/Wallpapers", HOME + "/Pictures/wallpapers", HOME + "/.local/share/wallpapers", "/usr/share/wallpapers", "/usr/share/backgrounds", "/usr/share/hypr"] + sys.argv[1:]
EXT = (".png", ".jpg", ".jpeg", ".webp", ".avif")
out, seen = [], set()
for folder in folders:
    if not os.path.isdir(folder):
        continue
    for root, dirs, files in os.walk(folder):
        if root[len(folder):].count("/") > 2:
            dirs[:] = []
        for f in sorted(files):
            if not f.lower().endswith(EXT):
                continue
            p = os.path.join(root, f)
            if p in seen:
                continue
            seen.add(p)
            out.append({"path": p, "name": os.path.splitext(f)[0].replace("_", " ").replace("-", " "), "folder": folder.replace(HOME, "~")})
print(json.dumps(out))
