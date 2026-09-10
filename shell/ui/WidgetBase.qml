import QtQuick

// What every widget's root is. The card draws the chrome; a widget sets `title`, `meta`, and draws its content.
Item {
    property var manifest: null
    property string size: "3x1"
    property string title: ""
    property string meta: ""
    property var settings: ({})

    readonly property int cols: parseInt(size.split("x")[0]) || 3
    readonly property int rows: parseInt(size.split("x")[1]) || 1
}
