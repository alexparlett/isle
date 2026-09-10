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
    // Stays up while the confirm dialog is open even after a keybind has cleared Surfaces.dashboard.
    visible: Surfaces.dashboard || confirming
    // Every close route (Escape, the keybind, a margin click) clears Surfaces.dashboard; this catches it and
    // asks first when there are unsaved edits, keeping the window up via `visible` until the choice is made.
    Connections {
        target: Surfaces
        function onDashboardChanged() {
            if (Surfaces.dashboard) return;                                      // opening: nothing to guard
            if (root.editing && root.dirty && !root.confirming) root.confirming = true;  // intercept: keep it up and ask
            else root.editing = false;                                          // clean or non-edit close
        }
    }

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
    // A header strip is always reserved at the top for the page tabs and the edit toggle, so no control
    // ever overlaps a widget.
    property int pagesH: 36 + gutter
    readonly property real gridTop: margin + pagesH
    readonly property real cellW: (width - margin * 2 - libraryW - gutter * (columns - 1)) / columns
    readonly property real cellH: (height - margin * 2 - pagesH - toolbarH - gutter * (rows - 1)) / rows
    Behavior on libraryW { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
    Behavior on toolbarH { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
    Behavior on pagesH { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }

    // Edit mode: drag to move, the corner to resize, × to remove; the library adds, by click or by drag. E toggles it.
    property bool editing: false
    onVisibleChanged: if (visible) grid.forceActiveFocus()
    onEditingChanged: { pending = null; focusKey = ""; preview = null; library.composing = false; grid.forceActiveFocus(); if (editing) Widgets.rescan(); }
    // Unsaved edits: the layout differs from the snapshot taken when editing began.
    readonly property bool dirty: editing && grid.before !== null && JSON.stringify(Widgets.pages) !== JSON.stringify(grid.before)
    property bool confirming: false
    // The store: the library as a gallery with the whole listing per widget.
    property bool storeOpen: false
    function focusGrid() { grid.forceActiveFocus(); }
    // The dialog's answers: Keep leaves the changes in place and closes; Discard restores the pre-edit layout
    // and closes; Cancel reopens the dashboard, still editing.
    function keepEdits() { editing = false; confirming = false; Surfaces.dashboard = false; }
    function discardEdits() { Prefs.p.dashboardPages = grid.before || []; Prefs.p.dashboardPage = grid.beforePage; editing = false; confirming = false; Surfaces.dashboard = false; }
    function cancelClose() { confirming = false; Surfaces.dashboard = true; }
    // The card the keys act on, by instance key.
    property string focusKey: ""
    // Where a dragged or resized card would land: { x, y, w, h, ok }, or null.
    // The outline shown while resizing (the live push shows a drag on its own).
    property var preview: null
    function previewResize(key, w, h) {
        const e = Widgets.layout.find(e => e.key === key); if (!e) return;
        preview = { x: e.x, y: e.y, w: w, h: h, ok: Widgets.canResize(key, w, h) };
    }
    function dropResize(key, w, h) { preview = null; return Widgets.resize(key, w, h); }
    // Arrow keys move the focused card a cell; with Shift they resize it.
    function nudge(dx, dy, resize) {
        const e = Widgets.layout.find(e => e.key === focusKey); if (!e) return;
        if (resize) { if (!Widgets.resize(focusKey, e.w + dx, e.h + dy)) notice = "That size does not fit here"; }
        else if (!Widgets.place(focusKey, e.x + dx, e.y + dy)) notice = "No room that way";
        if (notice) noticeTimer.restart();
    }
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
    function cellAt(px, py) { return { x: Math.round((px - margin) / (cellW + gutter)), y: Math.round((py - gridTop) / (cellH + gutter)) }; }

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
        // Clicks in the grid's own area (the gaps between and around the cards) do nothing; only a click in
        // the margin outside it reaches the backdrop and closes the dashboard.
        MouseArea {
            x: root.margin; y: root.margin
            width: root.width - root.margin * 2 - root.libraryW
            height: root.height - root.margin * 2 - root.toolbarH
            onClicked: {}
        }
        Keys.onPressed: event => {
            const shift = event.modifiers & Qt.ShiftModifier;
            if (event.key === Qt.Key_Escape) { if (root.storeOpen) root.storeOpen = false; else if (root.confirming) root.cancelClose(); else if (root.pending) root.pending = null; else Surfaces.dashboard = false; }
            else if (event.key === Qt.Key_E) root.editing = !root.editing;
            else if (event.key >= Qt.Key_1 && event.key <= Qt.Key_9) Widgets.setPage(event.key - Qt.Key_1);
            else if (root.editing && event.key === Qt.Key_Tab) { const l = Widgets.layout; const i = l.findIndex(e => e.key === root.focusKey); root.focusKey = l.length ? l[(i + 1) % l.length].key : ""; }
            else if (root.editing && root.focusKey && event.key === Qt.Key_Left) root.nudge(-1, 0, shift);
            else if (root.editing && root.focusKey && event.key === Qt.Key_Right) root.nudge(1, 0, shift);
            else if (root.editing && root.focusKey && event.key === Qt.Key_Up) root.nudge(0, -1, shift);
            else if (root.editing && root.focusKey && event.key === Qt.Key_Down) root.nudge(0, 1, shift);
            else if (root.editing && root.focusKey && (event.key === Qt.Key_Delete || event.key === Qt.Key_Backspace)) { Widgets.remove(root.focusKey); root.focusKey = ""; }
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
                y: root.gridTop + modelData.y * (root.cellH + root.gutter)
                width: root.cellW; height: root.cellH
                radius: Theme.radiusCard
                color: marked || under ? Qt.alpha(Theme.accent, 0.12) : cellArea.containsMouse ? Qt.alpha(Theme.text, 0.06) : Qt.alpha(Theme.text, 0.025)
                border.width: 1; border.color: marked || under ? Theme.accent : Qt.alpha(Theme.text, cellArea.containsMouse ? 0.3 : 0.12)
                Glyph { anchors.centerIn: parent; name: "plus"; size: 18; color: marked || under ? Theme.accent : Qt.alpha(Theme.text, cellArea.containsMouse ? 0.8 : 0.35) }
                MouseArea { id: cellArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.pending = parent.marked ? null : { x: parent.modelData.x, y: parent.modelData.y } }
            }
        }

        // Where a drag or a resize would land: accent when it fits or can make room, danger when not.
        Rectangle {
            visible: root.preview !== null
            x: root.preview ? root.margin + root.preview.x * (root.cellW + root.gutter) : 0
            y: root.preview ? root.gridTop + root.preview.y * (root.cellH + root.gutter) : 0
            width: root.preview ? root.preview.w * root.cellW + (root.preview.w - 1) * root.gutter : 0
            height: root.preview ? root.preview.h * root.cellH + (root.preview.h - 1) * root.gutter : 0
            radius: Theme.radiusPanel - 2
            color: root.preview && root.preview.ok ? Qt.alpha(Theme.accent, 0.12) : Qt.alpha(Theme.danger, 0.10)
            border.width: 2; border.color: root.preview && root.preview.ok ? Theme.accent : Theme.danger
            z: 4
            Behavior on x { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
            Behavior on y { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
            Behavior on width { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
            Behavior on height { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
        }

        // The pages: tabs across the top; while editing, add, rename by double-click, and remove.
        Row {
            anchors { horizontalCenter: parent.horizontalCenter; horizontalCenterOffset: -root.libraryW / 2 }
            y: root.margin
            height: 36
            spacing: Theme.s1
            z: 10
            Repeater {
                model: Widgets.pages
                Rectangle {
                    id: tab
                    required property var modelData
                    required property int index
                    readonly property bool sel: index === Widgets.page
                    property bool renaming: false
                    width: Math.max(72, tabLabel.implicitWidth + Theme.s3 * 2 + (root.editing && Widgets.pages.length > 1 ? 22 : 0)); height: 32
                    radius: 16
                    color: sel ? Theme.raised : tabArea.containsMouse ? Qt.alpha(Theme.text, 0.05) : "transparent"
                    border.width: 1; border.color: sel ? Theme.hairlineStrong : "transparent"
                    // Centred in the tab; when the remove × shows it takes a little from the right so the text stays centred in what is left.
                    Label { id: tabLabel; visible: !tab.renaming; anchors { horizontalCenter: parent.horizontalCenter; horizontalCenterOffset: root.editing && Widgets.pages.length > 1 ? -8 : 0; verticalCenter: parent.verticalCenter } text: tab.modelData.name; size: Theme.sizeSmall; weight: Font.DemiBold; color: tab.sel ? Theme.text : Theme.text2 }
                    Field {
                        visible: tab.renaming
                        anchors { fill: parent; margins: 2 }
                        size: Theme.sizeSmall
                        onVisibleChanged: if (visible) { text = tab.modelData.name; input.forceActiveFocus(); input.selectAll(); }
                        onAccepted: { Widgets.renamePage(tab.index, text.trim()); tab.renaming = false; grid.forceActiveFocus(); }
                        input.onActiveFocusChanged: if (!input.activeFocus) tab.renaming = false
                    }
                    MouseArea { id: tabArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; enabled: !tab.renaming; onClicked: Widgets.setPage(tab.index); onDoubleClicked: if (root.editing) tab.renaming = true }
                    Rectangle {
                        visible: root.editing && Widgets.pages.length > 1 && !tab.renaming
                        anchors { right: parent.right; verticalCenter: parent.verticalCenter; rightMargin: 6 }
                        width: 18; height: 18; radius: 9; color: closeArea.containsMouse ? Theme.pressed : "transparent"
                        Glyph { anchors.centerIn: parent; name: "x"; size: 9; color: Theme.text2 }
                        MouseArea { id: closeArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.removePage(tab.index) }
                    }
                }
            }
            Rectangle {
                visible: root.editing
                width: 32; height: 32; radius: 16
                color: addPageArea.containsMouse ? Qt.alpha(Theme.text, 0.08) : "transparent"
                border.width: 1; border.color: Theme.hairline
                Glyph { anchors.centerIn: parent; name: "plus"; size: 14; color: Theme.text2 }
                MouseArea { id: addPageArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.addPage("") }
            }
        }

        Repeater {
            model: Widgets.layout
            WidgetCard {
                id: widgetCard
                required property var modelData
                required property int index
                entry: modelData
                // Its planned cell (the same as its stored cell except while another card's drag is pushing
                // it), so a neighbour glides aside without the model changing under the drag.
                readonly property var pcell: Widgets.cellFor(modelData.key) || modelData
                // Not dragging: the planned grid position, which animates. Dragging: a free float from where the
                // card was grabbed, so the pushes happening under it never move it.
                x: dragging ? homeX + dragDX : root.margin + pcell.x * (root.cellW + root.gutter)
                y: dragging ? homeY + dragDY : root.gridTop + pcell.y * (root.cellH + root.gutter)
                width: modelData.w * root.cellW + (modelData.w - 1) * root.gutter
                height: modelData.h * root.cellH + (modelData.h - 1) * root.gutter
                editing: root.editing
                dashboard: root
                // Enter: fade and rise, staggered by row.
                opacity: root.visible ? 1 : 0
                transform: Translate { y: root.visible ? 0 : 12 + modelData.y * 4 }
                Behavior on opacity { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
                // A pushed neighbour glides aside; the dragged card itself floats free and only its release
                // animates, so a longer, gentler curve here reads as a soft push rather than a snap.
                Behavior on x { enabled: !dragging; NumberAnimation { duration: Theme.morph; easing.type: Easing.OutCubic } }
                Behavior on y { enabled: !dragging; NumberAnimation { duration: Theme.morph; easing.type: Easing.OutCubic } }
            }
        }

        // Edit, in the header's right end when viewing; in edit mode the bottom strip owns Done and Discard.
        Rectangle {
            visible: !root.editing
            anchors { right: parent.right; top: parent.top; rightMargin: root.margin + root.libraryW; topMargin: root.margin }
            width: editRow.implicitWidth + Theme.s3; height: 32; radius: 16
            color: editArea.containsMouse ? Theme.raised : Qt.alpha(Theme.glass, 0.9)
            border.width: 1; border.color: Theme.hairline
            z: 10
            RowLayout {
                id: editRow
                anchors.centerIn: parent
                spacing: Theme.s2
                Glyph { name: "pencil"; size: 14; color: Theme.text2 }
                Label { text: "Edit"; size: Theme.sizeSmall; weight: Font.DemiBold; color: Theme.text2 }
            }
            MouseArea { id: editArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.editing = true }
        }
        // The layout as it was when editing began, for Discard.
        property var before: null
        property int beforePage: 0
        Connections { target: root; function onEditingChanged() { if (root.editing) { grid.before = JSON.parse(JSON.stringify(Widgets.pages)); grid.beforePage = Widgets.page; } } }
        RowLayout {
            visible: root.editing
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom; leftMargin: root.margin; rightMargin: root.margin + root.libraryW; bottomMargin: root.margin }
            height: 44
            spacing: Theme.s2
            z: 10
            Label { text: root.notice || (root.pending ? "Pick a widget from the library for the marked cell" : "Click or drag a widget from the library; an empty cell marks where it goes"); size: Theme.sizeSmall; color: root.notice ? Theme.warn : Theme.text2 }
            Item { Layout.fillWidth: true }
            Label { text: "Drag to move · edges to resize · arrows nudge, Shift+arrows resize · Delete removes · 1–9 pages"; size: Theme.sizeCaption; color: Theme.text3 }
            Item { Layout.fillWidth: true }
            Button { text: "Reset layout"; variant: "text"; onClicked: Widgets.resetLayout() }
            Button { text: "Discard"; variant: "text"; onClicked: { Prefs.p.dashboardPages = grid.before || []; Prefs.p.dashboardPage = grid.beforePage; root.editing = false; } }
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
            // The form for one of the user's own replaces the list while it is open.
            property bool composing: false
            readonly property var rows: {
                const q = query.trim().toLowerCase();
                return Widgets.library.filter(m => !q || m.name.toLowerCase().indexOf(q) >= 0 || (m.description || "").toLowerCase().indexOf(q) >= 0 || m.category.toLowerCase().indexOf(q) >= 0);
            }
            WidgetForm {
                id: widgetForm
                visible: library.composing
                anchors { fill: parent; margins: Theme.s3 }
                onDone: library.composing = false
            }
            ColumnLayout {
                visible: !library.composing
                anchors { fill: parent; margins: Theme.s3 }
                spacing: Theme.s2
                RowLayout {
                    Layout.fillWidth: true
                    Label { text: "Widgets"; size: Theme.sizeHeading; weight: Font.DemiBold; Layout.fillWidth: true }
                    Button { text: "Browse"; glyph: "layout-grid"; variant: "text"; onClicked: root.storeOpen = true }
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
                                implicitWidth: 34; implicitHeight: 34; radius: 10
                                color: Theme.pressed
                                Glyph { anchors.centerIn: parent; name: row.modelData.glyph || "layout-grid"; size: 16; color: Theme.text }
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Label { text: row.modelData.name; weight: Font.DemiBold; size: Theme.sizeSmall; elide: Text.ElideRight; Layout.fillWidth: true }
                                Label { text: row.modelData.trust ? "Trusted · full access" : (row.modelData.permissions && row.modelData.permissions.length ? "Uses " + row.modelData.permissions.join(", ") : row.modelData.description || (row.modelData.sizes || []).map(s => s.replace("x", "×")).join(" · ")); size: Theme.sizeCaption; color: row.modelData.trust ? Theme.warn : Theme.text3; elide: Text.ElideRight; Layout.fillWidth: true }
                            }
                            // The user's own: edit and delete, shown on hover.
                            Repeater {
                                model: row.modelData.user && rowArea.containsMouse ? [["pencil", "edit"], ["trash", "delete"]] : []
                                Rectangle {
                                    required property var modelData
                                    implicitWidth: 24; implicitHeight: 24; radius: 12
                                    color: toolArea.containsMouse ? Theme.pressed : "transparent"
                                    Glyph { anchors.centerIn: parent; name: modelData[0]; size: 12; color: Theme.text2 }
                                    MouseArea {
                                        id: toolArea
                                        anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor
                                        onClicked: mouse => {
                                            mouse.accepted = true;
                                            if (modelData[1] === "edit" && row.modelData.source) { widgetForm.startEdit(row.modelData); library.composing = true; }
                                            else if (modelData[1] === "delete") Widgets.deleteUser(row.modelData.id);
                                        }
                                    }
                                }
                            }
                            Label { visible: row.count > 0; text: row.count > 1 ? "×" + row.count : "on"; size: Theme.sizeCaption; color: Theme.accent; tabular: true }
                            Glyph { visible: row.count > 0; name: "check"; size: 12; color: Theme.accent }
                            Glyph { visible: row.count === 0; name: "plus"; size: 14; color: Theme.text3 }
                        }
                        MouseArea {
                            id: rowArea
                            anchors.fill: parent
                            hoverEnabled: true
                            z: -1
                            cursorShape: row.addable ? Qt.PointingHandCursor : Qt.ArrowCursor
                            property real px: 0
                            property real py: 0
                            property bool moved: false
                            onPressed: mouse => { px = mouse.x; py = mouse.y; moved = false; }
                            onPositionChanged: mouse => {
                                if (!pressed || !row.addable) return;
                                if (!moved && Math.abs(mouse.x - px) + Math.abs(mouse.y - py) < 8) return;
                                moved = true;
                                const p = mapToItem(null, mouse.x, mouse.y);
                                root.dragId = row.modelData.id; root.dragX = p.x; root.dragY = p.y;
                            }
                            onReleased: mouse => {
                                if (!row.addable) return;
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
                Button { Layout.fillWidth: true; text: "New text widget"; glyph: "plus"; variant: "raised"; onClicked: { widgetForm.startNew(); library.composing = true; } }
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

        // The store, over everything else in edit mode.
        WidgetStore {
            id: widgetStore
            open: root.storeOpen
            onOpenChanged: root.storeOpen = open
            z: 35
            onPlace: id => { root.place(id, root.pending ? root.pending.x : -1, root.pending ? root.pending.y : -1); root.pending = null; }
            onEdit: m => { widgetForm.startEdit(m); library.composing = true; root.storeOpen = false; }
        }

        // Leaving edit mode with unsaved changes asks first.
        Item {
            visible: root.confirming
            anchors.fill: parent
            z: 40
            Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.4) }
            MouseArea { anchors.fill: parent; onClicked: root.cancelClose() }
            Glass {
                anchors.centerIn: parent
                width: 420
                height: dialogCol.implicitHeight + Theme.s4 * 2
                radius: Theme.radiusPanel
                MouseArea { anchors.fill: parent }
                ColumnLayout {
                    id: dialogCol
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
                    spacing: Theme.s3
                    Label { text: "Keep your changes?"; size: Theme.sizeHeading; weight: Font.DemiBold }
                    Label { text: "The dashboard layout has changed. Keep it, or go back to how it was."; size: Theme.sizeSmall; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.s2
                        Button { text: "Discard"; variant: "text"; onClicked: root.discardEdits() }
                        Item { Layout.fillWidth: true }
                        Button { text: "Cancel"; variant: "text"; onClicked: root.cancelClose() }
                        Button { text: "Keep"; glyph: "check"; variant: "accent"; onClicked: root.keepEdits() }
                    }
                }
            }
        }
    }
}
