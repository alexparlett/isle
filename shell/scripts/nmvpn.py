#!/usr/bin/env python3
"""NetworkManager's VPN, WireGuard and tunnel connections, as a VPN provider (the contract:
docs/ARCHITECTURE.md, "Providers").

    nmvpn.py status | list | action connect (a connection name on stdin) | action disconnect
"""
import json, shutil, subprocess, sys

CLI = shutil.which("nmcli")


def connections():
    # Saved connections only: one kept under /run is another client's tunnel of the moment (Proton VPN's
    # WireGuard link, say), which that client's provider reports and owns.
    p = subprocess.run([CLI, "-t", "-f", "NAME,TYPE,ACTIVE,FILENAME", "connection", "show"], capture_output=True, text=True, timeout=20)
    out = []
    for line in p.stdout.strip().split("\n"):
        f = line.replace("\\:", "\x00").split(":")
        if len(f) >= 4 and f[1] in ("vpn", "wireguard", "tun") and f[3] and not f[3].startswith("/run/"):
            out.append({"name": f[0].replace("\x00", ":"), "type": f[1], "active": f[2] == "yes"})
    return out


cmd = sys.argv[1] if len(sys.argv) > 1 else "status"

if cmd == "status":
    out = {"installed": bool(CLI), "ready": False, "state": "", "active": None, "actions": [], "error": ""}
    if not CLI:
        out["state"] = "nmcli is not on this machine"
    else:
        cs = connections()
        on = next((c for c in cs if c["active"]), None)
        out["ready"] = len(cs) > 0
        if on:
            out["active"] = {"name": on["name"], "detail": "WireGuard" if on["type"] == "wireguard" else "NetworkManager"}
            out["state"] = "Connected to " + on["name"]
            out["actions"] = [{"id": "disconnect", "label": "Disconnect", "primary": True}]
        elif cs:
            out["state"] = "%d connection%s" % (len(cs), "" if len(cs) == 1 else "s")
            out["actions"] = [{"id": "connect", "label": "Connect", "primary": True}]
        else:
            out["state"] = "No VPN connections set up"
    print(json.dumps(out))

elif cmd == "list":
    out = {"choices": [], "error": ""}
    if CLI:
        for c in connections():
            out["choices"].append({"id": c["name"], "title": c["name"], "subtitle": ("WireGuard" if c["type"] == "wireguard" else "NetworkManager") + ("  ·  connected" if c["active"] else "")})
    print(json.dumps(out))

elif cmd == "action":
    what = sys.argv[2] if len(sys.argv) > 2 else ""
    cs = connections()
    if what == "connect":
        name = sys.stdin.read().strip() or (cs[0]["name"] if cs else "")
        p = subprocess.run([CLI, "connection", "up", name], capture_output=True, text=True, timeout=90)
    elif what == "disconnect":
        on = next((c for c in cs if c["active"]), None)
        p = subprocess.run([CLI, "connection", "down", on["name"]], capture_output=True, text=True, timeout=60) if on else None
        if p is None:
            sys.exit(0)
    else:
        sys.stderr.write("No such action: " + what + "\n")
        sys.exit(2)
    if p.returncode != 0:
        sys.stderr.write((p.stderr.strip().split("\n")[-1] if p.stderr.strip() else "NetworkManager failed") + "\n")
    sys.exit(p.returncode)
