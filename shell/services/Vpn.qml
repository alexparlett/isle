pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// VPN: Proton VPN through its command line when it is installed, and the VPN connections NetworkManager knows about.
Singleton {
    id: root

    // NetworkManager: [{ name, type, active }]
    property var connections: []
    readonly property var activeConnection: connections.find(c => c.active) || null

    // Proton VPN. The sign-in happens in a terminal, so the password never passes through the shell.
    property bool protonInstalled: false
    property bool protonSignedIn: false
    property string protonAccount: ""
    // { connected, server, location, load, protocol }
    property var proton: ({ connected: false, server: "", location: "", load: "", protocol: "" })
    // [{ name, code }], once signed in.
    property var countries: []
    // Setting name -> value, as `protonvpn config list` prints them.
    property var protonSettings: ({})
    property bool busy: false
    property string error: ""

    // Whatever is up: Proton's server, or the NetworkManager connection.
    readonly property var active: proton.connected ? { name: proton.server + (proton.location ? " · " + proton.location : ""), type: "proton" } : activeConnection
    readonly property bool available: protonInstalled || connections.length > 0
    // Something can actually be brought up: Proton with an account signed in, or a NetworkManager connection.
    readonly property bool ready: protonSignedIn || connections.length > 0
    property int listeners: 0

    readonly property var env: ({ PYTHONWARNINGS: "ignore" })

    // --- NetworkManager ----------------------------------------------------------------

    Process {
        id: lister
        command: ["nmcli", "-t", "-f", "NAME,TYPE,ACTIVE", "connection", "show"]
        stdout: StdioCollector {
            onStreamFinished: {
                const out = [];
                for (const line of text.trim().split("\n")) {
                    const f = line.split(":");
                    if (f.length < 3) continue;
                    if (["vpn", "wireguard", "tun"].indexOf(f[1]) < 0) continue;
                    out.push({ name: f[0], type: f[1], active: f[2] === "yes" });
                }
                root.connections = out;
            }
        }
    }
    function refresh() { lister.running = true; }
    // NetworkManager announces every change on stdout; a refresh follows.
    Process {
        command: ["nmcli", "monitor"]
        running: true
        stdout: SplitParser { onRead: line => debounce.restart() }
    }
    Timer { id: debounce; interval: 500; onTriggered: root.refresh() }

    Process { id: toggler; onExited: root.refresh() }
    function up(c) { toggler.command = ["nmcli", "connection", "up", c.name]; toggler.running = true; }
    function down(c) { toggler.command = ["nmcli", "connection", "down", c.name]; toggler.running = true; }

    // --- Proton VPN ---------------------------------------------------------------------

    Process {
        id: which
        command: ["sh", "-c", "command -v protonvpn"]
        running: true
        onExited: code => { root.protonInstalled = code === 0; if (root.protonInstalled) { root.refreshInfo(); root.refreshStatus(); } }
    }

    Process {
        id: status
        command: ["timeout", "30", "protonvpn", "status"]
        environment: root.env
        stdout: StdioCollector {
            onStreamFinished: {
                const p = { connected: false, server: "", location: "", load: "", protocol: "" };
                for (const line of text.split("\n")) {
                    const m = line.match(/^(\w+): (.*)$/);
                    if (!m) continue;
                    if (m[1] === "Status") p.connected = m[2] === "Connected";
                    else if (m[1] === "Server") { const at = m[2].indexOf(" in "); p.server = at < 0 ? m[2] : m[2].slice(0, at); p.location = at < 0 ? "" : m[2].slice(at + 4); }
                    else if (m[1] === "Load") p.load = m[2];
                    else if (m[1] === "Protocol") p.protocol = m[2];
                }
                root.proton = p;
            }
        }
    }
    function refreshStatus() { if (protonInstalled && !status.running) status.running = true; }

    Process {
        id: info
        command: ["timeout", "30", "protonvpn", "info"]
        environment: root.env
        stdout: StdioCollector {
            onStreamFinished: {
                // Signed out, the CLI prints the account as 'None'.
                const m = text.match(/Account: '(.*)'/);
                const signedIn = m !== null && m[1] !== "" && m[1] !== "None";
                root.protonSignedIn = signedIn;
                root.protonAccount = signedIn ? m[1] : "";
                if (signedIn) { countriesProc.running = true; settingsProc.running = true; }
                else { root.countries = []; root.protonSettings = {}; }
            }
        }
    }
    function refreshInfo() { if (protonInstalled && !info.running) info.running = true; }

    Process {
        id: countriesProc
        command: ["timeout", "60", "protonvpn", "countries", "list"]
        environment: root.env
        stdout: StdioCollector {
            onStreamFinished: {
                const out = [];
                for (const line of text.split("\n")) {
                    const m = line.match(/^(.+?)\s{2,}([A-Z]{2})\s*$/);
                    if (m && m[1] !== "Country") out.push({ name: m[1].trim(), code: m[2] });
                }
                if (out.length) root.countries = out;
            }
        }
    }

    Process {
        id: settingsProc
        command: ["timeout", "30", "protonvpn", "config", "list"]
        environment: root.env
        stdout: StdioCollector {
            onStreamFinished: {
                const out = {};
                for (const line of text.split("\n")) {
                    const m = line.match(/^([a-z0-9-]+)\s{2,}(.+?)\s*$/);
                    if (m && m[1] !== "Setting") out[m[1]] = m[2];
                }
                root.protonSettings = out;
            }
        }
    }

    // The link table changes when a tunnel comes or goes, whoever brought it.
    Process {
        command: ["ip", "-o", "monitor", "link"]
        running: true
        stdout: SplitParser { onRead: line => linkDebounce.restart() }
    }
    Timer { id: linkDebounce; interval: 1200; onTriggered: root.refreshStatus() }

    Process {
        id: runner
        environment: root.env
        stderr: StdioCollector { id: runnerErr }
        onStarted: { root.busy = true; root.error = ""; }
        onExited: code => {
            root.busy = false;
            if (code === 124) root.error = "Proton VPN did not answer";
            else if (code !== 0) {
                const lines = runnerErr.text.trim().split("\n").filter(l => l && !/eventlet|deprecat|migration|monkey|framework/i.test(l));
                root.error = (lines[lines.length - 1] || "Proton VPN failed").replace(/^Error: /, "");
            }
            root.refreshStatus();
            root.refreshInfo();
        }
    }
    function run(args) { if (runner.running) return; runner.command = ["timeout", "90", "protonvpn"].concat(args); runner.running = true; }
    // country: a two-letter code, or "" for the fastest server anywhere.
    function connect(country) { run(country ? ["connect", "--country", country] : ["connect"]); }
    function disconnect() { run(["disconnect"]); }
    function setSetting(name, value) { run(["config", "set", name, value]); }
    function signOut() { run(["signout"]); }

    // Sign-in: the CLI, given no terminal, reads the password and then a 2FA code from stdin, so the shell's
    // fields feed it. Its prompts come on stderr without a newline, hence the split on the colon.
    property bool signingIn: false
    property bool needsCode: false
    property string pendingPassword: ""
    property string signinErr: ""
    Process {
        id: signin
        environment: root.env
        stdinEnabled: true
        stderr: SplitParser {
            splitMarker: ":"
            onRead: chunk => {
                root.signinErr += chunk + ":";
                if (/2FA Token/i.test(chunk)) root.needsCode = true;
            }
        }
        onStarted: { signin.write(root.pendingPassword + "\n"); root.pendingPassword = ""; }
        onExited: code => {
            root.signingIn = false;
            root.needsCode = false;
            if (code === 124) root.error = "Proton VPN did not answer";
            else if (code !== 0) {
                const m = root.signinErr.match(/Error: (.*?)(\.|$)/);
                root.error = m ? m[1].replace(/:$/, "") : "Sign-in failed";
            }
            root.signinErr = "";
            root.refreshInfo();
            root.refreshStatus();
        }
    }
    function signIn(username, password) {
        const u = (username || "").trim();
        if (!u || !password || signin.running) return;
        root.error = "";
        root.signinErr = "";
        root.pendingPassword = password;
        root.signingIn = true;
        signin.command = ["timeout", "180", "setsid", "protonvpn", "signin", u];
        signin.running = true;
    }
    function submitCode(code) {
        if (!signin.running) return;
        signin.write((code || "").trim() + "\n");
        root.needsCode = false;
    }

    // The panel tile: Proton when signed in, else the first NetworkManager connection.
    function toggle() {
        if (proton.connected) disconnect();
        else if (activeConnection) down(activeConnection);
        else if (protonSignedIn) connect("");
        else if (connections.length) up(connections[0]);
    }

    Component.onCompleted: refresh()
}
