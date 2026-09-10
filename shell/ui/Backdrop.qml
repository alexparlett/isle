import QtQuick
import QtQuick.Shapes
import QtQuick.Effects
import qs.theme

// The wallpaper: an image when one is set, otherwise the drawn backdrop. `blur` and `dim` are for the lock screen.
Item {
    id: root
    property string image: ""
    property bool blur: false
    property real dim: 0

    Item {
        id: content
        anchors.fill: parent
        visible: !root.blur
        layer.enabled: root.blur

        Rectangle { anchors.fill: parent; color: Theme.ink }

        Image {
            anchors.fill: parent
            visible: root.image !== ""
            source: root.image
            fillMode: Image.PreserveAspectCrop
            asynchronous: true
        }

        Shape {
            anchors.fill: parent
            visible: root.image === ""
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeWidth: -1
                fillGradient: RadialGradient {
                    centerX: content.width * 0.78; centerY: content.height * 1.15
                    focalX: centerX; focalY: centerY
                    centerRadius: content.width * 0.6
                    GradientStop { position: 0; color: "#1A2237" }
                    GradientStop { position: 1; color: "transparent" }
                }
                PathSvg { path: "M0 0 H" + content.width + " V" + content.height + " H0 Z" }
            }
            ShapePath {
                strokeWidth: -1
                fillGradient: RadialGradient {
                    centerX: content.width * 0.08; centerY: -content.height * 0.1
                    focalX: centerX; focalY: centerY
                    centerRadius: content.width * 0.45
                    GradientStop { position: 0; color: "#1A1626" }
                    GradientStop { position: 1; color: "transparent" }
                }
                PathSvg { path: "M0 0 H" + content.width + " V" + content.height + " H0 Z" }
            }
        }
    }

    MultiEffect {
        anchors.fill: parent
        visible: root.blur
        source: content
        blurEnabled: root.blur
        blur: 1
        blurMax: 48
        blurMultiplier: 1.5
    }

    Rectangle { anchors.fill: parent; color: Theme.ink; opacity: root.dim }
}
