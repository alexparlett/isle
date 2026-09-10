pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Services.Pam

// The session lock. Locks on request, on logind's Lock signal, and from idle; unlocks through PAM.
Singleton {
    id: root

    readonly property bool locked: sessionLock.locked
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
    }

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
    }
}
