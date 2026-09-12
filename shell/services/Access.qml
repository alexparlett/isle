pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// Accessibility: the magnifier is the compositor's cursor zoom, the screen scaled around the pointer; larger
// text and high contrast live in the theme, from the preferences.
Singleton {
    id: root

    // The magnifier's factor, 1 for off; steps of a quarter up to 4.
    property real zoom: 1
    Process { id: setter }
    function setZoom(z) {
        zoom = Math.max(1, Math.min(4, Math.round(z * 4) / 4));
        setter.command = ["hyprctl", "eval", "hl.config({ cursor = { zoom_factor = " + zoom + " } })"];
        setter.running = true;
    }
    function zoomIn() { setZoom(zoom + (zoom < 2 ? 0.25 : 0.5)); }
    function zoomOut() { setZoom(zoom - (zoom <= 2 ? 0.25 : 0.5)); }
    function zoomReset() { setZoom(1); }

    IpcHandler {
        target: "access"
        function zoom(action: string): void { if (action === "in") root.zoomIn(); else if (action === "out") root.zoomOut(); else root.zoomReset(); }
        function textScale(scale: real): void { Prefs.p.textScale = scale; }
        function contrast(on: bool): void { Prefs.p.highContrast = on; }
    }
}
