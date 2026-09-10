#!/usr/bin/env python3
"""XDG autostart entries, and switching them on and off for this desktop.

    autostart.py list                 [{id, name, icon, exec, enabled, source}]
    autostart.py enable <id>
    autostart.py disable <id>
    autostart.py add <desktop-id>     copy an application's entry into ~/.config/autostart
    autostart.py remove <id>          delete a user entry
"""
import configparser, json, os, shutil, sys

DESKTOP = "Hyprland"
home = os.path.expanduser("~")
user_dir = os.path.join(os.environ.get("XDG_CONFIG_HOME", home + "/.config"), "autostart")
system_dirs = [os.path.join(d, "autostart") for d in os.environ.get("XDG_CONFIG_DIRS", "/etc/xdg").split(":")]

def read(path):
    cp = configparser.RawConfigParser(strict=False, interpolation=None)
    cp.optionxform = str
    try:
        cp.read(path, encoding="utf-8")
        return cp, cp["Desktop Entry"]
    except Exception:
        return None, None

def enabled(e):
    if e.get("Hidden", "false").lower() == "true" or e.get("X-GNOME-Autostart-enabled", "true").lower() == "false":
        return False
    only = [x for x in e.get("OnlyShowIn", "").split(";") if x]
    if only and DESKTOP not in only:
        return False
    return DESKTOP not in [x for x in e.get("NotShowIn", "").split(";") if x]

def entries():
    seen, out = set(), []
    for source, dirs in (("user", [user_dir]), ("system", system_dirs)):
        for d in dirs:
            if not os.path.isdir(d):
                continue
            for f in sorted(os.listdir(d)):
                if not f.endswith(".desktop") or f in seen:
                    continue
                seen.add(f)
                cp, e = read(os.path.join(d, f))
                if e is None:
                    continue
                # A user file that only hides a system entry keeps the system entry's name.
                base = None
                if source == "user" and not e.get("Exec"):
                    for sd in system_dirs:
                        _, base = read(os.path.join(sd, f))
                        if base is not None:
                            break
                shown = base if base is not None else e
                out.append({"id": f, "name": shown.get("Name", f[:-8]), "icon": shown.get("Icon", ""), "exec": shown.get("Exec", ""),
                            "enabled": enabled(e), "source": "system" if base is not None else source})
    return out

def write_user(f, lines):
    os.makedirs(user_dir, exist_ok=True)
    with open(os.path.join(user_dir, f), "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")

def set_key(path, key, value):
    cp, e = read(path)
    if e is None:
        return
    e[key] = value
    with open(path, "w", encoding="utf-8") as fh:
        cp.write(fh, space_around_delimiters=False)

cmd = sys.argv[1] if len(sys.argv) > 1 else "list"
arg = sys.argv[2] if len(sys.argv) > 2 else ""
user_path = os.path.join(user_dir, arg)

if cmd == "list":
    print(json.dumps(entries()))
elif cmd == "disable":
    if os.path.exists(user_path):
        set_key(user_path, "Hidden", "true")
    else:
        write_user(arg, ["[Desktop Entry]", "Type=Application", "Name=" + arg[:-8], "Hidden=true"])
elif cmd == "enable":
    if os.path.exists(user_path):
        _, e = read(user_path)
        if e is not None and not e.get("Exec"):
            os.remove(user_path)
        else:
            set_key(user_path, "Hidden", "false")
            set_key(user_path, "X-GNOME-Autostart-enabled", "true")
elif cmd == "add":
    for d in [os.path.join(os.environ.get("XDG_DATA_HOME", home + "/.local/share"), "applications")] + \
             [os.path.join(x, "applications") for x in os.environ.get("XDG_DATA_DIRS", "/usr/local/share:/usr/share").split(":")] + \
             [home + "/.local/share/flatpak/exports/share/applications", "/var/lib/flatpak/exports/share/applications"]:
        src = os.path.join(d, arg.replace("-", "/")) if os.path.exists(os.path.join(d, arg.replace("-", "/"))) else os.path.join(d, arg)
        if os.path.isfile(src):
            os.makedirs(user_dir, exist_ok=True)
            shutil.copy(src, os.path.join(user_dir, os.path.basename(src)))
            break
elif cmd == "remove":
    if os.path.exists(user_path):
        os.remove(user_path)
