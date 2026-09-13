#!/usr/bin/env python3
"""What was opened lately, newest first, as [{name, path}].

Read from recently-used.xbel, the file every desktop writes when an application opens something.
Anything no longer there is left out.
"""
import json, os, sys, xml.etree.ElementTree as ET
from urllib.parse import unquote

home = os.path.expanduser("~")
data = os.environ.get("XDG_DATA_HOME") or os.path.join(home, ".local/share")
out = []
try:
    root = ET.parse(os.path.join(data, "recently-used.xbel")).getroot()
    for mark in root.findall("bookmark"):
        href = mark.get("href", "")
        if not href.startswith("file://"):
            continue
        path = unquote(href[7:])
        if os.path.exists(path):
            out.append({"name": os.path.basename(path), "path": path, "at": mark.get("modified", "")})
except (OSError, ET.ParseError):
    pass

out.sort(key=lambda r: r["at"], reverse=True)
json.dump([{ "name": r["name"], "path": r["path"] } for r in out[:40]], sys.stdout)
