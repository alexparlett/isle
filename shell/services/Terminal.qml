pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland

// The dropdown terminal: one kitty on the "terminal" special workspace, sized across the top, toggled from the keymap.
Singleton {
    id: root

    readonly property string appId: "isle-dropdown"
    readonly property bool exists: Hyprland.toplevels.values.some(t => t.wayland && t.wayland.appId === appId)
    property bool wanted: false

    function toggle() {
        if (exists) { Hyprland.dispatch("hl.dsp.workspace.toggle_special(\"terminal\")"); return; }
        wanted = true;
        Hyprland.dispatch("hl.dsp.exec_cmd(\"kitty --class " + appId + "\")");
    }

    // Once the window is up: 70% wide, half the screen tall, centred near the top, then shown.
    onExistsChanged: if (exists && wanted) { wanted = false; place.restart(); }
    Timer {
        id: place
        interval: 250
        onTriggered: {
            const m = Hyprland.focusedMonitor;
            const w = m ? Math.round(m.width / m.scale) : 1920, h = m ? Math.round(m.height / m.scale) : 1080;
            const sel = ", window = \"class:" + root.appId + "\"";
            Hyprland.dispatch("hl.dsp.window.resize({ x = " + Math.round(w * 0.7) + ", y = " + Math.round(h * 0.5) + sel + " })");
            Hyprland.dispatch("hl.dsp.window.move({ x = " + Math.round(w * 0.15) + ", y = 60" + sel + " })");
            Hyprland.dispatch("hl.dsp.workspace.toggle_special(\"terminal\")");
        }
    }

    IpcHandler {
        target: "terminal"
        function dropdown(): void { root.toggle(); }
    }
}
