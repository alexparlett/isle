#!/usr/bin/env python3
"""Isle's tiles in Steam Big Picture: two non-Steam shortcuts, "Isle settings" and "Desktop".

    steamshortcuts.py status     {"steam": bool, "running": bool, "installed": bool}
    steamshortcuts.py add        writes both shortcuts into every Steam user's shortcuts.vdf
    steamshortcuts.py remove     takes them out

Steam reads the file when it starts and writes it back when it quits, so a change only holds while Steam
is closed; add and remove refuse while it runs. The file is Valve's binary key-value format: a map is
0x00 name 0x00 … 0x08, a string 0x01 name 0x00 value 0x00, an int 0x02 name 0x00 and four bytes.
"""
import json, os, shutil, struct, subprocess, sys, zlib

HOME = os.path.expanduser("~")
ROOTS = [HOME + "/.local/share/Steam", HOME + "/.steam/steam", HOME + "/.var/app/com.valvesoftware.Steam/.local/share/Steam"]
SHELL = os.path.dirname(os.path.abspath(__file__))
OURS = {
    "Isle settings": {"exe": SHELL + "/isle-quick.sh", "args": ""},
    "Desktop": {"exe": SHELL + "/isle-quick.sh", "args": "desktop"},
}


def parse(data, pos=0):
    out = {}
    while pos < len(data):
        t = data[pos]; pos += 1
        if t == 0x08:
            return out, pos
        end = data.index(b"\x00", pos); name = data[pos:end].decode("utf-8", "replace"); pos = end + 1
        if t == 0x00:
            out[name], pos = parse(data, pos)
        elif t == 0x01:
            end = data.index(b"\x00", pos); out[name] = data[pos:end].decode("utf-8", "replace"); pos = end + 1
        elif t == 0x02:
            out[name] = struct.unpack("<I", data[pos:pos + 4])[0]; pos += 4
        else:
            raise ValueError("unknown type %d" % t)
    return out, pos


def dump(m):
    b = bytearray()
    for k, v in m.items():
        if isinstance(v, dict):
            b += b"\x00" + k.encode() + b"\x00" + dump(v) + b"\x08"
        elif isinstance(v, int):
            b += b"\x02" + k.encode() + b"\x00" + struct.pack("<I", v)
        else:
            b += b"\x01" + k.encode() + b"\x00" + str(v).encode() + b"\x00"
    return bytes(b)


def appid(exe, name):
    return (zlib.crc32((exe + name).encode()) | 0x80000000) & 0xFFFFFFFF


def entry(name, spec):
    exe = '"%s"' % spec["exe"]
    return {"appid": appid(exe, name), "AppName": name, "Exe": exe, "StartDir": '"%s"' % SHELL, "icon": "", "ShortcutPath": "",
            "LaunchOptions": spec["args"], "IsHidden": 0, "AllowDesktopConfig": 1, "AllowOverlay": 1, "OpenVR": 0, "Devkit": 0,
            "DevkitGameID": "", "DevkitOverrideAppID": 0, "LastPlayTime": 0, "FlatpakAppID": "", "tags": {}}


def files():
    for root in ROOTS:
        for user in sorted(os.listdir(root + "/userdata")) if os.path.isdir(root + "/userdata") else []:
            if user.isdigit() and user != "0":
                d = root + "/userdata/" + user + "/config"
                if os.path.isdir(d):
                    yield d + "/shortcuts.vdf"


def load(path):
    try:
        return parse(open(path, "rb").read())[0].get("shortcuts", {})
    except (OSError, ValueError, IndexError):
        return {}


def save(path, shortcuts):
    if os.path.exists(path):
        shutil.copy2(path, path + ".isle-backup")
    tmp = path + ".isle"
    with open(tmp, "wb") as f:
        f.write(dump({"shortcuts": shortcuts}))
    os.replace(tmp, path)


def running():
    return subprocess.run(["pgrep", "-x", "steam"], capture_output=True).returncode == 0


def status():
    paths = list(files())
    installed = bool(paths) and all(all(any(e.get("AppName") == n for e in load(p).values()) for n in OURS) for p in paths)
    print(json.dumps({"steam": shutil.which("steam") is not None, "running": running(), "installed": installed}))


def change(add):
    if running():
        sys.stderr.write("Steam is running; close it first\n"); return 1
    paths = list(files())
    if not paths:
        sys.stderr.write("no Steam user found\n"); return 1
    for p in paths:
        kept = [e for e in load(p).values() if e.get("AppName") not in OURS]
        if add:
            kept += [entry(n, s) for n, s in OURS.items()]
        save(p, {str(i): e for i, e in enumerate(kept)})
    return 0


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else ""
    if cmd == "status":
        status()
    elif cmd in ("add", "remove"):
        sys.exit(change(cmd == "add"))
    else:
        print(__doc__); sys.exit(2)
