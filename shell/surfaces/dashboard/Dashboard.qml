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
    // Editing gives the library a column on the right and the toolbar a strip at the bottom, so nothing sits
    // over a card; the grid animates to the room that is left.
    property int libraryW: editing ? 340 + gutter : 0
    property int toolbarH: editing ? 44 + gutter : 0
    readonly property real cellW: (width - margin * 2 - libraryW - gutter * (columns - 1)) / columns
    readonly property real cellH: (height - margin * 2 - toolbarH - gutter * (rows - 1)) / rows
    Behavior on libraryW { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
    Behavior on toolbarH { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }

    // Edit mode: drag to move, the corner to resize, × to remove; the library adds, by click or by drag. E toggles it.
    property bool editing: false
    onVisibleChanged: if (!visible) editing = false
    onEditingChanged: { pending = null; if (editing) Widgets.rescan(); }
    // A marked empty cell, {x, y}: the next widget picked from the library lands there.
    property var pending: null
    // Why the last placement failed, shown in the toolbar for a moment.
    property string notice: ""
    Timer { id: noticeTimer; interval: 4000; onTriggered: root.notice = "" }
    function place(id, x, y) {
        if (Widgets.addAt(id, x, y)) { notice = ""; return; }
        const m = Widgets.manifests[id];
        notice = "No room for " + (m ? m.name : id) + ": it needs " + (m ? (m.default || m.sizes[0]).replace("x", " × ") : "") + " free cells";
        noticeTimer.restart();
    }
    // A library row being dragged: its id and the pointer, in window coordinates.
    property string dragId: ""
    property real dragX: 0
    property real dragY: 0
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
            if (event.key === Qt.Key_Escape) { if (root.pending) root.pending = null; else if (root.editing) root.editing = false; else Surfaces.dashboard = false; }
            else if (event.key === Qt.Key_E) root.editing = !root.editing;
            else return;
            event.accepted = true;
        }

        // Empty cells in edit mode: a dashed tile with a plus, the way a free slot invites a widget.
        Repeater {
            model: root.editing ? root.emptyCells : []
            Rectangle {
                required property var modelData
                readonly property bool marked: root.pending && root.pending.x === modelData.x && root.pending.y === modelData.y
                readonly property bool under: root.dragId !== "" && root.dragX >= x && root.dragX < x + width && root.dragY >= y && root.dragY < y + height
                x: root.margin + modelData.x * (root.cellW + root.gutter)
                y: root.margin + modelData.y * (root.cellH + root.gutter)
                width: root.cellW; height: root.cellH
                radius: Theme.radiusCard
                color: marked || under ? Qt.alpha(Theme.accent, 0.12) : cellArea.containsMouse ? Qt.alpha(Theme.text, 0.06) : Qt.alpha(Theme.text, 0.025)
                border.width: 1; border.color: marked || under ? Theme.accent : Qt.alpha(Theme.text, cellArea.containsMouse ? 0.3 : 0.12)
                Glyph { anchors.centerIn: parent; name: "plus"; size: 18; color: marked || under ? Theme.accent : Qt.alpha(Theme.text, cellArea.containsMouse ? 0.8 : 0.35) }
                MouseArea { id: cellArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.pending = parent.marked ? null : { x: parent.modelData.x, y: parent.modelData.y } }
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
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: root.margin; rightMargin: root.margin + root.libraryW; bottomMargin: root.margin }
            height: 44
            spacing: Theme.s2
            z: 10
            Label { text: root.notice || (root.pending ? "Pick a widget from the library for the marked cell" : "Click or drag a widget from the library; an empty cell marks where it goes"); size: Theme.sizeSmall; color: root.notice ? Theme.warn : Theme.text2 }
            Item { Layout.fillWidth: true }
            Label { text: "Drag to move · corner to resize · × removes"; size: Theme.sizeCaption; color: Theme.text3 }
            Item { Layout.fillWidth: true }
            Button { text: "Reset layout"; variant: "text"; onClicked: Widgets.resetLayout() }
            Button { text: "Discard"; variant: "text"; onClicked: { Prefs.p.dashboard = grid.before || []; root.editing = false; } }
            Button { text: "Done"; glyph: "check"; variant: "raised"; onClicked: root.editing = false }
        }

        // The library: every widget by category, searchable. A click places it (at the marked cell, else the
        // first free spot); a drag drops it on a cell.
        Glass {
            id: library
            visible: root.editing || x < root.width
            x: root.editing ? root.width - root.margin - 340 : root.width
            y: root.margin
            width: 340
            height: root.height - root.margin * 2
            radius: Theme.radiusPanel
            z: 10
            Behavior on x { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
            MouseArea { anchors.fill: parent }
            property string query: ""
            readonly property var rows: {
                const q = query.trim().toLowerCase();
                return Widgets.library.filter(m => !q || m.name.toLowerCase().indexOf(q) >= 0 || (m.description || "").toLowerCase().indexOf(q) >= 0 || m.category.toLowerCase().indexOf(q) >= 0);
            }
            ColumnLayout {
                anchors { fill: parent; margins: Theme.s3 }
                spacing: Theme.s2
                RowLayout {
                    Layout.fillWidth: true
                    Label { text: "Widgets"; size: Theme.sizeHeading; weight: Font.DemiBold; Layout.fillWidth: true }
                    Label { text: Widgets.library.length; size: Theme.sizeCaption; color: Theme.text3; tabular: true }
                }
                Field { Layout.fillWidth: true; implicitHeight: 32; glyph: "search"; placeholder: "Search widgets"; size: Theme.sizeSmall; onTextChanged: library.query = text; onVisibleChanged: if (visible) text = "" }
                ListView {
                    id: list
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    model: library.rows
                    spacing: 2
                    boundsBehavior: Flickable.StopAtBounds
                    section.property: "category"
                    section.delegate: Label { required property string section; text: section; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3; topPadding: Theme.s2; bottomPadding: 4; leftPadding: Theme.s1 }
                    delegate: Rectangle {
                        id: row
                        required property var modelData
                        readonly property int count: Widgets.placed(modelData.id)
                        readonly property bool addable: Widgets.canAdd(modelData.id)
                        width: list.width
                        height: 54
                        radius: Theme.radiusControl
                        color: rowArea.containsMouse && addable ? Qt.alpha(Theme.text, 0.05) : "transparent"
                        opacity: addable ? 1 : 0.55
                        RowLayout {
                            anchors { fill: parent; leftMargin: Theme.s2; rightMargin: Theme.s2 }
                            spacing: Theme.s2 + 2
                            Rectangle {
                                width: 34; height: 34; radius: 10
                                color: Theme.pressed
                                Glyph { anchors.centerIn: parent; name: row.modelData.glyph || "layout-grid"; size: 16; color: Theme.text }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Label { text: row.modelData.name; weight: Font.DemiBold; size: Theme.sizeSmall; elide: Text.ElideRight; Layout.fillWidth: true }
                                Label { text: row.modelData.description || (row.modelData.sizes || []).map(s => s.replace("x", "×")).join(" · "); size: Theme.sizeCaption; color: Theme.text3; elide: Text.ElideRight; Layout.fillWidth: true }
                            }
                            Label { visible: row.count > 0; text: row.count > 1 ? "×" + row.count : "on"; size: Theme.sizeCaption; color: Theme.accent; tabular: true }
                            Glyph { visible: row.count > 0; name: "check"; size: 12; color: Theme.accent }
                            Glyph { visible: row.count === 0; name: "plus"; size: 14; color: Theme.text3 }
                        }
                        MouseArea {
                            id: rowArea
                            anchors.fill: parent
                            hoverEnabled: true
                            enabled: row.addable
                            cursorShape: Qt.PointingHandCursor
                            property real px: 0
                            property real py: 0
                            property bool moved: false
                            onPressed: mouse => { px = mouse.x; py = mouse.y; moved = false; }
                            onPositionChanged: mouse => {
                                if (!pressed) return;
                                if (!moved && Math.abs(mouse.x - px) + Math.abs(mouse.y - py) < 8) return;
                                moved = true;
                                const p = mapToItem(null, mouse.x, mouse.y);
                                root.dragId = row.modelData.id; root.dragX = p.x; root.dragY = p.y;
                            }
                            onReleased: mouse => {
                                if (moved) {
                                    const p = mapToItem(null, mouse.x, mouse.y);
                                    const cell = root.cellAt(p.x - root.cellW / 2, p.y - root.cellH / 2);
                                    if (p.x < root.width - root.libraryW - root.margin) root.place(row.modelData.id, cell.x, cell.y);
                                } else {
                                    root.place(row.modelData.id, root.pending ? root.pending.x : -1, root.pending ? root.pending.y : -1);
                                }
                                root.pending = null; root.dragId = ""; moved = false;
                            }
                            onCanceled: { root.dragId = ""; moved = false; }
                        }
                    }
                }
                Label { visible: library.rows.length === 0; text: "Nothing matches"; size: Theme.sizeSmall; color: Theme.text3 }
                Label { text: "Your own, from a command or a file: Settings › Widgets"; size: Theme.sizeCaption; color: Theme.text3; wrapMode: Text.WordWrap; Layout.fillWidth: true }
            }
        }

        // The ghost under the pointer while a library row is dragged.
        Rectangle {
            visible: root.dragId !== ""
            x: root.dragX - root.cellW / 2
            y: root.dragY - root.cellH / 2
            width: root.cellW; height: root.cellH
            radius: Theme.radiusCard
            color: Qt.alpha(Theme.glass, 0.9)
            border.width: 1; border.color: Theme.accent
            z: 30
            RowLayout {
                anchors.centerIn: parent
                spacing: Theme.s2
                Glyph { name: Widgets.manifests[root.dragId] ? Widgets.manifests[root.dragId].glyph : "plus"; size: 16 }
                Label { text: Widgets.manifests[root.dragId] ? Widgets.manifests[root.dragId].name : ""; weight: Font.DemiBold; size: Theme.sizeSmall }
            }
        }
    }
}
