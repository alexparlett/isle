import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Apps"
    subtitle: "Which app opens what, and what starts with the session."

    Component.onCompleted: { Defaults.refresh(); Startup.refresh(); }
    property bool addingStartup: false

    SettingsGroup {
        heading: "Default apps"
        SettingsRow {
            label: "Terminal"
            description: "For the shell's own shortcuts and the launcher."
            Dropdown { listWidth: 160; options: Defaults.terminals; value: Prefs.p.terminal || "kitty"; onPicked: v => Defaults.setTerminal(v) }
        }
        Repeater {
            model: Defaults.kinds
            SettingsRow {
                id: kindRow
                required property var modelData
                readonly property var options: Defaults.candidates(modelData)
                label: modelData.label
                description: modelData.mimes.join(", ")
                Dropdown {
                    listWidth: 240
                    options: kindRow.options.length ? kindRow.options : [["", "Nothing installed"]]
                    value: Defaults.current(modelData)
                    onPicked: v => { if (v) Defaults.set(modelData, v); }
                }
            }
        }
    }

    SettingsGroup {
        heading: "Startup"
        Repeater {
            model: Startup.entries
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: modelData.exec + (modelData.source === "system" ? "  ·  system" : "")
                RowLayout {
                    spacing: Theme.s2
                    Button { text: "Remove"; variant: "text"; visible: modelData.source === "user"; onClicked: Startup.remove(modelData) }
                    Toggle { checked: modelData.enabled; onToggled: v => Startup.setEnabled(modelData, v) }
                }
            }
        }
        SettingsRow { visible: Startup.entries.length === 0; label: "Nothing starts with the session" }
        SettingsRow {
            label: "Add an app"
            description: "Copies its launcher into ~/.config/autostart."
            RowLayout {
                spacing: Theme.s2
                Dropdown {
                    listWidth: 260; maxRows: 10
                    options: [["", "Choose"]].concat(DesktopEntries.applications.values.filter(e => !e.noDisplay).map(e => [e.id, e.name]).sort((a, b) => a[1].localeCompare(b[1])))
                    value: ""
                    onPicked: v => { if (v) Startup.add(v.endsWith(".desktop") ? v : v + ".desktop"); }
                }
            }
        }
    }
}
