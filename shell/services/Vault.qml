pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// The password managers, every one with a provider whose tool is on this machine: one face for the launcher
// and the Keychain, the providers behind it, their items in one list. A provider is a manifest and a script
// (shell/vaults for the shell's own, ~/.config/isle/vaults/<name>/provider.json for the user's) that lists
// titles only and fetches one field of one item when asked.
Singleton {
    id: root

    // [{ id, name, note, script, signIn, user }], as scripts/vaults.py finds them.
    property var manifests: []
    Process {
        id: finder
        command: ["python3", Quickshell.shellDir + "/scripts/vaults.py", Quickshell.shellDir]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { root.manifests = JSON.parse(text); } catch (e) { root.manifests = []; } } }
    }
    function rescan() { finder.running = true; }
    IpcHandler {
        target: "vault"
        function rescan(): void { root.rescan(); }
        function refresh(): void { root.refresh(); }
        // Titles only, as JSON; and one field of one item to the clipboard, for a script of the user's.
        function items(): string { return JSON.stringify(root.items.map(i => ({ provider: i.provider, id: i.id, shareId: i.shareId, vault: i.vault, title: i.title, type: i.type }))); }
        function copy(provider: string, id: string, field: string): void { const it = root.items.find(i => i.provider === provider && i.id === id); if (it) root.copy(it, field); }
    }

    // One VaultProvider per manifest; `providers` mirrors them as [{ id, name, note, user, impl }].
    Instantiator {
        id: made
        model: root.manifests
        delegate: VaultProvider {
            required property var modelData
            providerId: modelData.id; name: modelData.name; note: modelData.note; script: modelData.script; signInCommand: modelData.signIn; user: modelData.user
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

    readonly property var installed: providers.filter(p => p.impl.installed)
    // A detected manager can be switched off in Settings › Passwords; it stays out of the launcher and the Keychain.
    readonly property var off: Prefs.p.passwordProvidersOff || []
    function enabled(p) { return p.impl.installed && off.indexOf(p.id) < 0; }
    function setEnabled(id, on) { Prefs.p.passwordProvidersOff = on ? off.filter(x => x !== id) : off.concat([id]); }
    readonly property var ready: providers.filter(p => enabled(p) && p.impl.ready)
    // Any provider's tool is on this machine at all: the Keychain's chips and the launcher's prefix are worth showing.
    readonly property bool any: installed.length > 0
    readonly property bool busy: providers.some(p => p.impl.busy)
    function provider(id) { return providers.find(p => p.id === id) || null; }

    // [{ provider, providerName, id, shareId, vault, title, type }], by title across the ready providers; types are
    // login, note, credit_card, identity, ssh_key, wifi, alias, custom.
    readonly property var items: {
        let out = [];
        for (const p of providers) if (enabled(p) && p.impl.ready) out = out.concat(p.impl.items.map(i => Object.assign({ provider: p.id, providerName: p.name }, i)));
        return out.sort((a, b) => a.title.localeCompare(b.title));
    }
    function revealedOf(item) { const p = provider(item.provider); return p && p.impl.revealedId === item.id ? p.impl.revealed : ""; }

    function refresh() { for (const p of providers) p.impl.refresh(); }
    function copy(item, field) { const p = provider(item.provider); if (p) p.impl.copy(item, field); }
    function reveal(item, field) { const p = provider(item.provider); if (p) p.impl.reveal(item, field); }
    function conceal() { for (const p of providers) p.impl.conceal(); }
}
