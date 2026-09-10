import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "About"
    subtitle: "Isle, a desktop shell for Hyprland in Quickshell."

    property string versions: ""
    Process {
        command: ["sh", "-c", "printf 'Hyprland %s\\nQuickshell %s\\nKernel %s\\n' \"$(hyprctl version -j 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin).get(\"tag\",\"?\"))' 2>/dev/null)\" \"$(qs --version 2>/dev/null | head -1)\" \"$(uname -r)\""]
        running: true
        stdout: StdioCollector { onStreamFinished: versions = text.trim() }
    }

    SettingsGroup {
        heading: "Versions"
        Repeater {
            model: versions.split("\n").filter(l => l)
            SettingsRow {
                required property string modelData
                label: modelData.split(" ")[0]
                description: modelData.split(" ").slice(1).join(" ")
            }
        }
    }

    SettingsGroup {
        heading: "Paths"
        SettingsRow { label: "Shell"; description: Quickshell.shellDir }
        SettingsRow { label: "Preferences"; description: Prefs.dir + "/prefs.json" }
        SettingsRow { label: "Widgets"; description: Widgets.userDir }
    }
}
