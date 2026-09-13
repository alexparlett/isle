pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The Files windows that are open. One entry each, in the order they were opened; a window keeps its
// own tabs and tells this only where it is, so a reload can put them back.
//
// Nothing here may import Isle.Files (D72).
Singleton {
    id: root

    // [{ id, path }] — the folder a window is to open at, and what it is called while it lives.
    property var windows: []
    property int nextId: 1

    readonly property bool any: windows.length > 0

    function open(path) {
        if (!windows.length) { add(path); return; }
        // A window is already there: it takes another tab rather than a second window appearing.
        // Asking for no folder in particular is asking for the window itself, not for a tab.
        if (!path) return;
        lastAsked = { id: windows[windows.length - 1].id, path: path };
        raised();
    }

    // A window is asked to take a tab by watching this rather than by being called, since the
    // service cannot reach into a window it did not make. It is taken rather than read, so the same
    // ask cannot be answered twice.
    property var lastAsked: null
    function takeAsked(id) {
        const asked = lastAsked;
        if (!asked || asked.id !== id) return null;
        lastAsked = null;
        return asked;
    }
    signal raised()

    function add(path) {
        windows = windows.concat([{ id: nextId++, path: path || Quickshell.env("HOME") }]);
    }

    function close(id) {
        windows = windows.filter(w => w.id !== id);
    }

    function closeAll() { windows = []; }
}
