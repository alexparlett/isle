pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The widget library: every widget.json under shell/widgets and ~/.config/isle/widgets, and the dashboard layout.
Singleton {
    id: root

    // Built-ins load through the config's own URL scheme so they can import qs.* modules; a manifest-only
    // widget (one with a `source`) is drawn by the built-in text renderer; other user widgets load by path.
    readonly property string builtinDir: Quickshell.shellDir + "/widgets"
    readonly property string userDir: Prefs.dir + "/widgets"

    // id -> { id, name, glyph, category, description, sizes, default, multiple, requires, settings, source, view, dir, user }
    property var manifests: ({})
    property var ids: []
    readonly property var categoryOrder: ["Shell", "System", "Hardware", "Media", "Yours"]
    // Every manifest, by category then name: what the library lists.
    readonly property var library: ids.map(id => manifests[id]).sort((a, b) => {
        const ca = categoryOrder.indexOf(a.category), cb = categoryOrder.indexOf(b.category);
        return ca !== cb ? ca - cb : a.name.localeCompare(b.name);
    })

    Process {
        id: scan
        command: ["sh", "-c", "mkdir -p \"$2\"; for d in \"$1\" \"$2\"; do [ -d \"$d\" ] || continue; for m in \"$d\"/*/widget.json; do [ -f \"$m\" ] || continue; printf '%s\\t' \"$(dirname \"$m\")\"; tr -d '\\n' < \"$m\"; echo; done; done", "_", root.builtinDir, root.userDir]
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
                        m.user = m.dir.indexOf(root.userDir) === 0;
                        m.category = m.category || (m.user ? "Yours" : "Shell");
                        if (categoryOrder.indexOf(m.category) < 0) m.category = "Yours";
                        m.sizes = m.sizes && m.sizes.length ? m.sizes : [m.default || "3x1"];
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
        if (m.source) return Qt.resolvedUrl("../widgets/text/Widget.qml");
        if (!m.user) return Qt.resolvedUrl("../widgets/" + id + "/Widget.qml");
        return "file://" + m.dir + "/Widget.qml";
    }

    // --- the layout ----------------------------------------------------------------
    // An entry is { key, id, x, y, w, h } in grid cells: `key` names the instance (a second clock is
    // "clock#2"), `id` its widget. Entries without a key, from before instances, take their id.
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
    readonly property var layout: (Prefs.p.dashboard && Prefs.p.dashboard.length ? Prefs.p.dashboard : defaultLayout).map(e => e.key ? e : Object.assign({ key: e.id }, e))
    function setLayout(l) { Prefs.p.dashboard = l; }
    function idOf(key) { return String(key).split("#")[0]; }
    function placed(id) { return layout.filter(e => e.id === id).length; }
    // Any widget once; those marked `multiple` as often as wanted.
    function canAdd(id) { const m = manifests[id]; return !!m && (m.multiple || placed(id) === 0); }
    function uniqueKey(id) { let n = 1, k = id; while (layout.some(e => e.key === k)) k = id + "#" + (++n); return k; }

    // --- editing --------------------------------------------------------------------

    readonly property int columns: 12
    readonly property int rows: 4
    function overlaps(a, b) { return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h; }
    function fits(entry, others) {
        if (entry.x < 0 || entry.y < 0 || entry.x + entry.w > columns || entry.y + entry.h > rows) return false;
        return !others.some(o => o.key !== entry.key && overlaps(entry, o));
    }
    function move(key, x, y) {
        const l = layout.map(e => Object.assign({}, e));
        const e = l.find(e => e.key === key);
        if (!e) return false;
        const moved = Object.assign({}, e, { x: x, y: y });
        if (!fits(moved, l)) return false;
        Object.assign(e, moved); setLayout(l); return true;
    }
    // The next size in the manifest's list that fits where the widget is.
    function cycleSize(key) {
        const l = layout.map(e => Object.assign({}, e));
        const e = l.find(e => e.key === key), m = e && manifests[e.id];
        if (!e || !m) return;
        const sizes = (m.sizes || []).map(s => s.split("x").map(Number));
        const cur = sizes.findIndex(s => s[0] === e.w && s[1] === e.h);
        for (let i = 1; i <= sizes.length; i++) {
            const s = sizes[(cur + i) % sizes.length];
            const next = Object.assign({}, e, { w: s[0], h: s[1] });
            if (fits(next, l)) { Object.assign(e, next); setLayout(l); return; }
        }
    }
    function remove(key) { setLayout(layout.filter(e => e.key !== key)); }
    // Place at a cell when the widget fits there, else the first free spot; x < 0 asks for the first free spot.
    function addAt(id, x, y) {
        const m = manifests[id];
        if (!m || !canAdd(id)) return false;
        const [w, h] = (m.default || m.sizes[0] || "3x1").split("x").map(Number);
        const key = uniqueKey(id);
        if (x >= 0) {
            const e = { key: key, id: id, x: Math.min(x, columns - w), y: Math.min(y, rows - h), w: w, h: h };
            if (fits(e, layout)) { setLayout(layout.concat([e])); return true; }
        }
        for (let cy = 0; cy <= rows - h; cy++) for (let cx = 0; cx <= columns - w; cx++) {
            const e = { key: key, id: id, x: cx, y: cy, w: w, h: h };
            if (fits(e, layout)) { setLayout(layout.concat([e])); return true; }
        }
        return false;
    }
    function add(id) { return addAt(id, -1, -1); }
    function resetLayout() { Prefs.p.dashboard = []; }

    // --- settings -------------------------------------------------------------------
    // A manifest may carry `settings`: [{ key, label, type: "toggle" | "choice" | "number", default, options, min, max }].
    // Stored per instance in prefs; a widget reads them as `settings.<key>` with the manifest default filled in.
    readonly property var stored: Prefs.p.widgetSettings
    function settingsFor(key) {
        const m = manifests[idOf(key)], saved = stored[key] || {}, out = {};
        for (const s of (m && m.settings) || []) out[s.key] = saved[s.key] !== undefined ? saved[s.key] : s.default;
        return out;
    }
    function setSetting(key, k, value) {
        const all = Object.assign({}, stored);
        all[key] = Object.assign({}, all[key] || {});
        all[key][k] = value;
        Prefs.p.widgetSettings = all;
    }
    function clearSettings(key) { const all = Object.assign({}, stored); delete all[key]; Prefs.p.widgetSettings = all; }

    // --- the user's own widgets ---------------------------------------------------------
    // A text widget is a manifest alone: { id, name, glyph, source: { command | file, interval }, view, unit, max }.
    Process { id: writer; onExited: root.rescan() }
    function slug(name) { return String(name).toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "widget"; }
    function saveUser(manifest) {
        const m = Object.assign({}, manifest);
        delete m.dir; delete m.user;
        const dir = userDir + "/" + m.id;
        writer.command = ["sh", "-c", "mkdir -p \"$1\" && printf '%s\\n' \"$2\" > \"$1/widget.json\"", "_", dir, JSON.stringify(m, null, 2)];
        writer.running = true;
    }
    function deleteUser(id) {
        const m = manifests[id];
        if (!m || !m.user || m.dir.indexOf(userDir + "/") !== 0) return;
        setLayout(layout.filter(e => e.id !== id));
        writer.command = ["rm", "-r", "--", m.dir];
        writer.running = true;
    }
    function openUserDir() { Compositor.exec("xdg-open " + JSON.stringify(userDir)); }
}
