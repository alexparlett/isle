import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Network"
    subtitle: Network.wiredConnected ? "On ethernet" : Network.wifiConnected ? "On Wi-Fi" : "Not connected"

    Component.onCompleted: { Network.scan(); Vpn.refresh(); }
    Component.onDestruction: Network.stopScan()
    // "" is the fastest server anywhere; otherwise a country code.

    SettingsGroup {
        heading: "Wi-Fi"
        SettingsRow {
            label: "Wi-Fi"
            description: Network.wifiAvailable ? Network.summary : "No adapter"
            Toggle { checked: Network.wifiAvailable && Network.wifiEnabled; onToggled: v => Network.setWifiEnabled(v) }
        }
        Repeater {
            model: Network.networks
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: modelData.connected ? "Connected" : Network.securityLabel(modelData) + (modelData.known ? " · Saved" : "")
                glyph: Network.glyphFor(modelData)
                glyphColor: modelData.connected ? Theme.accent : Theme.text2
                RowLayout {
                    spacing: Theme.s1
                    Button { text: modelData.connected ? "Disconnect" : "Connect"; variant: modelData.connected ? "text" : "raised"; onClicked: modelData.connected ? Network.disconnect(modelData) : Network.connect(modelData, "") }
                    Button { text: "Forget"; variant: "text"; visible: modelData.known; onClicked: Network.forget(modelData) }
                }
            }
        }
    }

    SettingsGroup {
        heading: "Wired"
        SettingsRow {
            label: Network.wiredDevice ? Network.wiredDevice.name : "No adapter"
            description: Network.wiredDevice ? (Network.wiredConnected ? "Connected · " + Network.wiredDevice.address : "No link") : ""
        }
    }

    SettingsGroup {
        heading: "VPN"
        Repeater {
            model: Vpn.providers
            SettingsRow {
                id: vpnRow
                required property var modelData
                readonly property var impl: modelData.impl
                readonly property bool on: Vpn.enabled(modelData)
                label: modelData.name
                description: !impl.installed ? "Not installed" : !on ? modelData.note : impl.state + (impl.error ? "  ·  " + impl.error : "")
                RowLayout {
                    spacing: Theme.s2
                    // The provider's own actions, except connecting: a toggle, a choice, a button (sign in, sign out).
                    Repeater {
                        model: vpnRow.on ? vpnRow.impl.actions.filter(a => a.id !== "connect" && a.id !== "disconnect") : []
                        RowLayout {
                            required property var modelData
                            spacing: Theme.s1
                            Label { visible: modelData.on !== undefined || !!modelData.options; text: modelData.label; size: Theme.sizeCaption; color: Theme.text2 }
                            Toggle { visible: modelData.on !== undefined; checked: !!modelData.on; enabled: !vpnRow.impl.busy; onToggled: vpnRow.impl.act(modelData.id, "") }
                            Dropdown { visible: !!modelData.options; listWidth: 220; options: modelData.options || []; value: modelData.value || ""; enabled: !vpnRow.impl.busy; onPicked: v => vpnRow.impl.act(modelData.id, v) }
                            Button { visible: modelData.on === undefined && !modelData.options; text: modelData.label; variant: modelData.primary ? "accent" : "text"; implicitHeight: 32; enabled: !vpnRow.impl.busy; onClicked: vpnRow.impl.act(modelData.id, "") }
                        }
                    }
                    Toggle { enabled: vpnRow.impl.installed; checked: vpnRow.on; onToggled: v => Vpn.setEnabled(vpnRow.modelData.id, v) }
                }
            }
        }
        SettingsRow { visible: Vpn.providers.length === 0; label: "No VPN has a provider" }
    }
}
