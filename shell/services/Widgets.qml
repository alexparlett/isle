pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The widget registry: every widget.json under shell/widgets and ~/.config/isle/widgets, and the dashboard layout.
Singleton {
    id: root

    // Built-ins load through the config's own URL scheme so they can import qs.* modules; user widgets by path.
    readonly property string builtinDir: Quickshell.shellDir + "/widgets"
    readonly property string userDir: Prefs.dir + "/widgets"

    // id -> { id, name, glyph, sizes, default, requires, dir }
    property var manifests: ({})
    property var ids: []

    Process {
        id: scan
        command: ["sh", "-c", "for d in \"$1\" \"$2\"; do [ -d \"$d\" ] || continue; for m in \"$d\"/*/widget.json; do [ -f \"$m\" ] || continue; printf '%s\\t' \"$(dirname \"$m\")\"; tr -d '\\n' < \"$m\"; echo; done; done", "_", root.builtinDir, root.userDir]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                const out = {}, ids = [];
                for (const line of text.split("\n")) {
                    const tab = line.indexOf("\t");
                    if (tab < 0) continue;
                    try {
                        const m = JSON.parse(line.slice(tab + 1));
                        m.dir = line.slice(0, tab);
                        out[m.id] = m;
                        ids.push(m.id);
                    } catch (e) { console.warn("widget manifest unreadable:", line.slice(0, tab)); }
                }
                root.manifests = out;
                root.ids = ids;
            }
        }
    }

    function rescan() { scan.running = true; }
    function componentUrl(id) {
        const m = manifests[id];
        if (!m) return "";
        if (m.dir.indexOf(builtinDir) === 0) return Qt.resolvedUrl("../widgets/" + id + "/Widget.qml");
        return "file://" + m.dir + "/Widget.qml";
    }

    // A layout entry is { id, x, y, w, h } in grid cells. Unknown ids are kept but not drawn.
    readonly property var defaultLayout: [
        { id: "clock", x: 0, y: 0, w: 3, h: 1 },
        { id: "media", x: 3, y: 0, w: 3, h: 1 },
        { id: "workspaces", x: 6, y: 0, w: 3, h: 1 },
        { id: "notifications", x: 9, y: 0, w: 3, h: 4 },
        { id: "controls", x: 0, y: 1, w: 3, h: 2 },
        { id: "system", x: 3, y: 1, w: 3, h: 2 },
        { id: "agents", x: 6, y: 1, w: 3, h: 2 },
        { id: "session", x: 0, y: 3, w: 3, h: 1 },
        { id: "devices", x: 3, y: 3, w: 3, h: 1 },
        { id: "storage", x: 6, y: 3, w: 3, h: 1 },
    ]
    readonly property var layout: Prefs.p.dashboard && Prefs.p.dashboard.length ? Prefs.p.dashboard : defaultLayout
    function setLayout(l) { Prefs.p.dashboard = l; }

    // --- editing --------------------------------------------------------------------

    readonly property int columns: 12
    readonly property int rows: 4
    function overlaps(a, b) { return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h; }
    function fits(entry, others) {
        if (entry.x < 0 || entry.y < 0 || entry.x + entry.w > columns || entry.y + entry.h > rows) return false;
        return !others.some(o => o.id !== entry.id && overlaps(entry, o));
    }
    function move(id, x, y) {
        const l = layout.map(e => Object.assign({}, e));
        const e = l.find(e => e.id === id);
        if (!e) return false;
        const moved = Object.assign({}, e, { x: x, y: y });
        if (!fits(moved, l)) return false;
        Object.assign(e, moved); setLayout(l); return true;
    }
    // The next size in the manifest's list that fits where the widget is.
    function cycleSize(id) {
        const l = layout.map(e => Object.assign({}, e));
        const e = l.find(e => e.id === id), m = manifests[id];
        if (!e || !m) return;
        const sizes = (m.sizes || []).map(s => s.split("x").map(Number));
        const cur = sizes.findIndex(s => s[0] === e.w && s[1] === e.h);
        for (let i = 1; i <= sizes.length; i++) {
            const s = sizes[(cur + i) % sizes.length];
            const next = Object.assign({}, e, { w: s[0], h: s[1] });
            if (fits(next, l)) { Object.assign(e, next); setLayout(l); return; }
        }
    }
    function remove(id) { setLayout(layout.filter(e => e.id !== id)); }
    function add(id) {
        const m = manifests[id];
        if (!m || layout.some(e => e.id === id)) return;
        const [w, h] = (m.default || "3x1").split("x").map(Number);
        for (let y = 0; y <= rows - h; y++) for (let x = 0; x <= columns - w; x++) {
            const e = { id: id, x: x, y: y, w: w, h: h };
            if (fits(e, layout)) { setLayout(layout.concat([e])); return; }
        }
    }
    // Place at a cell when the widget fits there; otherwise the first free spot.
    function addAt(id, x, y) {
        const m = manifests[id];
        if (!m || layout.some(e => e.id === id)) return;
        const [w, h] = (m.default || "3x1").split("x").map(Number);
        const e = { id: id, x: Math.min(x, columns - w), y: Math.min(y, rows - h), w: w, h: h };
        if (fits(e, layout)) setLayout(layout.concat([e])); else add(id);
    }
    function resetLayout() { Prefs.p.dashboard = []; }

    // --- settings -------------------------------------------------------------------
    // A manifest may carry `settings`: [{ key, label, type: "toggle" | "choice" | "number", default, options, min, max }].
    // Stored per widget in prefs; a widget reads them as `settings.<key>` with the manifest default filled in.
    readonly property var stored: Prefs.p.widgetSettings
    function settingsFor(id) {
        const m = manifests[id], saved = stored[id] || {}, out = {};
        for (const s of (m && m.settings) || []) out[s.key] = saved[s.key] !== undefined ? saved[s.key] : s.default;
        return out;
    }
    function setSetting(id, key, value) {
        const all = Object.assign({}, stored);
        all[id] = Object.assign({}, all[id] || {});
        all[id][key] = value;
        Prefs.p.widgetSettings = all;
    }
    function clearSettings(id) { const all = Object.assign({}, stored); delete all[id]; Prefs.p.widgetSettings = all; }
    readonly property var unplaced: ids.filter(id => !layout.some(e => e.id === id))
}
