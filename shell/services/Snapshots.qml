pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// The snapshot and rollback tools, every one with a provider whose tool is on this machine: Settings ›
// Snapshots lists them with a switch, each one's snapshots beneath. A provider is a manifest and a script
// (shell/snapshots for the shell's own, ~/.config/isle/snapshots/<name>/provider.json for the user's).
Singleton {
    id: root

    property var manifests: []
    Process {
        id: finder
        command: ["python3", Quickshell.shellDir + "/scripts/providers.py", Quickshell.shellDir, "snapshots"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { root.manifests = JSON.parse(text); } catch (e) { root.manifests = []; } } }
    }
    function rescan() { finder.running = true; }

    Instantiator {
        id: made
        model: root.manifests
        delegate: SnapshotProvider {
            required property var modelData
            providerId: modelData.id; name: modelData.name; note: modelData.note; script: modelData.script; user: modelData.user
            Component.onCompleted: refresh()
        }
        onObjectAdded: root.rebuild()
        onObjectRemoved: root.rebuild()
    }
    property var providers: []
    function rebuild() {
        const out = [];
        for (let i = 0; i < made.count; i++) { const o = made.objectAt(i); if (o) out.push({ id: o.providerId, name: o.name, note: o.note, user: o.user, impl: o }); }
        providers = out;
    }
    function provider(id) { return providers.find(p => p.id === id) || null; }

    readonly property var installed: providers.filter(p => p.impl.installed)
    readonly property var off: Prefs.p.snapshotProvidersOff || []
    function enabled(p) { return p.impl.installed && off.indexOf(p.id) < 0; }
    function setEnabled(id, on) { Prefs.p.snapshotProvidersOff = on ? off.filter(x => x !== id) : off.concat([id]); }
    readonly property var shown: providers.filter(p => enabled(p))
    function refresh() { for (const p of providers) p.impl.refresh(); }

    IpcHandler {
        target: "snapshots"
        function rescan(): void { root.rescan(); }
        function refresh(): void { root.refresh(); }
        function act(provider: string, id: string, input: string): void { const p = root.provider(provider); if (p) p.impl.act(id, input); }
        function status(): string { return JSON.stringify(root.shown.map(p => ({ id: p.id, ready: p.impl.ready, state: p.impl.state, actions: p.impl.actions.map(a => a.id), items: p.impl.items.length, error: p.impl.error }))); }
        function items(provider: string): string { const p = root.provider(provider); return JSON.stringify(p ? p.impl.items.map(i => ({ id: i.id, title: i.title, subtitle: i.subtitle })) : []); }
    }
}
