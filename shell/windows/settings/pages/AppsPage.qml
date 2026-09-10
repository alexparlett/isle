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

    // Startup, in the three places a session starts things: XDG autostart (apps), systemd user units
    // (services) and the compositor's own start hook (the shell). Entries for other desktops are folded away.
    property bool showOthers: false
    SettingsGroup {
        heading: "Apps at login"
        Repeater {
            model: Startup.entries.filter(e => e.applies)
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: (modelData.comment || modelData.exec) + (modelData.source === "system" ? "  ·  came with a package" : "")
                RowLayout {
                    spacing: Theme.s2
                    Button { text: "Remove"; variant: "text"; visible: modelData.source === "user"; onClicked: Startup.remove(modelData) }
                    Toggle { checked: modelData.enabled; onToggled: v => Startup.setEnabled(modelData, v) }
                }
            }
        }
        SettingsRow { visible: Startup.entries.filter(e => e.applies).length === 0; label: "No apps start at login" }
        SettingsRow {
            label: "Add an app"
            description: "Copies its launcher into ~/.config/autostart."
            Dropdown {
                listWidth: 260; maxRows: 10
                options: [["", "Choose"]].concat(DesktopEntries.applications.values.filter(e => !e.noDisplay).map(e => [e.id, e.name]).sort((a, b) => a[1].localeCompare(b[1])))
                value: ""
                onPicked: v => { if (v) Startup.add(v.endsWith(".desktop") ? v : v + ".desktop"); }
            }
        }
        // Entries that name another desktop never run here; listed folded, so they are seen but not in the way.
        SettingsRow {
            visible: Startup.entries.some(e => !e.applies)
            label: Startup.entries.filter(e => !e.applies).length + " for other desktops"
            description: "Marked for GNOME or others; they never start here."
            Button { text: page.showOthers ? "Hide" : "Show"; variant: "text"; onClicked: page.showOthers = !page.showOthers }
        }
        Repeater {
            model: page.showOthers ? Startup.entries.filter(e => !e.applies) : []
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: "Only for " + modelData.onlyFor
                Label { text: "not here"; size: Theme.sizeCaption; color: Theme.text3 }
            }
        }
    }

    SettingsGroup {
        heading: "Services"
        Repeater {
            model: Startup.units
            SettingsRow {
                id: unitRow
                required property var modelData
                readonly property bool essential: Startup.essential(modelData)
                label: modelData.name
                description: modelData.id + (modelData.active ? "  ·  running" : "") + (essential ? "  ·  the shell needs this" : "")
                Toggle { visible: !unitRow.essential; checked: unitRow.modelData.enabled; onToggled: v => Startup.setUnit(unitRow.modelData, v) }
                Glyph { visible: unitRow.essential; name: "lock"; size: 14; color: Theme.text3 }
            }
        }
        SettingsRow { visible: Startup.units.length === 0; label: "No user services are enabled" }
    }

    SettingsGroup {
        heading: "With the compositor"
        Repeater {
            model: Startup.hook
            SettingsRow {
                required property string modelData
                label: modelData.indexOf("isle-session") >= 0 ? "The shell" : modelData.indexOf("import-environment") >= 0 ? "Session environment for services" : modelData.indexOf("hyprpm") >= 0 ? "Compositor plugins" : modelData.indexOf("dex") >= 0 ? "The apps above" : modelData
                description: modelData
                Label { text: "hyprland.lua"; size: Theme.sizeCaption; color: Theme.text3 }
            }
        }
    }
}
