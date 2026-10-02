#!/usr/bin/env python3
"""What the system keeps that can go, which only root can clear: `status` as JSON {items: [{id, name, note,
size}]}, read as the user; `run <id>` clears one, as root.
"""
import json, os, re, subprocess, sys

JOURNAL_KEEP = 100 * 1024 * 1024
COREDUMPS = "/var/lib/systemd/coredump"
UNITS = {"B": 1, "K": 1024, "M": 1024 ** 2, "G": 1024 ** 3, "T": 1024 ** 4}


def out(*cmd):
    try:
        return subprocess.run(cmd, capture_output=True, text=True).stdout
    except OSError:
        return ""


def to_bytes(number, unit):
    return int(float(number) * UNITS.get(unit[:1].upper(), 1))


def paccache_saved(*flags):
    m = re.search(r"disk space saved: ([\d.]+) (\w+)", out("paccache", *flags))
    return to_bytes(*m.groups()) if m else 0


def orphans():
    return out("pacman", "-Qtdq").split()


def status():
    items = []
    old = paccache_saved("-dk2") + paccache_saved("-duk0")
    items.append({"id": "packages", "name": "Old package versions",
                  "note": "pacman's downloads of versions no longer installed, keeping the last two of each",
                  "size": old})

    names = orphans()
    size = 0
    if names:
        for m in re.finditer(r"Installed Size\s*:\s*([\d.]+) (\w+)", out("pacman", "-Qi", *names)):
            size += to_bytes(*m.groups())
    items.append({"id": "orphans", "name": "Packages nothing needs",
                  "note": ", ".join(names[:6]) + (" and %d more" % (len(names) - 6) if len(names) > 6 else "")
                          if names else "Installed as a dependency of something since removed",
                  "size": size})

    m = re.search(r"take up ([\d.]+)(\w)", out("journalctl", "--disk-usage"))
    journal = to_bytes(*m.groups()) if m else 0
    items.append({"id": "journal", "name": "System logs",
                  "note": "The journal, cut down to its last 100 MB",
                  "size": max(0, journal - JOURNAL_KEEP)})

    dumps = 0
    if os.path.isdir(COREDUMPS):
        for name in os.listdir(COREDUMPS):
            try:
                dumps += os.lstat(os.path.join(COREDUMPS, name)).st_blocks * 512
            except OSError:
                pass
    items.append({"id": "coredumps", "name": "Crash dumps",
                  "note": "Memory saved when programs crashed, kept for debugging them",
                  "size": dumps})
    print(json.dumps({"items": items}))


def run(item):
    if item == "packages":
        subprocess.run(["paccache", "-rk2"], check=True)
        subprocess.run(["paccache", "-ruk0"], check=True)
    elif item == "orphans":
        names = orphans()
        if names:
            subprocess.run(["pacman", "-Rns", "--noconfirm", *names], check=True)
    elif item == "journal":
        subprocess.run(["journalctl", "--vacuum-size=%d" % JOURNAL_KEEP], check=True)
    elif item == "coredumps":
        for name in os.listdir(COREDUMPS):
            path = os.path.join(COREDUMPS, name)
            if os.path.isfile(path):
                os.remove(path)
    else:
        sys.exit("unknown item: " + item)


if __name__ == "__main__":
    if len(sys.argv) > 2 and sys.argv[1] == "run":
        run(sys.argv[2])
    else:
        status()
