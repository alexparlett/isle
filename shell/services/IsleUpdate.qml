pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The shell's own updates: this installed copy against its origin. Checking fetches and counts what is
// behind and changes nothing. Update starts tools/update.py detached and reads how far it has got from
// the state file that run writes, so the reload the pull causes does not take the update with it (D78).
Singleton {
    id: root

    // shellDir is a link into the installed copy; the parent must be taken after resolving it.
    readonly property string repo: Quickshell.shellDir + "/.."
    readonly property string tool: repo + "/tools/update.py"
    readonly property string stateFile: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/isle/update.json"

    // --- what the last check found -------------------------------------------------------------
    property string head: ""
    property string headDate: ""
    property string remote: ""
    property string publicUrl: ""
    property string branch: "main"
    property var incoming: []
    readonly property int behind: incoming.length
    property bool checking: false
    property string checkedAt: ""
    property string error: ""
    // origin refused the fetch and its https spelling served it: reads work, and a push from here will not.
    property bool viaHttps: false
    // The built plugins are older than the sources or the Hyprland they were built against.
    property bool pluginsStale: false

    Process {
        id: checker
        command: ["python3", root.tool, "check"]
        onStarted: root.checking = true
        onExited: root.checking = false
        stdout: StdioCollector {
            onStreamFinished: {
                let d;
                try { d = JSON.parse(text); } catch (e) { root.error = "the check said nothing that could be read"; return; }
                root.error = d.error || "";
                root.head = d.head || "";
                root.headDate = d.headDate || "";
                root.remote = d.remote || "";
                root.publicUrl = d.publicUrl || "";
                root.branch = d.branch || "main";
                root.viaHttps = !!d.viaHttps;
                root.incoming = d.incoming || [];
                root.pluginsStale = !!d.pluginsStale;
                root.checkedAt = d.checkedAt || "";
            }
        }
    }
    function check() { if (!checker.running) checker.running = true; }
    Timer { interval: 3600000; running: true; repeat: true; triggeredOnStart: true; onTriggered: root.check() }

    // --- the run, as the run itself writes it down ----------------------------------------------
    property bool running: false
    property string phase: ""
    // [{ name, state }] where state is waiting, running, done or stopped.
    property var steps: []
    property var log: []
    property string failed: ""
    property string fromHead: ""
    property string toHead: ""

    FileView {
        id: state
        path: root.stateFile
        watchChanges: true
        onFileChanged: reload()
        onLoaded: root.took(text())
        onLoadFailed: root.idle()
    }
    function idle() {
        running = false; phase = ""; steps = []; log = []; failed = ""; fromHead = ""; toHead = "";
    }
    // A finished run is worth seeing for a moment and then gone. The age comes from the file rather than
    // from when this instance read it, so the reload the pull causes does not start the moment again. One
    // that stopped stays: the reason it gives is the only record there is of it.
    readonly property int linger: 30000
    Timer { id: forget; onTriggered: if (!root.running) root.idle() }
    function took(text) {
        let d;
        try { d = JSON.parse(text); } catch (e) { return; }
        const was = running;
        running = d.state === "running";
        phase = d.phase || "";
        steps = d.steps || [];
        log = d.log || [];
        failed = d.failed || "";
        fromHead = d.from || "";
        toHead = d.to || "";
        // Only the instance that watched the run going says it is over; one that starts up afterwards
        // reads the same finished file and says nothing.
        if (was && !running) {
            check();
            IslandEvents.show({ kind: "text", duration: 5000, glyph: failed ? "x" : "check",
                text: failed ? "Isle update stopped" : "Isle updated", detail: failed });
        }
        if (!running && !failed) {
            const age = d.finishedAt ? Date.now() - d.finishedAt * 1000 : linger;
            if (age >= linger) idle();
            else { forget.interval = linger - age; forget.restart(); }
        }
    }

    // The run is not this process's child: the pull rewrites the files the shell is running from, the
    // shell reloads, and everything it owns is destroyed in the middle of the install.
    function update() {
        if (running) return;
        forget.stop();
        running = true; phase = "Starting"; failed = ""; log = [];
        Quickshell.execDetached(["setsid", "--fork", "python3", tool, "run"]);
    }

    IpcHandler {
        target: "isle"
        function check(): void { root.check(); }
        function update(): void { root.update(); }
        function status(): string {
            return JSON.stringify({ head: root.head, behind: root.behind, incoming: root.incoming,
                running: root.running, phase: root.phase, steps: root.steps, failed: root.failed,
                pluginsStale: root.pluginsStale, error: root.error });
        }
    }
}
