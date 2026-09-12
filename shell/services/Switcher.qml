pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The window switcher's state: open while the modifier is held, an index into a row of windows, commit on release.
Singleton {
    id: root

    property bool open: false
    property int index: 0
    // One entry per app with a window on a desktop (hidden ones come back through the dock or Mission
    // Control), in the apps' order; frozen while open so the row does not reorder. `win` is the window a
    // release lands on, the app's most recent, moved along its list by the backtick.
    // [{ appId, name, icon, win, windows }]
    property var items: []

    function begin() {
        // Mission Control has the windows already; Super+Tab there is its Tab.
        if (Surfaces.overview) return;
        const out = [];
        for (const a of Windows.apps) {
            const ws = a.windows.filter(w => !w.hidden);
            if (ws.length) out.push({ appId: a.appId, name: a.name, icon: a.icon, win: ws[0], windows: ws });
        }
        items = out;
        index = items.length > 1 ? 1 : 0;
        open = items.length > 0;
    }
    function next() { if (!open) begin(); else if (items.length) index = (index + 1) % items.length; }
    function prev() { if (!open) { begin(); index = items.length ? items.length - 1 : 0; } else if (items.length) index = (index + items.length - 1) % items.length; }
    function commit() {
        if (!open) return;
        const it = items[index];
        open = false;
        if (it) { Surfaces.dashboard = false; Windows.focus(it.win); }
    }
    function cancel() { open = false; }
    // The backtick with the switcher up moves through the highlighted app's windows, so the release lands on
    // the one shown; with it down, the focused app cycles its own.
    function cycleItem() {
        const it = items[index];
        if (!it || it.windows.length < 2) return;
        const out = items.slice();
        out[index] = Object.assign({}, it, { win: it.windows[(it.windows.indexOf(it.win) + 1) % it.windows.length] });
        items = out;
    }
    Process { id: hider }
    // Held on a window: Q closes every window of its app, W that window. The switcher stays open on the rest.
    function quit() { const it = items[index]; if (!open || !it) return; for (const w of it.windows) Windows.closeWindow(w); }
    function closeFront() { const it = items[index]; if (!open || !it) return; Windows.closeWindow(it.win); }

    IpcHandler {
        target: "switcher"
        function next(): void { root.next(); }
        function prev(): void { root.prev(); }
        function commit(): void { root.commit(); }
        function cancel(): void { root.cancel(); }
        function cycleApp(): void { if (root.open) root.cycleItem(); else Windows.cycleApp(); }
        function hide(): void { if (root.open) { const it = root.items[root.index]; if (it) { hider.command = ["python3", Quickshell.shellDir + "/scripts/hidewindow.py", it.win.address]; hider.running = true; return; } } Windows.hideActive(); }
        function quit(): void { root.quit(); }
        function closeFront(): void { root.closeFront(); }
    }
}
