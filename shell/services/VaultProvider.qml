import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// A password manager behind Vault: a Provider whose list is items by title, never a secret, and which
// fetches one field of one item when asked, to the clipboard or shown for a moment.
Provider {
    id: root
    listKey: "items"
    property bool inKeychain: true
    // [{ id, shareId, vault, title, type }], by title.
    readonly property var items: listed

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
}
