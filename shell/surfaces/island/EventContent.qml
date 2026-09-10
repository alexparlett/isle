import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// One event in the pill. `ev` is the IslandEvents payload; `kind` picks the layout.
RowLayout {
    id: root
    required property var ev
    readonly property string kind: ev.kind
    spacing: Theme.s2 + 2

    // osd: glyph, bar, value
    Glyph { visible: root.kind === "osd"; name: root.ev.glyph || "volume-2"; size: 14 }
    Slider { visible: root.kind === "osd"; value: root.ev.value || 0; interactive: false; implicitWidth: 120 }
    Label { visible: root.kind === "osd"; text: root.ev.label || ""; mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2 }

    // media: art, title · artist, state glyph
    Rectangle {
        visible: root.kind === "media"
        width: 20; height: 20; radius: 5
        color: Theme.raised
        clip: true
        Image { anchors.fill: parent; source: root.ev.artUrl || ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
        Glyph { anchors.centerIn: parent; name: "music"; size: 12; visible: !root.ev.artUrl }
    }
    Label { visible: root.kind === "media"; text: root.ev.title || ""; Layout.maximumWidth: 220 }
    Label { visible: root.kind === "media" && !!root.ev.artist; text: "·"; color: Theme.text3 }
    Label { visible: root.kind === "media" && !!root.ev.artist; text: root.ev.artist || ""; color: Theme.text2; Layout.maximumWidth: 160 }
    Glyph { visible: root.kind === "media"; name: "pause"; size: 13; weight: 1.6 }

    // recording: live dot, timer, stop
    Rectangle {
        visible: root.kind === "recording"
        width: 8; height: 8; radius: 4; color: Theme.live
        SequentialAnimation on opacity { running: root.kind === "recording"; loops: Animation.Infinite; NumberAnimation { to: 0.3; duration: 700 } NumberAnimation { to: 1; duration: 700 } }
    }
    Label { visible: root.kind === "recording"; text: { const e = root.ev.elapsed || 0; return Math.floor(e / 60).toString().padStart(2, "0") + ":" + (e % 60).toString().padStart(2, "0"); } mono: true; tabular: true; size: Theme.sizeSmall }
    Label { visible: root.kind === "recording"; text: "·"; color: Theme.text3 }
    Label { visible: root.kind === "recording"; text: "Recording"; size: Theme.sizeSmall; color: Theme.text2 }
    Glyph { visible: root.kind === "recording"; name: "square"; size: 13; weight: 1.6; color: Theme.text }

    // drive: name, size, mount or open, eject. The volume's state is read live, not from the event's snapshot.
    readonly property var vol: root.kind === "drive" && root.ev.volume ? Disks.current(root.ev.volume) : null
    Glyph { visible: root.kind === "drive"; name: "hard-drive"; size: 14 }
    Label { visible: root.kind === "drive"; text: root.ev.text || ""; Layout.maximumWidth: 220 }
    Label { visible: root.kind === "drive"; text: root.ev.detail || ""; size: Theme.sizeSmall; color: Theme.text2 }
    Rectangle {
        visible: root.kind === "drive"
        implicitHeight: 20; implicitWidth: mountLabel.implicitWidth + Theme.s2 * 2 + 2
        radius: 10; color: Theme.accent
        Label { id: mountLabel; anchors.centerIn: parent; text: root.vol && root.vol.mounted ? "Open" : "Mount"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.onAccent }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { const v = root.vol; if (v.mounted) { Disks.open(v); IslandEvents.dismiss(); } else Disks.mount(v); } }
    }
    Rectangle {
        visible: root.kind === "drive"
        implicitHeight: 20; implicitWidth: ejectLabel.implicitWidth + Theme.s2 * 2 + 2
        radius: 10; color: Theme.raised; border.width: 1; border.color: Theme.hairline
        Label { id: ejectLabel; anchors.centerIn: parent; text: "Eject"; size: Theme.sizeCaption; weight: Font.DemiBold }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Disks.eject(root.ev.volume) }
    }

    // text: glyph and a line; with actions, the pills go on a second line so the island's width holds.
    Rectangle { visible: root.kind === "text" && !!root.ev.color; width: 14; height: 14; radius: 4; color: root.ev.color || "transparent"; border.width: 1; border.color: Theme.hairlineStrong }
    Glyph { visible: root.kind === "text" && !root.ev.color; name: root.ev.glyph || "check"; size: 14 }
    ColumnLayout {
        visible: root.kind === "text"
        spacing: Theme.s2
        readonly property int lineWidth: Theme.panelWidth - Theme.s4 * 2 - 14 - Theme.s2 - 2
        RowLayout {
            spacing: Theme.s2 + 2
            Label { text: root.ev.text || ""; Layout.maximumWidth: parent.parent.lineWidth; elide: Text.ElideRight }
            Label { visible: !!root.ev.detail; text: root.ev.detail || ""; color: Theme.text2; size: Theme.sizeSmall; elide: Text.ElideMiddle; Layout.maximumWidth: parent.parent.lineWidth - 160 }
        }
        RowLayout {
            visible: !!root.ev.actions && root.ev.actions.length > 0
            spacing: Theme.s2
            Repeater {
                model: root.ev.actions || []
                Rectangle {
                    required property var modelData
                    required property int index
                    implicitHeight: 20; implicitWidth: pillLabel.implicitWidth + Theme.s2 * 2 + 2
                    radius: 10; color: index === 0 ? Theme.accent : Theme.raised; border.width: index === 0 ? 0 : 1; border.color: Theme.hairline
                    Label { id: pillLabel; anchors.centerIn: parent; text: modelData.label; size: Theme.sizeCaption; weight: Font.DemiBold; color: index === 0 ? Theme.onAccent : Theme.text }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { modelData.run(); IslandEvents.dismiss(); } }
                }
            }
        }
    }

    // notification: app icon, summary and age, one line of body
    Item {
        visible: root.kind === "notification"
        Layout.preferredWidth: 30; Layout.preferredHeight: 30
        AppIcon { anchors.fill: parent; size: 30; source: root.ev.icon || ""; visible: !!root.ev.icon }
        Rectangle {
            anchors.fill: parent; radius: 9; color: Theme.raised; visible: !root.ev.icon
            Glyph { anchors.centerIn: parent; name: "bell"; size: 14; color: root.ev.critical ? Theme.warn : Theme.text2 }
        }
    }
    ColumnLayout {
        visible: root.kind === "notification"
        Layout.preferredWidth: 320
        spacing: 1
        readonly property var n: root.ev.notification || null
        readonly property var actions: n ? n.actions.filter(a => a.identifier !== "default").slice(0, 3) : []
        property bool replying: false
        RowLayout {
            Layout.fillWidth: true
            Label { text: root.ev.summary || root.ev.app || ""; weight: Font.DemiBold; Layout.fillWidth: true }
            Label { text: root.ev.app && root.ev.summary && root.ev.app !== root.ev.summary ? root.ev.app : "now"; size: Theme.sizeCaption; color: Theme.text3 }
        }
        Label { text: root.ev.body || ""; size: Theme.sizeSmall; color: Theme.text2; Layout.fillWidth: true; visible: !!root.ev.body && !parent.replying }
        // Actions and reply, as pills under the body: the height of a small button, small text, medium weight.
        RowLayout {
            id: actionRow
            visible: (parent.actions.length > 0 || (parent.n && parent.n.hasInlineReply)) && !parent.replying
            spacing: Theme.s2
            Layout.topMargin: Theme.s1 + 2
            Repeater {
                model: actionRow.parent.actions
                Rectangle {
                    required property var modelData
                    implicitHeight: 24; implicitWidth: actLabel.implicitWidth + Theme.s3 * 2
                    radius: 12; color: actArea.containsMouse ? Theme.pressed : Theme.raised; border.width: 1; border.color: Theme.hairlineStrong
                    Label { id: actLabel; anchors.centerIn: parent; text: modelData.text; size: Theme.sizeSmall; weight: Font.Medium }
                    MouseArea { id: actArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { modelData.invoke(); IslandEvents.dismiss(); } }
                }
            }
            Rectangle {
                visible: actionRow.parent.n && actionRow.parent.n.hasInlineReply
                implicitHeight: 24; implicitWidth: replyLabel.implicitWidth + Theme.s3 * 2
                radius: 12; color: Theme.accent
                Label { id: replyLabel; anchors.centerIn: parent; text: "Reply"; size: Theme.sizeSmall; weight: Font.DemiBold; color: Theme.onAccent }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { actionRow.parent.replying = true; IslandEvents.hold(); } }
            }
        }
        // The image hint, as a small picture under the body.
        Rectangle {
            visible: !!root.ev.picture && !parent.replying
            Layout.preferredWidth: 120; Layout.preferredHeight: 68
            Layout.topMargin: 4
            radius: 8; color: Theme.raised; clip: true
            Image { anchors.fill: parent; source: root.ev.picture || ""; fillMode: Image.PreserveAspectCrop; asynchronous: true }
        }
        Field {
            visible: parent.replying
            Layout.fillWidth: true
            Layout.topMargin: 2
            implicitHeight: 28
            size: Theme.sizeSmall
            placeholder: parent.n && parent.n.inlineReplyPlaceholder ? parent.n.inlineReplyPlaceholder : "Reply"
            onVisibleChanged: if (visible) input.forceActiveFocus()
            onAccepted: { if (text !== "" && parent.n) parent.n.sendInlineReply(text); parent.replying = false; IslandEvents.dismiss(); }
            input.Keys.onEscapePressed: { parent.replying = false; IslandEvents.dismiss(); }
        }
    }
}
