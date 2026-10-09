#!/usr/bin/env python3
"""Writes emoji.tsv from Unicode's emoji-test.txt and UnicodeData.txt, with CLDR's keywords
when an annotations file is given.

    generate.py [--emoji-test PATH] [--unicode-data PATH] [--cldr annotations/en.xml]
"""
import argparse, os, xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))

# The symbol blocks Spotlight offers by name, v1's set.
SYMBOLS = [
    ("Arrows", 0x2190, 0x21FF),
    ("Mathematics", 0x2200, 0x22FF),
    ("Currency", 0x20A0, 0x20CF),
    ("Punctuation", 0x2010, 0x2027),
    ("Latin-1", 0x00A1, 0x00BF),
    ("Greek", 0x0391, 0x03C9),
    ("Letterlike", 0x2100, 0x214F),
    ("Number forms", 0x2150, 0x218F),
    ("Technical", 0x2300, 0x23FF),
    ("Enclosed", 0x2460, 0x24FF),
    ("Box drawing", 0x2500, 0x257F),
    ("Shapes", 0x25A0, 0x25FF),
]


def cldr_keywords(path):
    if not path:
        return {}
    out = {}
    for node in ET.parse(path).getroot().iter("annotation"):
        if node.get("type") == "tts" or not node.text:
            continue
        out[node.get("cp")] = [w.strip() for w in node.text.split("|") if w.strip()]
    return out


def emoji(path, keywords):
    groups, group, sub = [], None, None
    version = "?"
    by_name = {}
    for line in open(path, encoding="utf-8"):
        line = line.rstrip("\n")
        if line.startswith("# Version:"):
            version = line.split(":", 1)[1].strip()
        elif line.startswith("# group:"):
            group = (line.split(":", 1)[1].strip(), [])
            groups.append(group)
        elif line.startswith("# subgroup:"):
            sub = line.split(":", 1)[1].strip()
        elif line and not line.startswith("#") and "; fully-qualified" in line:
            comment = line.split("#", 1)[1].strip()
            text, rest = comment.split(" ", 1)
            name = rest.split(" ", 1)[1]
            if group[0] == "Component":
                continue
            head, _, tail = name.partition(": ")
            base = by_name.get(head) if "skin tone" in tail else None
            if base is not None:
                base["variants"].append((text, tail))
                continue
            entry = {"text": text, "name": name, "sub": sub, "variants": [], "keywords": keywords.get(text.replace("️", ""), keywords.get(text, []))}
            group[1].append(entry)
            by_name[name] = entry
    return version, [(g, entries) for g, entries in groups if entries]


def symbols(path):
    names = {}
    for line in open(path, encoding="utf-8"):
        fields = line.split(";")
        cp, name, category = int(fields[0], 16), fields[1], fields[2]
        if not name.startswith("<") and not category.startswith(("C", "Z", "M")):
            names[cp] = name.lower()
    out = []
    for title, start, end in SYMBOLS:
        out.append((title, [(chr(cp), names[cp]) for cp in range(start, end + 1) if cp in names]))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--emoji-test", default="/usr/share/unicode/emoji/emoji-test.txt")
    ap.add_argument("--unicode-data", default="/usr/share/unicode/UnicodeData.txt")
    ap.add_argument("--cldr")
    args = ap.parse_args()
    version, groups = emoji(args.emoji_test, cldr_keywords(args.cldr))
    # One line each, tab-separated: `group`/`symbols` and a name start a group; `e` is an
    # emoji with its name, subgroup and keywords; `v` a variant of the emoji above; `s` a symbol.
    lines = [f"# Emoji {version} and symbols, written by generate.py; regenerate rather than edit."]
    for title, entries in groups:
        lines.append(f"group\t{title}")
        for e in entries:
            lines.append("\t".join(["e", e["text"], e["name"], e["sub"], ";".join(e["keywords"])]))
            for text, name in e["variants"]:
                lines.append(f"v\t{text}\t{name}")
    for title, entries in symbols(args.unicode_data):
        lines.append(f"symbols\t{title}")
        for text, name in entries:
            lines.append(f"s\t{text}\t{name}")
    with open(os.path.join(HERE, "emoji.tsv"), "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")

if __name__ == "__main__":
    main()
