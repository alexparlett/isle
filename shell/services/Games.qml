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
    readonly property bool steamBigPicture: steamToplevel() !== null
    // Steam's Big Picture window, or gamescope's when Steam runs inside it. Steam minimises the window rather
    // than closing it when Big Picture is left, and a minimised one sits on the hidden workspace: not up.
    function hidden(t) { return t.workspace && t.workspace.name === "special:hidden"; }
    function steamToplevel() {
        for (const t of Hyprland.toplevels.values) {
            const w = t.wayland; if (!w || hidden(t)) continue;
            if (/^(steam|gamescope)$/i.test(w.appId || "") && /big picture/i.test(t.title || "")) return t;
        }
        return null;
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
        if (steamBigPicture) parkSteam(); else unparkSteam();
        if (!Prefs.p.steamFollowsMode) return;
        if (steamBigPicture && Modes.current !== "bigpicture") { steamDriven = true; Modes.set("bigpicture"); }
        else if (!steamBigPicture && steamDriven) { steamDriven = false; if (Modes.current === "bigpicture") Modes.set("normal"); }
    }
    // Big Picture is a window of its own: the desktop client's closes as it opens and a new one opens as it
    // closes, on whatever workspace is focused then. Big Picture goes fullscreen on a workspace of its own,
    // and when it closes the focus returns to where the client was, so the new client window opens there;
    // one that opens on the steam workspace all the same is moved.
    property string parkedAddress: ""
    property int parkedFrom: -1
    function parkSteam() {
        const t = steamToplevel(); if (!t || parkedAddress) return;
        parkedAddress = t.address.indexOf("0x") === 0 ? t.address : "0x" + t.address;
        // The title flips more than once while Big Picture opens, so the window may already sit on the
        // steam workspace: the origin is kept from the first park then.
        const ws = t.workspace && t.workspace.name !== "steam" ? t.workspace : null;
        if (ws) parkedFrom = ws.id;
        else if (parkedFrom <= 0) { const f = Hyprland.focusedWorkspace; parkedFrom = f && f.name !== "steam" ? f.id : 1; }
        const win = ', window = "address:' + parkedAddress + '"';
        Hyprland.dispatch('hl.dsp.window.move({ workspace = "name:steam"' + win + ' })');
        Hyprland.dispatch('hl.dsp.focus({' + win.slice(2) + ' })');
        Hyprland.dispatch('hl.dsp.window.fullscreen_state({ internal = 2, client = 2' + win + ' })');
    }
    function unparkSteam() {
        if (!parkedAddress) return;
        const addr = parkedAddress, from = parkedFrom > 0 ? parkedFrom : 1; parkedAddress = "";
        const t = Hyprland.toplevels.values.find(t => (t.address.indexOf("0x") === 0 ? t.address : "0x" + t.address) === addr);
        // A minimised window stays where it is: moving it would bring it back.
        if (t && !hidden(t)) {
            const win = ', window = "address:' + addr + '"';
            Hyprland.dispatch('hl.dsp.window.fullscreen_state({ internal = 0, client = 0' + win + ' })');
            Hyprland.dispatch('hl.dsp.window.move({ workspace = ' + from + win + ' })');
        }
        Hyprland.dispatch('hl.dsp.focus({ workspace = ' + from + ' })');
        // The desktop client comes back on whatever workspace was focused, which is the steam one when Big
        // Picture closes first: it goes where the client was.
        for (const t of Hyprland.toplevels.values) {
            const w = t.wayland; if (!w || !t.workspace || t.workspace.name !== "steam" || !(t.title || "")) continue;
            if (!/^(steam|gamescope)$/i.test(w.appId || "")) continue;
            const a = t.address.indexOf("0x") === 0 ? t.address : "0x" + t.address;
            if (a !== addr) Hyprland.dispatch('hl.dsp.window.move({ workspace = ' + from + ', window = "address:' + a + '" })');
        }
    }
    Connections {
        target: Hyprland
        function onRawEvent(event) {
            if (event.name !== "openwindow" || root.steamBigPicture) return;
            const [addr, ws, cls, title] = event.data.split(",");
            if (ws !== "steam" || !title || !/^(steam|gamescope)$/i.test(cls)) return;
            Hyprland.dispatch('hl.dsp.window.move({ workspace = ' + (root.parkedFrom > 0 ? root.parkedFrom : 1) + ', window = "address:0x' + addr + '" })');
        }
    }
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
