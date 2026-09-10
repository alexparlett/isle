import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Bluetooth"
    subtitle: !Bluetooth.available ? "No adapter" : Bluetooth.enabled ? "On" : "Off"

    Component.onCompleted: Bluetooth.setDiscovering(true)
    Component.onDestruction: Bluetooth.setDiscovering(false)

    SettingsGroup {
        SettingsRow {
            label: "Bluetooth"
            description: Bluetooth.discovering ? "Looking for devices" : ""
            Toggle { checked: Bluetooth.enabled; onToggled: v => Bluetooth.setEnabled(v) }
        }
    }

    SettingsGroup {
        visible: Bluetooth.request !== null
        heading: "Pairing"
        SettingsRow {
            label: Bluetooth.request ? (Bluetooth.request.kind === "display" ? "Type this on " + Bluetooth.request.device : Bluetooth.request.kind === "confirm" ? "Does " + Bluetooth.request.device + " show this code?" : "PIN for " + Bluetooth.request.device) : ""
            description: Bluetooth.request && Bluetooth.request.kind === "display" ? "Then press Enter on the device" : ""
            RowLayout {
                spacing: Theme.s2
                Label { visible: Bluetooth.request && Bluetooth.request.code !== ""; text: Bluetooth.request ? Bluetooth.request.code : ""; mono: true; tabular: true; size: Theme.sizeTitle; weight: Font.DemiBold; font.letterSpacing: 2 }
                Field { id: pinField; visible: Bluetooth.request && (Bluetooth.request.kind === "pin" || Bluetooth.request.kind === "passkey"); implicitWidth: 120; implicitHeight: 32; placeholder: "PIN"; onAccepted: { Bluetooth.reply(text); text = ""; } }
                Button { visible: Bluetooth.request && Bluetooth.request.kind === "confirm"; text: "Pair"; variant: "accent"; onClicked: Bluetooth.answer(true) }
                Button { visible: Bluetooth.request && Bluetooth.request.kind !== "display"; text: "Cancel"; variant: "text"; onClicked: Bluetooth.request.kind === "confirm" ? Bluetooth.answer(false) : Bluetooth.reply("") }
            }
        }
    }

    // One row per device; the same row serves the three lists below.
    component DeviceRow: SettingsRow {
        required property var modelData
        label: Bluetooth.nameOf(modelData)
        description: modelData.connected ? "Connected" + (modelData.batteryAvailable ? " · " + Math.round(modelData.battery * 100) + "%" : "") : modelData.paired ? "Paired" : modelData.pairing ? "Pairing…" : "Nearby"
        glyph: Bluetooth.glyphFor(modelData)
        glyphColor: modelData.connected ? Theme.accent : Theme.text2
        RowLayout {
            spacing: Theme.s2
            Button { text: modelData.connected ? "Disconnect" : modelData.paired ? "Connect" : "Pair"; variant: modelData.connected ? "text" : "raised"; enabled: !modelData.pairing; onClicked: modelData.paired ? Bluetooth.toggle(modelData) : modelData.pair() }
            Button { text: "Forget"; variant: "text"; visible: modelData.paired; onClicked: modelData.forget() }
        }
    }

    SettingsGroup {
        heading: "Your devices"
        Repeater { model: Bluetooth.connected.concat(Bluetooth.idle); DeviceRow {} }
        SettingsRow { visible: Bluetooth.connected.length + Bluetooth.idle.length === 0; label: "Nothing paired yet"; description: "Pair something from the list below." }
    }

    SettingsGroup {
        heading: "Nearby"
        Repeater { model: Bluetooth.nearby; DeviceRow {} }
        SettingsRow { visible: Bluetooth.nearby.length === 0; label: Bluetooth.enabled ? (Bluetooth.discovering ? "Looking…" : "Nothing nearby") : "Bluetooth is off"; description: Bluetooth.enabled ? "Put the device in pairing mode; headphones usually need the power button held until they say so." : "" }
    }
}
