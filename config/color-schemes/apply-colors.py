#!/usr/bin/env python3
"""
Merge a KDE .colors scheme into ~/.config/kdeglobals.

`plasma-apply-colorscheme` does this job better, but it ships in
plasma-workspace — which is exactly the package you remove when you finish
migrating off Plasma. This is the fallback that keeps Dolphin, Ark, Okular and
every other KDE app matching the rest of the desktop afterwards.

Only the groups present in the scheme file are touched; anything else already
in kdeglobals (icon theme, single-click, your own tweaks) is preserved.

    apply-colors.py CatppuccinMocha.colors [--target ~/.config/kdeglobals]
"""

import argparse
import configparser
import shutil
import sys
from datetime import datetime
from pathlib import Path


def reader():
    # KDE INI is case-sensitive and contains % signs in some values, so both
    # of configparser's helpful behaviours have to be switched off.
    parser = configparser.RawConfigParser(interpolation=None, strict=False)
    parser.optionxform = str
    return parser


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("scheme", type=Path)
    ap.add_argument("--target", type=Path,
                    default=Path.home() / ".config" / "kdeglobals")
    ap.add_argument("--no-backup", action="store_true")
    args = ap.parse_args()

    if not args.scheme.is_file():
        sys.exit(f"no such scheme: {args.scheme}")

    scheme = reader()
    scheme.read(args.scheme, encoding="utf-8")

    target = reader()
    if args.target.is_file():
        target.read(args.target, encoding="utf-8")
        if not args.no_backup:
            stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
            backup = args.target.with_name(f"{args.target.name}.bak-{stamp}")
            shutil.copy2(args.target, backup)
            print(f"backed up {args.target.name} to {backup.name}")

    changed = 0
    for section in scheme.sections():
        # Colour groups plus [WM]; [General] is handled below so we only take
        # the one key we care about rather than the scheme's metadata.
        if not (section.startswith("Colors:") or section == "WM"):
            continue
        if not target.has_section(section):
            target.add_section(section)
        for key, value in scheme.items(section):
            target.set(section, key, value)
            changed += 1

    name = scheme.get("General", "ColorScheme", fallback=args.scheme.stem)
    if not target.has_section("General"):
        target.add_section("General")
    target.set("General", "ColorScheme", name)

    args.target.parent.mkdir(parents=True, exist_ok=True)
    with args.target.open("w", encoding="utf-8") as handle:
        target.write(handle, space_around_delimiters=False)

    print(f"applied {name} to {args.target} ({changed} keys)")
    print("Already-running KDE apps need a restart to pick it up.")


if __name__ == "__main__":
    main()
