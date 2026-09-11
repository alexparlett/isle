import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// One password manager behind Vault, driven by a script that answers status, list, field, unlock and
// logout (the contract in docs/ARCHITECTURE.md): the account's state, the vaults' items by title, and a
// field of one item when asked for, to the clipboard or shown for a moment. Sign-in is the manager's own
// flow, in a terminal window.
QtObject {
    id: root
    required property string providerId
    required property string name
    property string note: ""
    required property string script
    property string signInCommand: ""
    property bool user: false

    property bool installed: false
    property bool loggedIn: false
    property bool locked: false
    property bool hasLock: false
    property string email: ""
    property string error: ""
    property bool busy: false
    // [{ id, shareId, vault, title, type }], by title.
    property var items: []
    readonly property bool ready: installed && loggedIn && !locked

    property Process status: Process {
        command: ["python3", root.script, "status"]
        stdout: StdioCollector {
            onStreamFinished: {
                let r; try { r = JSON.parse(text); } catch (e) { root.busy = false; root.error = "The provider's status was not JSON"; return; }
                root.installed = !!r.installed; root.loggedIn = !!r.loggedIn; root.locked = !!r.locked; root.hasLock = !!r.hasLock;
                root.email = r.email || ""; root.error = r.error || "";
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
                if (r.error === "locked") { root.locked = true; root.items = []; return; }
                if (r.error === "signed out") { root.loggedIn = false; root.items = []; return; }
                root.items = r.items || [];
                if (r.error) root.error = r.error;
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
    property string revealedField: ""
    property Process fielder: Process {
        stdout: StdioCollector {
            onStreamFinished: {
                const v = text;
                if (root.want === "copy") { root.copier.command = ["wl-copy", "--", v]; root.copier.running = true; IslandEvents.show({ kind: "text", glyph: "check", text: root.wantLabel + " copied", duration: 1800 }); }
                else if (root.want === "reveal") { root.revealed = v; root.revealedId = root.wantItem ? root.wantItem.id : ""; }
                root.want = "";
            }
        }
        stderr: StdioCollector { onStreamFinished: { if (text.trim()) { root.error = text.trim().split("\n").pop(); if (/lock/i.test(text)) root.locked = true; } } }
    }
    property Process copier: Process {}
    function fetch(item, field, purpose) {
        if (fielder.running || !item) return;
        want = purpose; wantItem = item; revealedField = field;
        wantLabel = ({ password: "Password", username: "Username", email: "Email", totp: "Code", urls: "Address", note: "Note" })[field] || field;
        fielder.command = ["python3", script, "field", item.shareId, item.id, field];
        fielder.running = true;
    }
    function copy(item, field) { fetch(item, field, "copy"); }
    function reveal(item, field) { fetch(item, field || "password", "reveal"); }
    function conceal() { revealed = ""; revealedId = ""; revealedField = ""; }

    function signIn() { if (signInCommand) Compositor.exec((Prefs.p.terminal || "kitty") + " -e sh -c " + JSON.stringify(signInCommand + "; echo; echo Done, close this window.; sleep 3")); }
    property Process actor: Process { onExited: root.refresh() }
    function signOut() { actor.command = ["python3", script, "logout"]; actor.running = true; }
    property Process unlocker: Process {
        stdinEnabled: true
        onExited: (code) => { root.refresh(); if (code !== 0) root.error = "The lock code was not accepted"; }
    }
    function unlock(code) {
        unlocker.command = ["python3", script, "unlock"];
        unlocker.running = true;
        unlocker.write(code + "\n");
        unlocker.stdinEnabled = false;
    }
}
