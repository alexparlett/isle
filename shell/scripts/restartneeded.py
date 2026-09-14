#!/usr/bin/env python3
"""Whether the machine runs older code than it has installed, so that a restart is due: JSON
{restart, reasons}. The kernel, when the running one's module tree is gone (pacman removes it with the
upgrade), the NVIDIA driver, when the module in the kernel is not the version of nvidia-utils, and a
compositor plugin, when the file Hyprland mapped is no longer the file on disk.
"""
import json, os, re, subprocess

PLUGIN_DIR = "/var/cache/hyprpm/"

reasons = []
running = os.uname().release
if not os.path.isdir("/usr/lib/modules/" + running):
    installed = sorted(d for d in os.listdir("/usr/lib/modules") if os.path.isdir("/usr/lib/modules/" + d + "/kernel")) if os.path.isdir("/usr/lib/modules") else []
    reasons.append("The kernel was updated: %s is running, %s is installed" % (running, installed[-1] if installed else "another"))
try:
    nvrm = open("/proc/driver/nvidia/version").read()
    m = re.search(r"NVRM version:.*?(\d+\.\d+(?:\.\d+)?)", nvrm)
    p = subprocess.run(["pacman", "-Q", "nvidia-utils"], capture_output=True, text=True)
    have = p.stdout.split()[1].split("-")[0] if p.returncode == 0 else ""
    if m and have and m.group(1) != have:
        reasons.append("The NVIDIA driver was updated: %s is loaded, %s is installed; until then nothing can use the GPU through Vulkan" % (m.group(1), have))
except OSError:
    pass


# A mapping line is `address perms offset dev inode path`, and the path carries " (deleted)" once the file it
# was mapped from is gone. hyprpm rebuilds a plugin as a new file at the same path and a session keeps the one
# it mapped, so a moved inode is a plugin this session is not running.
def stale_from_maps(lines):
    out = []
    for line in lines:
        f = line.split(maxsplit=5)
        if len(f) < 6:
            continue
        path = f[5].strip()
        gone = path.endswith(" (deleted)")
        if gone:
            path = path[: -len(" (deleted)")]
        if not path.startswith(PLUGIN_DIR) or not path.endswith(".so"):
            continue
        name = os.path.basename(path)[:-3]
        if name in out:
            continue
        try:
            if gone or os.stat(path).st_ino != int(f[4]):
                out.append(name)
        except (OSError, ValueError):
            out.append(name)
    return out


def hyprland_pid():
    for pid in os.listdir("/proc"):
        if not pid.isdigit():
            continue
        try:
            if open("/proc/%s/comm" % pid).read().strip() == "Hyprland":
                return pid
        except OSError:
            pass
    return None


stale = []
pid = hyprland_pid()
if pid:
    try:
        with open("/proc/%s/maps" % pid) as maps:
            stale = sorted(stale_from_maps(maps))
    except OSError:
        pass
if stale:
    reasons.append("%s %s rebuilt: this session is still running what it started with" % (
        ", ".join(stale), "was" if len(stale) == 1 else "were"))

print(json.dumps({"restart": bool(reasons), "reasons": reasons}))
