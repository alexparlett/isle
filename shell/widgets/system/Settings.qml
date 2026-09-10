import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// The System card's own settings pane: a list of series, each a measure in a colour, and how they are drawn.
Item {
    id: root
    property var settings: ({})
    property var manifest: ({})
    // The card supplies this; every change is written through it.
    property var set: function(key, value) {}

    readonly property var measures: [["cpu", "CPU"], ["gpu", "GPU"], ["memory", "Memory"], ["network", "Network ↓"], ["netUp", "Network ↑"], ["cpuTemp", "CPU temperature"], ["gpuTemp", "GPU temperature"], ["boardTemp", "Board temperature"], ["driveTemp", "Drive temperature"], ["fans", "Fan speed"]]
    readonly property var swatches: [String(Theme.accent), "#B48CFF", String(Theme.ok), "#5AC8FA", String(Theme.warn), "#FF9F5A", "#E8A0C8", "#8FD1B6", String(Theme.danger), String(Theme.text2)]
    // A list read back from prefs is a sequence, not an Array; copy it into one.
    readonly property var series: { const l = settings.series || [], out = []; for (let i = 0; i < (l.length || 0); i++) out.push(l[i]); return out; }
    // Which row has its swatches open, or -1.
    property int picking: -1

    function write(list) { root.set("series", list); }
    function update(i, patch) { const l = series.map((s, j) => j === i ? Object.assign({}, s, patch) : s); write(l); }
    function remove(i) { root.picking = -1; write(series.filter((s, j) => j !== i)); }
    function addSeries() {
        const used = series.map(s => s.measure), m = measures.find(x => used.indexOf(x[0]) < 0) || measures[0];
        const usedColours = series.map(s => String(s.color).toLowerCase()), c = swatches.find(x => usedColours.indexOf(String(x).toLowerCase()) < 0) || swatches[series.length % swatches.length];
        write(series.concat([{ measure: m[0], color: c }]));
    }

    implicitWidth: 360
    implicitHeight: col.implicitHeight

    ColumnLayout {
        id: col
        anchors { left: parent.left; right: parent.right }
        spacing: Theme.s2

        Label { text: "Series"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3 }
        Repeater {
            model: root.series
            ColumnLayout {
                id: row
                required property var modelData
                required property int index
                Layout.fillWidth: true
                spacing: Theme.s1
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.s2
                    // The colour: click for the swatches.
                    Rectangle {
                        width: 20; height: 20; radius: 5
                        color: row.modelData.color || Theme.text2
                        border.width: root.picking === row.index ? 2 : 1
                        border.color: root.picking === row.index ? Theme.text : Theme.hairlineStrong
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.picking = root.picking === row.index ? -1 : row.index }
                    }
                    Dropdown { Layout.fillWidth: true; implicitHeight: 28; listWidth: 200; options: root.measures; value: row.modelData.measure; onPicked: v => root.update(row.index, { measure: v }) }
                    Rectangle {
                        width: 24; height: 24; radius: 12
                        color: removeArea.containsMouse ? Theme.pressed : "transparent"
                        Glyph { anchors.centerIn: parent; name: "x"; size: 11; weight: 1.6; color: Theme.text2 }
                        MouseArea { id: removeArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.remove(row.index) }
                    }
                }
                Row {
                    visible: root.picking === row.index
                    Layout.leftMargin: 28
                    spacing: 6
                    Repeater {
                        model: root.swatches
                        Rectangle {
                            required property string modelData
                            width: 18; height: 18; radius: 5
                            color: modelData
                            border.width: String(row.modelData.color).toLowerCase() === modelData.toLowerCase() ? 2 : 0
                            border.color: Theme.text
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { root.picking = -1; root.update(row.index, { color: parent.modelData }); } }
                        }
                    }
                }
            }
        }
        Label {
            visible: root.series.length === 0
            text: "Nothing plotted yet"; size: Theme.sizeSmall; color: Theme.text3
        }
        Button { text: "Add series"; glyph: "plus"; variant: "text"; implicitHeight: 28; onClicked: root.addSeries() }

        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.hairline }

        RowLayout {
            Layout.fillWidth: true
            Label { text: "Style"; size: Theme.sizeSmall; Layout.fillWidth: true }
            Segmented { options: [["bars", "Bars"], ["line", "Line"], ["area", "Area"]]; value: root.settings.style || "bars"; onPicked: v => root.set("style", v) }
        }
        RowLayout {
            Layout.fillWidth: true
            Label { text: "Layout"; size: Theme.sizeSmall; Layout.fillWidth: true }
            Segmented { options: [["auto", "Auto"], ["panels", "Panels"], ["one", "One chart"]]; value: root.settings.layout || "auto"; onPicked: v => root.set("layout", v) }
        }
        RowLayout {
            visible: (root.settings.style || "bars") === "area"
            Layout.fillWidth: true
            spacing: Theme.s2
            Label { text: "Fill"; size: Theme.sizeSmall; Layout.fillWidth: true }
            Slider { implicitWidth: 90; value: (root.settings.fill !== undefined ? root.settings.fill : 22) / 100; onMoved: f => root.set("fill", Math.round(f * 100)) }
            Label { text: (root.settings.fill !== undefined ? root.settings.fill : 22) + "%"; mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 32; horizontalAlignment: Text.AlignRight }
        }
        RowLayout {
            Layout.fillWidth: true
            Label { text: "Legend"; size: Theme.sizeSmall; Layout.fillWidth: true }
            Toggle { checked: root.settings.legend !== false; onToggled: v => root.set("legend", v) }
        }
    }
}
