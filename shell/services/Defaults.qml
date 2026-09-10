pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Default applications: the browser and one handler per common MIME type, through xdg-mime and xdg-settings.
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/scripts/defaults.py"
    property string browser: ""
    // mime -> desktop id
    property var mimes: ({})
    // desktop id -> { name, icon, mimes }
    property var apps: ({})

    // What Settings shows: a label and the MIME types it covers. The first is the one queried; setting applies to all.
    readonly property var kinds: [
        // `category` is the desktop-entry category that marks an app as being for the role, so a browser does
        // not show up as an image viewer for declaring image/png, nor an editor as a browser for text/html.
        { id: "browser", label: "Web browser", glyph: "globe", category: "WebBrowser", mimes: ["x-scheme-handler/http", "x-scheme-handler/https", "text/html"] },
        { id: "mail", label: "Mail", glyph: "mail", category: "Email", mimes: ["x-scheme-handler/mailto"] },
        { id: "files", label: "File manager", glyph: "folder", category: "FileManager", mimes: ["inode/directory"] },
        { id: "text", label: "Text", glyph: "file-text", category: "TextEditor", mimes: ["text/plain"] },
        { id: "images", label: "Images", glyph: "image", category: "Viewer", mimes: ["image/png", "image/jpeg"] },
        { id: "video", label: "Video", glyph: "film", category: "Video", mimes: ["video/mp4", "video/x-matroska"] },
        { id: "music", label: "Music", glyph: "music", category: "Audio", mimes: ["audio/mpeg", "audio/flac"] },
        { id: "pdf", label: "PDF", glyph: "file-text", category: "Viewer", mimes: ["application/pdf"] },
        { id: "archives", label: "Archives", glyph: "archive", category: "Archiving", mimes: ["application/zip"] },
    ]

    Process {
        id: lister
        command: ["python3", root.script, "list"]
        stdout: StdioCollector { onStreamFinished: { try { const d = JSON.parse(text); root.browser = d.browser; root.mimes = d.mimes; root.apps = d.apps; } catch (e) {} } }
    }
    function refresh() { lister.running = true; }

    function current(kind) { return kind.id === "browser" && browser ? browser : mimes[kind.mimes[0]] || ""; }
    // Apps that handle the role's types and declare its category; when none declares it, any handler will do.
    function candidates(kind) {
        const handles = id => kind.mimes.some(m => apps[id].mimes.indexOf(m) >= 0);
        const declares = id => (apps[id].categories || []).indexOf(kind.category) >= 0;
        let ids = Object.keys(apps).filter(id => handles(id) && declares(id));
        if (ids.length === 0) ids = Object.keys(apps).filter(handles);
        const out = ids.map(id => [id, apps[id].name]);
        out.sort((a, b) => a[1].localeCompare(b[1]));
        const cur = current(kind);
        if (cur && !out.some(o => o[0] === cur)) out.unshift([cur, cur.replace(/\.desktop$/, "")]);
        return out;
    }

    Process { id: setter; onExited: root.refresh() }
    function set(kind, id) {
        let cmd = kind.mimes.map(m => "python3 " + JSON.stringify(script) + " set " + m + " " + JSON.stringify(id)).join("; ");
        if (kind.id === "browser") cmd += "; python3 " + JSON.stringify(script) + " set-browser " + JSON.stringify(id);
        setter.command = ["sh", "-c", cmd];
        setter.running = true;
    }

    // The terminal is the shell's own: the keymap's `terminal` argument, rendered into the bind fragment.
    // Only the terminals actually on the machine are offered; the one in use is always listed.
    readonly property var knownTerminals: [["kitty", "kitty"], ["foot", "foot"], ["alacritty", "Alacritty"], ["ghostty", "Ghostty"], ["wezterm", "WezTerm"]]
    property var installedTerminals: ["kitty"]
    readonly property var terminals: knownTerminals.filter(t => installedTerminals.indexOf(t[0]) >= 0 || t[0] === (Prefs.p.terminal || "kitty"))
    Process {
        command: ["sh", "-c", "for t in kitty foot alacritty ghostty wezterm; do command -v \"$t\" >/dev/null 2>&1 && echo \"$t\"; done"]
        running: true
        stdout: StdioCollector { onStreamFinished: root.installedTerminals = text.trim().split("\n").filter(Boolean) }
    }
    function setTerminal(cmd) { Prefs.p.terminal = cmd === "kitty" ? "" : cmd; Keyboard.render(true); }
}
