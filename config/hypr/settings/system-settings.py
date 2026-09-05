#!/usr/bin/env python3
"""
System Settings — a GTK4 control centre for a Hyprland desktop.

Hyprland is a compositor, not a desktop environment: there is no settings
daemon and no shared config framework for one window to drive. This is the
pragmatic substitute. Everything Hyprland itself owns is edited natively and
applies live through `hyprctl eval 'hl.config{...}'`. Everything it does not
own — audio graphs, Bluetooth pairing, printers — is summarised here with the
bits worth changing exposed directly, and the dedicated tool one click away.

On embedding: you cannot host another Wayland client inside this window.
Wayland has no XEmbed and GTK4 dropped GtkSocket, both deliberately — a client
cannot reparent another client's surface. So "Open pavucontrol" really does
mean a separate window, and no amount of shell around it changes that.

Hyprland options are live-applied but not saved until you press Save, which
writes ~/.config/hypr/local.lua. Everything else here — shortcuts, defaults,
timezone — is applied and persisted immediately, because those are files and
services rather than compositor state.
"""

import json
import os
import pwd
import re
import shutil
import subprocess
from datetime import datetime
from pathlib import Path

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import GLib, Gtk, Gdk  # noqa: E402

HYPR = Path.home() / ".config" / "hypr"
LOCAL_LUA = HYPR / "local.lua"
REBINDS = HYPR / "keybinds.conf"
IDLE_CONF = HYPR / "hypridle.conf"
KEYMAP_FILE = HYPR / "keymap"
MARKER = "-- managed by system-settings"

CSS = b"""
window { background-color: #1e1e2e; color: #cdd6f4;
         font-family: "JetBrainsMono Nerd Font", "Noto Sans", sans-serif; }
headerbar { background-color: #181825; color: #cdd6f4;
            border-bottom: 1px solid #313244; min-height: 44px; }
headerbar button { background: #313244; color: #cdd6f4; border: none; border-radius: 9px; }
headerbar button:hover { background: #45475a; }
headerbar button.suggested-action { background: #cba6f7; color: #1e1e2e; font-weight: bold; }

stacksidebar { background-color: #181825; border-right: 1px solid #313244; }
stacksidebar list { background: transparent; padding: 8px 6px; }
stacksidebar list row { border-radius: 10px; padding: 9px 12px; margin: 1px 4px; color: #a6adc8; }
stacksidebar list row:hover { background: #313244; color: #cdd6f4; }
stacksidebar list row:selected { background: #cba6f7; color: #1e1e2e; font-weight: bold; }

.page-title { font-size: 19px; font-weight: bold; color: #cdd6f4; margin: 4px 2px 2px 2px; }
.page-subtitle { color: #6c7086; font-size: 12px; margin: 0 2px 10px 2px; }
.group-title { font-size: 14px; font-weight: bold; color: #cba6f7; margin: 16px 4px 6px 4px; }
.card { background-color: #181825; border: 1px solid #313244;
        border-radius: 14px; padding: 4px 6px; }
row { padding: 10px 12px; border-radius: 10px; background: transparent; }
.row-title { color: #cdd6f4; }
.row-subtitle { color: #6c7086; font-size: 11px; }
.hint { color: #6c7086; font-size: 11px; margin: 8px 6px; }
.warn { color: #f9e2af; font-size: 11px; }
.mono { font-family: "JetBrainsMono Nerd Font", monospace; color: #a6adc8; font-size: 12px; }

scale trough { background-color: #313244; border-radius: 999px; min-height: 5px; }
scale highlight { background-color: #cba6f7; border-radius: 999px; }
scale slider { background-color: #cdd6f4; border-radius: 999px; min-width: 15px; min-height: 15px; }
scale value { color: #a6adc8; font-size: 11px; }
switch { background-color: #313244; border-radius: 999px; }
switch:checked { background-color: #cba6f7; }
switch slider { background-color: #cdd6f4; border-radius: 999px; }
dropdown > button { background: #313244; color: #cdd6f4; border: none; border-radius: 9px; }
popover contents { background: #181825; border: 1px solid #313244;
                   border-radius: 12px; color: #cdd6f4; }
button.link-tool { background: #313244; color: #cdd6f4; border: none;
                   border-radius: 10px; padding: 9px 14px; }
button.link-tool:hover { background: #45475a; color: #cba6f7; }
button.danger:hover { background: #f38ba8; color: #1e1e2e; }
.saved { color: #a6e3a1; font-size: 11px; }
"""

# --------------------------------------------------------------------------
# Hyprland options, live-applied. `key` is the getoption path, `lua` is where
# it sits in hl.config, `kind` picks the widget.
# --------------------------------------------------------------------------

APPEARANCE = [
    dict(key="general:gaps_in", lua=("general", "gaps_in"), kind="scale-int",
         title="Inner gaps", subtitle="Between windows", min=0, max=30, step=1),
    dict(key="general:gaps_out", lua=("general", "gaps_out"), kind="scale-int",
         title="Outer gaps", subtitle="Between windows and the screen edge", min=0, max=60, step=1),
    dict(key="general:border_size", lua=("general", "border_size"), kind="scale-int",
         title="Border width", subtitle=None, min=0, max=8, step=1),
    dict(key="decoration:rounding", lua=("decoration", "rounding"), kind="scale-int",
         title="Corner rounding", subtitle=None, min=0, max=24, step=1),
    dict(key="decoration:inactive_opacity", lua=("decoration", "inactive_opacity"),
         kind="scale-float", title="Inactive window opacity",
         subtitle="1.0 is fully opaque", min=0.5, max=1.0, step=0.01),
]

EFFECTS = [
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
]

MOUSE = [
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
    dict(key="input:follow_mouse", lua=("input", "follow_mouse"), kind="choice",
         title="Focus follows mouse", subtitle=None,
         options=["Off", "Always", "Loose", "Click to focus"]),
]

KEYBOARD = [
    dict(key="input:repeat_delay", lua=("input", "repeat_delay"), kind="scale-int",
         title="Repeat delay", subtitle="Milliseconds before a held key repeats",
         min=150, max=1000, step=25),
    dict(key="input:repeat_rate", lua=("input", "repeat_rate"), kind="scale-int",
         title="Repeat rate", subtitle="Repeats per second once it starts",
         min=10, max=80, step=1),
    dict(key="input:numlock_by_default", lua=("input", "numlock_by_default"), kind="switch",
         title="Num Lock on at login", subtitle=None),
    dict(key="input:kb_layout", lua=("input", "kb_layout"), kind="text-choice",
         title="Layout", subtitle="Restart applications to pick up a change",
         options=["gb", "us", "de", "fr", "es", "it", "no", "se", "dk"]),
]

BEHAVIOUR = [
    dict(key="misc:vrr", lua=("misc", "vrr"), kind="choice",
         title="Adaptive sync", subtitle="Fullscreen-only avoids desktop flicker on this panel",
         options=["Off", "Always on", "Fullscreen only", "Fullscreen games"]),
    dict(key="general:allow_tearing", lua=("general", "allow_tearing"), kind="switch",
         title="Allow tearing", subtitle="Games opt in individually; this is the master switch"),
    dict(key="misc:middle_click_paste", lua=("misc", "middle_click_paste"), kind="switch",
         title="Middle-click paste", subtitle="Primary selection"),
    dict(key="cursor:inactive_timeout", lua=("cursor", "inactive_timeout"), kind="scale-int",
         title="Hide cursor after", subtitle="Seconds of stillness; 0 never hides",
         min=0, max=20, step=1),
]

# Default-application categories: label, the mime types to set together, and a
# hint for which apps are sensible. Setting several types at once is what
# people actually mean by "open images with".
DEFAULT_APP_CATEGORIES = [
    ("Web browser", ["text/html", "x-scheme-handler/http", "x-scheme-handler/https"]),
    ("Email", ["x-scheme-handler/mailto"]),
    ("File manager", ["inode/directory"]),
    ("Text editor", ["text/plain"]),
    ("Images", ["image/png", "image/jpeg", "image/gif", "image/webp"]),
    ("Video", ["video/mp4", "video/x-matroska", "video/webm"]),
    ("Audio", ["audio/mpeg", "audio/flac", "audio/x-wav"]),
    ("PDF", ["application/pdf"]),
]


def run(args, timeout=5):
    """Run a command, returning stdout or None. Never raises at the caller."""
    try:
        out = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
        return out.stdout if out.returncode == 0 else None
    except (OSError, subprocess.SubprocessError):
        return None


def hyprctl(args):
    return run(["hyprctl", *args])


def get_option(path):
    raw = hyprctl(["getoption", path, "-j"])
    if not raw:
        return None
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        return None
    for field in ("int", "float", "str", "custom"):
        if data.get(field) is not None:
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


def desktop_entries():
    """Installed applications: desktop file id -> (name, set of mime types)."""
    entries = {}
    roots = [Path("/usr/share/applications"),
             Path("/usr/local/share/applications"),
             Path.home() / ".local/share/applications"]
    for root in roots:
        if not root.is_dir():
            continue
        for path in root.glob("*.desktop"):
            name, mimes, no_display = None, set(), False
            try:
                for line in path.read_text(errors="ignore").splitlines():
                    if line.startswith("Name=") and name is None:
                        name = line[5:].strip()
                    elif line.startswith("MimeType="):
                        mimes = {m for m in line[9:].strip().split(";") if m}
                    elif line.startswith(("NoDisplay=true", "Hidden=true")):
                        no_display = True
            except OSError:
                continue
            if name and not no_display:
                entries[path.name] = (name, mimes)
    return entries


class SystemSettings(Gtk.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app, title="System Settings")
        self.set_default_size(1000, 780)

        self.controls = {}
        self._pending = None
        self._apps = desktop_entries()

        header = Gtk.HeaderBar()
        header.set_show_title_buttons(True)
        self.set_titlebar(header)

        self.status = Gtk.Label(label="")
        self.status.add_css_class("saved")
        header.pack_start(self.status)

        revert = Gtk.Button(label="Reload")
        revert.connect("clicked", self.on_revert)
        header.pack_end(revert)

        save = Gtk.Button(label="Save")
        save.add_css_class("suggested-action")
        save.connect("clicked", self.on_save)
        header.pack_end(save)

        self.stack = Gtk.Stack()
        self.stack.set_transition_type(Gtk.StackTransitionType.CROSSFADE)
        self.stack.set_transition_duration(120)

        sidebar = Gtk.StackSidebar()
        sidebar.set_stack(self.stack)
        sidebar.set_size_request(190, -1)

        split = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        split.append(sidebar)
        split.append(self.stack)
        self.set_child(split)

        live = hyprctl(["version"]) is not None

        self._add_page("appearance", "Appearance", self._page_appearance(live))
        self._add_page("keyboard", "Keyboard", self._page_keyboard(live))
        self._add_page("mouse", "Mouse", self._page_mouse(live))
        self._add_page("display", "Display", self._page_display())
        self._add_page("sound", "Sound", self._page_sound())
        self._add_page("network", "Network", self._page_network())
        self._add_page("bluetooth", "Bluetooth", self._page_bluetooth())
        self._add_page("power", "Power & Lock", self._page_power())
        self._add_page("defaults", "Default Apps", self._page_defaults())
        self._add_page("datetime", "Date & Time", self._page_datetime())
        self._add_page("users", "Users", self._page_users())
        self._add_page("system", "System", self._page_system(live))

    # ---- scaffolding -----------------------------------------------------

    def _add_page(self, name, title, content):
        scroller = Gtk.ScrolledWindow(hexpand=True, vexpand=True)
        scroller.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        content.set_margin_top(16)
        content.set_margin_bottom(24)
        content.set_margin_start(22)
        content.set_margin_end(22)
        scroller.set_child(content)
        self.stack.add_titled(scroller, name, title)

    @staticmethod
    def _page(title, subtitle=None):
        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=0)
        head = Gtk.Label(label=title, xalign=0)
        head.add_css_class("page-title")
        box.append(head)
        if subtitle:
            sub = Gtk.Label(label=subtitle, xalign=0, wrap=True)
            sub.add_css_class("page-subtitle")
            box.append(sub)
        return box

    @staticmethod
    def _group(text):
        label = Gtk.Label(label=text, xalign=0)
        label.add_css_class("group-title")
        return label

    @staticmethod
    def _card():
        box = Gtk.ListBox()
        box.set_selection_mode(Gtk.SelectionMode.NONE)
        box.add_css_class("card")
        return box

    @staticmethod
    def _row(title, subtitle, widget):
        row = Gtk.ListBoxRow()
        row.set_activatable(False)
        line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=14)
        labels = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=1, hexpand=True)

        label = Gtk.Label(label=title, xalign=0)
        label.add_css_class("row-title")
        labels.append(label)
        if subtitle:
            sub = Gtk.Label(label=subtitle, xalign=0, wrap=True)
            sub.add_css_class("row-subtitle")
            labels.append(sub)

        line.append(labels)
        if widget is not None:
            line.append(widget)
        row.set_child(line)
        return row

    def _info_row(self, title, value):
        label = Gtk.Label(label=value or "—")
        label.add_css_class("mono")
        label.set_selectable(True)
        return self._row(title, None, label)

    def _tool_row(self, title, subtitle, label, command):
        if shutil.which(command.split()[0]) is None:
            return self._row(title, f"{subtitle} — {command.split()[0]} is not installed", None)
        button = Gtk.Button(label=label)
        button.add_css_class("link-tool")
        button.connect("clicked", lambda _b: self._spawn(command))
        return self._row(title, subtitle, button)

    def _spawn(self, command):
        try:
            subprocess.Popen(["sh", "-c", command], start_new_session=True,
                             stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        except OSError as exc:
            self._flash(f"Could not start {command}: {exc}")

    def _flash(self, text):
        self.status.set_text(text)
        GLib.timeout_add_seconds(5, lambda: (self.status.set_text(""), GLib.SOURCE_REMOVE)[1])

    # ---- live Hyprland options -------------------------------------------

    def _settings_card(self, specs, live):
        card = self._card()
        for spec in specs:
            card.append(self._row(spec["title"], spec["subtitle"], self._control(spec, live)))
        return card

    def _control(self, spec, live):
        current = get_option(spec["key"]) if live else None
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
            widget = Gtk.DropDown.new_from_strings(spec["options"])
            widget.set_valign(Gtk.Align.CENTER)
            if isinstance(current, str) and current in spec["options"]:
                widget.set_selected(spec["options"].index(current))
            widget.connect("notify::selected",
                           lambda w, _p: self._changed(spec, spec["options"][w.get_selected()]))

        else:
            is_float = kind == "scale-float"
            widget = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL,
                                              spec["min"], spec["max"], spec["step"])
            widget.set_size_request(260, -1)
            widget.set_draw_value(True)
            widget.set_value_pos(Gtk.PositionType.RIGHT)
            widget.set_digits(2 if is_float else 0)
            widget.set_valign(Gtk.Align.CENTER)
            if isinstance(current, (int, float)):
                widget.set_value(float(current))
            widget.connect("value-changed",
                           lambda w: self._changed(spec, w.get_value() if is_float else int(w.get_value())))

        widget.set_sensitive(live)
        self.controls[id(widget)] = (spec, widget)
        return widget

    def _changed(self, spec, value):
        if self._pending is not None:
            GLib.source_remove(self._pending)
        self._pending = GLib.timeout_add(80, self._apply, spec, value)

    def _apply(self, spec, value):
        self._pending = None
        body = lua_literal(value)
        for part in reversed(spec["lua"][1:]):
            body = "{ " + f"{part} = {body}" + " }"
        assignment = f"{spec['lua'][0]} = {body}"
        if hyprctl(["eval", "hl.config({ " + assignment + " })"]) is None:
            self._flash(f"Could not apply {spec['title'].lower()}")
        else:
            self._flash("Applied — not saved yet")
        return GLib.SOURCE_REMOVE

    # ---- pages -----------------------------------------------------------

    def _page_appearance(self, live):
        page = self._page("Appearance", "Applies as you drag. Save writes local.lua.")
        page.append(self._group("Layout"))
        page.append(self._settings_card(APPEARANCE, live))
        page.append(self._group("Effects"))
        page.append(self._settings_card(EFFECTS, live))
        page.append(self._group("Theme"))
        card = self._card()
        card.append(self._tool_row("GTK applications", "Theme, icons and fonts", "Open nwg-look", "nwg-look"))
        card.append(self._tool_row("Qt applications", "Non-KDE Qt theming", "Open qt6ct", "qt6ct"))
        page.append(card)
        return page

    def _page_keyboard(self, live):
        page = self._page("Keyboard", "Layout, repeat and every shortcut on the system.")

        page.append(self._group("Shortcut profile"))
        page.append(self._keymap_card())

        page.append(self._group("Typing"))
        page.append(self._settings_card(KEYBOARD, live))

        page.append(self._group("Shortcuts"))
        page.append(self._shortcuts_card())
        return page

    def _page_mouse(self, live):
        page = self._page("Mouse", "Pointer behaviour, applied live.")
        page.append(self._settings_card(MOUSE, live))
        return page

    def _page_display(self):
        page = self._page("Display", "Resolution and arrangement.")
        card = self._card()
        raw = hyprctl(["monitors", "-j"])
        try:
            monitors = json.loads(raw) if raw else []
        except json.JSONDecodeError:
            monitors = []

        if monitors:
            for m in monitors:
                card.append(self._info_row(
                    f"{m.get('name')} — {m.get('description', '')[:40]}",
                    f"{m.get('width')}x{m.get('height')} @ {float(m.get('refreshRate', 0)):.0f}Hz"
                    f"  scale {m.get('scale')}  vrr {m.get('vrr')}"))
        else:
            card.append(self._row("No monitor data", "Hyprland is not running", None))
        page.append(card)

        page.append(self._group("Arrangement"))
        tools = self._card()
        # nwg-displays defaults to writing ~/.config/hypr/monitors.lua, which is
        # a tracked file in this repo — redirect it so a save cannot clobber it.
        tools.append(self._tool_row(
            "Layout editor",
            "Writes monitors-generated.lua, which monitors.lua picks up",
            "Open nwg-displays",
            "nwg-displays -m ~/.config/hypr/monitors-generated.lua"))
        tools.append(self._row("Permanent changes",
                               "Edit ~/.config/hypr/monitors.lua for anything the editor cannot express",
                               None))
        page.append(tools)
        return page

    def _page_sound(self):
        page = self._page("Sound", "Output volume and device. PipeWire owns the rest.")
        card = self._card()

        raw = run(["wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"]) or ""
        match = re.search(r"([\d.]+)", raw)
        volume = float(match.group(1)) if match else 0.0
        muted = "MUTED" in raw

        scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 0, 100, 1)
        scale.set_size_request(260, -1)
        scale.set_draw_value(True)
        scale.set_value_pos(Gtk.PositionType.RIGHT)
        scale.set_digits(0)
        scale.set_value(volume * 100)
        scale.set_valign(Gtk.Align.CENTER)
        scale.connect("value-changed",
                      lambda w: run(["wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@",
                                     f"{int(w.get_value())}%"]))
        card.append(self._row("Output volume", None, scale))

        mute = Gtk.Switch(valign=Gtk.Align.CENTER)
        mute.set_active(muted)
        mute.connect("notify::active",
                     lambda w, _p: run(["wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@",
                                        "1" if w.get_active() else "0"]))
        card.append(self._row("Muted", None, mute))

        default_sink = (run(["pactl", "get-default-sink"]) or "").strip()
        card.append(self._info_row("Default output", default_sink))
        page.append(card)

        page.append(self._group("More"))
        tools = self._card()
        tools.append(self._tool_row("Devices and per-app volume", "Streams, profiles, inputs",
                                    "Open pavucontrol", "pavucontrol"))
        tools.append(self._tool_row("Patchbay", "Route PipeWire nodes by hand",
                                    "Open qpwgraph", "qpwgraph"))
        page.append(tools)
        return page

    def _page_network(self):
        page = self._page("Network", "Connection status. NetworkManager owns the details.")
        card = self._card()

        state = (run(["nmcli", "-t", "-f", "STATE,CONNECTIVITY", "general"]) or "").strip()
        card.append(self._info_row("Status", state.replace(":", " · ")))

        active = (run(["nmcli", "-t", "-f", "NAME,TYPE,DEVICE", "connection", "show", "--active"]) or "").strip()
        for line in filter(None, active.splitlines()):
            name, kind, device = (line.split(":") + ["", ""])[:3]
            card.append(self._info_row(f"{name} ({kind})", device))

        wifi_state = (run(["nmcli", "radio", "wifi"]) or "").strip()
        radio = Gtk.Switch(valign=Gtk.Align.CENTER)
        radio.set_active(wifi_state == "enabled")
        radio.connect("notify::active",
                      lambda w, _p: run(["nmcli", "radio", "wifi",
                                         "on" if w.get_active() else "off"]))
        card.append(self._row("Wi-Fi", None, radio))
        page.append(card)

        page.append(self._group("More"))
        tools = self._card()
        tools.append(self._tool_row("Connections", "Add, edit and forget networks",
                                    "Open editor", "nm-connection-editor"))
        page.append(tools)
        return page

    def _page_bluetooth(self):
        page = self._page("Bluetooth", "Adapter power. Pairing lives in Blueman.")
        card = self._card()

        show = run(["bluetoothctl", "show"]) or ""
        powered = "Powered: yes" in show
        name = re.search(r"Name:\s*(.+)", show)

        card.append(self._info_row("Adapter", name.group(1).strip() if name else "none found"))

        switch = Gtk.Switch(valign=Gtk.Align.CENTER)
        switch.set_active(powered)
        switch.connect("notify::active",
                       lambda w, _p: run(["bluetoothctl", "power",
                                          "on" if w.get_active() else "off"]))
        switch.set_sensitive(bool(show))
        card.append(self._row("Powered", None, switch))
        page.append(card)

        page.append(self._group("More"))
        tools = self._card()
        tools.append(self._tool_row("Devices", "Pair, connect and trust",
                                    "Open Blueman", "blueman-manager"))
        page.append(tools)
        return page

    def _page_power(self):
        page = self._page("Power & Lock",
                          "Profile, idle behaviour and locking. Idle actions are deferred "
                          "automatically while a coding agent is working.")

        page.append(self._group("Power profile"))
        profile_card = self._card()

        available = (run(["powerprofilesctl", "list"]) or "")
        # Profile names sit at two spaces of indent, or "* " when active; their
        # properties (CpuDriver, Degraded…) are indented four. Matching loosely
        # picks up the properties as if they were profiles.
        profiles = re.findall(r"^(?:  |\* )([\w-]+):", available, re.M)
        active = (run(["powerprofilesctl", "get"]) or "").strip()

        if profiles:
            dropdown = Gtk.DropDown.new_from_strings(profiles)
            dropdown.set_valign(Gtk.Align.CENTER)
            if active in profiles:
                dropdown.set_selected(profiles.index(active))

            def set_profile(widget, _param):
                choice = profiles[widget.get_selected()]
                if run(["powerprofilesctl", "set", choice]) is None:
                    self._flash(f"Could not switch to {choice}")
                else:
                    self._flash(f"Power profile: {choice}")

            dropdown.connect("notify::selected", set_profile)
            profile_card.append(self._row(
                "Profile",
                "performance keeps clocks up; balanced is the sane default on a desktop",
                dropdown))
        else:
            profile_card.append(self._row(
                "Profile", "power-profiles-daemon is not running", None))
        page.append(profile_card)

        page.append(self._group("Idle"))
        page.append(self._idle_card())

        page.append(self._group("Lock screen"))
        card = self._card()

        lock_now = Gtk.Button(label="Lock now")
        lock_now.add_css_class("link-tool")
        lock_now.connect("clicked", lambda _b: self._spawn("hyprlock"))
        card.append(self._row("Lock the screen", "Same as ⌃⌘Q / Win+L", lock_now))

        before_sleep = "before_sleep_cmd" in (self._idle_text() or "")
        sleep_switch = Gtk.Switch(valign=Gtk.Align.CENTER)
        sleep_switch.set_active(before_sleep)
        sleep_switch.set_sensitive(False)
        card.append(self._row("Lock before sleep",
                              "Set by hypridle.conf; edit that file to change it", sleep_switch))
        card.append(self._row("Appearance",
                              "Clock, blur and the password field live in ~/.config/hypr/hyprlock.conf",
                              None))
        page.append(card)

        page.append(self._group("Session"))
        session = self._card()

        logind = run(["loginctl", "show-session", os.environ.get("XDG_SESSION_ID", "self")]) or ""
        handle = re.search(r"HandlePowerKey=(\S+)", logind)
        session.append(self._info_row("Power button",
                                      handle.group(1) if handle else "handled by logind"))

        buttons = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        for label, command in (("Suspend", "systemctl suspend"),
                               ("Reboot", "hyprshutdown --vt 1 --post-cmd 'systemctl reboot'"),
                               ("Shut down", "hyprshutdown --vt 1 --post-cmd 'systemctl poweroff'")):
            button = Gtk.Button(label=label)
            button.add_css_class("link-tool")
            if label != "Suspend":
                button.add_css_class("danger")
            button.connect("clicked", lambda _b, c=command: self._spawn(c))
            buttons.append(button)
        session.append(self._row("End the session",
                                 "Reboot and shut down close applications gracefully first",
                                 buttons))
        page.append(session)

        page.append(self._group("More"))
        tools = self._card()
        tools.append(self._tool_row("Sensors and fans", "Curves and monitoring",
                                    "Open CoolerControl", "coolercontrol"))
        page.append(tools)
        return page

    def _page_defaults(self):
        page = self._page("Default Apps",
                          "Which application opens what. Written with xdg-mime.")
        card = self._card()

        for label, mimes in DEFAULT_APP_CATEGORIES:
            current = (run(["xdg-mime", "query", "default", mimes[0]]) or "").strip()

            candidates = sorted(
                (desktop_id for desktop_id, (_n, app_mimes) in self._apps.items()
                 if app_mimes & set(mimes)),
                key=lambda i: self._apps[i][0].lower())

            # A default pointing at an app that is not installed is worse than
            # none: the association silently fails. Surface it.
            missing = bool(current) and current not in self._apps
            if missing and current:
                candidates.insert(0, current)

            if not candidates:
                card.append(self._row(label, "No installed application claims this type", None))
                continue

            names = [f"{self._apps[i][0]}" if i in self._apps else f"{i} (not installed)"
                     for i in candidates]
            dropdown = Gtk.DropDown.new_from_strings(names)
            dropdown.set_valign(Gtk.Align.CENTER)
            if current in candidates:
                dropdown.set_selected(candidates.index(current))

            def on_pick(widget, _param, mimes=mimes, candidates=candidates, label=label):
                chosen = candidates[widget.get_selected()]
                failed = [m for m in mimes
                          if run(["xdg-mime", "default", chosen, m]) is None]
                if failed:
                    self._flash(f"Could not set {label}")
                else:
                    self._flash(f"{label} → {self._apps.get(chosen, (chosen,))[0]}")

            dropdown.connect("notify::selected", on_pick)

            subtitle = ", ".join(mimes[:2]) + ("…" if len(mimes) > 2 else "")
            if missing:
                subtitle = f"currently {current}, which is not installed"
            card.append(self._row(label, subtitle, dropdown))

        page.append(card)
        page.append(self._group("Note"))
        note = self._card()
        note.append(self._row(
            "Removing KDE applications",
            "Defaults pointing at Dolphin, Gwenview, Kate or Haruna stop working when those "
            "are removed. Re-pick them here afterwards.", None))
        page.append(note)
        return page

    def _page_datetime(self):
        page = self._page("Date & Time", "Timezone and clock. systemd-timesyncd keeps it accurate.")
        card = self._card()

        status = run(["timedatectl", "show"]) or ""
        fields = dict(line.split("=", 1) for line in status.splitlines() if "=" in line)

        card.append(self._info_row("Local time", datetime.now().strftime("%A %d %B %Y, %H:%M:%S")))
        card.append(self._info_row("Timezone", fields.get("Timezone", "unknown")))

        ntp = Gtk.Switch(valign=Gtk.Align.CENTER)
        ntp.set_active(fields.get("NTP") == "yes")
        ntp.connect("notify::active",
                    lambda w, _p: self._set_ntp(w.get_active()))
        card.append(self._row("Network time", "Synchronise the clock automatically", ntp))
        card.append(self._info_row("Synchronised", fields.get("NTPSynchronized", "?")))
        page.append(card)

        page.append(self._group("Timezone"))
        tz_card = self._card()
        zones = (run(["timedatectl", "list-timezones"]) or "").split()
        common = [z for z in zones if z.startswith(("Europe/", "America/New", "America/Los",
                                                    "Asia/Tok", "Australia/Syd", "UTC"))]
        if common:
            current_tz = fields.get("Timezone", "UTC")
            if current_tz not in common:
                common.insert(0, current_tz)
            dropdown = Gtk.DropDown.new_from_strings(common)
            dropdown.set_valign(Gtk.Align.CENTER)
            dropdown.set_selected(common.index(current_tz) if current_tz in common else 0)
            dropdown.connect("notify::selected",
                             lambda w, _p: self._set_timezone(common[w.get_selected()]))
            tz_card.append(self._row("Set timezone",
                                     "Asks for authentication — systemd requires it", dropdown))
        page.append(tz_card)
        return page

    def _set_ntp(self, enabled):
        if run(["timedatectl", "set-ntp", "true" if enabled else "false"]) is None:
            self._flash("Could not change network time — authentication declined?")
        else:
            self._flash(f"Network time {'enabled' if enabled else 'disabled'}")

    def _set_timezone(self, zone):
        if run(["timedatectl", "set-timezone", zone], timeout=30) is None:
            self._flash("Could not set the timezone — authentication declined?")
        else:
            self._flash(f"Timezone set to {zone}")

    def _page_users(self):
        page = self._page("Users", "Accounts on this machine.")
        card = self._card()

        me = pwd.getpwuid(os.getuid())
        # A uid floor alone is not enough: systemd dynamic users (earlyoom here)
        # get uids far above 1000. A login shell is what actually distinguishes
        # a person from a service account.
        nologin = ("/usr/bin/nologin", "/sbin/nologin", "/usr/sbin/nologin",
                   "/bin/false", "/usr/bin/false", "")
        for entry in sorted(pwd.getpwall(), key=lambda p: p.pw_uid):
            if entry.pw_uid < 1000 or entry.pw_uid >= 65534:
                continue
            if entry.pw_shell in nologin:
                continue
            groups = run(["id", "-Gn", entry.pw_name]) or ""
            label = entry.pw_gecos.split(",")[0] or entry.pw_name
            suffix = "  (you)" if entry.pw_name == me.pw_name else ""
            card.append(self._row(
                f"{label}{suffix}",
                f"{entry.pw_name} · uid {entry.pw_uid} · {entry.pw_dir} · {entry.pw_shell}\n"
                f"groups: {groups.strip()}", None))
        page.append(card)

        page.append(self._group("Actions"))
        actions = self._card()
        actions.append(self._tool_row("Change your password", "Opens passwd in a terminal",
                                      "Change password", "alacritty -e passwd"))
        actions.append(self._row(
            "Adding or removing accounts",
            "No GUI ships for this on Hyprland. Use useradd, userdel and gpasswd from a "
            "terminal — they need root either way.", None))
        page.append(actions)
        return page

    def _page_system(self, live):
        page = self._page("System", "Everything else.")

        page.append(self._group("Behaviour"))
        page.append(self._settings_card(BEHAVIOUR, live))

        page.append(self._group("Night light"))
        page.append(self._night_light_card())

        page.append(self._group("Machine"))
        card = self._card()
        try:
            kernel = os.uname().release
        except OSError:
            kernel = "?"
        card.append(self._info_row("Kernel", kernel))
        card.append(self._info_row("Compositor",
                                   (hyprctl(["version", "-j"]) or "{}").strip()[:0] or
                                   json.loads(hyprctl(["version", "-j"]) or "{}").get("tag", "not running")))
        card.append(self._info_row("Shortcut profile", self._current_keymap()))
        page.append(card)

        page.append(self._group("Tools"))
        tools = self._card()
        tools.append(self._tool_row("Updates", "Repo and AUR packages", "Open", "alacritty -e cachy-update"))
        tools.append(self._tool_row("Disks", "Partitions and SMART", "Open", "gnome-disk-utility"))
        tools.append(self._tool_row("Printers", "CUPS queues", "Open", "system-config-printer"))
        tools.append(self._tool_row("System info", "Hardware summary", "Open", "alacritty -e fastfetch"))
        page.append(tools)

        hint = Gtk.Label(
            label="Hyprland options above apply live and are saved to local.lua with Save. "
                  "Shortcuts, defaults and the timezone are written immediately.",
            xalign=0, wrap=True)
        hint.add_css_class("hint")
        page.append(hint)
        return page

    # ---- keymap and shortcuts -------------------------------------------

    @staticmethod
    def _current_keymap():
        try:
            return KEYMAP_FILE.read_text().strip() or "mac"
        except OSError:
            return "mac"

    def _keymap_card(self):
        card = self._card()
        options = ["mac", "windows"]
        current = self._current_keymap()

        dropdown = Gtk.DropDown.new_from_strings(["macOS", "Windows"])
        dropdown.set_valign(Gtk.Align.CENTER)
        dropdown.set_selected(options.index(current) if current in options else 0)

        def switched(widget, _param):
            choice = options[widget.get_selected()]
            script = HYPR / "scripts" / "keymap.sh"
            if run([str(script), choice], timeout=15) is None:
                self._flash("Could not switch profile")
            else:
                self._flash(f"Switched to {choice} — reopen this window to see the new shortcuts")

        dropdown.connect("notify::selected", switched)
        card.append(self._row(
            "Profile",
            "macOS: ⌘Space, ⌃⌘F, ⌘⇧4   ·   Windows: Win+arrows snap, Alt+Tab, Alt+F4",
            dropdown))
        return card

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
            "# Rebound shortcuts, written by System Settings.",
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
        card = self._card()
        binds = self._live_binds()
        rebinds = self._read_rebinds()

        if not binds:
            card.append(self._row("Shortcuts unavailable",
                                  "Read from the running compositor — start Hyprland to edit them",
                                  None))
            return card

        reset_all = Gtk.Button(label="Reset all")
        reset_all.add_css_class("link-tool")
        reset_all.add_css_class("danger")
        reset_all.connect("clicked", self._reset_all_rebinds)
        card.append(self._row(f"{len(binds)} shortcuts",
                              "Click one to rebind · Esc cancels · Backspace restores the default",
                              reset_all))

        for desc, keys in binds:
            button = Gtk.Button(label=keys)
            button.add_css_class("link-tool")
            button.connect("clicked", self._capture_shortcut, desc)
            card.append(self._row(desc, "changed" if desc in rebinds else None, button))
        return card

    def _capture_shortcut(self, _button, desc):
        dialog = Gtk.Window(transient_for=self, modal=True, title="Rebind")
        dialog.set_default_size(430, 180)

        body = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        for margin in ("top", "bottom", "start", "end"):
            getattr(body, f"set_margin_{margin}")(24)

        what = Gtk.Label(label=desc)
        what.add_css_class("page-title")
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
            if name in ("Super_L", "Super_R", "Control_L", "Control_R", "Alt_L",
                        "Alt_R", "Shift_L", "Shift_R", "Meta_L", "Meta_R"):
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
                self._flash(f"“{desc}” is now {combo}")
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

    # ---- idle and night light -------------------------------------------

    @staticmethod
    def _idle_text():
        try:
            return IDLE_CONF.read_text()
        except OSError:
            return None

    def _idle_card(self):
        card = self._card()
        body = self._idle_text() or ""
        timeouts = [int(t) for t in re.findall(r"^\s*timeout\s*=\s*(\d+)", body, re.M)]

        for title, index in (("Blank the screen after", 0), ("Lock the screen after", 1)):
            scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 1, 120, 1)
            scale.set_size_request(260, -1)
            scale.set_draw_value(True)
            scale.set_value_pos(Gtk.PositionType.RIGHT)
            scale.set_digits(0)
            scale.set_valign(Gtk.Align.CENTER)
            if index < len(timeouts):
                scale.set_value(timeouts[index] / 60)
            scale.set_sensitive(bool(body))
            scale.connect("value-changed", self._idle_changed, index)
            card.append(self._row(title, "Minutes", scale))
        return card

    def _idle_changed(self, scale, index):
        if self._pending is not None:
            GLib.source_remove(self._pending)
        self._pending = GLib.timeout_add(600, self._write_idle, index, int(scale.get_value()) * 60)

    def _write_idle(self, index, seconds):
        self._pending = None
        body = self._idle_text()
        if body is None:
            return GLib.SOURCE_REMOVE

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
            IDLE_CONF.write_text(updated)
        except OSError as exc:
            self._flash(f"Could not write hypridle.conf: {exc}")
            return GLib.SOURCE_REMOVE

        subprocess.run(["pkill", "-x", "hypridle"], capture_output=True)
        subprocess.Popen(["hypridle"], start_new_session=True,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self._flash(f"Idle timeout set to {seconds // 60} min")
        return GLib.SOURCE_REMOVE

    def _night_light_card(self):
        card = self._card()
        scale = Gtk.Scale.new_with_range(Gtk.Orientation.HORIZONTAL, 2500, 6500, 100)
        scale.set_size_request(240, -1)
        scale.set_draw_value(True)
        scale.set_value_pos(Gtk.PositionType.RIGHT)
        scale.set_digits(0)
        scale.set_value(6500)
        scale.set_valign(Gtk.Align.CENTER)
        scale.connect("value-changed",
                      lambda w: hyprctl(["hyprsunset", "temperature", str(int(w.get_value()))]))

        reset = Gtk.Button(label="Reset")
        reset.add_css_class("link-tool")
        reset.connect("clicked", lambda _b: (scale.set_value(6500), hyprctl(["hyprsunset", "identity"])))

        line = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        line.append(scale)
        line.append(reset)
        card.append(self._row("Colour temperature",
                              "Overrides hyprsunset until its next scheduled profile", line))
        return card

    # ---- save and revert -------------------------------------------------

    def _current_values(self):
        tree = {}
        for spec, widget in self.controls.values():
            if isinstance(widget, Gtk.Switch):
                value = widget.get_active()
            elif isinstance(widget, Gtk.DropDown):
                value = (spec["options"][widget.get_selected()]
                         if spec["kind"] == "text-choice" else widget.get_selected())
            else:
                value = widget.get_value()
                value = int(value) if spec["kind"] == "scale-int" else round(value, 2)
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
        content = (
            f"{MARKER}\n"
            "-- Written by System Settings. hyprland.lua requires this file last, so\n"
            "-- anything here wins over the tracked config. Hand-edit freely; this block\n"
            "-- is overwritten wholesale on the next Save.\n\n"
            "hl.config({\n"
            f"{self._render_lua(self._current_values())}\n"
            "})\n"
        )
        try:
            LOCAL_LUA.parent.mkdir(parents=True, exist_ok=True)
            if LOCAL_LUA.exists() and MARKER not in LOCAL_LUA.read_text():
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
        super().__init__(application_id="dev.parlett.systemsettings")

    def do_activate(self):
        provider = Gtk.CssProvider()
        provider.load_from_data(CSS)
        Gtk.StyleContext.add_provider_for_display(
            Gdk.Display.get_default(), provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
        SystemSettings(self).present()


if __name__ == "__main__":
    os.environ.setdefault("GDK_BACKEND", "wayland")
    raise SystemExit(App().run(None))
