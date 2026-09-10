pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.theme

// One state: normal, game, bigpicture. Entering applies a profile; leaving restores the last one.
Singleton {
    id: root

    property string current: "normal"
    readonly property bool game: current === "game" || current === "bigpicture"

    // Game: notifications that arrived while it was on.
    property int suppressed: 0
    property bool nightLightWas: false
    property string profileWas: "balanced"

    function set(mode) {
        if (mode === current) return;
        const was = current;
        current = mode;
        Theme.motion = game || Prefs.p.reducedMotion ? 0 : 1;


        const wasGame = was === "game" || was === "bigpicture";
        if (game && !wasGame) enterGame();
        else if (!game && wasGame) leaveGame();

        IslandEvents.show({ kind: "text", duration: 2000, glyph: mode === "normal" ? "check" : "gamepad-2",
                            text: mode === "normal" ? "Back to normal" : mode === "game" ? "Game mode" : "Big Picture" });
    }
    function toggle(mode) { set(current === mode ? "normal" : mode); }

    // --- game profile -------------------------------------------------------------

    Process { id: hypr }
    function evalLua(lua) { hypr.command = ["hyprctl", "eval", lua]; hypr.running = true; }

    function enterGame() {
        suppressed = 0;
        nightLightWas = NightLight.on;
        profileWas = Power.profile;
        if (NightLight.on) NightLight.setOn(false);
        if (Power.hasPerformance) Power.setProfile("performance");
        evalLua("hl.config({ animations = { enabled = false }, decoration = { blur = { enabled = false }, shadow = { enabled = false } }, " +
             "general = { gaps_in = 0, gaps_out = 0, border_size = 0, allow_tearing = " + (Prefs.p.gameTearing ? "true" : "false") + " }, misc = { vrr = 1 } })");
    }
    function leaveGame() {
        // The config file is the source of the normal profile: a reload restores everything at once.
        hypr.command = ["hyprctl", "reload"]; hypr.running = true;
        if (nightLightWas) NightLight.setOn(true);
        Power.setProfile(profileWas);
        if (suppressed > 0) IslandEvents.show({ kind: "text", duration: 4000, glyph: "bell", text: suppressed + (suppressed === 1 ? " notification" : " notifications") + " while you played" });
        suppressed = 0;
    }

    IpcHandler {
        target: "modes"
        function toggle(mode: string): void { root.toggle(mode); }
        function set(mode: string): void { root.set(mode); }
    }
}
