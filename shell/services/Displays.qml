pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

// Monitor layout: what the compositor reports, the preference per monitor, and the fragment rendered from it.
// prefs.monitors = { "<name>": { mode, scale, position, transform, vrr, enabled } }; a monitor without an entry is "preferred, auto".
Singleton {
    id: root

    readonly property var monitors: Hyprland.monitors.values
    function info(m) { return m.lastIpcObject || {}; }
    function modes(m) { return (info(m).availableModes || []); }
    function pref(name) { return (Prefs.p.monitors || {})[name] || {}; }
    // There is always a primary: the preference while that monitor is connected, else the first.
    readonly property string primary: (Prefs.p.primaryMonitor && monitors.find(m => m.name === Prefs.p.primaryMonitor)) ? Prefs.p.primaryMonitor : (monitors[0] ? monitors[0].name : "")
    // The mode a monitor is running, in the form its mode list uses.
    function currentMode(m) { const o = info(m); return o.width ? o.width + "x" + o.height + "@" + (o.refreshRate || 0).toFixed(2) + "Hz" : ""; }

    function set(name, key, value) {
        const all = Object.assign({}, Prefs.p.monitors || {});
        // The last display on stays on, whatever asks.
        if (key === "enabled" && value === false && monitors.filter(m => m.name !== name && (all[m.name] || {}).enabled !== false).length === 0) return;
        all[name] = Object.assign({}, all[name] || {}, { [key]: value });
        Prefs.p.monitors = all;
        render();
    }
    function reset(name) {
        const all = Object.assign({}, Prefs.p.monitors || {});
        delete all[name];
        Prefs.p.monitors = all;
        render();
    }

    // Dragging one monitor pins every monitor to where it is now, so the compositor cannot re-place the others.
    function place(name, x, y) {
        const all = Object.assign({}, Prefs.p.monitors || {});
        for (const m of monitors) {
            const o = info(m);
            const cur = m.name === name ? { x: x, y: y } : { x: o.x || 0, y: o.y || 0 };
            all[m.name] = Object.assign({}, all[m.name] || {}, { position: cur.x + "x" + cur.y });
        }
        Prefs.p.monitors = all;
        render();
    }

    // Positions are given relative to the first monitor by keyword, or absolute as "XxY".
    readonly property var positions: [["auto", "Auto"], ["auto-left", "Left of"], ["auto-right", "Right of"], ["auto-up", "Above"], ["auto-down", "Below"]]
    readonly property var transforms: [[0, "Normal"], [1, "90°"], [2, "180°"], [3, "270°"]]

    readonly property string out: Quickshell.shellDir + "/../hypr/generated/monitors.lua"
    Process { id: writer; onExited: reloader.running = true }
    Process { id: reloader; command: ["hyprctl", "reload"] }

    function render() {
        let lua = "-- Rendered by the Displays service from prefs.monitors. Edit in Settings, not here.\n";
        const all = Prefs.p.monitors || {};
        for (const name in all) {
            const p = all[name];
            if (p.enabled === false) { lua += "hl.monitor({ output = " + JSON.stringify(name) + ", disabled = true })\n"; continue; }
            lua += "hl.monitor({ output = " + JSON.stringify(name) + ", mode = " + JSON.stringify(p.mode || "preferred") + ", position = " + JSON.stringify(p.position || "auto")
                 + ", scale = " + (p.scale ? p.scale : "\"auto\"") + (p.transform ? ", transform = " + p.transform : "") + (p.vrr ? ", vrr = 1" : "") + " })\n";
        }
        lua += "hl.monitor({ output = \"\", mode = \"preferred\", position = \"auto\", scale = \"auto\" })\n";
        writer.command = ["sh", "-c", "printf '%s' \"$1\" > \"$2\"", "_", lua, out];
        writer.running = true;
    }

    // A monitor that comes back from a KVM switch or a cable is not always driven again: the compositor still
    // holds the connector up with a CRTC, so it never re-probes it and the link is never re-trained. Releasing
    // every output and reloading the config re-acquires them; the renderer reload alone does not (D56). It goes
    // as one shell command so nothing can leave the session without a screen half way through.
    property Process relinker: Process { id: relinker }
    function redetect() {
        if (relinker.running) return;
        let cmd = "";
        for (const m of monitors)
            if (m.name !== "FALLBACK")
                cmd += "hyprctl eval 'hl.monitor({ output = \"" + m.name + "\", disabled = true })' >/dev/null 2>&1; ";
        relinker.command = ["sh", "-c", cmd + "sleep 1; hyprctl reload >/dev/null 2>&1"];
        relinker.running = true;
    }

    // Every output gone leaves the compositor on its headless stand-in, with nothing on any screen; a reload
    // re-acquires the real ones. One attempt per time it happens, since a second changes nothing.
    readonly property bool stranded: monitors.length > 0 && monitors.every(m => m.name === "FALLBACK")
    onStrandedChanged: if (stranded) heal.restart()
    property Timer heal: Timer { id: heal; interval: 2000; onTriggered: if (root.stranded && !reloader.running) reloader.running = true }

    IpcHandler {
        target: "displays"
        function redetect(): void { root.redetect(); }
        function status(): string {
            return JSON.stringify({ stranded: root.stranded, monitors: root.monitors.map(m => ({ name: m.name, mode: root.currentMode(m), dpms: root.info(m).dpmsStatus })) });
        }
    }
}
