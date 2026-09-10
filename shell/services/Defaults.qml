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
        { id: "browser", label: "Web browser", glyph: "globe", mimes: ["x-scheme-handler/http", "x-scheme-handler/https", "text/html"] },
        { id: "mail", label: "Mail", glyph: "mail", mimes: ["x-scheme-handler/mailto"] },
        { id: "files", label: "File manager", glyph: "folder", mimes: ["inode/directory"] },
        { id: "text", label: "Text", glyph: "file-text", mimes: ["text/plain"] },
        { id: "images", label: "Images", glyph: "image", mimes: ["image/png", "image/jpeg"] },
        { id: "video", label: "Video", glyph: "film", mimes: ["video/mp4", "video/x-matroska"] },
        { id: "music", label: "Music", glyph: "music", mimes: ["audio/mpeg", "audio/flac"] },
        { id: "pdf", label: "PDF", glyph: "file-text", mimes: ["application/pdf"] },
        { id: "archives", label: "Archives", glyph: "archive", mimes: ["application/zip"] },
    ]

    Process {
        id: lister
        command: ["python3", root.script, "list"]
        stdout: StdioCollector { onStreamFinished: { try { const d = JSON.parse(text); root.browser = d.browser; root.mimes = d.mimes; root.apps = d.apps; } catch (e) {} } }
    }
    function refresh() { lister.running = true; }

    function current(kind) { return kind.id === "browser" && browser ? browser : mimes[kind.mimes[0]] || ""; }
    function candidates(kind) {
        const out = [];
        for (const id in apps) if (kind.mimes.some(m => apps[id].mimes.indexOf(m) >= 0)) out.push([id, apps[id].name]);
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
