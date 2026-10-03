#!/usr/bin/env python3
"""Checks every text and status pair in tokens.kdl against WCAG AA, per appearance."""

import pathlib
import re
import sys

TOKENS = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else pathlib.Path(__file__).with_name("tokens.kdl"))

# Text roles need 4.5:1; marks and large text 3:1.
TEXT = ["text", "text-2", "text-3"]
MARKS = ["ok", "warn", "danger", "live", "privacy-microphone", "privacy-camera", "privacy-screen"]
# Borders that identify a control need 3:1 only under Increase Contrast.
BORDERS = ["hairline-strong"]
GREY = (128, 128, 128, 1)
SURFACES = ["window", "window-raised", "window-sunken"]


def parse(src):
    """Flattens the subset of KDL the token file uses into {path: value}."""
    out, stack = {}, []
    src = re.sub(r"//[^\n]*", "", src)
    for tok in re.finditer(r'"[^"]*"|[{};]|[^\s{};"]+', src):
        t = tok.group(0)
        if t == "{":
            stack.append(pending.pop() if pending else None)
            continue
        if t == "}":
            stack.pop()
            pending.clear()
            continue
        if t == ";" or t == "\n":
            continue
        if not pending or pending[-1] is None:
            pending[:] = [t]
        else:
            name = pending[0]
            out[".".join([s for s in stack if s] + [name])] = t.strip('"')
            pending[:] = [None]
    return out


pending = []


def rgba(v):
    v = v.strip()
    if v.startswith("#"):
        h = v[1:]
        return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4)) + (1.0,)
    m = re.match(r"rgba?\(([^)]*)\)", v)
    parts = [p.strip() for p in m.group(1).split(",")]
    return tuple(float(p) for p in parts[:3]) + ((float(parts[3]) if len(parts) > 3 else 1.0),)


def over(fg, bg):
    a = fg[3]
    return tuple(fg[i] * a + bg[i] * (1 - a) for i in range(3)) + (1.0,)


def lum(c):
    def ch(x):
        x /= 255
        return x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4
    r, g, b = (ch(x) for x in c[:3])
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def main():
    t = parse(TOKENS.read_text())
    failures = 0
    for theme in ("dark", "light"):
        for contrast in (False, True):
            col = {k.split(".")[-1]: v for k, v in t.items() if k.startswith(f"colour.{theme}.")}
            if contrast:
                col.update({k.split(".")[-1]: v for k, v in t.items() if k.startswith(f"colour.contrast.{theme}.")})
            label = theme + (" + increased contrast" if contrast else "")
            accent = {k.split(".")[-1]: v for k, v in t.items() if re.match(rf"accent\.\w+\.{theme}\.", k) and k.split(".")[1] == t.get("accent.default", "blue")}
            surfaces = {s: rgba(col[s]) for s in SURFACES}
            surfaces["solid"] = rgba(t[f"material.solid.{theme}"])
            # Glass is checked over the extremes of any wallpaper. By default
            # primary text must pass over all of them and secondary text over
            # grey and the appearance's own extreme; Increase Contrast uses
            # the tinted glass and every role must pass everywhere.
            glass = t[f"material.glass-tinted.{theme}"] if contrast else t[f"material.glass.{theme}"]
            own = "black" if theme == "dark" else "white"
            for extreme, base in (("black", (0, 0, 0, 1)), ("grey", GREY), ("white", (255, 255, 255, 1))):
                surfaces[f"glass over {extreme}"] = over(rgba(glass), base)
            rows = []
            for sname, s in surfaces.items():
                for role in TEXT:
                    # Tertiary text is for windows only; glass carries text and text-2.
                    if sname.startswith("glass") and role == "text-3":
                        continue
                    if sname.startswith("glass") and role != "text" and not contrast and not sname.endswith((own, "grey")):
                        continue
                    rows.append((role, sname, ratio(over(rgba(col[role]), s), s), 4.5))
                if contrast and not sname.startswith("glass"):
                    for role in BORDERS:
                        rows.append((role, sname, ratio(over(rgba(col[role]), s), s), 3.0))
                if not sname.startswith("glass"):
                    rows.append(("accent text", sname, ratio(rgba(accent["text"]), s), 4.5))
                    for role in MARKS:
                        rows.append((role, sname, ratio(over(rgba(col[role]), s), s), 3.0))
            rows.append(("accent on", "accent fill", ratio(rgba(accent["on"]), rgba(accent["fill"])), 4.5))
            view = rgba(col["view"])
            for role in TEXT + ["ok-text", "warn-text", "danger-text"]:
                rows.append((role, "view", ratio(over(rgba(col[role]), view), view), 4.5))
            for role in ["ok-text", "warn-text", "danger-text"]:
                for sname in SURFACES:
                    s_ = rgba(col[sname])
                    rows.append((role, sname, ratio(rgba(col[role]), s_), 4.5))
            sel = over((*rgba(accent["fill"])[:3], float(t[f"selection.mix.{theme}"])), view)
            rows.append(("text", "selection", ratio(rgba(col["text"]), sel), 4.5))
            if not contrast:
                win = rgba(col["window"])
                skip = {"black"} if theme == "dark" else {"white", "bright-white", "black"}
                for k, val in t.items():
                    if k.startswith(f"term.{theme}."):
                        name = k.split(".")[-1]
                        if name in skip:
                            continue
                        need = 3.0 if name == "bright-black" else 4.5
                        if theme == "dark" and name == "bright-white":
                            need = 4.5
                        rows.append((f"term {name}", "window", ratio(rgba(val), win), need))
            for extreme, base in (("black", (0, 0, 0, 1)), ("grey", GREY), ("white", (255, 255, 255, 1))):
                isl = over(rgba(col["island"]), base)
                rows.append(("island text", f"island over {extreme}", ratio(rgba(col["island-text"]), isl), 4.5))
                if extreme in (own, "grey") or contrast:
                    rows.append(("text-2", f"island over {extreme}", ratio(over(rgba(col["text-2"]), isl), isl), 4.5))
            # Colour on the island sits in solid wells (surfaces/island.md).
            well = rgba(t[f"material.solid.{theme}"])
            for role in ["privacy-microphone", "privacy-camera", "privacy-screen", "live"]:
                rows.append((role, "island well", ratio(rgba(col[role]), well), 3.0))
            bad = [r for r in rows if r[2] < r[3]]
            failures += len(bad)
            print(f"{label}: {len(rows) - len(bad)}/{len(rows)} pass")
            for role, sname, r, need in bad:
                print(f"  FAIL {role} on {sname}: {r:.2f} < {need}")
    for name in sorted({k.split(".")[1] for k in t if k.startswith("accent.") and k.count(".") == 3}):
        for theme in ("dark", "light"):
            fill, on, text = (rgba(t[f"accent.{name}.{theme}.{k}"]) for k in ("fill", "on", "text"))
            win = rgba(t[f"colour.{theme}.window"])
            r_on, r_text = ratio(on, fill), ratio(text, win)
            if r_on < 4.5 or r_text < 4.5:
                failures += 1
                print(f"  FAIL accent {name} {theme}: on {r_on:.2f}, text on window {r_text:.2f}")
    print("all pass" if not failures else f"{failures} failing")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
