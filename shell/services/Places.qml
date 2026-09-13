pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Isle.Files
import qs.services

// The places a folder can be reached from: what the machine calls the person's folders, what they
// have bookmarked, and what is plugged in.
Singleton {
    id: root

    // The user's bookmarks, kept where GTK keeps them so the two agree about what is bookmarked.
    readonly property string bookmarksFile: (Quickshell.env("XDG_CONFIG_HOME") || Quickshell.env("HOME") + "/.config") + "/gtk-3.0/bookmarks"
    property var bookmarks: []

    // [{ group, name, path, icon, eject }]
    readonly property var places: {
        const out = [];
        for (const d of Engine.userDirs())
            out.push({ group: "Places", name: d.name, path: d.path, icon: d.icon, eject: false });
        for (const b of bookmarks)
            out.push({ group: "Bookmarks", name: b.name, path: b.path, icon: "folder", eject: false });
        for (const v of Disks.volumes) {
            if (!v.mounted) continue;
            out.push({ group: "Devices", name: v.label || v.name, path: v.mountpoint, icon: "hard-drive", eject: true, volume: v });
        }
        return out;
    }

    function isBookmarked(path) { return bookmarks.some(b => b.path === path); }

    function addBookmark(path) {
        if (!path || isBookmarked(path)) return;
        writer.command = ["sh", "-c", "mkdir -p \"$(dirname \"$1\")\" && printf '%s\\n' \"$2\" >> \"$1\"", "_", bookmarksFile, "file://" + encodeURI(path)];
        writer.running = true;
    }

    function removeBookmark(path) {
        if (!path) return;
        const uri = "file://" + encodeURI(path);
        writer.command = ["sh", "-c", "grep -vxF \"$2\" \"$1\" > \"$1.tmp\" && mv \"$1.tmp\" \"$1\"", "_", bookmarksFile, uri];
        writer.running = true;
    }

    Process { id: writer; onExited: reader.reload() }

    // A line is a uri and, after a space, the name to show it under.
    FileView {
        id: reader
        path: root.bookmarksFile
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            const out = [];
            for (const line of text().split("\n")) {
                if (!line.trim()) continue;
                const space = line.indexOf(" ");
                const uri = space < 0 ? line : line.slice(0, space);
                if (!uri.startsWith("file://")) continue;
                const path = decodeURI(uri.slice(7));
                out.push({ path: path, name: space < 0 ? Engine.displayName(path) : line.slice(space + 1) });
            }
            root.bookmarks = out;
        }
        onLoadFailed: root.bookmarks = []
    }
}
