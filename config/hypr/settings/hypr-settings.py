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
import shutil
import subprocess
from pathlib import Path

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk, Gdk  # noqa: E402

LOCAL_LUA = Path.home() / ".config" / "hypr" / "local.lua"
MARKER = "-- managed by hypr-settings"

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

# Tools that own settings this window has no business duplicating.
EXTERNAL_TOOLS = [
    ("GTK theme", "nwg-look"),
    ("Qt theme", "qt6ct"),
    ("Audio", "pavucontrol"),
    ("Bluetooth", "blueman-manager"),
    ("Network", "nm-connection-editor"),
    ("Notifications", "swaync-client -t -sw"),
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
                value = widget.get_selected()
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
