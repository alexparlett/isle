pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import qs.services

// The windows open at the end of a session come back at the start of the next: one launch per app and
// workspace, for every window whose class has a desktop entry, each put on its workspace once it maps.
// Noted from the compositor's client list as Windows polls it, kept in the state directory, replayed once
// per compositor instance so a shell restart replays nothing.
Singleton {
    id: root

    readonly property string file: (Quickshell.env("XDG_STATE_HOME") || Quickshell.env("HOME") + "/.local/state") + "/isle/session.json"
    readonly property bool enabled: Prefs.p.restoreSession !== false

    // [{ app, cls, workspace }], workspace 0 for a window on a special workspace: it opens wherever it opens.
    property var noted: []
    property string written: ""
    property bool restoring: false
    function note(list) {
        if (restoring || !enabled) return;
        const out = [];
        const seen = {};
        for (const c of list) {
            if (!c.mapped || !c["class"] || !c.title || !c.workspace) continue;
            const e = DesktopEntries.heuristicLookup(c["class"]);
            if (!e) continue;
            const ws = c.workspace.id > 0 ? c.workspace.id : 0;
            const key = c["class"] + "@" + ws;
            if (seen[key]) continue;
            seen[key] = true;
            out.push({ app: e.id, cls: c["class"], workspace: ws });
        }
        noted = out;
        const text = JSON.stringify(out);
        if (text === written || writer.running) return;
        written = text;
        writer.command = ["sh", "-c", "mkdir -p \"$(dirname \"$1\")\" && printf %s \"$2\" > \"$1\"", "_", file, text];
        writer.running = true;
    }
    Process { id: writer }

    // The last session's list, once per compositor instance: the marker is keyed by the instance signature.
    Process {
        id: reader
        command: ["sh", "-c", "d=\"${XDG_RUNTIME_DIR:-/tmp}/isle\"; mkdir -p \"$d\"; f=\"$d/restored-$HYPRLAND_INSTANCE_SIGNATURE\"; [ -e \"$f\" ] || { : > \"$f\"; cat \"$1\" 2>/dev/null; }", "_", file]
        running: root.enabled
        stdout: StdioCollector {
            onStreamFinished: {
                let list; try { list = JSON.parse(text); } catch (e) { return; }
                if (!Array.isArray(list) || !list.length) return;
                root.written = text;
                root.queued = list;
                root.restoring = true;
                Startup.refresh();
                launchLater.start();
            }
        }
    }
    property var queued: []
    // Autostart has had its moment by then, so what it starts is not started twice.
    Timer { id: launchLater; interval: 2500; onTriggered: root.launch() }
    // class → the workspaces its next windows go to, consumed as they map.
    property var pending: ({})
    function launch() {
        const auto = {};
        for (const e of Startup.entries) if (e.enabled && e.applies) auto[String(e.id).replace(/\.desktop$/, "")] = true;
        const open = {};
        for (const t of Hyprland.toplevels.values) if (t.wayland) open[t.wayland.appId] = true;
        const p = {};
        let n = 0;
        for (const w of queued) {
            const entry = DesktopEntries.byId(w.app) || DesktopEntries.heuristicLookup(w.cls);
            if (!entry) continue;
            if (!p[w.cls]) p[w.cls] = [];
            p[w.cls].push(w.workspace);
            if (auto[String(w.app).replace(/\.desktop$/, "")] || open[w.cls]) continue;
            entry.execute();
            n++;
        }
        pending = p;
        queued = [];
        if (n) IslandEvents.show({ kind: "text", duration: 4000, glyph: "rotate-cw", text: "Reopening " + n + (n === 1 ? " app" : " apps") });
        settle.start();
    }
    Timer { id: settle; interval: 60000; onTriggered: { root.pending = {}; root.restoring = false; } }
    Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (event.name !== "openwindow") return;
            const [addr, ws, cls, title] = event.data.split(",");
            const list = root.pending[cls];
            if (!list || !list.length) return;
            const to = list.shift();
            if (to > 0 && String(to) !== String(ws)) Hyprland.dispatch("hl.dsp.window.move({ workspace = " + to + ", window = \"address:0x" + addr + "\" })");
        }
    }

    IpcHandler {
        target: "restore"
        function status(): string { return JSON.stringify({ enabled: root.enabled, noted: root.noted, pending: root.pending, restoring: root.restoring }); }
        function replay(): void { replayer.running = true; }
    }
    // The list replayed on demand, for trying it without a new session.
    Process {
        id: replayer
        command: ["cat", root.file]
        stdout: StdioCollector { onStreamFinished: { let l; try { l = JSON.parse(text); } catch (e) { return; } root.queued = l; root.restoring = true; Startup.refresh(); launchLater.start(); } }
    }
}
