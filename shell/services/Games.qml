pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

// Notices a game: gamemoded registering a client, or a fullscreen window from a game launcher. Enters game mode when allowed.
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

    // --- the library, for Big Picture -------------------------------------------------

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

    function launchSteam() {
        const mon = Hyprland.focusedMonitor;
        const size = mon ? " -W " + Math.round(mon.width / mon.scale) + " -H " + Math.round(mon.height / mon.scale) : "";
        Compositor.exec(Prefs.p.gamescope && tools.gamescope ? "gamescope -f -e" + size + " -- steam -gamepadui" : "steam -gamepadui");
    }
    function launch(tool) { Compositor.exec(tool); }
    function play(game) { if (game) Compositor.exec(game.cmd); }

    readonly property bool detected: gamemodeClients > 0 || fullscreenGame
    property bool enteredByUs: false

    onDetectedChanged: {
        if (!Prefs.p.gameAuto) return;
        if (detected && Modes.current === "normal") { enteredByUs = true; Modes.set("game"); }
        else if (!detected && enteredByUs && Modes.current === "game") { enteredByUs = false; Modes.set("normal"); }
    }
}
