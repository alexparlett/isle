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
            visible: Notifications.count > 0
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
            // Newest first, whether its toast is still up or not: both are waiting to be read.
            model: Notifications.list.map(n => ({ n: n })).concat(Notifications.past.slice(0, 40).map(r => ({ past: r })))
            delegate: Item {
                required property var modelData
                width: ListView.view.width
                height: card.implicitHeight + 1
                NotificationCard {
                    id: card
                    anchors { left: parent.left; right: parent.right }
                    n: modelData.n || null
                    past: modelData.past || null
                    showBody: root.settings.body !== false
                    showPicture: root.settings.pictures !== false
                }
                Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.hairline }
            }
        }

        Label { Layout.alignment: Qt.AlignHCenter; Layout.fillHeight: true; verticalAlignment: Text.AlignVCenter; visible: Notifications.count === 0; text: "All clear"; color: Theme.text3 }
    }
}
