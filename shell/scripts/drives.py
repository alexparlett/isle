#!/usr/bin/env python3
"""Every drive udisks knows, with its block devices and SMART summary, as one JSON array.

    drives.py                      list
    drives.py smart <drive-path>   refresh SMART data for one drive
"""
import json, subprocess, sys

def dump():
    objects, path, iface = {}, None, None
    for line in subprocess.run(["udisksctl", "dump"], capture_output=True, text=True).stdout.splitlines():
        if not line.strip():
            continue
        if line.startswith("/"):
            path = line.rstrip(":")
            objects[path] = {}
        elif line.startswith("  ") and not line.startswith("    ") and line.endswith(":"):
            iface = line.strip().rstrip(":")
            objects[path][iface] = {}
        elif line.startswith("    ") and ":" in line and iface:
            key, _, value = line.strip().partition(":")
            objects[path][iface][key.strip()] = value.strip()
    return objects

def num(v, default=0):
    try:
        return float(v)
    except (TypeError, ValueError):
        return default

if len(sys.argv) > 2 and sys.argv[1] == "smart":
    path = sys.argv[2]
    for iface in ("org.freedesktop.UDisks2.Drive.Ata", "org.freedesktop.UDisks2.NVMe.Controller"):
        r = subprocess.run(["gdbus", "call", "--system", "--dest", "org.freedesktop.UDisks2", "--object-path", path,
                            "--method", iface + ".SmartUpdate", "{}"], capture_output=True, text=True)
        if r.returncode == 0:
            break
    sys.exit(0)

objects = dump()
drives = []
for path, ifaces in objects.items():
    d = ifaces.get("org.freedesktop.UDisks2.Drive")
    if d is None:
        continue
    ata = ifaces.get("org.freedesktop.UDisks2.Drive.Ata", {})
    nvme = ifaces.get("org.freedesktop.UDisks2.NVMe.Controller", {})
    smart = None
    if ata.get("SmartSupported") == "true":
        smart = {"kind": "ata", "failing": ata.get("SmartFailing") == "true",
                 "temp": round(num(ata.get("SmartTemperature")) - 273.15) if num(ata.get("SmartTemperature")) else None,
                 "hours": int(num(ata.get("SmartPowerOnSeconds")) / 3600), "badSectors": int(num(ata.get("SmartNumBadSectors"))),
                 "updated": int(num(ata.get("SmartUpdated"))), "selftest": ata.get("SmartSelftestStatus", "")}
    elif nvme:
        smart = {"kind": "nvme", "failing": nvme.get("SmartCriticalWarning", "[]") not in ("[]", ""),
                 "temp": round(num(nvme.get("SmartTemperature")) - 273.15) if num(nvme.get("SmartTemperature")) else None,
                 "hours": int(num(nvme.get("SmartPowerOnHours"))), "used": int(num(nvme.get("SmartPercentUsed"))),
                 "updated": int(num(nvme.get("SmartUpdated"))), "selftest": nvme.get("SmartSelftestStatus", "")}
    blocks = []
    for bpath, bifaces in objects.items():
        b = bifaces.get("org.freedesktop.UDisks2.Block")
        if not b or b.get("Drive", "'/'").strip("'") != path or "org.freedesktop.UDisks2.Partition" in bifaces:
            continue
        blocks.append({"device": b.get("Device", ""), "size": int(num(b.get("Size")))})
    drives.append({
        "path": path, "model": (d.get("Vendor", "") + " " + d.get("Model", "")).strip() or path.rsplit("/", 1)[-1],
        "serial": d.get("Serial", ""), "size": int(num(d.get("Size"))), "removable": d.get("Removable") == "true",
        "ejectable": d.get("Ejectable") == "true", "media": d.get("MediaAvailable") != "false",
        "bus": d.get("ConnectionBus", ""), "rotation": int(num(d.get("RotationRate"), -1)),
        "smart": smart, "devices": [b["device"] for b in blocks], "sortKey": d.get("SortKey", ""),
    })
drives.sort(key=lambda x: x["sortKey"])
print(json.dumps(drives))
