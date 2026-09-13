#!/usr/bin/env python3
"""What a person asks of a drive, from a file manager's menu or from Settings (D65).

    drivectl.py state  <path>          cloud | cached | pinned | folder
    drivectl.py pin    <path>...       keep it on this device
    drivectl.py unpin  <path>...
    drivectl.py evict  <path>...       give the space back; the file stays, its copy goes
    drivectl.py status [drive]         what the drive holds and what it costs here
    drivectl.py menus                  put the two actions in the file managers' menus

A path is an ordinary one under ~/Drives/<drive>, which is what a file manager passes.
"""
import json, os, socket, sys

ROOT = os.path.expanduser("~/Drives")
RUNTIME = os.environ.get("XDG_RUNTIME_DIR", "/run/user/%d" % os.getuid())


def sock_for(drive):
    return os.path.join(RUNTIME, "isle", "drives", drive + ".sock")


def drives():
    where = os.path.join(RUNTIME, "isle", "drives")
    try:
        return sorted(n[:-len(".sock")] for n in os.listdir(where) if n.endswith(".sock"))
    except OSError:
        return []


def split(path):
    """An absolute path to (drive, path within it)."""
    full = os.path.abspath(os.path.expanduser(path))
    rest = os.path.relpath(full, ROOT)
    if rest.startswith(".."):
        return None, None
    parts = rest.split(os.sep)
    return parts[0], "/".join(parts[1:])


def ask(drive, **q):
    where = sock_for(drive)
    if not os.path.exists(where):
        return {"error": "that drive is not running"}
    try:
        s = socket.socket(socket.AF_UNIX)
        s.settimeout(60)
        s.connect(where)
        s.sendall((json.dumps(q) + "\n").encode())
        said = s.recv(1 << 16).decode().strip()
        s.close()
        return json.loads(said or "{}")
    except (OSError, ValueError) as e:
        return {"error": str(e)}


THUNAR_ACTIONS = [
    ("Isle drive: keep", "Keep on this device", "emblem-favorite", "pin"),
    ("Isle drive: free", "Free up space", "emblem-remote", "evict"),
]


def thunar_menus(me):
    """Thunar's custom actions, which are a file of its own that a person may also have their own entries in,
    so ours are added to it rather than over it."""
    import xml.etree.ElementTree as ET
    path = os.path.expanduser("~/.config/Thunar/uca.xml")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    if os.path.exists(path):
        tree = ET.parse(path)
        root = tree.getroot()
    else:
        root = ET.Element("actions")
        tree = ET.ElementTree(root)
    have = {a.findtext("name") for a in root.findall("action")}
    for name, label, icon, op in THUNAR_ACTIONS:
        if name in have:
            continue
        a = ET.SubElement(root, "action")
        for tag, text in (("icon", icon), ("name", name), ("unique-id", name.replace(" ", "-")),
                          ("command", "python3 %s %s %%F" % (me, op)), ("description", label),
                          ("patterns", "*")):
            ET.SubElement(a, tag).text = text
        for tag in ("directories", "audio-files", "image-files", "other-files", "text-files", "video-files"):
            ET.SubElement(a, tag)
    tree.write(path, encoding="UTF-8", xml_declaration=True)
    return path


def kde_menus(me):
    """Dolphin's, which is one desktop file naming both actions."""
    where = os.path.expanduser("~/.local/share/kio/servicemenus")
    os.makedirs(where, exist_ok=True)
    path = os.path.join(where, "isle-drive.desktop")
    text = """[Desktop Entry]
Type=Service
MimeType=all/all;
Actions=IsleDriveKeep;IsleDriveFree;
X-KDE-Submenu=Cloud drive
X-KDE-Priority=TopLevel

[Desktop Action IsleDriveKeep]
Name=Keep on this device
Icon=emblem-favorite
Exec=python3 %s pin %%F

[Desktop Action IsleDriveFree]
Name=Free up space
Icon=emblem-remote
Exec=python3 %s evict %%F
""" % (me, me)
    open(path, "w").write(text)
    os.chmod(path, 0o755)
    return path


def main(argv):
    if not argv:
        sys.stderr.write(__doc__)
        return 2
    op, rest = argv[0], argv[1:]
    if op == "menus":
        me = os.path.abspath(__file__)
        print(json.dumps({"thunar": thunar_menus(me), "kde": kde_menus(me)}))
        return 0
    if op == "status":
        name = rest[0] if rest else (drives()[0] if drives() else "")
        if not name:
            print(json.dumps({"error": "no drive is running"}))
            return 1
        print(json.dumps(ask(name, op="status")))
        return 0
    if op not in ("state", "pin", "unpin", "evict") or not rest:
        sys.stderr.write("No such question\n")
        return 2
    out = []
    for path in rest:
        drive, within = split(path)
        if drive is None:
            out.append({"path": path, "error": "not on a drive"})
            continue
        said = ask(drive, op=op, path=within)
        said["path"] = path
        out.append(said)
    # One path answers plainly, several answer as a list: a menu asks about one, Settings about many.
    print(json.dumps(out[0] if len(out) == 1 else out))
    return 0


sys.exit(main(sys.argv[1:]))
