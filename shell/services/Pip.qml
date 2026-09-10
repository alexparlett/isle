pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

// Picture-in-picture windows from browsers: the compositor rule floats and pins them; this parks each new one in the
// bottom-right corner of its monitor, which the rule's move cannot do.
Singleton {
    id: root

    readonly property var pattern: /^(Picture-in-Picture|Picture in picture)$/
    property var placed: ({})

    // A window's title can arrive after the window does, so every toplevel is watched for its title.
    Instantiator {
        model: Hyprland.toplevels
        delegate: QtObject {
            required property var modelData
            readonly property string title: modelData.title || ""
            onTitleChanged: later.restart()
            Component.onCompleted: later.restart()
        }
    }
    Timer { id: later; interval: 150; onTriggered: root.place() }

    function place() {
        if (Hyprland.toplevels.values.some(t => pattern.test(t.title || "")) && !query.running) query.running = true;
    }

    // Geometry from hyprctl, since the toplevel's own IPC object lags the map.
    Process {
        id: query
        command: ["sh", "-c", "hyprctl -j clients; echo '@@'; hyprctl -j monitors"]
        stdout: StdioCollector {
            onStreamFinished: {
                let clients, monitors;
                try { const parts = text.split("@@"); clients = JSON.parse(parts[0]); monitors = JSON.parse(parts[1]); } catch (e) { return; }
                // Addresses get reused by later windows, so the key carries the pid and the map forgets closed ones.
                const live = {};
                for (const c of clients) live[c.address + ":" + c.pid] = true;
                for (const k in root.placed) if (!live[k]) delete root.placed[k];
                for (const c of clients) {
                    const key = c.address + ":" + c.pid;
                    if (!root.pattern.test(c.title || "") || root.placed[key]) continue;
                    const mon = monitors.find(m => m.id === c.monitor) || monitors[0];
                    if (!mon) continue;
                    const x = Math.round(mon.x + mon.width / mon.scale - c.size[0] - 16);
                    const y = Math.round(mon.y + mon.height / mon.scale - c.size[1] - 16);
                    Hyprland.dispatch("hl.dsp.window.move({ x = " + x + ", y = " + y + ", window = \"address:" + c.address + "\" })");
                    root.placed[key] = true;
                }
            }
        }
    }
}
