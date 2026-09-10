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
    readonly property var pageLabels: ({ appearance: "Appearance", keyboard: "Keyboard", shortcuts: "Shortcuts", displays: "Displays", mouse: "Mouse", audio: "Audio", network: "Network",
                                          bluetooth: "Bluetooth", storage: "Storage", apps: "Apps", users: "Users", datetime: "Date & time", region: "Language & region", printers: "Printers", updates: "Updates", notifications: "Notifications", power: "Power", modes: "Modes", about: "About" })

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
