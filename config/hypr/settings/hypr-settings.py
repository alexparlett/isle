#!/usr/bin/env python3
"""
Hyprland settings — a small GTK4 window for the knobs that are worth tuning by
feel rather than by editing a file and reloading.

Every change applies to the running compositor immediately via
`hyprctl eval 'hl.config{...}'`, which is how Hyprland 0.55+ takes runtime
config. Nothing is written to disk until you press Save, which writes
~/.config/hypr/local.lua — the git-ignored override file that hyprland.lua
requires last, so your saved values survive a restart without touching the
tracked config.

Deliberately only exposes settings that are safe to change live. Monitors,
keybinds and window rules are not here: they want a text editor and a diff,
not a slider.
"""

import json
import os
import re
import shutil
import subprocess
from pathlib import Path

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk, Gdk  # noqa: E402

LOCAL_LUA = Path.home() / ".config" / "hypr" / "local.lua"
MARKER = "-- managed by hypr-settings"
REBINDS = Path.home() / ".config" / "hypr" / "keybinds.conf"

# --------------------------------------------------------------------------
# Palette — Catppuccin Mocha, matching the rest of the desktop.
# --------------------------------------------------------------------------

CSS = b"""
window {
    background-color: #1e1e2e;
    color: #cdd6f4;
    font-family: "JetBrainsMono Nerd Font", "Noto Sans", sans-serif;
}
headerbar {
    background-color: #181825;
    color: #cdd6f4;
    border-bottom: 1px solid #313244;
    min-height: 44px;
}
headerbar button { background: #313244; color: #cdd6f4; border: none; border-radius: 9px; }
headerbar button:hover { background: #45475a; }
headerbar button.suggested-action { background: #cba6f7; color: #1e1e2e; font-weight: bold; }
headerbar button.suggested-action:hover { background: #b4befe; }

.group-title {
    font-size: 15px;
    font-weight: bold;
    color: #cba6f7;
    margin: 18px 4px 6px 4px;
}
.card {
    background-color: #181825;
    border: 1px solid #313244;
    border-radius: 14px;
    padding: 4px 6px;
}
row {
    padding: 10px 12px;
    border-radius: 10px;
    background: transparent;
}
.row-title { color: #cdd6f4; }
.row-subtitle { color: #6c7086; font-size: 11px; }
.hint { color: #6c7086; font-size: 11px; margin: 8px 6px; }

scale trough { background-color: #313244; border-radius: 999px; min-height: 5px; }
scale highlight { background-color: #cba6f7; border-radius: 999px; }
scale slider { background-color: #cdd6f4; border-radius: 999px; min-width: 15px; min-height: 15px; }
scale value { color: #a6adc8; font-size: 11px; }

switch { background-color: #313244; border-radius: 999px; }
switch:checked { background-color: #cba6f7; }
switch slider { background-color: #cdd6f4; border-radius: 999px; }

dropdown > button { background: #313244; color: #cdd6f4; border: none; border-radius: 9px; }
popover contents { background: #181825; border: 1px solid #313244; border-radius: 12px; color: #cdd6f4; }

button.link-tool {
    background: #313244;
    color: #cdd6f4;
    border: none;
    border-radius: 10px;
    padding: 9px 14px;
}
button.link-tool:hover { background: #45475a; color: #cba6f7; }
.saved { color: #a6e3a1; font-size: 11px; }
"""

# --------------------------------------------------------------------------
# The settings themselves.
#
#   key      hyprctl getoption path (colon form)
#   lua      where it lives in the hl.config table
#   kind     scale-int | scale-float | switch | choice
# --------------------------------------------------------------------------

SETTINGS = [
    ("Appearance", [
        dict(key="general:gaps_in", lua=("general", "gaps_in"), kind="scale-int",
             title="Inner gaps", subtitle="Between windows", min=0, max=30, step=1),
        dict(key="general:gaps_out", lua=("general", "gaps_out"), kind="scale-int",
             title="Outer gaps", subtitle="Between windows and the screen edge", min=0, max=60, step=1),
        dict(key="general:border_size", lua=("general", "border_size"), kind="scale-int",
             title="Border width", subtitle=None, min=0, max=8, step=1),
        dict(key="decoration:rounding", lua=("decoration", "rounding"), kind="scale-int",
             title="Corner rounding", subtitle=None, min=0, max=24, step=1),
        dict(key="decoration:inactive_opacity", lua=("decoration", "inactive_opacity"), kind="scale-float",
             title="Inactive window opacity", subtitle="1.0 is fully opaque", min=0.5, max=1.0, step=0.01),
    ]),
    ("Effects", [
        dict(key="decoration:blur:enabled", lua=("decoration", "blur", "enabled"), kind="switch",
             title="Blur", subtitle="Behind translucent windows and the bar"),
        dict(key="decoration:blur:size", lua=("decoration", "blur", "size"), kind="scale-int",
             title="Blur size", subtitle=None, min=1, max=20, step=1),
        dict(key="decoration:blur:passes", lua=("decoration", "blur", "passes"), kind="scale-int",
             title="Blur passes", subtitle="Costs GPU time; 3 is plenty", min=1, max=5, step=1),
        dict(key="decoration:shadow:enabled", lua=("decoration", "shadow", "enabled"), kind="switch",
             title="Shadows", subtitle=None),
        dict(key="animations:enabled", lua=("animations", "enabled"), kind="switch",
             title="Animations", subtitle=None),
    ]),
    ("Mouse", [
        dict(key="input:sensitivity", lua=("input", "sensitivity"), kind="scale-float",
             title="Pointer speed", subtitle="0 is the libinput default; negative is slower",
             min=-1.0, max=1.0, step=0.05),
        dict(key="input:accel_profile", lua=("input", "accel_profile"), kind="text-choice",
             title="Acceleration", subtitle="Flat is raw input — what you want for games",
             options=["flat", "adaptive"]),
        dict(key="input:natural_scroll", lua=("input", "natural_scroll"), kind="switch",
             title="Natural scrolling", subtitle="Content follows your fingers, macOS-style"),
        dict(key="input:scroll_factor", lua=("input", "scroll_factor"), kind="scale-float",
             title="Scroll speed", subtitle=None, min=0.1, max=3.0, step=0.1),
        dict(key="input:left_handed", lua=("input", "left_handed"), kind="switch",
             title="Swap mouse buttons", subtitle="Left-handed layout"),
    ]),
    ("Keyboard", [
        dict(key="input:repeat_delay", lua=("input", "repeat_delay"), kind="scale-int",
             title="Repeat delay", subtitle="Milliseconds before a held key repeats",
             min=150, max=1000, step=25),
        dict(key="input:repeat_rate", lua=("input", "repeat_rate"), kind="scale-int",
             title="Repeat rate", subtitle="Repeats per second once it starts",
             min=10, max=80, step=1),
        dict(key="input:numlock_by_default", lua=("input", "numlock_by_default"), kind="switch",
             title="Num Lock on at login", subtitle=None),
        dict(key="input:kb_layout", lua=("input", "kb_layout"), kind="text-choice",
             title="Layout", subtitle="Restart apps to pick up a change",
             options=["gb", "us", "de", "fr", "es", "it", "no", "se", "dk"]),
    ]),
    ("Behaviour", [
        dict(key="input:follow_mouse", lua=("input", "follow_mouse"), kind="choice",
             title="Focus follows mouse", subtitle=None,
             options=["Off", "Always", "Loose", "Click to focus"]),
        dict(key="misc:vrr", lua=("misc", "vrr"), kind="choice",
             title="Adaptive sync", subtitle="Fullscreen-only avoids desktop flicker on this panel",
             options=["Off", "Always on", "Fullscreen only", "Fullscreen games"]),
        dict(key="general:allow_tearing", lua=("general", "allow_tearing"), kind="switch",
             title="Allow tearing", subtitle="Games opt in individually; this is the master switch"),
        dict(key="misc:middle_click_paste", lua=("misc", "middle_click_paste"), kind="switch",
             title="Middle-click paste", subtitle="Primary selection"),
        dict(key="cursor:inactive_timeout", lua=("cursor", "inactive_timeout"), kind="scale-int",
             title="Hide cursor after", subtitle="Seconds of stillness; 0 never hides", min=0, max=20, step=1),
    ]),
]

# Hyprland has no System Settings and structurally cannot have KDE's: that is a
# shell over KConfig modules, where every component registers against a shared
# framework. Waybar, rofi, swaync and the rest are unrelated programs with
# unrelated config formats and no common schema, so there is nothing for one
# window to introspect.
#
# This is the pragmatic substitute — the Hyprland options worth tuning by feel
# live above, and everything else is one click away in the tool that actually
# owns it. Buttons for tools that are not installed are simply not shown.
EXTERNAL_TOOLS = [
    ("Displays", "nwg-displays"),
    ("GTK theme", "nwg-look"),
    ("Qt theme", "qt6ct"),
    ("Audio", "pavucontrol"),
    ("Bluetooth", "blueman-manager"),
    ("Network", "nm-connection-editor"),
    ("Notifications", "swaync-client -t -sw"),
    ("Printers", "system-config-printer"),
    ("Disks", "gnome-disk-utility"),
    ("Sensors & fans", "coolercontrol"),
    ("Updates", "alacritty -e cachy-update"),
    ("System info", "alacritty -e fastfetch"),
]


def hyprctl(args):
    """Run hyprctl, returning stdout or None. Never raises at the caller."""
    try:
        out = subprocess.run(["hyprctl", *args], capture_output=True, text=True, timeout=3)
        return out.stdout if out.returncode == 0 else None
    except (OSError, subprocess.SubprocessError):
        return None


def get_option(path):
    """Current value of a Hyprland option, or None if it can't be read."""
    raw = hyprctl(["getoption", path, "-j"])
    if not raw:
        return None
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        return None
    for field in ("int", "float", "str", "custom"):
        if field in data and data[field] is not None:
            return data[field]
    return None


def lua_literal(value):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, float):
        return f"{value:g}"
    if isinstance(value, str):
        return '"%s"' % value.replace('"', '\\"')
    return str(value)


def lua_assignment(path, value):
    """('decoration','blur','size'), 6  ->  decoration = { blur = { size = 6 } }"""
    body = lua_literal(value)
    for part in reversed(path[1:]):
        body = f"{part} = {body}"
        body = "{ " + body + " }"
    return f"{path[0]} = {body}" if len(path) == 1 else f"{path[0]} = {body}"


class SettingsWindow(Gtk.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="Hyprland Settings")
        self.set_default_size(680, 820)

        # widget -> spec, so Save can walk everything in one pass
        self.controls = {}
        self._pending = None

        header = Gtk.HeaderBar()
        header.set_show_title_buttons(True)
        self.set_titlebar(header)

        self.status = Gtk.Label(label="")
        self.status.add_css_class("saved")
        header.pack_start(self.status)

        revert = Gtk.Button(label="Reload from file")
        revert.connect("clicked", self.on_revert)
        header.pack_end(revert)

        save = Gtk.Button(label="Save")
        save.add_css_class("suggested-action")
        save.connect("clicked", self.on_save)
        header.pack_end(save)

        scroller = Gtk.ScrolledWindow(hexpand=True, vexpand=True)
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        self.set_child(scroller)

        page = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        page.set_margin_top(10)
        page.set_margin_bottom(20)
        page.set_margin_start(18)
        page.set_margin_end(18)
        scroller.set_child(page)

        if hyprctl(["version"]) is None:
            banner = Gtk.Label(label="Hyprland is not running — changes cannot be applied live.")
            banner.add_css_class("hint")
            banner.set_xalign(0)
            page.append(banner)

        for group, specs in SETTINGS:
            page.append(self._group_title(group))
            page.append(self._card(specs))

        page.append(self._group_title("Keyboard shortcuts"))
        page.append(self._keymap_card())
        page.append(self._shortcuts_card())

        page.append(self._group_title("Power and idle"))
        page.append(self._idle_card())

        page.append(self._group_title("Night light"))
        page.append(self._night_light_card())

        page.append(self._group_title("Elsewhere"))
        page.append(self._tools_card())

        hint = Gtk.Label(
            label="Monitors, keybinds and window rules live in ~/.config/hypr/*.lua — "
                  "they want a text editor, not a slider."
        )
        hint.add_css_class("hint")
        hint.set_xalign(0)
        hint.set_wrap(True)
        page.append(hint)

    # -- construction ------------------------------------------------------

    def _group_title(self, text):
        label = Gtk.Label(label=text)
        label.add_css_class("group-title")
        label.set_xalign(0)
        return label

    def _card(self, specs):
        box = Gtk.ListBox()
        box.set_selection_mode(Gtk.SelectionMode.NONE)
        box.add_css_class("card")
        for spec in specs:
            box.append(self._row(spec))
        return box

    def _row(self, spec):
        row = Gtk.ListBoxRow()
        row.set_activatable(False)

        line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        labels = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1, hexpand=True)

        title = Gtk.Label(label=spec["title"], xalign=0)
        title.add_css_class("row-title")
        labels.append(title)

        if spec.get("subtitle"):
            sub = Gtk.Label(label=spec["subtitle"], xalign=0)
            sub.add_css_class("row-subtitle")
            sub.set_wrap(True)
            labels.append(sub)

        line.append(labels)
        line.append(self._control(spec))
        row.set_child(line)
        return row

    def _control(self, spec):
        current = get_option(spec["key"])
        kind = spec["kind"]

        if kind == "switch":
            widget = Gtk.Switch(valign=Gtk.Align.CENTER)
            widget.set_active(bool(current))
            widget.connect("notify::active", lambda w, _p: self._changed(spec, w.get_active()))

        elif kind == "choice":
            widget = Gtk.DropDown.new_from_strings(spec["options"])
            widget.set_valign(Gtk.Align.CENTER)
            if isinstance(current, (int, float)) and 0 <= int(current) < len(spec["options"]):
                widget.set_selected(int(current))
            widget.connect("notify::selected", lambda w, _p: self._changed(spec, w.get_selected()))

        elif kind == "text-choice":
            # Same widget, but the option's *text* is the value rather than its
            # index — accel_profile wants "flat", not 0.
            widget = Gtk.DropDown.new_from_strings(spec["options"])
            widget.set_valign(Gtk.Align.CENTER)
            if isinstance(current, str) and current in spec["options"]:
                widget.set_selected(spec["options"].index(current))
            widget.connect(
                "notify::selected",
                lambda w, _p: self._changed(spec, spec["options"][w.get_selected()]),
            )

        else:
            is_float = kind == "scale-float"
            widget = Gtk.Scale.new_with_range(
                Gtk.Orientation.HORIZONTAL, spec["min"], spec["max"], spec["step"]
            )
            widget.set_size_request(270, -1)
            widget.set_draw_value(True)
            widget.set_value_pos(Gtk.PositionType.RIGHT)
            widget.set_digits(2 if is_float else 0)
            widget.set_valign(Gtk.Align.CENTER)
            if isinstance(current, (int, float)):
                widget.set_value(float(current))
            widget.connect(
                "value-changed",
                lambda w: self._changed(spec, w.get_value() if is_float else int(w.get_value())),
            )

        self.controls[id(widget)] = (spec, widget)
        return widget

    def _idle_card(self):
        """hypridle reads a file rather than taking runtime config, so these
        rewrite the timeouts in place and restart it. The agent-busy deferral
        and the rest of the file are left exactly as they are."""
        box = Gtk.ListBox()
        box.set_selection_mode(Gtk.SelectionMode.NONE)
        box.add_css_class("card")

        conf = Path.home() / ".config" / "hypr" / "hypridle.conf"
        try:
            body = conf.read_text()
        except OSError:
            body = ""

        # The two listeners in order: blank, then lock.
        timeouts = [int(t) for t in re.findall(r"^\s*timeout\s*=\s*(\d+)", body, re.M)]

        rows = [
            ("Blank the screen after", 0, 60, 3600, 60),
            ("Lock the screen after", 1, 60, 7200, 60),
        ]

        for title, index, low, high, step in rows:
            row = Gtk.ListBoxRow()
            row.set_activatable(False)
            line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)

            labels = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1, hexpand=True)
            label = Gtk.Label(label=title, xalign=0)
            label.add_css_class("row-title")
            labels.append(label)
            sub = Gtk.Label(label="Minutes. Deferred while a coding agent is working.", xalign=0)
            sub.add_css_class("row-subtitle")
            labels.append(sub)
            line.append(labels)

            scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, low / 60, high / 60, step / 60)
            scale.set_size_request(240, -1)
            scale.set_draw_value(True)
            scale.set_value_pos(Gtk.PositionType.RIGHT)
            scale.set_digits(0)
            scale.set_valign(Gtk.Align.CENTER)
            if index < len(timeouts):
                scale.set_value(timeouts[index] / 60)
            scale.connect("value-changed", self._idle_changed, index)
            line.append(scale)

            row.set_child(line)
            box.append(row)

        return box

    def _idle_changed(self, scale, index):
        if self._pending is not None:
            GLib.source_remove(self._pending)
        self._pending = GLib.timeout_add(600, self._write_idle, index, int(scale.get_value()) * 60)

    def _write_idle(self, index, seconds):
        self._pending = None
        conf = Path.home() / ".config" / "hypr" / "hypridle.conf"
        try:
            body = conf.read_text()
        except OSError as exc:
            self._flash(f"Could not read hypridle.conf: {exc}")
            return GLib.SOURCE_REMOVE

        # Replace only the nth `timeout =` line, leaving comments, condition_cmd
        # and everything else untouched.
        seen = 0

        def swap(match):
            nonlocal seen
            current = seen
            seen += 1
            return f"{match.group(1)}{seconds}" if current == index else match.group(0)

        updated = re.sub(r"(^\s*timeout\s*=\s*)\d+", swap, body, flags=re.M)
        if updated == body:
            return GLib.SOURCE_REMOVE

        try:
            conf.write_text(updated)
        except OSError as exc:
            self._flash(f"Could not write hypridle.conf: {exc}")
            return GLib.SOURCE_REMOVE

        subprocess.run(["pkill", "-x", "hypridle"], capture_output=True)
        subprocess.Popen(["hypridle"], start_new_session=True,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self._flash(f"Idle timeout set to {seconds // 60} min — hypridle restarted")
        return GLib.SOURCE_REMOVE

    def _keymap_card(self):
        """Switching profile rewrites ~/.config/hypr/keymap and reloads, because
        binds are registered at config load — there is no live-apply for this."""
        box = Gtk.ListBox()
        box.set_selection_mode(Gtk.SelectionMode.NONE)
        box.add_css_class("card")

        row = Gtk.ListBoxRow()
        row.set_activatable(False)
        line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)

        labels = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1, hexpand=True)
        title = Gtk.Label(label="Shortcut profile", xalign=0)
        title.add_css_class("row-title")
        labels.append(title)
        sub = Gtk.Label(
            label="macOS: ⌘Space, ⌃⌘F, ⌘⇧4   ·   Windows: Win+arrows snap, Alt+Tab, Alt+F4",
            xalign=0)
        sub.add_css_class("row-subtitle")
        sub.set_wrap(True)
        labels.append(sub)
        line.append(labels)

        keymap_file = Path.home() / ".config" / "hypr" / "keymap"
        try:
            current = keymap_file.read_text().strip() or "mac"
        except OSError:
            current = "mac"

        options = ["mac", "windows"]
        dropdown = Gtk.DropDown.new_from_strings(["macOS", "Windows"])
        dropdown.set_valign(Gtk.Align.CENTER)
        dropdown.set_selected(options.index(current) if current in options else 0)

        def switched(widget, _param):
            choice = options[widget.get_selected()]
            script = Path.home() / ".config" / "hypr" / "scripts" / "keymap.sh"
            try:
                subprocess.run([str(script), choice], capture_output=True, timeout=10)
                self._flash(f"Switched to the {choice} profile — Hyprland reloaded")
            except (OSError, subprocess.SubprocessError) as exc:
                self._flash(f"Could not switch: {exc}")

        dropdown.connect("notify::selected", switched)
        line.append(dropdown)

        row.set_child(line)
        box.append(row)
        return box

    # ---- shortcuts -------------------------------------------------------
    #
    # Binds are registered while the config loads, so there is no such thing as
    # rebinding a live one. Overrides go to ~/.config/hypr/keybinds.conf as
    # `description = keys`, and binds-shared.lua applies them as each bind
    # registers. A bind's description is its identity — already unique, already
    # what this list and the ⌘? cheatsheet show.

    def _read_rebinds(self):
        rebinds = {}
        try:
            for line in REBINDS.read_text().splitlines():
                if line.strip().startswith("#") or "=" not in line:
                    continue
                desc, keys = line.split("=", 1)
                rebinds[desc.strip()] = keys.strip()
        except OSError:
            pass
        return rebinds

    def _write_rebinds(self, rebinds):
        lines = [
            "# Rebound shortcuts, written by the settings window.",
            "# One `description = keys` per line; the description identifies the bind.",
            "# Delete a line to restore that shortcut's default.",
            "",
        ]
        lines += [f"{desc} = {keys}" for desc, keys in sorted(rebinds.items())]
        try:
            REBINDS.parent.mkdir(parents=True, exist_ok=True)
            REBINDS.write_text("\n".join(lines) + "\n")
        except OSError as exc:
            self._flash(f"Could not write {REBINDS.name}: {exc}")
            return False
        hyprctl(["reload"])
        return True

    @staticmethod
    def _live_binds():
        """Description → current keys, straight from the running compositor."""
        raw = hyprctl(["binds", "-j"])
        if not raw:
            return []
        try:
            binds = json.loads(raw)
        except json.JSONDecodeError:
            return []

        names = [(64, "SUPER"), (8, "ALT"), (4, "CTRL"), (1, "SHIFT")]
        out, seen = [], set()
        for b in binds:
            desc = (b.get("description") or "").strip()
            if not desc or desc in seen:
                continue
            seen.add(desc)
            mods = [n for bit, n in names if int(b.get("modmask", 0)) & bit]
            key = b.get("key") or f"code:{b.get('keycode')}"
            out.append((desc, " + ".join(mods + [key])))
        return sorted(out)

    def _shortcuts_card(self):
        box = Gtk.ListBox()
        box.set_selection_mode(Gtk.SelectionMode.NONE)
        box.add_css_class("card")

        binds = self._live_binds()
        rebinds = self._read_rebinds()

        if not binds:
            row = Gtk.ListBoxRow()
            row.set_activatable(False)
            label = Gtk.Label(
                label="Shortcuts are read from the running compositor — "
                      "start Hyprland to edit them.",
                xalign=0, wrap=True)
            label.add_css_class("row-subtitle")
            row.set_child(label)
            box.append(row)
            return box

        header = Gtk.ListBoxRow()
        header.set_activatable(False)
        head_line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        note = Gtk.Label(label=f"{len(binds)} shortcuts · click one to rebind", xalign=0, hexpand=True)
        note.add_css_class("row-subtitle")
        head_line.append(note)
        reset_all = Gtk.Button(label="Reset all")
        reset_all.add_css_class("link-tool")
        reset_all.connect("clicked", self._reset_all_rebinds)
        head_line.append(reset_all)
        header.set_child(head_line)
        box.append(header)

        for desc, keys in binds:
            row = Gtk.ListBoxRow()
            row.set_activatable(False)
            line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=12)

            label = Gtk.Label(label=desc, xalign=0, hexpand=True)
            label.add_css_class("row-title")
            line.append(label)

            if desc in rebinds:
                changed = Gtk.Label(label="changed")
                changed.add_css_class("row-subtitle")
                line.append(changed)

            button = Gtk.Button(label=keys)
            button.add_css_class("link-tool")
            button.connect("clicked", self._capture_shortcut, desc)
            line.append(button)

            row.set_child(line)
            box.append(row)

        return box

    def _capture_shortcut(self, button, desc):
        dialog = Gtk.Window(transient_for=self, modal=True, title="Rebind")
        dialog.set_default_size(420, 170)

        body = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        body.set_margin_top(24); body.set_margin_bottom(20)
        body.set_margin_start(24); body.set_margin_end(24)

        what = Gtk.Label(label=desc)
        what.add_css_class("group-title")
        body.append(what)

        prompt = Gtk.Label(label="Press the new shortcut…")
        prompt.add_css_class("row-title")
        body.append(prompt)

        hint = Gtk.Label(label="Esc cancels · Backspace restores the default")
        hint.add_css_class("row-subtitle")
        body.append(hint)

        def on_key(_controller, keyval, _keycode, state):
            name = Gdk.keyval_name(keyval)
            if name in (None, "Escape"):
                dialog.close()
                return True
            if name == "BackSpace":
                rebinds = self._read_rebinds()
                if rebinds.pop(desc, None) is not None and self._write_rebinds(rebinds):
                    self._flash(f"“{desc}” restored to its default")
                dialog.close()
                return True

            # Ignore a modifier pressed on its own — wait for the real key.
            if name in ("Super_L", "Super_R", "Control_L", "Control_R",
                        "Alt_L", "Alt_R", "Shift_L", "Shift_R", "Meta_L", "Meta_R"):
                return True

            mods = []
            if state & Gdk.ModifierType.SUPER_MASK:   mods.append("SUPER")
            if state & Gdk.ModifierType.ALT_MASK:     mods.append("ALT")
            if state & Gdk.ModifierType.CONTROL_MASK: mods.append("CTRL")
            if state & Gdk.ModifierType.SHIFT_MASK:   mods.append("SHIFT")

            combo = " + ".join(mods + [name])
            rebinds = self._read_rebinds()
            rebinds[desc] = combo
            if self._write_rebinds(rebinds):
                self._flash(f"“{desc}” is now {combo} — reloaded")
            dialog.close()
            return True

        controller = Gtk.EventControllerKey()
        controller.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)
        controller.connect("key-pressed", on_key)
        dialog.add_controller(controller)

        dialog.set_child(body)
        dialog.present()

    def _reset_all_rebinds(self, _button):
        if self._write_rebinds({}):
            self._flash("All shortcuts restored to their profile defaults")

    def _night_light_card(self):
        box = Gtk.ListBox()
        box.set_selection_mode(Gtk.SelectionMode.NONE)
        box.add_css_class("card")

        row = Gtk.ListBoxRow()
        row.set_activatable(False)
        line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)

        labels = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1, hexpand=True)
        title = Gtk.Label(label="Colour temperature", xalign=0)
        title.add_css_class("row-title")
        labels.append(title)
        sub = Gtk.Label(label="Overrides hyprsunset until the next profile change", xalign=0)
        sub.add_css_class("row-subtitle")
        labels.append(sub)
        line.append(labels)

        scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 2500, 6500, 100)
        scale.set_size_request(240, -1)
        scale.set_draw_value(True)
        scale.set_value_pos(Gtk.PositionType.RIGHT)
        scale.set_digits(0)
        scale.set_value(6500)
        scale.set_valign(Gtk.Align.CENTER)
        scale.connect("value-changed", self.on_temperature)
        line.append(scale)

        reset = Gtk.Button(label="Reset")
        reset.add_css_class("link-tool")
        reset.set_valign(Gtk.Align.CENTER)
        reset.connect("clicked", lambda _b: (scale.set_value(6500), hyprctl(["hyprsunset", "identity"])))
        line.append(reset)

        row.set_child(line)
        box.append(row)
        return box

    def _tools_card(self):
        box = Gtk.ListBox()
        box.set_selection_mode(Gtk.SelectionMode.NONE)
        box.add_css_class("card")

        row = Gtk.ListBoxRow()
        row.set_activatable(False)
        flow = Gtk.FlowBox()
        flow.set_selection_mode(Gtk.SelectionMode.NONE)
        flow.set_max_children_per_line(3)
        flow.set_row_spacing(8)
        flow.set_column_spacing(8)

        for label, command in EXTERNAL_TOOLS:
            if shutil.which(command.split()[0]) is None:
                continue
            button = Gtk.Button(label=label)
            button.add_css_class("link-tool")
            button.connect("clicked", lambda _b, c=command: self._spawn(c))
            flow.append(button)

        row.set_child(flow)
        box.append(row)
        return box

    # -- behaviour ---------------------------------------------------------

    def _spawn(self, command):
        try:
            subprocess.Popen(["sh", "-c", command], start_new_session=True)
        except OSError as exc:
            self._flash(f"Could not start {command}: {exc}")

    def on_temperature(self, scale):
        hyprctl(["hyprsunset", "temperature", str(int(scale.get_value()))])

    def _changed(self, spec, value):
        """Apply live, debounced — sliders emit a lot of events while dragging."""
        if self._pending is not None:
            GLib.source_remove(self._pending)
        self._pending = GLib.timeout_add(80, self._apply, spec, value)

    def _apply(self, spec, value):
        self._pending = None
        assignment = lua_assignment(spec["lua"], value)
        if hyprctl(["eval", "hl.config({ " + assignment + " })"]) is None:
            self._flash(f"Could not apply {spec['title'].lower()}")
        else:
            self._flash("Applied — not saved yet")
        return GLib.SOURCE_REMOVE

    def _flash(self, text):
        self.status.set_text(text)
        GLib.timeout_add_seconds(4, lambda: (self.status.set_text(""), GLib.SOURCE_REMOVE)[1])

    def _current_values(self):
        """Read every control's widget state, grouped into a nested dict."""
        tree = {}
        for spec, widget in self.controls.values():
            if isinstance(widget, Gtk.Switch):
                value = widget.get_active()
            elif isinstance(widget, Gtk.DropDown):
                value = (spec["options"][widget.get_selected()]
                         if spec["kind"] == "text-choice" else widget.get_selected())
            else:
                value = widget.get_value()
                if spec["kind"] == "scale-int":
                    value = int(value)
                else:
                    value = round(value, 2)
            node = tree
            for part in spec["lua"][:-1]:
                node = node.setdefault(part, {})
            node[spec["lua"][-1]] = value
        return tree

    def _render_lua(self, tree, indent=1):
        pad = "    " * indent
        lines = []
        for key, value in tree.items():
            if isinstance(value, dict):
                lines.append(f"{pad}{key} = {{")
                lines.append(self._render_lua(value, indent + 1))
                lines.append(f"{pad}}},")
            else:
                lines.append(f"{pad}{key} = {lua_literal(value)},")
        return "\n".join(lines)

    def on_save(self, _button):
        body = self._render_lua(self._current_values())
        content = (
            f"{MARKER}\n"
            "-- Written by the settings window. hyprland.lua requires this file last,\n"
            "-- so anything here wins over the tracked config. Hand-edit freely; the\n"
            "-- settings window will overwrite this block wholesale on the next Save.\n\n"
            "hl.config({\n"
            f"{body}\n"
            "})\n"
        )
        try:
            LOCAL_LUA.parent.mkdir(parents=True, exist_ok=True)
            if LOCAL_LUA.exists() and MARKER not in LOCAL_LUA.read_text():
                # Someone wrote their own local.lua — never silently clobber it.
                backup = LOCAL_LUA.with_suffix(".lua.bak")
                shutil.copy2(LOCAL_LUA, backup)
                self._flash(f"Existing local.lua backed up to {backup.name}")
            LOCAL_LUA.write_text(content)
            self.status.set_text(f"Saved to {LOCAL_LUA.name}")
        except OSError as exc:
            self._flash(f"Could not write {LOCAL_LUA}: {exc}")

    def on_revert(self, _button):
        hyprctl(["reload"])
        for spec, widget in self.controls.values():
            current = get_option(spec["key"])
            if current is None:
                continue
            if isinstance(widget, Gtk.Switch):
                widget.set_active(bool(current))
            elif isinstance(widget, Gtk.DropDown):
                if spec["kind"] == "text-choice":
                    if isinstance(current, str) and current in spec["options"]:
                        widget.set_selected(spec["options"].index(current))
                else:
                    widget.set_selected(int(current))
            else:
                widget.set_value(float(current))
        self._flash("Reloaded from config files")


class App(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="dev.parlett.hyprsettings")

    def do_activate(self):
        provider = Gtk.CssProvider()
        provider.load_from_data(CSS)
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(), provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )
        SettingsWindow(self).present()


if __name__ == "__main__":
    os.environ.setdefault("GDK_BACKEND", "wayland")
    raise SystemExit(App().run(None))
