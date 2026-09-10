import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The card chrome around one widget: glass, title, meta. The widget draws only its content.
Glass {
    id: root
    required property var entry
    property bool editing: false
    property var dashboard: null
    property bool dragging: false
    readonly property var manifest: Widgets.manifests[entry.id] || null
    readonly property string size: entry.w + "x" + entry.h

    readonly property bool focused: editing && dashboard && dashboard.focusKey === entry.key
    property bool resizing: false
    radius: Theme.radiusPanel - 2
    visible: manifest !== null
    border.color: focused ? Theme.accent : editing ? Theme.hairlineStrong : Theme.hairline
    border.width: focused ? 2 : 1
    opacity: dragging ? 0.85 : 1
    z: dragging || resizing ? 5 : 0

    // Edit mode: the whole card drags; on release it snaps to the nearest cell that fits, or springs back.
    // A click on the card's own body stays on the card; only the backdrop closes the dashboard.
    MouseArea { anchors.fill: parent; enabled: !root.editing; onClicked: {} }
    MouseArea {
        id: dragArea
        anchors.fill: parent
        enabled: root.editing
        z: 3
        cursorShape: root.editing ? (root.dragging ? Qt.ClosedHandCursor : Qt.OpenHandCursor) : Qt.ArrowCursor
        property real startX: 0
        property real startY: 0
        property real originX: 0
        property real originY: 0
        onPressed: mouse => { startX = mouse.x; startY = mouse.y; originX = root.x; originY = root.y; root.dragging = true; root.dashboard.focusKey = root.entry.key; }
        onPositionChanged: mouse => {
            if (!root.dragging) return;
            root.x = originX + (mouse.x - startX); root.y = originY + (mouse.y - startY);
            const cell = root.dashboard.cellAt(root.x, root.y);
            root.dashboard.previewMove(root.entry.key, cell.x, cell.y);
        }
        onReleased: {
            root.dragging = false;
            const cell = root.dashboard.cellAt(root.x, root.y);
            if (!root.dashboard.dropMove(root.entry.key, cell.x, cell.y)) { root.x = originX; root.y = originY; }
        }
        onCanceled: { root.dragging = false; root.dashboard.preview = null; root.x = originX; root.y = originY; }
    }
    // Settings, top left, when the manifest declares any.
    property bool configuring: false
    onEditingChanged: if (!editing) configuring = false
    readonly property var settingSpecs: manifest && manifest.settings ? manifest.settings : []
    Rectangle {
        visible: root.editing && root.settingSpecs.length > 0
        anchors { top: parent.top; left: parent.left; margins: 6 }
        width: 22; height: 22; radius: 11; color: root.configuring ? Theme.accent : Theme.raised; border.width: 1; border.color: root.configuring ? "transparent" : Theme.hairlineStrong
        z: 4
        Glyph { anchors.centerIn: parent; name: "settings-2"; size: 10; color: root.configuring ? Theme.onAccent : Theme.text }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.configuring = !root.configuring }
    }
    // The settings sheet covers the widget while it is open.
    Rectangle {
        visible: root.configuring
        anchors.fill: parent
        radius: root.radius
        color: Theme.light ? "#FFFFFF" : Theme.glass
        z: 3
        MouseArea { anchors.fill: parent }
        ColumnLayout {
            anchors { fill: parent; margins: Theme.s3; topMargin: Theme.s3 + 22 }
            spacing: Theme.s2
            Repeater {
                model: root.settingSpecs
                RowLayout {
                    required property var modelData
                    readonly property var current: Widgets.settingsFor(root.entry.key)[modelData.key]
                    Layout.fillWidth: true
                    spacing: Theme.s2
                    Label { text: modelData.label; size: Theme.sizeSmall; Layout.fillWidth: true; elide: Text.ElideRight }
                    Toggle { visible: modelData.type === "toggle"; checked: !!parent.current; onToggled: v => Widgets.setSetting(root.entry.key, modelData.key, v) }
                    Dropdown { visible: modelData.type === "choice"; listWidth: 160; options: modelData.options || []; value: parent.current; onPicked: v => Widgets.setSetting(root.entry.key, modelData.key, v) }
                    RowLayout {
                        visible: modelData.type === "number"
                        spacing: Theme.s2
                        readonly property real lo: modelData.min !== undefined ? modelData.min : 0
                        readonly property real hi: modelData.max !== undefined ? modelData.max : 10
                        Slider { implicitWidth: 90; value: (Number(parent.parent.current) - parent.lo) / (parent.hi - parent.lo); onMoved: f => Widgets.setSetting(root.entry.key, modelData.key, Math.round(parent.lo + f * (parent.hi - parent.lo))) }
                        Label { text: String(parent.parent.current); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 24; horizontalAlignment: Text.AlignRight }
                    }
                }
            }
            Item { Layout.fillHeight: true }
            RowLayout {
                Layout.fillWidth: true
                Label { text: "Defaults"; size: Theme.sizeCaption; color: Theme.accent
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.clearSettings(root.entry.key) } }
                Item { Layout.fillWidth: true }
                Button { text: "Done"; variant: "raised"; onClicked: root.configuring = false }
            }
        }
    }
    // Remove, top right.
    Rectangle {
        visible: root.editing
        anchors { top: parent.top; right: parent.right; margins: 6 }
        width: 22; height: 22; radius: 11; color: Theme.raised; border.width: 1; border.color: Theme.hairlineStrong
        z: 4
        Glyph { anchors.centerIn: parent; name: "x"; size: 10; weight: 1.6 }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.remove(root.entry.key) }
    }
    // Resize: the right edge, the bottom edge and the corner drag the span, held to the manifest's range.
    Repeater {
        model: root.editing ? [["right", Qt.SizeHorCursor], ["bottom", Qt.SizeVerCursor], ["corner", Qt.SizeFDiagCursor]] : []
        MouseArea {
            id: handle
            required property var modelData
            readonly property string edge: modelData[0]
            x: edge === "bottom" ? 0 : root.width - (edge === "corner" ? 18 : 8)
            y: edge === "right" ? 0 : root.height - (edge === "corner" ? 18 : 8)
            width: edge === "bottom" ? root.width - 18 : edge === "corner" ? 18 : 8
            height: edge === "right" ? root.height - 18 : edge === "corner" ? 18 : 8
            z: 5
            cursorShape: modelData[1]
            property real sx: 0
            property real sy: 0
            property int w: 0
            property int h: 0
            function span(mouse) {
                const p = mapToItem(root, mouse.x, mouse.y);
                const step = root.dashboard.cellW + root.dashboard.gutter, stepH = root.dashboard.cellH + root.dashboard.gutter;
                w = edge === "bottom" ? root.entry.w : Math.max(1, Math.round((root.width + (p.x - sx)) / step));
                h = edge === "right" ? root.entry.h : Math.max(1, Math.round((root.height + (p.y - sy)) / stepH));
            }
            onPressed: mouse => { const p = mapToItem(root, mouse.x, mouse.y); sx = p.x; sy = p.y; root.resizing = true; root.dashboard.focusKey = root.entry.key; }
            onPositionChanged: mouse => { if (!root.resizing) return; span(mouse); root.dashboard.previewResize(root.entry.key, w, h); }
            onReleased: mouse => { root.resizing = false; span(mouse); root.dashboard.dropResize(root.entry.key, w, h); }
            onCanceled: { root.resizing = false; root.dashboard.preview = null; }
            Rectangle {
                visible: handle.edge === "corner"
                anchors { right: parent.right; bottom: parent.bottom; margins: 5 }
                width: 8; height: 8; radius: 2
                color: root.focused ? Theme.accent : Theme.text3
            }
        }
    }
    // Size, bottom right: cycles the manifest's sizes.
    Rectangle {
        visible: root.editing && root.manifest && (root.manifest.sizes || []).length > 1
        anchors { bottom: parent.bottom; right: parent.right; margins: 6; rightMargin: 22 }
        width: sizeLabel.implicitWidth + Theme.s2 * 2; height: 22; radius: 11; color: Theme.raised; border.width: 1; border.color: Theme.hairlineStrong
        z: 4
        Label { id: sizeLabel; anchors.centerIn: parent; text: root.size; mono: true; size: Theme.sizeCaption }
        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.cycleSize(root.entry.key) }
    }

    ColumnLayout {
        anchors { fill: parent; margins: Theme.s3 }
        spacing: Theme.s2 + 2

        RowLayout {
            Layout.fillWidth: true
            visible: (loader.item ? loader.item.title : "") !== ""
            // Room for the settings gear while editing.
            Layout.leftMargin: root.editing && root.settingSpecs.length > 0 ? 22 : 0
            Label { text: loader.item ? loader.item.title : ""; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.fillWidth: true }
            Label { text: loader.item ? loader.item.meta : ""; size: Theme.sizeCaption; color: Theme.text3 }
        }

        Loader {
            id: loader
            Layout.fillWidth: true
            Layout.fillHeight: true
            source: Widgets.componentUrl(root.entry.id)
            onLoaded: {
                item.manifest = Qt.binding(() => root.manifest);
                item.size = Qt.binding(() => root.size);
                item.settings = Qt.binding(() => Widgets.settingsFor(root.entry.key));
            }
        }
    }
}
