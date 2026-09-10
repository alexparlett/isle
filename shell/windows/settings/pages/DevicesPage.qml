import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The machine as a tree: categories the way Device Manager names them, every device under its own, and the
// properties of whatever is selected on the right.
SettingsPage {
    id: root
    title: "Devices"
    subtitle: "Everything attached, by kind. Select a device for its details; the tree follows what is plugged in."

    // The shell's own knowledge joins the script's categories.
    readonly property var categories: {
        const out = Devices.categories.slice();
        out.push({ id: "monitors", name: "Monitors", glyph: "monitor", page: "displays", items: Displays.monitors.map(m => {
            const i = Displays.info(m);
            return { name: ((i.make || "") + " " + (i.model || "")).trim() || m.name,
                     props: [["Connector", m.name], ["Resolution", i.width ? i.width + "×" + i.height + " at " + Math.round(i.refreshRate || 0) + " Hz" : ""], ["Scale", i.scale ? String(i.scale) : ""],
                             ["Variable refresh", i.vrr ? "On" : "Off"], ["Serial", i.serial || ""]].filter(p => p[1] !== "") };
        }) });
        out.push({ id: "audio-io", name: "Audio inputs and outputs", glyph: "volume-2", page: "audio", items:
            Audio.sinks.map(n => ({ name: n.description || n.name, props: [["Kind", "Output"], ["Node", n.name]] }))
            .concat(Audio.sources.map(n => ({ name: n.description || n.name, props: [["Kind", "Input"], ["Node", n.name]] }))) });
        const bt = out.find(c => c.id === "bt-adapters");
        const peers = Bluetooth.paired.map(d => ({ name: d.name || d.address, props: [["Kind", "Device"], ["Address", d.address], ["Status", d.connected ? "Connected" : "Paired"]].concat(d.batteryAvailable ? [["Battery", Math.round(d.battery * 100) + "%"]] : []) }));
        if (bt) { bt.items = bt.items.concat(peers); bt.page = "bluetooth"; }
        else if (peers.length) out.push({ id: "bt-adapters", name: "Bluetooth", glyph: "bluetooth", page: "bluetooth", items: peers });
        out.push({ id: "batteries", name: "Batteries", glyph: "plug-zap", page: "power", items: Power.devices.map(d => ({ name: d.model || "Battery", props: [["Charge", Math.round(d.percentage) + "%"]] })) });
        return out.filter(c => c.items && c.items.length > 0).sort((a, b) => a.name.localeCompare(b.name));
    }
    readonly property int total: categories.reduce((t, c) => t + c.items.length, 0)

    property var open: ({})
    function isOpen(c) { return open[c.id] !== undefined ? open[c.id] : !c.collapsed; }
    function toggle(c) { const o = Object.assign({}, open); o[c.id] = !isOpen(c); open = o; }
    property string selected: ""
    readonly property var current: {
        for (const c of categories) for (let i = 0; i < c.items.length; i++) if (c.id + ":" + i === selected) return { cat: c, item: c.items[i] };
        return null;
    }
    readonly property var rows: {
        const out = [];
        for (const c of categories) {
            out.push({ kind: "cat", cat: c, key: c.id });
            if (isOpen(c)) c.items.forEach((it, i) => out.push({ kind: "item", cat: c, item: it, key: c.id + ":" + i }));
        }
        return out;
    }

    RowLayout {
        Layout.fillWidth: true
        Layout.preferredHeight: 540
        spacing: Theme.s4

        Card {
            Layout.preferredWidth: 340
            Layout.fillHeight: true
            padding: Theme.s2
            ListView {
                id: tree
                anchors.fill: parent
                clip: true
                model: root.rows
                boundsBehavior: Flickable.StopAtBounds
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool cat: modelData.kind === "cat"
                    readonly property bool sel: !cat && root.selected === modelData.key
                    width: tree.width
                    height: 30
                    radius: Theme.radiusControl
                    color: sel ? Theme.pressed : rowArea.containsMouse ? Qt.alpha(Theme.text, 0.04) : "transparent"
                    RowLayout {
                        anchors { fill: parent; leftMargin: cat ? Theme.s2 : Theme.s5 + Theme.s2; rightMargin: Theme.s2 }
                        spacing: Theme.s2
                        Glyph { visible: cat; name: root.isOpen(modelData.cat) ? "chevron-down" : "chevron-right"; size: 12; color: Theme.text3; Layout.preferredWidth: 14 }
                        Glyph { name: modelData.cat.glyph; size: 14; color: cat ? Theme.text2 : Theme.text3; Layout.preferredWidth: 18 }
                        Label { text: cat ? modelData.cat.name : modelData.item.name; weight: cat ? Font.DemiBold : Font.Normal; color: cat ? Theme.text : sel ? Theme.text : Theme.text2; elide: Text.ElideRight; Layout.fillWidth: true }
                        Label { visible: cat; text: modelData.cat.items.length; size: Theme.sizeCaption; color: Theme.text3; tabular: true }
                    }
                    MouseArea { id: rowArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: cat ? root.toggle(modelData.cat) : root.selected = modelData.key }
                }
            }
            Scrollbar { target: tree; anchors { top: parent.top; bottom: parent.bottom; right: parent.right } }
        }

        Card {
            Layout.fillWidth: true
            Layout.fillHeight: true
            padding: Theme.s4
            ColumnLayout {
                anchors { left: parent.left; right: parent.right; top: parent.top }
                spacing: Theme.s3
                RowLayout {
                    spacing: Theme.s3
                    Rectangle {
                        width: 40; height: 40; radius: 12
                        color: Qt.alpha(Theme.accent, 0.14)
                        Glyph { anchors.centerIn: parent; name: root.current ? root.current.cat.glyph : "plug-zap"; size: 18; color: Theme.accent }
                    }
                    ColumnLayout {
                        spacing: 2
                        Label { text: root.current ? root.current.item.name : "This machine"; size: Theme.sizeTitle; weight: Font.DemiBold; elide: Text.ElideRight; Layout.fillWidth: true }
                        Label { text: root.current ? root.current.cat.name : root.total + " devices in " + root.categories.length + " categories"; size: Theme.sizeSmall; color: Theme.text2 }
                    }
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.hairline }
                Repeater {
                    model: root.current ? root.current.item.props : []
                    RowLayout {
                        required property var modelData
                        spacing: Theme.s3
                        Label { text: modelData[0]; color: Theme.text2; size: Theme.sizeSmall; Layout.preferredWidth: 150; Layout.alignment: Qt.AlignTop }
                        Label { text: modelData[1]; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    }
                }
                Label { visible: !root.current; text: "Select a device on the left for its details."; color: Theme.text3; size: Theme.sizeSmall }
                Button {
                    visible: root.current && !!root.current.cat.page
                    text: "Open settings"
                    variant: "raised"
                    Layout.topMargin: Theme.s2
                    onClicked: Surfaces.settingsPage = root.current.cat.page
                }
            }
        }
    }
}
