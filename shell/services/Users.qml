pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Accounts on this machine through AccountsService: who they are, admin or not, their picture; polkit guards the rest.
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/scripts/users.py"
    // [{ path, name, realName, admin, icon, locked, autoLogin, uid, me }]
    property var users: []
    readonly property var me: users.find(u => u.me) || null
    readonly property var others: users.filter(u => !u.me)
    property string error: ""

    Process {
        id: lister
        command: ["python3", root.script, "list"]
        stdout: StdioCollector { onStreamFinished: { try { root.users = JSON.parse(text); } catch (e) {} } }
    }
    function refresh() { lister.running = true; }

    Process {
        id: actor
        onExited: code => root.refresh()
        stderr: StdioCollector { onStreamFinished: { const l = text.trim() ? text.trim().split("\n").pop() : ""; root.error = l && l.indexOf("dismissed") < 0 ? l.replace(/^.*: /, "").slice(0, 120) : ""; } }
    }
    function run(args) { error = ""; actor.command = ["python3", script].concat(args); actor.running = true; }
    function setName(u, name) { run(["set-name", u.name, name]); }
    function setIcon(u, path) { run(["set-icon", u.name, path]); }
    function setAdmin(u, on) { run(["set-admin", u.name, on ? "1" : "0"]); }
    function add(name, realName, admin) { run(["add", name, realName, admin ? "1" : "0"]); }
    function remove(u, files) { run(["remove", u.name, files ? "1" : "0"]); }
    function setPassword(u, password) {
        error = "";
        actor.command = ["sh", "-c", "printf '%s' \"$1\" | python3 \"$2\" set-password \"$3\" -", "_", password, script, u.name];
        actor.running = true;
    }
    // Your own password goes through passwd, which PAM checks against the old one.
    function changeOwnPassword() { Compositor.exec("kitty --class isle-windows --title Password -e passwd"); }
    function pickIcon(u) {
        Compositor.exec("sh -c 'f=$(zenity --file-selection --file-filter=\"Images | *.png *.jpg *.jpeg\" 2>/dev/null); [ -n \"$f\" ] && python3 " + JSON.stringify(script) + " set-icon " + JSON.stringify(u.name) + " \"$f\" && qs -p " + Quickshell.shellDir + " ipc call users refresh'");
    }
    function initials(u) { const n = (u.realName || u.name).trim().split(/\s+/); return (n[0].charAt(0) + (n[1] ? n[1].charAt(0) : "")).toUpperCase(); }

    // Automatic login is greetd's initial session; the file is world-readable, the change goes through polkit.
    property string autoLoginUser: ""
    Process {
        id: greetdConf
        command: ["sh", "-c", "awk '/^\\[initial_session\\]/{s=1; next} /^\\[/{s=0} s && /^user *=/{gsub(/[\" ]/, \"\", $0); sub(/^user=/, \"\", $0); print}' /etc/greetd/config.toml 2>/dev/null"]
        running: true
        stdout: StdioCollector { onStreamFinished: root.autoLoginUser = text.trim() }
    }
    Process { id: autologin; onExited: greetdConf.running = true }
    function setAutoLogin(on) {
        autologin.command = ["pkexec", Quickshell.shellDir + "/scripts/autologin.sh"].concat(on && me ? ["on", me.name] : ["off"]);
        autologin.running = true;
    }
    IpcHandler { target: "users"; function refresh(): void { root.refresh(); greetdConf.running = true; } }
}
