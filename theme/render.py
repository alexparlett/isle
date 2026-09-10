#!/usr/bin/env python3
"""Renders shell/theme/tokens.json (plus the accent preference) into the installed apps' theme files.

    theme/render.py            write every target
    theme/render.py --dry      print what would be written

Templates live in theme/templates; a `{{name}}` is a token. Targets are listed in TARGETS.
"""
import shutil, json, os, re, sys, subprocess

HERE = os.path.dirname(os.path.realpath(__file__))
REPO = os.path.dirname(HERE)
HOME = os.path.expanduser("~")
CFG = os.environ.get("XDG_CONFIG_HOME", HOME + "/.config")

tokens = json.load(open(os.path.join(REPO, "shell/theme/tokens.json")))
try:
    prefs = json.load(open(os.path.join(CFG, "isle/prefs.json")))
except Exception:
    prefs = {}

def _light():
    mode = prefs.get("theme", "dark")
    if mode != "auto":
        return mode == "light"
    import datetime, sun
    now = datetime.datetime.now()
    s = sun.today()
    return s["rise"] <= now.hour * 60 + now.minute < s["set"]

LIGHT = _light()
c = dict(tokens["color"])
if LIGHT:
    c.update(tokens.get("light", {}))

def hex_to_rgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))

def mix(a, b, t):
    ra, rb = hex_to_rgb(a), hex_to_rgb(b)
    return "#%02X%02X%02X" % tuple(round(ra[i] + (rb[i] - ra[i]) * t) for i in range(3))

def alpha(h, a):
    r, g, b = hex_to_rgb(h)
    return f"rgba({r}, {g}, {b}, {a:.2f})"

def hexa(h, a):
    return h + "%02X" % round(a * 255)

accent = prefs.get("accent") or c["accent"]
text = c["text"]
ink = c["ink"]
window = "#FFFFFF" if LIGHT else "#131417"

V = {
    "ink": ink,
    "window": window,
    "raised": c["raised"],
    "pressed": c["pressed"],
    "text": text,
    "text2": mix(text, window, 1 - c["text2Alpha"]),
    "text3": mix(text, window, 1 - c["text3Alpha"]),
    "hairline": mix(text, window, 1 - c["hairlineAlpha"]),
    "hairline_strong": mix(text, window, 1 - c["hairlineStrongAlpha"]),
    "hairline_rgba": alpha(text, c["hairlineAlpha"]),
    "accent": accent,
    "accent_hover": mix(accent, text, 0.12),
    "on_accent": c["onAccent"],
    "ok": c["ok"],
    "warn": c["warn"],
    "danger": c["danger"],
    "live": c["live"],
    "font_ui": tokens["font"]["ui"],
    "font_mono": tokens["font"]["mono"],
    "radius_control": tokens["radius"]["control"],
    "radius_card": tokens["radius"]["card"],
    "blur_size": tokens["blur"]["size"],
    "blur_passes": tokens["blur"]["passes"],
    "gtk_theme": "adw-gtk3" if LIGHT else "adw-gtk3-dark",
    "icon_theme": "Papirus-Light" if LIGHT else "Papirus-Dark",
    "prefer_dark": "0" if LIGHT else "1",
    "border_active": hexa(text, c["hairlineStrongAlpha"])[1:],
    "border_inactive": hexa(text, c["hairlineAlpha"])[1:],
    "pressed": c["pressed"],
    "bars_off": "true" if prefs.get("titleBars", True) is False else "false",
}
# Terminal palette: near-neutral base, accent for blue, token semantics for the rest.
V.update({
    "term0": "#E8E8EB" if LIGHT else "#1B1C21", "term8": "#C8C8CE" if LIGHT else "#3A3C44",
    "term1": c["danger"], "term9": mix(c["danger"], text, 0.2),
    "term2": c["ok"], "term10": mix(c["ok"], text, 0.2),
    "term3": c["warn"], "term11": mix(c["warn"], text, 0.2),
    "term4": accent, "term12": mix(accent, text, 0.2),
    "term5": "#C8B5FF", "term13": "#D8CAFF",
    "term6": "#8FD3C5", "term14": "#A9E2D7",
    "term7": V["text2"], "term15": text,
})

# (template, target, post-step)
TARGETS = [
    ("gtk.css", CFG + "/gtk-3.0/gtk.css", None),
    ("gtk.css", CFG + "/gtk-4.0/gtk.css", None),
    ("settings.ini", CFG + "/gtk-3.0/settings.ini", None),
    ("settings.ini", CFG + "/gtk-4.0/settings.ini", None),
    ("qt6ct.conf", CFG + "/qt6ct/qt6ct.conf", None),
    ("qt-colors.conf", CFG + "/qt6ct/colors/isle.conf", None),
    ("kitty.conf", CFG + "/kitty/isle.conf", "kitty"),
    ("yazi-theme.toml", CFG + "/yazi/theme.toml", None),
    ("btop.theme", CFG + "/btop/themes/isle.theme", "btop"),
    ("portals.conf", CFG + "/xdg-desktop-portal/portals.conf", None),
    ("hypr-theme.lua", REPO + "/hypr/generated/theme.lua", None),
]

def render(name):
    src = open(os.path.join(HERE, "templates", name)).read()
    return re.sub(r"\{\{(\w+)\}\}", lambda m: str(V[m.group(1)]), src)

def ensure_line(path, line):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    cur = open(path).read() if os.path.exists(path) else ""
    if line not in cur:
        with open(path, "a") as f:
            f.write(("" if cur.endswith("\n") or not cur else "\n") + line + "\n")

# Chromium-based browsers and Electron default to X11 unless told to follow the session; each reads a flags
# file, and a hint is added to the ones for browsers that are installed, when nothing about ozone is set.
BROWSER_FLAGS = [("vivaldi-stable", "vivaldi-stable.conf"), ("chromium", "chromium-flags.conf"), ("brave", "brave-flags.conf"),
                 ("google-chrome-stable", "chrome-flags.conf"), ("microsoft-edge-stable", "microsoft-edge-stable-flags.conf")]

def browser_hints():
    # The Qt line: on Wayland Chromium points its Qt toolkit integration at the Wayland platform, and Qt 6 is the
    # one the desktop themes and ships the plugin for.
    for binary, conf in BROWSER_FLAGS:
        if not shutil.which(binary):
            continue
        path = os.path.join(CFG, conf)
        cur = open(path).read() if os.path.exists(path) else ""
        add = []
        if "ozone" not in cur:
            add.append("--ozone-platform-hint=auto")
        if "qt-version" not in cur:
            add.append("--qt-version=6")
        if not add:
            continue
        with open(path, "a") as f:
            f.write(("" if cur.endswith("\n") or not cur else "\n") + "# Wayland when the session is Wayland, with Qt 6; added by Isle.\n" + "\n".join(add) + "\n")

dry = "--dry" in sys.argv
if not dry:
    browser_hints()
for tmpl, target, post in TARGETS:
    out = render(tmpl)
    if dry:
        print(f"--- {target}\n{out}")
        continue
    os.makedirs(os.path.dirname(target), exist_ok=True)
    old = open(target).read() if os.path.exists(target) else None
    if old != out:
        open(target, "w").write(out)
    if post == "kitty":
        ensure_line(CFG + "/kitty/kitty.conf", "include isle.conf")
    elif post == "btop":
        conf = CFG + "/btop/btop.conf"
        cur = open(conf).read() if os.path.exists(conf) else ""
        if 'color_theme = "isle"' not in cur:
            cur = re.sub(r'(?m)^color_theme = .*$', "", cur)
            open(conf, "w").write(cur.rstrip("\n") + '\ncolor_theme = "isle"\n' if cur else 'color_theme = "isle"\n')

if not dry:
    # libadwaita and GTK4 read these from gsettings, not settings.ini.
    for key, val in [("color-scheme", "prefer-light" if LIGHT else "prefer-dark"), ("gtk-theme", V["gtk_theme"]), ("icon-theme", V["icon_theme"]),
                     ("cursor-theme", "Bibata-Modern-Classic"), ("cursor-size", "24"), ("font-name", f"{V['font_ui']} 10"),
                     ("monospace-font-name", f"{V['font_mono']} 10")]:
        subprocess.run(["gsettings", "set", "org.gnome.desktop.interface", key, val], capture_output=True)
    # A running kitty reloads its colours on SIGUSR1; Hyprland re-reads the generated theme, title bars included.
    subprocess.run(["pkill", "-USR1", "-x", "kitty"], capture_output=True)
    if os.environ.get("HYPRLAND_INSTANCE_SIGNATURE"):
        subprocess.run(["hyprctl", "reload"], capture_output=True)
    print("theme rendered")
