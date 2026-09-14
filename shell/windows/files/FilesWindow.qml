import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Isle.Files
import Quickshell.Widgets
import qs.theme
import qs.ui
import qs.services
// Loaded by URL rather than declared (see shell.qml), which leaves no implicit scope for a sibling.
import qs.windows.files

// One Files window: a bar of tabs, a toolbar with the trail the open tab has walked, and its folder.
FloatingWindow {
    id: root
    required property var modelData

    title: root.tabs.length ? Engine.displayName(root.tab.path) + " — Files" : "Files"
    visible: true
    implicitWidth: 980
    implicitHeight: 680
    minimumSize: Qt.size(640, 420)
    color: Theme.light ? "#FFFFFF" : "#131417"
    onVisibleChanged: if (!visible) FileWindows.close(root.modelData.id)

    // A tab is a folder and the trail it has walked: { path, history, at }. One Directory serves
    // whichever is open, since only the open one is being looked at.
    property var tabs: [{ path: modelData.path, history: [modelData.path], at: 0 }]
    property int current: 0
    readonly property var tab: tabs[current] || tabs[0]
    readonly property bool canBack: tab.at > 0
    readonly property bool canForward: tab.at < tab.history.length - 1

    // A var array only says it changed when it is put back whole.
    function setTab(i, next) {
        const all = tabs.slice();
        all[i] = next;
        tabs = all;
    }

    readonly property bool showingRecents: tab.path === Places.recentsPath

    // A place that is not a folder says its own name rather than the path it happens to live at.
    readonly property var crumbs: showingRecents ? [{ name: "Recents", path: Places.recentsPath }]
        : searchingUnder ? [{ name: "Found under " + Engine.displayName(tab.path), path: tab.path }]
        : showingTrash ? [{ name: "Trash", path: Places.trashFiles }]
        : Engine.crumbs(dir.path)
    readonly property bool showingTrash: tab.path === Places.trashFiles

    // The one place the folder is pointed anywhere. Recents is a gathering rather than a folder, so
    // it is named rather than opened; everything else is a path. Every way of moving goes through
    // here, so the tab and what is on screen cannot drift apart.
    function show(to) {
        clearSearch();
        if (to === Places.recentsPath) {
            Places.refreshRecents();
            dir.paths = Places.recents.map(r => r.path);
            return;
        }
        // The trash is not one folder: a file on another volume goes to that volume's own.
        if (to === Places.trashFiles) {
            dir.paths = FileJobs.trashContents();
            return;
        }
        dir.paths = [];
        // A gathering leaves the path where it was, so pointing it back at the same folder has to
        // say so rather than be taken for no change at all.
        if (dir.path === to) dir.refresh();
        else dir.path = to;
    }

    function go(to) {
        if (!to || (to === tab.path && to !== Places.recentsPath)) return;
        show(to);
        const h = tab.history.slice(0, tab.at + 1);
        h.push(to);
        setTab(current, { path: to, history: h.slice(-50), at: Math.min(h.length, 50) - 1 });
    }
    function back() {
        if (!canBack) return;
        const to = tab.history[tab.at - 1];
        setTab(current, { path: to, history: tab.history, at: tab.at - 1 });
        show(to);
    }
    function forward() {
        if (!canForward) return;
        const to = tab.history[tab.at + 1];
        setTab(current, { path: to, history: tab.history, at: tab.at + 1 });
        show(to);
    }
    function up() { if (!showingRecents) go(Engine.parentOf(tab.path)); }

    function newTab(path) {
        const to = path || Engine.home;
        tabs = tabs.concat([{ path: to, history: [to], at: 0 }]);
        current = tabs.length - 1;
    }
    // Another application asking the desktop to open a folder wants to see that folder, not a second
    // tab on it. Asking for a new tab here is asking for a new tab, even on the folder already open.
    function showFolderInTab(path) {
        const already = tabs.findIndex(t => t.path === path);
        if (already >= 0) showTab(already);
        else newTab(path);
    }
    function closeTab(i) {
        if (tabs.length <= 1) { FileWindows.close(root.modelData.id); return; }
        const all = tabs.slice();
        all.splice(i, 1);
        const was = current;
        tabs = all;
        const next = Math.max(0, Math.min(all.length - 1, was > i ? was - 1 : was));
        if (next === current) show(tabs[next].path);
        else current = next;
    }
    function showTab(i) {
        if (i < 0 || i >= tabs.length) return;
        current = i;
    }
    // Showing a tab is what moves the folder, so the two cannot drift apart. The tab is read out of
    // the list rather than through `tab`, whose binding has not been worked out again by the time a
    // handler on the property it depends on runs: it would still name the tab being left.
    onCurrentChanged: if (tabs[current]) root.show(tabs[current].path)

    // A bookmark dropped on another takes its place in the order, the rest closing up behind it.
    function reorderPlace(moved, before) {
        if (!moved || moved === before) return;
        const order = Places.bookmarks.map(b => b.path).filter(p => p !== moved);
        const at = order.indexOf(before);
        order.splice(at < 0 ? order.length : at, 0, moved);
        Places.reorderBookmarks(order);
    }

    // A search worth keeping: what was looked for, where, and how it was narrowed.
    function askSaveSearch() {
        sheet.mode = "name"; sheet.title = "Save this search"; sheet.message = "";
        sheet.acceptLabel = "Save"; sheet.danger = false; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "saveSearch";
        sheet.ask(search.text.trim() || Engine.displayName(tab.path));
    }
    function saveSearch(name) {
        if (!name) return;
        Prefs.p.filesSearches = Prefs.p.filesSearches.concat([{
            name: name, path: tab.path, term: search.text.trim(),
            kind: searchKind, when: searchWhen, under: searchingUnder,
        }]);
    }
    function forgetSearch(name) {
        Prefs.p.filesSearches = Prefs.p.filesSearches.filter(s => s.name !== name);
    }
    // Running a kept search puts the window back the way it was when the search was kept.
    function runSearch(saved) {
        show(saved.path);
        setTab(current, { path: saved.path, history: [saved.path], at: 0 });
        search.text = saved.term;
        searchKind = saved.kind || "";
        searchWhen = saved.when || "any";
        if (saved.under) searchUnder();
        else dir.filter = saved.term;
    }

    function putBack() { if (acting.length) FileJobs.restoreFromTrash(acting); }
    function askEmptyTrash() {
        sheet.mode = "confirm"; sheet.title = "Empty the trash?";
        sheet.message = "Everything in it goes for good.";
        sheet.acceptLabel = "Empty"; sheet.danger = true; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "emptyTrash";
        sheet.ask("");
    }

    // Another window asking this one to take a tab.
    // The list of recents is read by a script and arrives after it is asked for; showing Recents
    // has to follow the answer rather than whatever was there before the question.
    Connections {
        target: Places
        enabled: root.showingRecents && !root.searchingUnder
        function onRecentsChanged() { dir.paths = Places.recents.map(r => r.path); }
    }

    Connections {
        target: FileWindows
        function onRaised() {
            const asked = FileWindows.takeAsked(root.modelData.id);
            if (asked) root.showFolderInTab(asked.path);
        }
    }

    // What Copy or Cut put aside, and which of the two it was.
    property var clipboard: []
    property bool clipboardCut: false

    // Everything an action applies to: what is picked, or the row the keys are on.
    readonly property var acting: view ? view.acting : []
    readonly property string actingOne: acting.length === 1 ? acting[0] : ""
    // How much is picked, which is what a person wants before copying it somewhere.
    // Only files are added up. A folder's size means walking all of it, which is not something to
    // do in a binding that runs on every change of what is picked.
    readonly property string pickedSize: view && view.selection.length
        && !view.selection.some(p => Engine.isDir(p)) ? Engine.sizeOf(view.selection) : ""

    function said(n) { return n === 1 ? Engine.displayName(acting[0]) : n + " items"; }

    function copyToClipboard(cut) {
        if (!acting.length) return;
        clipboard = acting.slice();
        clipboardCut = cut;
        IslandEvents.show({ kind: "text", glyph: cut ? "scissors" : "copy",
                            text: said(acting.length) + (cut ? " cut" : " copied") });
    }
    function paste() {
        if (!clipboard.length) return;
        clipboardCut ? FileJobs.move(clipboard, dir.path) : FileJobs.copy(clipboard, dir.path);
        if (clipboardCut) { clipboard = []; clipboardCut = false; }
    }
    function askRename() {
        if (!actingOne) return;
        view.beginRename(actingOne);
    }

    function askRenameMany() {
        sheet.mode = "name"; sheet.title = "Rename " + acting.length + " items";
        sheet.message = "One name for all of them. # is where the number goes; each keeps its ending.";
        sheet.acceptLabel = "Rename"; sheet.danger = false; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "renameMany";
        sheet.ask("item #");
    }

    function askCompress() {
        if (!acting.length) return;
        sheet.mode = "name"; sheet.title = "Compress " + said(acting.length);
        sheet.message = "The ending decides the format.";
        sheet.acceptLabel = "Compress"; sheet.danger = false; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "compress";
        sheet.ask((acting.length === 1 ? Engine.displayName(acting[0]) : Engine.displayName(dir.path)) + ".tar.gz");
    }
    function askGoTo() {
        sheet.mode = "name"; sheet.title = "Go to folder"; sheet.message = "";
        sheet.acceptLabel = "Go"; sheet.danger = false; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "goto";
        sheet.ask(dir.path);
    }

    function duplicate() { if (acting.length) FileJobs.duplicate(acting); }

    // Opening a file: whatever the desktop says handles it, and when nothing does, the question of
    // what should, rather than xdg-open's guess.
    property string opening: ""
    function openFile(path) {
        opening = path;
        opener.command = ["python3", Quickshell.shellDir + "/scripts/openwith.py", "open", path];
        opener.running = true;
    }
    Process {
        id: opener
        onExited: code => { if (code === 3) root.askOpenWith(root.opening); }
    }

    // Everything that says it handles the file, for the person to choose from.
    function askOpenWith(path) {
        opening = path;
        lister.command = ["python3", Quickshell.shellDir + "/scripts/openwith.py", "list", path];
        lister.running = true;
    }
    Process {
        id: lister
        stdout: StdioCollector {
            onStreamFinished: {
                let found;
                try { found = JSON.parse(text); } catch (e) { return; }
                const run = (how, id) => Compositor.exec("python3 "
                    + Compositor.quote(Quickshell.shellDir + "/scripts/openwith.py")
                    + " " + how + " " + Compositor.quote(id) + " " + Compositor.quote(root.opening));
                const apps = (found.apps || []).map(a => ({
                    label: a.name,
                    glyph: a.id === found.default ? "check" : "",
                    action: () => run("with", a.id),
                }));
                // Making a choice stand for every file of the type is a choice about the app that
                // was picked, so it hangs off each one rather than off the list.
                const always = (found.apps || []).map(a => ({
                    label: "Always open " + (found.kind || "these") + " with " + a.name,
                    glyph: a.id === found.default ? "check" : "",
                    action: () => run("always", a.id),
                }));
                menu.items = apps.length
                    ? apps.concat([null]).concat(always)
                    : [{ label: "Nothing here opens " + (found.kind || "this"), enabled: false, action: () => {} }];
                menu.popup(overlay.width / 2 - 105, overlay.height / 3);
            }
        }
    }

    // An empty file, named where it lands, as a new folder is. What it is actually called is the
    // job's to say: a folder that already has an "untitled" gets the next free name instead.
    function newFile() { root.nameWhenMade(FileJobs.newFile(dir.path, "untitled")); }

    function openTerminalHere() { Compositor.exec("kitty -d " + Compositor.quote(dir.path)); }

    // The search field narrows the folder as it is typed; Enter looks underneath it as well, which
    // is what a search is for once the folder in front of you does not have the thing.
    property bool searchingUnder: false
    // Whether anything is being searched for at all, which is when the narrowing bar is worth having.
    readonly property bool searching: searchingUnder || search.text.trim() !== ""

    // What "changed since" means, as a moment or nothing at all.
    function sinceFor(when) {
        const now = new Date();
        if (when === "today") return new Date(now.getFullYear(), now.getMonth(), now.getDate());
        if (when === "week") return new Date(now.getTime() - 7 * 24 * 3600 * 1000);
        if (when === "month") return new Date(now.getTime() - 30 * 24 * 3600 * 1000);
        if (when === "year") return new Date(now.getFullYear(), 0, 1);
        return undefined;
    }
    property string searchKind: ""
    property string searchWhen: "any"
    onSearchKindChanged: dir.kind = searchKind
    onSearchWhenChanged: dir.since = sinceFor(searchWhen)

    function searchUnder() {
        const term = search.text.trim();
        if (!term) return;
        searchingUnder = true;
        dir.filter = "";
        finder.running = false;
        finder.command = ["fd", "--max-results", "500", "-i", "-H",
                          "-E", ".git", "-E", "node_modules", "-E", ".cache",
                          "-p", term.split(" ").join(".*"), dir.path];
        finder.running = true;
    }
    // Stop looking underneath and put the folder itself back, keeping whatever is in the field so
    // it goes on narrowing what is there.
    function leaveSearchUnder() {
        searchingUnder = false;
        finder.running = false;
        dir.paths = [];
        dir.path = tab.path;
        dir.refresh();
    }
    // Forget the search entirely; the caller decides where the folder goes next.
    function clearSearch() {
        searchingUnder = false;
        finder.running = false;
        dir.filter = "";
        search.text = "";
        searchKind = "";
        searchWhen = "any";
    }
    // Stopping a search puts back the folder the search was run under.
    function stopSearching() { show(tab.path); }

    Process {
        id: finder
        stdout: StdioCollector {
            onStreamFinished: dir.paths = text.split("\n").filter(l => l.length)
        }
    }

    function askNewFolder() { root.nameWhenMade(FileJobs.newFolder(dir.path, "untitled folder")); }

    // What a new file or folder is called is the job's to say: a folder that already has an
    // "untitled" in it gets the next free name, and guessing would open the editor on the wrong row.
    function nameWhenMade(job) {
        if (!job) return;
        if (job.state !== FileJob.Running && job.state !== FileJob.Asking) {
            if (job.made.length) view.beginRenameWhenSeen(job.made[0]);
            return;
        }
        naming.target = job;
    }
    Connections {
        id: naming
        target: null
        function onStateChanged() {
            const job = naming.target;
            if (job.state === FileJob.Running || job.state === FileJob.Asking) return;
            naming.target = null;
            if (job.made.length) view.beginRenameWhenSeen(job.made[0]);
        }
    }
    function askDelete() {
        if (!acting.length) return;
        sheet.mode = "confirm"; sheet.title = "Delete " + said(acting.length) + "?";
        sheet.message = "This does not go to the trash and cannot be undone.";
        sheet.acceptLabel = "Delete"; sheet.danger = true; sheet.offerAll = false; sheet.alternateLabel = "";
        sheet.job = null; sheet.what = "delete";
        sheet.ask("");
    }
    function toTrash() {
        if (acting.length) FileJobs.trash(acting);
    }

    // A job that meets something already there stops and asks through the same sheet. Only the
    // window that started the job asks, so two windows do not both put the question up.
    Connections {
        target: FileJobs
        function onJobAsking(job) {
            if (sheet.job === job) return;
            if (job.destination && job.destination !== dir.path) return;
            root.askConflict(job);
        }
        function onJobFinished(job) {
            if (job.state === FileJob.Failed)
                IslandEvents.show({ kind: "text", glyph: "triangle-alert", text: job.error });
            // A gathering is named rather than watched, so what the trash holds is asked for again.
            if (root.showingTrash) dir.paths = FileJobs.trashContents();
        }
    }

    function askConflict(job) {
        sheet.mode = "confirm";
        sheet.title = job.conflictName + " is already there";
        sheet.message = "Replace it, keep both, or leave it as it is.";
        sheet.acceptLabel = "Replace"; sheet.danger = true; sheet.offerAll = job.total > 1;
        sheet.alternateLabel = "Keep both";
        sheet.job = job; sheet.what = "conflict";
        sheet.ask("");
    }

    // Whether the keyboard is in a text field. Qt matches a Shortcut before the focused item ever
    // sees the key, so the three places that take typing say when they have it.
    readonly property bool typing: search.input.activeFocus || view.renaming !== "" || sheet.typing

    // Which name on the bar has its menu open, so the bar can mark it and the next one can take over.
    property string openBarMenu: ""

    function barItems(name) {
        const on = acting.length > 0;
        const many = acting.length > 1;
        if (name === "File") return [
            on && !many ? { label: "Open", glyph: "external-link", action: () => view.openPath(actingOne) } : undefined,
            on && !many ? { label: "Open with…", glyph: "app-window", action: () => askOpenWith(actingOne) } : undefined,
            null,
            { label: "New folder", glyph: "folder-plus", action: askNewFolder },
            { label: "New file", glyph: "file-plus", action: newFile },
            { label: "Open in terminal", glyph: "terminal", action: openTerminalHere },
            on ? null : undefined,
            on && !many ? { label: "Rename", glyph: "pencil", action: askRename } : undefined,
            many ? { label: "Rename " + acting.length + " items…", glyph: "pencil", action: askRenameMany } : undefined,
            on && !many ? { label: "Get info", glyph: "info", action: () => peek.look(actingOne) } : undefined,
            on ? { label: "Duplicate", glyph: "copy", action: duplicate } : undefined,
            on ? { label: "Compress", glyph: "archive", action: askCompress } : undefined,
            on && !many && FileJobs.isArchive(acting[0]) ? { label: "Extract here", glyph: "package-open", action: () => FileJobs.extract(acting[0]) } : undefined,
            null,
            { label: "New tab", glyph: "plus", action: () => newTab() },
            { label: "New window", glyph: "app-window", action: () => FileWindows.add(dir.path) },
            null,
            { label: "Close tab", glyph: "x", action: () => closeTab(current) },
        ].filter(i => i !== undefined);
        if (name === "Edit") return [
            on ? { label: "Copy", glyph: "copy", action: () => copyToClipboard(false) } : undefined,
            on ? { label: "Cut", glyph: "scissors", action: () => copyToClipboard(true) } : undefined,
            { label: "Paste", glyph: "clipboard", enabled: clipboard.length > 0, action: paste },
            null,
            { label: "Select all", glyph: "check", action: () => view.selectAll() },
            null,
            { label: FileJobs.canUndo ? FileJobs.undoLabel : "Undo", glyph: "corner-up-left", enabled: FileJobs.canUndo, action: () => FileJobs.undo() },
            null,
            showingTrash && on ? { label: "Put back", glyph: "corner-up-left", action: putBack } : undefined,
            on ? { label: "Move to trash", glyph: "trash", enabled: !showingTrash, action: toTrash } : undefined,
            on ? { label: "Delete", glyph: "x", danger: true, action: askDelete } : undefined,
            showingTrash ? { label: "Empty the trash", glyph: "trash", danger: true, enabled: dir.count > 0, action: askEmptyTrash } : undefined,
        ].filter(i => i !== undefined);
        if (name === "View") return [
            { label: "List", glyph: "list", action: () => view.mode = "list" },
            { label: "Columns", glyph: "columns-3", action: () => view.mode = "columns" },
            { label: "Grid", glyph: "layout-grid", action: () => view.mode = "grid" },
            null,
            { label: "Sort by name", glyph: dir.sort === Directory.ByName ? "check" : "", action: () => dir.sort = Directory.ByName },
            { label: "Sort by size", glyph: dir.sort === Directory.BySize ? "check" : "", action: () => dir.sort = Directory.BySize },
            { label: "Sort by date", glyph: dir.sort === Directory.ByModified ? "check" : "", action: () => dir.sort = Directory.ByModified },
            { label: "Sort by kind", glyph: dir.sort === Directory.ByKind ? "check" : "", action: () => dir.sort = Directory.ByKind },
            null,
            { label: "No groups", glyph: dir.grouping === Directory.NoGroups ? "check" : "", action: () => dir.grouping = Directory.NoGroups },
            { label: "Group by kind", glyph: dir.grouping === Directory.ByKindGroups ? "check" : "", action: () => dir.grouping = Directory.ByKindGroups },
            { label: "Group by date", glyph: dir.grouping === Directory.ByDateGroups ? "check" : "", action: () => dir.grouping = Directory.ByDateGroups },
            { label: "Group by size", glyph: dir.grouping === Directory.BySizeGroups ? "check" : "", action: () => dir.grouping = Directory.BySizeGroups },
            null,
            { label: "Bigger icons", glyph: "zoom-in", enabled: view.iconSize < 160, action: () => view.iconSize = Math.min(160, view.iconSize + 32) },
            { label: "Smaller icons", glyph: "zoom-out", enabled: view.iconSize > 32, action: () => view.iconSize = Math.max(32, view.iconSize - 32) },
            null,
            { label: Prefs.p.filesPreview ? "Hide preview" : "Show preview", glyph: "panel-right",
              action: () => Prefs.p.filesPreview = !Prefs.p.filesPreview },
            { label: dir.showHidden ? "Hide hidden files" : "Show hidden files", glyph: "eye", action: () => dir.showHidden = !dir.showHidden },
            { label: "Refresh", glyph: "refresh-cw", action: () => dir.refresh() },
        ];
        return [
            { label: "Back", glyph: "chevron-left", enabled: canBack, action: back },
            { label: "Forward", glyph: "chevron-right", enabled: canForward, action: forward },
            { label: "Up", glyph: "arrow-up", enabled: dir.path !== "/", action: up },
            null,
            { label: "Home", glyph: "house", action: () => go(Engine.home) },
            null,
            { label: "Go to folder…", glyph: "search", action: askGoTo },
        ];
    }

    function openBarMenuAt(name, item) {
        openBarMenu = name;
        menu.items = barItems(name);
        const at = item.mapToItem(overlay, 0, item.height);
        menu.popup(at.x, at.y);
    }

    // The menu the right button and the Menu key both open, at a point in the overlay's own frame.
    function showMenu(x, y, path) {
        const on = acting.length > 0;
        const many = acting.length > 1;
        menu.items = (showingTrash ? [
            on ? { label: "Put back", glyph: "corner-up-left", action: putBack } : undefined,
            on ? { label: "Delete", glyph: "x", danger: true, action: askDelete } : undefined,
            on ? null : undefined,
            { label: "Empty the trash", glyph: "trash", danger: true, enabled: dir.count > 0, action: askEmptyTrash },
        ] : [
            on && !many ? { label: "Open", glyph: "external-link", action: () => view.openPath(path || actingOne) } : undefined,
            on && !many ? { label: "Open with…", glyph: "app-window", action: () => askOpenWith(path || actingOne) } : undefined,
            on && !many ? { label: "Rename", glyph: "pencil", action: askRename } : undefined,
            many ? { label: "Rename " + acting.length + " items…", glyph: "pencil", action: askRenameMany } : undefined,
            on ? { label: "Duplicate", glyph: "copy", action: duplicate } : undefined,
            on ? { label: "Get info", glyph: "info", enabled: !many, action: () => peek.look(actingOne) } : undefined,
            on && !many && FileJobs.isArchive(path) ? { label: "Extract here", glyph: "package-open", action: () => FileJobs.extract(path) } : undefined,
            on ? { label: "Compress", glyph: "archive", action: askCompress } : undefined,
            on ? null : undefined,
            on ? { label: "Copy", glyph: "copy", action: () => copyToClipboard(false) } : undefined,
            on ? { label: "Cut", glyph: "scissors", action: () => copyToClipboard(true) } : undefined,
            { label: "Paste", glyph: "clipboard", enabled: clipboard.length > 0, action: paste },
            { label: "Select all", glyph: "check", action: () => view.selectAll() },
            { label: FileJobs.canUndo ? FileJobs.undoLabel : "Undo", glyph: "corner-up-left", enabled: FileJobs.canUndo, action: () => FileJobs.undo() },
            null,
            { label: "New folder", glyph: "folder-plus", action: askNewFolder },
            { label: "New file", glyph: "file-plus", action: newFile },
            { label: "Open in terminal", glyph: "terminal", action: openTerminalHere },
            on ? null : undefined,
            on ? { label: "Move to trash", glyph: "trash", action: toTrash } : undefined,
            on ? { label: "Delete", glyph: "x", danger: true, action: askDelete } : undefined,
        ]).filter(i => i !== undefined);
        openBarMenu = "";
        const at = view.mapToItem(overlay, x, y);
        menu.popup(at.x, at.y);
    }

    Directory {
        id: dir
        path: root.modelData.path
        // How it was last left, and kept that way for the next window and the next session.
        showHidden: Prefs.p.filesHidden
        sort: Prefs.p.filesSort
        grouping: Prefs.p.filesGrouping
        onShowHiddenChanged: if (Prefs.loaded) Prefs.p.filesHidden = showHidden
        onSortChanged: if (Prefs.loaded) Prefs.p.filesSort = sort
        onGroupingChanged: if (Prefs.loaded) Prefs.p.filesGrouping = grouping
    }

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.BackButton | Qt.ForwardButton
        onClicked: mouse => mouse.button === Qt.BackButton ? root.back() : root.forward()
    }
    Shortcut { sequences: ["Alt+Left", StandardKey.Back]; onActivated: root.back() }
    Shortcut { sequences: ["Alt+Right", StandardKey.Forward]; onActivated: root.forward() }
    Shortcut { sequence: "Alt+Up"; onActivated: root.up() }
    Shortcut { sequence: "Ctrl+H"; onActivated: dir.showHidden = !dir.showHidden }
    Shortcut { sequences: [StandardKey.Refresh]; onActivated: dir.refresh() }
    Shortcut { sequence: "Ctrl+F"; onActivated: search.input.forceActiveFocus() }
    Shortcut { sequences: ["Ctrl+1"]; onActivated: view.mode = "list" }
    Shortcut { sequences: ["Ctrl+2"]; onActivated: view.mode = "columns" }
    Shortcut { sequences: ["Ctrl+3"]; onActivated: view.mode = "grid" }
    Shortcut { sequence: "F2"; onActivated: root.askRename() }
    Shortcut { sequence: "Ctrl+Shift+F"; onActivated: root.newFile() }
    Shortcut { sequences: [StandardKey.Copy]; onActivated: root.copyToClipboard(false) }
    Shortcut { sequences: [StandardKey.Cut]; onActivated: root.copyToClipboard(true) }
    Shortcut { sequences: [StandardKey.Paste]; onActivated: root.paste() }

    Shortcut { sequences: [StandardKey.Undo]; onActivated: FileJobs.undo() }
    Shortcut { sequence: "Ctrl+Shift+N"; onActivated: root.askNewFolder() }
    Shortcut { sequences: [StandardKey.SelectAll]; onActivated: view.selectAll() }
    Shortcut { sequences: [StandardKey.AddTab]; onActivated: root.newTab() }
    Shortcut { sequences: [StandardKey.Close]; onActivated: root.closeTab(root.current) }
    Shortcut { sequence: "Ctrl+N"; onActivated: FileWindows.add(dir.path) }
    Shortcut { sequence: "Ctrl+D"; onActivated: root.duplicate() }
    Shortcut { sequences: ["Ctrl+L", "Ctrl+Shift+G"]; onActivated: root.askGoTo() }

    Shortcut { sequence: "Ctrl+I"; onActivated: if (root.actingOne) peek.look(root.actingOne) }
    Shortcut { sequence: "Ctrl+Shift+P"; onActivated: Prefs.p.filesPreview = !Prefs.p.filesPreview }
    Shortcut { sequences: ["Ctrl++", "Ctrl+="]; onActivated: view.iconSize = Math.min(160, view.iconSize + 32) }
    Shortcut { sequence: "Ctrl+-"; onActivated: view.iconSize = Math.max(32, view.iconSize - 32) }
    // A key that is also a character belongs to whatever is being typed into before it belongs to
    // the window. Qt matches a Shortcut before the focused item ever sees the key, so these say so.
    Shortcut { sequence: "Backspace"; enabled: !root.typing; onActivated: root.up() }
    Shortcut { sequences: [StandardKey.Delete]; enabled: !root.typing; onActivated: root.toTrash() }
    Shortcut { sequence: "Shift+Delete"; enabled: !root.typing; onActivated: root.askDelete() }
    Shortcut { sequence: "Space"; enabled: !root.typing; onActivated: if (root.actingOne) peek.toggle(root.actingOne) }
    Shortcut { sequences: [StandardKey.NextChild]; onActivated: root.showTab((root.current + 1) % root.tabs.length) }
    Shortcut { sequences: [StandardKey.PreviousChild]; onActivated: root.showTab((root.current + root.tabs.length - 1) % root.tabs.length) }
    // The keyboard's own way to the context menu, which every desktop offers and which is also the
    // only way to reach it without a pointer.
    Shortcut {
        sequences: ["Menu", "Shift+F10"]
        onActivated: root.showMenu(view.width / 2, view.height / 3, view.acting.length ? view.acting[0] : "")
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // The menu bar every file manager has, so the things the right button offers can also be
        // found by reading rather than by guessing where to click.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 28
            color: "transparent"
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.hairline }

            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s2; rightMargin: Theme.s2 }
                spacing: 0
                Repeater {
                    model: ["File", "Edit", "View", "Go"]
                    delegate: Rectangle {
                        id: barItem
                        required property string modelData
                        implicitWidth: barLabel.implicitWidth + Theme.s3 * 2
                        Layout.fillHeight: true
                        radius: Theme.radiusChip
                        color: root.openBarMenu === barItem.modelData ? Theme.pressed
                             : barArea.containsMouse ? Theme.raised : "transparent"
                        Label {
                            id: barLabel
                            anchors.centerIn: parent
                            text: barItem.modelData
                            size: Theme.sizeSmall
                            color: Theme.text2
                        }
                        MouseArea {
                            id: barArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: root.openBarMenuAt(barItem.modelData, barItem)
                            // With one open, moving along the bar opens the next, as a menu bar does.
                            onEntered: if (root.openBarMenu !== "") root.openBarMenuAt(barItem.modelData, barItem)
                        }
                    }
                }
                Item { Layout.fillWidth: true }
            }
        }

        // The tabs, which appear once there is more than one folder open in this window.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 34
            visible: root.tabs.length > 1
            // The strip sits back so the chips on it come forward; a chip the same shade as the
            // window behind it is not a chip at all.
            color: Qt.alpha(Theme.ink, 0.45)
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.hairline }

            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s2; rightMargin: Theme.s2 }
                spacing: 2
                Repeater {
                    model: root.tabs
                    delegate: Rectangle {
                        id: tabItem
                        required property var modelData
                        required property int index
                        // Wide enough for the name and no wider, so two tabs are two chips rather
                        // than two halves of the window, and narrowing only starts once they fill it.
                        Layout.fillWidth: true
                        Layout.preferredWidth: label.implicitWidth + 18 + Theme.s3 + Theme.s2 * 2
                        Layout.maximumWidth: 200
                        Layout.minimumWidth: 90
                        // A chip standing clear of the line under the strip. Filling the height
                        // would put its bottom edge on that line and paint over it.
                        Layout.fillHeight: true
                        Layout.topMargin: 4
                        Layout.bottomMargin: 5
                        radius: Theme.radiusChip
                        // Every tab is a chip so the strip reads as tabs; the open one is the one
                        // that stands out of the row rather than the only one in it.
                        color: root.current === tabItem.index ? Theme.pressed
                             : tabArea.containsMouse ? Theme.pressed : Theme.raised

                        RowLayout {
                            anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s2 }
                            spacing: Theme.s2
                            // Above the tab's own click area, which covers the whole tab and is
                            // declared after this; only the cross in here takes a click.
                            z: 1
                            Label {
                                id: label
                                Layout.fillWidth: true
                                size: Theme.sizeSmall
                                elide: Text.ElideRight
                                color: root.current === tabItem.index ? Theme.text : Theme.text2
                                text: Engine.displayName(tabItem.modelData.path)
                            }
                            // A box big enough to hit around a glyph that is smaller than a target
                            // wants to be. A click outside the box belongs to the tab. The room is
                            // kept whether or not the cross is showing, so nothing shifts under the
                            // pointer as it moves along the strip.
                            Item {
                                Layout.preferredWidth: 18
                                Layout.preferredHeight: 18
                                Glyph {
                                    anchors.centerIn: parent
                                    visible: tabArea.containsMouse || closeArea.containsMouse
                                        || root.current === tabItem.index
                                    name: "x"
                                    size: 12
                                    color: closeArea.containsMouse ? Theme.text : Theme.text3
                                }
                                MouseArea {
                                    id: closeArea
                                    anchors.fill: parent
                                    hoverEnabled: true
                                    onClicked: root.closeTab(tabItem.index)
                                }
                            }
                        }
                        MouseArea {
                            id: tabArea
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                            onClicked: mouse => mouse.button === Qt.MiddleButton ? root.closeTab(tabItem.index)
                                                                                 : root.showTab(tabItem.index)
                        }
                    }
                }
                Button { variant: "text"; glyph: "plus"; onClicked: root.newTab() }
                Item { Layout.fillWidth: true }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

        // Sidebar
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 190
            color: "transparent"
            Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.hairline }

            // A folder dragged here is bookmarked, which is the way most desktops let one be added.
            DropArea {
                anchors.fill: parent
                keys: ["text/uri-list"]
                onDropped: drop => {
                    const paths = String(drop.getDataAsString("text/uri-list")).split(/\r?\n/)
                        .filter(u => u.startsWith("file://")).map(u => decodeURI(u.slice(7)));
                    for (const p of paths)
                        if (Engine.isDir(p)) Places.addBookmark(p);
                    drop.acceptProposedAction();
                }
                Rectangle {
                    anchors { fill: parent; margins: Theme.s2 }
                    visible: parent.containsDrag
                    color: "transparent"
                    border.width: 2
                    border.color: Theme.accent
                    radius: Theme.radiusControl
                }
            }

            Flickable {
                anchors { fill: parent; margins: Theme.s2; topMargin: Theme.s3 }
                contentHeight: sideCol.implicitHeight
                clip: true
                ColumnLayout {
                    id: sideCol
                    width: parent.width
                    spacing: 1
                    // Searches worth keeping sit under their own heading, above the desktop's places.
                    Label {
                        visible: Prefs.p.filesSearches.length > 0
                        Layout.leftMargin: Theme.s3
                        Layout.topMargin: Theme.s3
                        Layout.bottomMargin: Theme.s1
                        text: "Searches"
                        size: Theme.sizeCaption
                        color: Theme.text3
                    }
                    Repeater {
                        model: Prefs.p.filesSearches
                        delegate: ListRow {
                            id: saved
                            required property var modelData
                            Layout.fillWidth: true
                            glyph: "search"
                            title: saved.modelData.name
                            onClicked: root.runSearch(saved.modelData)
                            HoverHandler { id: savedHover }
                            Glyph {
                                visible: savedHover.hovered
                                name: "x"
                                size: 14
                                color: forgetArea.containsMouse ? Theme.text : Theme.text3
                                MouseArea {
                                    id: forgetArea
                                    anchors { fill: parent; margins: -6 }
                                    hoverEnabled: true
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: root.forgetSearch(saved.modelData.name)
                                }
                            }
                        }
                    }

                    Repeater {
                        model: Places.places
                        delegate: Item {
                            id: place
                            required property var modelData
                            required property int index
                            Layout.fillWidth: true
                            implicitHeight: heading.visible ? 30 + row.implicitHeight : row.implicitHeight

                            readonly property bool first: index === 0
                                || Places.places[index - 1].group !== place.modelData.group

                            Label {
                                id: heading
                                visible: place.first
                                x: Theme.s3
                                y: Theme.s3
                                text: place.modelData.group
                                size: Theme.sizeCaption
                                color: Theme.text3
                            }
                            HoverHandler { id: rowHover }

                            // A bookmark can be dragged up and down the list to reorder it; the
                            // groups above it are the desktop's own and do not move.
                            readonly property bool movable: place.modelData.bookmark === true

                            DropArea {
                                anchors.fill: parent
                                keys: ["isle/place"]
                                enabled: place.movable
                                onDropped: drop => {
                                    root.reorderPlace(drop.getDataAsString("isle/place"), place.modelData.path);
                                    drop.acceptProposedAction();
                                }
                                Rectangle {
                                    anchors { left: parent.left; right: parent.right; top: parent.top }
                                    height: 2
                                    visible: parent.containsDrag
                                    color: Theme.accent
                                }
                            }

                            ListRow {
                                id: row
                                anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                                glyph: place.modelData.glyph
                                title: place.modelData.name
                                selected: dir.path === place.modelData.path
                                onClicked: root.go(place.modelData.path)

                                Drag.active: dragger.active
                                Drag.dragType: Drag.Automatic
                                Drag.supportedActions: Qt.MoveAction
                                Drag.keys: ["isle/place"]
                                Drag.mimeData: ({ "isle/place": place.modelData.path })
                                DragHandler {
                                    id: dragger
                                    enabled: place.movable
                                    target: null
                                    onActiveChanged: if (active) row.Drag.startDrag()
                                }
                                // Ejecting a device and taking a bookmark out are the row's own
                                // actions and belong on it, not in a menu.
                                Glyph {
                                    visible: place.modelData.eject || (place.modelData.bookmark && rowHover.hovered)
                                    name: place.modelData.eject ? "eject" : "x"
                                    size: 14
                                    color: actionArea.containsMouse ? Theme.text : Theme.text3
                                    MouseArea {
                                        id: actionArea
                                        anchors { fill: parent; margins: -6 }
                                        hoverEnabled: true
                                        cursorShape: Qt.PointingHandCursor
                                        onClicked: place.modelData.eject ? Disks.eject(place.modelData.volume)
                                                                         : Places.removeBookmark(place.modelData.path)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

    ColumnLayout {
        Layout.fillWidth: true
        Layout.fillHeight: true
        spacing: 0

        // Toolbar
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 56
            color: "transparent"
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.hairline }

            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                spacing: Theme.s2

                Button { variant: "text"; glyph: "chevron-left"; enabled: root.canBack; onClicked: root.back() }
                Button { variant: "text"; glyph: "chevron-right"; enabled: root.canForward; onClicked: root.forward() }
                Button { variant: "text"; glyph: "arrow-up"; enabled: dir.path !== "/"; onClicked: root.up() }

                // The trail as buttons; the last one is where you are and does nothing.
                Flickable {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 34
                    contentWidth: crumbRow.implicitWidth
                    flickableDirection: Flickable.HorizontalFlick
                    clip: true
                    // A deep folder keeps its tail in view rather than its root.
                    onContentWidthChanged: contentX = Math.max(0, contentWidth - width)
                    RowLayout {
                        id: crumbRow
                        height: parent.height
                        spacing: 0
                        Repeater {
                            model: root.crumbs
                            delegate: RowLayout {
                                id: crumb
                                required property var modelData
                                required property int index
                                spacing: 0
                                Glyph { visible: crumb.index > 0; name: "chevron-right"; size: 12; color: Theme.text3 }
                                Button {
                                    variant: "text"
                                    text: crumb.modelData.name
                                    enabled: crumb.modelData.path !== dir.path
                                    onClicked: root.go(crumb.modelData.path)
                                }
                            }
                        }
                    }
                }

                Dropdown {
                    listWidth: 150
                    value: view.mode
                    options: [["list", "List"], ["columns", "Columns"], ["grid", "Grid"]]
                    onPicked: v => view.mode = v
                }

                Field {
                    id: search
                    Layout.fillWidth: true
                    Layout.preferredWidth: 200
                    Layout.maximumWidth: 200
                    Layout.minimumWidth: 90
                    glyph: "search"
                    placeholder: root.searchingUnder ? "Searching underneath" : "Search this folder"
                    // Typing again narrows the folder in front of you, which ends any search that
                    // was running underneath it.
                    onTextChanged: {
                        if (root.searchingUnder) root.leaveSearchUnder();
                        dir.filter = text;
                    }
                    onAccepted: root.searchUnder()
                    input.Keys.onEscapePressed: { root.stopSearching(); view.forceActiveFocus(); }
                }
            }
        }

        // What is being looked for, narrowed the way Finder's search bar narrows it, and kept for
        // next time if it is worth keeping.
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            visible: root.searching
            color: "transparent"
            clip: true
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.hairline }
            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                spacing: Theme.s2
                // The one thing here that can give up room is what it says about where it is looking.
                Label {
                    Layout.fillWidth: true
                    Layout.minimumWidth: 0
                    elide: Text.ElideRight
                    size: Theme.sizeCaption
                    color: Theme.text3
                    text: root.searchingUnder ? "Underneath " + Engine.displayName(root.tab.path) : "In this folder"
                }
                Button {
                    variant: "text"
                    text: root.searchingUnder ? "This folder only" : "Search underneath"
                    onClicked: root.searchingUnder ? root.leaveSearchUnder() : root.searchUnder()
                }
                Dropdown {
                    listWidth: 150
                    value: root.searchKind
                    options: [["", "Any kind"], ["folder", "Folders"], ["image", "Images"],
                              ["audio", "Audio"], ["video", "Video"], ["text", "Text"],
                              ["application", "Other"]]
                    onPicked: v => root.searchKind = v
                }
                Dropdown {
                    listWidth: 150
                    value: root.searchWhen
                    options: [["any", "Any time"], ["today", "Today"], ["week", "Past week"],
                              ["month", "Past month"], ["year", "This year"]]
                    onPicked: v => root.searchWhen = v
                }
                Button { variant: "text"; glyph: "bookmark"; text: "Save"; onClicked: root.askSaveSearch() }
            }
        }

        FolderView {
            id: view
            focus: true
            Component.onCompleted: view.forceActiveFocus()
            Layout.fillWidth: true
            Layout.fillHeight: true
            directory: dir
            mode: Prefs.p.filesView
            iconSize: Prefs.p.filesIconSize
            onModeChanged: if (Prefs.loaded) Prefs.p.filesView = mode
            onIconSizeChanged: if (Prefs.loaded) Prefs.p.filesIconSize = iconSize
            onActivated: path => root.go(path)
            onRenamed: (path, name) => FileJobs.rename(path, name)
            onAsked: path => root.openFile(path)
            onOpenedInTab: path => root.newTab(path)
            // Dropped from somewhere: moved when it is already on this machine and in another
            // folder, since that is what dragging within a desktop means.
            onDropped: (paths, into) => {
                const from = paths.filter(p => Engine.parentOf(p) !== into);
                if (from.length) FileJobs.move(from, into);
            }
            onMenuAsked: (x, y, path) => root.showMenu(x, y, path)
        }

        // Status
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 28
            color: "transparent"
            Rectangle { anchors.top: parent.top; width: parent.width; height: 1; color: Theme.hairline }
            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                Label {
                    Layout.fillWidth: true
                    size: Theme.sizeCaption
                    color: Theme.text3
                    text: dir.status === Directory.Error ? dir.error
                        : dir.status === Directory.Loading ? "Reading…"
                        : view.selection.length ? view.selection.length + " of " + dir.count + " selected"
                            + (root.pickedSize ? ", " + root.pickedSize : "")
                        : dir.filter ? dir.count + (dir.count === 1 ? " match" : " matches")
                        : dir.count + (dir.count === 1 ? " item" : " items")
                }
                Label {
                    visible: dir.showHidden
                    size: Theme.sizeCaption
                    color: Theme.text3
                    text: "Hidden files shown"
                }
                Label {
                    size: Theme.sizeCaption
                    color: Theme.text3
                    text: Engine.freeSpace(dir.path)
                }
                // What is being copied or moved, while it is.
                Repeater {
                    model: FileJobs.running
                    delegate: RowLayout {
                        id: job
                        required property var modelData
                        spacing: Theme.s2
                        // A job waiting to be told what to do is still a job the person may want to
                        // stop, so the row and its cancel stay while it asks.
                        visible: job.modelData.state === FileJob.Running || job.modelData.state === FileJob.Asking
                        Label {
                            size: Theme.sizeCaption
                            color: Theme.text2
                            text: job.modelData.state === FileJob.Asking ? "Waiting on an answer"
                                : (job.modelData.kind === FileJob.Copy ? "Copying " : "Moving ") + job.modelData.current
                                + (job.modelData.total > 1 ? " (" + job.modelData.count + " of " + job.modelData.total + ")" : "")
                        }
                        Rectangle {
                            Layout.preferredWidth: 90
                            Layout.preferredHeight: 4
                            radius: 2
                            color: Theme.hairline
                            Rectangle {
                                width: parent.width * Math.max(0, Math.min(1, job.modelData.progress))
                                height: parent.height
                                radius: 2
                                color: Theme.accent
                            }
                        }
                        Button { variant: "text"; glyph: "x"; onClicked: job.modelData.cancel() }
                    }
                }
            }
        }
    }

        // The preview stands beside the folder, as Finder's does, rather than over it.
        FilePreview {
            Layout.fillHeight: true
            Layout.preferredWidth: 240
            visible: Prefs.p.filesPreview
            picked: root.acting
            onOpened: path => root.openFile(path)
        }
    }
    }

    Item {
        id: overlay
        anchors.fill: parent

        FileMenu { id: menu; onShownChanged: if (!shown) root.openBarMenu = "" }

        QuickLook { id: peek; onOpened: path => { root.openFile(path); peek.close(); } }

        FileSheet {
            id: sheet
            // What the answer is for, and the job waiting on it when it is a conflict.
            property string what: ""
            property var job: null

            onRejected: {
                if (what === "conflict" && job) job.cancel();
                close();
            }
            onAlternate: forAll => {
                if (job) job.answer(FileJob.Keep, forAll);
                close();
            }
            onAccepted: (value, forAll) => {
                if (what === "goto") root.go(value.replace(/^~/, Engine.home));
                else if (what === "renameMany") FileJobs.renameMany(root.acting, value);
                else if (what === "compress") FileJobs.compress(root.acting, value);
                else if (what === "saveSearch") root.saveSearch(value);
                else if (what === "delete") FileJobs.remove(root.acting);
                else if (what === "emptyTrash") FileJobs.emptyTrash();
                else if (what === "conflict" && job) job.answer(FileJob.Replace, forAll);
                close();
            }
        }
    }
}
