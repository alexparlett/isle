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
    Policy { id: screen; seconds: Prefs.p.idleScreenOff; onIdle: idle => Compositor.dispatch(idle ? "hl.dsp.dpms({ action = \"off\" })" : "hl.dsp.dpms({ action = \"on\" })") }

    onInhibitedChanged: { dim.apply(); lock.apply(); screen.apply(); if (inhibited) dimmed = false; }
    // A shell that starts while the screens are off has no idle-to-active edge to turn them on again: it
    // asks for them at once, and the next idle turns them off as before.
    Component.onCompleted: Compositor.dispatch("hl.dsp.dpms({ action = \"on\" })")
}
