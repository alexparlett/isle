import QtQuick
import QtQuick.Layouts
import Isle.Files
import qs.theme
import qs.ui
import "kinds.js" as Kinds

// A folder's children as tiles whose areas are their sizes, squarified (Bruls et al.), each in the
// colour of the kind most of it is.
Item {
    id: root
    required property var app
    property var items: []
    // What the percentages are of.
    property real total: 1

    readonly property var tiles: layout(items.filter(i => i.size > 0 && !i.skipped), width, height)
    // Unticked tiles stand back only while something here is ticked.
    readonly property bool anyPicked: { app.basketRev; return items.some(i => !i.other && app.inBasket(i.path)); }

    function layout(list, w, h) {
        const sum = list.reduce((s, i) => s + i.size, 0);
        if (!sum || w <= 0 || h <= 0) return [];
        const scale = w * h / sum;
        const out = [];
        let rect = { x: 0, y: 0, w: w, h: h };
        let row = [];
        // The worst aspect ratio in a row laid along a side of this length.
        function worst(r, side) {
            const s = r.reduce((t, a) => t + a.area, 0);
            let max = 0, min = Infinity;
            for (const a of r) { max = Math.max(max, a.area); min = Math.min(min, a.area); }
            return Math.max(side * side * max / (s * s), s * s / (side * side * min));
        }
        function place(r) {
            const s = r.reduce((t, a) => t + a.area, 0);
            if (rect.w >= rect.h) {
                const cw = s / rect.h;
                let y = rect.y;
                for (const a of r) { const th = a.area / cw; out.push({ item: a.item, x: rect.x, y: y, w: cw, h: th }); y += th; }
                rect = { x: rect.x + cw, y: rect.y, w: rect.w - cw, h: rect.h };
            } else {
                const rh = s / rect.w;
                let x = rect.x;
                for (const a of r) { const tw = a.area / rh; out.push({ item: a.item, x: x, y: rect.y, w: tw, h: rh }); x += tw; }
                rect = { x: rect.x, y: rect.y + rh, w: rect.w, h: rect.h - rh };
            }
        }
        for (const item of list) {
            const a = { item: item, area: item.size * scale };
            const side = Math.min(rect.w, rect.h);
            if (!row.length || worst(row.concat([a]), side) <= worst(row, side)) row.push(a);
            else { place(row); row = [a]; }
        }
        if (row.length) place(row);
        return out;
    }

    Repeater {
        model: root.tiles
        Rectangle {
            id: tile
            required property var modelData
            readonly property var item: modelData.item
            readonly property bool big: width > 150 && height > 110
            readonly property bool roomy: width > 84 && height > 58
            readonly property bool picked: { root.app.basketRev; return !item.other && root.app.inBasket(item.path); }
            readonly property color ink: "#0A0B0D"
            x: modelData.x + 2
            y: modelData.y + 2
            width: Math.max(0, modelData.w - 4)
            height: Math.max(0, modelData.h - 4)
            radius: Math.min(Theme.radiusControl, width / 3, height / 3)
            color: item.other ? Qt.alpha(Theme.text, 0.12) : Kinds.color(item.kind)
            opacity: root.anyPicked && !picked ? 0.72 : 1
            clip: true

            Rectangle {
                anchors.fill: parent
                radius: parent.radius
                color: Qt.alpha("#FFFFFF", tileArea.containsMouse ? 0.16 : 0)
                Behavior on color { ColorAnimation { duration: Theme.quick } }
            }
            MouseArea {
                id: tileArea
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: tile.item.dir ? Qt.PointingHandCursor : Qt.ArrowCursor
                onClicked: if (!tile.item.other) root.app.open(tile.item)
            }

            RowLayout {
                visible: tile.roomy
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s3 }
                spacing: Theme.s2
                Rectangle {
                    visible: !tile.item.other
                    implicitWidth: 24
                    implicitHeight: 24
                    radius: 7
                    color: Qt.alpha(tile.ink, tile.picked || tileArea.containsMouse ? 0.22 : 0.1)
                    Glyph { anchors.centerIn: parent; visible: !tile.picked && !tileArea.containsMouse; name: tile.item.dir ? "folder" : Kinds.glyph(tile.item.kind); size: 13; color: tile.ink }
                    Check { anchors.centerIn: parent; visible: tile.picked || tileArea.containsMouse; app: root.app; item: tile.item }
                }
                Item { Layout.fillWidth: true }
                Label {
                    text: (tile.item.size / root.total * 100).toFixed(tile.item.size / root.total < 0.1 ? 1 : 0) + "%"
                    size: Theme.sizeCaption
                    weight: Font.DemiBold
                    color: Qt.alpha(tile.ink, 0.7)
                    mono: true
                    tabular: true
                }
            }

            ColumnLayout {
                visible: tile.roomy
                anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.s3 }
                spacing: 0
                Label {
                    Layout.fillWidth: true
                    text: tile.item.other ? root.app.count(tile.item.files) + " smaller items" : tile.item.name
                    weight: Font.DemiBold
                    size: tile.big ? Theme.sizeBody : Theme.sizeSmall
                    color: tile.item.other ? Theme.text2 : tile.ink
                    elide: Text.ElideRight
                }
                Label {
                    Layout.fillWidth: true
                    text: Engine.formatSize(tile.item.size)
                    size: tile.big ? Theme.sizeTitle : Theme.sizeSmall
                    weight: Font.Bold
                    color: tile.item.other ? Theme.text : tile.ink
                }
                Label {
                    visible: tile.big && tile.item.dir
                    Layout.fillWidth: true
                    text: root.app.count(tile.item.files) + (tile.item.files === 1 ? " file" : " files")
                    size: Theme.sizeCaption
                    color: Qt.alpha(tile.ink, 0.65)
                }
            }
        }
    }
}
