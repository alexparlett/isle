#!/usr/bin/env python3
"""Snapper as a snapshots provider (the contract: docs/ARCHITECTURE.md, "Providers"). Reading needs the user
in the config's ALLOW_USERS, which the Allow action sets once through polkit; a rollback is root's.

    snapper.py status | list | action create (a description on stdin) | delete | rollback (a number on stdin) | allow
"""
import json, os, shutil, subprocess, sys

CLI = shutil.which("snapper")
CONFIG = "root"
USER = os.environ.get("USER") or os.getlogin()


def run(*args, root=False, stdin=None, timeout=120):
    cmd = (["pkexec", CLI] if root else [CLI]) + list(args)
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, input=stdin, timeout=timeout)
    except subprocess.TimeoutExpired:
        return 124, "", "snapper did not answer"
    return p.returncode, p.stdout, p.stderr.strip().split("\n")[-1] if p.stderr.strip() else ""


def snapshots():
    code, so, se = run("--jsonout", "-c", CONFIG, "list")
    if code != 0:
        return None, se
    try:
        return json.loads(so).get(CONFIG, []), ""
    except ValueError:
        return None, "Unreadable answer from snapper"


cmd = sys.argv[1] if len(sys.argv) > 1 else "status"

if cmd == "status":
    out = {"installed": bool(CLI) and os.path.exists("/etc/snapper/configs/" + CONFIG), "ready": False, "state": "", "actions": [], "error": ""}
    if not CLI:
        out["state"] = "snapper is not on this machine"
    elif not out["installed"]:
        out["state"] = "No snapper config for the root"
    else:
        snaps, err = snapshots()
        if snaps is None:
            if "permission" in err.lower():
                out.update(state="Not allowed to read the snapshots as you", actions=[{"id": "allow", "label": "Allow me", "primary": True}])
            else:
                out.update(state="snapper did not answer", error=err)
        else:
            n = len([s for s in snaps if s.get("number")])
            out.update(ready=True, state="%d snapshot%s of the root" % (n, "" if n == 1 else "s"),
                       actions=[{"id": "create", "label": "Take a snapshot", "input": "Description", "primary": True}])
    print(json.dumps(out))

elif cmd == "list":
    out = {"items": [], "error": ""}
    snaps, err = snapshots()
    if snaps is None:
        out["error"] = err
    else:
        for s in reversed(snaps):
            n = s.get("number")
            if not n:
                continue
            kind = {"pre": "before a change", "post": "after a change", "single": "single"}.get(s.get("type"), s.get("type", ""))
            when = (s.get("date") or "")[:16]
            out["items"].append({"id": str(n), "title": s.get("description") or "Snapshot %d" % n,
                                 "subtitle": "  ·  ".join(x for x in ["#%d" % n, when, kind, s.get("cleanup") or ""] if x),
                                 "actions": [{"id": "rollback", "label": "Roll back", "danger": True}, {"id": "delete", "label": "Delete", "danger": True}]})
    print(json.dumps(out))

elif cmd == "action":
    what = sys.argv[2] if len(sys.argv) > 2 else ""
    data = sys.stdin.read().strip()
    if what == "allow":
        code, so, se = run("-c", CONFIG, "set-config", "ALLOW_USERS=" + USER, "SYNC_ACL=yes", root=True)
    elif what == "create":
        code, so, se = run("-c", CONFIG, "create", "--description", data or "Taken from Isle", "--cleanup-algorithm", "number")
    elif what == "delete":
        code, so, se = run("-c", CONFIG, "delete", data)
    elif what == "rollback":
        code, so, se = run("-c", CONFIG, "rollback", data, root=True)
        if code == 0:
            sys.stdout.write("Rolled back to snapshot %s; the next boot uses it\n" % data)
    else:
        sys.stderr.write("No such action: " + what + "\n")
        sys.exit(2)
    if code != 0:
        sys.stderr.write((se or "snapper failed") + "\n")
    sys.exit(code)
