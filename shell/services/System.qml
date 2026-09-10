pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// CPU, memory, network and GPU readouts from /proc and sysfs, sampled every two seconds while anything listens.
Singleton {
    id: root

    property int listeners: 0
    readonly property int samples: 30

    property real cpu: 0          // 0..1
    property real memUsed: 0      // bytes
    property real memTotal: 0
    property real netDown: 0      // bytes/s
    property real netUp: 0
    property real gpu: 0          // 0..1, -1 when unknown
    property int gpuTemp: 0
    property string uptime: ""
    property string kernel: ""

    property var cpuHistory: []
    property var memHistory: []
    property var netHistory: []
    property var gpuHistory: []

    property var last: null

    Timer {
        interval: 2000
        running: root.listeners > 0
        repeat: true
        triggeredOnStart: true
        onTriggered: if (!sampler.running) sampler.running = true
    }

    Process {
        id: sampler
        command: ["sh", "-c",
            "head -1 /proc/stat; echo ---; grep -E '^(MemTotal|MemAvailable):' /proc/meminfo; echo ---; " +
            "tail -n +3 /proc/net/dev; echo ---; cut -d' ' -f1 /proc/uptime; echo ---; uname -r; echo ---; " +
            "nvidia-smi --query-gpu=utilization.gpu,temperature.gpu --format=csv,noheader,nounits 2>/dev/null || " +
            "{ b=$(cat /sys/class/drm/card*/device/gpu_busy_percent 2>/dev/null | head -1); t=$(cat /sys/class/hwmon/hwmon*/temp1_input 2>/dev/null | head -1); [ -n \"$b\" ] && echo \"$b, $((${t:-0}/1000))\"; }"]
        stdout: StdioCollector { onStreamFinished: root.parse(text) }
    }

    function push(list, v) { const l = list.slice(); l.push(v); while (l.length > samples) l.shift(); return l; }

    function parse(text) {
        const parts = text.split("---\n");
        if (parts.length < 6) return;
        const now = Date.now();

        // cpu: user nice system idle iowait irq softirq steal
        const c = parts[0].trim().split(/\s+/).slice(1).map(Number);
        const idle = c[3] + c[4], total = c.reduce((a, b) => a + b, 0);
        let rx = 0, tx = 0;
        for (const line of parts[2].trim().split("\n")) {
            const f = line.trim().split(/\s+/);
            if (!f[0] || f[0].indexOf("lo:") === 0) continue;
            rx += Number(f[1]); tx += Number(f[9]);
        }
        if (last) {
            const dt = (now - last.t) / 1000;
            const dTotal = total - last.total, dIdle = idle - last.idle;
            cpu = dTotal > 0 ? 1 - dIdle / dTotal : 0;
            netDown = Math.max(0, (rx - last.rx) / dt);
            netUp = Math.max(0, (tx - last.tx) / dt);
            cpuHistory = push(cpuHistory, cpu);
            netHistory = push(netHistory, netDown + netUp);
        }
        last = { t: now, total: total, idle: idle, rx: rx, tx: tx };

        const mem = {};
        for (const line of parts[1].trim().split("\n")) { const m = line.match(/^(\w+):\s+(\d+)/); if (m) mem[m[1]] = Number(m[2]) * 1024; }
        memTotal = mem.MemTotal || 0;
        memUsed = memTotal - (mem.MemAvailable || 0);
        memHistory = push(memHistory, memTotal ? memUsed / memTotal : 0);

        const up = Number(parts[3].trim());
        const d = Math.floor(up / 86400), h = Math.floor((up % 86400) / 3600), mi = Math.floor((up % 3600) / 60);
        uptime = d > 0 ? d + "d " + h + "h" : h > 0 ? h + "h " + mi + "m" : mi + "m";
        kernel = parts[4].trim();

        const g = (parts[5] || "").trim().split(",").map(s => Number(s.trim()));
        if (g.length >= 1 && !isNaN(g[0]) && parts[5].trim() !== "") { gpu = g[0] / 100; gpuTemp = g[1] || 0; }
        else gpu = -1;
        gpuHistory = push(gpuHistory, Math.max(0, gpu));
    }

    function bytes(n) {
        if (n >= 1e12) return (n / 1e12).toFixed(1) + " TB";
        if (n >= 1e9) return (n / 1e9).toFixed(1) + " GB";
        if (n >= 1e6) return (n / 1e6).toFixed(0) + " MB";
        if (n >= 1e3) return (n / 1e3).toFixed(0) + " kB";
        return n + " B";
    }
    function rate(n) {
        if (n >= 1e6) return (n / 1e6).toFixed(1) + " MB/s";
        if (n >= 1e3) return (n / 1e3).toFixed(0) + " kB/s";
        return Math.round(n) + " B/s";
    }
}
