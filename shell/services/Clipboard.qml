pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Keeps the clipboard history: wl-paste feeds cliphist for as long as the shell runs.
Singleton {
    Process { command: ["sh", "-c", "rm -rf \"${XDG_RUNTIME_DIR:-/tmp}/isle-clip\""]; running: true }
    Process {
        command: ["sh", "-c", "wl-paste --type text --watch cliphist store & wl-paste --type image --watch cliphist store & wait"]
        running: true
    }
}
