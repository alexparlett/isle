pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Services.Pipewire

// Default sink and source with their volume and mute state; every sink and source; the app streams; who is capturing.
Singleton {
    id: root

    readonly property var sink: Pipewire.defaultAudioSink
    readonly property var source: Pipewire.defaultAudioSource

    readonly property var nodes: Pipewire.nodes.values
    readonly property var sinks: nodes.filter(n => n.type === PwNodeType.AudioSink)
    readonly property var sources: nodes.filter(n => n.type === PwNodeType.AudioSource && n.name.indexOf(".monitor") < 0)
    // Apps playing sound, and apps capturing it.
    readonly property var streams: nodes.filter(n => n.type === PwNodeType.AudioOutStream)
    readonly property var captures: nodes.filter(n => n.type === PwNodeType.AudioInStream)
    readonly property var videoCaptures: nodes.filter(n => n.type === PwNodeType.VideoSource && n.isStream)
    readonly property bool micInUse: captures.length > 0
    readonly property bool cameraInUse: videoCaptures.length > 0
    // A portal screencast is a video stream the portal itself produces, xdph-streaming-N from
    // xdg-desktop-portal-hyprland: another app is seeing the screen. Quickshell's node list leaves video
    // streams out, so these come from pw-dump, read again whenever pw-mon reports a change.
    property var screencasts: []
    property var watchers: []
    readonly property bool screenShared: screencasts.length > 0
    Process { command: ["pw-mon", "-N"]; running: true; stdout: SplitParser { onRead: line => { if (/media\.class|removed|added/.test(line)) castDebounce.restart(); } } }
    Timer { id: castDebounce; interval: 400; onTriggered: if (!castDump.running) castDump.running = true }
    Timer { interval: 15000; running: true; repeat: true; onTriggered: if (!castDump.running) castDump.running = true }
    Process {
        id: castDump
        command: ["sh", "-c", "pw-dump 2>/dev/null | python3 -c \"import json,sys\nout=[]\nfor n in json.load(sys.stdin):\n p=(n.get('info') or {}).get('props') or {}\n c=p.get('media.class','')\n if c in ('Stream/Output/Video','Stream/Input/Video'): out.append({'id':n['id'],'cls':c,'name':p.get('node.name') or '','app':p.get('application.name') or ''})\nprint(json.dumps(out))\""]
        stdout: StdioCollector {
            onStreamFinished: {
                let list; try { list = JSON.parse(text); } catch (e) { return; }
                root.screencasts = list.filter(n => n.cls === "Stream/Output/Video" && /xdph|xdg-desktop-portal|screencast|screen-cast/i.test(n.name + " " + n.app));
                root.watchers = list.filter(n => n.cls === "Stream/Input/Video").map(n => n.app || n.name).filter((v, i, a) => v && a.indexOf(v) === i);
            }
        }
    }
    Component.onCompleted: castDump.running = true
    // Ending a share is destroying the portal's stream: the app sees its capture end.
    Process { id: stopper }
    function stopScreencast() {
        if (!screencasts.length) return;
        stopper.command = ["sh", "-c", "for i in \"$@\"; do pw-cli destroy \"$i\"; done", "_"].concat(screencasts.map(n => String(n.id)));
        stopper.running = true;
    }
    IpcHandler {
        target: "privacy"
        function status(): string { return JSON.stringify({ mic: root.micInUse, camera: root.cameraInUse, screen: root.screenShared, watchers: root.watchers, casts: root.screencasts.map(n => n.name) }); }
        function stopSharing(): void { root.stopScreencast(); }
    }
    onScreenSharedChanged: {
        if (screenShared) IslandEvents.show({ kind: "text", duration: 24 * 3600 * 1000, glyph: "screen-share", color: "", text: "Sharing the screen", detail: watchers.length ? "with " + watchers.join(", ") : "", actions: [{ label: "Stop", run: () => root.stopScreencast() }] });
        else IslandEvents.dismissKind("text");
    }

    PwObjectTracker { objects: [root.sink, root.source].concat(root.streams, root.sources).filter(o => o) }

    // A node's properties are only live once the tracker has bound it: PwNode.ready, not PwNode.audio.
    readonly property bool ready: sink !== null && sink.ready
    readonly property real volume: ready && sink.audio ? sink.audio.volume : 0
    readonly property bool muted: ready && sink.audio ? sink.audio.muted : false
    readonly property bool sourceReady: source !== null && source.ready
    readonly property real sourceVolume: sourceReady && source.audio ? source.audio.volume : 0
    readonly property bool sourceMuted: sourceReady && source.audio ? source.audio.muted : false

    function setSink(node) { Pipewire.preferredDefaultAudioSink = node; }
    function setSource(node) { Pipewire.preferredDefaultAudioSource = node; }
    function setVolume(v) { if (ready && sink.audio) sink.audio.volume = Math.max(0, Math.min(1, v)); }
    function setMuted(m) { if (ready && sink.audio) sink.audio.muted = m; }
    function setSourceVolume(v) { if (sourceReady && source.audio) source.audio.volume = Math.max(0, Math.min(1, v)); }
    function setSourceMuted(m) { if (sourceReady && source.audio) source.audio.muted = m; }

    // What to call a stream: the app's name, then the media title.
    function streamName(n) { return n.properties["application.name"] || n.nickname || n.description || n.name; }
    function streamDetail(n) { return n.properties["media.name"] || ""; }
    // A game's stream carries the game's name: its cover from the library, before the icon theme is asked.
    function streamIcon(n) {
        const name = n.properties["application.name"] || "";
        const game = name ? Games.library.find(g => g.name.toLowerCase() === name.toLowerCase()) : null;
        if (game && game.art) return game.art.indexOf("/") === 0 ? "file://" + game.art : game.art;
        const iconName = n.properties["application.icon-name"] || "";
        if (iconName) return Quickshell.iconPath(iconName, "");
        const entry = name ? DesktopEntries.heuristicLookup(name) : null;
        return entry && entry.icon ? Quickshell.iconPath(entry.icon, "") : name ? Quickshell.iconPath(name, "") : "";
    }
    function setStreamVolume(n, v) { if (n.audio) n.audio.volume = Math.max(0, Math.min(1, v)); }
    function setStreamMuted(n, m) { if (n.audio) n.audio.muted = m; }
    // One row per app: a game opens several streams under one name and one process, and they are one
    // volume to a person. `node` is the first stream, read for the value; a change goes to every stream.
    readonly property var apps: {
        const out = [], byKey = {};
        for (const n of streams) {
            const key = n.properties["application.process.id"] || streamName(n);
            if (byKey[key]) { byKey[key].nodes.push(n); continue; }
            byKey[key] = { key: key, node: n, nodes: [n], name: streamName(n), icon: streamIcon(n) };
            out.push(byKey[key]);
        }
        for (const a of out) a.detail = a.nodes.length > 1 ? a.nodes.length + " streams" : streamDetail(a.node);
        return out;
    }
    function setAppVolume(a, v) { for (const n of a.nodes) setStreamVolume(n, v); }
    function setAppMuted(a, m) { for (const n of a.nodes) setStreamMuted(n, m); }

    // Changes after the first moments are the user's, and show as the OSD.
    property bool settled: false
    Timer { interval: 1500; running: root.ready; onTriggered: root.settled = true }
    onVolumeChanged: osd()
    onMutedChanged: osd()
    function osd() {
        if (!settled) return;
        IslandEvents.show({ kind: "osd", duration: 1500, glyph: muted ? "volume-x" : "volume-2", value: muted ? 0 : volume,
                            label: muted ? "Muted" : Math.round(volume * 100).toString() });
    }
}
