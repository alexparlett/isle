#!/usr/bin/env python3
"""The machine's hardware as a tree, for the Devices page: categories the way Device Manager names them,
each device with the properties a person might look up. Monitors, audio endpoints, Bluetooth peers and
batteries are the shell's own knowledge and are added there.
"""
import json, os, re, subprocess


def run(*args):
    try:
        return subprocess.run(args, capture_output=True, text=True, timeout=10).stdout
    except (OSError, subprocess.TimeoutExpired):
        return ""


def clean(s):
    return re.sub(r"\s+", " ", (s or "").replace("_", " ")).strip()


def vendor(s):
    # "Advanced Micro Devices, Inc. [AMD]" reads as AMD; corporate suffixes carry nothing.
    s = clean(s)
    m = re.search(r"\[([^\]]+)\]$", s)
    if m:
        return m.group(1)
    return re.sub(r",?\s*(Inc\.?|Corp\.?|Corporation|Co\.,? Ltd\.?|Ltd\.?|GmbH|LLC|Limited|Technology|Semiconductor)\s*$", "", s).strip(" ,")


def props(*pairs):
    return [[k, str(v)] for k, v in pairs if v not in (None, "", [])]


def udev_db():
    out, cur = [], {}
    for line in run("udevadm", "info", "-e").splitlines():
        if not line.strip():
            if cur:
                out.append(cur)
            cur = {}
        elif line.startswith("E: ") and "=" in line:
            k, v = line[3:].split("=", 1)
            cur[k] = v
        elif line.startswith("P: "):
            cur["_path"] = line[3:]
    if cur:
        out.append(cur)
    return out


BUS = {"usb": "USB", "bluetooth": "Bluetooth", "i8042": "PS/2", "platform": "Built in", "pci": "PCI", "i2c": "I²C", "virtual": "Virtual"}


def input_devices(db):
    # Interfaces of one USB device share ID_SERIAL; one entry per physical device, with every kind it has.
    seen = {}
    for e in db:
        if e.get("SUBSYSTEM") != "input" or "NAME" not in e:
            continue
        name = e["NAME"].strip('"')
        if name in ("Power Button", "Sleep Button", "Lid Switch", "Video Bus") or "HDMI" in name or "HDA " in name:
            continue
        kinds = [label for key, label in (("ID_INPUT_KEYBOARD", "Keyboard"), ("ID_INPUT_MOUSE", "Mouse"), ("ID_INPUT_TOUCHPAD", "Touchpad"),
                                         ("ID_INPUT_JOYSTICK", "Game controller"), ("ID_INPUT_TABLET", "Tablet"), ("ID_INPUT_TOUCHSCREEN", "Touchscreen"))
                 if e.get(key) == "1"]
        bus = e.get("ID_BUS", "")
        if not kinds or not bus:
            continue
        key = e.get("ID_SERIAL") or e.get("UNIQ") or name
        vend = clean(e.get("ID_VENDOR", "")) or clean(e.get("ID_VENDOR_FROM_DATABASE", ""))
        model = clean(e.get("ID_MODEL", "")) or clean(e.get("ID_MODEL_FROM_DATABASE", ""))
        title = f"{vend} {model}" if vend and model and not vend.lower().startswith(model.lower()[:4]) else (model or clean(name))
        d = seen.setdefault(key, {"name": title, "kinds": [], "vendor": vend, "model": model, "bus": BUS.get(bus, bus.upper()),
                                  "usbid": f'{e.get("ID_VENDOR_ID", "")}:{e.get("ID_MODEL_ID", "")}' if e.get("ID_VENDOR_ID") else "",
                                  "serial": e.get("ID_USB_SERIAL_SHORT") or e.get("ID_SERIAL_SHORT", ""), "nodes": []})
        for k in kinds:
            if k not in d["kinds"]:
                d["kinds"].append(k)
        if e.get("DEVNAME"):
            d["nodes"].append(os.path.basename(e["DEVNAME"]))
    return list(seen.values())


def pci_devices():
    out, cur = [], {}
    for line in run("lspci", "-vmm", "-k").splitlines():
        if not line.strip():
            if cur:
                out.append(cur)
            cur = {}
        elif ":" in line:
            k, v = line.split(":", 1)
            cur[k.strip()] = v.strip()
    if cur:
        out.append(cur)
    return out


def pci_item(p):
    name = f'{vendor(p.get("Vendor", ""))} {clean(p.get("Device", ""))}'.strip()
    return {"name": name, "props": props(("Manufacturer", vendor(p.get("Vendor", ""))), ("Model", p.get("Device")), ("Class", p.get("Class")),
                                          ("Driver", p.get("Driver")), ("Module", p.get("Module")), ("Location", "PCI " + p.get("Slot", "")),
                                          ("Subsystem", f'{vendor(p.get("SVendor", ""))} {p.get("SDevice", "")}'.strip()), ("Revision", p.get("Rev")))}


def net_adapters(pci):
    out = []
    by_slot = {("0000:" + p["Slot"]) if len(p.get("Slot", "")) == 7 else p.get("Slot", ""): p for p in pci}
    for n in sorted(os.listdir("/sys/class/net")):
        if n == "lo":
            continue
        base = f"/sys/class/net/{n}"
        def read(f):
            try:
                return open(f"{base}/{f}").read().strip()
            except OSError:
                return ""
        kind = "Wi-Fi" if os.path.isdir(f"{base}/wireless") else "Ethernet"
        slot = ""
        try:
            slot = next((l.strip().split("=", 1)[1] for l in open(f"{base}/device/uevent") if l.startswith("PCI_SLOT_NAME=")), "")
        except OSError:
            pass
        p = by_slot.get(slot, {})
        hw = f'{vendor(p.get("Vendor", ""))} {clean(p.get("Device", ""))}'.strip()
        driver = os.path.basename(os.path.realpath(f"{base}/device/driver")) if os.path.exists(f"{base}/device/driver") else p.get("Driver", "")
        speed = read("speed")
        out.append({"name": hw or f"{kind} adapter", "props": props(("Kind", kind), ("Interface", n), ("Status", {"up": "Up", "down": "Down"}.get(read("operstate"), read("operstate"))),
                                                                  ("MAC address", read("address")), ("Link speed", f"{speed} Mb/s" if speed and speed != "-1" else ""),
                                                                  ("Driver", driver), ("Location", "PCI " + slot[5:] if slot else ""))})
    return out


def usb_devices(db):
    names, out = {}, []
    for e in db:
        if e.get("SUBSYSTEM") == "usb" and e.get("DEVTYPE") == "usb_device":
            names[os.path.basename(e.get("_path", ""))] = (clean(e.get("ID_VENDOR_FROM_DATABASE", "")), clean(e.get("ID_MODEL_FROM_DATABASE", "")))
    base = "/sys/bus/usb/devices"
    for d in sorted(os.listdir(base)):
        p = f"{base}/{d}"
        if not re.match(r"^\d+-\d", d) or ":" in d or not os.path.exists(f"{p}/idProduct"):
            continue
        def read(f):
            try:
                return open(f"{p}/{f}").read().strip()
            except OSError:
                return ""
        cls = read("bDeviceClass")
        classes = set()
        for i in os.listdir(p):
            if i.startswith(d + ":") and os.path.exists(f"{p}/{i}/bInterfaceClass"):
                try:
                    classes.add(open(f"{p}/{i}/bInterfaceClass").read().strip())
                except OSError:
                    pass
        kind = {"09": "Hub", "01": "Audio", "02": "Modem", "03": "Input", "07": "Printer", "08": "Storage", "0e": "Camera", "e0": "Wireless"}.get(cls, "")
        if not kind:
            for c, label in (("0e", "Camera"), ("01", "Audio"), ("08", "Storage"), ("07", "Printer"), ("e0", "Wireless"), ("03", "Input")):
                if c in classes:
                    kind = label
                    break
        vend, prod = names.get(d, ("", ""))
        manufacturer = vendor(read("manufacturer") or vend)
        product = read("product") or prod
        if manufacturer and product.lower().startswith(manufacturer.lower()):
            product = product[len(manufacturer):].strip()
        speed = read("speed")
        out.append({"name": clean(f"{manufacturer} {product}") or f'{read("idVendor")}:{read("idProduct")}', "kind": kind or "Device",
                    "usbid": f'{read("idVendor")}:{read("idProduct")}',
                    "props": props(("Manufacturer", manufacturer), ("Model", product), ("Kind", kind or "Device"), ("Hardware ID", f'USB {read("idVendor")}:{read("idProduct")}'),
                                   ("Serial", read("serial")), ("Speed", f"{speed} Mb/s" if speed else ""), ("USB version", read("version")), ("Location", f"Bus {read('busnum')} port {d}"))})
    return out


def disks():
    out = []
    try:
        data = json.loads(run("lsblk", "-J", "-d", "-o", "NAME,MODEL,SIZE,TRAN,TYPE,VENDOR,ROTA,SERIAL,REV"))
    except ValueError:
        return out
    for b in data.get("blockdevices", []):
        if b.get("type") != "disk" or b["name"].startswith(("zram", "loop", "ram")):
            continue
        tran = (b.get("tran") or "").upper()
        kind = "Solid state" if tran == "NVME" or b.get("rota") in (False, "0") else "Hard disk"
        out.append({"name": clean(f'{b.get("vendor") or ""} {b.get("model") or ""}') or b["name"],
                    "props": props(("Kind", kind), ("Size", b.get("size")), ("Interface", tran), ("Device", "/dev/" + b["name"]), ("Serial", b.get("serial")), ("Firmware", b.get("rev")))})
    return out


def processors():
    fields = {}
    try:
        for x in json.loads(run("lscpu", "-J")).get("lscpu", []):
            fields[x["field"].rstrip(":")] = x["data"]
    except ValueError:
        pass
    name = fields.get("Model name", "")
    if not name:
        return []
    mhz = fields.get("CPU max MHz", "")
    return [{"name": name, "props": props(("Manufacturer", {"AuthenticAMD": "AMD", "GenuineIntel": "Intel"}.get(fields.get("Vendor ID"), fields.get("Vendor ID"))),
                                             ("Cores", fields.get("Core(s) per socket")), ("Threads", fields.get("CPU(s)")), ("Maximum clock", (mhz.split(".")[0] + " MHz") if mhz else ""),
                                             ("L3 cache", fields.get("L3 cache")), ("Architecture", fields.get("Architecture")), ("Virtualisation", fields.get("Virtualization")))}]


def memory():
    try:
        kb = int(next(l for l in open("/proc/meminfo") if l.startswith("MemTotal")).split()[1])
    except (OSError, StopIteration, ValueError):
        return []
    return [{"name": f"{kb / 1048576:.0f} GB memory", "props": props(("Total", f"{kb / 1048576:.1f} GB"))}]


def bluetooth_adapters(db):
    out = []
    base = "/sys/class/bluetooth"
    if not os.path.isdir(base):
        return out
    for h in sorted(os.listdir(base)):
        p = f"{base}/{h}"
        try:
            addr = open(f"{p}/address").read().strip()
        except OSError:
            addr = ""
        parent = os.path.realpath(f"{p}/device")
        name = "Bluetooth adapter"
        for e in db:
            if e.get("SUBSYSTEM") == "usb" and e.get("DEVTYPE") == "usb_device" and parent.startswith("/sys" + e.get("_path", "//")):
                name = clean(f'{vendor(e.get("ID_VENDOR_FROM_DATABASE", ""))} {e.get("ID_MODEL_FROM_DATABASE", "")}') or name
        driver = os.path.basename(os.path.realpath(f"{parent}/driver")) if os.path.exists(f"{parent}/driver") else ""
        out.append({"name": name, "props": props(("Kind", "Adapter"), ("Address", addr), ("Interface", h), ("Driver", driver))})
    return out


def cameras():
    out = []
    base = "/sys/class/video4linux"
    if not os.path.isdir(base):
        return out
    for v in sorted(os.listdir(base)):
        try:
            name = open(f"{base}/{v}/name").read().strip()
        except OSError:
            continue
        out.append({"name": name, "props": props(("Device", "/dev/" + v))})
    return out


def printers():
    out = []
    for line in run("lpstat", "-p").splitlines():
        m = re.match(r"printer (\S+) (.*)", line)
        if m:
            out.append({"name": m.group(1), "props": props(("Status", m.group(2)))})
    return out


db = udev_db()
pci = pci_devices()
inputs = input_devices(db)


def input_items(kind):
    out = []
    for d in inputs:
        if kind not in d["kinds"]:
            continue
        out.append({"name": d["name"], "props": props(("Manufacturer", d["vendor"]), ("Model", d["model"]), ("Capabilities", ", ".join(d["kinds"])), ("Bus", d["bus"]),
                                                     ("Hardware ID", ("USB " + d["usbid"]) if d["usbid"] else ""), ("Serial", d["serial"]), ("Device nodes", ", ".join(sorted(set(d["nodes"]))[:6])))})
    return out


def pci_items(pattern):
    return [pci_item(p) for p in pci if re.search(pattern, p.get("Class", ""))]


usb = usb_devices(db)
KNOWN = r"Audio|audio|VGA|3D|Display|Ethernet|Network|Non-Volatile|SATA|RAID|Mass storage|SCSI|IDE|USB"
categories = [
    {"id": "audio-hw", "name": "Sound controllers", "glyph": "headphones", "items": pci_items(r"Audio|audio") + [u for u in usb if u["kind"] == "Audio"]},
    {"id": "bt-adapters", "name": "Bluetooth", "glyph": "bluetooth", "items": bluetooth_adapters(db)},
    {"id": "cameras", "name": "Cameras", "glyph": "camera", "items": cameras() + [u for u in usb if u["kind"] == "Camera"]},
    {"id": "disks", "name": "Disk drives", "glyph": "hard-drive", "items": disks()},
    {"id": "gpus", "name": "Display adapters", "glyph": "cpu", "items": pci_items(r"VGA|3D|Display")},
    {"id": "gamepads", "name": "Game controllers", "glyph": "gamepad-2", "items": input_items("Game controller"), "page": "mouse"},
    {"id": "keyboards", "name": "Keyboards", "glyph": "keyboard", "items": input_items("Keyboard"), "page": "keyboard"},
    {"id": "memory", "name": "Memory", "glyph": "layout-grid", "items": memory()},
    {"id": "mice", "name": "Mice and pointing devices", "glyph": "mouse", "items": input_items("Mouse") + input_items("Touchpad") + input_items("Tablet"), "page": "mouse"},
    {"id": "network", "name": "Network adapters", "glyph": "globe", "items": net_adapters(pci), "page": "network"},
    {"id": "printers", "name": "Printers", "glyph": "printer", "items": printers() + [u for u in usb if u["kind"] == "Printer"], "page": "printers"},
    {"id": "cpu", "name": "Processors", "glyph": "cpu", "items": processors()},
    {"id": "storage-ctl", "name": "Storage controllers", "glyph": "hard-drive", "items": pci_items(r"Non-Volatile|SATA|RAID|Mass storage|SCSI|IDE") + [u for u in usb if u["kind"] == "Storage"]},
    {"id": "usb-ctl", "name": "USB controllers", "glyph": "plug-zap", "items": pci_items(r"USB") + [u for u in usb if u["kind"] == "Hub"]},
    {"id": "usb", "name": "USB devices", "glyph": "plug-zap", "items": [u for u in usb if u["kind"] in ("Device", "Input", "Wireless", "Modem") and u["usbid"] not in {i["usbid"] for i in inputs}]},
    {"id": "system", "name": "System devices", "glyph": "settings-2", "collapsed": True, "items": [pci_item(p) for p in pci if not re.search(KNOWN, p.get("Class", ""))]},
]
for c in categories:
    for i in c["items"]:
        i.pop("kind", None)
        i.pop("usbid", None)
print(json.dumps({"categories": categories}))
