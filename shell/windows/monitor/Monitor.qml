import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui
import qs.services

// The Monitor window: processes by what they use (CPU, memory, energy, disk, network), each tab with its own columns
// and a summary strip beneath, and a Sensors tab for temperatures, fans and the GPU.
FloatingWindow {
    id: root
    title: "Monitor"
    visible: Surfaces.monitor
    implicitWidth: 1180
    implicitHeight: 720
    minimumSize: Qt.size(900, 520)
    color: Theme.light ? "#FFFFFF" : "#131417"
    onVisibleChanged: {
        if (!visible) Surfaces.monitor = false;
        System.listeners += visible ? 1 : -1;
        Monitor.listeners += visible ? 1 : -1;
    }

    property string tab: "cpu"
    readonly property var tabs: [["cpu", "CPU"], ["memory", "Memory"], ["energy", "Energy"], ["disk", "Disk"], ["network", "Network"], ["sensors", "Sensors"]]

    // Columns per tab. The first numeric column is the default sort.
    readonly property var columnSets: ({
        cpu: [
            { key: "name", label: "Process", width: 0, align: Text.AlignLeft },
            { key: "cpu", label: "% CPU", width: 70, align: Text.AlignRight },
            { key: "cputime", label: "CPU time", width: 90, align: Text.AlignRight },
            { key: "threads", label: "Threads", width: 64, align: Text.AlignRight },
            { key: "state", label: "State", width: 72, align: Text.AlignLeft },
            { key: "pid", label: "PID", width: 60, align: Text.AlignRight },
            { key: "user", label: "User", width: 80, align: Text.AlignLeft },
        ],
        memory: [
            { key: "name", label: "Process", width: 0, align: Text.AlignLeft },
            { key: "rss", label: "Memory", width: 84, align: Text.AlignRight },
            { key: "memPct", label: "% Memory", width: 84, align: Text.AlignRight },
            { key: "gpuMem", label: "GPU memory", width: 90, align: Text.AlignRight },
            { key: "threads", label: "Threads", width: 64, align: Text.AlignRight },
            { key: "pid", label: "PID", width: 60, align: Text.AlignRight },
            { key: "user", label: "User", width: 80, align: Text.AlignLeft },
        ],
        energy: [
            { key: "name", label: "Process", width: 0, align: Text.AlignLeft },
            { key: "energy", label: "Energy impact", width: 100, align: Text.AlignRight },
            { key: "cpu", label: "% CPU", width: 70, align: Text.AlignRight },
            { key: "gpuMem", label: "GPU memory", width: 90, align: Text.AlignRight },
            { key: "inhibit", label: "Preventing sleep", width: 110, align: Text.AlignLeft },
            { key: "pid", label: "PID", width: 60, align: Text.AlignRight },
            { key: "user", label: "User", width: 80, align: Text.AlignLeft },
        ],
        disk: [
            { key: "name", label: "Process", width: 0, align: Text.AlignLeft },
            { key: "ioRead", label: "Read/s", width: 84, align: Text.AlignRight },
            { key: "ioWrite", label: "Written/s", width: 84, align: Text.AlignRight },
            { key: "ioReadTotal", label: "Bytes read", width: 90, align: Text.AlignRight },
            { key: "ioWriteTotal", label: "Bytes written", width: 100, align: Text.AlignRight },
            { key: "pid", label: "PID", width: 60, align: Text.AlignRight },
            { key: "user", label: "User", width: 80, align: Text.AlignLeft },
        ],
        network: [
            { key: "name", label: "Process", width: 0, align: Text.AlignLeft },
            { key: "netDown", label: "Received/s", width: 90, align: Text.AlignRight },
            { key: "netUp", label: "Sent/s", width: 90, align: Text.AlignRight },
            { key: "pid", label: "PID", width: 60, align: Text.AlignRight },
            { key: "user", label: "User", width: 80, align: Text.AlignLeft },
        ],
    })
    readonly property var defaultSort: ({ cpu: "cpu", memory: "rss", energy: "energy", disk: "ioWriteTotal", network: "netDown" })
    onTabChanged: if (columnSets[tab]) { table.sortKey = defaultSort[tab]; table.sortDesc = true; }

    component Figure: ColumnLayout {
        property string label
        property string value
        property color tint: Theme.text
        spacing: 0
        Label { text: label; size: Theme.sizeCaption; color: Theme.text3 }
        Label { text: value; mono: true; tabular: true; size: Theme.sizeSmall; color: tint }
    }

    ColumnLayout {
        anchors { fill: parent; margins: Theme.s5 }
        spacing: Theme.s4

        RowLayout {
            spacing: Theme.s4
            Label { text: "Monitor"; size: Theme.sizeTitle; weight: Font.DemiBold }
            Rectangle {
                height: 28; radius: Theme.radiusChip; color: Theme.pressed
                width: tabRow.implicitWidth + 4
                RowLayout {
                    id: tabRow
                    anchors.centerIn: parent
                    spacing: 2
                    Repeater {
                        model: root.tabs
                        Rectangle {
                            required property var modelData
                            readonly property bool sel: root.tab === modelData[0]
                            implicitHeight: 24; implicitWidth: tabLabel.implicitWidth + Theme.s3 * 2
                            radius: Theme.radiusChip - 2
                            color: sel ? Theme.raised : "transparent"
                            Label { id: tabLabel; anchors.centerIn: parent; text: modelData[1]; size: Theme.sizeSmall; weight: Font.DemiBold; color: parent.sel ? Theme.text : Theme.text3 }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.tab = modelData[0] }
                        }
                    }
                }
            }
            Item { Layout.fillWidth: true }
            Label { text: (System.uptime ? "up " + System.uptime : "") + (System.kernel ? "  ·  " + System.kernel : ""); size: Theme.sizeCaption; color: Theme.text3 }
        }

        ProcessTable {
            id: table
            visible: root.tab !== "sensors"
            Layout.fillWidth: true
            Layout.fillHeight: true
            columns: root.columnSets[root.tab] || root.columnSets.cpu
            sortKey: "cpu"
        }

        // The strip under the table: the system's totals for what the tab shows, with a short history.
        Card {
            visible: root.tab !== "sensors"
            Layout.fillWidth: true
            padding: Theme.s3
            RowLayout {
                width: parent.width
                spacing: Theme.s5

                // CPU
                RowLayout {
                    visible: root.tab === "cpu"
                    spacing: Theme.s5
                    Figure { label: "System"; value: Monitor.summary.cpu ? Monitor.summary.cpu.system.toFixed(1) + "%" : "—"; tint: Theme.danger }
                    Figure { label: "User"; value: Monitor.summary.cpu ? Monitor.summary.cpu.user.toFixed(1) + "%" : "—"; tint: Theme.accent }
                    Figure { label: "Idle"; value: Monitor.summary.cpu ? Monitor.summary.cpu.idle.toFixed(1) + "%" : "—"; tint: Theme.text2 }
                    Figure { label: "Load"; value: Monitor.summary.load.join("  ") }
                    Figure { label: "Cores"; value: String(Monitor.summary.cpus) }
                    Figure { label: "Processes"; value: String(Monitor.summary.count) }
                    Figure { label: "Threads"; value: String(Monitor.summary.threads) }
                }
                // Memory
                RowLayout {
                    visible: root.tab === "memory" && !!Monitor.summary.mem
                    spacing: Theme.s5
                    Figure { label: "Physical"; value: Monitor.summary.mem ? System.bytes(Monitor.summary.mem.total) : "" }
                    Figure { label: "Used"; value: Monitor.summary.mem ? System.bytes(Monitor.summary.mem.total - Monitor.summary.mem.available) : ""; tint: Theme.accent }
                    Figure { label: "Cached"; value: Monitor.summary.mem ? System.bytes(Monitor.summary.mem.cached) : "" }
                    Figure { label: "Free"; value: Monitor.summary.mem ? System.bytes(Monitor.summary.mem.free) : "" }
                    Figure { label: "Dirty"; value: Monitor.summary.mem ? System.bytes(Monitor.summary.mem.dirty) : "" }
                    Figure { label: "Swap"; value: Monitor.summary.mem ? (Monitor.summary.mem.swapTotal ? System.bytes(Monitor.summary.mem.swapUsed) + " of " + System.bytes(Monitor.summary.mem.swapTotal) : "none") : ""; tint: Monitor.summary.mem && Monitor.summary.mem.swapUsed > 0 ? Theme.warn : Theme.text }
                    Figure { label: "GPU memory"; value: Monitor.gpu && Monitor.gpu.memTotal ? System.bytes(Monitor.gpu.memUsed) + " of " + System.bytes(Monitor.gpu.memTotal) : "—" }
                }
                // Energy
                RowLayout {
                    visible: root.tab === "energy"
                    spacing: Theme.s5
                    Figure { label: "Package power"; value: Monitor.summary.power && Monitor.summary.power.package !== null ? Monitor.summary.power.package + " W" : "not readable"; tint: Theme.accent }
                    Figure { label: "GPU power"; value: Monitor.gpu && Monitor.gpu.power !== null ? Math.round(Monitor.gpu.power) + " W" : "—" }
                    Figure { label: "Battery draw"; value: Monitor.summary.power && Monitor.summary.power.battery !== null ? Monitor.summary.power.battery + " W" : "no battery" }
                    Figure { label: "Power profile"; value: Power.profileLabel }
                    Figure { label: "Preventing sleep"; value: Monitor.summary.inhibitors ? (Monitor.summary.inhibitors.length ? Monitor.summary.inhibitors.map(i => i.who).filter((v, i, a) => a.indexOf(v) === i).join(", ") : "nothing") : ""; tint: Monitor.summary.inhibitors && Monitor.summary.inhibitors.length ? Theme.warn : Theme.text }
                }
                // Disk
                RowLayout {
                    visible: root.tab === "disk"
                    spacing: Theme.s5
                    Figure { label: "Reading"; value: Monitor.summary.disk ? System.rate(Monitor.summary.disk.read) : ""; tint: Theme.accent }
                    Figure { label: "Writing"; value: Monitor.summary.disk ? System.rate(Monitor.summary.disk.write) : ""; tint: Theme.danger }
                    Sparkline { Layout.preferredWidth: 160; implicitHeight: 28; values: Monitor.diskReadHistory.map(v => Math.min(1, v / 5e7)); color: Theme.accent; bars: 30 }
                    Sparkline { Layout.preferredWidth: 160; implicitHeight: 28; values: Monitor.diskWriteHistory.map(v => Math.min(1, v / 5e7)); color: Theme.danger; bars: 30 }
                    Repeater {
                        model: Storage.mounts.slice(0, 3)
                        Figure { required property var modelData; label: Storage.label(modelData); value: System.bytes(modelData.size - modelData.used) + " free" }
                    }
                }
                // Network
                RowLayout {
                    visible: root.tab === "network"
                    spacing: Theme.s5
                    Figure { label: "Receiving"; value: System.rate(System.netDown); tint: Theme.accent }
                    Figure { label: "Sending"; value: System.rate(System.netUp); tint: Theme.danger }
                    Sparkline { Layout.preferredWidth: 220; implicitHeight: 28; values: System.netHistory.map(v => Math.min(1, v / 1e7)); color: Theme.accent; bars: 30 }
                    Figure { label: "Link"; value: Network.wifiConnected ? ((Network.activeNetwork && Network.activeNetwork.ssid) || "Wi-Fi") : Network.wiredConnected ? "wired" : "offline" }
                    Figure { label: "VPN"; value: Vpn.active ? Vpn.active.name : "none" }
                }
                Item { Layout.fillWidth: true }
            }
        }

        // Sensors
        RowLayout {
            visible: root.tab === "sensors"
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.alignment: Qt.AlignTop
            spacing: Theme.s3

            Card {
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                Layout.alignment: Qt.AlignTop
                ColumnLayout {
                    width: parent.width
                    spacing: Theme.s2
                    Label { text: "Temperatures"; weight: Font.DemiBold }
                    GridLayout {
                        columns: 2
                        columnSpacing: Theme.s4
                        rowSpacing: 2
                        Layout.fillWidth: true
                        Repeater {
                            model: Monitor.temps.slice(0, 16)
                            RowLayout {
                                required property var modelData
                                Layout.fillWidth: true
                                Label { text: modelData.chip + " " + modelData.label; size: Theme.sizeSmall; color: Theme.text2; Layout.fillWidth: true; elide: Text.ElideRight }
                                Label { text: Math.round(modelData.value) + "°"; mono: true; tabular: true; size: Theme.sizeSmall; color: modelData.value > 85 ? Theme.danger : modelData.value > 70 ? Theme.warn : Theme.text }
                            }
                        }
                    }
                    Label { visible: Monitor.temps.length === 0; text: "No sensors"; size: Theme.sizeCaption; color: Theme.text3 }

                    Label { text: "Fans"; weight: Font.DemiBold; Layout.topMargin: Theme.s2 }
                    Repeater {
                        model: Monitor.fans
                        RowLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            Label { text: modelData.chip + " " + modelData.label; size: Theme.sizeSmall; color: Theme.text2; Layout.preferredWidth: 160; elide: Text.ElideRight }
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: 4; radius: 2; color: Theme.pressed
                                Rectangle { width: parent.width * Math.min(1, (modelData.pct !== null ? modelData.pct / 100 : modelData.rpm / 3000)); height: parent.height; radius: 2; color: Theme.accent }
                            }
                            Label { text: modelData.rpm + " rpm" + (modelData.pct !== null ? "  ·  " + modelData.pct + "%" : ""); mono: true; tabular: true; size: Theme.sizeSmall; Layout.preferredWidth: 130; horizontalAlignment: Text.AlignRight }
                        }
                    }
                    RowLayout {
                        visible: Monitor.gpu !== null && Monitor.gpu.fan !== null
                        Layout.fillWidth: true
                        Label { text: "GPU fan"; size: Theme.sizeSmall; color: Theme.text2; Layout.preferredWidth: 160 }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 4; radius: 2; color: Theme.pressed
                            Rectangle { width: parent.width * Math.min(1, (Monitor.gpu ? Monitor.gpu.fan : 0) / 100); height: parent.height; radius: 2; color: Theme.accent }
                        }
                        Label { text: (Monitor.gpu ? Math.round(Monitor.gpu.fan) : 0) + "%"; mono: true; tabular: true; size: Theme.sizeSmall; Layout.preferredWidth: 130; horizontalAlignment: Text.AlignRight }
                    }
                    Label { visible: Monitor.fans.length === 0 && !(Monitor.gpu && Monitor.gpu.fan !== null); text: "No fans reported by hwmon. CoolerControl exposes more boards."; size: Theme.sizeCaption; color: Theme.text3 }
                }
            }

            Card {
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                Layout.alignment: Qt.AlignTop
                ColumnLayout {
                    width: parent.width
                    spacing: Theme.s2
                    RowLayout {
                        Label { text: Monitor.gpu ? Monitor.gpu.name : "GPU"; weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                        Label { text: Monitor.gpu ? Monitor.gpu.vendor.toUpperCase() : "not found"; size: Theme.sizeCaption; color: Theme.text3 }
                    }
                    Sparkline { Layout.fillWidth: true; implicitHeight: 40; values: System.gpuHistory; color: Theme.accent; bars: 30; visible: Monitor.gpu !== null }
                    GridLayout {
                        columns: 4
                        columnSpacing: Theme.s4
                        rowSpacing: 4
                        visible: Monitor.gpu !== null
                        Repeater {
                            model: Monitor.gpu ? [
                                ["Load", Monitor.gpu.util !== null ? Math.round(Monitor.gpu.util) + "%" : "—"],
                                ["Memory", Monitor.gpu.memTotal ? System.bytes(Monitor.gpu.memUsed) + " / " + System.bytes(Monitor.gpu.memTotal) : "—"],
                                ["Temperature", Monitor.gpu.temp !== null ? Math.round(Monitor.gpu.temp) + "°" : "—"],
                                ["Power", Monitor.gpu.power !== null ? Math.round(Monitor.gpu.power) + " W" : "—"],
                                ["Core clock", Monitor.gpu.clockCore !== null ? Math.round(Monitor.gpu.clockCore) + " MHz" : "—"],
                                ["Memory clock", Monitor.gpu.clockMem !== null ? Math.round(Monitor.gpu.clockMem) + " MHz" : "—"],
                                ["Fan", Monitor.gpu.fan !== null ? Math.round(Monitor.gpu.fan) + "%" : "—"],
                            ] : []
                            Figure { required property var modelData; label: modelData[0]; value: modelData[1] }
                        }
                    }
                    RowLayout {
                        spacing: Theme.s2
                        Layout.topMargin: Theme.s2
                        Label { text: "Clocks, power limit and fan curves:"; size: Theme.sizeCaption; color: Theme.text3 }
                        Button { text: "LACT"; implicitHeight: 26; enabled: Monitor.lact !== null; onClicked: Monitor.launchLact() }
                        Button { text: "CoolerControl"; implicitHeight: 26; enabled: Monitor.coolercontrol !== null; onClicked: Monitor.launchCoolerControl() }
                        Label { visible: Monitor.lact === null && Monitor.coolercontrol === null; text: "neither daemon is running"; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true; elide: Text.ElideRight }
                    }
                }
            }
        }
    }
}
