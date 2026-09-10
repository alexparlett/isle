import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// A panel with a graph for each measure picked in the gear. Load, memory and network come with their
// histories from System; temperatures and fans are sampled by Monitor and kept here for the last minute.
WidgetBase {
    id: root
    title: "System"
    meta: (System.uptime ? "up " + System.uptime : "") + (System.kernel ? " · " + System.kernel : "")

    Component.onCompleted: { System.listeners++; Monitor.listeners++; }
    Component.onDestruction: { System.listeners--; Monitor.listeners--; }

    // The hottest reading of a chip family, or null.
    function hottest(prefixes) {
        let best = null;
        for (const t of Monitor.temps || []) if (prefixes.some(p => t.chip.indexOf(p) === 0) && t.value > 0 && (!best || t.value > best.value)) best = t;
        return best;
    }
    readonly property var cpuTemp: hottest(["k10temp", "coretemp", "zenpower"])
    readonly property var boardTemp: hottest(["nct", "it87", "asus"])
    readonly property var driveTemp: hottest(["nvme", "drivetemp"])
    readonly property real gpuTempValue: Monitor.gpu && Monitor.gpu.temp ? Monitor.gpu.temp : (System.gpuTemp || 0)
    readonly property var fastestFan: (Monitor.fans || []).reduce((m, f) => f.rpm > (m ? m.rpm : 0) ? f : m, null)

    // Histories for what System does not keep, sampled with Monitor.
    property var cpuTempHistory: []
    property var gpuTempHistory: []
    property var boardTempHistory: []
    property var driveTempHistory: []
    property var fanHistory: []
    function push(h, v) { return h.concat([v]).slice(-60); }
    Connections {
        target: Monitor
        function onTempsChanged() {
            root.cpuTempHistory = root.push(root.cpuTempHistory, root.cpuTemp ? root.cpuTemp.value / 100 : 0);
            root.boardTempHistory = root.push(root.boardTempHistory, root.boardTemp ? root.boardTemp.value / 100 : 0);
            root.driveTempHistory = root.push(root.driveTempHistory, root.driveTemp ? root.driveTemp.value / 100 : 0);
            root.gpuTempHistory = root.push(root.gpuTempHistory, root.gpuTempValue / 100);
            root.fanHistory = root.push(root.fanHistory, root.fastestFan ? Math.min(1, root.fastestFan.rpm / 3000) : 0);
        }
    }

    // Each measure's figure and history; series pick from these. Network and fans are scaled to a full bar
    // at 10 MB/s and 3000 rpm; temperatures at 100°.
    readonly property var measures: ({
        cpu: { label: "CPU", value: Math.round(System.cpu * 100) + "%", history: System.cpuHistory },
        gpu: { label: "GPU", value: System.gpu < 0 ? "—" : Math.round(System.gpu * 100) + "%", history: System.gpuHistory },
        memory: { label: "Memory", value: System.bytes(System.memUsed), history: System.memHistory },
        network: { label: "Network ↓", value: System.rate(System.netDown), history: System.netHistory.map(v => Math.min(1, v / 1e7)) },
        netUp: { label: "Network ↑", value: System.rate(System.netUp), history: System.netUpHistory.map(v => Math.min(1, v / 1e7)) },
        cpuTemp: { label: "CPU °", value: cpuTemp ? Math.round(cpuTemp.value) + "°" : "—", history: cpuTempHistory, hot: cpuTemp && cpuTemp.value >= 85 },
        gpuTemp: { label: "GPU °", value: gpuTempValue ? Math.round(gpuTempValue) + "°" : "—", history: gpuTempHistory, hot: gpuTempValue >= 85 },
        boardTemp: { label: "Board °", value: boardTemp ? Math.round(boardTemp.value) + "°" : "—", history: boardTempHistory, hot: boardTemp && boardTemp.value >= 85 },
        driveTemp: { label: "Drive °", value: driveTemp ? Math.round(driveTemp.value) + "°" : "—", history: driveTempHistory, hot: driveTemp && driveTemp.value >= 85 },
        fans: { label: "Fans", value: fastestFan ? fastestFan.rpm + " rpm" : "—", history: fanHistory }
    })
    // The series to draw, in the gear's order: a measure each, in its colour, red while it runs hot.
    readonly property var panels: {
        const out = [];
        // A list read back from prefs is a sequence, not an Array; index it.
        const list = root.settings.series || [];
        for (let i = 0; i < (list.length || 0); i++) {
            const s = list[i];
            const m = root.measures[s.measure];
            if (m) out.push(Object.assign({ key: s.measure, tint: m.hot ? Theme.danger : (s.color || Theme.text2) }, m));
        }
        return out;
    }
    readonly property string style: root.settings.style || "bars"
    // One chart when asked, or when auto and there is under two cells per panel.
    readonly property bool combined: panels.length > 1 && (root.settings.layout === "one" || (root.settings.layout !== "panels" && root.cols * root.rows < panels.length * 2))

    // One chart for all, when the card is small.
    Rectangle {
        visible: root.combined
        anchors.fill: parent
        radius: Theme.radiusCard
        color: Theme.raised
        border.width: 1
        border.color: Theme.hairline
        ColumnLayout {
            anchors { fill: parent; margins: Theme.s2 + 2 }
            spacing: Theme.s1
            Flow {
                visible: root.settings.legend !== false
                Layout.fillWidth: true
                spacing: Theme.s2
                Repeater {
                    model: root.combined && root.settings.legend !== false ? root.panels : []
                    Row {
                        required property var modelData
                        spacing: 4
                        Rectangle { width: 8; height: 8; radius: 4; color: parent.modelData.tint; anchors.verticalCenter: parent.verticalCenter }
                        Label { text: parent.modelData.label + " " + parent.modelData.value; size: Theme.sizeCaption; tabular: true; color: Theme.text2 }
                    }
                }
            }
            Sparkline { Layout.fillWidth: true; Layout.fillHeight: true; style: root.style === "bars" ? "line" : root.style; fill: (root.settings.fill !== undefined ? root.settings.fill : 22) / 100; series: root.combined ? root.panels.map(p => ({ values: p.history, color: p.tint })) : [] }
        }
    }

    GridLayout {
        visible: !root.combined
        anchors.fill: parent
        // Two across, as the card has always been; more than four panels on a wide card go four across.
        columns: Math.max(1, Math.min(root.panels.length > 4 && root.cols >= 6 ? 4 : 2, root.panels.length))
        columnSpacing: Theme.s2 - 2
        rowSpacing: Theme.s2 - 2
        Repeater {
            model: root.combined ? [] : root.panels
            Rectangle {
                required property var modelData
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: Theme.radiusCard
                color: Theme.raised
                border.width: 1
                border.color: Theme.hairline
                ColumnLayout {
                    anchors { fill: parent; margins: Theme.s2 + 2 }
                    spacing: Theme.s1
                    RowLayout {
                        Layout.fillWidth: true
                        Label { text: modelData.label; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2; Layout.fillWidth: true }
                        Label { text: modelData.value; mono: true; tabular: true; size: Theme.sizeCaption }
                    }
                    Sparkline { Layout.fillWidth: true; Layout.fillHeight: true; values: modelData.history; color: modelData.tint; style: root.style; fill: (root.settings.fill !== undefined ? root.settings.fill : 22) / 100 }
                }
            }
        }
        Label { visible: root.panels.length === 0; Layout.columnSpan: parent.columns; Layout.alignment: Qt.AlignHCenter; text: "Add a series in the settings"; color: Theme.text3; size: Theme.sizeSmall }
    }
}
