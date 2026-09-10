import QtQuick
import qs.theme

// The last `bars` of a list of 0..1 values, newest on the right: as bars, a line, or a filled area.
Item {
    id: root
    property var values: []
    property color color: Theme.text2
    property int bars: 24
    // "bars", "line" or "area".
    property string style: "bars"
    implicitHeight: 28

    readonly property var shown: values.slice(-bars)

    Row {
        visible: root.style === "bars"
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        spacing: 2
        layoutDirection: Qt.RightToLeft
        Repeater {
            model: root.style === "bars" ? root.shown.slice().reverse() : []
            Rectangle {
                required property real modelData
                width: Math.max(2, (root.width - (root.bars - 1) * 2) / root.bars)
                height: Math.max(2, root.height * Math.min(1, modelData))
                anchors.bottom: parent.bottom
                radius: 1
                color: root.color
                opacity: 0.8
            }
        }
    }

    // The line runs through one point per slot, so a short history sits at the right the way the bars do.
    Canvas {
        id: plot
        visible: root.style !== "bars"
        anchors.fill: parent
        onWidthChanged: requestPaint()
        onHeightChanged: requestPaint()
        Connections { target: root; function onShownChanged() { plot.requestPaint(); } function onColorChanged() { plot.requestPaint(); } function onStyleChanged() { plot.requestPaint(); } }
        onPaint: {
            const ctx = getContext("2d"), v = root.shown, n = root.bars;
            ctx.reset();
            if (v.length < 2) return;
            const step = width / Math.max(1, n - 1), x0 = width - (v.length - 1) * step;
            const yOf = val => height - 1 - Math.min(1, Math.max(0, val)) * (height - 2);
            ctx.beginPath();
            ctx.moveTo(x0, yOf(v[0]));
            for (let i = 1; i < v.length; i++) ctx.lineTo(x0 + i * step, yOf(v[i]));
            if (root.style === "area") {
                ctx.lineTo(width, height); ctx.lineTo(x0, height); ctx.closePath();
                ctx.fillStyle = Qt.alpha(root.color, 0.22); ctx.fill();
                ctx.beginPath();
                ctx.moveTo(x0, yOf(v[0]));
                for (let i = 1; i < v.length; i++) ctx.lineTo(x0 + i * step, yOf(v[i]));
            }
            ctx.strokeStyle = root.color; ctx.lineWidth = 1.5; ctx.lineJoin = "round"; ctx.stroke();
        }
    }
}
