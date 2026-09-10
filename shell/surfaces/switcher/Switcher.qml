import QtQuick
import QtQuick.Layouts
import QtQuick.Effects
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// A row of live window previews across the centre while the modifier is held: one card per window, showing
// its front window, the app's icon and name beneath.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Switcher.open

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-switcher"
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    readonly property int boxW: 240
    readonly property int boxH: 150
    // Cards wrap when the row would not fit in three quarters of the screen.
    readonly property int perRow: Math.max(1, Math.floor((width * 0.75 - Theme.s3 * 2) / (boxW + Theme.s3 * 2 + Theme.s2)))

    Glass {
        anchors.centerIn: parent
        implicitWidth: grid.implicitWidth + Theme.s3 * 2
        implicitHeight: grid.implicitHeight + Theme.s3 * 2
        radius: Theme.radiusPanel + 4

        GridLayout {
            id: grid
            anchors.centerIn: parent
            columns: Math.min(root.perRow, Math.max(1, Switcher.items.length))
            columnSpacing: Theme.s2
            rowSpacing: Theme.s2
            Repeater {
                model: Switcher.items
                Rectangle {
                    id: card
                    required property var modelData
                    required property int index
                    readonly property bool sel: index === Switcher.index
                    readonly property var front: modelData.win
                    implicitWidth: root.boxW + Theme.s3 * 2
                    implicitHeight: root.boxH + Theme.s3 * 2 + 28
                    radius: Theme.radiusCard + 2
                    color: sel ? Theme.raised : "transparent"
                    border.width: 1
                    border.color: sel ? Theme.hairlineStrong : "transparent"
                    Behavior on color { ColorAnimation { duration: Theme.quick } }

                    Item {
                        id: box
                        anchors { top: parent.top; horizontalCenter: parent.horizontalCenter; topMargin: Theme.s3 }
                        width: root.boxW; height: root.boxH
                        // The preview keeps the window's shape inside the box; without a frame yet, the icon stands in.
                        Item {
                            id: picture
                            anchors.centerIn: parent
                            width: view.hasContent ? view.implicitWidth : root.boxW
                            height: view.hasContent ? view.implicitHeight : root.boxH
                            layer.enabled: true
                            layer.effect: MultiEffect { maskEnabled: true; maskSource: mask; maskThresholdMin: 0.5; maskSpreadAtMin: 1 }
                            Rectangle { anchors.fill: parent; color: Theme.pressed; visible: !view.hasContent; AppIcon { anchors.centerIn: parent; size: 56; source: card.modelData.icon } }
                            ScreencopyView {
                                id: view
                                anchors.centerIn: parent
                                constraintSize: Qt.size(root.boxW, root.boxH)
                                captureSource: card.front ? card.front.toplevel.wayland : null
                                live: root.visible
                                paintCursor: false
                            }
                        }
                        Item {
                            id: mask
                            anchors.fill: picture
                            layer.enabled: true
                            visible: false
                            Rectangle { anchors.fill: parent; radius: 8 }
                        }
                        Rectangle {
                            anchors.fill: picture
                            radius: 8
                            color: "transparent"
                            border.width: card.sel ? 2 : 1
                            border.color: card.sel ? Theme.accent : Theme.hairlineStrong
                        }
                    }
                    RowLayout {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: Theme.s3; rightMargin: Theme.s3; bottomMargin: Theme.s2 + 2 }
                        spacing: Theme.s2
                        AppIcon { size: 18; source: card.modelData.icon }
                        // The shell's own windows go by their titles: Settings, not Quickshell.
                        Label { text: card.modelData.appId === "org.quickshell" && card.front ? card.front.title : card.modelData.name; weight: Font.DemiBold; size: Theme.sizeSmall; elide: Text.ElideRight; Layout.fillWidth: true; color: card.sel ? Theme.text : Theme.text2 }
                    }
                    MouseArea { anchors.fill: parent; hoverEnabled: true; onEntered: Switcher.index = card.index; onClicked: Switcher.commit() }
                }
            }
        }
    }

    // The selected window's title, and what the held keys do, under the cards.
    ColumnLayout {
        anchors { horizontalCenter: parent.horizontalCenter; top: parent.verticalCenter; topMargin: (grid.implicitHeight + Theme.s3 * 2) / 2 + Theme.s3 }
        spacing: 2
        visible: Switcher.items.length > 0
        Label {
            Layout.alignment: Qt.AlignHCenter
            Layout.maximumWidth: 640
            elide: Text.ElideRight
            readonly property var it: Switcher.items[Switcher.index] || null
            text: it ? (it.win.title || it.name) : ""
            size: Theme.sizeSmall; color: Theme.text2
        }
        Label { Layout.alignment: Qt.AlignHCenter; text: "Tab next  ·  ` this app's windows  ·  W close window  ·  Q quit  ·  M hide"; size: Theme.sizeCaption; color: Theme.text3 }
    }
}
