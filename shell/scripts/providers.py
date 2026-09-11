#!/usr/bin/env python3
"""The providers of one kind (vaults, vpns): the shell's own manifests under shell/<kind>, then the user's
under ~/.config/isle/<kind>/<name>/provider.json, as one JSON list with every script path made absolute.

    providers.py <shell-dir> <kind>
"""
import json, os, sys

shell = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
kind = sys.argv[2] if len(sys.argv) > 2 else "vaults"
user = os.path.join(os.environ.get("XDG_CONFIG_HOME", os.path.expanduser("~/.config")), "isle", kind)
out, seen = [], set()


def add(path, base, own):
    try:
        m = json.load(open(path))
    except (OSError, ValueError):
        return
    if not isinstance(m, dict) or not m.get("id") or not m.get("script") or m["id"] in seen:
        return
    script = m["script"] if os.path.isabs(m["script"]) else os.path.join(base, m["script"])
    if not os.path.isfile(script):
        return
    seen.add(m["id"])
    entry = {"id": m["id"], "name": m.get("name") or m["id"], "note": m.get("note") or "", "script": script, "user": not own,
             "keychain": m.get("keychain", True) is not False}
    for k, v in m.items():
        if k not in entry and k != "script":
            entry[k] = v
    out.append(entry)


own_dir = os.path.join(shell, kind)
for f in sorted(os.listdir(own_dir)) if os.path.isdir(own_dir) else []:
    if f.endswith(".json"):
        add(os.path.join(own_dir, f), os.path.join(shell, "scripts"), True)
if os.path.isdir(user):
    for d in sorted(os.listdir(user)):
        add(os.path.join(user, d, "provider.json"), os.path.join(user, d), False)
print(json.dumps(out))
