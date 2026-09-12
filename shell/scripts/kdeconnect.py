#!/usr/bin/env python3
"""KDE Connect as a phone provider (the contract: docs/ARCHITECTURE.md, "Providers"): the daemon's state,
every device it knows with its battery, and the actions on one.

    kdeconnect.py status | list | action <id>   ring, clipboard, share (a path on stdin), pair, unpair, refresh, start
"""
import json, os, shutil, subprocess, sys

CLI = shutil.which("kdeconnect-cli")
DAEMON = next((p for p in ["/usr/lib/kdeconnectd", "/usr/bin/kdeconnectd", "/usr/lib/kdeconnect/kdeconnectd", "/usr/libexec/kdeconnectd"] if os.path.exists(p)), "")


def run(*args, timeout=20, stdin=None):
    try:
        p = subprocess.run([CLI, *args], capture_output=True, text=True, timeout=timeout, input=stdin)
    except subprocess.TimeoutExpired:
        return 1, "", "kdeconnect did not answer"
    return p.returncode, p.stdout, p.stderr.strip().split("\n")[-1] if p.stderr.strip() else ""


def prop(dev, iface, name):
    p = subprocess.run(["busctl", "--user", "get-property", "org.kde.kdeconnect", "/modules/kdeconnect/devices/" + dev + ("/" + iface.split(".")[-1] if iface != "org.kde.kdeconnect.device" else ""), iface, name],
                       capture_output=True, text=True, timeout=5)
    if p.returncode != 0:
        return None
    parts = p.stdout.split(None, 1)
    return parts[1].strip().strip('"') if len(parts) == 2 else None


def daemon_up():
    p = subprocess.run(["busctl", "--user", "list"], capture_output=True, text=True, timeout=5)
    return "org.kde.kdeconnect" in p.stdout


def devices():
    code, so, se = run("-l", "--id-name-only")
    out = []
    if code != 0:
        return out
    for line in so.strip().split("\n"):
        if not line.strip():
            continue
        dev_id, _, name = line.strip().partition(" ")
        reach = prop(dev_id, "org.kde.kdeconnect.device", "isReachable") == "true"
        paired = prop(dev_id, "org.kde.kdeconnect.device", "isPaired") == "true"
        charge = prop(dev_id, "org.kde.kdeconnect.device.battery", "charge") if reach and paired else None
        charging = prop(dev_id, "org.kde.kdeconnect.device.battery", "isCharging") == "true" if charge else False
        out.append({"id": dev_id, "name": name or dev_id, "reachable": reach, "paired": paired, "charge": int(charge) if charge and charge.lstrip("-").isdigit() and int(charge) >= 0 else None, "charging": charging})
    return out


cmd = sys.argv[1] if len(sys.argv) > 1 else "status"

if cmd == "status":
    out = {"installed": bool(CLI), "ready": False, "state": "", "actions": [], "error": ""}
    if not CLI:
        out["state"] = "KDE Connect is not on this machine"
    elif not daemon_up():
        out.update(state="KDE Connect is not running", actions=[{"id": "start", "label": "Start", "primary": True}] if DAEMON else [])
    else:
        ds = devices()
        paired = [d for d in ds if d["paired"]]
        near = [d for d in paired if d["reachable"]]
        out["ready"] = True
        out["state"] = (near[0]["name"] + " is nearby" + ("  ·  %d%%" % near[0]["charge"] + (", charging" if near[0]["charging"] else "") if near[0]["charge"] is not None else "")) if near else \
                       (paired[0]["name"] + " is out of reach" if paired else "No phone paired yet; open KDE Connect on the phone on this network")
        out["actions"] = [{"id": "refresh", "label": "Look again"}]
    print(json.dumps(out))

elif cmd == "list":
    out = {"items": [], "error": ""}
    if CLI and daemon_up():
        for d in devices():
            if d["paired"]:
                sub = ("Nearby" if d["reachable"] else "Out of reach") + ("  ·  %d%%" % d["charge"] + (", charging" if d["charging"] else "") if d["charge"] is not None else "")
                acts = ([{"id": "ring", "label": "Ring"}, {"id": "clipboard", "label": "Send clipboard"}, {"id": "share", "label": "Send a file", "input": "Path to a file"}] if d["reachable"] else []) + [{"id": "unpair", "label": "Unpair", "danger": True}]
            else:
                sub = "Not paired" + ("  ·  nearby" if d["reachable"] else "")
                acts = [{"id": "pair", "label": "Pair", "primary": True}] if d["reachable"] else []
            out["items"].append({"id": d["id"], "title": d["name"], "subtitle": sub, "actions": acts})
    print(json.dumps(out))

elif cmd == "action":
    what = sys.argv[2] if len(sys.argv) > 2 else ""
    data = sys.stdin.read().strip()
    if what == "start":
        if DAEMON:
            subprocess.Popen([DAEMON], start_new_session=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        sys.exit(0)
    if what == "refresh":
        code, so, se = run("--refresh")
        sys.exit(0)
    # An item's action: the id on stdin, or "id\npath" for a share.
    dev, _, rest = data.partition("\n")
    args = {"ring": ["--ring"], "clipboard": ["--send-clipboard"], "pair": ["--pair"], "unpair": ["--unpair"], "share": ["--share", os.path.expanduser(rest.strip())] if rest.strip() else None}.get(what)
    if not dev or not args:
        sys.stderr.write("No such action, or nothing to send\n")
        sys.exit(2)
    code, so, se = run("-d", dev, *args, timeout=60)
    if code != 0:
        sys.stderr.write((se or "KDE Connect refused") + "\n")
    sys.exit(code)
