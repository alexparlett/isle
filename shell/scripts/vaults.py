#!/usr/bin/env python3
"""The vault providers: the shell's own manifests under shell/vaults, then the user's under
~/.config/isle/vaults/<name>/provider.json, as one JSON list with every script path made absolute."""
import json, os, sys

shell = sys.argv[1] if len(sys.argv) > 1 else os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
user = os.path.join(os.environ.get("XDG_CONFIG_HOME", os.path.expanduser("~/.config")), "isle", "vaults")
out, seen = [], set()


def add(path, base):
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
    out.append({"id": m["id"], "name": m.get("name") or m["id"], "note": m.get("note") or "", "script": script, "keychain": m.get("keychain", True) is not False, "user": base != os.path.join(shell, "scripts")})


for f in sorted(os.listdir(os.path.join(shell, "vaults"))) if os.path.isdir(os.path.join(shell, "vaults")) else []:
    if f.endswith(".json"):
        add(os.path.join(shell, "vaults", f), os.path.join(shell, "scripts"))
if os.path.isdir(user):
    for d in sorted(os.listdir(user)):
        add(os.path.join(user, d, "provider.json"), os.path.join(user, d))
print(json.dumps(out))
