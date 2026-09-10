pragma Singleton
import QtQuick
import Quickshell

// What the island is showing instead of rest, and the queue behind it.
// An event is { kind, duration, ...payload }. Same-kind events replace each other.
Singleton {
    id: root

    property var current: null
    property var queue: []

    Timer {
        id: timer
        onTriggered: root.next()
    }

    function show(ev) {
        if (ev.duration === undefined) ev.duration = 2500;
        if (current && current.kind === ev.kind) {
            current = ev;
            timer.interval = ev.duration;
            timer.restart();
        } else if (current) {
            queue = queue.filter(q => q.kind !== ev.kind).concat([ev]);
        } else {
            current = ev;
            timer.interval = ev.duration;
            timer.start();
        }
    }

    function next() {
        timer.stop();
        if (queue.length) {
            const q = queue.slice();
            current = q.shift();
            queue = q;
            timer.interval = current.duration;
            timer.start();
        } else {
            current = null;
        }
    }

    // Stop the clock on the current event, for as long as the user is interacting with it.
    function hold() { timer.stop(); }
    function resume() { if (current && !timer.running) { timer.interval = current.duration; timer.start(); } }
    function dismiss() { next(); }
    function dismissKind(kind) {
        queue = queue.filter(q => q.kind !== kind);
        if (current && current.kind === kind) next();
    }
}
