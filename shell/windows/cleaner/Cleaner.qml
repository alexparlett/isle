import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Isle.Files
import qs.theme
import qs.ui
import qs.services
// Loaded by URL rather than declared (see shell.qml), which leaves no implicit scope for a sibling.
import qs.windows.cleaner

// The Cleaner window: what fills a disk or folder, and a basket of what to send to the trash.
FloatingWindow {
    id: root
    title: "Cleaner"
    visible: Surfaces.cleaner
    implicitWidth: 1280
    implicitHeight: 820
    minimumSize: Qt.size(980, 620)
    color: Theme.light ? "#FFFFFF" : "#131417"
    onVisibleChanged: {
        if (!visible) Surfaces.cleaner = false;
        Storage.listeners += visible ? 1 : -1;
        if (!visible) return;
        take();
        if (!DiskUsage.ready && !DiskUsage.scanning) start("all");
        readSystem();
    }

    property string page: "overview"
    property string spaceView: "tiles"
    property string cleanupTab: "safe"
    readonly property var pages: [
        { id: "overview", label: "Overview", glyph: "house" },
        { id: "cleanup", label: "Cleanup", glyph: "wand-sparkles" },
        { id: "space", label: "Space", glyph: "layout-grid" },
        { id: "duplicates", label: "Duplicates", glyph: "copy" },
        { id: "large", label: "Large files", glyph: "file-search" },
        { id: "old", label: "Untouched", glyph: "clock" },
    ]

    // The folder Space is showing; empty or outside the scan means its root.
    property string path: ""
    readonly property string at: path && inside(path) ? path : DiskUsage.root

    readonly property var targets: [["all", "All disks"], [Engine.home, "Home"]].concat(Storage.mounts.map(m => [m.target, labelFor(m.target)]))
    readonly property bool many: DiskUsage.roots.length > 1
    // The volume a path is on: the mount with the longest path it is under.
    function mountOf(p) { return Storage.mounts.filter(m => within(p, m.target)).sort((a, b) => b.target.length - a.target.length)[0] || null; }
    // The disks the scan is on, one each.
    readonly property var disks: {
        const out = [];
        for (const r of DiskUsage.roots) {
            const m = mountOf(r);
            if (m && !out.some(o => o.target === m.target)) out.push(m);
        }
        return out;
    }
    readonly property real disksFree: disks.reduce((s, m) => s + m.size - m.used, 0)
    readonly property string placeName: many ? "All disks" : labelFor(DiskUsage.root)
    function labelFor(p) {
        if (p === Engine.home) return "Home";
        if (p === "/") return "System";
        const m = Storage.mounts.find(m => m.target === p);
        return m ? Storage.label(m) : Engine.displayName(p);
    }

    readonly property string trashDir: Engine.parentOf(FileJobs.trashPath)
    // The trash's own bookkeeping takes a few blocks even when it is empty.
    readonly property real trashHeld: { DiskUsage.generation; const n = DiskUsage.ready ? DiskUsage.node(trashDir) : ({}); return n.size > 65536 ? n.size : 0; }

    // The cleanup rules: the shell's own, then the person's from ~/.config/isle/cleaner.json, whose
    // groups add to one of the same id or stand on their own.
    property var shippedGroups: []
    property var ownGroups: []
    readonly property var ruleGroups: {
        const out = shippedGroups.map(g => Object.assign({}, g, { rules: g.rules.slice() }));
        for (const g of ownGroups) {
            const same = out.find(o => o.id === g.id);
            if (same) same.rules = same.rules.concat(g.rules || []);
            else out.push(Object.assign({ glyph: "folder", kind: 0, note: "", safe: false }, g, { rules: g.rules || [] }));
        }
        return out;
    }
    readonly property var flatRules: {
        const out = [];
        ruleGroups.forEach((g, gi) => g.rules.forEach(r => out.push(Object.assign({ group: gi, safe: !!g.safe }, r))));
        return out;
    }
    FileView {
        path: Quickshell.shellDir + "/windows/cleaner/rules.json"
        onLoaded: { try { root.shippedGroups = JSON.parse(text()).groups || []; } catch (e) { root.shippedGroups = []; } }
    }
    FileView {
        path: Quickshell.env("HOME") + "/.config/isle/cleaner.json"
        printErrors: false
        watchChanges: true
        onFileChanged: reload()
        onLoaded: { try { root.ownGroups = JSON.parse(text()).groups || []; } catch (e) { root.ownGroups = []; } }
        onLoadFailed: root.ownGroups = []
    }

    // What the rules match in the scan, as { safe: [group], worth: [group], safeSize, worthSize }, each
    // group with its items largest first. Nothing already in the trash is offered again.
    readonly property var cleanup: {
        DiskUsage.generation;
        const empty = { safe: [], worth: [], safeSize: 0, worthSize: 0 };
        if (!DiskUsage.ready || DiskUsage.scanning || !flatRules.length) return empty;
        const groups = ruleGroups.map(g => ({ id: g.id, name: g.name, glyph: g.glyph, kind: g.kind, note: g.note, safe: !!g.safe, items: [], size: 0 }));
        for (const m of DiskUsage.evaluate(flatRules)) {
            if (within(m.path, trashDir)) continue;
            const rule = flatRules[m.rule], g = groups[rule.group];
            g.items.push(Object.assign({}, m, { name: label(rule, m) }));
            g.size += m.size;
        }
        const out = empty;
        for (const g of groups) {
            if (!g.items.length) continue;
            g.items.sort((a, b) => b.size - a.size);
            (g.safe ? out.safe : out.worth).push(g);
            if (g.safe) out.safeSize += g.size; else out.worthSize += g.size;
        }
        out.safe.sort((a, b) => b.size - a.size);
        out.worth.sort((a, b) => b.size - a.size);
        return out;
    }
    // A match's name from its rule: {name}, {project} (the folder it sits in), {1} and on for what each *
    // matched, then the rule's own replacements; a name in `names` is said the way the rule says it.
    function label(rule, m) {
        if (!rule.label) return rule.name;
        let t = rule.label.replace(/\{name\}/g, rule.names && rule.names[m.name] ? rule.names[m.name] : m.name)
                          .replace(/\{project\}/g, projectOf(m.path))
                          .replace(/\{(\d+)\}/g, (_, n) => m.captures[Number(n) - 1] || "");
        for (const r of rule.replace || []) t = t.replace(new RegExp(r[0], "g"), r[1]);
        return t;
    }
    // The folder a match belongs to, looking past a worktrees folder to the repository that has it.
    function projectOf(path) {
        let p = Engine.parentOf(path);
        if (Engine.displayName(p) === "worktrees") p = Engine.parentOf(p);
        if (Engine.displayName(p) === ".claude") p = Engine.parentOf(p);
        return Engine.displayName(p);
    }
    // Safe items start ticked, once for each scan, the first time Cleanup is looked at.
    property real preselectedFor: -1
    function preselect() {
        if (page !== "cleanup" || !DiskUsage.ready || !scannedAt || preselectedFor === scannedAt || !cleanup.safe.length) return;
        preselectedFor = scannedAt;
        addAll(cleanup.safe.reduce((all, g) => all.concat(g.items), []));
    }
    onPageChanged: preselect()
    onCleanupChanged: preselect()

    readonly property var dupes: { DiskUsage.generation; dupesRev; return DiskUsage.duplicatesReady ? DiskUsage.duplicates() : ({ groups: [], canFree: 0, files: 0, sharing: 0 }); }
    Connections {
        target: DiskUsage
        function onDuplicatesChanged() { root.dupesRev++; }
    }
    property int dupesRev: 0

    // What the system keeps that only root can clear: [{ id, name, note, size }].
    property var system: []
    property string systemRunning: ""
    readonly property string systemScript: Quickshell.shellDir + "/scripts/syscleanup.py"
    Process {
        id: systemStatus
        command: ["python3", root.systemScript, "status"]
        stdout: StdioCollector { onStreamFinished: { try { root.system = JSON.parse(text).items || []; } catch (e) { root.system = []; } } }
    }
    Process {
        id: systemRun
        onExited: { root.systemRunning = ""; root.readSystem(); }
    }
    function readSystem() { if (!systemStatus.running) systemStatus.running = true; }
    function clearSystem(id) {
        systemRunning = id;
        systemRun.command = ["pkexec", "/usr/bin/python3", systemScript, "run", id];
        systemRun.running = true;
    }

    // What is to go to the trash, by path. A folder in it stands for everything under it.
    property var basket: ({})
    property int basketRev: 0
    readonly property int basketCount: { basketRev; return Object.keys(basket).length; }
    readonly property real basketSize: { basketRev; return Object.values(basket).reduce((s, i) => s + i.size, 0); }
    // Sizes of what was sent to the trash, by path, until the job says it went.
    property var sent: ({})
    // What the last Move to Trash took, for the bar that offers to undo it.
    property var lastTrashed: null
    property bool askEmpty: false

    property real scannedAt: 0
    property real now: Date.now()
    Timer { interval: 30000; running: root.visible; repeat: true; onTriggered: root.now = Date.now() }
    Connections {
        target: DiskUsage
        function onStateChanged() { if (DiskUsage.ready && !DiskUsage.scanning && !root.scannedAt) { root.scannedAt = Date.now(); root.now = Date.now(); } }
    }

    function within(p, dir) { return dir === "/" ? p.startsWith("/") : p === dir || p.startsWith(dir + "/"); }
    function inside(p) { return DiskUsage.roots.some(r => within(p, r)); }
    // A place to scan, or "all" for every disk mounted, which waits for df to have said what they are.
    property bool allWanted: false
    function start(p) {
        allWanted = p === "all" && !Storage.mounts.length;
        if (allWanted) return;
        path = "";
        scannedAt = 0;
        clear();
        lastTrashed = null;
        DiskUsage.scan(p === "all" ? Storage.mounts.map(m => m.target) : [p]);
    }
    Connections {
        target: Storage
        function onMountsChanged() { if (root.allWanted) root.start("all"); }
    }
    function take() {
        if (!Surfaces.cleanerPath) return;
        start(Surfaces.cleanerPath);
        Surfaces.cleanerPath = "";
    }
    function goTo(p) { path = p; page = "space"; }
    function up() {
        if (at === DiskUsage.root) return;
        path = DiskUsage.roots.indexOf(at) >= 0 ? "" : Engine.parentOf(at);
    }
    function open(item) { goTo(item.dir && !item.skipped ? item.path : Engine.parentOf(item.path)); }
    function reveal(item) { Surfaces.showFiles(item.dir ? item.path : Engine.parentOf(item.path)); }
    function isProtected(p) { return DiskUsage.isProtected(p); }

    function inBasket(p) {
        if (basket[p] !== undefined) return true;
        for (const k in basket) if (within(p, k)) return true;
        return false;
    }
    function toggle(item) {
        const b = Object.assign({}, basket);
        if (b[item.path] !== undefined) delete b[item.path];
        else if (!isProtected(item.path)) {
            for (const k in b) if (within(k, item.path)) delete b[k];
            b[item.path] = { path: item.path, size: item.size };
        }
        basket = b;
        basketRev++;
    }
    function addAll(rows) {
        for (const r of rows) if (r.path && !r.other && !r.skipped && !inBasket(r.path)) toggle(r);
    }
    function removeAll(rows) {
        const b = Object.assign({}, basket);
        for (const r of rows) delete b[r.path];
        basket = b;
        basketRev++;
    }
    function clear() { basket = {}; basketRev++; }
    function trashBasket() {
        const paths = Object.keys(basket);
        if (!paths.length) return;
        const s = Object.assign({}, sent);
        for (const p of paths) s[p] = basket[p].size;
        sent = s;
        lastTrashed = { count: paths.length, size: basketSize };
        askEmpty = false;
        clear();
        FileJobs.trash(paths);
    }

    function count(n) { return Number(n).toLocaleString(Qt.locale(), "f", 0); }
    function ago(seconds) {
        if (!seconds) return "";
        const days = Math.floor((Date.now() / 1000 - seconds) / 86400);
        return days < 1 ? "today" : days < 2 ? "yesterday" : days < 31 ? days + " days ago"
             : days < 365 ? Math.floor(days / 30) + (days < 60 ? " month ago" : " months ago")
             : Math.floor(days / 365) + (days < 730 ? " year ago" : " years ago");
    }
    function sinceScan() {
        const m = Math.floor((now - scannedAt) / 60000);
        return m < 1 ? "just now" : m === 1 ? "a minute ago" : m < 60 ? m + " minutes ago" : Math.floor(m / 60) + (m < 120 ? " hour ago" : " hours ago");
    }
    // A path as a person reads it: home is a tilde.
    function place(p) { return p === Engine.home ? "~" : p.startsWith(Engine.home + "/") ? "~" + p.slice(Engine.home.length) : p; }
    function crumbs() {
        const out = [{ name: placeName, path: DiskUsage.root }];
        if (!at) return out;
        // With several roots, the one this is under comes next, by its own name.
        const base = DiskUsage.roots.filter(r => within(at, r)).sort((a, b) => b.length - a.length)[0];
        if (many) out.push({ name: labelFor(base), path: base });
        let acc = base;
        for (const part of at.slice(base === "/" ? 1 : base.length).split("/").filter(x => x)) {
            acc = Engine.join(acc, part);
            out.push({ name: part, path: acc });
        }
        return out;
    }

    Connections {
        target: Surfaces
        function onCleanerPathChanged() { if (root.visible) root.take(); }
    }
    Connections {
        target: FileJobs
        function onJobFinished(job) {
            if (!DiskUsage.ready || job.state !== FileJob.Done) return;
            if (job.kind === FileJob.Trash) {
                const s = Object.assign({}, root.sent);
                for (const p of Object.keys(s))
                    if (DiskUsage.forget(p, s[p])) delete s[p];
                root.sent = s;
                DiskUsage.refresh([root.trashDir]);
            } else {
                root.lastTrashed = null;
                DiskUsage.refresh(job.made.concat(job.returnsTo).map(p => Engine.parentOf(p)).concat([root.trashDir]));
            }
        }
    }
    Shortcut { sequences: ["Backspace", "Alt+Up"]; enabled: root.page === "space"; onActivated: root.up() }
    Shortcut { sequences: [StandardKey.Undo]; onActivated: FileJobs.undo() }
    Shortcut { sequences: ["Escape"]; enabled: root.basketCount > 0 || root.askEmpty; onActivated: { root.askEmpty = false; root.clear(); } }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Sidebar
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 228
            color: "transparent"
            Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.hairline }

            ColumnLayout {
                anchors { fill: parent; margins: Theme.s3; topMargin: Theme.s4 }
                spacing: 2

                Label { text: "Cleaner"; size: Theme.sizeHeading; weight: Font.DemiBold; Layout.leftMargin: Theme.s2 + 2; Layout.bottomMargin: Theme.s3 }
                Repeater {
                    model: root.pages
                    Rectangle {
                        id: nav
                        required property var modelData
                        readonly property bool sel: root.page === modelData.id
                        Layout.fillWidth: true
                        implicitHeight: 34
                        radius: Theme.radiusControl
                        opacity: DiskUsage.ready ? 1 : 0.45
                        color: sel ? Theme.raised : navArea.containsMouse ? Qt.alpha(Theme.raised, 0.5) : "transparent"
                        border.width: sel ? 1 : 0
                        border.color: Theme.hairline
                        RowLayout {
                            anchors { fill: parent; leftMargin: Theme.s2 + 2; rightMargin: Theme.s2 }
                            spacing: Theme.s2 + 2
                            Glyph { name: nav.modelData.glyph; size: 15; color: nav.sel ? Theme.accent : Theme.text2 }
                            Label { text: nav.modelData.label; size: Theme.sizeSmall; weight: Font.DemiBold; color: nav.sel ? Theme.text : Theme.text2; Layout.fillWidth: true }
                            Label {
                                readonly property real badge: nav.modelData.id === "cleanup" ? root.cleanup.safeSize
                                    : nav.modelData.id === "duplicates" && DiskUsage.duplicatesReady ? root.dupes.canFree : 0
                                visible: badge > 0
                                text: Engine.formatSize(badge)
                                size: Theme.sizeCaption
                                weight: Font.DemiBold
                                mono: true
                                color: Theme.accent
                            }
                        }
                        MouseArea { id: navArea; anchors.fill: parent; hoverEnabled: true; enabled: DiskUsage.ready; cursorShape: Qt.PointingHandCursor; onClicked: root.page = nav.modelData.id }
                    }
                }

                Item { Layout.fillHeight: true }

                // The disks the scan is on, and what the basket and the trash would give back.
                Rectangle {
                    visible: root.disks.length > 0
                    Layout.fillWidth: true
                    implicitHeight: disk.implicitHeight + Theme.s3 * 2
                    radius: Theme.radiusCard
                    color: Theme.raised
                    border.width: 1
                    border.color: Theme.hairline
                    ColumnLayout {
                        id: disk
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s3 }
                        spacing: Theme.s2
                        Repeater {
                            model: root.disks
                            ColumnLayout {
                                id: one
                                required property var modelData
                                Layout.fillWidth: true
                                spacing: Theme.s1 + 2
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.s2
                                    Glyph { name: "hard-drive"; size: 14; color: Theme.text2 }
                                    Label { text: root.labelFor(one.modelData.target); size: Theme.sizeSmall; weight: Font.DemiBold; Layout.fillWidth: true }
                                    Label { text: Math.round(one.modelData.pct * 100) + "%"; size: Theme.sizeSmall; mono: true; tabular: true; color: Theme.text2 }
                                }
                                Rectangle {
                                    id: usage
                                    Layout.fillWidth: true
                                    implicitHeight: 8
                                    radius: 4
                                    color: Theme.hairline
                                    readonly property real used: one.modelData.pct
                                    // What the basket and the trash hold on this disk.
                                    readonly property real held: {
                                        root.basketRev;
                                        const on = p => { const m = root.mountOf(p); return !!m && m.target === one.modelData.target; };
                                        return Object.values(root.basket).filter(i => on(i.path)).reduce((s, i) => s + i.size, 0) + (on(root.trashDir) ? root.trashHeld : 0);
                                    }
                                    readonly property real freed: one.modelData.size ? Math.min(used, held / one.modelData.size) : 0
                                    Rectangle { height: parent.height; radius: 4; width: parent.width * usage.used; color: usage.used > 0.9 ? Theme.warn : Theme.text2 }
                                    Rectangle {
                                        visible: usage.freed > 0
                                        height: parent.height
                                        radius: 4
                                        x: parent.width * (usage.used - usage.freed)
                                        width: Math.max(3, parent.width * usage.freed)
                                        color: Theme.accent
                                    }
                                }
                                Label {
                                    text: Engine.formatSize(one.modelData.size - one.modelData.used) + " free of " + Engine.formatSize(one.modelData.size)
                                    size: Theme.sizeCaption
                                    color: Theme.text3
                                }
                            }
                        }
                        RowLayout {
                            visible: root.basketSize + root.trashHeld > 0
                            Layout.fillWidth: true
                            spacing: Theme.s2
                            Rectangle { Layout.alignment: Qt.AlignTop; Layout.topMargin: 4; implicitWidth: 7; implicitHeight: 7; radius: 4; color: Theme.accent }
                            Label {
                                Layout.fillWidth: true
                                wrapMode: Text.WordWrap
                                text: [root.basketSize > 0 ? Engine.formatSize(root.basketSize) + " selected" : "",
                                       root.trashHeld > 0 ? Engine.formatSize(root.trashHeld) + " in the trash" : ""].filter(t => t).join(", ")
                                size: Theme.sizeCaption
                                color: Theme.text2
                            }
                        }
                        Button {
                            visible: root.trashHeld > 0
                            Layout.fillWidth: true
                            implicitHeight: 28
                            text: "Empty trash"
                            glyph: "trash"
                            onClicked: root.askEmpty = true
                        }
                    }
                }
            }
        }

        // Content
        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: Theme.s5
            spacing: Theme.s4

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.s3
                ColumnLayout {
                    spacing: 2
                    Label { text: DiskUsage.ready ? root.pages.find(p => p.id === root.page).label : "Cleaner"; size: Theme.sizeTitle; weight: Font.Bold }
                    Label {
                        text: !DiskUsage.ready ? "" : root.placeName + "  ·  " + root.count(DiskUsage.files) + " files  ·  scanned " + root.sinceScan()
                            + (DiskUsage.unreadable ? "  ·  " + root.count(DiskUsage.unreadable) + " folders could not be read" : "")
                        size: Theme.sizeSmall
                        color: Theme.text3
                    }
                }
                Item { Layout.fillWidth: true }
                Dropdown {
                    listWidth: 220
                    options: root.targets
                    value: root.many ? "all" : DiskUsage.root || Engine.home
                    onPicked: v => root.start(v)
                }
                Button {
                    text: DiskUsage.scanning ? "Stop" : "Scan again"
                    glyph: DiskUsage.scanning ? "x" : "refresh-cw"
                    variant: DiskUsage.scanning ? "raised" : "accent"
                    onClicked: DiskUsage.scanning ? DiskUsage.cancel() : DiskUsage.scan(DiskUsage.roots.length ? DiskUsage.roots : [Engine.home])
                }
            }

            // Scanning, or nothing scanned yet.
            ColumnLayout {
                visible: !DiskUsage.ready
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: Theme.s2
                Item { Layout.fillHeight: true }
                Label { Layout.alignment: Qt.AlignHCenter; text: root.count(DiskUsage.files); font.pixelSize: Math.round(Theme.sizeDisplay * 1.6); weight: Font.Bold; mono: true; tabular: true }
                Label { Layout.alignment: Qt.AlignHCenter; text: DiskUsage.scanning ? "files looked at in " + root.placeName + ", " + Engine.formatSize(DiskUsage.bytes) + " so far" : "Nothing scanned"; color: Theme.text2 }
                Rectangle {
                    visible: DiskUsage.scanning
                    Layout.alignment: Qt.AlignHCenter
                    Layout.topMargin: Theme.s3
                    implicitWidth: 280
                    implicitHeight: 4
                    radius: 2
                    color: Theme.hairline
                    clip: true
                    Rectangle {
                        width: 90
                        height: parent.height
                        radius: 2
                        color: Theme.accent
                        NumberAnimation on x { from: -90; to: 280; duration: 1100; loops: Animation.Infinite; running: DiskUsage.scanning }
                    }
                }
                Item { Layout.fillHeight: true }
            }

            Loader {
                visible: DiskUsage.ready
                Layout.fillWidth: true
                Layout.fillHeight: true
                sourceComponent: !DiskUsage.ready ? null
                    : root.page === "cleanup" ? cleanupPage : root.page === "duplicates" ? duplicatesPage : root.page === "space" ? spacePage : root.page === "large" ? largePage : root.page === "old" ? oldPage : overviewPage
            }

            // The basket, what was just sent, or the question before emptying the trash.
            Rectangle {
                id: bar
                readonly property string mode: root.askEmpty ? "empty" : root.basketCount > 0 ? "basket" : root.lastTrashed ? "sent" : ""
                visible: mode !== ""
                Layout.fillWidth: true
                implicitHeight: 64
                radius: Theme.radiusCard
                color: Theme.raised
                border.width: 1
                border.color: Theme.hairlineStrong
                RowLayout {
                    anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                    spacing: Theme.s3
                    Rectangle {
                        implicitWidth: 36
                        implicitHeight: 36
                        radius: 18
                        color: Qt.alpha(bar.mode === "sent" ? Theme.ok : Theme.danger, 0.16)
                        Glyph { anchors.centerIn: parent; name: bar.mode === "sent" ? "check" : "trash"; size: 16; color: bar.mode === "sent" ? Theme.ok : Theme.danger }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0
                        Label {
                            text: bar.mode === "empty" ? "Empty the trash?"
                                : bar.mode === "basket" ? Engine.formatSize(root.basketSize) + " selected"
                                : bar.mode === "sent" ? "Moved " + Engine.formatSize(root.lastTrashed.size) + " to the trash" : ""
                            weight: Font.DemiBold
                        }
                        Label {
                            Layout.fillWidth: true
                            text: bar.mode === "empty" ? Engine.formatSize(root.trashHeld) + " goes for good. This cannot be undone."
                                : bar.mode === "basket" ? root.count(root.basketCount) + (root.basketCount === 1 ? " item" : " items") + " to the trash, where each can be put back"
                                : bar.mode === "sent" ? "The space comes back when the trash is emptied." : ""
                            size: Theme.sizeCaption
                            color: Theme.text3
                        }
                    }
                    Button {
                        visible: bar.mode === "sent" && FileJobs.canUndo
                        text: "Undo"
                        glyph: "corner-up-left"
                        onClicked: { FileJobs.undo(); root.lastTrashed = null; }
                    }
                    Button {
                        visible: bar.mode !== "sent"
                        text: bar.mode === "empty" ? "Cancel" : "Clear"
                        onClicked: bar.mode === "empty" ? root.askEmpty = false : root.clear()
                    }
                    Button {
                        text: bar.mode === "basket" ? "Move to Trash" : "Empty trash"
                        variant: "danger"
                        visible: bar.mode !== "sent" || root.trashHeld > 0
                        onClicked: {
                            if (bar.mode === "basket") root.trashBasket();
                            else if (bar.mode === "sent") root.askEmpty = true;
                            else { root.askEmpty = false; root.lastTrashed = null; FileJobs.emptyTrash(); }
                        }
                    }
                    Rectangle {
                        visible: bar.mode === "sent"
                        implicitWidth: 28
                        implicitHeight: 28
                        radius: 14
                        color: closeArea.containsMouse ? Theme.pressed : "transparent"
                        Glyph { anchors.centerIn: parent; name: "x"; size: 14; color: Theme.text2 }
                        MouseArea { id: closeArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.lastTrashed = null }
                    }
                }
            }
        }
    }

    Component { id: overviewPage; Overview { app: root } }
    Component { id: spacePage; Space { app: root } }
    Component { id: cleanupPage; Cleanup { app: root } }
    Component { id: duplicatesPage; Duplicates { app: root } }
    Component {
        id: largePage
        FileList {
            app: root
            rows: { DiskUsage.generation; return DiskUsage.largest(DiskUsage.root, 200); }
            summary: "The largest files in " + root.placeName + ", wherever they are."
            empty: "No files"
        }
    }
    Component {
        id: oldPage
        FileList {
            readonly property var found: { DiskUsage.generation; return DiskUsage.untouched(DiskUsage.root, 365, 200); }
            app: root
            showAge: true
            rows: found.items || []
            summary: Engine.formatSize(found.size || 0) + " in folders where nothing has changed for a year. Old is not the same as unwanted, so look before you clear."
            empty: "Everything here has changed in the last year."
        }
    }
}
