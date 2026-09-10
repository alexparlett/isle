import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// Every process in the columns the tab asks for: sortable, searchable, a selection with End, Force quit, pause and priority.
ColumnLayout {
    id: root
    spacing: Theme.s2

    // [{ key, label, width, align }]; width 0 fills.
    property var columns: []
    property string sortKey: "cpu"
    property bool sortDesc: true
    property string filter: ""
    property bool mineOnly: false
    property int selectedPid: -1
    readonly property var selected: Monitor.processes.find(p => p.pid === selectedPid) || null
    // Grouped: a process with children is one row carrying their totals, opened with its chevron. A search keeps the
    // tree, pruned to the matches and their ancestors, and opens it down to them.
    readonly property bool grouped: Prefs.p.monitorGrouped
    property var expanded: ({})
    function toggle(pid) { const e = Object.assign({}, expanded); if (e[pid]) delete e[pid]; else e[pid] = true; expanded = e; }
    // Session managers and the compositor are not groups; what they start is top level.
    function container(p) { return !p || p.pid <= 2 || p.name === "systemd" || p.name === "Hyprland" || p.name === "kthreadd"; }
    readonly property var summed: ["cpu", "cputime", "rss", "memPct", "threads", "ioRead", "ioWrite", "ioReadTotal", "ioWriteTotal", "netDown", "netUp", "energy", "gpuMem"]

    readonly property var rows: {
        const q = filter.toLowerCase();
        const hit = p => !q || p.name.toLowerCase().indexOf(q) >= 0 || p.cmd.toLowerCase().indexOf(q) >= 0 || String(p.pid) === q;
        let list = Monitor.processes.filter(p => !mineOnly || p.user === Monitor.me)
            .map(p => { const n = Monitor.netFor(p.pid); return Object.assign({ netDown: n.down, netUp: n.up, energy: Monitor.energyFor(p), gpuMem: Monitor.gpuMemFor(p.pid), inhibit: Monitor.inhibitorFor(p.pid) ? 1 : 0, depth: 0, kids: 0 }, p); });
        const k = sortKey, d = sortDesc ? -1 : 1;
        const cmp = (a, b) => {
            const x = a[k], y = b[k];
            const r = typeof x === "string" ? x.localeCompare(y) : x - y;
            return r !== 0 ? r * d : a.pid - b.pid;
        };
        if (!grouped) { list = list.filter(hit); list.sort(cmp); return list; }
        const byPid = {}; for (const p of list) byPid[p.pid] = p;
        const under = {};
        for (const p of list) {
            const parent = byPid[p.ppid];
            const key = container(parent) ? 0 : p.ppid;
            (under[key] = under[key] || []).push(p);
        }
        // With a search, a process stays when it or something below it matches.
        const keep = {};
        const stays = p => { if (keep[p.pid] !== undefined) return keep[p.pid]; return keep[p.pid] = hit(p) || (under[p.pid] || []).some(stays); };
        const children = {};
        for (const key in under) children[key] = q ? under[key].filter(stays) : under[key];
        // Totals climb from the leaves; kids counts every descendant.
        const total = (p) => {
            const kids = children[p.pid] || [];
            for (const c of kids) { total(c); for (const f of summed) p[f] = (p[f] || 0) + (c[f] || 0); p.kids += c.kids + 1; }
            return p;
        };
        const out = [];
        const walk = (nodes, depth) => {
            nodes.sort(cmp);
            for (const p of nodes) { p.depth = depth; out.push(p); if ((expanded[p.pid] || q) && children[p.pid]) walk(children[p.pid], depth + 1); }
        };
        walk((children[0] || []).map(total), 0);
        return out;
    }

    function cell(p, key) {
        switch (key) {
        case "cpu": return p.cpu.toFixed(1) + "%";
        case "cputime": return p.cputime >= 3600 ? Math.floor(p.cputime / 3600) + "h " + Math.floor(p.cputime % 3600 / 60) + "m" : Math.floor(p.cputime / 60) + "m " + Math.round(p.cputime % 60) + "s";
        case "rss": return System.bytes(p.rss);
        case "memPct": return p.memPct.toFixed(1) + "%";
        case "gpuMem": return p.gpuMem ? System.bytes(p.gpuMem) : "";
        case "energy": return p.energy >= 0.05 ? p.energy.toFixed(1) : "";
        case "inhibit": return p.inhibit ? "Yes" : "";
        case "name": return p.name + (p.kids ? "  (" + (p.kids + 1) + ")" : "");
        case "ioRead": return p.ioRead > 1024 ? System.rate(p.ioRead) : "";
        case "ioWrite": return p.ioWrite > 1024 ? System.rate(p.ioWrite) : "";
        case "ioReadTotal": return p.ioReadTotal > 1e6 ? System.bytes(p.ioReadTotal) : "";
        case "ioWriteTotal": return p.ioWriteTotal > 1e6 ? System.bytes(p.ioWriteTotal) : "";
        case "netDown": return p.netDown > 512 ? System.rate(p.netDown) : "";
        case "netUp": return p.netUp > 512 ? System.rate(p.netUp) : "";
        default: return String(p[key]);
        }
    }
    // A new sample replaces the model, which puts the list back at the top. The model is assigned by hand so the
    // position the user scrolled to is known before the swap and put back after it.
    property real keepY: 0
    property bool restoring: false
    // Put back in the same event, before a frame can show the top.
    onRowsChanged: {
        restoring = true;
        list.model = rows;
        list.contentY = keepY;
        restoring = false;
    }
    Component.onCompleted: list.model = rows
    readonly property var textKeys: ["name", "user", "state", "inhibit"]
    function sortBy(key) { if (sortKey === key) sortDesc = !sortDesc; else { sortKey = key; sortDesc = textKeys.indexOf(key) < 0; } }
    // A different tab, sort, search or scope is a new list, read from the top.
    function rewind() { keepY = 0; list.contentY = 0; }
    onColumnsChanged: rewind()
    onSortKeyChanged: rewind()
    onSortDescChanged: rewind()
    onFilterChanged: rewind()
    onMineOnlyChanged: rewind()
    onGroupedChanged: rewind()

    // Toolbar
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.s2
        Field { implicitWidth: 240; implicitHeight: 32; glyph: "search"; placeholder: "Search processes"; onTextChanged: root.filter = text }
        Button { text: root.mineOnly ? "My processes" : "All processes"; implicitHeight: 32; onClicked: root.mineOnly = !root.mineOnly }
        Button { text: Prefs.p.monitorGrouped ? "Grouped" : "Flat"; glyph: Prefs.p.monitorGrouped ? "list-tree" : "list"; implicitHeight: 32; onClicked: Prefs.p.monitorGrouped = !Prefs.p.monitorGrouped }
        Item { Layout.fillWidth: true }
        Label { visible: !Monitor.netAvailable && root.columns.some(c => c.key === "netDown"); text: "Per-process network needs nethogs with capabilities (tools/install.sh)"; size: Theme.sizeCaption; color: Theme.warn }
        Label { text: Monitor.summary.count + " processes  ·  " + Monitor.summary.threads + " threads"; size: Theme.sizeCaption; color: Theme.text3 }
    }

    // Header
    Item {
        Layout.fillWidth: true
        implicitHeight: 28
        Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.hairline }
        RowLayout {
            anchors { fill: parent; leftMargin: Theme.s2; rightMargin: Theme.s2 }
            spacing: Theme.s3
            Repeater {
                model: root.columns
                Item {
                    required property var modelData
                    Layout.preferredWidth: modelData.width || 160
                    Layout.minimumWidth: modelData.width || 120
                    Layout.fillWidth: modelData.width === 0
                    implicitHeight: 28
                    RowLayout {
                        anchors.fill: parent
                        spacing: 4
                        layoutDirection: modelData.align === Text.AlignRight ? Qt.RightToLeft : Qt.LeftToRight
                        Label { text: modelData.label; size: Theme.sizeCaption; weight: Font.DemiBold; color: root.sortKey === modelData.key ? Theme.text : Theme.text3 }
                        Glyph { visible: root.sortKey === modelData.key; name: root.sortDesc ? "chevron-down" : "chevron-up"; size: 10; color: Theme.text2 }
                        Item { Layout.fillWidth: true }
                    }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: root.sortBy(modelData.key) }
                }
            }
        }
    }

    // Rows
    ListView {
        id: list
        Layout.fillWidth: true
        Layout.fillHeight: true
        clip: true
        reuseItems: true
        onContentYChanged: if (!root.restoring) root.keepY = contentY
        Scrollbar { target: list }
        delegate: Rectangle {
            required property var modelData
            required property int index
            readonly property bool sel: modelData.pid === root.selectedPid
            width: list.width
            height: 26
            radius: Theme.radiusChip
            color: sel ? Theme.pressed : hover.containsMouse ? Theme.raised : index % 2 ? Qt.alpha(Theme.raised, 0.35) : "transparent"
            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s2; rightMargin: Theme.s2 }
                spacing: Theme.s3
                Repeater {
                    model: root.columns
                    Label {
                        required property var modelData
                        readonly property var p: parent.parent.modelData
                        Layout.preferredWidth: modelData.width || 160
                        Layout.minimumWidth: modelData.width || 120
                        Layout.fillWidth: modelData.width === 0
                        // The name column carries the tree: an indent per level and a chevron on a group.
                        Layout.leftMargin: modelData.key === "name" ? p.depth * 16 + 16 : 0
                        text: root.cell(p, modelData.key)
                        size: Theme.sizeSmall
                        mono: root.textKeys.indexOf(modelData.key) < 0
                        tabular: true
                        horizontalAlignment: modelData.align
                        color: modelData.key === "name" ? (p.kernel ? Theme.text3 : Theme.text)
                             : modelData.key === "cpu" && p.cpu >= 50 ? Theme.warn
                             : modelData.key === "state" && p.state === "Stopped" ? Theme.warn
                             : modelData.key === "inhibit" && p.inhibit ? Theme.warn : Theme.text2
                        elide: Text.ElideRight
                    }
                }
            }
            MouseArea {
                id: hover
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.LeftButton | Qt.RightButton
                onClicked: root.selectedPid = modelData.pid
                onDoubleClicked: if (modelData.kids) root.toggle(modelData.pid)
            }
            Glyph {
                visible: modelData.kids > 0
                x: Theme.s2 + modelData.depth * 16 + 1
                anchors.verticalCenter: parent.verticalCenter
                name: root.expanded[modelData.pid] || root.filter ? "chevron-down" : "chevron-right"
                size: 10
                color: Theme.text2
                MouseArea { anchors { fill: parent; margins: -6 } cursorShape: Qt.PointingHandCursor; onClicked: root.toggle(modelData.pid) }
            }
        }
    }

    // Selection and details
    Card {
        Layout.fillWidth: true
        visible: root.selected !== null
        padding: Theme.s3
        ColumnLayout {
            width: parent.width
            spacing: Theme.s2
            RowLayout {
                spacing: Theme.s2
                Label { text: root.selected ? root.selected.name : ""; weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                Button { text: root.selected && root.selected.state === "Stopped" ? "Resume" : "Pause"; variant: "text"; onClicked: root.selected.state === "Stopped" ? Monitor.resume(root.selected) : Monitor.stop(root.selected) }
                Button { text: "Lower priority"; variant: "text"; enabled: root.selected && root.selected.nice < 19; onClicked: Monitor.renice(root.selected, Math.min(19, root.selected.nice + 5)) }
                Button { text: "Normal priority"; variant: "text"; visible: root.selected && root.selected.nice !== 0; onClicked: Monitor.renice(root.selected, 0) }
                Button { text: "End"; onClicked: Monitor.terminate(root.selected) }
                Button { text: "Force quit"; variant: "danger"; onClicked: Monitor.forceQuit(root.selected) }
            }
            GridLayout {
                columns: 5
                columnSpacing: Theme.s4
                rowSpacing: 2
                Repeater {
                    model: root.selected ? [
                        ["PID", root.selected.pid + "  ·  parent " + root.selected.ppid],
                        ["User", root.selected.user],
                        ["Started", Qt.formatDateTime(new Date(root.selected.started * 1000), "ddd HH:mm")],
                        ["CPU time", root.cell(root.selected, "cputime")],
                        ["Priority", "nice " + root.selected.nice],
                        ["Memory", System.bytes(root.selected.rss) + "  ·  " + root.selected.memPct + "%"],
                        ["Threads", String(root.selected.threads)],
                        ["GPU memory", Monitor.gpuMemFor(root.selected.pid) ? System.bytes(Monitor.gpuMemFor(root.selected.pid)) : "—"],
                        ["Network", System.rate(Monitor.netFor(root.selected.pid).down) + " down  ·  " + System.rate(Monitor.netFor(root.selected.pid).up) + " up"],
                        ["Disk", System.rate(root.selected.ioRead) + " read  ·  " + System.rate(root.selected.ioWrite) + " write"],
                    ] : []
                    ColumnLayout {
                        required property var modelData
                        spacing: 0
                        Label { text: modelData[0]; size: Theme.sizeCaption; color: Theme.text3 }
                        Label { text: modelData[1]; mono: true; tabular: true; size: Theme.sizeSmall }
                    }
                }
            }
            Label { visible: !!(root.selected && Monitor.inhibitorFor(root.selected.pid)); text: root.selected && Monitor.inhibitorFor(root.selected.pid) ? "Holding off " + Monitor.inhibitorFor(root.selected.pid).what + ": " + Monitor.inhibitorFor(root.selected.pid).why : ""; size: Theme.sizeCaption; color: Theme.warn }
            Label { text: root.selected ? root.selected.cmd : ""; mono: true; size: Theme.sizeCaption; color: Theme.text2; Layout.fillWidth: true; elide: Text.ElideMiddle }
        }
    }
}
