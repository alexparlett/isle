import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// One password manager behind Vault, driven by a script that answers status, list, field and action (the
// contract in docs/ARCHITECTURE.md). The provider says its state in words and names its own actions; the
// shell renders them and runs them, and knows nothing of signing in or locks itself.
QtObject {
    id: root
    required property string providerId
    required property string name
    property string note: ""
    required property string script
    property bool user: false

    property bool installed: false
    property bool ready: false
    property string state: ""
    // [{ id, label, input, terminal, primary }]: what the provider offers in its current state.
    property var actions: []
    property string error: ""
    property bool busy: false
    // [{ id, shareId, vault, title, type }], by title.
    property var items: []

    property Process status: Process {
        command: ["python3", root.script, "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                let r; try { r = JSON.parse(text); } catch (e) { root.busy = false; root.error = "The provider's status was not JSON"; return; }
                root.installed = !!r.installed; root.ready = !!r.ready; root.state = r.state || ""; root.actions = r.actions || []; root.error = r.error || "";
                if (root.ready) root.lister.running = true; else { root.items = []; root.busy = false; }
            }
        }
    }
    property Process lister: Process {
        command: ["python3", root.script, "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                root.busy = false;
                let r; try { r = JSON.parse(text); } catch (e) { root.error = "The provider's list was not JSON"; return; }
                root.items = r.items || [];
                if (r.error) { root.error = r.error; if (!root.items.length) root.status.running = true; }
            }
        }
    }
    function refresh() { if (status.running) return; busy = true; error = ""; status.running = true; }
    property Timer poll: Timer { interval: 10 * 60 * 1000; running: root.ready; repeat: true; onTriggered: root.refresh() }

    // What a fetched field is for: "copy" puts it on the clipboard, "reveal" shows it against the item.
    property string want: ""
    property var wantItem: null
    property string wantLabel: ""
    property string revealed: ""
    property string revealedId: ""
    property Process fielder: Process {
        stdout: StdioCollector {
            onStreamFinished: {
                const v = text;
                if (root.want === "copy") { root.copier.command = ["wl-copy", "--", v]; root.copier.running = true; IslandEvents.show({ kind: "text", glyph: "check", text: root.wantLabel + " copied", duration: 1800 }); }
                else if (root.want === "reveal") { root.revealed = v; root.revealedId = root.wantItem ? root.wantItem.id : ""; }
                root.want = "";
            }
        }
        stderr: StdioCollector { onStreamFinished: { if (text.trim()) { root.error = text.trim().split("\n").pop(); root.refresh(); } } }
    }
    property Process copier: Process {}
    function fetch(item, field, purpose) {
        if (fielder.running || !item) return;
        want = purpose; wantItem = item;
        wantLabel = ({ password: "Password", username: "Username", email: "Email", totp: "Code", urls: "Address", note: "Note" })[field] || field;
        fielder.command = ["python3", script, "field", item.shareId, item.id, field];
        fielder.running = true;
    }
    function copy(item, field) { fetch(item, field, "copy"); }
    function reveal(item, field) { fetch(item, field || "password", "reveal"); }
    function conceal() { revealed = ""; revealedId = ""; }

    // An action of the provider's: in a terminal window when it says so, else run with its input on stdin.
    property Process actor: Process {
        stdinEnabled: true
        stderr: StdioCollector { onStreamFinished: if (text.trim()) root.error = text.trim().split("\n").pop() }
        onExited: root.refresh()
    }
    function act(id, input) {
        const a = actions.find(x => x.id === id); if (!a) return;
        if (a.terminal) { Compositor.exec((Prefs.p.terminal || "kitty") + " -e sh -c " + JSON.stringify("python3 " + JSON.stringify(script) + " action " + id + "; echo; echo Done, close this window.; sleep 3")); return; }
        if (actor.running) return;
        error = "";
        actor.command = ["python3", script, "action", id];
        actor.running = true;
        if (a.input) actor.write((input || "") + "\n");
        actor.stdinEnabled = false;
    }
}
