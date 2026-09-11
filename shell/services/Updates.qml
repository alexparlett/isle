pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// Pending package updates: pacman through checkupdates, the AUR through paru or yay. Checked hourly and on
// demand. Updating runs pacman, or the helper with pacman behind pkexec, so the privilege comes through the
// shell's own auth dialog and the output is read line by line into a list of what is happening to what.
Singleton {
    id: root

    // [{ name, from, to, aur }]
    property var pending: []
    property bool checking: false
    property string checkedAt: ""
    property string helper: ""
    property string error: ""
    readonly property int count: pending.length

    Process {
        command: ["sh", "-c", "command -v paru || command -v yay || echo"]
        running: true
        stdout: StdioCollector { onStreamFinished: root.helper = text.trim().split("/").pop() }
    }
    Timer { interval: 3600000; running: true; repeat: true; triggeredOnStart: true; onTriggered: root.check() }

    Process {
        id: checker
        command: ["sh", "-c", "checkupdates 2>/dev/null | sed 's/^/repo /'; h=$(command -v paru || command -v yay); [ -n \"$h\" ] && \"$h\" -Qua 2>/dev/null | sed 's/^/aur /'; true"]
        onStarted: root.checking = true
        stdout: StdioCollector {
            onStreamFinished: {
                const out = [];
                for (const line of text.split("\n")) {
                    const m = line.match(/^(repo|aur) (\S+) (\S+) -> (\S+)/);
                    if (m) out.push({ name: m[2], from: m[3], to: m[4], aur: m[1] === "aur" });
                }
                const had = root.count;
                root.pending = out;
                root.checking = false;
                root.checkedAt = Qt.formatTime(new Date(), "HH:mm");
                if (out.length && !had) IslandEvents.show({ kind: "text", duration: 5000, glyph: "download", text: out.length + " updates available", detail: "Settings › Updates" });
            }
        }
    }
    function check() { if (!checker.running) checker.running = true; if (!restartCheck.running) restartCheck.running = true; }

    // Older code than installed is still running, the kernel or the NVIDIA driver: a restart finishes the update.
    property bool restartNeeded: false
    property var restartReasons: []
    Process {
        id: restartCheck
        command: ["python3", Quickshell.shellDir + "/scripts/restartneeded.py"]
        stdout: StdioCollector {
            onStreamFinished: {
                let r; try { r = JSON.parse(text); } catch (e) { return; }
                const was = root.restartNeeded;
                root.restartReasons = r.reasons || [];
                root.restartNeeded = !!r.restart;
                if (root.restartNeeded && !was && !root.running) IslandEvents.show({ kind: "text", duration: 8000, glyph: "rotate-cw", text: "Restart to finish the update", detail: root.restartReasons[0] || "", actions: [{ label: "Restart", run: () => Session.restart() }] });
            }
        }
    }

    // --- the run ----------------------------------------------------------------------------
    // What is happening: `phase` in words, `steps` one per package with its state (waiting, downloading,
    // installing, done, removed), `log` the last lines said, `failed` the reason when it stopped short.
    property bool running: false
    property string phase: ""
    property var steps: []
    property var log: []
    property string failed: ""
    function stepState(name, state) {
        const out = steps.slice();
        const i = out.findIndex(s => s.name === name);
        if (i >= 0) out[i] = Object.assign({}, out[i], { state: state }); else out.push({ name: name, state: state });
        steps = out;
    }
    function took(line) {
        const l = line.replace(/\x1b\[[0-9;]*m/g, "").trimEnd();
        if (!l) return;
        log = log.concat([l]).slice(-200);
        let m;
        if ((m = l.match(/^:: (Synchronizing|Starting|Retrieving|Checking|Processing|Running|Resolving|Looking)(.*)/))) phase = l.replace(/^:: /, "").replace(/\.\.\.$/, "");
        else if ((m = l.match(/^\s*(\S+?)-[^-\s]+-[^-\s]+\s+downloading/))) { stepState(m[1], "downloading"); phase = "Downloading " + m[1]; }
        else if ((m = l.match(/^\((\d+)\/(\d+)\) (upgrading|installing|reinstalling|downgrading) (\S+)/))) { markInstalled(); stepState(m[4], "installing"); phase = "Installing " + m[4] + "  ·  " + m[1] + " of " + m[2]; }
        else if ((m = l.match(/^\((\d+)\/(\d+)\) removing (\S+)/))) { markInstalled(); stepState(m[3], "removed"); phase = "Removing " + m[3]; }
        else if ((m = l.match(/^\((\d+)\/(\d+)\) (.+?)\.\.\.$/))) { markInstalled(); phase = m[3] + "  ·  " + m[1] + " of " + m[2]; }
        else if ((m = l.match(/^error: (.*)/i))) failed = m[1];
        else if (/^==> (Making|Building|Cloning|Downloading)/.test(l)) phase = l.replace(/^==> /, "");
    }
    // A package being installed is done once the next line moves on.
    function markInstalled() { if (steps.some(s => s.state === "installing")) steps = steps.map(s => s.state === "installing" ? Object.assign({}, s, { state: "done" }) : s); }
    Process {
        id: runner
        stdout: SplitParser { onRead: line => root.took(line) }
        stderr: SplitParser { onRead: line => root.took(line) }
        onStarted: { root.running = true; root.failed = ""; root.log = []; root.phase = "Asking for permission"; }
        onExited: (code) => {
            root.markInstalled();
            root.running = false;
            if (code === 126 || code === 127) { root.failed = "Permission was not given"; root.phase = "Stopped"; }
            else if (code !== 0) { if (!root.failed) root.failed = "Stopped with code " + code; root.phase = "Stopped"; }
            else root.phase = "Done";
            root.check();
            if (code === 0) done.start();
        }
    }
    // Said once the pending list and the restart check have caught up.
    Timer {
        id: done
        interval: 2500
        onTriggered: {
            if (root.restartNeeded) IslandEvents.show({ kind: "text", duration: 15000, glyph: "rotate-cw", text: "Updated; restart to finish", detail: root.restartReasons[0] || "", actions: [{ label: "Restart", run: () => Session.restart() }] });
            else IslandEvents.show({ kind: "text", duration: 5000, glyph: "check", text: "Updated" });
        }
    }
    // pacman behind pkexec, so the shell's auth dialog asks; the AUR helper builds as the user and hands pacman
    // to pkexec the same way. No prompts: every question takes its default.
    function run(args, aur) {
        if (runner.running) return;
        steps = pending.map(p => ({ name: p.name, state: "waiting" }));
        runner.command = aur && helper ? [helper].concat(args, ["--noconfirm", "--skipreview", "--noprogressbar", "--sudo", "pkexec"]) : ["pkexec", "pacman"].concat(args, ["--noconfirm", "--noprogressbar"]);
        runner.running = true;
    }
    function update() { run(["-Syu"], pending.some(p => p.aur)); }
    function updateOne(u) { steps = []; run(["-S", u.name], u.aur); }
    IpcHandler {
        target: "updates"
        function check(): void { root.check(); }
        function update(): void { root.update(); }
        function clean(): void { root.cleanCache(); }
        function orphans(): void { root.removeOrphans(); }
        function status(): string { return JSON.stringify({ pending: root.count, running: root.running, phase: root.phase, failed: root.failed, restart: root.restartNeeded, steps: root.steps, log: root.log.slice(-5) }); }
    }
    // Housekeeping through the same run: old package versions, and packages nothing depends on any more.
    function cleanCache() { if (runner.running) return; steps = []; runner.command = ["pkexec", "paccache", "-rk2"]; runner.running = true; }
    function removeOrphans() { if (runner.running) return; steps = []; runner.command = ["sh", "-c", "o=$(pacman -Qtdq); [ -n \"$o\" ] || { echo 'No orphans.'; exit 0; }; exec pkexec pacman -Rns --noconfirm $o"]; runner.running = true; }
}
