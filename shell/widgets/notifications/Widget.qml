import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

WidgetBase {
    id: root
    title: "Notifications"
    meta: Notifications.count > 0 ? Notifications.count + "" : ""

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
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: Notifications.list
            spacing: 0
            delegate: Item {
                required property var modelData
                required property int index
                width: ListView.view.width
                height: row.implicitHeight + Theme.s2 * 2 + 1

                RowLayout {
                    id: row
                    anchors { left: parent.left; right: parent.right; top: parent.top; topMargin: Theme.s2 }
                    spacing: Theme.s2 + 2
                    Item {
                        Layout.preferredWidth: 26; Layout.preferredHeight: 26
                        Layout.alignment: Qt.AlignTop
                        readonly property string icon: Notifications.iconFor(modelData)
                        AppIcon { anchors.fill: parent; size: 26; source: parent.icon; visible: parent.icon !== "" }
                        Rectangle { anchors.fill: parent; radius: 8; color: Theme.raised; visible: parent.icon === ""
                            Glyph { anchors.centerIn: parent; name: "bell"; size: 14 } }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 1
                        RowLayout {
                            Layout.fillWidth: true
                            Label { text: modelData.summary || modelData.appName; weight: Font.DemiBold; Layout.fillWidth: true }
                            Label { text: Notifications.age(modelData); size: Theme.sizeCaption; color: Theme.text3 }
                        }
                        Label { text: Notifications.plain(modelData.body); size: Theme.sizeSmall; color: Theme.text2; Layout.fillWidth: true; wrapMode: Text.WordWrap; maximumLineCount: 2; visible: text !== "" && root.settings.body !== false }
                    }
                    // The image hint, when the app has an icon of its own.
                    Rectangle {
                        readonly property string picture: root.settings.pictures !== false ? Notifications.pictureFor(modelData) : ""
                        visible: picture !== ""
                        Layout.preferredWidth: 48; Layout.preferredHeight: 48
                        Layout.alignment: Qt.AlignTop
                        radius: 8; color: Theme.raised; clip: true
                        Image { anchors.fill: parent; source: parent.picture; fillMode: Image.PreserveAspectCrop; asynchronous: true }
                    }
                }
                Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.hairline; visible: index < Notifications.count - 1 }
                MouseArea {
                    anchors.fill: parent
                    z: -1
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    cursorShape: Qt.PointingHandCursor
                    onClicked: mouse => mouse.button === Qt.LeftButton ? Notifications.activate(modelData) : Notifications.dismiss(modelData)
                }
            }
        }

        Label { Layout.alignment: Qt.AlignHCenter; Layout.fillHeight: true; verticalAlignment: Text.AlignVCenter; visible: Notifications.count === 0; text: "All clear"; color: Theme.text3 }
    }
}
