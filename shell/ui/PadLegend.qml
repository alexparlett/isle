import QtQuick
import qs.theme

// A row of the pad's buttons and what they do: `items` are [{ keys: ["a"], label }], drawn in the labels of
// the `kind` of pad (sony, xbox, nintendo); it wraps rather than widen.
Flow {
    id: root
    property var items: []
    property string kind: ""
    // Ten-foot: larger chips and text.
    property bool big: false
    spacing: big ? Theme.s4 : Theme.s3

    readonly property var glyphs: ({
        sony: { a: ["✕", "#7FA6FF"], b: ["○", "#FF6B6B"], x: ["□", "#E8A0C8"], y: ["△", "#6FCF97"], lb: ["L1"], rb: ["R1"], lt: ["L2"], rt: ["R2"], start: ["Options"], select: ["Share"], guide: ["PS"] },
        nintendo: { a: ["B"], b: ["A"], x: ["Y"], y: ["X"], lb: ["L"], rb: ["R"], lt: ["ZL"], rt: ["ZR"], start: ["+"], select: ["−"], guide: ["Home"] },
        xbox: { a: ["A", "#6FCF97"], b: ["B", "#FF6B6B"], x: ["X", "#7FA6FF"], y: ["Y", "#F2C94C"], lb: ["LB"], rb: ["RB"], lt: ["LT"], rt: ["RT"], start: ["Menu"], select: ["View"], guide: ["Xbox"] }
    })
    readonly property var shared: ({ left: ["◀"], right: ["▶"], up: ["▲"], down: ["▼"] })
    readonly property var map: Object.assign({}, shared, glyphs[kind] || glyphs.xbox)

    Repeater {
        model: root.items
        Row {
            required property var modelData
            spacing: root.big ? 8 : 4
            Repeater {
                model: parent.modelData.keys
                Rectangle {
                    required property string modelData
                    readonly property var g: root.map[modelData] || [modelData]
                    readonly property int h: root.big ? 28 : 18
                    width: Math.max(h, chip.implicitWidth + (root.big ? 14 : 8)); height: h; radius: h / 2
                    color: Theme.pressed; border.width: 1; border.color: g[1] ? Qt.alpha(g[1], 0.6) : Theme.hairlineStrong
                    Label { id: chip; anchors.centerIn: parent; text: parent.g[0]; size: root.big ? Theme.sizeBody : Theme.sizeCaption; weight: Font.DemiBold; color: parent.g[1] || Theme.text2 }
                }
            }
            Label { text: parent.modelData.label; size: root.big ? Theme.sizeHeading : Theme.sizeCaption; color: Theme.text3; anchors.verticalCenter: parent.verticalCenter }
        }
    }
}
