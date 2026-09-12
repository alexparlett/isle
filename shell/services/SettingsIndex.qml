pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// What Settings search looks through: every row label and description on every page, read once from the page files.
// Rows on the open page register themselves so a hit can be scrolled to and rung.
Singleton {
    id: root

    // [{ page, pageLabel, label, description }]
    property var entries: []
    // The pages, grouped the way KDE, macOS and Windows do: connect, look and feel, hardware, people and system.
    readonly property var pages: [
        { id: "network", glyph: "wifi", label: "Network" },
        { id: "bluetooth", glyph: "bluetooth", label: "Bluetooth" },
        { group: "Look and feel" },
        { id: "appearance", glyph: "palette", label: "Appearance" },
        { id: "accessibility", glyph: "accessibility", label: "Accessibility" },
        { id: "wallpaper", glyph: "image", label: "Wallpaper" },
        { id: "notifications", glyph: "bell", label: "Notifications" },
        { id: "modes", glyph: "gamepad-2", label: "Modes" },
        { group: "Hardware" },
        { id: "displays", glyph: "monitor", label: "Displays" },
        { id: "audio", glyph: "volume-2", label: "Audio" },
        { id: "keyboard", glyph: "keyboard", label: "Keyboard" },
        { id: "shortcuts", glyph: "command", label: "Shortcuts" },
        { id: "mouse", glyph: "mouse", label: "Mouse" },
        { id: "controllers", glyph: "gamepad-2", label: "Controllers" },
        { id: "printers", glyph: "printer", label: "Printers" },
        { id: "storage", glyph: "hard-drive", label: "Storage" },
        { id: "power", glyph: "power", label: "Power" },
        { id: "devices", glyph: "plug-zap", label: "Devices" },
        { id: "phone", glyph: "smartphone", label: "Phone" },
        { group: "System" },
        { id: "apps", glyph: "layout-grid", label: "Apps" },
        { id: "users", glyph: "user", label: "Users" },
        { id: "datetime", glyph: "calendar-clock", label: "Date & time" },
        { id: "calendars", glyph: "calendar", label: "Calendars" },
        { id: "passwords", glyph: "key-round", label: "Passwords" },
        { id: "region", glyph: "languages", label: "Language & region" },
        { id: "updates", glyph: "download", label: "Updates" },
        { id: "snapshots", glyph: "database-backup", label: "Snapshots" },
        { id: "about", glyph: "info", label: "About" },
    ]
    readonly property var pageLabels: { const m = {}; for (const p of pages) if (p.id) m[p.id] = p.label; return m; }

    Process {
        command: ["sh", "-c", "cd \"$1\" && for f in *Page.qml; do grep -oE '(label|description|heading): \"[^\"]*\"' \"$f\" | sed \"s/^/$f\\t/\"; done", "_", Quickshell.shellDir + "/windows/settings/pages"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                const out = [];
                let page = "", last = null;
                for (const line of text.split("\n")) {
                    const m = line.match(/^(\w+)Page\.qml\t(label|description|heading): "([^"]*)"$/);
                    if (!m) continue;
                    page = m[1].toLowerCase();
                    if (m[2] === "label") { last = { page: page, pageLabel: root.pageLabels[page] || m[1], label: m[3], description: "" }; out.push(last); }
                    else if (m[2] === "description" && last && last.page === page && !last.description) last.description = m[3];
                }
                root.entries = out;
            }
        }
    }

    function search(q) {
        const t = q.trim().toLowerCase();
        if (!t) return [];
        const words = t.split(/\s+/);
        return entries
            .map(e => {
                const hay = (e.pageLabel + " " + e.label + " " + e.description).toLowerCase();
                if (!words.every(w => hay.indexOf(w) >= 0)) return null;
                const score = e.label.toLowerCase().indexOf(t) === 0 ? 3 : e.label.toLowerCase().indexOf(t) >= 0 ? 2 : e.pageLabel.toLowerCase().indexOf(t) >= 0 ? 1 : 0;
                return { e: e, score: score };
            })
            .filter(x => x !== null)
            .sort((a, b) => b.score - a.score)
            .slice(0, 12)
            .map(x => x.e);
    }

    // The row to ring on the open page, and the rows that page has registered.
    property string highlight: ""
    property var rows: ({})
    function register(label, item) { rows[label] = item; rowsChanged(); }
    function unregister(label, item) { if (rows[label] === item) { delete rows[label]; rowsChanged(); } }
    function goTo(entry) {
        Surfaces.showSettings(entry.page);
        highlight = entry.label;
        clear.restart();
    }
    Timer { id: clear; interval: 3500; onTriggered: root.highlight = "" }
}
