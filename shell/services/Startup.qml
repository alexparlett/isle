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

    // Left by tools/iso/target-setup.sh; bootstrap.sh --finish removes it.
    readonly property string finishMarker: Quickshell.shellDir + "/../.finish-setup"
    Process {
        command: ["sh", "-c", "[ -e \"$1\" ] && echo yes", "_", root.finishMarker]
        running: true
        stdout: StdioCollector {
            onStreamFinished: if (text.trim() === "yes") IslandEvents.show({
                kind: "text", duration: 20000, glyph: "package",
                text: "Finish setting up Isle", detail: "keyboard profiles, cursor, title bars",
                actions: [{ label: "Finish", run: () => Compositor.exec("kitty -- bash -c 'cd \"" + Quickshell.shellDir + "/..\" && tools/bootstrap.sh --finish; read -rp \"Enter to close\"'") }]
            })
        }
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
