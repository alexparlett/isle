pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// SSH keys in ~/.ssh and what the session's agent holds. Passphrases go to ssh-add through a one-shot file, never argv.
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/scripts/sshkeys.py"
    readonly property string sock: Quickshell.env("SSH_AUTH_SOCK") || (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/gcr/ssh"
    readonly property string passDir: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/isle-askpass"
    // [{ path, name, type, bits, fingerprint, comment, pubkey, hasPrivate, locked, loaded }]
    property var keys: []
    property var agent: ({ sock: "", alive: false, loaded: 0 })
    property string error: ""

    Process {
        id: lister
        command: ["python3", root.script, "list"]
        stdout: StdioCollector { onStreamFinished: { try { root.keys = JSON.parse(text); } catch (e) {} } }
    }
    Process {
        id: agentProbe
        command: ["python3", root.script, "agent"]
        stdout: StdioCollector { onStreamFinished: { try { root.agent = JSON.parse(text); } catch (e) {} } }
    }
    function refresh() { lister.running = true; agentProbe.running = true; }

    Process {
        id: actor
        onExited: code => { if (code !== 0 && root.error === "") root.error = "That did not work"; root.refresh(); }
        stderr: StdioCollector { onStreamFinished: root.error = text.trim().split("\n").pop() || "" }
    }
    // The passphrase file is made by a shell with a 0600 umask; the script deletes it after one read.
    function runWith(args, passphrase) {
        if (!passphrase) { actor.command = args; actor.running = true; return; }
        const file = passDir + "/" + Date.now();
        actor.command = ["sh", "-c", "umask 077; mkdir -p \"$1\" && printf '%s' \"$2\" > \"$3\" && shift 3 && exec \"$@\"", "_", passDir, passphrase, file].concat(args).concat([file]);
        actor.running = true;
    }
    function load(key, passphrase) { error = ""; runWith(["python3", script, "load", key.path], passphrase); }
    function unload(key) { error = ""; actor.command = ["python3", script, "unload", key.path]; actor.running = true; }
    function generate(name, type, comment, passphrase) { error = ""; runWith(["python3", script, "generate", name, type, comment], passphrase); }

    Process { id: copier }
    function copyPublic(key) { copier.command = ["wl-copy", key.pubkey]; copier.running = true; IslandEvents.show({ kind: "text", duration: 2500, glyph: "terminal", text: "Public key copied", detail: key.name }); }
}
