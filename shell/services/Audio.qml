pragma Singleton
import QtQuick
import Quickshell
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
