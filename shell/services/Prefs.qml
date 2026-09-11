pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Preferences: ~/.config/isle/prefs.json, written on every change and reloaded when edited by hand.
Singleton {
    id: root

    readonly property string dir: (Quickshell.env("XDG_CONFIG_HOME") || Quickshell.env("HOME") + "/.config") + "/isle"
    readonly property alias p: adapter

    Process { command: ["mkdir", "-p", root.dir]; running: true }

    IpcHandler {
        target: "prefs"
        function set(key: string, value: string): void {
            if (adapter[key] === undefined) return;
            const cur = adapter[key];
            adapter[key] = typeof cur === "boolean" ? value === "true" : typeof cur === "number" ? Number(value) : value;
        }
        // Appends to a list preference, once.
        function add(key: string, value: string): void {
            const cur = adapter[key];
            if (!Array.isArray(cur) || cur.indexOf(value) >= 0) return;
            adapter[key] = cur.concat([value]);
        }
    }

    // True once the file has been read (or found missing and written fresh); nothing that writes on the
    // strength of a value should run before then, since until then the adapter holds defaults.
    property bool loaded: false
    FileView {
        id: file
        path: root.dir + "/prefs.json"
        watchChanges: true
        onFileChanged: reload()
        onAdapterUpdated: if (root.loaded) writeAdapter()
        onLoaded: root.loaded = true
        onLoadFailed: error => { if (error === FileViewError.FileNotFound) { root.loaded = true; writeAdapter(); } }

        adapter: JsonAdapter {
            id: adapter
            property bool dnd: false
            // Quiet hours (from hour to hour, wrapping midnight), auto-silence under a fullscreen window, apps
            // that toast through, a summary when silence lifts, and a sound per toast.
            property bool dndQuiet: false
            property int dndQuietFrom: 22
            property int dndQuietTo: 7
            property bool dndFullscreen: true
            property var dndAllow: []
            property bool dndSummary: true
            property bool notifySound: false
            property bool reducedMotion: false
            property string wallpaper: ""
            property var wallpaperFolders: []
            property bool autoMount: true
            property bool nightLight: false
            property int nightLightTemperature: 4000
            property var dashboard: []
            // [{ name, layout }] and which is shown; empty means one page holding `dashboard`.
            property var dashboardPages: []
            property int dashboardPage: 0
            property int idleDim: 300
            property int idleLock: 600
            property int idleScreenOff: 900
            property var keyboardProfiles: ({})
            property string accent: ""
            property bool gameAuto: true
            property bool gameTearing: false
            property bool gamescope: false
            // Steam opens in its Big Picture from the launcher: one window, no X11 menus.
            property bool steamBigPicture: false
            // Steam's Big Picture appearing puts Isle in Big Picture mode.
            property bool steamFollowsMode: true
            // A pad on the desktop: Guide held opens Big Picture.
            property bool padHome: true
            // Window classes the pad is bridged to as keys while a mode is on.
            property var padBridgeApps: ["heroic", "net.lutris.Lutris", "lutris", "spotify"]
            property var launchCounts: ({})
            property var mutedApps: []
            property var knownApps: []
            property int captureDelay: 0
            property bool captureCursor: false
            property bool captureAudio: false
            property string theme: "dark"
            // Monitor name the shell's surfaces sit on; empty follows focus.
            property string primaryMonitor: ""
            property bool shellFollowsFocus: false
            property var monitors: ({})
            property bool monitorGrouped: true
            // Where each app's window last sat, by class: [x, y, w, h].
            property var windowPlaces: ({})
            property var input: ({})
            property var widgetSettings: ({})
            // The row count the saved layouts were made for; a change is migrated once.
            property int dashboardGrid: 0
            // Instance key -> what the widget kept through host.store.
            property var widgetState: ({})
            // Widget id -> the permissions the user allows it, of those it declares; absent means all declared.
            property var widgetGrants: ({})
            // Registry git URLs beyond the default one derived from the shell's origin.
            property var widgetSources: []
            // Ids of installed widgets the user has granted the shell's full reach.
            property var widgetTrust: []
            property var keymapOverrides: ({})
            property var clipPins: []
            property string terminal: ""
            property bool clock12: false
            property bool titleBars: true
        }
    }
}
