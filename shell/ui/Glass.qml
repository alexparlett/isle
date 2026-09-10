import QtQuick
import qs.theme

// The material: glass with a hairline. Blur comes from the compositor's layer rule.
Rectangle {
    property bool opaque: false
    color: opaque ? Theme.glassOpaque : Theme.glass
    border.width: 1
    border.color: Theme.hairline
    radius: Theme.radiusPanel
    antialiasing: true
}
