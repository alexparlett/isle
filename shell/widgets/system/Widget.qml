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

    function tone(v) { return v >= 85 ? Theme.danger : v >= 70 ? Theme.warn : Theme.text2; }
    // The panels to draw, in a fixed order, each with its label, figure, history and colour.
    readonly property var panels: {
        const s = root.settings, out = [];
        if (s.cpu !== false) out.push({ label: "CPU", value: Math.round(System.cpu * 100) + "%", history: System.cpuHistory, tint: Theme.text2 });
        if (s.gpu !== false) out.push({ label: "GPU", value: System.gpu < 0 ? "—" : Math.round(System.gpu * 100) + "%", history: System.gpuHistory, tint: Theme.accent });
        if (s.memory !== false) out.push({ label: "Memory", value: System.bytes(System.memUsed), history: System.memHistory, tint: Theme.text2 });
        if (s.network !== false) out.push({ label: "Network", value: "↓ " + System.rate(System.netDown), history: System.netHistory.map(v => Math.min(1, v / 1e7)), tint: Theme.text2 });
        if (s.cpuTemp) out.push({ label: "CPU °", value: cpuTemp ? Math.round(cpuTemp.value) + "°" : "—", history: cpuTempHistory, tint: tone(cpuTemp ? cpuTemp.value : 0) });
        if (s.gpuTemp) out.push({ label: "GPU °", value: gpuTempValue ? Math.round(gpuTempValue) + "°" : "—", history: gpuTempHistory, tint: tone(gpuTempValue) });
        if (s.boardTemp) out.push({ label: "Board °", value: boardTemp ? Math.round(boardTemp.value) + "°" : "—", history: boardTempHistory, tint: tone(boardTemp ? boardTemp.value : 0) });
        if (s.driveTemp) out.push({ label: "Drive °", value: driveTemp ? Math.round(driveTemp.value) + "°" : "—", history: driveTempHistory, tint: tone(driveTemp ? driveTemp.value : 0) });
        if (s.fans) out.push({ label: "Fans", value: fastestFan ? fastestFan.rpm + " rpm" : "—", history: fanHistory, tint: Theme.text2 });
        return out;
    }

    GridLayout {
        anchors.fill: parent
        // Two across, as the card has always been; more than four panels on a wide card go four across.
        columns: Math.max(1, Math.min(root.panels.length > 4 && root.cols >= 6 ? 4 : 2, root.panels.length))
        columnSpacing: Theme.s2 - 2
        rowSpacing: Theme.s2 - 2
        Repeater {
            model: root.panels
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
                    Sparkline { Layout.fillWidth: true; Layout.fillHeight: true; values: modelData.history; color: modelData.tint }
                }
            }
        }
        Label { visible: root.panels.length === 0; Layout.columnSpan: parent.columns; Layout.alignment: Qt.AlignHCenter; text: "Pick something to plot in the settings"; color: Theme.text3; size: Theme.sizeSmall }
    }
}
