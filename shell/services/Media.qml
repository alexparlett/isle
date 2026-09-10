pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.Mpris

// The player that matters: the one playing, else the last one that can be controlled.
Singleton {
    id: root

    readonly property var players: Mpris.players.values
    readonly property var active: players.find(p => p.playbackState === MprisPlaybackState.Playing)
        || players.find(p => p.canControl) || null

    readonly property bool present: active !== null
    readonly property bool playing: present && active.playbackState === MprisPlaybackState.Playing
    readonly property string title: present ? (active.trackTitle || "") : ""
    readonly property string artist: present ? (active.trackArtist || "") : ""
    readonly property string artUrl: present ? (active.trackArtUrl || "") : ""

    function togglePlaying() { if (present && active.canTogglePlaying) active.togglePlaying(); }
    function next() { if (present && active.canGoNext) active.next(); }
    function previous() { if (present && active.canGoPrevious) active.previous(); }

    // A new track, or playback starting, morphs the island.
    property string lastKey: ""
    readonly property string key: playing ? title + " " + artist : ""
    onKeyChanged: {
        if (key !== "" && key !== lastKey)
            IslandEvents.show({ kind: "media", duration: 4000, title: title, artist: artist, artUrl: artUrl });
        lastKey = key;
    }
}
