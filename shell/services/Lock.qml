pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Pam

// The session lock. Locks on request, on logind's Lock signal, and from idle; unlocks through PAM.
Singleton {
    id: root

    // The compositor's word: `locked` is the request and does not notify when it changes.
    readonly property bool locked: sessionLock.secure
    property string password: ""
    property bool failed: false
    property bool checking: pam.active
    property string message: ""
    property int attempts: 0
    property bool capsLock: false
    property var surfaceComponent: null

    // Caps Lock, from the compositor's device state, while locked.
    Process {
        id: caps
        command: ["sh", "-c", "hyprctl devices -j | python3 -c 'import json,sys; ks=json.load(sys.stdin)[\"keyboards\"]; print(any(k.get(\"capsLock\") for k in ks))'"]
        stdout: StdioCollector { onStreamFinished: root.capsLock = text.trim() === "True" }
    }
    Timer { interval: 1000; repeat: true; running: root.locked; triggeredOnStart: true; onTriggered: if (!caps.running) caps.running = true }

    WlSessionLock {
        id: sessionLock
        surface: root.surfaceComponent
        // A marker for as long as the session is locked: a shell that starts and finds it locks again at once,
        // taking over the compositor's dead lock (hyprland.lua allows the restore), so a shell that dies or
        // restarts while locked leaves the session locked rather than stuck.
        onSecureChanged: { marker.command = ["sh", "-c", secure ? "mkdir -p \"$1\" && : > \"$1/locked\"" : "rm -f \"$1/locked\"", "_", root.runDir]; marker.running = true; }
    }
    readonly property string runDir: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/isle"
    Process { id: marker }
    Process {
        id: markerCheck
        command: ["sh", "-c", "test -e \"$1/locked\" && echo locked", "_", root.runDir]
        running: true
        stdout: StdioCollector { onStreamFinished: if (text.indexOf("locked") >= 0) root.relock = true }
    }
    // The surface component arrives from shell.qml after the singletons; a relock waits for it.
    property bool relock: false
    onRelockChanged: if (relock && surfaceComponent) { lock(); relock = false; }
    onSurfaceComponentChanged: if (relock && surfaceComponent) { lock(); relock = false; }

    PamContext {
        id: pam
        config: "login"
        user: Quickshell.env("USER")
        onPamMessage: {
            if (responseRequired) respond(root.password);
            else if (messageIsError) root.message = message;
        }
        onCompleted: result => {
            root.password = "";
            if (result === PamResult.Success) { root.failed = false; root.message = ""; root.attempts = 0; sessionLock.locked = false; }
            else { root.failed = true; root.attempts++; root.message = result === PamResult.Failed ? "Wrong password" + (root.attempts > 1 ? " · " + root.attempts + " attempts" : "") : "Could not check the password"; }
        }
        onError: error => { root.failed = true; root.message = "PAM error"; }
    }

    function lock() { if (surfaceComponent) sessionLock.locked = true; }

    // Fingerprints: while locked and a finger is enrolled, fprintd verifies in the background beside the
    // password; a match unlocks, a miss says so and listens again.
    property bool fingerprint: false
    property string fingerMessage: ""
    Process {
        id: fingers
        command: ["fprintd-list", Quickshell.env("USER")]
        stdout: StdioCollector { onStreamFinished: { root.fingerprint = text.indexOf("- #") >= 0; if (root.fingerprint && root.locked) verifier.running = true; } }
    }
    Process {
        id: verifier
        command: ["fprintd-verify"]
        property string result: ""
        stdout: SplitParser {
            onRead: line => {
                const m = line.match(/Verify result: (\S+)/);
                if (m) verifier.result = m[1];
            }
        }
        onExited: {
            const r = verifier.result; verifier.result = "";
            if (!root.locked) return;
            if (r === "verify-match") { root.fingerMessage = ""; root.failed = false; root.message = ""; root.attempts = 0; sessionLock.locked = false; return; }
            if (r === "verify-no-match") { root.fingerMessage = "Not recognised"; retry.start(); return; }
            if (r === "verify-retry-scan" || r === "verify-swipe-too-short" || r === "verify-finger-not-centered" || r === "verify-remove-and-retry") { root.fingerMessage = "Try again"; retry.start(); return; }
            // The reader would not open (a finger still on it, or fprintd busy): a few more tries, then give up until the next lock.
            if (root.verifyFailures++ < 4) { retry.interval = 1500; retry.start(); return; }
            root.fingerprint = false;
        }
    }
    property int verifyFailures: 0
    Timer { id: retry; interval: 600; onTriggered: { retry.interval = 600; if (root.locked && root.fingerprint) verifier.running = true; } }
    onLockedChanged: {
        fingerMessage = "";
        verifyFailures = 0;
        if (locked) fingers.running = true;
        else if (verifier.running) verifier.signal(15);
    }
    function submit() {
        if (!sessionLock.locked || pam.active || password === "") return;
        failed = false;
        pam.start();
    }

    // logind: `loginctl lock-session`, and lid or idle policies elsewhere, arrive as a Lock signal.
    Process {
        command: ["gdbus", "monitor", "--system", "--dest", "org.freedesktop.login1"]
        running: true
        stdout: SplitParser {
            onRead: line => {
                if (line.indexOf("/org/freedesktop/login1/session/") < 0) return;
                if (line.indexOf(".Lock ()") >= 0) root.lock();
                else if (line.indexOf(".Unlock ()") >= 0 && sessionLock.locked) sessionLock.locked = false;
            }
        }
    }

    IpcHandler {
        target: "lock"
        function lock(): void { root.lock(); }
        function locked(): bool { return root.locked; }
    }
}
