pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Mouse, touchpad and keyboard layout preferences, rendered into the compositor's input fragment.
Singleton {
    id: root

    readonly property var p: Prefs.p.input || {}
    function get(key, fallback) { return p[key] !== undefined ? p[key] : fallback; }
    // Without a preference the layout is the system's, as localectl reports it, and gb when it has none.
    function clean(v) { return v && v !== "(unset)" ? v : ""; }
    // Layouts in order: [{ layout, variant }]; the system's, then gb, when none is set.
    readonly property var layouts: {
        const l = get("layouts", null);
        if (l && l.length) return l;
        const legacy = clean(get("layout", ""));
        return [{ layout: legacy || SystemLocale.x11Layout || "gb", variant: legacy ? clean(get("variant", "")) : SystemLocale.x11Variant }];
    }
    readonly property string layout: layouts.map(l => l.layout).join(",")
    readonly property string variant: layouts.map(l => l.variant || "").join(",")
    // XKB options: the ones chosen as toggles, plus anything typed.
    readonly property string options: (get("optionSet", []) || []).concat((get("options", "") || "").split(",").map(s => s.trim()).filter(Boolean)).filter((v, i, a) => a.indexOf(v) === i).join(",")
    onLayoutChanged: render()
    onVariantChanged: render()
    onOptionsChanged: render()
    function setLayouts(list) { set("layouts", list); }
    function setOption(name, on) {
        const cur = (get("optionSet", []) || []).filter(o => o !== name);
        if (on) cur.push(name);
        set("optionSet", cur);
    }
    function hasOption(name) { return (get("optionSet", []) || []).indexOf(name) >= 0; }
    function set(key, value) {
        Prefs.p.input = Object.assign({}, Prefs.p.input || {}, { [key]: value });
        render();
    }

    readonly property string out: Quickshell.shellDir + "/../hypr/generated/input.lua"
    Process { id: writer; onExited: reloader.running = true }
    Process { id: reloader; command: ["hyprctl", "reload"] }

    function render() {
        let lua = "-- Rendered by the Input service from prefs.input. Edit in Settings, not here.\n"
            + "hl.config({\n    input = {\n"
            + "        kb_layout = " + JSON.stringify(layout) + ",\n"
            + "        kb_variant = " + JSON.stringify(variant) + ",\n"
            + "        kb_options = " + JSON.stringify(options) + ",\n"
            + "        numlock_by_default = " + (get("numlock", false) ? "true" : "false") + ",\n"
            + "        repeat_rate = " + get("repeatRate", 25) + ",\n"
            + "        repeat_delay = " + get("repeatDelay", 400) + ",\n"
            + "        sensitivity = " + get("sensitivity", 0) + ",\n"
            + "        accel_profile = " + JSON.stringify(get("accelProfile", "flat")) + ",\n"
            + "        natural_scroll = " + (get("naturalScroll", false) ? "true" : "false") + ",\n"
            + "        left_handed = " + (get("leftHanded", false) ? "true" : "false") + ",\n"
            + "        scroll_factor = " + get("scrollFactor", 1) + ",\n"
            + "        touchpad = {\n"
            + "            natural_scroll = " + (get("touchpadNatural", true) ? "true" : "false") + ",\n"
            + "            tap_to_click = " + (get("tapToClick", true) ? "true" : "false") + ",\n"
            + "            disable_while_typing = " + (get("disableWhileTyping", true) ? "true" : "false") + ",\n"
            + "            scroll_factor = " + get("touchpadScroll", 1) + ",\n"
            + "        },\n    },\n})\n";
        lua += Keyboard.deviceLua();
        // Every window floats and snaps, as on Windows and macOS. The size is absolute, 60% by 65% of the first
        // monitor, since the compositor ignores a percent size in a rule.
        {
            const m = Displays.monitors[0], o = m ? Displays.info(m) : {};
            const w = Math.round(((o.width || 1920) / (o.scale || 1)) * 0.6), h = Math.round(((o.height || 1080) / (o.scale || 1)) * 0.65);
            lua += 'hl.window_rule({ name = "isle-float", match = { class = ".*", xwayland = false }, float = true, size = "' + w + ' ' + h + '", center = true })\n';
            // X11 windows float too, at the size they ask for: their menus and dialogs come through as windows.
            lua += 'hl.window_rule({ name = "isle-float-x11", match = { xwayland = true }, float = true })\n';
        }

        writer.command = ["sh", "-c", "printf '%s' \"$1\" > \"$2\"", "_", lua, out];
        writer.running = true;
    }
}
