pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// The cloud drives, one provider per service: Settings › Drives lists them with a switch, the accounts signed
// in to each beneath. A provider is a manifest and a script (shell/drives for the shell's own,
// ~/.config/isle/drives/<name>/provider.json for the user's), and every service here runs the same script
// told which of rclone's backends it is, so another one is a manifest rather than code.
Singleton {
    id: root

    property var manifests: []
    Process {
        id: finder
        command: ["python3", Quickshell.shellDir + "/scripts/providers.py", Quickshell.shellDir, "drives"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { root.manifests = JSON.parse(text); } catch (e) { root.manifests = []; } } }
    }
    function rescan() { finder.running = true; }

    Instantiator {
        id: made
        model: root.manifests
        delegate: ItemProvider {
            required property var modelData
            providerId: modelData.id; name: modelData.name; note: modelData.note; script: modelData.script; user: modelData.user
            args: modelData.args || []
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

    readonly property var off: Prefs.p.driveProvidersOff || []
    function enabled(p) { return p.impl.installed && off.indexOf(p.id) < 0; }
    function setEnabled(id, on) { Prefs.p.driveProvidersOff = on ? off.filter(x => x !== id) : off.concat([id]); }
    readonly property var shown: providers.filter(p => enabled(p))
    function refresh() { for (const p of providers) p.impl.refresh(); }
    function act(providerId, id, input) { const p = provider(providerId); if (p) p.impl.act(id, input); }

    // What can be added, each service with the values it needs: the picker in the sheet is this list, and a
    // manifest dropped in ~/.config/isle/drives adds to it without code.
    property var catalogue: []
    Process {
        id: fieldsProc
        stdout: StdioCollector { onStreamFinished: root.tookFields(text) }
        onExited: root.askNext()
    }
    property int asking: -1
    property var gathered: []
    function readCatalogue() { gathered = []; asking = -1; askNext(); }
    function askNext() {
        asking++;
        if (asking >= providers.length) { catalogue = gathered; return; }
        const p = providers[asking];
        fieldsProc.command = ["python3", p.impl.script].concat(p.impl.args || [], ["fields"]);
        fieldsProc.running = true;
    }
    function tookFields(text) {
        let f; try { f = JSON.parse(text); } catch (e) { return; }
        const p = providers[asking];
        if (p) gathered = gathered.concat([{ id: p.id, name: p.name, fields: f.fields || [], browser: !!f.browser }]);
    }
    onProvidersChanged: readCatalogue()

    // Adding one: the values the sheet collected, or a browser for a service that authenticates that way.
    function add(serviceId, name, values) {
        const p = provider(serviceId);
        if (!p) return;
        const entry = catalogue.find(c => c.id === serviceId);
        if (entry && entry.browser) p.impl.act("browser", name);
        else p.impl.act("add", JSON.stringify({ name: name, values: values || {} }));
    }

    // Every account across the services, and where each is mounted: what the file search keeps out of its way.
    readonly property var accounts: {
        let out = [];
        for (const p of shown) if (p.impl.ready) out = out.concat(p.impl.items.map(i => Object.assign({ provider: p.id }, i)));
        return out;
    }
    readonly property string root_: Quickshell.env("HOME") + "/Drives"

    IpcHandler {
        target: "drives"
        function rescan(): void { root.rescan(); }
        function refresh(): void { root.refresh(); }
        function act(provider: string, id: string, input: string): void { const p = root.provider(provider); if (p) p.impl.act(id, input); }
        function add(service: string, name: string): void { root.add(service, name, {}); }
        function services(): string { return JSON.stringify(root.catalogue); }
        function status(): string {
            return JSON.stringify(root.shown.map(p => ({ id: p.id, ready: p.impl.ready, state: p.impl.state,
                accounts: p.impl.items.map(i => ({ id: i.id, subtitle: i.subtitle })), error: p.impl.error })));
        }
    }
}
