pragma Singleton
import QtQuick
import Qt.labs.folderlistmodel
import Quickshell
import Quickshell.Io
import qs.services

// The AppImages in ~/Applications and the launcher entries that stand for them. The folder is
// watched rather than a gesture hooked, so however a bundle gets in there it is installed, and
// throwing it away uninstalls it (D82).
//
// Nothing here may import Isle.Files: a service that cannot compile takes the shell with it (D72).
Singleton {
    id: root

    readonly property string folder: Quickshell.env("HOME") + "/Applications"
    // path -> { id, name, icon }, so a view can draw a bundle as the app it is.
    property var known: ({})

    function iconFor(path) { const one = known[path]; return one ? one.icon : ""; }
    // Whether a path is somewhere this would register it from, which is what makes moving it there
    // an installation rather than a copy into a folder.
    function inFolder(path) { return path.lastIndexOf("/") === folder.length && path.indexOf(folder + "/") === 0; }

    // Run it where it sits, installed or not, as a Mac runs an app from whatever folder it is in.
    // The bit is set first: a bundle that has only just been downloaded does not have one.
    function launch(path) {
        Compositor.exec("sh -c 'chmod +x \"$1\" && exec \"$1\"' _ " + Compositor.quote(path));
    }

    Process { command: ["mkdir", "-p", root.folder]; running: true }

    // The first reading is what is already installed, not news; a change after it is announced.
    property bool primed: false
    Process {
        id: syncer
        command: ["python3", Quickshell.shellDir + "/scripts/appimages.py", "sync"]
        stdout: StdioCollector {
            onStreamFinished: {
                let changed;
                try { changed = JSON.parse(text); } catch (e) { return; }
                if (root.primed) root.announce(changed.added || []);
                lister.running = true;
            }
        }
    }
    Process {
        id: lister
        command: ["python3", Quickshell.shellDir + "/scripts/appimages.py", "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                let rows;
                try { rows = JSON.parse(text); } catch (e) { rows = []; }
                const out = {};
                for (const r of rows) out[r.path] = r;
                root.known = out;
                root.primed = true;
            }
        }
    }

    function sync() { if (!syncer.running) syncer.running = true; }

    function announce(added) {
        if (!added.length) return;
        const one = added[0];
        IslandEvents.show({ kind: "text", duration: 6000, glyph: "layout-grid",
                            text: added.length === 1 ? one.name + " added to Applications"
                                                     : added.length + " apps added to Applications",
                            detail: added.length === 1 ? "" : added.map(a => a.name).join(", "),
                            actions: added.length === 1
                                ? [{ label: "Open", run: () => root.launch(one.path) }] : [] });
    }

    // The folder itself says when something has landed in it or left. A bundle written over in
    // place keeps the entry it has until the folder changes again.
    FolderListModel {
        folder: "file://" + root.folder
        showDirs: false
        showHidden: false
        onCountChanged: settle.restart()
    }
    Timer { id: settle; interval: 400; onTriggered: root.sync() }

    Component.onCompleted: sync()
}
