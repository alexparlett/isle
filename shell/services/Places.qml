pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// The places a folder can be reached from: what the machine calls the person's folders, what they
// have bookmarked, and what is plugged in.
//
// Nothing here may import Isle.Files. A directory of QML is one module to the engine, so a service
// that cannot compile takes every other service with it, and the shell with them (D72).
Singleton {
    id: root

    readonly property string home: Quickshell.env("HOME")
    readonly property string bookmarksFile: (Quickshell.env("XDG_CONFIG_HOME") || home + "/.config") + "/gtk-3.0/bookmarks"

    property var userDirs: []
    property var bookmarks: []

    // [{ group, name, path, glyph, eject, bookmark }]. A bookmark is a place like any other: it sits
    // with the rest under Places, and is only marked so it can be taken out again.
    readonly property var places: {
        const out = [{ group: "Places", name: "Home", path: home, glyph: "house", eject: false, bookmark: false }];
        for (const d of userDirs)
            out.push({ group: "Places", name: d.name, path: d.path, glyph: d.glyph, eject: false, bookmark: false });
        for (const b of bookmarks)
            out.push({ group: "Places", name: b.name, path: b.path, glyph: "folder", eject: false, bookmark: true });
        for (const v of Disks.volumes) {
            if (!v.mounted) continue;
            out.push({ group: "Devices", name: v.label || v.name, path: v.mountpoint, glyph: "hard-drive", eject: true, bookmark: false, volume: v });
        }
        return out;
    }

    function nameOf(path) {
        if (path === home) return "Home";
        if (path === "/") return "/";
        const cut = path.replace(/\/+$/, "").lastIndexOf("/");
        return cut < 0 ? path : path.slice(cut + 1);
    }

    function isBookmarked(path) { return bookmarks.some(b => b.path === path); }

    function addBookmark(path) {
        if (!path || isBookmarked(path)) return;
        writer.command = ["sh", "-c", "mkdir -p \"$(dirname \"$1\")\" && printf '%s\\n' \"$2\" >> \"$1\"", "_", bookmarksFile, "file://" + encodeURI(path)];
        writer.running = true;
    }

    function removeBookmark(path) {
        if (!path) return;
        writer.command = ["sh", "-c", "grep -vxF \"$2\" \"$1\" > \"$1.tmp\" && mv \"$1.tmp\" \"$1\"", "_", bookmarksFile, "file://" + encodeURI(path)];
        writer.running = true;
    }

    Process { id: writer; onExited: reader.reload() }

    // What this machine calls the person's folders, from the file xdg-user-dirs writes; only the ones
    // that are there and are not simply home again.
    readonly property var glyphs: ({ DESKTOP: "monitor", DOCUMENTS: "file-text", DOWNLOAD: "download",
                                     PICTURES: "image", MUSIC: "music", VIDEOS: "film" })
    Process {
        id: dirs
        running: true
        command: ["python3", Quickshell.shellDir + "/scripts/userdirs.py"]
        stdout: StdioCollector {
            onStreamFinished: {
                let found;
                try { found = JSON.parse(text); } catch (e) { root.userDirs = []; return; }
                root.userDirs = found.map(d => ({ name: d.name, path: d.path,
                                                  glyph: root.glyphs[d.key] || "folder" }));
            }
        }
    }

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
                out.push({ path: path, name: space < 0 ? root.nameOf(path) : line.slice(space + 1) });
            }
            root.bookmarks = out;
        }
        onLoadFailed: root.bookmarks = []
    }
}
