#!/usr/bin/env python3
"""The folders this machine calls the person's own, as [{key, name, path}].

Read from the file xdg-user-dirs writes; a machine that has none still has the conventional
folders, so those are tried too. Home itself, and anything not there, is left out.
"""
import json, os, re, sys

WANTED = [("DESKTOP", "Desktop"), ("DOCUMENTS", "Documents"), ("DOWNLOAD", "Downloads"),
          ("PICTURES", "Pictures"), ("MUSIC", "Music"), ("VIDEOS", "Videos")]

home = os.path.expanduser("~")
config = os.environ.get("XDG_CONFIG_HOME") or os.path.join(home, ".config")

named = {}
try:
    with open(os.path.join(config, "user-dirs.dirs")) as f:
        for line in f:
            m = re.match(r'\s*XDG_([A-Z]+)_DIR\s*=\s*"(.*)"\s*$', line)
            if m:
                named[m.group(1)] = os.path.expandvars(m.group(2).replace("$HOME", home))
except OSError:
    pass

out = []
for key, fallback in WANTED:
    path = named.get(key) or os.path.join(home, fallback)
    if path.rstrip("/") == home.rstrip("/") or not os.path.isdir(path):
        continue
    out.append({"key": key, "name": os.path.basename(path.rstrip("/")), "path": path})

json.dump(out, sys.stdout)
