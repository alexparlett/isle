pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Fingerprints through fprintd: the reader, the enrolled fingers, an enrollment in progress, and whether a
// finger stands in for the password at login and for sudo (PAM edits through pkexec).
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/scripts/fingerprint.py"
    property bool installed: false
    property string device: ""
    property var fingers: []
    property var fingerNames: []
    property bool login: false
    property bool sudo: false
    property string error: ""
    readonly property bool ready: installed && device !== ""

    Process {
        id: status
        command: ["python3", root.script, "status"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                let s; try { s = JSON.parse(text); } catch (e) { return; }
                root.installed = !!s.installed; root.device = s.device || ""; root.fingers = s.fingers || [];
                root.fingerNames = s.fingerNames || []; root.login = !!s.login; root.sudo = !!s.sudo;
            }
        }
    }
    function refresh() { status.running = true; }
    function label(f) { return String(f).replace("-finger", "").replace("-", " ").replace(/^\w/, c => c.toUpperCase()); }

    // Enrollment: fprintd asks for the same finger several times; `stage` counts what it has accepted.
    property bool enrolling: false
    property string enrollingFinger: ""
    property int stage: 0
    property string hint: ""
    Process {
        id: enroller
        stdout: SplitParser {
            onRead: line => {
                const l = line.trim();
                if (l.indexOf("stage ") === 0) { root.stage = parseInt(l.slice(6), 10) || root.stage + 1; root.hint = "Lift, then touch again"; }
                else if (l === "done") { root.hint = "Done"; }
                else if (l.indexOf("retry ") === 0) { root.hint = "Try again: " + l.slice(6); }
            }
        }
        onExited: code => {
            root.enrolling = false;
            if (code !== 0 && root.hint !== "Done") root.error = "Enrollment stopped" + (root.hint.indexOf("Try again") === 0 ? " · " + root.hint.slice(11) : "");
            root.refresh();
        }
    }
    function enroll(finger) {
        if (enroller.running) return;
        error = ""; stage = 0; hint = "Touch the reader"; enrollingFinger = finger; enrolling = true;
        enroller.command = ["python3", "-u", script, "enroll", finger];
        enroller.running = true;
    }
    function cancelEnroll() { if (enroller.running) enroller.signal(2); }

    Process {
        id: actor
        onExited: code => root.refresh()
        // Closing the auth dialog is a choice, not a failure.
        stderr: StdioCollector { onStreamFinished: { const l = text.trim() ? text.trim().split("\n").pop() : ""; root.error = l && l.indexOf("dismissed") < 0 ? l.replace(/^Error executing command as another user: /, "").slice(0, 120) : ""; } }
    }
    function remove(finger) { error = ""; actor.command = ["python3", script, "delete", finger]; actor.running = true; }
    // The PAM edits run as root; pkexec asks through the shell's own auth dialog.
    function setLogin(on) { error = ""; actor.command = ["pkexec", "python3", script, "set", "login", on ? "on" : "off"]; actor.running = true; }
    function setSudo(on) { error = ""; actor.command = ["pkexec", "python3", script, "set", "sudo", on ? "on" : "off"]; actor.running = true; }
    function install() { error = ""; actor.command = ["pkexec", "pacman", "-S", "--needed", "--noconfirm", "fprintd"]; actor.running = true; }
}
