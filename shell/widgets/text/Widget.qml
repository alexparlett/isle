import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import qs.theme
import qs.ui

// The renderer for a manifest-only widget: a command or a file, read every `interval` seconds, shown as
// text, a number, a gauge against `max`, a sparkline of the numbers so far, or a list of lines.
WidgetBase {
    id: root
    title: manifest ? (manifest.title !== undefined ? manifest.title : manifest.name) : ""
    meta: error ? "failed" : ""

    readonly property var source: manifest && manifest.source ? manifest.source : ({})
    readonly property string view: manifest && manifest.view ? manifest.view : "text"
    readonly property real max: manifest && manifest.max ? Number(manifest.max) : 100
    readonly property string unit: manifest && manifest.unit ? manifest.unit : ""
    property string output: ""
    property string error: ""
    property var history: []
    readonly property real number: { const m = output.match(/-?\d+(\.\d+)?/); return m ? Number(m[0]) : NaN; }
    readonly property string figure: isNaN(number) ? "–" : (Number.isInteger(number) ? String(number) : number.toFixed(1))

    Process {
        id: reader
        command: source.file ? ["cat", "--", source.file] : ["sh", "-c", source.command || "true"]
        stdout: StdioCollector { onStreamFinished: { root.output = text.trim(); if (root.view === "sparkline" && !isNaN(root.number)) root.history = root.history.concat([root.number]).slice(-60); } }
        stderr: StdioCollector { id: err }
        onExited: code => root.error = code === 0 ? "" : (err.text.trim().split("\n").pop() || "exit " + code)
    }
    // A change while a read is under way is picked up when it finishes; the refresh is deferred past the
    // bindings that follow a manifest change, so the reader starts with the new command and not the old.
    function refresh() { if (reader.running) { stale = true; return; } reader.running = true; }
    property bool stale: false
    Timer { interval: Math.max(1, Number(source.interval) || 30) * 1000; running: !!root.manifest; repeat: true; onTriggered: root.refresh() }
    onSourceChanged: Qt.callLater(root.refresh)
    Connections { target: reader; function onExited() { if (root.stale) { root.stale = false; Qt.callLater(root.refresh); } } }

    // text
    Label {
        visible: root.view === "text"
        anchors.fill: parent
        text: root.error || root.output || "…"
        color: root.error ? Theme.danger : Theme.text
        mono: !!(root.manifest && root.manifest.mono)
        wrapMode: Text.Wrap
        elide: Text.ElideRight
        maximumLineCount: Math.max(1, Math.floor(height / 20))
        size: root.rows > 1 ? Theme.sizeBody : Theme.sizeSmall
    }
    // number and gauge
    ColumnLayout {
        visible: root.view === "number" || root.view === "gauge"
        anchors.fill: parent
        spacing: Theme.s2
        RowLayout {
            spacing: Theme.s1
            Label { text: root.figure; size: root.rows > 1 ? 40 : 30; weight: Font.DemiBold; tabular: true; font.letterSpacing: -1 }
            Label { text: root.unit; size: Theme.sizeBody; color: Theme.text2; Layout.alignment: Qt.AlignBottom; Layout.bottomMargin: root.rows > 1 ? 8 : 5 }
        }
        Rectangle {
            visible: root.view === "gauge"
            Layout.fillWidth: true
            height: 4; radius: 2
            color: Theme.hairlineStrong
            Rectangle { width: parent.width * Math.max(0, Math.min(1, root.number / root.max)); height: parent.height; radius: 2; color: Theme.accent }
        }
        Label { visible: root.error !== ""; text: root.error; size: Theme.sizeCaption; color: Theme.danger; elide: Text.ElideRight; Layout.fillWidth: true }
        Item { Layout.fillHeight: true }
    }
    // sparkline
    ColumnLayout {
        visible: root.view === "sparkline"
        anchors.fill: parent
        spacing: Theme.s2
        RowLayout {
            spacing: Theme.s1
            Label { text: root.figure; size: 24; weight: Font.DemiBold; tabular: true }
            Label { text: root.unit; size: Theme.sizeSmall; color: Theme.text2; Layout.alignment: Qt.AlignBottom; Layout.bottomMargin: 3 }
        }
        Sparkline { Layout.fillWidth: true; Layout.fillHeight: true; values: root.history; color: Theme.accent; bars: 40 }
    }
    // list: a row per line; "label: value" and "label = value" split into two columns
    ColumnLayout {
        visible: root.view === "list"
        anchors.fill: parent
        spacing: 2
        Repeater {
            model: root.view === "list" ? (root.error ? [root.error] : root.output.split("\n").filter(l => l.trim() !== "").slice(0, Math.max(1, Math.floor(root.height / 20)))) : []
            RowLayout {
                required property string modelData
                readonly property var parts: { const m = modelData.match(/^\s*([^:=\t]+?)\s*[:=\t]\s*(.+?)\s*$/); return m ? [m[1], m[2]] : [modelData.trim(), ""]; }
                Layout.fillWidth: true
                spacing: Theme.s2
                Label { text: parent.parts[0]; size: Theme.sizeSmall; color: parent.parts[1] ? Theme.text2 : Theme.text; elide: Text.ElideRight; Layout.fillWidth: true }
                Label { visible: parent.parts[1] !== ""; text: parent.parts[1]; size: Theme.sizeSmall; mono: true; tabular: true; elide: Text.ElideRight; Layout.maximumWidth: root.width * 0.55 }
            }
        }
        Item { Layout.fillHeight: true }
    }
}
