#!/usr/bin/env python3
"""The physical keyboards the kernel knows, as JSON: name, bus, vendor, the compositor's slug for the device,
what its key bitmap says about its size, and a guess at the XKB model.

    keyboards.py [first-layout]     the layout decides ISO (105) or ANSI (104) for the generic guess
"""
import json, re, subprocess, sys

ISO_ANSI = {"us": "pc104", "jp": "jp106", "kr": "kr106", "br": "abnt2"}
KEY_KP1, KEY_F1, KEY_HOME = 79, 59, 102
layout = sys.argv[1] if len(sys.argv) > 1 else "us"


def bits(words):
    # /proc lists the bitmap high word first, 64 bits a word.
    out = set()
    for i, w in enumerate(reversed(words)):
        v = int(w, 16)
        for b in range(64):
            if v >> b & 1:
                out.add(i * 64 + b)
    return out


def udev(path, key):
    try:
        for line in subprocess.run(["udevadm", "info", "-q", "property", "-p", path], capture_output=True, text=True, timeout=5).stdout.splitlines():
            if line.startswith(key + "="):
                return line.split("=", 1)[1]
    except Exception:
        pass
    return ""


out = []
for block in open("/proc/bus/input/devices").read().split("\n\n"):
    f = {}
    for line in block.splitlines():
        m = re.match(r"^([A-Z]): (.*)$", line)
        if m:
            f.setdefault(m.group(1), []).append(m.group(2))
    if not f or "kbd" not in " ".join(f.get("H", [])):
        continue
    name = f.get("N", [""])[0].strip('Name=').strip('"')
    keys = set()
    for b in f.get("B", []):
        if b.startswith("KEY="):
            keys = bits(b[4:].split())
    letters = sum(1 for k in range(16, 51) if k in keys)  # q..m
    if letters < 20:
        continue  # a power button, a mouse's media keys, a headset
    ident = dict(p.split("=") for p in f.get("I", [""])[0].split())
    vendor_id, product_id = ident.get("Vendor", "0000"), ident.get("Product", "0000")
    sysfs = "/sys" + f.get("S", ["Sysfs="])[0].split("=", 1)[1]
    bus = {"0003": "USB", "0005": "Bluetooth", "0011": "PS/2", "0019": "Built in", "0006": "Virtual"}.get(ident.get("Bus", ""), "")
    if bus == "USB" and "bluetooth" in sysfs.lower():
        bus = "Bluetooth"
    vendor = udev(sysfs, "ID_VENDOR_FROM_DATABASE") or udev(sysfs, "ID_VENDOR").replace("_", " ")
    reported = sum(1 for k in keys if 1 <= k < 256)
    # A USB keyboard's descriptor usually claims every usage, which says nothing about its size.
    if reported >= 200:
        size = "reports every key"
    elif KEY_F1 not in keys:
        size = "60% guessed"
    elif KEY_KP1 not in keys:
        size = "tenkeyless guessed" if KEY_HOME in keys else "75% guessed"
    else:
        size = "full size guessed"
    apple = vendor_id.lower() == "05ac" or "apple" in name.lower()
    model = ("applealu_iso" if layout not in ISO_ANSI else "applealu_ansi") if apple else ISO_ANSI.get(layout, "pc105")
    slug = re.sub(r"[^a-z0-9]", "-", name.lower())
    if any(o["slug"] == slug for o in out):
        continue
    out.append({"name": name, "slug": slug, "bus": bus, "vendor": vendor, "vendorId": vendor_id, "productId": product_id,
                "size": size, "model": model})
print(json.dumps(out))
