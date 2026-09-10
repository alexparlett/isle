pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Which on-demand surfaces are open. Keybindings reach these through the "surfaces" IPC target.
Singleton {
    id: root

    property bool dashboard: false
    property bool launcher: false
    property string launcherPrefix: ""
    property bool overview: false
    property bool capture: false
    property bool power: false
    property bool settings: false
    property bool monitor: false
    property bool keychain: false
    property string settingsPage: "network"
    // Where Settings has been, for its back and forward buttons; a page set from anywhere lands on the trail.
    property var settingsHistory: ["network"]
    property int settingsIndex: 0
    property bool settingsNavigating: false
    readonly property bool settingsCanBack: settingsIndex > 0
    readonly property bool settingsCanForward: settingsIndex < settingsHistory.length - 1
    onSettingsPageChanged: {
        if (settingsNavigating) return;
        const h = settingsHistory.slice(0, settingsIndex + 1);
        if (h[h.length - 1] === settingsPage) return;
        h.push(settingsPage);
        settingsHistory = h.slice(-50);
        settingsIndex = settingsHistory.length - 1;
    }
    function settingsBack() { if (!settingsCanBack) return; settingsNavigating = true; settingsIndex--; settingsPage = settingsHistory[settingsIndex]; settingsNavigating = false; }
    // Opening a window surface that is already open brings its window to the front instead.
    readonly property var windowTitles: ({ settings: "Settings", keychain: "Keychain", monitor: "Monitor" })
    function show(name) {
        if (root[name]) Windows.focusShellWindow(windowTitles[name]);
        else root[name] = true;
    }
    function showSettings(page) { settingsPage = page; show("settings"); }
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
        function power(action: string): void {
            if (action === "open") root.power = true;
            else if (action === "close") root.power = false;
            else root.power = !root.power;
        }
        function closeAll(): void { root.closeAll(); }
    }
}
