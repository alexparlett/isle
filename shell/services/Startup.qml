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
    function refresh() { lister.running = true; unitLister.running = true; }

    // The enabled user units: what systemd starts at login. The shell's own are marked so they are not
    // switched off by accident.
    readonly property string servicesScript: Quickshell.shellDir + "/scripts/services.py"
    property var units: []
    readonly property var essentialUnits: ["pipewire.socket", "pipewire-pulse.socket", "wireplumber.service", "gnome-keyring-daemon.socket", "gcr-ssh-agent.socket", "p11-kit-server.socket", "xremap.service", "xdg-user-dirs.service"]
    function essential(u) { return essentialUnits.indexOf(u.id) >= 0; }
    Process {
        id: unitLister
        command: ["python3", root.servicesScript, "list"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { root.units = JSON.parse(text); } catch (e) {} } }
    }
    Process { id: unitActor; onExited: unitLister.running = true }
    function setUnit(u, on) { unitActor.command = ["python3", servicesScript, on ? "enable" : "disable", u.id]; unitActor.running = true; }

    // What the compositor's start hook launches, read from hyprland.lua so the list cannot go stale.
    property var hook: []
    Process {
        command: ["sh", "-c", "grep -oE 'hl\\.exec_cmd\\(\"[^\"]*' \"$1\" | sed 's/^hl\\.exec_cmd(\"//'", "_", Quickshell.shellDir + "/../hypr/hyprland.lua"]
        running: true
        stdout: StdioCollector { onStreamFinished: root.hook = text.trim().split("\n").filter(Boolean) }
    }

    Process { id: actor; onExited: root.refresh() }
    function setEnabled(e, on) { actor.command = ["python3", script, on ? "enable" : "disable", e.id]; actor.running = true; }
    function add(desktopId) { actor.command = ["python3", script, "add", desktopId]; actor.running = true; }
    function remove(e) { actor.command = ["python3", script, "remove", e.id]; actor.running = true; }
}
