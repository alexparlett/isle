import QtQuick
import Quickshell.Io
import qs.theme
import qs.services

// The wrapper a sandboxed widget sees instead of the services. A permission is a service name
// ("audio") for read access, with ".write" ("audio.write") for the guarded actions. A service the
// manifest did not ask for is null; a write called without ".write" is a no-op. Sandboxed widgets are
// loaded by file:// so they cannot import the real singletons; this is their only reach.
QtObject {
    id: host
    // The manifest's `permissions`, verbatim.
    property var permissions: []
    // The instance whose state `store` keeps ("clock#2"), and the name a notification is posted under.
    property string instance: ""
    property string appName: "Widget"
    function has(p) { return permissions.indexOf(p) >= 0; }
    // A service is readable when named at all; a ".write" scope grants its actions and reading with it.
    function reads(svc) { return permissions.some(p => p === svc || p.indexOf(svc + ".") === 0); }
    function canWrite(svc) { return permissions.indexOf(svc + ".write") >= 0; }

    // fetch(url, done): a GET the shell performs, only to a host named by a "fetch:<host>" permission
    // (a subdomain of it counts). done(text, ok) is called once; a refused URL answers ("", false).
    readonly property var fetchHosts: permissions.filter(p => p.indexOf("fetch:") === 0).map(p => p.slice(6).toLowerCase())
    function mayFetch(url) {
        const m = String(url).match(/^https?:\/\/([^\/:?#]+)/);
        if (!m) return false;
        const h = m[1].toLowerCase();
        return fetchHosts.some(a => h === a || h.endsWith("." + a));
    }
    function fetch(url, done) {
        if (!mayFetch(url)) { console.warn("widget", appName, "may not fetch", url); if (done) done("", false); return; }
        fetcher.createObject(host, { url: String(url), done: done || null });
    }
    property Component fetcher: Component {
        Process {
            id: req
            property string url
            property var done
            property string body: ""
            command: ["curl", "-sL", "--max-time", "20", "--max-filesize", "4000000", "-A", "isle-widget", "--", url]
            running: true
            stdout: StdioCollector { onStreamFinished: req.body = text }
            onExited: code => { if (req.done) req.done(req.body, code === 0); req.destroy(); }
        }
    }

    // store: the widget's own state, kept in prefs per instance; no permission needed, it is its own.
    readonly property QtObject store: QtObject {
        function all() { return (Prefs.p.widgetState || {})[host.instance] || {}; }
        function get(key, fallback) { const v = all()[key]; return v === undefined ? fallback : v; }
        function set(key, value) {
            const st = Object.assign({}, Prefs.p.widgetState || {});
            st[host.instance] = Object.assign({}, st[host.instance] || {}, { [key]: value });
            Prefs.p.widgetState = st;
        }
        function remove(key) {
            const st = Object.assign({}, Prefs.p.widgetState || {}), mine = Object.assign({}, st[host.instance] || {});
            delete mine[key]; st[host.instance] = mine; Prefs.p.widgetState = st;
        }
    }

    // notify(summary, body): a desktop notification under the widget's name, with the "notify" permission.
    function notify(summary, body) {
        if (!has("notify")) return;
        notifier.createObject(host, { args: ["notify-send", "-a", appName, String(summary || appName), String(body || "")] });
    }
    property Component notifier: Component {
        Process { property var args; command: args; running: true; onExited: destroy() }
    }

    // Read-only styling, so a sandboxed widget can match the shell without qs.theme.
    readonly property QtObject theme: QtObject {
        readonly property color text: Theme.text
        readonly property color text2: Theme.text2
        readonly property color text3: Theme.text3
        readonly property color accent: Theme.accent
        readonly property color ok: Theme.ok
        readonly property color warn: Theme.warn
        readonly property color danger: Theme.danger
        readonly property color surface: Theme.raised
        readonly property color hairline: Theme.hairline
        readonly property int radius: Theme.radiusCard
        readonly property string fontUi: Theme.fontUi
        readonly property string fontMono: Theme.fontMono
    }

    readonly property var audio: reads("audio") ? audioObj : null
    property QtObject audioObj: QtObject {
        readonly property real volume: Audio.volume
        readonly property bool muted: Audio.muted
        readonly property string device: Audio.ready && Audio.sink ? (Audio.sink.description || Audio.sink.name) : ""
        function setVolume(v) { if (host.canWrite("audio")) Audio.setVolume(v); }
        function setMuted(m) { if (host.canWrite("audio")) Audio.setMuted(m); }
    }

    readonly property var network: reads("network") ? networkObj : null
    property QtObject networkObj: QtObject {
        readonly property string summary: Network.summary
        readonly property bool online: Network.online
        readonly property bool wifi: Network.wifiConnected
        readonly property bool wired: Network.wiredConnected
    }

    readonly property var media: reads("media") ? mediaObj : null
    property QtObject mediaObj: QtObject {
        readonly property bool playing: Media.playing
        readonly property bool present: Media.present
        readonly property string title: Media.title
        readonly property string artist: Media.artist
        function playPause() { if (host.canWrite("media")) Media.togglePlaying(); }
        function next() { if (host.canWrite("media")) Media.next(); }
        function previous() { if (host.canWrite("media")) Media.previous(); }
    }

    readonly property var system: reads("system") ? systemObj : null
    property QtObject systemObj: QtObject {
        readonly property real cpu: System.cpu
        readonly property real gpu: System.gpu
        readonly property real memoryUsed: System.memUsed
        readonly property real memoryTotal: System.memTotal
        readonly property real netDown: System.netDown
        readonly property real netUp: System.netUp
    }

    readonly property var notifications: reads("notifications") ? notificationsObj : null
    property QtObject notificationsObj: QtObject {
        readonly property int count: Notifications.count
        readonly property bool dnd: Notifications.dnd
        function setDnd(on) { if (host.canWrite("notifications")) Notifications.setDnd(on); }
    }

    readonly property var power: reads("power") ? powerObj : null
    property QtObject powerObj: QtObject {
        readonly property bool onBattery: Power.onBattery
        readonly property real percentage: Power.laptop ? Power.percent(Power.laptop) : -1
        readonly property string profile: Power.profileLabel
    }

    readonly property var bluetooth: reads("bluetooth") ? bluetoothObj : null
    property QtObject bluetoothObj: QtObject {
        readonly property bool enabled: Bluetooth.enabled
        readonly property int connected: Bluetooth.connected.length
    }

    readonly property var vpn: reads("vpn") ? vpnObj : null
    property QtObject vpnObj: QtObject {
        readonly property bool active: Vpn.active !== null
        readonly property string name: Vpn.active ? Vpn.active.name : ""
    }
}
