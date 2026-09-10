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
    // Editing keeps a strip at the bottom for the toolbar, so nothing sits over a card.
    readonly property int toolbarH: editing ? 44 + gutter : 0
    readonly property real cellH: (height - margin * 2 - toolbarH - gutter * (rows - 1)) / rows

    // Edit mode: drag to move, the corner to resize, × to remove, an empty cell or Add for the picker. E toggles it.
    property bool editing: false
    onVisibleChanged: if (!visible) editing = false
    // The picker, and the cell it will fill: {x, y} or null for the first free spot.
    property var picking: null
    // Cells no widget covers, as {x, y}.
    readonly property var emptyCells: {
        const out = [];
        for (let y = 0; y < rows; y++) for (let x = 0; x < columns; x++)
            if (!Widgets.layout.some(e => x >= e.x && x < e.x + e.w && y >= e.y && y < e.y + e.h)) out.push({ x: x, y: y });
        return out;
    }
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
            if (event.key === Qt.Key_Escape) { if (root.picking) root.picking = null; else if (root.editing) root.editing = false; else Surfaces.dashboard = false; }
            else if (event.key === Qt.Key_E) root.editing = !root.editing;
            else return;
            event.accepted = true;
        }

        // Empty cells in edit mode: a dashed tile with a plus, the way a free slot invites a widget.
        Repeater {
            model: root.editing ? root.emptyCells : []
            Rectangle {
                required property var modelData
                x: root.margin + modelData.x * (root.cellW + root.gutter)
                y: root.margin + modelData.y * (root.cellH + root.gutter)
                width: root.cellW; height: root.cellH
                radius: Theme.radiusCard
                color: cellArea.containsMouse ? Qt.alpha(Theme.text, 0.06) : Qt.alpha(Theme.text, 0.025)
                border.width: 1; border.color: Qt.alpha(Theme.text, cellArea.containsMouse ? 0.3 : 0.12)
                Glyph { anchors.centerIn: parent; name: "plus"; size: 18; color: Qt.alpha(Theme.text, cellArea.containsMouse ? 0.8 : 0.35) }
                MouseArea { id: cellArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.picking = { x: parent.modelData.x, y: parent.modelData.y } }
            }
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

        // Edit, alone when viewing; in edit mode the strip holds Add, Reset layout, Discard and Done.
        Rectangle {
            visible: !root.editing
            anchors { right: parent.right; bottom: parent.bottom; margins: root.margin }
            width: editRow.implicitWidth + Theme.s3; height: 32; radius: 16
            color: Qt.alpha(Theme.glass, 0.9)
            border.width: 1; border.color: Theme.hairline
            z: 10
            RowLayout {
                id: editRow
                anchors.centerIn: parent
                spacing: Theme.s2
                Glyph { name: "pencil"; size: 14; color: Theme.text2 }
                Label { text: "Edit"; size: Theme.sizeSmall; weight: Font.DemiBold; color: Theme.text2 }
            }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.editing = true }
        }
        // The layout as it was when editing began, for Discard.
        property var before: null
        Connections { target: root; function onEditingChanged() { if (root.editing) grid.before = (Prefs.p.dashboard || []).slice(); } }
        RowLayout {
            visible: root.editing
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: root.margin; rightMargin: root.margin; bottomMargin: root.margin }
            height: 44
            spacing: Theme.s2
            z: 10
            Button { text: "Add widget"; glyph: "plus"; variant: "accent"; enabled: Widgets.unplaced.length > 0; onClicked: root.picking = { x: -1, y: -1 } }
            Label { text: Widgets.unplaced.length ? Widgets.unplaced.length + " to add" : "Every widget is placed"; size: Theme.sizeSmall; color: Theme.text3 }
            Item { Layout.fillWidth: true }
            Label { text: "Drag to move · corner to resize · × removes"; size: Theme.sizeCaption; color: Theme.text3 }
            Item { Layout.fillWidth: true }
            Button { text: "Reset layout"; variant: "text"; onClicked: Widgets.resetLayout() }
            Button { text: "Discard"; variant: "text"; onClicked: { Prefs.p.dashboard = grid.before || []; root.editing = false; } }
            Button { text: "Done"; glyph: "check"; variant: "raised"; onClicked: root.editing = false }
        }

        // The picker: every widget not yet placed, with its glyph, name and size.
        Item {
            visible: root.picking !== null
            anchors.fill: parent
            z: 20
            MouseArea { anchors.fill: parent; onClicked: root.picking = null }
            Glass {
                anchors.centerIn: parent
                width: 520
                height: pickCol.implicitHeight + Theme.s4 * 2
                radius: Theme.radiusPanel
                MouseArea { anchors.fill: parent }
                ColumnLayout {
                    id: pickCol
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
                    spacing: Theme.s3
                    Label { text: "Add a widget"; size: Theme.sizeHeading; weight: Font.DemiBold }
                    Label { visible: Widgets.unplaced.length === 0; text: "Every widget is on the dashboard."; color: Theme.text2 }
                    GridLayout {
                        Layout.fillWidth: true
                        columns: 3
                        columnSpacing: Theme.s2; rowSpacing: Theme.s2
                        Repeater {
                            model: Widgets.unplaced
                            Rectangle {
                                required property string modelData
                                readonly property var manifest: Widgets.manifests[modelData] || ({})
                                Layout.fillWidth: true
                                implicitHeight: 88
                                radius: Theme.radiusCard
                                color: pickArea.containsMouse ? Theme.pressed : Theme.raised
                                border.width: 1; border.color: Theme.hairline
                                ColumnLayout {
                                    anchors.centerIn: parent
                                    spacing: 4
                                    Glyph { Layout.alignment: Qt.AlignHCenter; name: manifest.glyph || "layout-grid"; size: 22; color: Theme.text }
                                    Label { Layout.alignment: Qt.AlignHCenter; text: manifest.name || modelData; weight: Font.DemiBold; size: Theme.sizeSmall }
                                    Label { Layout.alignment: Qt.AlignHCenter; text: (manifest.default || "3x1").replace("x", " × "); size: Theme.sizeCaption; color: Theme.text3 }
                                }
                                MouseArea {
                                    id: pickArea
                                    anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        const at = root.picking;
                                        if (at && at.x >= 0) Widgets.addAt(parent.modelData, at.x, at.y); else Widgets.add(parent.modelData);
                                        root.picking = null;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
