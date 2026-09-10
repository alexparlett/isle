pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// XDG autostart: runs the entries once per session, and lists and switches them for Settings.
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/scripts/autostart.py"
    // [{ id, name, icon, exec, enabled, source }]
    property var entries: []

    // The compositor spawns them, so a shell restart neither repeats nor kills them.
    Process {
        command: ["sh", "-c", "f=\"${XDG_RUNTIME_DIR:-/tmp}/isle-autostart\"; [ -e \"$f\" ] || { : > \"$f\"; echo run; }"]
        running: true
        stdout: StdioCollector { onStreamFinished: if (text.trim() === "run") Compositor.exec("dex -a -e Hyprland") }
    }

    Process {
        id: lister
        command: ["python3", root.script, "list"]
        stdout: StdioCollector { onStreamFinished: { try { root.entries = JSON.parse(text); } catch (e) {} } }
    }
    function refresh() { lister.running = true; }

    Process { id: actor; onExited: root.refresh() }
    function setEnabled(e, on) { actor.command = ["python3", script, on ? "enable" : "disable", e.id]; actor.running = true; }
    function add(desktopId) { actor.command = ["python3", script, "add", desktopId]; actor.running = true; }
    function remove(e) { actor.command = ["python3", script, "remove", e.id]; actor.running = true; }
}
