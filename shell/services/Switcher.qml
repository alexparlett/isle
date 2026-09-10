pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The app switcher's state: open while the modifier is held, an index into Windows.apps, commit on release.
Singleton {
    id: root

    property bool open: false
    property int index: 0
    // Frozen while open so the row does not reorder under the pointer.
    property var apps: []

    function begin() {
        // Mission Control has the windows already; Super+Tab there is its Tab.
        if (Surfaces.overview) return;
        apps = Windows.apps;
        index = apps.length > 1 ? 1 : 0;
        open = apps.length > 0;
    }
    function next() { if (!open) begin(); else if (apps.length) index = (index + 1) % apps.length; }
    function prev() { if (!open) { begin(); index = apps.length ? apps.length - 1 : 0; } else if (apps.length) index = (index + apps.length - 1) % apps.length; }
    function commit() {
        if (!open) return;
        const app = apps[index];
        open = false;
        if (!app) return;
        const win = app.windows.find(w => w.focused) || app.windows[0];
        if (win) { Surfaces.dashboard = false; Windows.focus(win); }
    }
    function cancel() { open = false; }
    Process { id: hider }
    // Held on an app: Q closes every window of it, W its front one. The switcher stays open on the rest.
    function quit() { const app = apps[index]; if (!open || !app) return; for (const w of app.windows) Windows.closeWindow(w); }
    function closeFront() { const app = apps[index]; if (!open || !app) return; Windows.closeWindow(app.windows.find(w => w.focused) || app.windows[0]); }

    IpcHandler {
        target: "switcher"
        function next(): void { root.next(); }
        function prev(): void { root.prev(); }
        function commit(): void { root.commit(); }
        function cancel(): void { root.cancel(); }
        function cycleApp(): void { Windows.cycleApp(); }
        function hide(): void { if (root.open) { const app = root.apps[root.index]; const w = app && (app.windows.find(w => w.focused) || app.windows[0]); if (w) { hider.command = ["python3", Quickshell.shellDir + "/scripts/hidewindow.py", w.address]; hider.running = true; return; } } Windows.hideActive(); }
        function quit(): void { root.quit(); }
        function closeFront(): void { root.closeFront(); }
    }
}
