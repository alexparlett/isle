pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Pending package updates: pacman through checkupdates, the AUR through paru or yay. Checked hourly and on demand;
// updating runs the helper in a terminal so it can ask and show its work.
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
                if (root.restartNeeded && !was) IslandEvents.show({ kind: "text", duration: 8000, glyph: "rotate-cw", text: "Restart to finish the update", detail: root.restartReasons[0] || "" });
            }
        }
    }

    // The terminal closes when the helper finishes; the list refreshes then.
    Process {
        id: updater
        onExited: root.check()
    }
    function update() {
        const cmd = helper ? helper + " -Syu" : "sudo pacman -Syu";
        updater.command = ["kitty", "--class", "isle-windows", "--title", "Updates", "-e", "sh", "-c", cmd + "; echo; echo Done. Press Enter.; read x"];
        updater.running = true;
    }
    function updateOne(u) {
        updater.command = ["kitty", "--class", "isle-windows", "--title", "Updates", "-e", "sh", "-c", (u.aur && helper ? helper + " -S " : "sudo pacman -S ") + u.name + "; echo; echo Done. Press Enter.; read x"];
        updater.running = true;
    }
}
