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
    // Where the trash keeps what it holds. Worked out here rather than asked of the engine, since a
    // service may not depend on it (D72).
    readonly property string dataHome: Quickshell.env("XDG_DATA_HOME") || home + "/.local/share"
    readonly property string trashFiles: dataHome + "/Trash/files"
    // The gathering of what was opened lately, which is a list of paths rather than a folder.
    readonly property string recentsPath: "recents:"

    property var recents: []

    readonly property string bookmarksFile: (Quickshell.env("XDG_CONFIG_HOME") || home + "/.config") + "/gtk-3.0/bookmarks"

    property var userDirs: []
    property var bookmarks: []

    // [{ group, name, path, glyph, eject, bookmark }]. A bookmark is a place like any other: it sits
    // among the favourites, and is only marked so it can be taken out again. Recents is what was
    // open lately rather than somewhere to be, so it has no heading and stands above them all.
    readonly property var places: {
        const out = [
            { group: "", name: "Recents", path: recentsPath, glyph: "clock", eject: false, bookmark: false },
            { group: "Favourites", name: "Home", path: home, glyph: "house", eject: false, bookmark: false },
        ];
        for (const d of userDirs)
            out.push({ group: "Favourites", name: d.name, path: d.path, glyph: d.glyph, eject: false, bookmark: false });
        for (const b of bookmarks)
            out.push({ group: "Favourites", name: b.name, path: b.path, glyph: "folder", eject: false, bookmark: true });
        for (const v of Disks.volumes) {
            if (!v.mounted) continue;
            out.push({ group: "Locations", name: v.label || v.name, path: v.mountpoint, glyph: "hard-drive", eject: true, bookmark: false, volume: v });
        }
        out.push({ group: "Locations", name: "Trash", path: trashFiles, glyph: "trash", eject: false, bookmark: false });
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

    // A line is the uri, and after it whatever name whoever wrote it gave, so a bookmark is matched
    // on its first field alone and never on the whole line.
    function removeBookmark(path) {
        if (!path) return;
        writer.command = ["sh", "-c",
            "[ -f \"$1\" ] || exit 0; awk -v u=\"$2\" '$1 != u' \"$1\" > \"$1.tmp\" && mv \"$1.tmp\" \"$1\"",
            "_", bookmarksFile, "file://" + encodeURI(path)];
        writer.running = true;
    }

    // The bookmarks in a new order, written back as the lines they already were.
    function reorderBookmarks(paths) {
        const byPath = {};
        for (const b of bookmarks) byPath[b.path] = b.line;
        const lines = paths.map(p => byPath[p]).filter(l => l !== undefined);
        if (lines.length !== bookmarks.length) return;
        writer.command = ["sh", "-c",
            "f=$1; shift; printf '%s\n' \"$@\" > \"$f.tmp\" && mv \"$f.tmp\" \"$f\"",
            "_", bookmarksFile].concat(lines);
        writer.running = true;
    }

    Process { id: writer; onExited: reader.reload() }

    function refreshRecents() { recentsReader.running = true; }

    Process {
        id: recentsReader
        running: true
        command: ["python3", Quickshell.shellDir + "/scripts/recents.py"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { root.recents = JSON.parse(text); } catch (e) { root.recents = []; }
            }
        }
    }

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
                // The line is kept as it was written, so reordering rewrites the file without
                // rewriting anyone else's names.
                out.push({ path: path, name: space < 0 ? root.nameOf(path) : line.slice(space + 1), line: line });
            }
            root.bookmarks = out;
        }
        onLoadFailed: root.bookmarks = []
    }
}
