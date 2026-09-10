import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Mouse and touchpad"
    subtitle: "Applied to the compositor at once."

    component Pct: RowLayout {
        property string key
        property real min: -1
        property real max: 1
        property real fallback: 0
        property string unit: ""
        spacing: Theme.s2
        readonly property real v: Input.get(key, fallback)
        Slider { implicitWidth: 180; value: (v - min) / (max - min); onMoved: f => Input.set(key, Math.round((min + f * (max - min)) * 20) / 20) }
        Label { text: v.toFixed(2) + unit; mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 44; horizontalAlignment: Text.AlignRight }
    }

    SettingsGroup {
        heading: "Mouse"
        SettingsRow { label: "Speed"; description: "0 is the device's own."; Pct { key: "sensitivity"; min: -1; max: 1 } }
        SettingsRow {
            label: "Acceleration"
            description: "Flat is what games and precision want."
            RowLayout {
                spacing: Theme.s2
                Button { text: "Flat"; variant: Input.get("accelProfile", "flat") === "flat" ? "accent" : "raised"; onClicked: Input.set("accelProfile", "flat") }
                Button { text: "Adaptive"; variant: Input.get("accelProfile", "flat") === "adaptive" ? "accent" : "raised"; onClicked: Input.set("accelProfile", "adaptive") }
            }
        }
        SettingsRow { label: "Scroll speed"; Pct { key: "scrollFactor"; min: 0.2; max: 3; fallback: 1; unit: "×" } }
        SettingsRow { label: "Natural scrolling"; description: "Content follows the wheel."; Toggle { checked: Input.get("naturalScroll", false); onToggled: v => Input.set("naturalScroll", v) } }
        SettingsRow { label: "Left-handed"; Toggle { checked: Input.get("leftHanded", false); onToggled: v => Input.set("leftHanded", v) } }
    }

    SettingsGroup {
        heading: "Touchpad"
        SettingsRow { label: "Natural scrolling"; Toggle { checked: Input.get("touchpadNatural", true); onToggled: v => Input.set("touchpadNatural", v) } }
        SettingsRow { label: "Tap to click"; Toggle { checked: Input.get("tapToClick", true); onToggled: v => Input.set("tapToClick", v) } }
        SettingsRow { label: "Disable while typing"; Toggle { checked: Input.get("disableWhileTyping", true); onToggled: v => Input.set("disableWhileTyping", v) } }
        SettingsRow { label: "Scroll speed"; Pct { key: "touchpadScroll"; min: 0.2; max: 3; fallback: 1; unit: "×" } }
    }
}
