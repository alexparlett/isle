import QtQuick
import qs.theme
import "../assets/Icons.js" as Icons

// A Lucide glyph by name, drawn in a `size` box in `color`. An Item, so layouts size it by the box and not by the raster.
Item {
    id: root
    property string name
    property int size: 16
    property color color: Theme.text2
    property real weight: Theme.glyphWeight
    readonly property real px: Math.round(size * Theme.glyphScale)

    // SVG takes an opaque colour; the token's alpha is applied as opacity.
    readonly property string rgb: Qt.rgba(color.r, color.g, color.b, 1).toString()

    implicitWidth: px
    implicitHeight: px
    width: px
    height: px
    opacity: color.a

    Image {
        anchors.fill: parent
        // Rasterised at 2× and scaled, which keeps the strokes smooth at these sizes.
        sourceSize: Qt.size(root.px * 2, root.px * 2)
        source: Icons.url(root.name, root.rgb, root.px * 2, root.weight)
        smooth: true
        fillMode: Image.PreserveAspectFit
    }
}
