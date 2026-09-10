pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Gamepad presses from the evdev helper, read only while a surface asks for them.
Singleton {
    id: root
    property bool active: false
    signal pressed(string button)

    Process {
        command: ["python3", Quickshell.shellDir + "/scripts/gamepad.py"]
        running: root.active
        stdout: SplitParser { onRead: line => { const b = line.trim(); if (b) root.pressed(b); } }
    }
}
