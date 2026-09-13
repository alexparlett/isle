#!/usr/bin/env python3
"""What a desktop entry says that Quickshell's DesktopEntry does not carry.

    entries.py single

`single` lists the ids of entries that say they have one main window, which the launcher raises rather than
starting again. An app that does not say so is one where a second window is a thing a person asks for, a
file manager or an editor, and starting it again is what they meant. Two keys say it: KDE's
`SingleMainWindow` and GNOME's `X-GNOME-SingleWindow`.
"""
import json, os, sys


def app_dirs():
    home = os.path.expanduser("~")
    dirs = [os.path.join(os.environ.get("XDG_DATA_HOME", home + "/.local/share"), "applications")]
    dirs += [os.path.join(d, "applications") for d in os.environ.get("XDG_DATA_DIRS", "/usr/local/share:/usr/share").split(":")]
    dirs += [home + "/.local/share/flatpak/exports/share/applications", "/var/lib/flatpak/exports/share/applications"]
    return [d for d in dirs if os.path.isdir(d)]


def single():
    """The ids that declare one main window. The first directory to carry an id wins, as XDG says."""
    out, seen = [], set()
    for d in app_dirs():
        for name in os.listdir(d):
            if not name.endswith(".desktop") or name in seen:
                continue
            seen.add(name)
            try:
                with open(os.path.join(d, name), errors="ignore") as f:
                    # Only the first group is the entry's own; an action's keys are not it.
                    body = f.read().split("\n[", 1)[0]
            except OSError:
                continue
            for line in body.splitlines():
                flat = line.replace(" ", "").lower()
                if flat in ("singlemainwindow=true", "x-gnome-singlewindow=true"):
                    out.append(name[:-len(".desktop")])
                    break
    return sorted(out)


cmd = sys.argv[1] if len(sys.argv) > 1 else "single"
if cmd == "single":
    print(json.dumps({"single": single()}))
else:
    sys.stderr.write("No such command\n")
    sys.exit(2)
