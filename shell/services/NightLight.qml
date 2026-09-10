pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Night light through hyprsunset. The state is a preference so it survives a restart.
Singleton {
    id: root

    readonly property bool available: found
    property bool found: false
    readonly property bool on: Prefs.p.nightLight
    readonly property int temperature: Prefs.p.nightLightTemperature

    Process {
        command: ["sh", "-c", "command -v hyprsunset"]
        running: true
        onExited: code => { root.found = code === 0; if (root.found) root.apply(); }
    }

    Process {
        id: daemon
        command: ["hyprsunset"]
        stdout: SplitParser {}
        stderr: SplitParser {}
    }
    // A daemon that stops answering is replaced and the request sent once more.
    property bool retried: false
    Process {
        id: ctl
        onExited: code => {
            if (code === 0 || root.retried) { root.retried = false; return; }
            root.retried = true;
            daemon.running = false;
            daemon.running = true;
            retry.restart();
        }
    }
    Timer { id: retry; interval: 700; onTriggered: root.apply() }

    function apply() {
        if (!found) return;
        if (!daemon.running) daemon.running = true;
        ctl.command = ["hyprctl", "hyprsunset", on ? "temperature" : "identity"].concat(on ? [String(temperature)] : []);
        ctl.running = true;
    }

    function setOn(v) { Prefs.p.nightLight = v; apply(); }
}
