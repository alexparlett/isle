pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Removable drives through udisks2: what is plugged in, mounted where, and mount / unmount / eject.
// A newly seen filesystem morphs the island with mount and eject.
Singleton {
    id: root

    // [{ path, name, label, size, fstype, mountpoint, mounted, removable, drive }]
    property var volumes: []
    property var seen: ({})
    property bool primed: false

    Process {
        id: lsblk
        command: ["lsblk", "-J", "-b", "-o", "PATH,NAME,TYPE,FSTYPE,MOUNTPOINT,LABEL,SIZE,HOTPLUG,RM,PKNAME,MODEL,VENDOR"]
        stdout: StdioCollector { onStreamFinished: root.parse(text) }
    }
    function refresh() { lsblk.running = true; }

    function parse(text) {
        let devs;
        try { devs = JSON.parse(text).blockdevices; } catch (e) { return; }
        const out = [];
        const walk = (d, parent) => {
            const removable = !!(d.hotplug || d.rm || (parent && (parent.hotplug || parent.rm)) || d.type === "loop");
            if (d.fstype && removable && d.type !== "disk" || (d.fstype && removable && !d.children)) {
                out.push({ path: d.path, name: d.name, label: d.label || (parent && parent.model ? parent.model.trim() : d.name), size: Number(d.size) || 0,
                           fstype: d.fstype, mountpoint: d.mountpoint || "", mounted: !!d.mountpoint, removable: true, drive: parent ? parent.path : d.path });
            }
            for (const c of d.children || []) walk(c, d);
        };
        for (const d of devs) walk(d, null);
        volumes = out;

        // Announce filesystems that were not there before, once the first scan has primed the set.
        const now = {};
        for (const v of out) {
            now[v.path] = true;
            if (primed && !seen[v.path]) announce(v);
        }
        seen = now;
        primed = true;
    }

    // A new removable filesystem is mounted as it appears when the preference says so; the toast follows.
    function announce(v) {
        if (Prefs.p.autoMount && !v.mounted) mount(v);
        IslandEvents.show({ kind: "drive", duration: 8000, volume: v, text: v.label, detail: sizeLabel(v.size) + (v.mounted || Prefs.p.autoMount ? " · mounted" : "") });
    }

    // udisks announces every change; the listing follows.
    Process {
        command: ["udisksctl", "monitor"]
        running: true
        stdout: SplitParser { onRead: line => debounce.restart() }
    }
    Timer { id: debounce; interval: 400; onTriggered: root.refresh() }
    Component.onCompleted: refresh()

    Process {
        id: action
        stdout: StdioCollector { onStreamFinished: root.refresh() }
        stderr: StdioCollector { onStreamFinished: if (text.trim()) { console.warn("disks:", text.trim()); IslandEvents.show({ kind: "text", duration: 4000, glyph: "hard-drive", text: "Could not do that", detail: text.trim().split("\n").pop().slice(0, 80) }); } }
    }
    function run(args) { console.log("disks:", args.join(" ")); action.command = args; action.running = true; }
    function mount(v) { run(["udisksctl", "mount", "-b", v.path]); }
    function unmount(v) { run(["udisksctl", "unmount", "-b", v.path]); }
    // The event carries a snapshot; act on the volume as it is now.
    function current(v) { return volumes.find(x => x.path === v.path) || v; }
    function eject(v) {
        const c = current(v);
        run(["sh", "-c", (c.mounted ? "udisksctl unmount -b " + c.path + " && " : "") + "{ udisksctl power-off -b " + c.drive + " 2>/dev/null || udisksctl loop-delete -b " + c.drive + "; }"]);
        IslandEvents.dismissKind("drive");
    }
    function open(v) { Compositor.exec("xdg-open " + JSON.stringify(v.mountpoint)); }

    function sizeLabel(n) {
        if (n >= 1e12) return (n / 1e12).toFixed(1) + " TB";
        if (n >= 1e9) return Math.round(n / 1e9) + " GB";
        if (n >= 1e6) return Math.round(n / 1e6) + " MB";
        return Math.round(n / 1e3) + " kB";
    }
}
