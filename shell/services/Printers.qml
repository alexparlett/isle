pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// CUPS: printers and their jobs while Settings looks; default, pause, resume, cancel; the admin page for the rest.
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/scripts/printers.py"
    property bool available: false
    property var printers: []
    property var jobs: []
    property int listeners: 0
    property string error: ""

    Timer { interval: 5000; running: root.listeners > 0; repeat: true; triggeredOnStart: true; onTriggered: root.refresh() }
    Process {
        id: lister
        command: ["python3", root.script]
        stdout: StdioCollector { onStreamFinished: { try { const d = JSON.parse(text); root.available = d.available; root.printers = d.printers; root.jobs = d.jobs; } catch (e) {} } }
    }
    function refresh() { lister.running = true; }

    Process { id: actor; onExited: root.refresh(); stderr: StdioCollector { onStreamFinished: root.error = text.trim().split("\n").pop() || "" } }
    function run(args) { error = ""; actor.command = args; actor.running = true; }
    function setDefault(p) { run(["lpoptions", "-d", p.name]); }
    function pause(p) { run(["pkexec", "cupsdisable", p.name]); }
    function resume(p) { run(["pkexec", "cupsenable", p.name]); }
    function cancel(j) { run(["cancel", j.printer + "-" + j.id]); }
    function cancelAll(p) { run(["cancel", "-a", p.name]); }
    function testPage(p) { run(["lp", "-d", p.name, "/usr/share/cups/data/testprint"]); }
    function startService() { run(["pkexec", "systemctl", "enable", "--now", "cups.socket"]); }
    function admin() { Compositor.exec("xdg-open http://localhost:631/admin"); }
    function stateLabel(p) { return p.state === "idle" ? "Ready" : p.state === "printing" ? "Printing" : p.state === "disabled" ? "Paused" : p.state; }
}
