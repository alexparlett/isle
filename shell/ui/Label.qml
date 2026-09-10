import QtQuick
import qs.theme

// Inter at a named size. `mono` switches to JetBrains Mono, `tabular` to tabular figures.
Text {
    property bool mono: false
    property bool tabular: false
    property int size: Theme.sizeBody
    property int weight: Font.Medium

    color: Theme.text
    font.family: mono ? Theme.fontMono : Theme.fontUi
    font.pixelSize: size
    font.weight: weight
    font.features: tabular ? { "tnum": 1 } : {}
    renderType: Text.NativeRendering
    verticalAlignment: Text.AlignVCenter
    elide: Text.ElideRight
}
