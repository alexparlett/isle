import QtQuick
import qs.theme

// A raised container with a hairline, card radius, and 12px padding around `contentItem`.
Rectangle {
    id: root
    default property alias content: inner.data
    property int padding: Theme.s3

    implicitHeight: inner.childrenRect.height + padding * 2
    radius: Theme.radiusCard
    color: Theme.raised
    border.width: 1
    border.color: Theme.hairline

    Item {
        id: inner
        anchors { fill: parent; margins: root.padding }
    }
}
