pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Which on-demand surfaces are open. Keybindings reach these through the "surfaces" IPC target.
Singleton {
    id: root

    property bool dashboard: false
    // A press on the dashboard that no widget took: menus a widget has open close on it.
    signal dashboardPressed()
    property bool launcher: false
    property string launcherPrefix: ""
    property bool overview: false
    property bool capture: false
    property bool power: false
    property bool settings: false
    property bool monitor: false
    property bool keychain: false
    property bool files: false
    // The folder Files opens at; an empty one means home.
    property string filesPath: ""
    property string settingsPage: "network"
    // Where Settings has been, for its back and forward buttons; a page set from anywhere lands on the trail.
    property var settingsHistory: ["network"]
    property int settingsIndex: 0
    property bool settingsNavigating: false
    readonly property bool settingsCanBack: settingsIndex > 0
    readonly property bool settingsCanForward: settingsIndex < settingsHistory.length - 1
    onSettingsPageChanged: {
        note();
        if (settingsNavigating) return;
        const h = settingsHistory.slice(0, settingsIndex + 1);
        if (h[h.length - 1] === settingsPage) return;
        h.push(settingsPage);
        settingsHistory = h.slice(-50);
        settingsIndex = settingsHistory.length - 1;
    }
    function settingsBack() { if (!settingsCanBack) return; settingsNavigating = true; settingsIndex--; settingsPage = settingsHistory[settingsIndex]; settingsNavigating = false; }
    // Opening a window surface that is already open brings its window to the front instead.
    // Files is not here: it is as many windows as are wanted, each named for the folder it shows,
    // and showFiles below answers before this is ever reached.
    readonly property var windowTitles: ({ settings: "Settings", keychain: "Keychain", monitor: "Monitor" })
    function show(name) {
        if (name === "files") { root.showFiles(""); return; }
        if (root[name]) Windows.focusShellWindow(windowTitles[name]);
        else root[name] = true;
    }
    function showSettings(page) { settingsPage = page; show("settings"); }
    function showFiles(path) { FileWindows.open(path); files = true; }

    // A window surface that is open when the shell reloads comes back where it was. Quickshell rebuilds every
    // window from the changed files, so an update run from Settings would otherwise take the page away while
    // it was being read. The note lives in the runtime directory and is only honoured while it is fresh, so a
    // window closed by hand stays closed and one left open a day ago does not reappear.
    readonly property string reopenFile: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/isle/reopen.json"
    property bool restored: false
    Process { id: noteWriter }
    function note() {
        if (!restored) return;
        const open = ["settings", "keychain", "monitor", "files"].filter(n => root[n]);
        const text = open.length ? JSON.stringify({ open: open, page: settingsPage, at: Date.now() }) : "";
        noteWriter.command = ["sh", "-c", text ? "mkdir -p \"$(dirname \"$1\")\" && printf %s \"$2\" > \"$1\"" : "rm -f \"$1\"", "_", reopenFile, text];
        noteWriter.running = true;
    }
    onSettingsChanged: note()
    onKeychainChanged: note()
    onMonitorChanged: note()
    onFilesChanged: note()
    Process {
        id: noteReader
        command: ["cat", root.reopenFile]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                let n; try { n = JSON.parse(text); } catch (e) { root.restored = true; return; }
                // Two minutes: long enough for an update to run, short enough not to surprise.
                if (n && n.at && Date.now() - n.at < 120000) {
                    if (n.page) root.settingsPage = n.page;
                    for (const name of n.open || []) root[name] = true;
                }
                root.restored = true;
            }
        }
        onExited: root.restored = true
    }
    function settingsForward() { if (!settingsCanForward) return; settingsNavigating = true; settingsIndex++; settingsPage = settingsHistory[settingsIndex]; settingsNavigating = false; }
    // Something full-screen is up; the island keeps to its pill.
    readonly property bool busy: dashboard || launcher || overview || capture || power

    // One full-screen surface at a time: opening any of these closes the others, and its own key closes it.
    onDashboardChanged: if (dashboard) { launcher = false; overview = false; capture = false; power = false; }
    onOverviewChanged: if (overview) { dashboard = false; launcher = false; capture = false; power = false; Switcher.cancel(); }
    onLauncherChanged: if (launcher) { dashboard = false; overview = false; capture = false; power = false; }
    onCaptureChanged: if (capture) { dashboard = false; launcher = false; overview = false; power = false; }
    onPowerChanged: if (power) { dashboard = false; launcher = false; overview = false; capture = false; }

    function toggleDashboard() { dashboard = !dashboard; }
    function openLauncher(prefix) { launcherPrefix = prefix || ""; launcher = true; }
    function closeAll() { dashboard = false; launcher = false; overview = false; capture = false; power = false; Switcher.cancel(); }

    IpcHandler {
        target: "surfaces"
        function dashboard(action: string): void {
            if (action === "open") root.dashboard = true;
            else if (action === "close") root.dashboard = false;
            else root.toggleDashboard();
        }
        function launcher(action: string, prefix: string): void {
            if (action === "close") root.launcher = false;
            else if (action === "open" || !root.launcher) root.openLauncher(prefix === "-" ? "" : prefix);
            else root.launcher = false;
        }
        function overview(action: string): void {
            if (action === "open") root.overview = true;
            else if (action === "close") root.overview = false;
            else root.overview = !root.overview;
        }
        // The bind passes one arg through ipc(), e.g. "toggle region"; split it into action and mode.
        function capture(spec: string): void {
            const parts = (spec || "").split(" ");
            const action = parts[0], mode = parts[1] || "";
            if (mode && mode !== "-") Capture.mode = mode;
            if (action === "close") root.capture = false;
            else if (action === "open") root.capture = true;
            else root.capture = !root.capture;
        }
        function settings(action: string): void {
            if (action === "close") root.settings = false;
            else if (action === "open") root.show("settings");
            else if (action === "toggle") root.settings = !root.settings;
            else { root.settingsPage = action; root.show("settings"); }
        }
        function keychain(action: string): void {
            if (action === "open") root.show("keychain");
            else if (action === "close") root.keychain = false;
            else root.keychain = !root.keychain;
        }
        function monitor(action: string): void {
            if (action === "open") root.show("monitor");
            else if (action === "close") root.monitor = false;
            else root.monitor = !root.monitor;
        }
        // "open", "close", "toggle", or a folder to open at.
        // "open", "close", "toggle", or a folder to open at. A window already there takes a tab.
        function files(action: string): void {
            if (action === "close") { FileWindows.closeAll(); root.files = false; }
            else if (action === "open" || !action || action === "-") root.showFiles("");
            else if (action === "toggle") { if (FileWindows.any) { FileWindows.closeAll(); root.files = false; } else root.showFiles(""); }
            else root.showFiles(action);
        }
        function power(action: string): void {
            if (action === "open") root.power = true;
            else if (action === "close") root.power = false;
            else root.power = !root.power;
        }
        function closeAll(): void { root.closeAll(); }
    }
}
