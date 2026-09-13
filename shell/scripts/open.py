#!/usr/bin/env python3
"""Open a file with whatever the desktop says handles it.

A type with no handler of its own is left to xdg-open, which guesses, and its guess for anything it
does not recognise is the browser. Text is not a thing to open in a browser, so a type that descends
from text/plain is opened with whatever opens text. The descent is read from the mime database
rather than from a list of types kept here, so a language nobody thought of still counts as text.
"""
import os, subprocess, sys

MIME_DIRS = [os.path.join(d, "mime") for d in
             [os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")]
             + (os.environ.get("XDG_DATA_DIRS") or "/usr/local/share:/usr/share").split(":")]


def ask(*args):
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=5).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return ""


def parents():
    """type -> the types it descends from, as the mime database records them."""
    out = {}
    for d in MIME_DIRS:
        try:
            with open(os.path.join(d, "subclasses")) as f:
                for line in f:
                    child, _, parent = line.partition(" ")
                    if parent:
                        out.setdefault(child.strip(), []).append(parent.strip())
        except OSError:
            pass
    return out


def descends_from(kind, ancestor, tree, seen=None):
    if kind == ancestor:
        return True
    # The spec makes every text/* type a subclass of text/plain and the database does not bother to
    # say so, which is why text/x-shellscript is recorded only as an executable.
    if ancestor == "text/plain" and kind.startswith("text/"):
        return True
    seen = seen or set()
    if kind in seen:
        return False
    seen.add(kind)
    return any(descends_from(p, ancestor, tree, seen) for p in tree.get(kind, []))


path = sys.argv[1] if len(sys.argv) > 1 else ""
if not path:
    sys.exit(2)

kind = ask("xdg-mime", "query", "filetype", path)
handler = ask("xdg-mime", "query", "default", kind) if kind else ""

if not handler and kind and descends_from(kind, "text/plain", parents()):
    handler = ask("xdg-mime", "query", "default", "text/plain")
    if handler:
        os.execvp("gtk-launch", ["gtk-launch", handler, path])

os.execvp("xdg-open", ["xdg-open", path])
