import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The control panel below the header band: toggles, output, media, footer. Drill-downs replace the body in place.
ColumnLayout {
    id: root
    // Raised when a click opens something elsewhere; the island folds and stays folded under the pointer.
    signal dismiss()
    spacing: Theme.s3

    // "main" | "wifi" | "bluetooth" | "output" | "input"
    property string page: "main"
    // The secured network waiting for its password, if any.
    property var pskFor: null

    // --- main -----------------------------------------------------------------

    GridLayout {
        visible: root.page === "main"
        Layout.fillWidth: true
        columns: 2
        columnSpacing: Theme.s2
        rowSpacing: Theme.s2

        ToggleTile {
            glyph: "wifi"; label: "Wi-Fi"; sub: Network.wifiAvailable ? Network.summary : "No adapter"
            on: Network.wifiAvailable && Network.wifiEnabled; enabled: Network.wifiAvailable; hasMore: Network.wifiAvailable
            onToggled: v => Network.setWifiEnabled(v)
            onMore: root.page = "wifi"
        }
        ToggleTile {
            glyph: "bluetooth"; label: "Bluetooth"
            sub: !Bluetooth.available ? "No adapter" : Bluetooth.anyConnected ? Bluetooth.primaryName : Bluetooth.enabled ? "On" : "Off"
            on: Bluetooth.enabled; enabled: Bluetooth.available; hasMore: Bluetooth.available
            onToggled: v => Bluetooth.setEnabled(v)
            onMore: root.page = "bluetooth"
        }
        ToggleTile {
            glyph: "moon"; label: "Night light"; sub: !NightLight.available ? "Needs hyprsunset" : NightLight.on ? NightLight.temperature + " K" : "Off"
            on: NightLight.on; enabled: NightLight.available
            onToggled: v => NightLight.setOn(v)
        }
        ToggleTile {
            glyph: Notifications.count > 0 ? "bell" : "bell-off"; label: "Notifications"
            // The count is always there; the silence reason joins it when something is holding toasts back.
            sub: (Notifications.count > 0 ? Notifications.count + " waiting" : "All clear") + (Notifications.silenced ? "  ·  silenced" : "")
            on: Notifications.dnd; hasMore: true
            onToggled: v => Notifications.setDnd(v)
            onMore: root.page = "notifications"
        }
        ToggleTile {
            glyph: "gamepad-2"; label: "Game mode"; sub: Modes.game ? "On" : "Off"
            on: Modes.game
            onToggled: v => Modes.toggle("game")
        }
        ToggleTile {
            visible: Vpn.available
            glyph: "shield"; label: "VPN"
            sub: Vpn.busy ? "Working" : Vpn.active ? Vpn.active.name : Vpn.protonSignedIn ? "Proton VPN" : Vpn.protonInstalled ? "Sign in to Proton VPN" : Vpn.connections.map(c => c.name).join(", ")
            on: Vpn.active !== null; enabled: !Vpn.busy && Vpn.ready; hasMore: true
            onToggled: v => Vpn.toggle()
            onMore: root.page = "vpn"
        }
    }

    Card {
        visible: root.page === "main"
        Layout.fillWidth: true
        ColumnLayout {
            width: parent.width
            spacing: Theme.s3
            RowLayout {
                Label { text: "Output"; size: Theme.sizeCaption; color: Theme.text2 }
                Item { Layout.fillWidth: true }
                Item {
                    implicitWidth: outRow.implicitWidth; implicitHeight: outRow.implicitHeight
                    RowLayout {
                        id: outRow
                        spacing: Theme.s1
                        Glyph { name: "headphones"; size: 10; visible: Bluetooth.anyConnected }
                        Label { text: Audio.sink ? Audio.sink.description : "No output"; size: Theme.sizeCaption; color: Theme.text2; Layout.maximumWidth: 200 }
                        Glyph { name: "chevron-right"; size: 10; color: Theme.text3 }
                    }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.page = "output" }
                }
            }
            RowLayout {
                spacing: Theme.s2
                Glyph { name: Audio.muted ? "volume-x" : "volume-2"; size: 14
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Audio.setMuted(!Audio.muted) } }
                Slider { Layout.fillWidth: true; value: Audio.muted ? 0 : Audio.volume; onMoved: v => { Audio.setMuted(false); Audio.setVolume(v); } }
                Label { text: Audio.muted ? "0" : Math.round(Audio.volume * 100); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 24; horizontalAlignment: Text.AlignRight }
            }
            RowLayout {
                visible: Brightness.available
                spacing: Theme.s2
                Glyph { name: "sun"; size: 14 }
                Slider { Layout.fillWidth: true; value: Brightness.value; onMoved: v => Brightness.set(v) }
                Label { text: Math.round(Brightness.value * 100); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 24; horizontalAlignment: Text.AlignRight }
            }
            // The microphone, only while something is listening or it is muted.
            RowLayout {
                visible: Audio.micInUse || Audio.sourceMuted
                spacing: Theme.s2
                Glyph { name: Audio.sourceMuted ? "mic-off" : "mic"; size: 14; color: Audio.micInUse && !Audio.sourceMuted ? Theme.live : Theme.text2
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Audio.setSourceMuted(!Audio.sourceMuted) } }
                Slider { Layout.fillWidth: true; value: Audio.sourceMuted ? 0 : Audio.sourceVolume; onMoved: v => { Audio.setSourceMuted(false); Audio.setSourceVolume(v); } }
                Item {
                    implicitWidth: inRow.implicitWidth; implicitHeight: inRow.implicitHeight
                    RowLayout { id: inRow; spacing: Theme.s1
                        Label { text: Audio.captures.map(c => Audio.streamName(c)).join(", ") || "Input"; size: Theme.sizeCaption; color: Theme.text2; Layout.maximumWidth: 120 }
                        Glyph { name: "chevron-right"; size: 10; color: Theme.text3 } }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.page = "input" }
                }
            }
        }
    }

    Card {
        visible: root.page === "main" && Media.present
        Layout.fillWidth: true
        padding: Theme.s2 + 2
        RowLayout {
            width: parent.width
            spacing: Theme.s3
            Rectangle {
                width: 44; height: 44; radius: Theme.radiusControl
                color: Theme.pressed
                clip: true
                Image { anchors.fill: parent; source: Media.artUrl; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                Glyph { anchors.centerIn: parent; name: "music"; size: 14; visible: Media.artUrl === "" }
            }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2
                Label { text: Media.title; weight: Font.DemiBold; Layout.fillWidth: true }
                Label { text: Media.artist; size: Theme.sizeSmall; color: Theme.text2; Layout.fillWidth: true }
            }
            RowLayout {
                spacing: Theme.s2 + 2
                Glyph { name: "skip-back"; size: 14; MouseArea { anchors.fill: parent; onClicked: Media.previous() } }
                Glyph { name: Media.playing ? "pause" : "play"; size: 14; weight: 1.6; color: Theme.text; MouseArea { anchors.fill: parent; onClicked: Media.togglePlaying() } }
                Glyph { name: "skip-forward"; size: 14; MouseArea { anchors.fill: parent; onClicked: Media.next() } }
            }
        }
    }

    RowLayout {
        visible: root.page === "main"
        Layout.fillWidth: true
        Layout.leftMargin: Theme.s1
        Layout.rightMargin: Theme.s1
        Repeater {
            model: Power.peripherals
            RowLayout {
                required property var modelData
                spacing: Theme.s1 + 2
                Glyph { name: Power.glyphFor(modelData); size: 14 }
                Label { text: modelData.model; size: Theme.sizeCaption; color: Theme.text2 }
                Label { text: Math.round(modelData.percentage) + "%"; size: Theme.sizeCaption; tabular: true; color: modelData.percentage < 20 ? Theme.warn : Theme.ok }
            }
        }
        Item {
            visible: Updates.count > 0 || IsleUpdate.behind > 0
            implicitWidth: updRow.implicitWidth; implicitHeight: updRow.implicitHeight
            RowLayout {
                id: updRow
                spacing: Theme.s1 + 2
                Glyph { name: "download"; size: 14; color: Theme.accent }
                Label {
                    text: Updates.count > 0 ? Updates.count + (Updates.count === 1 ? " update" : " updates") + (IsleUpdate.behind > 0 ? " + Isle" : "") : "Isle update"
                    size: Theme.sizeCaption; color: Theme.text2
                }
            }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { root.dismiss(); Surfaces.showSettings("updates"); } }
        }
        Item { Layout.fillWidth: true }
        Item {
            implicitWidth: profRow.implicitWidth; implicitHeight: profRow.implicitHeight
            RowLayout {
                id: profRow
                spacing: Theme.s1 + 2
                Glyph { name: "power"; size: 14 }
                Label { text: Power.profileLabel; size: Theme.sizeCaption; color: Theme.text2 }
            }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { root.dismiss(); Surfaces.power = true; } }
        }
    }

    // --- drill-downs ---------------------------------------------------------------

    PanelPage {
        visible: root.page === "wifi"
        Layout.fillWidth: true
        title: "Wi-Fi"
        onBack: { root.page = "main"; root.pskFor = null; }
        onShown: Network.scan()
        onHidden: { Network.stopScan(); root.pskFor = null; }
        Repeater {
            model: Network.networks
            ColumnLayout {
                id: netRow
                required property var modelData
                Layout.fillWidth: true
                spacing: 2
                readonly property bool asking: root.pskFor === modelData
                ListRow {
                    Layout.fillWidth: true
                    glyph: Network.glyphFor(netRow.modelData)
                    glyphColor: netRow.modelData.connected ? Theme.accent : Theme.text2
                    title: netRow.modelData.name
                    subtitle: netRow.modelData.connected ? "Connected" : Network.securityLabel(netRow.modelData) + (netRow.modelData.known ? " · Saved" : "")
                    onClicked: {
                        const n = netRow.modelData;
                        if (n.connected) Network.disconnect(n);
                        else if (Network.needsPassword(n)) root.pskFor = netRow.asking ? null : n;
                        else Network.connect(n, "");
                    }
                    Glyph { name: "check"; size: 14; color: Theme.accent; visible: netRow.modelData.connected }
                }
                // The password, asked for in place.
                RowLayout {
                    visible: netRow.asking
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.s3
                    Layout.rightMargin: Theme.s3
                    Layout.bottomMargin: Theme.s2
                    spacing: Theme.s2
                    Field {
                        id: psk
                        Layout.fillWidth: true
                        implicitHeight: 34
                        glyph: "key-round"
                        placeholder: "Password"
                        input.echoMode: TextInput.Password
                        onVisibleChanged: if (visible) { text = ""; input.forceActiveFocus(); }
                        onAccepted: { Network.connect(netRow.modelData, text); root.pskFor = null; }
                        input.Keys.onEscapePressed: root.pskFor = null
                    }
                    Button { text: "Connect"; variant: "accent"; implicitHeight: 34; enabled: psk.text.length >= 8; onClicked: { Network.connect(netRow.modelData, psk.text); root.pskFor = null; } }
                }
            }
        }
        Label { visible: Network.networks.length === 0; text: Network.wifiEnabled ? "Looking for networks" : "Wi-Fi is off"; color: Theme.text3; Layout.margins: Theme.s3 }
    }

    // The notification centre: what is waiting, with its actions and reply live; earlier ones beneath.
    PanelPage {
        visible: root.page === "notifications"
        Layout.fillWidth: true
        title: "Notifications"
        onBack: root.page = "main"
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.s2
            Layout.rightMargin: Theme.s2
            Label { text: Notifications.silenced ? "Silenced: " + Notifications.silencedBy : Notifications.count > 0 ? Notifications.count + " waiting" : "All clear"; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true }
            Label { visible: Notifications.count > 0 || Notifications.past.length > 0; text: "Clear all"; size: Theme.sizeCaption; color: Theme.accent
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Notifications.clearAll() } }
        }
        Item {
            Layout.fillWidth: true
            implicitHeight: Math.min(centreList.contentHeight, 360)
            visible: Notifications.count > 0 || Notifications.past.length > 0
            ListView {
                id: centreList
                anchors.fill: parent
                clip: true
                spacing: 0
                boundsBehavior: Flickable.StopAtBounds
                // Live first, then the past; a header row divides them.
                model: Notifications.list.map(n => ({ n: n })).concat(Notifications.past.length ? [{ header: "Earlier" }] : [], Notifications.past.slice(0, 40).map(r => ({ past: r })))
                delegate: Item {
                    required property var modelData
                    width: centreList.width
                    height: modelData.header ? 26 : card.implicitHeight + 1
                    Label { visible: !!modelData.header; anchors { left: parent.left; bottom: parent.bottom; leftMargin: Theme.s2; bottomMargin: 6 } text: modelData.header || ""; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3 }
                    NotificationCard { id: card; visible: !modelData.header; anchors { left: parent.left; right: parent.right; leftMargin: Theme.s2; rightMargin: Theme.s2 } n: modelData.n || null; past: modelData.past || null }
                    Rectangle { visible: !modelData.header; anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: Theme.s2; rightMargin: Theme.s2 } height: 1; color: Theme.hairline }
                }
            }
            Scrollbar { target: centreList; anchors { top: parent.top; bottom: parent.bottom; right: parent.right } }
        }
        Label { visible: Notifications.count === 0 && Notifications.past.length === 0; text: "Nothing yet"; color: Theme.text3; Layout.margins: Theme.s3 }
    }

    PanelPage {
        visible: root.page === "vpn"
        Layout.fillWidth: true
        title: "VPN"
        onBack: root.page = "main"
        onShown: { Vpn.refreshStatus(); Vpn.refresh(); }
        ListRow {
            visible: Vpn.protonInstalled && !Vpn.protonSignedIn
            Layout.fillWidth: true
            glyph: "shield"; title: "Proton VPN"; subtitle: "Sign in from Settings"
            onClicked: { root.dismiss(); Surfaces.showSettings("network"); }
        }
        ListRow {
            visible: Vpn.protonSignedIn
            Layout.fillWidth: true
            glyph: "shield"; glyphColor: Vpn.proton.connected ? Theme.accent : Theme.text2
            title: Vpn.proton.connected ? Vpn.proton.server : "Fastest server"
            subtitle: Vpn.busy ? "Working" : Vpn.proton.connected ? Vpn.proton.location + " · " + Vpn.proton.protocol : "Proton VPN"
            onClicked: Vpn.proton.connected ? Vpn.disconnect() : Vpn.connect("")
            Glyph { name: "check"; size: 14; color: Theme.accent; visible: Vpn.proton.connected }
        }
        // The countries run past a hundred: a fixed window that scrolls.
        Item {
            visible: Vpn.countries.length > 0
            Layout.fillWidth: true
            implicitHeight: Math.min(countryList.contentHeight, 240)
            ListView {
                id: countryList
                anchors.fill: parent
                clip: true
                model: Vpn.countries
                spacing: 2
                boundsBehavior: Flickable.StopAtBounds
                delegate: ListRow {
                    required property var modelData
                    width: countryList.width
                    glyph: "globe"
                    title: modelData.name
                    subtitle: modelData.code
                    onClicked: Vpn.connect(modelData.code)
                }
            }
            Scrollbar { target: countryList; anchors { top: parent.top; bottom: parent.bottom; right: parent.right } }
        }
        Repeater {
            model: Vpn.connections
            ListRow {
                required property var modelData
                Layout.fillWidth: true
                glyph: "shield"; glyphColor: modelData.active ? Theme.accent : Theme.text2
                title: modelData.name
                subtitle: modelData.active ? "Connected" : modelData.type === "wireguard" ? "WireGuard" : "NetworkManager"
                onClicked: modelData.active ? Vpn.down(modelData) : Vpn.up(modelData)
                Glyph { name: "check"; size: 14; color: Theme.accent; visible: modelData.active }
            }
        }
        Label { visible: Vpn.error !== ""; text: Vpn.error; color: Theme.danger; size: Theme.sizeCaption; wrapMode: Text.WordWrap; Layout.fillWidth: true; Layout.margins: Theme.s3 }
    }

    PanelPage {
        visible: root.page === "bluetooth"
        Layout.fillWidth: true
        title: "Bluetooth"
        onBack: root.page = "main"
        onShown: Bluetooth.setDiscovering(true)
        onHidden: Bluetooth.setDiscovering(false)
        // A pairing code, while a device asks.
        Card {
            visible: Bluetooth.request !== null
            Layout.fillWidth: true
            RowLayout {
                width: parent.width
                spacing: Theme.s2
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 1
                    Label { text: Bluetooth.request ? Bluetooth.request.device : ""; weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                    Label { text: Bluetooth.request ? (Bluetooth.request.kind === "confirm" ? "Shows this code?" : Bluetooth.request.kind === "display" ? "Type this, then Enter" : "Enter its PIN") : ""; size: Theme.sizeCaption; color: Theme.text2 }
                }
                Label { visible: Bluetooth.request && Bluetooth.request.code !== ""; text: Bluetooth.request ? Bluetooth.request.code : ""; mono: true; tabular: true; size: Theme.sizeHeading; weight: Font.DemiBold }
                Field { visible: Bluetooth.request && (Bluetooth.request.kind === "pin" || Bluetooth.request.kind === "passkey"); implicitWidth: 90; implicitHeight: 30; placeholder: "PIN"; onAccepted: { Bluetooth.reply(text); text = ""; } }
                Button { visible: Bluetooth.request && Bluetooth.request.kind === "confirm"; text: "Pair"; variant: "accent"; implicitHeight: 30; onClicked: Bluetooth.answer(true) }
                Button { visible: Bluetooth.request && Bluetooth.request.kind === "confirm"; text: "No"; variant: "text"; implicitHeight: 30; onClicked: Bluetooth.answer(false) }
            }
        }
        Repeater {
            model: Bluetooth.devices
            ListRow {
                required property var modelData
                Layout.fillWidth: true
                visible: modelData.paired || modelData.trusted || modelData.name !== ""
                glyph: Bluetooth.glyphFor(modelData)
                glyphColor: modelData.connected ? Theme.accent : Theme.text2
                title: modelData.name || modelData.address
                subtitle: modelData.connected ? "Connected" + (modelData.batteryAvailable ? " · " + Math.round(modelData.battery * 100) + "%" : "")
                        : modelData.pairing ? "Pairing" : modelData.paired ? "Paired" : "Nearby"
                onClicked: modelData.paired ? Bluetooth.toggle(modelData) : modelData.pair()
                Glyph { name: "check"; size: 14; color: Theme.accent; visible: modelData.connected }
            }
        }
        Label { visible: Bluetooth.devices.length === 0; text: Bluetooth.enabled ? "Looking for devices" : "Bluetooth is off"; color: Theme.text3; Layout.margins: Theme.s3 }
    }

    PanelPage {
        visible: root.page === "output"
        Layout.fillWidth: true
        title: "Output"
        onBack: root.page = "main"
        Repeater {
            model: Audio.sinks
            ListRow {
                required property var modelData
                Layout.fillWidth: true
                glyph: modelData.name.indexOf("bluez") === 0 ? "headphones" : "volume-2"
                glyphColor: modelData === Audio.sink ? Theme.accent : Theme.text2
                title: modelData.description
                onClicked: Audio.setSink(modelData)
                Glyph { name: "check"; size: 14; color: Theme.accent; visible: modelData === Audio.sink }
            }
        }
        // Apps: each stream's own volume.
        Label { visible: Audio.streams.length > 0; text: "Apps"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.margins: Theme.s3; Layout.bottomMargin: 0 }
        Repeater {
            model: Audio.streams
            RowLayout {
                id: streamRow
                required property var modelData
                Layout.fillWidth: true
                Layout.leftMargin: Theme.s3
                Layout.rightMargin: Theme.s3
                spacing: Theme.s2
                readonly property string icon: Audio.streamIcon(modelData)
                readonly property bool muted: modelData.audio ? modelData.audio.muted : false
                Item {
                    implicitWidth: 20; implicitHeight: 20
                    AppIcon { anchors.fill: parent; size: 20; source: streamRow.icon; visible: streamRow.icon !== "" }
                    Glyph { anchors.centerIn: parent; name: "music"; size: 14; visible: streamRow.icon === "" }
                }
                ColumnLayout {
                    Layout.preferredWidth: 110
                    spacing: 0
                    Label { text: Audio.streamName(streamRow.modelData); size: Theme.sizeSmall; Layout.fillWidth: true }
                    Label { text: Audio.streamDetail(streamRow.modelData); size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true; visible: text !== "" }
                }
                Slider { Layout.fillWidth: true; value: streamRow.muted ? 0 : (streamRow.modelData.audio ? streamRow.modelData.audio.volume : 0); onMoved: v => { Audio.setStreamMuted(streamRow.modelData, false); Audio.setStreamVolume(streamRow.modelData, v); } }
                Glyph { name: streamRow.muted ? "volume-x" : "volume-2"; size: 14; color: streamRow.muted ? Theme.warn : Theme.text3
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Audio.setStreamMuted(streamRow.modelData, !streamRow.muted) } }
            }
        }
    }

    PanelPage {
        visible: root.page === "input"
        Layout.fillWidth: true
        title: "Input"
        onBack: root.page = "main"
        Repeater {
            model: Audio.sources
            ListRow {
                required property var modelData
                Layout.fillWidth: true
                glyph: modelData.name.indexOf("bluez") === 0 ? "headphones" : "mic"
                glyphColor: modelData === Audio.source ? Theme.accent : Theme.text2
                title: modelData.description
                onClicked: Audio.setSource(modelData)
                Glyph { name: "check"; size: 14; color: Theme.accent; visible: modelData === Audio.source }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Theme.s3
            spacing: Theme.s2
            Glyph { name: Audio.sourceMuted ? "mic-off" : "mic"; size: 14; color: Audio.sourceMuted ? Theme.warn : Theme.text2
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Audio.setSourceMuted(!Audio.sourceMuted) } }
            Slider { Layout.fillWidth: true; value: Audio.sourceMuted ? 0 : Audio.sourceVolume; onMoved: v => { Audio.setSourceMuted(false); Audio.setSourceVolume(v); } }
            Label { text: Audio.sourceMuted ? "0" : Math.round(Audio.sourceVolume * 100); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 24; horizontalAlignment: Text.AlignRight }
        }
        Label { visible: Audio.captures.length > 0; text: "Listening: " + Audio.captures.map(c => Audio.streamName(c)).join(", "); size: Theme.sizeCaption; color: Theme.live; Layout.margins: Theme.s3; Layout.topMargin: 0 }
    }
}
