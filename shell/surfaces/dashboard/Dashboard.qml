import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// The dashboard: full screen over the dimmed wallpaper, a 12-column grid of widgets. Escape or the backdrop closes it.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Surfaces.dashboard

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-dashboard"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    readonly property int margin: Theme.s5
    readonly property int gutter: Theme.s3
    readonly property int columns: 12
    readonly property int rows: 4
    readonly property real cellW: (width - margin * 2 - gutter * (columns - 1)) / columns
    readonly property real cellH: (height - margin * 2 - gutter * (rows - 1)) / rows

    // Edit mode: drag to move, the corner to resize, × to remove, + to add. E toggles it.
    property bool editing: false
    onVisibleChanged: if (!visible) editing = false
    function cellAt(px, py) { return { x: Math.round((px - margin) / (cellW + gutter)), y: Math.round((py - margin) / (cellH + gutter)) }; }

    Rectangle {
        id: backdrop
        anchors.fill: parent
        color: Qt.alpha(Theme.ink, 0.55)
        opacity: root.visible ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.move } }
        MouseArea { anchors.fill: parent; onClicked: Surfaces.dashboard = false }
    }

    Item {
        id: grid
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            if (event.key === Qt.Key_Escape) { if (root.editing) root.editing = false; else Surfaces.dashboard = false; }
            else if (event.key === Qt.Key_E) root.editing = !root.editing;
            else return;
            event.accepted = true;
        }

        Repeater {
            model: Widgets.layout
            WidgetCard {
                required property var modelData
                required property int index
                entry: modelData
                x: root.margin + modelData.x * (root.cellW + root.gutter)
                y: root.margin + modelData.y * (root.cellH + root.gutter)
                width: modelData.w * root.cellW + (modelData.w - 1) * root.gutter
                height: modelData.h * root.cellH + (modelData.h - 1) * root.gutter
                editing: root.editing
                dashboard: root
                // Enter: fade and rise, staggered by row.
                opacity: root.visible ? 1 : 0
                transform: Translate { y: root.visible ? 0 : 12 + modelData.y * 4 }
                Behavior on opacity { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
                Behavior on x { enabled: !dragging; NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
                Behavior on y { enabled: !dragging; NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
            }
        }

        // The edit toggle, and in edit mode the add row and reset.
        Rectangle {
            anchors { right: parent.right; bottom: parent.bottom; margins: root.margin }
            width: editRow.implicitWidth + Theme.s3; height: 32; radius: 16
            color: root.editing ? Theme.accent : Qt.alpha(Theme.glass, 0.9)
            border.width: 1; border.color: root.editing ? "transparent" : Theme.hairline
            z: 10
            RowLayout {
                id: editRow
                anchors.centerIn: parent
                spacing: Theme.s2
                Glyph { name: root.editing ? "check" : "pencil"; size: 14; color: root.editing ? Theme.onAccent : Theme.text2 }
                Label { text: root.editing ? "Done" : "Edit"; size: Theme.sizeSmall; weight: Font.DemiBold; color: root.editing ? Theme.onAccent : Theme.text2 }
            }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.editing = !root.editing }
        }
        // The layout as it was when editing began, for Discard.
        property var before: null
        Connections { target: root; function onEditingChanged() { if (root.editing) grid.before = (Prefs.p.dashboard || []).slice(); } }
        Rectangle {
            visible: root.editing
            anchors { right: parent.right; bottom: parent.bottom; margins: root.margin; rightMargin: root.margin + 96 }
            width: discardRow.implicitWidth + Theme.s3; height: 32; radius: 16
            color: Qt.alpha(Theme.glass, 0.9); border.width: 1; border.color: Theme.hairline
            z: 10
            RowLayout { id: discardRow; anchors.centerIn: parent; spacing: Theme.s2
                Glyph { name: "x"; size: 14; color: Theme.text2 }
                Label { text: "Discard"; size: Theme.sizeSmall; weight: Font.DemiBold; color: Theme.text2 } }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { Prefs.p.dashboard = grid.before || []; root.editing = false; } }
        }
        Glass {
            visible: root.editing
            anchors { left: parent.left; bottom: parent.bottom; margins: root.margin }
            width: addRow.implicitWidth + Theme.s3 * 2; height: 36; radius: 18
            z: 10
            RowLayout {
                id: addRow
                anchors.centerIn: parent
                spacing: Theme.s2
                Label { text: Widgets.unplaced.length ? "Add" : "Every widget is placed"; size: Theme.sizeSmall; color: Theme.text2 }
                Repeater {
                    model: Widgets.unplaced
                    Rectangle {
                        required property string modelData
                        implicitHeight: 24; implicitWidth: addLabel.implicitWidth + Theme.s2 * 2 + 4
                        radius: 12; color: Theme.raised; border.width: 1; border.color: Theme.hairline
                        Label { id: addLabel; anchors.centerIn: parent; text: Widgets.manifests[modelData] ? Widgets.manifests[modelData].name : modelData; size: Theme.sizeCaption; weight: Font.DemiBold }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.add(modelData) }
                    }
                }
                Label { text: "·"; color: Theme.text3 }
                Label { text: "Reset layout"; size: Theme.sizeCaption; color: Theme.accent
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.resetLayout() } }
            }
        }
    }
}
