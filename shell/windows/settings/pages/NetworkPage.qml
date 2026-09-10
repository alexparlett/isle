import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Network"
    subtitle: Network.wiredConnected ? "On ethernet" : Network.wifiConnected ? "On Wi-Fi" : "Not connected"

    Component.onCompleted: { Network.scan(); Vpn.refreshStatus(); Vpn.refreshInfo(); }
    Component.onDestruction: Network.stopScan()
    // "" is the fastest server anywhere; otherwise a country code.
    property string vpnCountry: ""

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
        visible: Vpn.protonInstalled || Vpn.connections.length > 0
        SettingsRow {
            visible: Vpn.protonInstalled && !Vpn.protonSignedIn
            label: "Proton VPN"
            description: Vpn.needsCode ? "Enter the code from your authenticator." : Vpn.signingIn ? "Signing in" : "Signed out."
            RowLayout {
                spacing: Theme.s2
                Spinner { visible: Vpn.signingIn && !Vpn.needsCode; Layout.rightMargin: Theme.s2 }
                Field { id: protonUser; visible: !Vpn.signingIn; implicitWidth: 200; implicitHeight: 32; placeholder: "Username"; next: protonPass; onAccepted: protonPass.input.forceActiveFocus() }
                Field {
                    id: protonPass
                    visible: !Vpn.signingIn
                    implicitWidth: 180; implicitHeight: 32
                    placeholder: "Password"
                    input.echoMode: TextInput.Password
                    onAccepted: { Vpn.signIn(protonUser.text, text); text = ""; }
                }
                Field {
                    id: protonCode
                    visible: Vpn.needsCode
                    implicitWidth: 140; implicitHeight: 32
                    placeholder: "Two-factor code"
                    onVisibleChanged: if (visible) { text = ""; input.forceActiveFocus(); }
                    onAccepted: { Vpn.submitCode(text); text = ""; }
                }
                Button { visible: !Vpn.signingIn; text: "Sign in"; variant: "raised"; enabled: protonUser.text !== "" && protonPass.text !== ""; onClicked: { Vpn.signIn(protonUser.text, protonPass.text); protonPass.text = ""; } }
                Button { visible: Vpn.needsCode; text: "Verify"; variant: "raised"; enabled: protonCode.text !== ""; onClicked: { Vpn.submitCode(protonCode.text); protonCode.text = ""; } }
            }
        }
        SettingsRow {
            visible: Vpn.protonSignedIn
            label: "Proton VPN"
            description: Vpn.busy ? "Working" : Vpn.proton.connected ? "Connected to " + Vpn.proton.server + " in " + Vpn.proton.location + " · " + Vpn.proton.protocol + (Vpn.proton.load ? " · load " + Vpn.proton.load : "") : "Signed in as " + Vpn.protonAccount + " · Disconnected"
            RowLayout {
                spacing: Theme.s2
                Dropdown {
                    visible: !Vpn.proton.connected
                    listWidth: 240
                    options: [["", "Fastest server"]].concat(Vpn.countries.map(c => [c.code, c.name]))
                    value: page.vpnCountry
                    onPicked: v => page.vpnCountry = v
                }
                Button { text: Vpn.proton.connected ? "Disconnect" : "Connect"; variant: Vpn.proton.connected ? "text" : "raised"; enabled: !Vpn.busy; onClicked: Vpn.proton.connected ? Vpn.disconnect() : Vpn.connect(page.vpnCountry) }
                Button { text: "Sign out"; variant: "text"; enabled: !Vpn.busy; onClicked: Vpn.signOut() }
            }
        }
        SettingsRow {
            visible: Vpn.protonSignedIn && Vpn.protonSettings["kill-switch"] !== undefined
            label: "Kill switch"
            description: "Block the internet when the VPN drops"
            Toggle { checked: Vpn.protonSettings["kill-switch"] === "standard"; enabled: !Vpn.busy; onToggled: v => Vpn.setSetting("kill-switch", v ? "standard" : "off") }
        }
        SettingsRow {
            visible: Vpn.protonSignedIn && Vpn.protonSettings["netshield"] !== undefined
            label: "NetShield"
            description: "Block malware, ads and trackers on the way in"
            Dropdown {
                listWidth: 220
                options: [["off", "Off"], ["malware-only", "Malware"], ["malware-ads-trackers", "Malware, ads and trackers"]]
                value: Vpn.protonSettings["netshield"] || "off"
                enabled: !Vpn.busy
                onPicked: v => Vpn.setSetting("netshield", v)
            }
        }
        SettingsRow {
            visible: Vpn.error !== ""
            label: "Proton VPN said"
            description: Vpn.error
        }
        Repeater {
            model: Vpn.connections
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: (modelData.type === "wireguard" ? "WireGuard" : "NetworkManager VPN") + (modelData.active ? " · Connected" : "")
                Toggle { checked: modelData.active; onToggled: v => v ? Vpn.up(modelData) : Vpn.down(modelData) }
            }
        }
    }
}
