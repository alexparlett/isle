#!/usr/bin/env python3
"""Checks every text and status pair in tokens.kdl against WCAG AA, per appearance."""

import pathlib
import re
import sys

TOKENS = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else pathlib.Path(__file__).with_name("tokens.kdl"))

# Text roles need 4.5:1; marks, focus rings and large text 3:1.
TEXT = ["text", "text-2", "text-3"]
MARKS = ["ok", "warn", "danger", "live", "privacy-microphone", "privacy-camera", "privacy-screen"]
# Borders that identify a control need 3:1 only under Increase Contrast.
BORDERS = ["hairline-strong"]
GREY = (128, 128, 128, 1)
EXTREMES = (("black", (0, 0, 0, 1)), ("grey", GREY), ("white", (255, 255, 255, 1)))
SURFACES = ["window", "window-raised", "window-sunken"]
# Every surface a window draws its content and chrome on.
WINDOW = SURFACES + ["view", "sidebar", "titlebar"]
# Data colours are marks on every window surface; series also on solid wells.
DATA_SURFACES = SURFACES + ["view", "sidebar"]
# Fills that carry a label or a selection, and which text roles sit on them.
FILLS = {
    "control": ["text", "text-2", "danger-text"],
    "control-hover": ["text"],
    "control-pressed": ["text"],
    "fill": ["text", "text-2", "text-3"],
    "fill-strong": ["text", "text-2"],
    "selection-unfocused": ["text", "text-2"],
}


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


def mix(a, b, share):
    """`b` mixed into `a` at `share`, as CSS color-mix in sRGB."""
    return tuple(a[i] * (1 - share) + b[i] * share for i in range(3)) + (1.0,)


def lum(c):
    def ch(x):
        x /= 255
        return x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4
    r, g, b = (ch(x) for x in c[:3])
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def on_fill(fg, fill, base):
    """`fg` on a translucent `fill` laid over `base`."""
    f = over(fill, base)
    return ratio(over(fg, f), f)


def accent_rows(t, name, theme, col):
    """Pairs every accent must pass, not only the default."""
    fill, on, text = (rgba(t[f"accent.{name}.{theme}.{k}"]) for k in ("fill", "on", "text"))
    # A default button's hover and pressed fills mix the inverse of `on` into the accent.
    away = (255 - on[0], 255 - on[1], 255 - on[2], 1.0)
    rows = [
        (f"{name} on", "its fill", ratio(on, fill), 4.5),
        (f"{name} on", "hover fill", ratio(on, mix(fill, away, float(t["accent.hover-mix"]))), 4.5),
        (f"{name} on", "pressed fill", ratio(on, mix(fill, away, float(t["accent.pressed-mix"]))), 4.5),
        (f"{name} text", "window", ratio(text, rgba(col["window"])), 4.5),
    ]
    view = rgba(col["view"])
    sel = over((*fill[:3], float(t[f"selection.mix.{theme}"])), view)
    rows.append(("text", f"{name} text selection", ratio(rgba(col["text"]), sel), 4.5))
    # The focus ring is the full accent round a control in a window or on a solid well.
    for sname in WINDOW + ["solid"]:
        s = rgba(t[f"material.solid.{theme}"]) if sname == "solid" else rgba(col[sname])
        rows.append((f"{name} focus ring", sname, ratio(fill, s), 3.0))
    return rows


def main():
    t = parse(TOKENS.read_text())
    failures = 0
    accents = sorted({k.split(".")[1] for k in t if k.startswith("accent.") and k.count(".") == 3})
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
            for extreme, base in EXTREMES:
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
                if contrast:
                    for role in BORDERS:
                        rows.append((role, sname, ratio(over(rgba(col[role]), s), s), 3.0))
                if not sname.startswith("glass"):
                    rows.append(("accent text", sname, ratio(rgba(accent["text"]), s), 4.5))
                    for role in MARKS:
                        rows.append((role, sname, ratio(over(rgba(col[role]), s), s), 3.0))
                else:
                    # On glass the focus ring carries a colour-text line inside it.
                    rows.append(("focus line", sname, ratio(rgba(col["text"]), s), 3.0))
                    # Tiles, fields and quiet badges on glass: text on either
                    # fill, text-2 on `fill` only (schema.md).
                    for fill in ("fill", "fill-strong"):
                        rows.append(("text", f"{fill} on {sname}", on_fill(rgba(col["text"]), rgba(col[fill]), s), 4.5))
                    if contrast or sname.endswith((own, "grey")):
                        rows.append(("text-2", f"fill on {sname}", on_fill(rgba(col["text-2"]), rgba(col["fill"]), s), 4.5))
            for sname in ("titlebar", "view", "sidebar"):
                s = rgba(col[sname])
                if contrast:
                    rows.append(("hairline-strong", sname, ratio(over(rgba(col["hairline-strong"]), s), s), 3.0))
            view = rgba(col["view"])
            for role in TEXT + ["ok-text", "warn-text", "danger-text"]:
                rows.append((role, "view", ratio(over(rgba(col[role]), view), view), 4.5))
            for role in ["ok-text", "warn-text", "danger-text"]:
                for sname in SURFACES:
                    s_ = rgba(col[sname])
                    rows.append((role, sname, ratio(rgba(col[role]), s_), 4.5))
            # Labels, placeholders and destructive labels on controls and fills in windows.
            for fill, roles in FILLS.items():
                for sname in WINDOW:
                    base = rgba(col[sname])
                    for role in roles:
                        rows.append((role, f"{fill} on {sname}", on_fill(rgba(col[role]), rgba(col[fill]), base), 4.5))
            # Badges: the count on `danger`.
            rows.append(("on-danger", "danger", ratio(rgba(col["on-danger"]), rgba(col["danger"])), 4.5))
            # A search match: text unchanged on the highlight run.
            for sname in WINDOW:
                rows.append(("text", f"match on {sname}", on_fill(rgba(col["text"]), rgba(col["match"]), rgba(col[sname])), 4.5))
            for name in accents:
                rows += accent_rows(t, name, theme, col)
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
                for group, extra in (("series", ["solid"]), ("kind", [])):
                    marks = {k.split(".")[-1]: v for k, v in t.items() if k.startswith(f"{group}.{theme}.")}
                    for name, val in marks.items():
                        for sname in DATA_SURFACES + extra:
                            s_ = rgba(t[f"material.solid.{theme}"]) if sname == "solid" else rgba(col[sname])
                            rows.append((f"{group} {name}", sname, ratio(rgba(val), s_), 3.0))
                # Cleaner: a kind tile's glyph in `view` on the kind; a map tile
                # is the kind mixed into `view`, with text on it and a band of the kind.
                share = float(t["kind.mix"])
                for k, val in t.items():
                    if k.startswith(f"kind.{theme}."):
                        name, kc = k.split(".")[-1], rgba(val)
                        tile = mix(view, kc, share)
                        rows.append(("view glyph", f"kind {name}", ratio(view, kc), 3.0))
                        rows.append(("text", f"map tile {name}", ratio(rgba(col["text"]), tile), 4.5))
                        rows.append(("text-2", f"map tile {name}", ratio(over(rgba(col["text-2"]), tile), tile), 4.5))
                        rows.append((f"band {name}", "its map tile", ratio(kc, tile), 3.0))
            for extreme, base in EXTREMES:
                isl = over(rgba(col["island"]), base)
                rows.append(("island text", f"island over {extreme}", ratio(rgba(col["island-text"]), isl), 4.5))
                if extreme in (own, "grey") or contrast:
                    rows.append(("text-2", f"island over {extreme}", ratio(over(rgba(col["text-2"]), isl), isl), 4.5))
            # The lock: softened wallpaper plus the lock's wash, over any picture.
            wash = t[f"material.lock-wash-contrast.{theme}"] if contrast else t[f"material.lock-wash.{theme}"]
            for extreme, base in EXTREMES:
                lock = over(rgba(wash), over(rgba(t[f"material.softened.{theme}"]), base))
                rows.append(("text", f"lock over {extreme}", ratio(rgba(col["text"]), lock), 4.5))
                if extreme in (own, "grey") or contrast:
                    rows.append(("text-2", f"lock over {extreme}", ratio(over(rgba(col["text-2"]), lock), lock), 4.5))
            # Colour on glass sits in solid wells: the island's marks, the lock
            # keyboard's Caps Lock dot, status marks (surfaces/island.md, keyboard-osk.md).
            well = rgba(t[f"material.solid.{theme}"])
            for role in ["privacy-microphone", "privacy-camera", "privacy-screen", "live", "ok", "warn", "danger"]:
                rows.append((role, "solid well", ratio(rgba(col[role]), well), 3.0))
            bad = [r for r in rows if r[2] < r[3]]
            failures += len(bad)
            print(f"{label}: {len(rows) - len(bad)}/{len(rows)} pass")
            for role, sname, r, need in bad:
                print(f"  FAIL {role} on {sname}: {r:.2f} < {need}")
    print("all pass" if not failures else f"{failures} failing")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
