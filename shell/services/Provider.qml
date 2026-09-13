import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// A provider behind Vault or Vpn: a script that answers status, list and action (the contract in
// docs/ARCHITECTURE.md, "Providers"). It says its state in words and names its own actions; the shell
// renders and runs them and knows nothing of the manager behind.
QtObject {
    id: root
    required property string providerId
    required property string name
    property string note: ""
    required property string script
    property bool user: false
    // The key of `list`'s answer: items for a vault, choices for a VPN.
    property string listKey: "items"

    property bool installed: false
    property bool ready: false
    property string state: ""
    // [{ id, label, input, terminal, primary, on, options, value }]: what the provider offers in its state. `input`
    // is the placeholder of a value sent on stdin; `terminal` runs it in a terminal window; `on` draws a
    // toggle; `options` with `value` draw a choice whose pick is sent on stdin.
    property var actions: []
    property var status: ({})
    property var listed: []
    property string error: ""
    property bool busy: false

    property Process statusProc: Process {
        command: ["python3", root.script, "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                let r; try { r = JSON.parse(text); } catch (e) { root.busy = false; root.error = "The provider's status was not JSON"; return; }
                root.status = r;
                root.installed = !!r.installed; root.ready = !!r.ready; root.state = r.state || ""; root.actions = r.actions || []; root.error = r.error || "";
                if (root.ready) root.listProc.running = true; else { root.listed = []; root.busy = false; }
            }
        }
    }
    property Process listProc: Process {
        command: ["python3", root.script, "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                root.busy = false;
                let r; try { r = JSON.parse(text); } catch (e) { root.error = "The provider's list was not JSON"; return; }
                root.listed = r[root.listKey] || [];
                if (r.error) { root.error = r.error; if (!root.listed.length) root.statusProc.running = true; }
            }
        }
    }
    function refresh() { if (statusProc.running) return; busy = true; error = ""; statusProc.running = true; }
    property Timer poll: Timer { interval: 10 * 60 * 1000; running: root.ready; repeat: true; onTriggered: root.refresh() }

    // What the last action said on stdout, for the page to show ("Rolled back to 42; the next boot uses it").
    property string said: ""
    // An action of the provider's, or one it named on an item of its list: in a terminal window when it says
    // so, else run with its input on stdin (an item's id, a typed value, a choice).
    // The input goes on stdin once the process is up and stdin is closed behind it: written earlier it is
    // lost, and a script left waiting on stdin would hold the actor for good.
    property string pendingInput: ""
    property Process actor: Process {
        stdinEnabled: true
        stdout: StdioCollector { onStreamFinished: root.said = text.trim() }
        stderr: StdioCollector { onStreamFinished: if (text.trim()) root.error = text.trim().split("\n").pop() }
        onStarted: { root.busy = true; root.said = ""; actor.write(root.pendingInput + "\n"); root.pendingInput = ""; actor.stdinEnabled = false; }
        onExited: { actor.stdinEnabled = true; root.refresh(); }
    }
    function act(id, input) {
        const a = actions.find(x => x.id === id) || { id: id };
        if (a.terminal) { Compositor.exec((Prefs.p.terminal || "kitty") + " -e sh -c " + JSON.stringify("python3 " + JSON.stringify(script) + " action " + id + "; echo; echo Done, close this window.; sleep 3")); return; }
        if (actor.running) return;
        error = "";
        pendingInput = input || "";
        actor.command = ["python3", script, "action", id];
        actor.running = true;
    }
}
