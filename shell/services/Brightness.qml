pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Brightness: the backlight through brightnessctl on a laptop, else DDC/CI through ddcutil on desktop monitors.
// ddcutil is slow (a second per call), so reads are cached and writes debounced.
Singleton {
    id: root

    property bool available: false
    property real value: 1
    property int max: 1
    // "backlight" | "ddc" | ""
    property string backend: ""
    // DDC display numbers, when that is the backend.
    property var ddcDisplays: []

    Process {
        id: probe
        command: ["brightnessctl", "-m", "-c", "backlight"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                // device,class,current,percent,max
                const line = text.trim().split("\n").find(l => l.indexOf(",backlight,") > 0);
                if (!line) { ddcProbe.running = true; return; }
                const f = line.split(",");
                root.max = parseInt(f[4]) || 1;
                root.value = (parseInt(f[2]) || 0) / root.max;
                root.backend = "backlight";
                root.available = true;
            }
        }
    }

    Process {
        id: ddcProbe
        command: ["sh", "-c", "command -v ddcutil >/dev/null && ddcutil detect --terse 2>/dev/null | awk '/^Display/ {print $2}'"]
        stdout: StdioCollector {
            onStreamFinished: {
                const nums = text.trim().split("\n").filter(l => l).map(Number).filter(n => n > 0);
                root.ddcDisplays = nums;
                if (!nums.length) { root.available = false; return; }
                root.backend = "ddc";
                root.available = true;
                ddcRead.running = true;
            }
        }
    }
    Process {
        id: ddcRead
        command: ["ddcutil", "getvcp", "10", "--terse", "--display", String(root.ddcDisplays[0] || 1)]
        stdout: StdioCollector {
            onStreamFinished: {
                // VCP 10 C <current> <max>
                const f = text.trim().split(/\s+/);
                if (f.length >= 5) { root.max = parseInt(f[4]) || 100; root.value = (parseInt(f[3]) || 0) / root.max; }
            }
        }
    }

    Process { id: setter }
    property real pending: -1
    Timer { id: debounce; interval: 250; onTriggered: root.flush() }

    function set(v) {
        if (!available) return;
        value = Math.max(0.01, Math.min(1, v));
        pending = value;
        debounce.restart();
    }
    function flush() {
        if (pending < 0) return;
        const pct = Math.round(pending * 100);
        pending = -1;
        if (backend === "backlight") setter.command = ["brightnessctl", "-q", "-c", "backlight", "set", pct + "%"];
        else setter.command = ["sh", "-c", ddcDisplays.map(d => "ddcutil setvcp 10 " + pct + " --display " + d).join("; ")];
        setter.running = true;
    }

    function refresh() { probe.running = true; }
}
