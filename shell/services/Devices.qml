pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The machine's hardware, from devices.py, refreshed when udev reports a change.
Singleton {
    id: root

    // [{ id, name, glyph, collapsed, page, items: [{ name, props: [[label, value]] }] }]
    property var categories: []
    property bool loaded: false

    Process {
        id: scan
        command: ["python3", Quickshell.shellDir + "/scripts/devices.py"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                let d; try { d = JSON.parse(text); } catch (e) { return; }
                root.categories = d.categories || [];
                root.loaded = true;
            }
        }
    }
    function refresh() { if (!scan.running) scan.running = true; }

    // A burst of udev events settles before the rescan.
    Timer { id: debounce; interval: 600; onTriggered: root.refresh() }
    Process {
        command: ["udevadm", "monitor", "-u", "-s", "usb", "-s", "input", "-s", "block", "-s", "net"]
        running: true
        stdout: SplitParser { onRead: debounce.restart() }
    }
}
