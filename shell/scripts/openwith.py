#!/usr/bin/env python3
"""Which applications can open a file, and opening it with one of them.

    openwith.py list <path>        {"kind", "default": id, "apps": [{id, name}]}
    openwith.py open <path>        open with the default; exit 3 when there is none
    openwith.py with <id> <path>   open with that application

A type with no handler of its own but which is text is opened with whatever opens text: the spec
makes every text/* type a subclass of text/plain and the database does not record it. Anything else
with no handler is not guessed at — xdg-open's guess is the browser, which is never right.
"""
import json, os, subprocess, sys

MIME_DIRS = [os.path.join(d, "mime") for d in
             [os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")]
             + (os.environ.get("XDG_DATA_DIRS") or "/usr/local/share:/usr/share").split(":")]
APP_DIRS = [os.path.join(d, "applications") for d in
            [os.environ.get("XDG_DATA_HOME") or os.path.expanduser("~/.local/share")]
            + (os.environ.get("XDG_DATA_DIRS") or "/usr/local/share:/usr/share").split(":")]


def ask(*args):
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=5).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return ""


def kind_of(path):
    return ask("xdg-mime", "query", "filetype", path)


def is_text(kind):
    if kind.startswith("text/"):
        return True
    parents = {}
    for d in MIME_DIRS:
        try:
            with open(os.path.join(d, "subclasses")) as f:
                for line in f:
                    child, _, parent = line.partition(" ")
                    parents.setdefault(child.strip(), []).append(parent.strip())
        except OSError:
            pass

    seen, stack = set(), [kind]
    while stack:
        one = stack.pop()
        if one == "text/plain":
            return True
        if one in seen:
            continue
        seen.add(one)
        stack.extend(parents.get(one, []))
    return False


def default_for(kind):
    handler = ask("xdg-mime", "query", "default", kind)
    if not handler and is_text(kind):
        handler = ask("xdg-mime", "query", "default", "text/plain")
    return handler


def name_of(entry_id):
    """The application's own name, from its desktop entry."""
    for d in APP_DIRS:
        path = os.path.join(d, entry_id)
        if not os.path.exists(path):
            continue
        try:
            with open(path) as f:
                for line in f:
                    if line.startswith("Name="):
                        return line[5:].strip()
        except OSError:
            pass
    return entry_id.removesuffix(".desktop")


def candidates(kind):
    """Everything that says it handles the type, the desktop's own answer first."""
    out, seen = [], set()
    for one in [default_for(kind)] + ask("gio", "mime", kind).split():
        if one.endswith(".desktop") and one not in seen:
            seen.add(one)
            out.append({"id": one, "name": name_of(one)})
    return out


def launch(entry_id, path):
    os.execvp("gtk-launch", ["gtk-launch", entry_id, path])


what = sys.argv[1] if len(sys.argv) > 1 else ""
if what == "list" and len(sys.argv) > 2:
    kind = kind_of(sys.argv[2])
    json.dump({"kind": kind, "default": default_for(kind), "apps": candidates(kind)}, sys.stdout)
elif what == "open" and len(sys.argv) > 2:
    kind = kind_of(sys.argv[2])
    handler = default_for(kind)
    if not handler:
        sys.exit(3)
    launch(handler, sys.argv[2])
elif what == "with" and len(sys.argv) > 3:
    launch(sys.argv[2], sys.argv[3])
elif what == "always" and len(sys.argv) > 3:
    # The choice becomes the standing one for the type, which is what xdg-mime keeps.
    subprocess.run(["xdg-mime", "default", sys.argv[2], kind_of(sys.argv[3])])
    launch(sys.argv[2], sys.argv[3])
else:
    sys.exit(2)
