#!/usr/bin/env python3
"""Which key carries a symbol, under a layout, as JSON: { "grave": "grave", "a": "a", ... }, by the kernel's
name for the key, which is what a remap layer matches.

A remap layer matches and emits kernel keys, while a chord is written as a symbol, and where a symbol sits
moves with the layout: AZERTY's A is the key QWERTY calls Q, and a Macintosh variant puts grave beside the
left Shift rather than left of the 1. The keymap the layout compiles to says where each one is.

    keysyms.py gb                 the layout's own
    keysyms.py gb mac             a variant of it
    keysyms.py gb,us mac,         several, the first that carries a symbol wins

Only symbols a key gives unshifted are listed: one that needs Shift is not a key on its own, and a chord
naming it would be missing a modifier nobody wrote.
"""
import json, re, subprocess, sys

# The keymap gives XKB's codes, which are the kernel's plus eight.
XKB_OFFSET = 8
NAME = re.compile(r"^\s*<(\w+)>\s*=\s*(\d+);")
CODES = "/usr/include/linux/input-event-codes.h"
DEFINE = re.compile(r"^#define\s+KEY_([A-Z0-9_]+)\s+(\d+)\b")


def key_names():
    """The kernel's own name for each key, lowercased as a remap layer spells them."""
    out = {}
    try:
        for line in open(CODES):
            m = DEFINE.match(line)
            if m and int(m.group(2)) < 256:
                out.setdefault(int(m.group(2)), m.group(1).lower())
    except OSError:
        pass
    return out
KEY_START = re.compile(r"^\s*key\s*<(\w+)>\s*\{")
LEVELS = re.compile(r"\[([^\]]*)\]")
INDEX = re.compile(r"\w+\[\d+\]")


def keymap(layout, variant):
    args = ["xkbcli", "compile-keymap", "--layout", layout]
    if variant:
        args += ["--variant", variant]
    try:
        p = subprocess.run(args, capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return ""
    return p.stdout if p.returncode == 0 else ""


def symbols(text):
    """symbol -> kernel key code, for the first level of each key. A key's levels are written on its own line
    or spread over a block, so the block is followed to its end."""
    codes, out, key, levels = {}, {}, "", None
    for line in text.splitlines():
        m = NAME.match(line)
        if m:
            codes[m.group(1)] = int(m.group(2)) - XKB_OFFSET
            continue
        m = KEY_START.match(line)
        if m:
            key, levels = m.group(1), None
        if not key:
            continue
        if levels is None:
            # "symbols[1]= [ ... ]" carries a bracket of its own before the levels.
            m = LEVELS.search(INDEX.sub("symbols", line))
            if m:
                levels = [t.strip() for t in m.group(1).split(",")]
        if "}" in line:
            first = levels[0] if levels else ""
            # NoSymbol and the like say the key carries nothing there.
            if first and key in codes and first not in ("NoSymbol", "VoidSymbol") and not first.startswith("Any"):
                # A symbol can sit on more than one key, Print on the print-screen key and again out in the
                # far ranges: the one in the ordinary block, the lower code, is the one a person presses.
                if first not in out or codes[key] < out[first]:
                    out[first] = codes[key]
            key, levels = "", None
    return out


def main(argv):
    layouts = (argv[0] if argv else "us").split(",")
    variants = (argv[1] if len(argv) > 1 else "").split(",")
    names = key_names()
    if not names:
        sys.stderr.write("no key names: %s is not there\n" % CODES)
        return 1
    out = {}
    for i, layout in enumerate(layouts):
        variant = variants[i] if i < len(variants) else ""
        text = keymap(layout.strip(), variant.strip())
        if not text:
            continue
        for sym, code in symbols(text).items():
            # A symbol on a key the kernel does not name is one no rule can ask for.
            if code in names:
                out.setdefault(sym, names[code])
    if not out:
        sys.stderr.write("no keymap compiled; is xkbcli there?\n")
        return 1
    print(json.dumps(out))
    return 0


sys.exit(main(sys.argv[1:]))
