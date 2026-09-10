#!/usr/bin/env python3
"""Gamepad presses as lines on stdout: a b x y lb rb lt rt start select guide up down left right,
and rup rdown rleft rright from the right stick. The d-pad and the left stick both give directions;
a held direction repeats. A button held for HOLD seconds gives one more line, "<button>-hold".
A pad found gives "pad:sony", "pad:xbox", "pad:nintendo" or "pad:generic", from its vendor."""
import fcntl, glob, os, select, struct, sys, time

EV_KEY, EV_ABS = 1, 3
BTN = {0x130: "a", 0x131: "b", 0x133: "x", 0x134: "y", 0x136: "lb", 0x137: "rb", 0x138: "lt", 0x139: "rt",
       0x13a: "select", 0x13b: "start", 0x13c: "guide", 0x220: "up", 0x221: "down", 0x222: "left", 0x223: "right"}
AXES = {0x00: "x", 0x01: "y", 0x03: "rx", 0x04: "ry", 0x10: "hx", 0x11: "hy"}
# Analogue triggers: a press once past half travel.
TRIGGERS = {0x02: "lt", 0x05: "rt"}
VENDORS = {"054c": "sony", "045e": "xbox", "057e": "nintendo", "28de": "generic"}
EVENT = struct.calcsize("llHHi")
BTN_GAMEPAD = 0x130
REPEAT_DELAY, REPEAT_RATE = 0.35, 0.16
HOLD = 0.6

def is_gamepad(node):
    try:
        with open(f"/sys/class/input/{os.path.basename(node)}/device/capabilities/key") as f:
            bits = 0
            for word in f.read().split():
                bits = (bits << 64) | int(word, 16)
    except OSError:
        return False
    return bool(bits >> BTN_GAMEPAD & 1)

def abs_range(fd, axis):
    # EVIOCGABS(axis): _IOR('E', 0x40 + axis, struct input_absinfo)
    buf = bytearray(24)
    try:
        fcntl.ioctl(fd, (2 << 30) | (24 << 16) | (0x45 << 8) | (0x40 + axis), buf)
        _, lo, hi = struct.unpack("iii", bytes(buf[:12]))
        return (lo, hi) if hi > lo else (-32768, 32767)
    except OSError:
        return (-32768, 32767)

def emit(name):
    sys.stdout.write(name + "\n")
    sys.stdout.flush()

pads = {}          # fd -> { path, ranges }
held = {}          # (fd, axis) -> (direction, next_repeat)
down = {}          # (fd, button) -> when pressed, until the hold line is sent

def scan():
    for path in glob.glob("/dev/input/event*"):
        if any(p["path"] == path for p in pads.values()) or not is_gamepad(path):
            continue
        try:
            fd = os.open(path, os.O_RDONLY | os.O_NONBLOCK)
        except OSError:
            continue
        pads[fd] = {"path": path, "ranges": {a: abs_range(fd, a) for a in (0, 1, 2, 3, 4, 5)}, "trig": set()}
        try:
            vendor = open(f"/sys/class/input/{os.path.basename(path)}/device/id/vendor").read().strip().lower()
        except OSError:
            vendor = ""
        emit("pad:" + VENDORS.get(vendor, "generic"))

def drop(fd):
    os.close(fd)
    pads.pop(fd, None)
    for k in [k for k in held if k[0] == fd]:
        held.pop(k)
    for k in [k for k in down if k[0] == fd]:
        down.pop(k)

def direction(fd, axis, value):
    if axis in (0x10, 0x11):
        d = -1 if value < 0 else 1 if value > 0 else 0
    else:
        lo, hi = pads[fd]["ranges"][axis]
        mid, half = (lo + hi) / 2, (hi - lo) / 2 or 1
        n = (value - mid) / half
        d = -1 if n < -0.5 else 1 if n > 0.5 else 0
    horizontal = axis in (0x00, 0x03, 0x10)
    name = None if d == 0 else ("left" if d < 0 else "right") if horizontal else ("up" if d < 0 else "down")
    return name if name is None or axis not in (0x03, 0x04) else "r" + name

last_scan = 0
while True:
    now = time.monotonic()
    if now - last_scan > 2:
        scan()
        last_scan = now
    timeout = 0.05 if held or down else 0.5
    readable, _, _ = select.select(list(pads), [], [], timeout)
    for fd in readable:
        try:
            data = os.read(fd, EVENT * 64)
        except OSError:
            drop(fd)
            continue
        for off in range(0, len(data) - EVENT + 1, EVENT):
            _, _, etype, code, value = struct.unpack_from("llHHi", data, off)
            if etype == EV_KEY and code in BTN:
                if value == 1:
                    emit(BTN[code]); down[(fd, code)] = time.monotonic()
                elif value == 0:
                    down.pop((fd, code), None)
            elif etype == EV_ABS and code in TRIGGERS:
                lo, hi = pads[fd]["ranges"][code]
                pressed = value > (lo + hi) / 2
                if pressed and code not in pads[fd]["trig"]:
                    pads[fd]["trig"].add(code); emit(TRIGGERS[code])
                elif not pressed:
                    pads[fd]["trig"].discard(code)
            elif etype == EV_ABS and code in AXES:
                d = direction(fd, code, value)
                key = (fd, code)
                if d is None:
                    held.pop(key, None)
                elif held.get(key, (None,))[0] != d:
                    held[key] = (d, time.monotonic() + REPEAT_DELAY)
                    emit(d)
    now = time.monotonic()
    for key, at in list(down.items()):
        if now - at >= HOLD:
            emit(BTN[key[1]] + "-hold"); down.pop(key)
    for key, (d, at) in list(held.items()):
        if now >= at:
            held[key] = (d, now + REPEAT_RATE)
            emit(d)
