#!/usr/bin/env python3
"""AppImages in ~/Applications, and the launcher entries that point at them.

    appimages.py sync            register what is there, forget what is gone; {added, removed}
    appimages.py list            [{path, id, name, icon}] of what is registered
    appimages.py inspect <path>  {name, icon, version, appimage} without registering anything
    appimages.py add <path>      register one, wherever it is; prints its row
    appimages.py forget <path>   take its entry and icon away; the file itself is left alone

An AppImage is an ELF with a filesystem appended, so what it calls itself is read out of that
filesystem rather than by running it: the entry is written before anyone has decided to trust it.
"""
import hashlib, json, os, re, shutil, struct, subprocess, sys, tempfile

HOME = os.path.expanduser("~")
DATA = os.environ.get("XDG_DATA_HOME") or os.path.join(HOME, ".local/share")
APPLICATIONS = os.path.join(HOME, "Applications")
ENTRIES = os.path.join(DATA, "applications")
ICONS = os.path.join(DATA, "icons/hicolor")
PREFIX = "isle-appimage-"
# The path the entry was written for, and the file as it was then, so a version dropped over an
# older one is registered again rather than mistaken for the one already known.
PATH_KEY = "X-Isle-AppImage"
STAMP_KEY = "X-Isle-AppImage-Stamp"
# What is carried over from the bundle's own entry. StartupWMClass is how the shell matches a
# window to the app that opened it, so it matters as much as the name.
KEEP = ["Name", "GenericName", "Comment", "Categories", "Keywords", "MimeType", "Terminal",
        "StartupWMClass", "StartupNotify", "NoDisplay"]


# --- the file itself ---------------------------------------------------------------------

def kind_of(path):
    """1 or 2 for an AppImage of that type, 0 for anything else."""
    try:
        with open(path, "rb") as f:
            head = f.read(64)
    except OSError:
        return 0
    if len(head) < 64 or head[:4] != b"\x7fELF" or head[8:10] != b"AI":
        return 0
    return head[10] if head[10] in (1, 2) else 0


def payload_offset(path):
    """Where the appended filesystem starts: the end of the ELF, which is its last section header."""
    with open(path, "rb") as f:
        head = f.read(64)
    end = "<" if head[5] == 1 else ">"
    if head[4] == 2:
        shoff, = struct.unpack_from(end + "Q", head, 0x28)
        entsize, num = struct.unpack_from(end + "HH", head, 0x3A)
    else:
        shoff, = struct.unpack_from(end + "I", head, 0x20)
        entsize, num = struct.unpack_from(end + "HH", head, 0x2E)
    return shoff + entsize * num


def unpack(path, into):
    """The few things an entry is made of, out of the bundle's root and its icon theme."""
    kind = kind_of(path)
    if kind == 2:
        if not shutil.which("unsquashfs"):
            return False
        cmd = ["unsquashfs", "-no-progress", "-force", "-dest", into,
               "-offset", str(payload_offset(path)), path,
               "/*.desktop", "/.DirIcon", "/*.png", "/*.svg", "/usr/share/icons"]
    elif kind == 1:
        # A type 1 image is an ISO9660 filesystem, which libarchive reads and squashfs-tools do not.
        cmd = ["bsdtar", "-x", "-f", path, "-C", into]
    else:
        return False
    try:
        run = subprocess.run(cmd, capture_output=True, timeout=60)
    except (OSError, subprocess.SubprocessError):
        return False
    return run.returncode == 0 and os.path.isdir(into)


def parse_entry(path):
    """The [Desktop Entry] group as a dict, ignoring the actions and the translations."""
    out = {}
    try:
        with open(path, encoding="utf-8", errors="replace") as f:
            inside = False
            for line in f:
                line = line.strip()
                if line.startswith("["):
                    inside = line == "[Desktop Entry]"
                    continue
                if not inside or "=" not in line or line.startswith("#"):
                    continue
                key, _, value = line.partition("=")
                key = key.strip()
                if "[" not in key:
                    out[key] = value.strip()
    except OSError:
        return {}
    return out


def bundled_entry(root):
    """The bundle's own desktop file, which sits at its root."""
    try:
        names = sorted(n for n in os.listdir(root) if n.endswith(".desktop"))
    except OSError:
        return None, {}
    for name in names:
        found = parse_entry(os.path.join(root, name))
        if found.get("Name"):
            return name[:-len(".desktop")], found
    return (names[0][:-len(".desktop")], {}) if names else (None, {})


def png_size(path):
    """The width an IHDR says, so an icon lands in the theme directory it belongs to."""
    try:
        with open(path, "rb") as f:
            head = f.read(24)
        if head[:8] != b"\x89PNG\r\n\x1a\n":
            return 0
        return struct.unpack_from(">I", head, 16)[0]
    except (OSError, struct.error):
        return 0


def bundled_icon(root, named):
    """The best picture in the bundle for the icon its entry names, or its .DirIcon."""
    wanted = os.path.basename(named or "")
    found = []
    for base, _, names in os.walk(root):
        for name in names:
            stem, ext = os.path.splitext(name)
            if ext.lower() not in (".png", ".svg"):
                continue
            if wanted and stem != wanted:
                continue
            full = os.path.join(base, name)
            if os.path.islink(full) and not os.path.exists(full):
                continue
            # An svg is any size that is asked of it, so it beats every png.
            found.append((1 if ext.lower() == ".svg" else 0, png_size(full), full))
    if found:
        return max(found)[2]
    direct = os.path.join(root, ".DirIcon")
    return direct if os.path.exists(direct) else ""


# --- the entries this writes -------------------------------------------------------------

def registered():
    """Every entry this has written, as {path: row}."""
    out = {}
    try:
        names = os.listdir(ENTRIES)
    except OSError:
        return out
    for name in names:
        if not name.startswith(PREFIX) or not name.endswith(".desktop"):
            continue
        found = parse_entry(os.path.join(ENTRIES, name))
        path = found.get(PATH_KEY)
        if not path:
            continue
        out[path] = {"path": path, "id": name[:-len(".desktop")], "name": found.get("Name", ""),
                     "icon": found.get("Icon", ""), "stamp": found.get(STAMP_KEY, "")}
    return out


def stamp_of(path):
    try:
        at = os.stat(path)
    except OSError:
        return ""
    return "%d:%d" % (int(at.st_mtime), at.st_size)


def id_for(path, stem):
    """The bundle's own name for itself, kept stable across versions so a newer one replaces the
    older entry. Two different images that call themselves the same thing are told apart by path."""
    base = re.sub(r"[^a-zA-Z0-9_.-]", "-", stem or os.path.basename(path)).strip("-") or "app"
    wanted = PREFIX + base
    at = os.path.join(ENTRIES, wanted + ".desktop")
    if os.path.exists(at) and parse_entry(at).get(PATH_KEY) not in (path, None):
        wanted += "-" + hashlib.sha1(path.encode()).hexdigest()[:6]
    return wanted


def put_icon(source, name):
    """The picture into the icon theme under our own name, so everything that draws an icon by
    name finds it: the launcher, the notification, the window list."""
    if not source:
        return ""
    ext = os.path.splitext(source)[1].lower()
    size = png_size(source) if ext == ".png" else 0
    where = "scalable" if ext == ".svg" else "%dx%d" % (size, size) if size else "48x48"
    into = os.path.join(ICONS, where, "apps")
    try:
        os.makedirs(into, exist_ok=True)
        shutil.copyfile(source, os.path.join(into, name + ext))
    except OSError:
        return ""
    return name


def drop_icon(name):
    if not name.startswith(PREFIX):
        return
    for base, _, names in os.walk(ICONS):
        for one in names:
            if os.path.splitext(one)[0] == name:
                try:
                    os.remove(os.path.join(base, one))
                except OSError:
                    pass


def quoted(path):
    return '"' + path.replace("\\", "\\\\").replace('"', '\\"') + '"'


def write_entry(ident, path, found, icon):
    """The bundle's entry as our own: the same app, told to run the file where it sits."""
    # Whatever the bundle's Exec asked for after its own name, which is how a file opened with it
    # reaches it; the command itself is replaced, since the bundle's is a path inside the image.
    args = ""
    exec_line = found.get("Exec", "")
    if exec_line:
        rest = exec_line.split(" ", 1)
        args = " " + rest[1].strip() if len(rest) > 1 and rest[1].strip() else ""
    lines = ["[Desktop Entry]", "Type=Application"]
    for key in KEEP:
        if found.get(key):
            lines.append(key + "=" + found[key])
    if not found.get("Name"):
        lines.append("Name=" + os.path.splitext(os.path.basename(path))[0])
    lines.append("Exec=" + quoted(path) + args)
    # The entry hides itself the moment the file is gone, before anything has reconciled the two.
    lines.append("TryExec=" + path)
    lines.append("Icon=" + (icon or "application-x-executable"))
    lines.append(PATH_KEY + "=" + path)
    lines.append(STAMP_KEY + "=" + stamp_of(path))
    os.makedirs(ENTRIES, exist_ok=True)
    with open(os.path.join(ENTRIES, ident + ".desktop"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")


def database():
    if shutil.which("update-desktop-database"):
        subprocess.run(["update-desktop-database", ENTRIES], capture_output=True)


# --- what the shell asks for --------------------------------------------------------------

def inspect(path):
    if not kind_of(path):
        return {"appimage": False}
    with tempfile.TemporaryDirectory() as tmp:
        root = os.path.join(tmp, "squashfs-root")
        if not unpack(path, root):
            return {"appimage": True, "name": os.path.splitext(os.path.basename(path))[0]}
        stem, found = bundled_entry(root)
        return {"appimage": True, "stem": stem or "",
                "name": found.get("Name") or os.path.splitext(os.path.basename(path))[0],
                "version": found.get("X-AppImage-Version", ""),
                "comment": found.get("Comment", "")}


def add(path):
    path = os.path.abspath(path)
    if not kind_of(path):
        return None
    try:
        os.chmod(path, os.stat(path).st_mode | 0o111)
    except OSError:
        pass
    with tempfile.TemporaryDirectory() as tmp:
        root = os.path.join(tmp, "squashfs-root")
        found, stem, picture = {}, None, ""
        if unpack(path, root):
            stem, found = bundled_entry(root)
            picture = bundled_icon(root, found.get("Icon", ""))
        ident = id_for(path, stem or os.path.splitext(os.path.basename(path))[0])
        icon = put_icon(picture, ident) if picture else ""
        write_entry(ident, path, found, icon)
    return {"path": path, "id": ident, "name": found.get("Name") or
            os.path.splitext(os.path.basename(path))[0], "icon": icon or "application-x-executable"}


def forget(path):
    known = registered().get(os.path.abspath(path))
    if not known:
        return None
    try:
        os.remove(os.path.join(ENTRIES, known["id"] + ".desktop"))
    except OSError:
        pass
    drop_icon(known["icon"])
    return known


def present():
    """What is in the folder, by what it is rather than by what it is called."""
    out = []
    try:
        names = sorted(os.listdir(APPLICATIONS))
    except OSError:
        return out
    for name in names:
        full = os.path.join(APPLICATIONS, name)
        if os.path.isfile(full) and kind_of(full):
            out.append(full)
    return out


def sync():
    known, here = registered(), present()
    added, removed = [], []
    for path in here:
        was = known.get(path)
        if was and was["stamp"] == stamp_of(path):
            continue
        # A file written over in place is a different app as far as the entry is concerned.
        if was:
            forget(path)
        row = add(path)
        if row:
            added.append(row)
    inside = set(here)
    for path, row in known.items():
        # An entry is ours to keep only while its file is there; one registered from somewhere else
        # is kept as long as that file is, so adding from outside the folder is not undone at once.
        gone = not os.path.exists(path)
        left = os.path.dirname(path) == APPLICATIONS and path not in inside
        if gone or left:
            forget(path)
            removed.append(row)
    if added or removed:
        database()
    return {"added": added, "removed": removed}


def main():
    verb = sys.argv[1] if len(sys.argv) > 1 else "sync"
    where = sys.argv[2] if len(sys.argv) > 2 else ""
    if verb == "sync":
        json.dump(sync(), sys.stdout)
    elif verb == "list":
        json.dump(sorted(registered().values(), key=lambda r: r["name"].lower()), sys.stdout)
    elif verb == "inspect":
        json.dump(inspect(where), sys.stdout)
    elif verb == "add":
        row = add(where)
        database()
        json.dump(row or {}, sys.stdout)
    elif verb == "forget":
        row = forget(where)
        database()
        json.dump(row or {}, sys.stdout)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
