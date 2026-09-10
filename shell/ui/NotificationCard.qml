import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.services

// One notification, wherever it is listed: icon, summary, app and age, body, picture, then the actions and
// reply as pills, still live from history. `n` is a live notification; `past` a record kept from before
// a restart, which reads the same but has no actions. Left click activates, right click dismisses.
Item {
    id: card
    property var n: null
    property var past: null
    property bool showBody: true
    property bool showPicture: true
    property bool replying: false
    signal done
    readonly property string app: n ? n.appName : past ? past.app : ""
    readonly property string summary: n ? n.summary : past ? past.summary : ""
    readonly property string body: Notifications.plain(n ? n.body : past ? past.body : "")
    readonly property string icon: n ? Notifications.iconFor(n) : past ? past.icon || "" : ""
    readonly property string picture: n && showPicture ? Notifications.pictureFor(n) : ""
    readonly property string age: n ? Notifications.age(n) : past ? Notifications.ageOf(past.time) : ""
    readonly property var actions: n ? n.actions.filter(a => a.identifier !== "default").slice(0, 3) : []
    readonly property bool canReply: !!(n && n.hasInlineReply)

    implicitHeight: row.implicitHeight + Theme.s2 * 2
    RowLayout {
        id: row
        anchors { left: parent.left; right: parent.right; top: parent.top; topMargin: Theme.s2 }
        spacing: Theme.s2 + 2
        Item {
            Layout.preferredWidth: 26; Layout.preferredHeight: 26
            Layout.alignment: Qt.AlignTop
            AppIcon { anchors.fill: parent; size: 26; source: card.icon; visible: card.icon !== "" }
            Rectangle { anchors.fill: parent; radius: 8; color: Theme.raised; visible: card.icon === ""
                Glyph { anchors.centerIn: parent; name: "bell"; size: 14 } }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 1
            RowLayout {
                Layout.fillWidth: true
                Label { text: card.summary || card.app; weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                Label { text: card.app && card.summary && card.app !== card.summary ? card.app + " · " + card.age : card.age; size: Theme.sizeCaption; color: Theme.text3 }
                Rectangle {
                    visible: !!card.n && hover.containsMouse
                    width: 18; height: 18; radius: 9; color: closeArea.containsMouse ? Theme.pressed : "transparent"
                    Glyph { anchors.centerIn: parent; name: "x"; size: 9; color: Theme.text2 }
                    MouseArea { id: closeArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { if (card.n) Notifications.dismiss(card.n); card.done(); } }
                }
            }
            Label { visible: card.showBody && card.body !== "" && !card.replying; text: card.body; size: Theme.sizeSmall; color: Theme.text2; Layout.fillWidth: true; wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight }
            RowLayout {
                visible: (card.actions.length > 0 || card.canReply) && !card.replying
                spacing: Theme.s2
                Layout.topMargin: Theme.s1
                Repeater {
                    model: card.actions
                    Rectangle {
                        required property var modelData
                        implicitHeight: 24; implicitWidth: actLabel.implicitWidth + Theme.s3 * 2
                        radius: 12; color: actArea.containsMouse ? Theme.pressed : Theme.raised; border.width: 1; border.color: Theme.hairlineStrong
                        Label { id: actLabel; anchors.centerIn: parent; text: modelData.text; size: Theme.sizeSmall; weight: Font.Medium }
                        MouseArea { id: actArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { modelData.invoke(); card.done(); } }
                    }
                }
                Rectangle {
                    visible: card.canReply
                    implicitHeight: 24; implicitWidth: replyLabel.implicitWidth + Theme.s3 * 2
                    radius: 12; color: Theme.accent
                    Label { id: replyLabel; anchors.centerIn: parent; text: "Reply"; size: Theme.sizeSmall; weight: Font.DemiBold; color: Theme.onAccent }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: card.replying = true }
                }
            }
            Field {
                visible: card.replying
                Layout.fillWidth: true
                Layout.topMargin: 2
                implicitHeight: 28
                size: Theme.sizeSmall
                placeholder: card.n && card.n.inlineReplyPlaceholder ? card.n.inlineReplyPlaceholder : "Reply"
                onVisibleChanged: if (visible) { text = ""; input.forceActiveFocus(); }
                onAccepted: { if (text !== "" && card.n) card.n.sendInlineReply(text); card.replying = false; card.done(); }
                input.Keys.onEscapePressed: card.replying = false
            }
        }
        Rectangle {
            visible: card.picture !== "" && !card.replying
            Layout.preferredWidth: 48; Layout.preferredHeight: 48
            Layout.alignment: Qt.AlignTop
            radius: 8; color: Theme.raised; clip: true
            Image { anchors.fill: parent; source: card.picture; fillMode: Image.PreserveAspectCrop; asynchronous: true }
        }
    }
    MouseArea {
        id: hover
        anchors.fill: parent
        z: -1
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        cursorShape: card.n ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: mouse => {
            if (!card.n) return;
            if (mouse.button === Qt.LeftButton) Notifications.activate(card.n); else Notifications.dismiss(card.n);
            card.done();
        }
    }
}
