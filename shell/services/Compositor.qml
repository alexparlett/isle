pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Hyprland

// The compositor, behind one interface. Nothing else imports Quickshell.Hyprland.
Singleton {
    id: root

    readonly property var workspaces: Hyprland.workspaces
    readonly property var focusedWorkspace: Hyprland.focusedWorkspace
    readonly property var focusedMonitor: Hyprland.focusedMonitor
    readonly property var monitors: Hyprland.monitors.values

    // The monitor the shell's surfaces sit on: the primary, or the focused one when the shell follows the pointer.
    readonly property var shellMonitor: {
        if (Prefs.p.shellFollowsFocus) return focusedMonitor;
        return monitors.find(m => m.name === Displays.primary) || focusedMonitor;
    }
    readonly property var shellScreen: shellMonitor ? Quickshell.screens.find(s => s.name === shellMonitor.name) ?? null : null

    // Workspace ids on the focused monitor, ascending. Special workspaces are negative and excluded.
    readonly property var workspaceIds: {
        const out = [];
        for (const w of Hyprland.workspaces.values)
            if (w.id > 0 && (!focusedMonitor || w.monitor === focusedMonitor)) out.push(w.id);
        return out.sort((a, b) => a - b);
    }
    readonly property int focusedId: focusedWorkspace ? focusedWorkspace.id : -1
    // The same for one monitor by name: an island on each screen shows that screen's desktops.
    function workspaceIdsOn(name) {
        const out = [];
        for (const w of Hyprland.workspaces.values) if (w.id > 0 && w.monitor && w.monitor.name === name) out.push(w.id);
        return out.sort((a, b) => a - b);
    }
    function activeIdOn(name) { const m = monitors.find(m => m.name === name); return m && m.activeWorkspace ? m.activeWorkspace.id : -1; }

    // Dispatchers are Lua expressions since Hyprland 0.55.
    function focusWorkspace(id) { Hyprland.dispatch("hl.dsp.focus({ workspace = " + id + " })"); }
    function exec(cmd) { Hyprland.dispatch("hl.dsp.exec_cmd(" + JSON.stringify(cmd) + ")"); }
    function dispatch(lua) { Hyprland.dispatch(lua); }
}
