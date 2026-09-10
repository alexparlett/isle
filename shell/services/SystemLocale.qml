pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Language and formats through localectl. A change applies to sessions started after it.
Singleton {
    id: root

    property string lang: ""
    property string time: ""
    property string numeric: ""
    property string monetary: ""
    // The keyboard layout localectl holds for X11 and Wayland, which the compositor does not read itself.
    property string x11Layout: ""
    property string x11Variant: ""
    property var available: []
    // XKB layout codes, for the keyboard's layout choice.
    property var layouts: []
    Process {
        command: ["localectl", "list-x11-keymap-layouts"]
        running: true
        stdout: StdioCollector { onStreamFinished: root.layouts = text.trim().split("\n").filter(l => l) }
    }
    property string error: ""

    Process {
        id: status
        command: ["localectl", "status"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                // "System Locale: LANG=en_GB.UTF-8" then one indented "LC_x=..." line per extra variable.
                const parts = [];
                for (const line of text.split("\n")) {
                    const m = line.match(/(?:System Locale:\s*)?([A-Z_]+=\S+)/);
                    if (m && (line.indexOf("System Locale") >= 0 || /^\s+LC_/.test(line) || /^\s+LANG/.test(line))) parts.push(m[1]);
                }
                const get = k => { const p = parts.find(x => x.indexOf(k + "=") === 0); return p ? p.slice(k.length + 1) : ""; };
                root.lang = get("LANG"); root.time = get("LC_TIME"); root.numeric = get("LC_NUMERIC"); root.monetary = get("LC_MONETARY");
                const x = (name) => { const m = text.match(new RegExp("X11 " + name + ":\\s*(\\S+)")); return m && m[1] !== "(unset)" ? m[1] : ""; };
                root.x11Layout = x("Layout"); root.x11Variant = x("Variant");
            }
        }
    }
    Process {
        command: ["python3", Quickshell.shellDir + "/scripts/locales.py"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { root.named = JSON.parse(text); } catch (e) { root.named = []; } root.available = root.named.map(l => l.code); } }
    }
    // [{ code, language, territory }] as glibc names them.
    property var named: []
    function refresh() { status.running = true; }

    Process { id: actor; onExited: root.refresh(); stderr: StdioCollector { onStreamFinished: root.error = text.trim().split("\n").pop() || "" } }
    function apply(lang, time, numeric, monetary) {
        error = "";
        const args = ["localectl", "set-locale", "LANG=" + lang];
        if (time) args.push("LC_TIME=" + time);
        if (numeric) args.push("LC_NUMERIC=" + numeric);
        if (monetary) args.push("LC_MONETARY=" + monetary);
        actor.command = args; actor.running = true;
    }
    // "English (UK)", as language pickers put it.
    function label(l) {
        const n = named.find(x => x.code === l);
        if (!n) return l.split(".")[0];
        return n.territory ? n.language + " (" + n.territory + ")" : n.language;
    }
}
