#!/usr/bin/env python3
"""What a keyboard's chords actually produce, measured rather than read off the config.

A virtual keyboard is created through uinput under a name the remap layer matches, a chord is pressed on it,
and what the remap layer emits is read back from its own device: the modifiers held when the last key goes
down are the chord an application sees.

    tools/keycheck.py                       every case below, on a Mac-profile keyboard
    tools/keycheck.py --name "My Keyboard"  under another device name
    tools/keycheck.py --only cmd            only the cases whose group matches

Run it in the VM: the keys it presses reach whatever window has focus.
"""
import argparse, fcntl, glob, json, os, select, struct, subprocess, sys, time

KEYS = {"b": 48, "d": 32, "f": 33, "h": 35, "n": 49, "p": 25, "a": 30, "c": 46, "e": 18, "k": 37, "u": 22, "v": 47, "w": 17, "t": 20, "s": 31, "z": 44,
        "1": 2, "3": 4, "4": 5, "5": 6, "space": 57, "tab": 15, "enter": 28, "backspace": 14, "delete": 111,
        "left": 105, "right": 106, "up": 103, "down": 108, "home": 102, "end": 107, "grave": 41, "iso": 86,
        "ctrl": 29, "shift": 42, "alt": 56, "meta": 125, "print": 99}
NAMES = {v: k for k, v in KEYS.items()}
MODS = {KEYS["ctrl"]: "CTRL", KEYS["shift"]: "SHIFT", KEYS["alt"]: "ALT", KEYS["meta"]: "SUPER"}
EV_KEY, EV_SYN = 1, 0
SZ = struct.calcsize("llHHi")

# [group, what is pressed, what an application should see]
CASES = [
    ("cmd", "meta+a", "CTRL+a"),
    ("cmd", "meta+c", "CTRL+c"),
    ("cmd", "meta+shift+z", "CTRL+SHIFT+z"),
    ("shell", "meta+space", "SUPER+space"),
    ("shell", "meta+tab", "SUPER+tab"),
    ("text", "meta+left", "home"),
    ("text", "meta+right", "end"),
    ("text", "meta+backspace", "SHIFT+home,backspace"),
    ("text", "meta+shift+left", "SHIFT+home"),
    ("text", "meta+shift+right", "SHIFT+end"),
    ("text", "meta+up", "CTRL+home"),
    ("text", "meta+down", "CTRL+end"),
    ("text", "alt+left", "CTRL+left"),
    ("text", "alt+shift+left", "CTRL+SHIFT+left"),
    ("text", "alt+delete", "CTRL+delete"),
    ("window", "meta+alt+left", "ALT+SUPER+left"),
    ("window", "meta+grave", "ALT+grave"),
    ("capture", "meta+shift+3", "CTRL+print"),
    ("capture", "meta+shift+5", "SHIFT+SUPER+s"),
    # macOS's control key: the line editor's chords in a text field.
    ("control", "ctrl+a", "home"),
    ("control", "ctrl+e", "end"),
    ("control", "ctrl+b", "left"),
    ("control", "ctrl+f", "right"),
    ("control", "ctrl+p", "up"),
    ("control", "ctrl+n", "down"),
    ("control", "ctrl+d", "delete"),
    ("control", "ctrl+h", "backspace"),
    ("control", "ctrl+k", "SHIFT+end,delete"),
    ("control", "ctrl+u", "SHIFT+home,backspace"),
    # The shell's own, which macOS puts on Ctrl.
    ("shell", "ctrl+up", "CTRL+SUPER+up"),
    ("shell", "ctrl+left", "CTRL+SUPER+left"),
    ("shell", "ctrl+right", "CTRL+SUPER+right"),
    ("shell", "ctrl+1", "SUPER+1"),
    ("shell", "ctrl+shift+1", "SHIFT+SUPER+1"),
]

UI_SET_EVBIT, UI_SET_KEYBIT, UI_DEV_CREATE, UI_DEV_DESTROY, UI_DEV_SETUP = 0x40045564, 0x40045565, 0x5501, 0x5502, 0x405c5503


def make_keyboard(name):
    fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
    fcntl.ioctl(fd, UI_SET_EVBIT, EV_KEY)
    # The whole ordinary key range, or it does not read as a keyboard to what enumerates them.
    for code in range(1, 128):
        fcntl.ioctl(fd, UI_SET_KEYBIT, code)
    fcntl.ioctl(fd, UI_DEV_SETUP, struct.pack("HHHH80sI", 3, 0x3434, 0x0001, 1, name.encode()[:79], 0))
    fcntl.ioctl(fd, UI_DEV_CREATE)
    return fd


def send(fd, code, value):
    os.write(fd, struct.pack("llHHi", 0, 0, EV_KEY, code, value))
    os.write(fd, struct.pack("llHHi", 0, 0, EV_SYN, 0, 0))


def node_named(name, skip=()):
    for path in sorted(glob.glob("/sys/class/input/event*/device/name")):
        try:
            if open(path).read().strip() == name:
                node = "/dev/input/" + os.path.basename(os.path.dirname(os.path.dirname(path)))
                if node not in skip:
                    return node
        except OSError:
            continue
    return None


def chord(spec):
    """"meta+shift+a" -> ([mod codes], key code)"""
    parts = spec.split("+")
    return [KEYS[p] for p in parts[:-1]], KEYS[parts[-1]]


def effective(events):
    """Every key an application receives with the modifiers held at the time, in order: a rule may send more
    than one, as deleting to the start of a line does."""
    held, out = set(), []
    for code, down in events:
        if code in MODS:
            held.add(code) if down else held.discard(code)
        elif down:
            out.append("+".join(sorted(MODS[m] for m in held) + [NAMES.get(code, str(code))]))
    return ",".join(out) or "nothing"


def run(args):
    out_node = node_named("xremap")
    if not out_node:
        print("no device called xremap: the remap layer is not running, so there is nothing to measure")
        return 1
    kb = make_keyboard(args.name)
    print("created %r; waiting for the remap layer to take it" % args.name)
    time.sleep(args.settle)
    reader = os.open(out_node, os.O_RDONLY | os.O_NONBLOCK)
    bad = 0
    for group, press, want in CASES:
        if args.only and args.only != group:
            continue
        mods, key = chord(press)
        while select.select([reader], [], [], 0)[0]:
            os.read(reader, SZ * 64)
        for m in mods:
            send(kb, m, 1)
        send(kb, key, 1)
        send(kb, key, 0)
        for m in reversed(mods):
            send(kb, m, 0)
        events, end = [], time.time() + 0.35
        while time.time() < end:
            if select.select([reader], [], [], 0.05)[0]:
                data = os.read(reader, SZ * 64)
                for off in range(0, len(data) - SZ + 1, SZ):
                    _, _, t, c, v = struct.unpack_from("llHHi", data, off)
                    if t == EV_KEY and v in (0, 1):
                        events.append((c, v))
        got = effective(events)
        ok = got.lower() == want.lower()
        bad += 0 if ok else 1
        print("%-8s %-22s want %-20s got %-20s %s" % (group, press, want, got, "ok" if ok else "WRONG"))
        time.sleep(0.1)
    fcntl.ioctl(kb, UI_DEV_DESTROY)
    os.close(kb)
    print("\n%d wrong" % bad if bad else "\nevery case as expected")
    return 1 if bad else 0


def serve(args):
    """Hold one keyboard and press what arrives on stdin, a chord to a line: the remap layer grabs a device
    once, so pressing through the same one avoids racing it on every chord."""
    kb = make_keyboard(args.name)
    print("serving %r" % args.name, flush=True)
    for line in sys.stdin:
        spec = line.strip()
        if not spec or spec == "quit":
            break
        try:
            mods, key = chord(spec)
        except KeyError:
            print("unknown chord %r" % spec, flush=True)
            continue
        for m in mods:
            send(kb, m, 1)
        send(kb, key, 1)
        send(kb, key, 0)
        for m in reversed(mods):
            send(kb, m, 0)
        print("pressed " + spec, flush=True)
    fcntl.ioctl(kb, UI_DEV_DESTROY)
    os.close(kb)
    return 0


def press(args):
    """Press one chord and leave, so a caller can then look at what the compositor or the shell did with it."""
    kb = make_keyboard(args.name)
    time.sleep(args.settle)
    mods, key = chord(args.press)
    for m in mods:
        send(kb, m, 1)
    send(kb, key, 1)
    send(kb, key, 0)
    for m in reversed(mods):
        send(kb, m, 0)
    time.sleep(0.4)
    fcntl.ioctl(kb, UI_DEV_DESTROY)
    os.close(kb)
    return 0


def hold(args):
    """Keep the keyboard in existence, so the shell renders a profile for it and the remap layer takes it."""
    kb = make_keyboard(args.name)
    print("holding %r for %ds" % (args.name, args.hold), flush=True)
    time.sleep(args.hold)
    fcntl.ioctl(kb, UI_DEV_DESTROY)
    os.close(kb)
    return 0


p = argparse.ArgumentParser()
p.add_argument("--serve", action="store_true", help="hold a keyboard and press chords arriving on stdin")
p.add_argument("--press", default="", help="press one chord on a keyboard of this name, then exit")
p.add_argument("--hold", type=float, default=0, help="create the keyboard and keep it for this long, then exit")
p.add_argument("--name", default="Keychron Keycheck Keyboard")
p.add_argument("--only", default="")
p.add_argument("--settle", type=float, default=4.0)
a = p.parse_args()
sys.exit(serve(a) if a.serve else hold(a) if a.hold else press(a) if a.press else run(a))
