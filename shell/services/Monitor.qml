pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The Monitor window's readouts: every process with live rates, temperatures, the GPU, the control daemons.
// Sampled every two seconds while listened to. Actions on other users' processes go through polkit.
Singleton {
    id: root

    property int listeners: 0
    property var processes: []
    property var summary: ({ count: 0, threads: 0, load: ["0", "0", "0"], cpus: 1 })
    property var temps: []
    property var fans: []
    property var gpu: null
    // Thirty samples of whole-disk throughput, for the Disk tab's strip.
    property var diskReadHistory: []
    property var diskWriteHistory: []
    property var lact: null
    property var coolercontrol: null
    readonly property string me: Quickshell.env("USER")

    Timer {
        interval: 2000
        running: root.listeners > 0
        repeat: true
        triggeredOnStart: true
        onTriggered: if (!sampler.running) sampler.running = true
    }

    Process {
        id: sampler
        command: ["python3", Quickshell.shellDir + "/scripts/sysinfo.py"]
        stdout: StdioCollector {
            onStreamFinished: {
                try {
                    const d = JSON.parse(text);
                    root.processes = d.processes; root.summary = d.summary; root.temps = d.temps; root.fans = d.fans || []; root.gpu = d.gpu; root.lact = d.lact; root.coolercontrol = d.coolercontrol;
                    root.diskReadHistory = root.diskReadHistory.concat([d.summary.disk.read]).slice(-30);
                    root.diskWriteHistory = root.diskWriteHistory.concat([d.summary.disk.write]).slice(-30);
                } catch (e) {}
            }
        }
    }
    function refresh() { if (!sampler.running) sampler.running = true; }

    // kill / renice, escalating through polkit when the process is not ours.
    Process {
        id: actor
        property var fallback: null
        onExited: code => { if (code !== 0 && fallback) { const f = fallback; fallback = null; f(); } else { fallback = null; root.refresh(); } }
    }
    function act(args, escalate) {
        actor.fallback = escalate ? () => { actor.command = ["pkexec"].concat(args); actor.running = true; } : null;
        actor.command = args;
        actor.running = true;
    }
    function terminate(p) { act(["kill", "-TERM", String(p.pid)], p.user !== me); }
    function forceQuit(p) { act(["kill", "-KILL", String(p.pid)], p.user !== me); }
    function stop(p) { act(["kill", "-STOP", String(p.pid)], p.user !== me); }
    function resume(p) { act(["kill", "-CONT", String(p.pid)], p.user !== me); }
    // Raising priority (a lower nice) always needs root.
    function renice(p, nice) { act(["renice", "-n", String(nice), "-p", String(p.pid)], p.user !== me || nice < p.nice); }

    // Per-process network through nethogs, which needs cap_net_admin and cap_net_raw (tools/install.sh sets them).
    // pid -> { down, up } in bytes per second.
    property var net: ({})
    property bool netAvailable: false
    property var netPending: ({})
    Process {
        id: nethogs
        command: ["nethogs", "-t", "-d", "2"]
        running: root.listeners > 0
        stdout: SplitParser {
            onRead: line => {
                if (line.indexOf("Refreshing") === 0) { root.net = root.netPending; root.netPending = {}; root.netAvailable = true; return; }
                const f = line.split("\t");
                if (f.length < 3) return;
                const parts = f[0].split("/");
                const pid = parseInt(parts[parts.length - 2]);
                if (!pid) return;
                root.netPending[pid] = { up: parseFloat(f[1]) * 1024, down: parseFloat(f[2]) * 1024 };
            }
        }
        onExited: code => { if (code !== 0) root.netAvailable = false; }
    }
    function netFor(pid) { return net[pid] || { up: 0, down: 0 }; }

    function launchLact() { Compositor.exec("lact"); }
    function launchCoolerControl() { Compositor.exec("coolercontrol"); }

    // A rough energy impact, as Activity Monitor has one: CPU share, plus disk and network traffic, plus GPU use.
    function energyFor(p) {
        const n = netFor(p.pid);
        return p.cpu + (p.ioRead + p.ioWrite + n.down + n.up) / 5e5 + (gpuMemFor(p.pid) > 0 ? 5 : 0);
    }
    function inhibitorFor(pid) { return (summary.inhibitors || []).find(i => i.pid === pid) || null; }

    function gpuMemFor(pid) {
        if (!gpu || !gpu.apps) return 0;
        const a = gpu.apps.find(x => x.pid === pid);
        return a ? a.mem : 0;
    }
}
