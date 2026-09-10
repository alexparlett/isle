pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Mounted filesystems with their usage, from df, refreshed every thirty seconds while anything listens.
Singleton {
    id: root

    property int listeners: 0
    // [{ target, size, used, fstype, pct }]
    property var mounts: []

    Timer {
        interval: 30000
        running: root.listeners > 0
        repeat: true
        triggeredOnStart: true
        onTriggered: if (!df.running) df.running = true
    }

    Process {
        id: df
        command: ["df", "-B1", "--output=target,size,used,fstype", "-x", "tmpfs", "-x", "devtmpfs", "-x", "efivarfs", "-x", "overlay", "-x", "squashfs", "-x", "9p"]
        stdout: StdioCollector {
            onStreamFinished: {
                const out = [];
                for (const line of text.trim().split("\n").slice(1)) {
                    const f = line.trim().split(/\s+/);
                    if (f.length < 4 || f[0].indexOf("/boot") === 0 || f[0].indexOf("/run") === 0) continue;
                    const size = Number(f[1]), used = Number(f[2]);
                    out.push({ target: f[0], size: size, used: used, fstype: f[3], pct: size ? used / size : 0 });
                }
                root.mounts = out;
            }
        }
    }

    function refresh() { df.running = true; }
    function label(m) { return m.target === "/" ? "System" : m.target.split("/").pop() || m.target; }
}
