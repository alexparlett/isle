import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.services

// The monitors as scaled rectangles you drag into place. On release the dragged one snaps flush to a neighbour's edge
// and the layout is written as absolute positions, so nothing else moves.
Rectangle {
    id: root
    implicitHeight: 220
    radius: Theme.radiusCard
    color: Theme.pressed
    border.width: 1
    border.color: Theme.hairline
    clip: true

    readonly property var mons: Displays.monitors
    // Logical geometry: width and height in layout pixels (physical over scale).
    function geo(m) { const o = Displays.info(m); const s = o.scale || m.scale || 1; return { x: o.x || 0, y: o.y || 0, w: Math.round((o.width || m.width) / s), h: Math.round((o.height || m.height) / s), name: m.name }; }
    readonly property var geos: mons.map(geo)
    readonly property var bounds: {
        if (!geos.length) return { x: 0, y: 0, w: 1, h: 1 };
        const x0 = Math.min(...geos.map(g => g.x)), y0 = Math.min(...geos.map(g => g.y));
        const x1 = Math.max(...geos.map(g => g.x + g.w)), y1 = Math.max(...geos.map(g => g.y + g.h));
        return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
    }
    readonly property int pad: Theme.s4
    readonly property real k: Math.min((width - pad * 2) / Math.max(1, bounds.w), (height - pad * 2) / Math.max(1, bounds.h)) * 0.9
    readonly property real ox: (width - bounds.w * k) / 2 - bounds.x * k
    readonly property real oy: (height - bounds.h * k) / 2 - bounds.y * k
    readonly property int snap: 48

    // The dragged monitor's logical position, snapped to the others and kept from overlapping them.
    function settle(g, nx, ny) {
        const others = geos.filter(o => o.name !== g.name);
        let x = nx, y = ny;
        for (const o of others) {
            if (Math.abs(x - (o.x + o.w)) < snap) x = o.x + o.w;
            if (Math.abs((x + g.w) - o.x) < snap) x = o.x - g.w;
            if (Math.abs(y - (o.y + o.h)) < snap) y = o.y + o.h;
            if (Math.abs((y + g.h) - o.y) < snap) y = o.y - g.h;
            if (Math.abs(x - o.x) < snap) x = o.x;
            if (Math.abs(y - o.y) < snap) y = o.y;
            if (Math.abs((x + g.w) - (o.x + o.w)) < snap) x = o.x + o.w - g.w;
            if (Math.abs((y + g.h) - (o.y + o.h)) < snap) y = o.y + o.h - g.h;
        }
        for (const o of others) {
            const overlap = x < o.x + o.w && o.x < x + g.w && y < o.y + o.h && o.y < y + g.h;
            if (!overlap) continue;
            // Push out the shorter way.
            const dx = (x + g.w / 2) < (o.x + o.w / 2) ? o.x - g.w - x : o.x + o.w - x;
            const dy = (y + g.h / 2) < (o.y + o.h / 2) ? o.y - g.h - y : o.y + o.h - y;
            if (Math.abs(dx) <= Math.abs(dy)) x += dx; else y += dy;
        }
        return { x: Math.round(x), y: Math.round(y) };
    }

    Repeater {
        model: root.geos
        Rectangle {
            id: card
            required property var modelData
            required property int index
            readonly property bool focused: Compositor.focusedMonitor && Compositor.focusedMonitor.name === modelData.name
            readonly property bool primary: Displays.primary === modelData.name
            x: root.ox + modelData.x * root.k
            y: root.oy + modelData.y * root.k
            width: Math.max(24, modelData.w * root.k)
            height: Math.max(16, modelData.h * root.k)
            radius: 6
            color: drag.active ? Theme.accent : Theme.raised
            border.width: 1
            border.color: focused ? Theme.accent : Theme.hairlineStrong
            z: drag.active ? 2 : 1
            Behavior on x { enabled: !drag.active; NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
            Behavior on y { enabled: !drag.active; NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 0
                Label { text: modelData.name; size: Theme.sizeSmall; weight: Font.DemiBold; color: drag.active ? Theme.onAccent : Theme.text; Layout.alignment: Qt.AlignHCenter }
                Label { text: modelData.w + "×" + modelData.h + (card.primary ? "  ·  primary" : ""); size: Theme.sizeCaption; color: drag.active ? Theme.onAccent : Theme.text3; Layout.alignment: Qt.AlignHCenter }
            }
            MouseArea {
                id: drag
                anchors.fill: parent
                // The page scrolls; a drag here must not become a scroll.
                preventStealing: true
                cursorShape: active ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                property bool active: false
                property real sx: 0
                property real sy: 0
                onPressed: mouse => { active = true; sx = mouse.x; sy = mouse.y; }
                onPositionChanged: mouse => { if (!active) return; card.x += mouse.x - sx; card.y += mouse.y - sy; }
                onReleased: {
                    active = false;
                    const g = card.modelData;
                    const p = root.settle(g, (card.x - root.ox) / root.k, (card.y - root.oy) / root.k);
                    Displays.place(g.name, p.x, p.y);
                }
            }
        }
    }
    Label { anchors { right: parent.right; bottom: parent.bottom; margins: Theme.s2 } text: "Drag a display to move it. Edges snap."; size: Theme.sizeCaption; color: Theme.text3 }
}
