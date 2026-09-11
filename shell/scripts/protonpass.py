#!/usr/bin/env python3
"""Proton Pass through pass-cli: the account's state, every vault's items, one field of one item.

    protonpass.py status                       {installed, loggedIn, locked, email, username, hasLock, error}
    protonpass.py list                         {items: [{id, shareId, vault, title, type}], error}
    protonpass.py field <share-id> <item-id> <field>   the value on stdout: username, email, password, totp, urls
    protonpass.py unlock                       the lock code on stdin
    protonpass.py logout

Secrets never come through `list`: titles only, and a field is fetched when asked for. This is the vault
provider contract (docs/ARCHITECTURE.md, "Password manager providers"): a script of your own with the same
four answers, under ~/.config/isle/vaults/<name>/, is a provider too.
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


cmd = sys.argv[1] if len(sys.argv) > 1 else "status"

if cmd == "status":
    out = {"installed": bool(CLI), "loggedIn": False, "locked": False, "email": "", "username": "", "hasLock": False, "error": ""}
    if CLI:
        code, so, se = run("info", "--output", "json")
        if code == 0:
            try:
                info = json.loads(so)
                out.update(loggedIn=True, email=info.get("email") or "", username=info.get("username") or "", hasLock=bool(info.get("session_has_lock")))
            except ValueError:
                out["error"] = "Unreadable answer from pass-cli"
        elif locked(se):
            out.update(loggedIn=True, locked=True, hasLock=True)
        elif not signed_out(se):
            out["error"] = se[:160]
    print(json.dumps(out))

elif cmd == "list":
    out = {"items": [], "error": ""}
    if not CLI:
        out["error"] = "pass-cli is not installed"
    else:
        code, so, se = run("vault", "list", "--output", "json")
        if code != 0:
            out["error"] = "locked" if locked(se) else "signed out" if signed_out(se) else se[:160]
        else:
            vaults = json.loads(so).get("vaults", [])
            for v in vaults:
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
    code, so, se = run("item", "view", "--share-id", share, "--item-id", item, "--field", field)
    if code != 0:
        sys.stderr.write(se + "\n")
        sys.exit(1)
    sys.stdout.write(so.rstrip("\n"))

elif cmd == "unlock":
    code = sys.stdin.read().strip()
    rc, so, se = run("session", "unlock", stdin=code + "\n")
    if rc != 0:
        sys.stderr.write(se + "\n")
    sys.exit(rc)

elif cmd == "logout":
    rc, so, se = run("logout")
    if rc != 0:
        sys.stderr.write(se + "\n")
    sys.exit(rc)
