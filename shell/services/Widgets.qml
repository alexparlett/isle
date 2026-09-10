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
        command: ["python3", Quickshell.shellDir + "/scripts/widgets.py", root.builtinDir, root.userDir]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                let list; try { list = JSON.parse(text); } catch (e) { console.warn("widget scan failed"); return; }
                const out = {}, ids = [];
                for (const m of list) {
                    m.category = m.category || (m.user ? "Yours" : "Shell");
                    if (categoryOrder.indexOf(m.category) < 0) m.category = "Yours";
                    m.sizes = m.sizes && m.sizes.length ? m.sizes : [m.default || "3x1"];
                    out[m.id] = m;
                    ids.push(m.id);
                }
                root.manifests = out;
                root.ids = ids;
            }
        }
    }
    function rescan() { scan.running = true; }
    // A user's QML widget is sandboxed: it gets a permission-gated `host` and nothing else. QML has no
    // hard boundary, so the sandbox is enforced by source: a widget that imports `qs.*` reaches the whole
    // shell, and one that has not declared `trust: true` is refused rather than run. `trust: true` on the
    // user's own widget runs it with that full reach.
    function trusted(m) { return !!(m && m.user && m.trust === true); }
    // A user QML widget the scan flagged as reaching past the sandbox, and not trusted; the card says why.
    function blocked(m) { return !!(m && m.user && !m.source && m.restrictedIssues && m.restrictedIssues.length && !trusted(m)); }
    function issuesOf(id) { const m = manifests[id]; return (m && m.restrictedIssues) || []; }
    function componentUrl(id) {
        const m = manifests[id];
        if (!m || blocked(m)) return "";
        if (m.source) return Qt.resolvedUrl("../widgets/text/Widget.qml");
        if (!m.user) return Qt.resolvedUrl("../widgets/" + id + "/Widget.qml");
        return Qt.resolvedUrl("../userwidgets/" + id + "/Widget.qml");
    }
    function permissionsOf(id) { const m = manifests[id]; return (m && m.permissions) || []; }
    // What each permission lets a widget do, in the words the store shows.
    readonly property var permissionCatalogue: ({
        "audio": "See the volume, mute and the output device", "audio.write": "Change the volume and mute",
        "network": "See whether you are online, on Wi-Fi or wired", "media": "See what is playing", "media.write": "Play, pause and skip",
        "system": "See CPU, GPU, memory and network load", "notifications": "See the count and do-not-disturb", "notifications.write": "Turn do-not-disturb on and off",
        "power": "See the battery and power profile", "bluetooth": "See whether Bluetooth is on and what is connected", "vpn": "See whether a VPN is up",
    })
    function permissionLabel(p) { return permissionCatalogue[p] || p; }
    // The permissions a widget actually gets: those it declares, minus any the user has withdrawn.
    function grantsFor(id) {
        const declared = permissionsOf(id), g = (Prefs.p.widgetGrants || {})[id];
        return g === undefined ? declared : declared.filter(p => g.indexOf(p) >= 0);
    }
    function setGrant(id, perm, on) {
        const all = Object.assign({}, Prefs.p.widgetGrants || {});
        const cur = all[id] !== undefined ? all[id].slice() : permissionsOf(id).slice();
        all[id] = on ? cur.concat(cur.indexOf(perm) < 0 ? [perm] : []) : cur.filter(p => p !== perm);
        Prefs.p.widgetGrants = all;
    }
    function resetGrants(id) { const all = Object.assign({}, Prefs.p.widgetGrants || {}); delete all[id]; Prefs.p.widgetGrants = all; }
    // "1.2.0" as a comparable number; missing parts count as zero.
    function versionNumber(v) { const p = String(v || "0").split(".").map(x => parseInt(x, 10) || 0); return p[0] * 1000000 + (p[1] || 0) * 1000 + (p[2] || 0); }

    // --- pages and the layout --------------------------------------------------------------
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
    // Pages: the single layout from before pages becomes the first.
    readonly property var pages: Prefs.p.dashboardPages && Prefs.p.dashboardPages.length ? Prefs.p.dashboardPages : [{ name: "Home", layout: Prefs.p.dashboard || [] }]
    readonly property int page: Math.max(0, Math.min(Prefs.p.dashboardPage || 0, pages.length - 1))
    function normalize(l) { return l.map(e => e.key ? e : Object.assign({ key: e.id }, e)); }
    // The first page empty means the default layout; another page empty is empty.
    readonly property var layout: normalize(pages[page].layout && pages[page].layout.length ? pages[page].layout : (page === 0 ? defaultLayout : []))
    // A live drag keeps its planned arrangement here without touching `layout`, so the Repeater's delegates
    // are not recreated under the drag (which would drop the grab and snap the pushed cards). Cards read
    // their position through cellFor; the plan commits to the layout only on release.
    property var dragBase: null
    property var dragPlan: null
    function cellFor(key) { const src = dragPlan !== null ? dragPlan : layout; return src.find(e => e.key === key) || null; }
    function setLayout(l) {
        const ps = pages.map(p => ({ name: p.name, layout: p.layout }));
        ps[page] = { name: ps[page].name, layout: l };
        Prefs.p.dashboardPages = ps;
    }
    function setPage(i) { Prefs.p.dashboardPage = Math.max(0, Math.min(i, pages.length - 1)); }
    function addPage(name) {
        const ps = pages.map(p => ({ name: p.name, layout: p.layout })).concat([{ name: name || "Page " + (pages.length + 1), layout: [] }]);
        Prefs.p.dashboardPages = ps;
        Prefs.p.dashboardPage = ps.length - 1;
    }
    function renamePage(i, name) { const ps = pages.map(p => ({ name: p.name, layout: p.layout })); if (ps[i]) { ps[i].name = name || ps[i].name; Prefs.p.dashboardPages = ps; } }
    function removePage(i) {
        if (pages.length < 2) return;
        const ps = pages.filter((p, k) => k !== i).map(p => ({ name: p.name, layout: p.layout }));
        Prefs.p.dashboardPages = ps;
        Prefs.p.dashboardPage = Math.min(page, ps.length - 1);
    }
    function idOf(key) { return String(key).split("#")[0]; }
    function placed(id) { return layout.filter(e => e.id === id).length; }
    // Any widget once per page; those marked `multiple` as often as wanted.
    function canAdd(id) { const m = manifests[id]; return !!m && (m.multiple || placed(id) === 0); }
    function uniqueKey(id) {
        const taken = {};
        for (const p of pages) for (const e of normalize(p.layout || [])) taken[e.key] = 1;
        for (const e of layout) taken[e.key] = 1;
        let n = 1, k = id; while (taken[k]) k = id + "#" + (++n); return k;
    }
    // A widget's smallest and largest spans: `min` and `max` in the manifest, else the extremes of `sizes`.
    function spanOf(s) { const p = String(s || "3x1").split("x").map(Number); return { w: p[0] || 1, h: p[1] || 1 }; }
    function minOf(m) { if (m.min) return spanOf(m.min); const ss = (m.sizes || ["3x1"]).map(spanOf); return { w: Math.min(...ss.map(s => s.w)), h: Math.min(...ss.map(s => s.h)) }; }
    function maxOf(m) { if (m.max) return spanOf(m.max); const ss = (m.sizes || ["3x1"]).map(spanOf); return { w: Math.max(...ss.map(s => s.w)), h: Math.max(...ss.map(s => s.h)) }; }

    // --- editing --------------------------------------------------------------------

    readonly property int columns: 12
    readonly property int rows: 4
    function overlaps(a, b) { return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h; }
    function fits(entry, others) {
        if (entry.x < 0 || entry.y < 0 || entry.x + entry.w > columns || entry.y + entry.h > rows) return false;
        return !others.some(o => o.key !== entry.key && overlaps(entry, o));
    }
    // The layout with `key` at (x, y), making room: a lone neighbour of the same size swaps places,
    // otherwise neighbours are pushed down while there is room. Null when nothing works.
    function plan(key, x, y, w, h) { return planFrom(layout, key, x, y, w, h); }
    // The layout with `key` moved to (x, y): the cards it lands on cascade out of the way in the direction it
    // came from, each shoving the next, so a card dragged across the row pushes the others along rather than
    // swapping one to the far origin. Null when a pushed card would leave the grid.
    function planFrom(base, key, x, y, w, h) {
        const l = base.map(e => Object.assign({}, e));
        const e = l.find(o => o.key === key);
        if (!e) return null;
        const origX = e.x, origY = e.y;
        e.w = w || e.w; e.h = h || e.h;
        e.x = Math.max(0, Math.min(x, columns - e.w));
        e.y = Math.max(0, Math.min(y, rows - e.h));
        // Push along whichever axis the card travelled furthest, in the direction back toward where it started,
        // so displaced cards move into the space it is leaving.
        const horizontal = Math.abs(e.x - origX) >= Math.abs(e.y - origY);
        const dir = horizontal ? (origX >= e.x ? 1 : -1) : (origY >= e.y ? 1 : -1);
        // Resolve every card that `mover` overlaps by shoving it just past `mover` in `dir`, then resolving the
        // cards it in turn hits. Positions move strictly one way, so this terminates.
        function push(mover, guard) {
            for (const o of l) {
                if (o.key === mover.key || guard.indexOf(o.key) >= 0 || !overlaps(o, mover)) continue;
                if (horizontal) o.x = dir > 0 ? mover.x + mover.w : mover.x - o.w;
                else o.y = dir > 0 ? mover.y + mover.h : mover.y - o.h;
                if (o.x < 0 || o.y < 0 || o.x + o.w > columns || o.y + o.h > rows) return false;
                if (!push(o, guard.concat([mover.key]))) return false;
            }
            return true;
        }
        return push(e, [e.key]) ? l : null;
    }
    function canPlace(key, x, y, w, h) { return plan(key, x, y, w, h) !== null; }
    function place(key, x, y, w, h) { const l = plan(key, x, y, w, h); if (l) setLayout(l); return l !== null; }
    function move(key, x, y) { return place(key, x, y); }
    // Live drag: neighbours flow while the card is dragged, always re-planned from the pre-drag layout.
    function beginDrag(key) { dragBase = layout.map(e => Object.assign({}, e)); dragPlan = dragBase; }
    // Move the dragged card to (x, y) among the pre-drag arrangement; returns whether it placed there.
    function dragStep(key, x, y) {
        if (dragBase === null) return false;
        const l = planFrom(dragBase, key, x, y);
        if (l) { dragPlan = l; return true; }
        return false;
    }
    function commitDrag() { if (dragPlan !== null && dragBase !== null) setLayout(dragPlan); dragPlan = null; dragBase = null; }
    function cancelDrag() { dragPlan = null; dragBase = null; }
    // A new span, held to the manifest's range and the grid; neighbours are not moved for a resize.
    function resize(key, w, h) {
        const e = layout.find(e => e.key === key), m = e && manifests[e.id];
        if (!e || !m) return false;
        const lo = minOf(m), hi = maxOf(m);
        const nw = Math.max(lo.w, Math.min(hi.w, w, columns - e.x)), nh = Math.max(lo.h, Math.min(hi.h, h, rows - e.y));
        const next = Object.assign({}, e, { w: nw, h: nh });
        if (!fits(next, layout)) return false;
        setLayout(layout.map(o => o.key === key ? next : o));
        return true;
    }
    function canResize(key, w, h) {
        const e = layout.find(e => e.key === key), m = e && manifests[e.id];
        if (!e || !m) return false;
        const lo = minOf(m), hi = maxOf(m);
        if (w < lo.w || h < lo.h || w > hi.w || h > hi.h) return false;
        return fits(Object.assign({}, e, { w: w, h: h }), layout);
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
    function resetLayout() { setLayout([]); }

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
