pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

// Screenshots, recordings and the colour picker over grim, slurp, wf-recorder, satty and hyprpicker.
// A region comes from slurp; "window" is slurp snapping to the compositor's client rectangles.
Singleton {
    id: root

    readonly property string home: Quickshell.env("HOME")
    readonly property string shotDir: home + "/Pictures/Screenshots"
    readonly property string videoDir: home + "/Videos/Recordings"
    property string lastFile: ""
    // The directory of frozen frames while a pick is up; FreezeFrame shows them.
    property string frozenDir: ""

    // "region" | "window" | "screen"
    property string mode: "region"
    readonly property bool recording: recorder.running
    property int elapsed: 0

    Process { command: ["mkdir", "-p", root.shotDir, root.videoDir]; running: true }

    function stamp() { return Qt.formatDateTime(new Date(), "yyyy-MM-dd_HH-mm-ss"); }
    // A clear veil and a 2px accent edge, so the pick reads as a mode and not a glitch.
    function slurpArgs() { return ["slurp", "-b", "0A0B0DA6", "-c", "7FA6FFFF", "-s", "7FA6FF22", "-w", "2", "-d"]; }
    function screenName() { return Hyprland.focusedMonitor ? Hyprland.focusedMonitor.name : ""; }

    readonly property string geometryScript: Quickshell.shellDir + "/scripts/geometry.py"
    readonly property string captureScript: Quickshell.shellDir + "/scripts/capture.sh"

    // --- screenshots ----------------------------------------------------------

    // A delay counts down in the island; the region is picked first so the countdown covers the moment itself.
    property int countdown: 0
    Timer { id: delayTimer; interval: 1000; repeat: true; onTriggered: { root.countdown--; if (root.countdown > 0) root.showCountdown(); else { delayTimer.stop(); IslandEvents.dismissKind("countdown"); root.shootNow(); } } }
    function showCountdown() { IslandEvents.show({ kind: "text", duration: 1500, glyph: "timer", text: "Capturing in " + countdown }); }
    function shoot(m) {
        if (m) mode = m;
        if (Prefs.p.captureDelay > 0) { countdown = Prefs.p.captureDelay; showCountdown(); delayTimer.start(); return; }
        shootNow();
    }
    // The pipeline runs as a compositor child (see capture.sh): slurp's overlay only maps that way. The frame
    // is frozen before the pick and the region cut out of it.
    function shootNow() {
        const file = shotDir + "/" + stamp() + ".png";
        Compositor.exec("sh " + JSON.stringify(captureScript) + " shoot " + mode + " " + JSON.stringify(file) + (Prefs.p.captureCursor ? " -c" : ""));
    }
    function shotDone(file) {
        if (!file) return;
        lastFile = file;
        IslandEvents.show({ kind: "text", duration: 6000, glyph: "camera", text: "Screenshot copied", detail: file.split("/").pop(),
                            actions: [{ label: "Annotate", run: () => root.annotate(file) }, { label: "Folder", run: () => Compositor.exec("xdg-open " + JSON.stringify(shotDir)) }] });
    }

    Process { id: annotator }
    function annotate(file) {
        const f = file || lastFile;
        if (!f) return;
        annotator.command = ["satty", "--filename", f, "--output-filename", f, "--copy-command", "wl-copy"];
        annotator.running = true;
    }

    // --- recordings -------------------------------------------------------------

    property string videoFile: ""
    Process {
        id: recorder
        onRunningChanged: {
            if (running) { root.elapsed = 0; root.showRecording(); }
            else { root.elapsed = 0; IslandEvents.dismissKind("recording"); if (root.videoFile) root.recordDone(root.videoFile); }
        }
    }
    Timer { interval: 1000; running: root.recording; repeat: true; onTriggered: { root.elapsed++; root.showRecording(); } }

    readonly property string geomFile: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/isle-capture-geom"
    function record(m) {
        if (recording) { stop(); return; }
        if (m) mode = m;
        videoFile = videoDir + "/" + stamp() + ".mp4";
        // The region is picked by a compositor child (slurp overlay); recordgeom() then starts wf-recorder.
        Compositor.exec("sh " + JSON.stringify(captureScript) + " geom " + mode + " " + JSON.stringify(geomFile));
    }
    function startRecording(geom) {
        const audio = Prefs.p.captureAudio ? " --audio" : "";
        recorder.command = ["sh", "-c", "exec wf-recorder" + audio + " -g \"$1\" -f " + JSON.stringify(videoFile), "_", geom];
        recorder.running = true;
    }
    Process { id: geomReader; stdout: StdioCollector { onStreamFinished: { const g = text.trim(); if (g) root.startRecording(g); } } }
    function stop() { if (recording) recorder.signal(2); }
    function showRecording() {
        IslandEvents.show({ kind: "recording", duration: 24 * 3600 * 1000, elapsed: elapsed });
    }
    function recordDone(file) {
        IslandEvents.show({ kind: "text", duration: 6000, glyph: "video", text: "Recording saved", detail: file.split("/").pop(),
                            actions: [{ label: "Play", run: () => Compositor.exec("xdg-open " + JSON.stringify(file)) }, { label: "Folder", run: () => Compositor.exec("xdg-open " + JSON.stringify(videoDir)) }] });
        videoFile = "";
    }

    // --- colour picker ------------------------------------------------------------

    function pick() { Compositor.exec("sh " + JSON.stringify(captureScript) + " pick"); }

    IpcHandler {
        target: "capture"
        function shoot(mode: string): void { root.shoot(mode === "-" ? "" : mode); }
        function record(mode: string): void { root.record(mode === "-" ? "" : mode); }
        function stop(): void { root.stop(); }
        function pick(): void { root.pick(); }
        function annotate(): void { root.annotate(""); }
        // Results reported by capture.sh, which runs as a compositor child.
        function frozen(dir: string): void { root.frozenDir = dir; }
        function thaw(): void { root.frozenDir = ""; }
        function saved(file: string): void { IslandEvents.dismissKind("text"); root.shotDone(file); }
        function cancelled(): void { IslandEvents.dismissKind("text"); }
        function picked(hex: string): void { IslandEvents.show({ kind: "text", duration: 3000, glyph: "pipette", text: "Copied " + hex, color: hex }); }
        function recordgeom(out: string): void { geomReader.command = ["cat", out]; geomReader.running = true; }
    }
}
