#!/usr/bin/env python3
"""Emoji and symbols for the launcher, as JSON [{c, name, words}]: the emoji from unicode-emoji's
emoji-test.txt when it is installed (every fully-qualified sequence, its group and subgroup as words), else
the emoji blocks of the Unicode data Python carries; then the symbols worth typing: arrows, mathematics,
currency, punctuation, Greek, box and shape characters, by their Unicode names."""
import json, os, sys, unicodedata

out, seen = [], set()


def add(c, name, words=""):
    if c in seen or not name:
        return
    seen.add(c)
    out.append({"c": c, "name": name.lower(), "words": words.lower()})


test = next((p for p in ["/usr/share/unicode/emoji/emoji-test.txt", "/usr/share/unicode-emoji/emoji-test.txt", "/usr/share/unicode/emoji-test.txt"] if os.path.exists(p)), None)
if test:
    group = sub = ""
    for line in open(test, encoding="utf-8"):
        if line.startswith("# group:"):
            group = line.split(":", 1)[1].strip()
        elif line.startswith("# subgroup:"):
            sub = line.split(":", 1)[1].strip().replace("-", " ")
        elif ";" in line and "fully-qualified" in line and "#" in line:
            codes, rest = line.split(";", 1)
            after = rest.split("#", 1)[1].strip()
            # "😀 E1.0 grinning face"
            parts = after.split(" ", 2)
            if len(parts) == 3:
                add(parts[0], parts[2], group + " " + sub)
else:
    for lo, hi in [(0x1F300, 0x1F5FF), (0x1F600, 0x1F64F), (0x1F680, 0x1F6FF), (0x1F900, 0x1F9FF), (0x1FA70, 0x1FAFF), (0x2600, 0x26FF), (0x2700, 0x27BF)]:
        for cp in range(lo, hi + 1):
            c = chr(cp)
            if unicodedata.category(c) == "So":
                add(c, unicodedata.name(c, ""), "emoji")

for lo, hi, words in [(0x2190, 0x21FF, "arrow"), (0x2200, 0x22FF, "math"), (0x20A0, 0x20BF, "currency"), (0x2010, 0x2027, "punctuation"),
                      (0x2030, 0x205E, "punctuation"), (0x00A1, 0x00BF, "latin symbol"), (0x00D7, 0x00D7, "math"), (0x00F7, 0x00F7, "math"),
                      (0x0391, 0x03C9, "greek"), (0x2500, 0x257F, "box drawing"), (0x25A0, 0x25FF, "shape"), (0x2100, 0x214F, "letterlike"),
                      (0x2150, 0x218F, "number fraction"), (0x2460, 0x24FF, "enclosed number"), (0x2300, 0x23FF, "technical")]:
    for cp in range(lo, hi + 1):
        c = chr(cp)
        if unicodedata.category(c)[0] in ("S", "P", "L", "N"):
            add(c, unicodedata.name(c, ""), words)

json.dump(out, sys.stdout, ensure_ascii=False)
