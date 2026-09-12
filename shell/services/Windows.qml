pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Hyprland
import Quickshell.Io

// Open windows: grouped by workspace, grouped by app, and the ones hidden on the "hidden" special workspace.
Singleton {
    id: root

    // The icon theme usually has the app id itself; the desktop entry is the better source when it exists.
    function iconFor(appId) {
        const entry = DesktopEntries.heuristicLookup(appId);
        return Quickshell.iconPath(entry && entry.icon ? entry.icon : appId, "application-x-executable");
    }
    function nameFor(appId) {
        const entry = DesktopEntries.heuristicLookup(appId);
        return entry ? entry.name : appId;
    }

    // A gamescope window is the app it hosts: its title starts with that app's name.
    function hostedId(appId, title) {
        if (appId !== "gamescope") return appId;
        const first = (title || "").split(/[\s:—-]/)[0];
        return first && DesktopEntries.heuristicLookup(first) ? first.toLowerCase() : appId;
    }
    function entry(t) {
        const appId = hostedId(t.wayland ? t.wayland.appId : "", t.title);
        // Dispatchers want the address as hyprctl prints it, with the 0x.
        return { appId: appId, icon: iconFor(appId), title: t.title, address: t.address.indexOf("0x") === 0 ? t.address : "0x" + t.address,
                 workspace: t.workspace ? t.workspace.id : -1, hidden: t.workspace ? t.workspace.name === "special:hidden" : false,
                 focused: t.wayland ? t.wayland.activated : false, toplevel: t };
    }

    // [{ id, focused, apps: [entry] }] for every workspace on the focused monitor, ascending.
    readonly property var groups: {
        const entries = DesktopEntries.applications.values; // dependency: entries load after start
        const byWs = {};
        for (const t of Hyprland.toplevels.values) {
            if (!t.workspace || t.workspace.id <= 0) continue;
            if (!byWs[t.workspace.id]) byWs[t.workspace.id] = [];
            byWs[t.workspace.id].push(entry(t));
        }
        const out = [];
        const focused = Hyprland.focusedWorkspace ? Hyprland.focusedWorkspace.id : -1;
        const mon = Hyprland.focusedMonitor;
        for (const w of Hyprland.workspaces.values) {
            if (w.id <= 0 || (mon && w.monitor !== mon)) continue;
            out.push({ id: w.id, focused: w.id === focused, apps: byWs[w.id] || [] });
        }
        return out.sort((a, b) => a.id - b.id);
    }

    // Windows minimised with Super+M, for the hidden stack; an app kept running lives in the tray instead.
    readonly property var hidden: {
        const out = [];
        for (const t of Hyprland.toplevels.values) if (t.workspace && t.workspace.name === "special:hidden") { const e = entry(t); if (!keeps(e.appId)) out.push(e); }
        return out;
    }

    // Most recently used first: the compositor's focus events, newest at the front, by address.
    property var recent: []
    Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (event.name !== "activewindowv2" || !event.data) return;
            const addr = event.data.indexOf("0x") === 0 ? event.data : "0x" + event.data;
            const rest = root.recent.filter(a => a !== addr);
            rest.unshift(addr);
            root.recent = rest.slice(0, 64);
        }
    }
    function rank(e) { const i = recent.indexOf(e.address); return i < 0 ? 1e6 : i; }

    // A shell command that pastes into the window at addr: the compositor sends it the paste chord, Ctrl+V or
    // Ctrl+Shift+V in a terminal. The clipboard launcher runs it after its own surface has gone.
    function pasteCommand(addr) {
        if (!addr) return "true";
        const t = Hyprland.toplevels.values.find(x => entry(x).address === addr);
        const appId = t && t.wayland ? t.wayland.appId : "";
        const mods = terminalIds.indexOf(appId) >= 0 ? "CTRL SHIFT" : "CTRL";
        return "hyprctl dispatch 'hl.dsp.send_shortcut({ mods = \"" + mods + "\", key = \"v\", window = \"address:" + addr + "\" })'";
    }

    // [{ appId, name, icon, windows: [entry], hidden, focused }] across workspaces, most recently used first, each
    // app's windows likewise, so Super+Tab twice is the window before this one.
    readonly property var apps: {
        const entries = DesktopEntries.applications.values;
        const byApp = {}, order = [];
        for (const t of Hyprland.toplevels.values) {
            if (!t.workspace) continue;
            const e = entry(t);
            if (!byApp[e.appId]) { byApp[e.appId] = { appId: e.appId, name: nameFor(e.appId), icon: e.icon, windows: [], hidden: true, focused: false }; order.push(e.appId); }
            byApp[e.appId].windows.push(e);
            if (!e.hidden) byApp[e.appId].hidden = false;
            if (e.focused) byApp[e.appId].focused = true;
        }
        const out = order.map(id => byApp[id]);
        for (const a of out) { a.windows.sort((x, y) => rank(x) - rank(y)); a.rank = rank(a.windows[0]); }
        out.sort((a, b) => (b.focused - a.focused) || (a.rank - b.rank) || (a.hidden - b.hidden));
        return out;
    }

    readonly property var terminalIds: ["kitty", "foot", "footclient", "Alacritty", "alacritty", "com.mitchellh.ghostty", "org.wezfurlong.wezterm", "wezterm"]

    // Terminal windows: where the coding agents live.
    readonly property var terminals: {
        const out = [];
        for (const t of Hyprland.toplevels.values) {
            const appId = t.wayland ? t.wayland.appId : "";
            if (terminalIds.indexOf(appId) < 0) continue;
            out.push(entry(t));
        }
        return out;
    }

    // By address through the compositor, which follows the window to its workspace; focus alone leaves a floating
    // window under the others, so it is also raised.
    function focus(e) {
        if (e.hidden) { restore(e); return; }
        Hyprland.dispatch("hl.dsp.focus({ window = \"address:" + e.address + "\" })");
        Hyprland.dispatch("hl.dsp.window.bring_to_top({ window = \"address:" + e.address + "\" })");
    }

    // Minimise: park the window on the hidden special workspace. Moving the focused window there shows that
    // The apps kept running, while they run: each sits in the tray, shown or put away as a whole.
    readonly property var kept: apps.filter(a => keeps(a.appId))
    function toggleKept(a) { if (a.hidden) { for (const w of a.windows) restore(w); } else hideApp(a); }
    function hideApp(a) {
        const addrs = a.windows.filter(w => !w.hidden).map(w => w.address);
        if (!addrs.length) return;
        hider.command = ["sh", "-c", 'for a in "$@"; do python3 "$0" "$a"; done', Quickshell.shellDir + "/scripts/hidewindow.py"].concat(addrs);
        hider.running = true;
    }
    // Closed for real, whatever the app's setting.
    function quit(e) { if (e && e.address) Hyprland.dispatch("hl.dsp.window.close({ window = \"address:" + e.address + "\" })"); }
    function quitApp(a) { for (const w of a.windows) quit(w); }
    // workspace over the desktop, so it is closed again straight after.
    Process { id: hider; command: ["python3", Quickshell.shellDir + "/scripts/hidewindow.py"] }
    function hideActive() { hider.command = ["python3", Quickshell.shellDir + "/scripts/hidewindow.py"]; hider.running = true; }
    // A window's own minimise button is honoured by the isle-windows plugin, which runs hidewindow.py.
    // Back from the hidden workspace: onto the current one, focused and on top.
    function restore(e) {
        const ws = Hyprland.focusedWorkspace ? Hyprland.focusedWorkspace.id : 1;
        Hyprland.dispatch("hl.dsp.window.move({ workspace = " + ws + ", window = \"address:" + e.address + "\" })");
        Hyprland.dispatch("hl.dsp.focus({ window = \"address:" + e.address + "\" })");
        Hyprland.dispatch("hl.dsp.window.bring_to_top({ window = \"address:" + e.address + "\" })");
    }

    // Focus the next window of the active app.
    function cycleApp() {
        const app = apps.find(a => a.focused);
        if (!app || app.windows.length < 2) return;
        const i = app.windows.findIndex(w => w.focused);
        focus(app.windows[(i + 1) % app.windows.length]);
    }

    // Windows reopen where they were last: the place of an app's main window is noted every few seconds, by
    // class, and the app's next first window is put there once it has mapped. The main window is the app's
    // only titled window: an untitled one is a menu or an overlay, and a second titled one a dialog, and
    // neither is noted nor placed, since a place is the size and position of the main window.
    function titled(cls) {
        return Hyprland.toplevels.values.filter(t => t.wayland && t.wayland.appId === cls && (t.title || "") !== "").length;
    }
    Process {
        id: placer
        command: ["hyprctl", "-j", "clients"]
        stdout: StdioCollector {
            onStreamFinished: {
                let list; try { list = JSON.parse(text); } catch (e) { return; }
                const places = Object.assign({}, Prefs.p.windowPlaces || {});
                const titledCount = {};
                for (const c of list) if (c.title) titledCount[c["class"]] = (titledCount[c["class"]] || 0) + 1;
                let changed = false;
                for (const c of list) {
                    if (!c.floating || c.fullscreen || !c.mapped || !c["class"] || !c.title || titledCount[c["class"]] !== 1) continue;
                    if (c.size[0] < 100 || c.size[1] < 100) continue;
                    const v = [c.at[0], c.at[1], c.size[0], c.size[1]];
                    if (String(places[c["class"]]) !== String(v)) { places[c["class"]] = v; changed = true; }
                }
                if (changed) Prefs.p.windowPlaces = places;
                root.reconcileFull(list);
            }
        }
    }
    Timer { interval: 4000; running: true; repeat: true; onTriggered: if (!placer.running) placer.running = true }
    Process { id: restorer }
    Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (event.name !== "openwindow") return;
            const [addr, ws, cls, title] = event.data.split(",");
            const p = (Prefs.p.windowPlaces || {})[cls];
            if (!p || Modes.game || !title || titled(cls) > 1) return;
            // After the float rule has sized it.
            restorer.command = ["sh", "-c", "sleep 0.15; hyprctl dispatch 'hl.dsp.window.resize({ x = " + p[2] + ", y = " + p[3] + ", exact = true, window = \"address:0x" + addr + "\" })'; hyprctl dispatch 'hl.dsp.window.move({ x = " + p[0] + ", y = " + p[1] + ", exact = true, window = \"address:0x" + addr + "\" })'"];
            restorer.running = true;
        }
    }

    // A window going fullscreen takes a workspace of its own, as a macOS app does, and comes back to where it
    // was when it leaves fullscreen; a window already alone on its workspace stays put. A borderless window
    // the size of its monitor, a game's "borderless fullscreen", counts as fullscreen for this. Reconciled
    // from the compositor's client list on every fullscreen event, once at start, and with the placer's poll,
    // which is what catches a window growing to the screen with no event of its own.
    property var fullFrom: ({})
    Connections {
        target: Hyprland
        function onRawEvent(event) { if (event.name === "fullscreen") fullDebounce.restart(); }
    }
    Timer { id: fullDebounce; interval: 150; onTriggered: if (!fullProc.running) fullProc.running = true }
    Component.onCompleted: fullDebounce.restart()
    Process {
        id: fullProc
        command: ["hyprctl", "-j", "clients"]
        stdout: StdioCollector { onStreamFinished: { let list; try { list = JSON.parse(text); } catch (e) { return; } root.reconcileFull(list); } }
    }
    function coversMonitor(c) {
        const m = Hyprland.monitors.values.find(m => m.id === c.monitor);
        if (!m) return false;
        const w = Math.round(m.width / m.scale), h = Math.round(m.height / m.scale);
        return c.floating && c.size[0] >= w && c.size[1] >= h && !/^(quickshell|org\.quickshell|isle-dropdown)$/.test(c["class"] || "");
    }
    function reconcileFull(list) {
        const from = Object.assign({}, root.fullFrom);
        const byWs = {};
        for (const c of list) if (c.mapped && c.workspace && c.workspace.id > 0) byWs[c.workspace.id] = (byWs[c.workspace.id] || 0) + 1;
        for (const c of list) {
            if (!c.mapped || !c.workspace) continue;
            const addr = c.address, ws = c.workspace.id, whole = c.fullscreen > 0 || coversMonitor(c);
            // The view follows only the window in use; one growing in the background stays out of the way.
            const follow = c.focusHistoryID === 0 ? "true" : "false";
            if (whole && ws > 0 && from[addr] === undefined && byWs[ws] > 1) {
                from[addr] = ws;
                Hyprland.dispatch("hl.dsp.window.move({ workspace = \"empty\", follow = " + follow + ", window = \"address:" + addr + "\" })");
            } else if (!whole && from[addr] !== undefined) {
                const back = from[addr]; delete from[addr];
                if (Hyprland.workspaces.values.some(w => w.id === back)) Hyprland.dispatch("hl.dsp.window.move({ workspace = " + back + ", follow = " + follow + ", window = \"address:" + addr + "\" })");
            }
        }
        for (const addr of Object.keys(from)) if (!list.some(c => c.address === addr)) delete from[addr];
        root.fullFrom = from;
    }

    // One of the shell's own windows (Settings, Keychain, Monitor), by title.
    function focusShellWindow(title) {
        const t = Hyprland.toplevels.values.find(t => t.wayland && t.wayland.appId === "org.quickshell" && t.title === title);
        if (t) focus(entry(t));
    }

    // The app whose id or name contains the given name, case-insensitively: a notification's sender.
    function focusApp(name) {
        const q = (name || "").toLowerCase();
        if (!q) return;
        const app = apps.find(a => a.appId.toLowerCase() === q || a.name.toLowerCase() === q)
                 || apps.find(a => a.appId.toLowerCase().indexOf(q) >= 0 || a.name.toLowerCase().indexOf(q) >= 0 || q.indexOf(a.appId.toLowerCase()) >= 0);
        if (!app) return;
        focus(app.windows.find(w => w.focused) || app.windows[0]);
    }

    // Tiling is the isle-windows plugin. The target box, in layout coordinates, while a drag hovers an edge;
    // null otherwise.
    property var ghost: null
    // Closing goes through the compositor by address: a close on a toplevel handle that is already going away
    // is a protocol error that takes the shell down.
    function closeWindow(e) {
        if (!e || !e.address) return;
        if (keeps(e.appId)) { hider.command = ["python3", Quickshell.shellDir + "/scripts/hidewindow.py", e.address]; hider.running = true; return; }
        Hyprland.dispatch("hl.dsp.window.close({ window = \"address:" + e.address + "\" })");
    }
    // An app the user keeps running has its window put away on close instead, as a Dock would; by the app's
    // desktop entry, so every window of it counts.
    function keeps(appId) {
        const entry = DesktopEntries.heuristicLookup(appId || "");
        return (Prefs.p.keepRunning || []).indexOf(entry ? entry.id : appId) >= 0;
    }
    // Super+Q: the switcher's app when it is held open, the active window otherwise.
    function closeActive() {
        if (Switcher.open) { Switcher.quit(); return; }
        const t = Hyprland.activeToplevel;
        if (t && t.wayland && keeps(t.wayland.appId)) hideActive(); else Hyprland.dispatch("hl.dsp.window.close()");
    }
    IpcHandler {
        target: "windows"
        function close(): void { root.closeActive(); }
        // The plugin posts the drag target here as it moves between edges; empty clears it.
        function ghost(rect: string): void {
            const p = rect.trim().split(/\s+/).map(Number);
            root.ghost = p.length === 4 && !p.some(isNaN) ? { x: p[0], y: p[1], w: p[2], h: p[3] } : null;
        }
    }
}
