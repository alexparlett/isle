pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The secret service (gnome-keyring): every stored secret, reveal, delete, add, lock.
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/scripts/secrets.py"

    // The daemon, if PAM did not start it at login. Unlocked by PAM, else it prompts on first use.
    Process { command: ["gnome-keyring-daemon", "--start", "--components=secrets"]; running: true }
    // [{ path, label, collection, attributes, locked, created, modified }]
    property var items: []
    property bool available: true
    property string error: ""
    property string revealed: ""
    property string revealedPath: ""

    Process {
        id: lister
        command: ["python3", root.script, "list"]
        stdout: StdioCollector {
            onStreamFinished: {
                const out = [];
                for (const line of text.split("\n")) { if (!line) continue; try { out.push(JSON.parse(line)); } catch (e) {} }
                out.sort((a, b) => (b.modified || 0) - (a.modified || 0));
                root.items = out;
                root.available = true;
            }
        }
        stderr: StdioCollector { onStreamFinished: { if (text.trim()) { root.error = text.trim().split("\n").pop(); root.available = false; } } }
    }
    function refresh() { error = ""; lister.running = true; }

    Process {
        id: revealer
        stdout: StdioCollector { onStreamFinished: { root.revealed = text; hide.restart(); } }
    }
    Timer { id: hide; interval: 15000; onTriggered: root.conceal() }
    function reveal(item) { revealedPath = item.path; revealer.command = ["python3", script, "reveal", item.path]; revealer.running = true; }
    function conceal() { revealed = ""; revealedPath = ""; }

    Process { id: actor; onExited: root.refresh() }
    function remove(item) { actor.command = ["python3", script, "delete", item.path]; actor.running = true; }
    function lock() { actor.command = ["python3", script, "lock"]; actor.running = true; }
    function unlock() { actor.command = ["python3", script, "unlock"]; actor.running = true; }

    Process { id: storer; onExited: root.refresh() }
    function store(label, attrs, secret) {
        const list = Array.isArray(attrs) ? attrs : [attrs];
        const quoted = list.map((_, i) => "\"$" + (i + 4) + "\"").join(" ");
        storer.command = ["sh", "-c", "printf '%s' \"$1\" | python3 \"$2\" store \"$3\" " + quoted + " -", "_", secret, script, label].concat(list);
        storer.running = true;
    }

    Process { id: copier }
    function copy(text) { copier.command = ["wl-copy", text]; copier.running = true; }

    // --- categories ----------------------------------------------------------------
    readonly property var categories: [["all", "All"], ["logins", "Logins"], ["wifi", "Wi-Fi"], ["browser", "Browser"], ["ssh", "SSH"], ["apps", "Apps"], ["other", "Other"]]
    function categoryOf(item) {
        const a = item.attributes || {}, schema = a["xdg:schema"] || "";
        if (a.category) return a.category;
        if (schema.indexOf("NetworkManager") >= 0 || a["connection.type"] === "802-11-wireless") return "wifi";
        if (/chrom|vivaldi|brave|mozilla|firefox|libsecret_os_crypt/i.test(schema + " " + (a.application || ""))) return "browser";
        if (schema.indexOf("ssh") >= 0 || (a.unique || "").indexOf("ssh") === 0) return "ssh";
        if (schema === "org.gnome.keyring.NetworkPassword" || a.user || a.username) return "logins";
        if (schema) return "apps";
        return "other";
    }

    // --- generator -----------------------------------------------------------------
    property string generated: ""
    Process {
        id: generator
        stdout: StdioCollector { onStreamFinished: root.generated = text.trim() }
    }
    function generate(length, symbols) {
        const alphabet = "string.ascii_letters + string.digits" + (symbols ? " + '!@#$%^&*-_=+?'" : "");
        generator.command = ["python3", "-c", "import secrets, string; print(''.join(secrets.choice(" + alphabet + ") for _ in range(" + Math.round(length) + ")))"];
        generator.running = true;
    }
}
