#!/usr/bin/env python3
"""The user's systemd units that start with the session, and switching them.

    services.py list                  [{id, name, enabled, active}]
    services.py enable <unit>         enable --now
    services.py disable <unit>        disable --now

Listed: the enabled services and sockets, which is what starts at login. Units that only activate on
demand (D-Bus, gpg, gvfs, the portals) are infrastructure, not choices, and stay out.
"""
import json, subprocess, sys

def run(*args):
    return subprocess.run(["systemctl", "--user", *args], capture_output=True, text=True).stdout

def noise(unit):
    return unit.startswith(("dbus-", "systemd-", "app-", "run-")) or "@" in unit

cmd = sys.argv[1] if len(sys.argv) > 1 else "list"
if cmd in ("enable", "disable"):
    subprocess.run(["systemctl", "--user", cmd, "--now", sys.argv[2]], capture_output=True)
    sys.exit(0)

units = {}
for line in run("list-unit-files", "--type=service,socket", "--no-legend", "--plain").splitlines():
    parts = line.split()
    if len(parts) < 2:
        continue
    unit, state = parts[0], parts[1]
    if noise(unit) or state not in ("enabled", "enabled-runtime"):
        continue
    units[unit] = {"id": unit, "enabled": True, "active": False, "name": unit}
for line in run("list-units", "--type=service,socket", "--no-legend", "--plain", "--state=active").splitlines():
    parts = line.split(None, 4)
    if len(parts) >= 5 and parts[0] in units:
        units[parts[0]]["active"] = True
        units[parts[0]]["name"] = parts[4]
for unit, u in units.items():
    if u["name"] == unit:
        d = run("show", "-p", "Description", "--value", unit).strip()
        if d:
            u["name"] = d
print(json.dumps(sorted(units.values(), key=lambda u: u["name"].lower())))
