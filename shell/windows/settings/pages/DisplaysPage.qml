import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Displays"
    subtitle: "Drag displays into place. Resolution, refresh rate, scale and rotation per display; changes apply at once."

    SettingsGroup {
        heading: "Arrangement"
        DisplayMap { Layout.fillWidth: true; Layout.margins: Theme.s2 }
        SettingsRow {
            label: "Primary display"
            description: "Where new windows open and the shell's surfaces appear."
            Dropdown {
                listWidth: 220
                options: Displays.monitors.map(m => [m.name, m.name + "  ·  " + (Displays.info(m).description || "").split(" ").slice(0, 2).join(" ")])
                value: Displays.primary
                onPicked: v => Prefs.p.primaryMonitor = v
            }
        }
        SettingsRow {
            visible: Displays.monitors.length > 1
            label: "Shell follows the pointer"
            description: "The island, launcher, notifications and dialogs go to whichever display the pointer is on."
            Toggle { checked: Prefs.p.shellFollowsFocus; onToggled: v => Prefs.p.shellFollowsFocus = v }
        }
    }

    Repeater {
        model: Displays.monitors
        SettingsGroup {
            id: group
            required property var modelData
            required property int index
            readonly property var mon: modelData
            readonly property var p: Displays.pref(mon.name)
            readonly property var o: Displays.info(mon)
            readonly property bool primary: Displays.primary === mon.name
            heading: (o.description || mon.name).split(" 0x")[0] + "  ·  " + mon.name + (primary ? "  ·  primary" : "")

            // The mode in use, whether chosen or the monitor's own: sizes and the rates each offers.
            readonly property var modes: Displays.modes(mon)
            readonly property string mode: p.mode && p.mode !== "preferred" ? p.mode : Displays.currentMode(mon)
            readonly property string size: mode.split("@")[0]
            readonly property var sizes: modes.map(m => m.split("@")[0]).filter((v, i, a) => a.indexOf(v) === i)
            readonly property var rates: modes.filter(m => m.split("@")[0] === size)
            function pick(size, rate) { Displays.set(mon.name, "mode", rate ? size + "@" + rate : (modes.find(m => m.split("@")[0] === size) || size)); }
            function rateLabel(m) { return Math.round(parseFloat(m.split("@")[1])) + " Hz" + (m === modes[0] ? "  ·  default" : ""); }

            SettingsRow {
                visible: group.index === 0 && Brightness.available
                label: "Brightness"
                description: Brightness.backend === "ddc" ? "An external display takes a moment to respond." : ""
                RowLayout {
                    spacing: Theme.s2
                    Slider { implicitWidth: 200; value: Brightness.value; onMoved: v => Brightness.set(v) }
                    Label { text: Math.round(Brightness.value * 100) + "%"; mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 40; horizontalAlignment: Text.AlignRight }
                }
            }
            SettingsRow {
                label: "Resolution"
                Dropdown {
                    listWidth: 200
                    options: group.sizes.map(v => [v, v.replace("x", " × ") + (v === (group.modes[0] || "").split("@")[0] ? "  ·  default" : "")])
                    value: group.size
                    onPicked: v => group.pick(v, "")
                }
            }
            SettingsRow {
                label: "Refresh rate"
                Dropdown {
                    listWidth: 180
                    options: group.rates.map(m => [m.split("@")[1], group.rateLabel(m)])
                    value: group.mode.split("@")[1] || ""
                    onPicked: v => group.pick(group.size, v)
                }
            }
            SettingsRow {
                label: "Scale"
                Dropdown { listWidth: 140; options: [["auto", "Auto"], [1, "100%"], [1.25, "125%"], [1.5, "150%"], [1.75, "175%"], [2, "200%"]]; value: group.p.scale || "auto"; onPicked: v => Displays.set(group.mon.name, "scale", v === "auto" ? 0 : v) }
            }
            SettingsRow {
                label: "Rotation"
                Dropdown { listWidth: 120; options: Displays.transforms; value: group.p.transform || 0; onPicked: v => Displays.set(group.mon.name, "transform", v) }
            }
            SettingsRow {
                label: "Variable refresh rate"
                description: "FreeSync and G-Sync. Game mode turns it on regardless."
                Toggle { checked: !!group.p.vrr; onToggled: v => Displays.set(group.mon.name, "vrr", v) }
            }
            SettingsRow {
                visible: !group.primary
                label: "Enabled"
                Toggle { checked: group.p.enabled !== false; onToggled: v => Displays.set(group.mon.name, "enabled", v) }
            }
            SettingsRow {
                visible: Object.keys(group.p).length > 0
                label: "Reset"
                description: "Back to what the display asks for."
                Button { text: "Reset"; variant: "text"; onClicked: Displays.reset(group.mon.name) }
            }
        }
    }

    SettingsGroup {
        heading: "Night light"
        SettingsRow {
            label: "Night light"
            description: NightLight.available ? "Warmer colours, easier on the eyes in the evening." : "Not available on this system."
            Toggle { checked: NightLight.on; onToggled: v => NightLight.setOn(v) }
        }
        SettingsRow {
            label: "Warmth"
            description: "Daylight at the right, candlelight at the left."
            RowLayout {
                spacing: Theme.s2
                Slider { implicitWidth: 200; value: (Prefs.p.nightLightTemperature - 1000) / 5500; onMoved: v => { Prefs.p.nightLightTemperature = Math.round((1000 + v * 5500) / 100) * 100; NightLight.apply(); } }
                Label { text: Prefs.p.nightLightTemperature + " K"; mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 52; horizontalAlignment: Text.AlignRight }
            }
        }
    }
}
