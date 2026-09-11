pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import qs.services

// Games and Big Picture: notices a game (gamemoded, or a fullscreen launcher window), keeps the library,
// follows Steam's own Big Picture, and while a mode is on owns the pad: the home screen when it is in front,
// otherwise a bridge that turns the pad into keys for launchers that have none of their own.
Singleton {
    id: root

    property int gamemodeClients: 0

    // gamemoded announces registrations on the session bus.
    Process {
        command: ["gdbus", "monitor", "--session", "--dest", "com.feralinteractive.GameMode", "--object-path", "/com/feralinteractive/GameMode"]
        running: true
        stdout: SplitParser {
            onRead: line => {
                if (line.indexOf("GameRegistered") >= 0) root.gamemodeClients++;
                else if (line.indexOf("GameUnregistered") >= 0) root.gamemodeClients = Math.max(0, root.gamemodeClients - 1);
            }
        }
    }

    // A fullscreen window that looks like a game: Steam's per-app ids, or a Wine/Proton, Heroic or Lutris child.
    readonly property var gameClassPattern: /^(steam_app_\d+|steam_proton|.*\.exe|gamescope|heroic|net\.lutris\.Lutris|lutris)$/i
    readonly property bool fullscreenGame: {
        for (const t of Hyprland.toplevels.values) {
            if (!t.wayland || !t.wayland.fullscreen) continue;
            if (gameClassPattern.test(t.wayland.appId || "")) return true;
        }
        return false;
    }
    // Steam's gamepad UI, by its window: it is the console while it is up.
    readonly property bool steamBigPicture: {
        for (const t of Hyprland.toplevels.values) {
            const w = t.wayland; if (!w) continue;
            if ((w.appId || "").toLowerCase() === "steam" && /big picture/i.test(t.title || "")) return true;
        }
        return false;
    }

    // --- the library, for the home screen -------------------------------------------------

    property var library: []
    property var tools: ({ steam: false, heroic: false, lutris: false, gamescope: false })
    Process {
        id: scanner
        command: ["python3", Quickshell.shellDir + "/scripts/games.py"]
        stdout: StdioCollector {
            onStreamFinished: {
                try { const d = JSON.parse(text); root.library = d.games; root.tools = { steam: d.steam, heroic: d.heroic, lutris: d.lutris, gamescope: d.gamescope }; } catch (e) {}
            }
        }
    }
    function refresh() { scanner.running = true; }

    // Steam opens in its own Big Picture, into a running client or a fresh one; gamescope when asked.
    function openSteam() {
        if (Prefs.p.gamescope && tools.gamescope) {
            const mon = Hyprland.focusedMonitor;
            const size = mon ? " -W " + Math.round(mon.width / mon.scale) + " -H " + Math.round(mon.height / mon.scale) : "";
            Compositor.exec("gamescope -f -e" + size + " -- steam -gamepadui");
        } else Compositor.exec("steam steam://open/bigpicture");
    }
    function launchSteam() { openSteam(); }
    // Heroic's console mode and Lutris are windows without pad support of their own: the home steps back
    // behind them and the bridge below drives them.
    function openHeroic() { Compositor.exec("heroic --console"); homeAway = true; }
    function openLutris() { Compositor.exec("lutris"); homeAway = true; }
    function launch(tool) { if (tool === "heroic") openHeroic(); else if (tool === "lutris") openLutris(); else Compositor.exec(tool); }
    function play(game) { if (game) Compositor.exec(game.cmd); }

    // --- the mode ---------------------------------------------------------------------------

    readonly property bool detected: gamemodeClients > 0 || fullscreenGame
    property bool enteredByUs: false
    onDetectedChanged: {
        if (!Prefs.p.gameAuto || Modes.current === "bigpicture") return;
        if (detected && Modes.current === "normal") { enteredByUs = true; Modes.set("game"); }
        else if (!detected && enteredByUs && Modes.current === "game") { enteredByUs = false; Modes.set("normal"); }
    }
    // Steam entering its Big Picture brings the mode along, and leaving it takes the mode away again,
    // unless Isle was in Big Picture first.
    property bool steamDriven: false
    onSteamBigPictureChanged: {
        if (!steamBigPicture) steamQuit.restart();
        if (!Prefs.p.steamFollowsMode) return;
        if (steamBigPicture && Modes.current !== "bigpicture") { steamDriven = true; Modes.set("bigpicture"); }
        else if (!steamBigPicture && steamDriven) { steamDriven = false; if (Modes.current === "bigpicture") Modes.set("normal"); }
    }
    // Leaving Steam's Big Picture can quit Steam, when the desktop client it falls back to is not wanted.
    // A moment's grace, since the window's title can change while Steam is loading.
    readonly property bool steamDesktopWindow: {
        for (const t of Hyprland.toplevels.values) {
            const w = t.wayland; if (!w) continue;
            if ((w.appId || "").toLowerCase() === "steam" && (t.title || "") === "Steam") return true;
        }
        return false;
    }
    Timer { id: steamQuit; interval: 2000; onTriggered: if (Prefs.p.steamQuitsWithBigPicture && !root.steamBigPicture && root.steamDesktopWindow) Compositor.exec("steam -shutdown") }
    // Something else owns the screen and the pad: a game, or Steam's UI.
    readonly property bool inFront: detected || steamBigPicture
    // The home stepped back behind a launcher it opened; Guide held brings it forward.
    property bool homeAway: false
    readonly property bool homeShown: Modes.current === "bigpicture" && !inFront && !homeAway
    property bool quickOpen: false
    function home() { homeAway = false; quickOpen = false; if (Modes.current !== "bigpicture") Modes.set("bigpicture"); }
    function desktop() {
        if (steamBigPicture) Compositor.exec("steam steam://close/bigpicture");
        steamDriven = false; homeAway = false; quickOpen = false;
        Modes.set("normal");
    }
    onHomeShownChanged: if (homeShown) refresh()

    // --- the pad ----------------------------------------------------------------------------

    // Read while a mode is on, and on the desktop when a pad may open Big Picture.
    readonly property bool padWatch: Modes.game || Prefs.p.padHome
    onPadWatchChanged: Gamepad.listeners += padWatch ? 1 : -1
    Component.onCompleted: if (padWatch) Gamepad.listeners++
    // The bridge: a launcher without pad support in front gets the pad as keys, the way Steam Input does
    // for its games. Games themselves read the pad and are left alone.
    readonly property var bridgeKeys: ({ up: "Up", down: "Down", left: "Left", right: "Right", rup: "Up", rdown: "Down", rleft: "Left", rright: "Right",
                                         a: "Return", start: "Return", b: "Escape", x: "space", y: "Tab", lb: "Prior", rb: "Next", lt: "Home", rt: "End" })
    readonly property string frontClass: Hyprland.activeToplevel && Hyprland.activeToplevel.wayland ? (Hyprland.activeToplevel.wayland.appId || "") : ""
    readonly property bool bridged: Modes.game && !homeShown && !quickOpen && !Osk.open && !inFront && (Prefs.p.padBridgeApps || []).some(c => c.toLowerCase() === frontClass.toLowerCase())
    Connections {
        target: Gamepad
        function onPressed(b) {
            if (b === "guide-hold") { if (Modes.game || Prefs.p.padHome) root.home(); return; }
            if (b === "guide") { if (Modes.game && !root.steamBigPicture && !Osk.open) root.quickOpen = !root.quickOpen; return; }
            if (root.bridged && root.bridgeKeys[b]) Osk.run(["-k", root.bridgeKeys[b]]);
        }
    }

    // --- Isle inside Steam's Big Picture ------------------------------------------------------

    // Two non-Steam shortcuts, Isle settings and Desktop, so the quick menu and the way out are tiles in
    // Steam's UI; Steam hands them the pad as it does any game.
    property bool shortcutsInstalled: false
    property bool steamRunning: false
    property string shortcutsError: ""
    Process {
        id: shortcuts
        command: ["python3", Quickshell.shellDir + "/scripts/steamshortcuts.py", "status"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { const s = JSON.parse(text); root.shortcutsInstalled = !!s.installed; root.steamRunning = !!s.running; } catch (e) {} } }
    }
    Process {
        id: shortcutsChange
        stderr: StdioCollector { onStreamFinished: root.shortcutsError = text.trim() }
        onExited: shortcuts.running = true
    }
    function refreshShortcuts() { shortcuts.running = true; }
    function setShortcuts(on) { shortcutsError = ""; shortcutsChange.command = ["python3", Quickshell.shellDir + "/scripts/steamshortcuts.py", on ? "add" : "remove"]; shortcutsChange.running = true; }

    IpcHandler {
        target: "bigpicture"
        function home(): void { root.home(); }
        function desktop(): void { root.desktop(); }
        function quick(action: string): void { root.quickOpen = action === "open" ? true : action === "close" ? false : !root.quickOpen; }
        function quickOpen(): bool { return root.quickOpen; }
    }
}
