#!/usr/bin/env python3
"""Proton Pass through pass-cli, as a vault provider (the contract: docs/ARCHITECTURE.md, "Password manager
providers"). The shell knows nothing of Proton: the provider says its state in words and names its actions.

    protonpass.py status                       {installed, ready, state, actions: [{id, label, input, terminal, primary}], error}
    protonpass.py list                         {items: [{id, shareId, vault, title, type}], error}
    protonpass.py field <share-id> <item-id> <field>   the value on stdout: username, email, password, totp, urls
    protonpass.py action <id>                  login (in a terminal), unlock (the lock code on stdin), logout

Secrets never come through `list`: titles only, and a field is fetched when asked for.
"""
import json, re, shutil, subprocess, sys

CLI = shutil.which("pass-cli")
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def run(*args, stdin=None, timeout=40):
    try:
        p = subprocess.run([CLI, *args], capture_output=True, text=True, input=stdin, timeout=timeout)
    except subprocess.TimeoutExpired:
        return 1, "", "pass-cli did not answer in time"
    # The CLI logs to stderr in colour above its error line; the last "Error:" line is the one that says why.
    err = ANSI.sub("", p.stderr)
    lines = [l.strip() for l in err.split("\n") if l.strip()]
    said = next((l for l in reversed(lines) if l.startswith("Error")), lines[-1] if lines else "")
    return p.returncode, p.stdout, said


def locked(err):
    return "lock" in err.lower() and "unlock" in err.lower() or "session is locked" in err.lower()


def signed_out(err):
    return "authenticated client" in err or "not logged in" in err.lower() or "no session" in err.lower()


LOGIN = {"id": "login", "label": "Sign in", "terminal": True, "primary": True}
UNLOCK = {"id": "unlock", "label": "Unlock", "input": "Lock code", "primary": True}
LOGOUT = {"id": "logout", "label": "Sign out"}

cmd = sys.argv[1] if len(sys.argv) > 1 else "status"

if cmd == "status":
    out = {"installed": bool(CLI), "ready": False, "state": "", "actions": [], "error": ""}
    if not CLI:
        out["state"] = "pass-cli is not on this machine"
    else:
        code, so, se = run("info", "--output", "json")
        if code == 0:
            try:
                info = json.loads(so)
                who = info.get("email") or info.get("username") or ""
                lock = info.get("session_lock_after_seconds")
                out.update(ready=True, state="Signed in" + (" as " + who if who else "") + ("  ·  locks after %d minutes idle" % (lock // 60) if lock else ""), actions=[LOGOUT])
            except ValueError:
                out["error"] = "Unreadable answer from pass-cli"
        elif locked(se):
            out.update(state="Locked", actions=[UNLOCK, LOGOUT])
        elif signed_out(se):
            out.update(state="Not signed in", actions=[LOGIN])
        else:
            out.update(state="pass-cli did not answer", error=se[:160], actions=[LOGIN])
    print(json.dumps(out))

elif cmd == "list":
    out = {"items": [], "error": ""}
    if not CLI:
        out["error"] = "pass-cli is not installed"
    else:
        code, so, se = run("vault", "list", "--output", "json")
        if code != 0:
            out["error"] = se[:160]
        else:
            for v in json.loads(so).get("vaults", []):
                code, so, se = run("item", "list", "--share-id", v["share_id"], "--filter-state", "active", "--output", "json")
                if code != 0:
                    out["error"] = se[:160]
                    continue
                for it in json.loads(so).get("items", []):
                    out["items"].append({"id": it["id"], "shareId": it["share_id"], "vault": v["name"], "title": it.get("title") or "(untitled)", "type": it.get("item_type", "")})
            out["items"].sort(key=lambda i: i["title"].lower())
    print(json.dumps(out))

elif cmd == "field":
    share, item, field = sys.argv[2:5]
    # pass-cli reports an empty field as missing; a login's identity is its username or, failing that, its email.
    tries = {"username": ["username", "email"], "email": ["email", "username"]}.get(field, [field])
    for f in tries:
        code, so, se = run("item", "view", "--share-id", share, "--item-id", item, "--field", f)
        if code == 0 and so.strip():
            sys.stdout.write(so.rstrip("\n"))
            sys.exit(0)
    sys.stderr.write((se or "No " + field) + "\n")
    sys.exit(1)

elif cmd == "action":
    what = sys.argv[2] if len(sys.argv) > 2 else ""
    if what == "login":
        # In a terminal: pass-cli prints a browser address, or prompts.
        sys.exit(subprocess.call([CLI, "login"]))
    elif what == "unlock":
        code = sys.stdin.read().strip()
        rc, so, se = run("session", "unlock", stdin=code + "\n")
        if rc != 0:
            sys.stderr.write((se or "The lock code was not accepted") + "\n")
        sys.exit(rc)
    elif what == "logout":
        rc, so, se = run("logout")
        if rc != 0:
            sys.stderr.write(se + "\n")
        sys.exit(rc)
    else:
        sys.stderr.write("No such action: " + what + "\n")
        sys.exit(2)
