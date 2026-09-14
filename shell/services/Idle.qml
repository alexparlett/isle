pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Wayland

// Idle policy: dim, lock, screens off, at the preferences' timeouts. Game mode inhibits all three.
// Each monitor is created with its final timeout and recreated when the preference changes; a monitor whose
// timeout moves while it is live does not fire again.
Singleton {
    id: root

    readonly property bool inhibited: Modes.game
    property bool dimmed: false

    component Policy: Loader {
        id: policy
        property int seconds: 0
        property var onIdle
        active: false
        sourceComponent: IdleMonitor {
            enabled: true
            timeout: policy.seconds
            respectInhibitors: true
            onIsIdleChanged: policy.onIdle(isIdle)
        }
        function apply() { active = false; if (seconds > 0 && !root.inhibited) active = true; }
        onSecondsChanged: apply()
        Component.onCompleted: apply()
    }

    Policy { id: dim; seconds: Prefs.p.idleDim; onIdle: idle => { root.dimmed = idle; } }
    Policy { id: lock; seconds: Prefs.p.idleLock; onIdle: idle => { if (idle) Lock.lock(); } }
    // What the screen policy last decided, and telling the compositor so.
    property bool screensOff: false
    function driveScreens() { Compositor.dispatch("hl.dsp.dpms({ action = \"" + (screensOff ? "off" : "on") + "\" })"); }

    // A policy replaced — the timeout edited, game mode on or off — starts over from not-idle, so the screens
    // it left off are turned on rather than held off by a decision nothing will revisit.
    Policy {
        id: screen
        seconds: Prefs.p.idleScreenOff
        onIdle: idle => { root.screensOff = idle; root.driveScreens(); }
        onActiveChanged: { root.screensOff = false; root.driveScreens(); }
    }

    onInhibitedChanged: { dim.apply(); lock.apply(); screen.apply(); if (inhibited) dimmed = false; }
    // A shell that starts while the screens are off has no idle-to-active edge to turn them on again: it
    // asks for them at once, and the next idle turns them off as before.
    Component.onCompleted: driveScreens()
    // Neither has a screen that was away for that edge: a KVM's, turned off while it was gone and handed back
    // long after the input that woke the seat. A screen list that changes is told the policy's state again,
    // once the compositor has settled on it (D76).
    readonly property string screens: Quickshell.screens.map(s => s.name).join(",")
    onScreensChanged: settle.restart()
    Timer { id: settle; interval: 500; onTriggered: root.driveScreens() }
}
