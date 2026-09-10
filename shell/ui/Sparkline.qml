import QtQuick
import QtQuick.Shapes
import qs.theme

// The last `bars` of one or more 0..1 series, newest on the right: bars for one series, a line or a filled
// area for any number, each in its colour. Drawn with Shapes, so a new sample never blanks the frame.
Item {
    id: root
    property var values: []
    property color color: Theme.text2
    // Several series at once: [{ values, color }]; when set, `values` and `color` are ignored.
    property var series: []
    property int bars: 24
    // "bars", "line" or "area".
    property string style: "bars"
    // The area's opacity, 0..1.
    property real fill: 0.22
    implicitHeight: 28

    readonly property var all: series.length ? series : [{ values: values, color: color }]
    readonly property var shown: values.slice(-bars)

    Row {
        visible: root.style === "bars" && root.series.length === 0
        anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
        spacing: 2
        layoutDirection: Qt.RightToLeft
        Repeater {
            model: root.style === "bars" && root.series.length === 0 ? root.shown.slice().reverse() : []
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

    // One shape per series; a short history sits at the right the way the bars do.
    Repeater {
        model: root.style === "bars" && root.series.length === 0 ? [] : root.all
        Shape {
            id: shape
            required property var modelData
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            readonly property var pts: {
                const v = (modelData.values || []).slice(-root.bars), n = root.bars, out = [];
                if (v.length < 2) return out;
                const step = root.width / Math.max(1, n - 1), x0 = root.width - (v.length - 1) * step;
                for (let i = 0; i < v.length; i++) out.push(Qt.point(x0 + i * step, root.height - 1 - Math.min(1, Math.max(0, v[i])) * (root.height - 2)));
                return out;
            }
            ShapePath {
                strokeColor: "transparent"
                fillColor: root.style === "area" ? Qt.alpha(shape.modelData.color, root.fill) : "transparent"
                startX: shape.pts.length ? shape.pts[0].x : 0
                startY: root.height
                PathPolyline { path: shape.pts.length ? [Qt.point(shape.pts[0].x, root.height)].concat(shape.pts, [Qt.point(root.width, root.height)]) : [] }
            }
            ShapePath {
                strokeColor: shape.modelData.color
                strokeWidth: 1.5
                fillColor: "transparent"
                joinStyle: ShapePath.RoundJoin
                startX: shape.pts.length ? shape.pts[0].x : 0
                startY: shape.pts.length ? shape.pts[0].y : 0
                PathPolyline { path: shape.pts }
            }
        }
    }
}
