#!/usr/bin/env python3
"""A virtual gamepad through uinput, for driving Big Picture in the VM: run as root, then feed button names on stdin.

    sudo tools/fake-gamepad.py <<< $'right\na'
"""
import fcntl, os, struct, sys, time

UI_SET_EVBIT, UI_SET_KEYBIT, UI_SET_ABSBIT = 0x40045564, 0x40045565, 0x40045567
UI_DEV_CREATE, UI_DEV_DESTROY = 0x5501, 0x5502
UI_ABS_SETUP = 0x401c5504
UI_DEV_SETUP = 0x405c5503
EV_SYN, EV_KEY, EV_ABS = 0, 1, 3
KEYS = {"a": 0x130, "b": 0x131, "x": 0x133, "y": 0x134, "select": 0x13a, "start": 0x13b, "guide": 0x13c}
HATS = {"left": (0x10, -1), "right": (0x10, 1), "up": (0x11, -1), "down": (0x11, 1)}

fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
fcntl.ioctl(fd, UI_SET_EVBIT, EV_KEY)
fcntl.ioctl(fd, UI_SET_EVBIT, EV_ABS)
for code in KEYS.values():
    fcntl.ioctl(fd, UI_SET_KEYBIT, code)
for axis in (0x10, 0x11):
    fcntl.ioctl(fd, UI_SET_ABSBIT, axis)
    # struct uinput_abs_setup { __u16 code; struct input_absinfo { __s32 value, minimum, maximum, fuzz, flat, resolution; } }
    fcntl.ioctl(fd, UI_ABS_SETUP, struct.pack("H2xiiiiii", axis, 0, -1, 1, 0, 0, 0))
# struct uinput_setup { struct input_id { __u16 bustype, vendor, product, version; }; char name[80]; __u32 ff_effects_max; }
fcntl.ioctl(fd, UI_DEV_SETUP, struct.pack("HHHH80sI", 3, 0x28de, 0x11ff, 1, b"Isle Fake Gamepad", 0))
fcntl.ioctl(fd, UI_DEV_CREATE)
time.sleep(1.5)

def send(etype, code, value):
    os.write(fd, struct.pack("llHHi", 0, 0, etype, code, value))
    os.write(fd, struct.pack("llHHi", 0, 0, EV_SYN, 0, 0))

for line in sys.stdin:
    name = line.strip()
    if name in KEYS:
        send(EV_KEY, KEYS[name], 1); time.sleep(0.05); send(EV_KEY, KEYS[name], 0)
    elif name in HATS:
        axis, v = HATS[name]
        send(EV_ABS, axis, v); time.sleep(0.05); send(EV_ABS, axis, 0)
    elif name.startswith("sleep"):
        time.sleep(float(name.split()[1]))
    time.sleep(0.15)
time.sleep(0.3)
fcntl.ioctl(fd, UI_DEV_DESTROY)
