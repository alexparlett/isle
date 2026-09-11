pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// The VPNs, every one with a provider whose tool is on this machine: one face for the panel, the launcher
// and the island, the providers behind it. A provider is a manifest and a script (shell/vpns for the
// shell's own, ~/.config/isle/vpns/<name>/provider.json for the user's) that says its state, what is up,
// where it can connect and what it offers.
Singleton {
    id: root

    property var manifests: []
    Process {
        id: finder
        command: ["python3", Quickshell.shellDir + "/scripts/providers.py", Quickshell.shellDir, "vpns"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { root.manifests = JSON.parse(text); } catch (e) { root.manifests = []; } } }
    }
    function rescan() { finder.running = true; }

    Instantiator {
        id: made
        model: root.manifests
        delegate: VpnProvider {
            required property var modelData
            providerId: modelData.id; name: modelData.name; note: modelData.note; script: modelData.script; user: modelData.user
            Component.onCompleted: refresh()
        }
        onObjectAdded: root.rebuild()
        onObjectRemoved: root.rebuild()
    }
    // [{ id, name, note, user, impl }], one per manifest.
    property var providers: []
    function rebuild() {
        const out = [];
        for (let i = 0; i < made.count; i++) { const o = made.objectAt(i); if (o) out.push({ id: o.providerId, name: o.name, note: o.note, user: o.user, impl: o }); }
        providers = out;
    }
    function provider(id) { return providers.find(p => p.id === id) || null; }

    readonly property var installed: providers.filter(p => p.impl.installed)
    readonly property var off: Prefs.p.vpnProvidersOff || []
    function enabled(p) { return p.impl.installed && off.indexOf(p.id) < 0; }
    function setEnabled(id, on) { Prefs.p.vpnProvidersOff = on ? off.filter(x => x !== id) : off.concat([id]); }
    // The providers in use, ready or not: the panel's VPN page lists each.
    readonly property var shown: providers.filter(p => enabled(p))
    readonly property var ready: shown.filter(p => p.impl.ready)
    readonly property bool available: shown.length > 0
    readonly property bool busy: shown.some(p => p.impl.busy)
    readonly property string error: { for (const p of shown) if (p.impl.error) return p.name + ": " + p.impl.error; return ""; }

    // The connection that is up, with its provider: { name, detail, provider }, else null.
    readonly property var active: {
        for (const p of shown) if (p.impl.active) return Object.assign({ provider: p.id }, p.impl.active);
        return null;
    }
    function refresh() { for (const p of providers) p.impl.refresh(); }
    function connect(providerId, choice) { const p = provider(providerId); if (p) p.impl.connect(choice); }
    function disconnect() { const p = active ? provider(active.provider) : null; if (p) p.impl.disconnect(); }
    // Off if anything is up; else the first ready provider's default.
    function toggle() { if (active) disconnect(); else if (ready.length) ready[0].impl.connect(""); }
    // The link table changes when a tunnel comes or goes, whoever brought it; NetworkManager announces its own.
    Process { command: ["ip", "-o", "monitor", "link"]; running: true; stdout: SplitParser { onRead: line => debounce.restart() } }
    Process { command: ["nmcli", "monitor"]; running: true; stdout: SplitParser { onRead: line => debounce.restart() } }
    Timer { id: debounce; interval: 1200; onTriggered: root.refresh() }

    IpcHandler {
        target: "vpn"
        function toggle(): void { root.toggle(); }
        function rescan(): void { root.rescan(); }
        function refresh(): void { root.refresh(); }
        function connect(provider: string, choice: string): void { root.connect(provider, choice); }
        function disconnect(): void { root.disconnect(); }
        function act(provider: string, id: string, input: string): void { const p = root.provider(provider); if (p) p.impl.act(id, input); }
        function status(): string { return JSON.stringify(root.shown.map(p => ({ id: p.id, name: p.name, ready: p.impl.ready, state: p.impl.state, active: p.impl.active, actions: p.impl.actions.map(a => a.id), choices: p.impl.choices.length }))); }
    }
}
