import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The same cards as the island's centre: what is waiting, actions and reply live, earlier ones beneath.
WidgetBase {
    id: root
    title: "Notifications"
    meta: Notifications.silenced ? Notifications.silencedBy : Notifications.count > 0 ? Notifications.count + "" : ""

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            visible: Notifications.count > 0 || Notifications.past.length > 0
            Item { Layout.fillWidth: true }
            Label {
                text: "Clear all"; size: Theme.sizeCaption; color: Theme.accent
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Notifications.clearAll() }
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 0
            boundsBehavior: Flickable.StopAtBounds
            model: Notifications.list.map(n => ({ n: n })).concat(Notifications.past.length ? [{ header: "Earlier" }] : [], Notifications.past.slice(0, 40).map(r => ({ past: r })))
            delegate: Item {
                required property var modelData
                width: ListView.view.width
                height: modelData.header ? 26 : card.implicitHeight + 1
                Label { visible: !!modelData.header; anchors { left: parent.left; bottom: parent.bottom; bottomMargin: 6 } text: modelData.header || ""; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3 }
                NotificationCard {
                    id: card
                    visible: !modelData.header
                    anchors { left: parent.left; right: parent.right }
                    n: modelData.n || null
                    past: modelData.past || null
                    showBody: root.settings.body !== false
                    showPicture: root.settings.pictures !== false
                }
                Rectangle { visible: !modelData.header; anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.hairline }
            }
        }

        Label { Layout.alignment: Qt.AlignHCenter; Layout.fillHeight: true; verticalAlignment: Text.AlignVCenter; visible: Notifications.count === 0 && Notifications.past.length === 0; text: "All clear"; color: Theme.text3 }
    }
}
