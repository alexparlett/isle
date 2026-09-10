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
    // The permission-gated wrapper a sandboxed widget is handed in place of the services.
    WidgetHost { id: host; permissions: root.manifest ? (root.manifest.permissions || []) : [] }

    readonly property bool focused: editing && dashboard && dashboard.focusKey === entry.key
    property bool resizing: false
    // The live drag offset from the card's grid position, in pixels; zero when not dragging.
    property real dragDX: 0
    property real dragDY: 0
    // The card's pixel position when the drag began, so live layout changes under it do not move it.
    property real homeX: 0
    property real homeY: 0
    property int lastCellX: -1
    property int lastCellY: -1
    radius: Theme.radiusPanel - 2
    visible: manifest !== null
    border.color: focused ? Theme.accent : editing ? Theme.hairlineStrong : Theme.hairline
    border.width: focused ? 2 : 1
    opacity: dragging ? 0.85 : 1
    z: dragging || resizing ? 5 : 0

    // Edit mode: the whole card drags; on release it snaps to the nearest cell that fits, or springs back.
    // A click on the card's own body stays on the card; only the backdrop closes the dashboard.
    MouseArea { anchors.fill: parent; enabled: !root.editing; onClicked: {} }
    // The card keeps its grid-position binding; a drag adds an offset the delegate applies, so on release
    // the binding snaps it to its cell. Mouse coordinates are read in the grid's frame, not the card's own
    // (which moves under the cursor), so the offset does not feed back into itself.
    MouseArea {
        id: dragArea
        anchors.fill: parent
        enabled: root.editing
        z: 3
        cursorShape: root.editing ? (root.dragging ? Qt.ClosedHandCursor : Qt.OpenHandCursor) : Qt.ArrowCursor
        property real pressX: 0
        property real pressY: 0
        onPressed: mouse => {
            const p = mapToItem(root.parent, mouse.x, mouse.y);
            pressX = p.x; pressY = p.y;
            root.homeX = root.x; root.homeY = root.y;
            root.dragDX = 0; root.dragDY = 0;
            root.lastCellX = root.entry.x; root.lastCellY = root.entry.y;
            root.dashboard.focusKey = root.entry.key;
            Widgets.beginDrag(root.entry.key);
            root.dragging = true;
        }
        onPositionChanged: mouse => {
            if (!root.dragging) return;
            const p = mapToItem(root.parent, mouse.x, mouse.y);
            root.dragDX = p.x - pressX;
            root.dragDY = p.y - pressY;
            // The cell the floating card's centre sits over, mapped back to a top-left, so a neighbour flows
            // only once the card has clearly moved onto it, not at the first pixel of overlap.
            const cw = root.dashboard.cellW + root.dashboard.gutter, ch = root.dashboard.cellH + root.dashboard.gutter;
            const cx = root.homeX + root.dragDX + (root.entry.w * cw - root.dashboard.gutter) / 2;
            const cy = root.homeY + root.dragDY + (root.entry.h * ch - root.dashboard.gutter) / 2;
            const cell = { x: Math.round((cx - root.dashboard.margin) / cw - root.entry.w / 2),
                           y: Math.round((cy - root.dashboard.gridTop) / ch - root.entry.h / 2) };
            if (cell.x === root.lastCellX && cell.y === root.lastCellY) return;
            if (Widgets.dragStep(root.entry.key, cell.x, cell.y)) { root.lastCellX = cell.x; root.lastCellY = cell.y; }
        }
        onReleased: {
            if (!root.dragging) return;
            root.dragging = false;
            root.dragDX = 0; root.dragDY = 0;
            Widgets.commitDrag();
        }
        onCanceled: { root.dragging = false; root.dragDX = 0; root.dragDY = 0; Widgets.cancelDrag(); }
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
                if (item.hasOwnProperty("manifest")) item.manifest = Qt.binding(() => root.manifest);
                if (item.hasOwnProperty("size")) item.size = Qt.binding(() => root.size);
                if (item.hasOwnProperty("settings")) item.settings = Qt.binding(() => Widgets.settingsFor(root.entry.key));
                // A sandboxed widget receives the wrapper; the shell's own and trusted ones ignore it.
                if (item.hasOwnProperty("host")) item.host = host;
            }
            // A widget refused for reaching the shell without trust, or one with a QML error, says so
            // rather than sitting blank.
            Label {
                anchors.centerIn: parent
                visible: Widgets.blocked(root.manifest) || loader.status === Loader.Error
                text: Widgets.blocked(root.manifest) ? "Reaches past the sandbox (" + Widgets.issuesOf(root.entry.id).join(", ") + "). Add \"trust\": true to its widget.json to run it."
                    : "This widget could not load"
                color: Theme.danger; size: Theme.sizeSmall; wrapMode: Text.WordWrap
                width: parent.width - Theme.s3 * 2; horizontalAlignment: Text.AlignHCenter
            }
        }
    }
}
