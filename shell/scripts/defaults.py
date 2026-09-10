#!/usr/bin/env python3
"""Default applications by MIME type.

    defaults.py list                     {"browser": id, "mimes": {mime: id}, "apps": {id: {name, icon, mimes}}}
    defaults.py set <mime> <desktop-id>
    defaults.py set-browser <desktop-id>
"""
import configparser, json, os, subprocess, sys

MIMES = ["x-scheme-handler/http", "x-scheme-handler/https", "text/html", "x-scheme-handler/mailto", "inode/directory",
         "text/plain", "image/png", "image/jpeg", "video/mp4", "video/x-matroska", "audio/mpeg", "audio/flac",
         "application/pdf", "application/zip"]

def app_dirs():
    home = os.path.expanduser("~")
    dirs = [os.path.join(os.environ.get("XDG_DATA_HOME", home + "/.local/share"), "applications")]
    dirs += [os.path.join(d, "applications") for d in os.environ.get("XDG_DATA_DIRS", "/usr/local/share:/usr/share").split(":")]
    dirs += [home + "/.local/share/flatpak/exports/share/applications", "/var/lib/flatpak/exports/share/applications"]
    return dirs

def apps():
    out = {}
    for d in app_dirs():
        for root, _, files in os.walk(d):
            for f in files:
                if not f.endswith(".desktop"):
                    continue
                app_id = os.path.relpath(os.path.join(root, f), d).replace("/", "-")
                if app_id in out:
                    continue
                cp = configparser.RawConfigParser(strict=False, interpolation=None)
                try:
                    cp.read(os.path.join(root, f), encoding="utf-8")
                    e = cp["Desktop Entry"]
                except Exception:
                    continue
                mimes = [m for m in e.get("MimeType", "").split(";") if m]
                if not mimes:
                    continue
                out[app_id] = {"name": e.get("Name", app_id), "icon": e.get("Icon", ""), "mimes": mimes}
    return out

cmd = sys.argv[1] if len(sys.argv) > 1 else "list"
if cmd == "set":
    subprocess.run(["xdg-mime", "default", sys.argv[3], sys.argv[2]])
elif cmd == "set-browser":
    subprocess.run(["xdg-settings", "set", "default-web-browser", sys.argv[2]])
else:
    mimes = {}
    for m in MIMES:
        r = subprocess.run(["xdg-mime", "query", "default", m], capture_output=True, text=True)
        mimes[m] = r.stdout.strip()
    browser = subprocess.run(["xdg-settings", "get", "default-web-browser"], capture_output=True, text=True).stdout.strip()
    print(json.dumps({"browser": browser, "mimes": mimes, "apps": apps()}))
