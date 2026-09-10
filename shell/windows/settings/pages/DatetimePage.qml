import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Date and time"
    subtitle: "The system clock through timedatectl. Changing it asks for an administrator."

    Component.onCompleted: DateTime.refresh()
    SystemClock { id: clock; precision: SystemClock.Seconds }
    property string zoneFilter: ""

    SettingsGroup {
        heading: "Now"
        SettingsRow {
            label: Qt.formatDateTime(clock.date, "dddd d MMMM yyyy")
            description: DateTime.timezone + (DateTime.ntp ? (DateTime.inSync ? "  ·  synchronised" : "  ·  waiting for network time") : "  ·  set by hand")
            Label { text: Qt.formatTime(clock.date, (Prefs.p.clock12 ? "h:mm:ss AP" : "HH:mm:ss")); mono: true; tabular: true; size: Theme.sizeTitle; weight: Font.DemiBold }
        }
    }

    SettingsGroup {
        heading: "Clock"
        SettingsRow {
            label: "Network time"
            description: DateTime.canNtp ? "Keep the clock right from the internet." : "No time service is installed."
            Toggle { checked: DateTime.ntp; onToggled: v => DateTime.setNtp(v) }
        }
        SettingsRow {
            visible: !DateTime.ntp
            label: "Set the time"
            description: "Date and time, as the machine should have them now."
            RowLayout {
                spacing: Theme.s2
                Field { id: dateField; implicitWidth: 120; implicitHeight: 32; text: Qt.formatDate(clock.date, "yyyy-MM-dd"); placeholder: "YYYY-MM-DD" }
                Field { id: timeField; implicitWidth: 90; implicitHeight: 32; text: Qt.formatTime(clock.date, "HH:mm:ss"); placeholder: "HH:MM:SS" }
                Button { text: "Set"; variant: "accent"; implicitHeight: 32; onClicked: DateTime.setTime(dateField.text + " " + timeField.text) }
            }
        }
        SettingsRow {
            label: "Time zone"
            description: DateTime.timezone
            RowLayout {
                spacing: Theme.s2
                Field { implicitWidth: 140; implicitHeight: 32; placeholder: "Filter, e.g. Lon"; onTextChanged: page.zoneFilter = text }
                Dropdown {
                    listWidth: 260; maxRows: 12
                    readonly property var shown: DateTime.zones.filter(z => !page.zoneFilter || z.toLowerCase().indexOf(page.zoneFilter.toLowerCase()) >= 0)
                    options: (shown.indexOf(DateTime.timezone) >= 0 ? shown : [DateTime.timezone].concat(shown)).map(z => [z, z.replace(/_/g, " ")])
                    value: DateTime.timezone
                    onPicked: v => DateTime.setTimezone(v)
                }
            }
        }
        SettingsRow {
            label: "24-hour clock"
            description: "For the island, the lock screen and Big Picture. The Clock widget has its own setting."
            Toggle { checked: !Prefs.p.clock12; onToggled: v => Prefs.p.clock12 = !v }
        }
        SettingsRow {
            label: "Hardware clock in local time"
            description: "Only for a machine that also boots Windows. Otherwise leave it in UTC."
            Toggle { checked: DateTime.localRtc; onToggled: v => DateTime.setLocalRtc(v) }
        }
        SettingsRow { visible: DateTime.error !== ""; label: "That did not work"; description: DateTime.error }
    }
}
