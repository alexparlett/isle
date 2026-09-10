pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Gamepad presses from the evdev helper, read only while a surface asks for them (`listeners`).
Singleton {
    id: root
    property int listeners: 0
    readonly property bool active: listeners > 0
    signal pressed(string button)
    // "sony", "xbox", "nintendo" or "generic" once a pad is seen; the keyboard's legend draws its glyphs.
    property string kind: ""
    signal connected(string kind)

    Process {
        command: ["python3", Quickshell.shellDir + "/scripts/gamepad.py"]
        running: root.active
        stdout: SplitParser { onRead: line => { const b = line.trim(); if (b.indexOf("pad:") === 0) { root.kind = b.slice(4); root.connected(root.kind); } else if (b) root.pressed(b); } }
    }
}
