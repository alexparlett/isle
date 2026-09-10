import QtQuick
import QtQuick.Layouts
import qs.theme

// A number input with a unit: commits on Enter or when focus leaves, clamped to [from, to].
Rectangle {
    id: root
    property int value: 0
    property int from: 0
    property int to: 999999
    property string unit: ""
    signal committed(int v)

    implicitHeight: 32
    implicitWidth: 64 + (unitLabel.visible ? unitLabel.implicitWidth + Theme.s2 : 0) + Theme.s3 * 2
    radius: Theme.radiusControl
    color: Theme.raised
    border.width: 1
    border.color: input.activeFocus ? Theme.accent : Theme.hairline

    function commit() {
        const n = Math.max(from, Math.min(to, parseInt(input.text) || 0));
        input.text = String(n);
        if (n !== root.value) root.committed(n);
    }
    onValueChanged: if (!input.activeFocus) input.text = String(value)

    RowLayout {
        anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
        spacing: Theme.s2
        TextInput {
            id: input
            Layout.fillWidth: true
            text: String(root.value)
            color: Theme.text
            font.family: Theme.fontMono
            font.pixelSize: Theme.sizeSmall
            font.weight: Font.Medium
            horizontalAlignment: Text.AlignRight
            selectionColor: Theme.accent
            selectedTextColor: Theme.onAccent
            validator: IntValidator { bottom: root.from; top: root.to }
            renderType: Text.NativeRendering
            onAccepted: root.commit()
            onActiveFocusChanged: if (!activeFocus) root.commit()
        }
        Label { id: unitLabel; visible: root.unit !== ""; text: root.unit; size: Theme.sizeCaption; color: Theme.text3 }
    }
}
