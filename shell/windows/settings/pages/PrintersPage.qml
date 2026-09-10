import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Printers"
    subtitle: Printers.available ? "Through CUPS. Adding a printer or changing its driver happens on the CUPS admin page." : "CUPS is not running."

    Component.onCompleted: Printers.listeners++
    Component.onDestruction: Printers.listeners--

    SettingsGroup {
        visible: !Printers.available
        SettingsRow {
            label: "Printing service"
            description: "Start CUPS to see and add printers. Needs the cups package."
            Button { text: "Start"; variant: "accent"; onClicked: Printers.startService() }
        }
    }

    SettingsGroup {
        visible: Printers.available
        heading: "Printers"
        Repeater {
            model: Printers.printers
            SettingsRow {
                id: row
                required property var modelData
                label: modelData.name + (modelData.default ? "  ·  default" : "")
                description: Printers.stateLabel(modelData) + (modelData.reason ? "  ·  " + modelData.reason : "") + (modelData.uri ? "  ·  " + modelData.uri : "")
                RowLayout {
                    spacing: Theme.s2
                    Rectangle { width: 8; height: 8; radius: 4; color: modelData.state === "disabled" ? Theme.warn : modelData.state === "printing" ? Theme.accent : Theme.ok }
                    Button { visible: !modelData.default; text: "Make default"; variant: "text"; onClicked: Printers.setDefault(modelData) }
                    Button { text: modelData.state === "disabled" ? "Resume" : "Pause"; variant: "text"; onClicked: modelData.state === "disabled" ? Printers.resume(modelData) : Printers.pause(modelData) }
                    Button { text: "Test page"; variant: "text"; onClicked: Printers.testPage(modelData) }
                }
            }
        }
        SettingsRow { visible: Printers.printers.length === 0; label: "No printers"; description: "Add one on the CUPS admin page; network printers usually appear on their own." }
        SettingsRow {
            label: "Add or configure a printer"
            description: "The CUPS admin page, at localhost:631."
            Button { text: "Open"; glyph: "printer"; variant: "raised"; onClicked: Printers.admin() }
        }
    }

    SettingsGroup {
        visible: Printers.available
        heading: "Queue"
        Repeater {
            model: Printers.jobs
            SettingsRow {
                required property var modelData
                label: "Job " + modelData.id + " on " + modelData.printer
                description: modelData.user + "  ·  " + System.bytes(modelData.size) + "  ·  " + modelData.time
                Button { text: "Cancel"; variant: "text"; onClicked: Printers.cancel(modelData) }
            }
        }
        SettingsRow { visible: Printers.jobs.length === 0; label: "Nothing queued" }
        SettingsRow { visible: Printers.error !== ""; label: "That did not work"; description: Printers.error }
    }
}
