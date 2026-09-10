pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Physical drives through udisks: model, size, bus, and the SMART summary udisks keeps.
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/scripts/drives.py"
    // [{ path, model, serial, size, removable, ejectable, media, bus, rotation, smart, devices }]
    property var drives: []
    property int listeners: 0
    property bool hasDisksApp: false

    Process { command: ["sh", "-c", "command -v gnome-disks >/dev/null && echo yes"]; running: true; stdout: StdioCollector { onStreamFinished: root.hasDisksApp = text.trim() === "yes" } }

    Timer { interval: 30000; running: root.listeners > 0; repeat: true; triggeredOnStart: true; onTriggered: root.refresh() }

    Process {
        id: lister
        command: ["python3", root.script]
        stdout: StdioCollector { onStreamFinished: { try { root.drives = JSON.parse(text); } catch (e) {} } }
    }
    function refresh() { lister.running = true; }

    Process { id: updater; onExited: root.refresh() }
    function smartUpdate(d) { updater.command = ["python3", script, "smart", d.path]; updater.running = true; }
    function openDisks(d) { Compositor.exec("gnome-disks" + (d && d.devices.length ? " --block-device " + d.devices[0] : "")); }

    function health(d) {
        if (!d.smart) return d.media ? "No SMART data" : "No media";
        if (d.smart.failing) return "Failing";
        return "Healthy";
    }
    function detail(d) {
        const s = d.smart;
        if (!s) return "";
        const parts = [];
        if (s.temp !== null && s.temp !== undefined) parts.push(s.temp + " °C");
        if (s.hours) parts.push(s.hours >= 24 * 30 ? Math.round(s.hours / 24) + " days on" : s.hours + " h on");
        if (s.kind === "nvme" && s.used !== undefined) parts.push(s.used + "% worn");
        if (s.kind === "ata" && s.badSectors) parts.push(s.badSectors + " bad sectors");
        return parts.join("  ·  ");
    }
    function kind(d) {
        if (d.bus === "usb") return "USB";
        if (d.rotation === 0 && d.path.indexOf("nvme") < 0 && d.ejectable) return "Optical";
        if (d.path.toLowerCase().indexOf("nvme") >= 0 || d.smart && d.smart.kind === "nvme") return "NVMe";
        if (d.rotation > 0) return "HDD";
        if (d.rotation === 0) return "SSD";
        return "Disk";
    }
}
