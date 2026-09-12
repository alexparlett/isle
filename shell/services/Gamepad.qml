pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// Gamepad presses from the evdev helper, read only while a surface asks for them (`listeners`); and every
// pad's whole state, read only while Settings › Controllers watches (`watchers`).
Singleton {
    id: root
    property int listeners: 0
    readonly property bool active: listeners > 0
    signal pressed(string button)
    // "sony", "xbox", "nintendo" or "generic" once a pad is seen; the keyboard's legend draws its glyphs.
    property string kind: ""
    signal connected(string kind)
    readonly property string script: Quickshell.shellDir + "/scripts/gamepad.py"
    readonly property real deadZone: Prefs.p.padDeadZone || 0.5

    Process {
        command: ["python3", root.script, "--deadzone", String(root.deadZone)]
        running: root.active
        stdout: SplitParser { onRead: line => { const b = line.trim(); if (b.indexOf("pad:") === 0) { root.kind = b.slice(4); root.connected(root.kind); } else if (b) root.pressed(b); } }
    }

    property int watchers: 0
    // [{ id, name, kind, bus, buttons, axes: {x, y, rx, ry}, triggers: {lt, rt}, hat: {x, y} }]
    property var pads: []
    Process {
        command: ["python3", root.script, "--live"]
        running: root.watchers > 0
        stdout: SplitParser { onRead: line => { try { root.pads = JSON.parse(line).pads || []; } catch (e) {} } }
        onExited: root.pads = []
    }
    IpcHandler {
        target: "gamepad"
        function status(): string { return JSON.stringify({ listeners: root.listeners, watchers: root.watchers, kind: root.kind, pads: root.pads }); }
    }
}
