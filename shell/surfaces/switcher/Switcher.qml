import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// A row of app icons across the centre while the modifier is held; the selected app's name and windows beneath.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Switcher.open

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-switcher"
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    readonly property var app: Switcher.apps[Switcher.index] || null

    ColumnLayout {
        anchors.centerIn: parent
        spacing: Theme.s3

        Glass {
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: row.implicitWidth + Theme.s2 * 2
            implicitHeight: row.implicitHeight + Theme.s2 * 2
            radius: Theme.radiusPanel + 4

            RowLayout {
                id: row
                anchors.centerIn: parent
                spacing: Theme.s2
                Repeater {
                    model: Switcher.apps
                    Rectangle {
                        required property var modelData
                        required property int index
                        readonly property bool sel: index === Switcher.index
                        implicitWidth: 64; implicitHeight: 64
                        radius: Theme.radiusCard + 2
                        color: sel ? Theme.raised : "transparent"
                        border.width: 1
                        border.color: sel ? Theme.hairlineStrong : "transparent"
                        opacity: modelData.hidden ? 0.4 : 1
                        Behavior on color { ColorAnimation { duration: Theme.quick } }
                        AppIcon { anchors.centerIn: parent; size: 48; source: modelData.icon }
                        Rectangle {
                            visible: modelData.windows.length > 1
                            anchors { right: parent.right; bottom: parent.bottom; margins: 6 }
                            width: 16; height: 16; radius: 8
                            color: Theme.pressed
                            Label { anchors.centerIn: parent; text: modelData.windows.length; size: 10; weight: Font.DemiBold; tabular: true }
                        }
                        MouseArea { anchors.fill: parent; hoverEnabled: true; onEntered: Switcher.index = index; onClicked: Switcher.commit() }
                    }
                }
            }
        }

        ColumnLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 2
            visible: root.app !== null
            Label { Layout.alignment: Qt.AlignHCenter; text: root.app ? root.app.name : ""; weight: Font.DemiBold }
            Label {
                Layout.alignment: Qt.AlignHCenter
                Layout.maximumWidth: 520
                text: root.app ? (root.app.windows.length === 1 ? (root.app.windows[0].title || "") : root.app.windows.length + " windows" + (root.app.hidden ? " · hidden" : "")) : ""
                size: Theme.sizeSmall; color: Theme.text2
            }
        }
    }
}
