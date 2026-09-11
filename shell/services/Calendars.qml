pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// Subscribed calendars, read-only: ICS by URL or file, fetched on a schedule and expanded to events around
// now by scripts/calendars.py. The widget draws them, the island says when one is about to start.
Singleton {
    id: root

    // [{ id, name, url, color }], from Settings › Calendars.
    readonly property var subscriptions: Prefs.p.calendars || []
    // Named colours a subscription can take; the hex is what the widget draws.
    readonly property var palette: [["accent", "Accent"], ["#5cc46b", "Green"], ["#e8a33d", "Amber"], ["#e05a5a", "Red"], ["#a373e6", "Purple"], ["#3bbfc0", "Teal"], ["#e26fa0", "Pink"]]

    // [{ cal, uid, title, start, end, allDay, location }], start ascending; times are local, ISO without a zone.
    property var events: []
    // [{ id, ok, error, fetched, count }], one per subscription, in its order.
    property var status: []
    property bool busy: false
    // A refresh asked for while one runs is run after it: the subscriptions arrive after the first run starts.
    property bool pending: false
    property double refreshed: 0

    function sub(id) { return subscriptions.find(s => s.id === id) || null; }
    function colorOf(id) { const s = sub(id); return !s || !s.color || s.color === "accent" ? "" : s.color; }
    // The events of one calendar day, keyed as the widget keys days: local yyyy-mm-dd.
    readonly property var byDay: {
        const m = {};
        for (const e of events) {
            // An all-day or multi-day event marks every day it covers, the end being exclusive.
            const s = new Date(e.start), end = new Date(e.end);
            const d = new Date(s.getFullYear(), s.getMonth(), s.getDate());
            do {
                const k = key(d);
                if (!m[k]) m[k] = [];
                m[k].push(e);
                d.setDate(d.getDate() + 1);
            } while (d < end);
        }
        return m;
    }
    function key(d) { return d.getFullYear() + "-" + String(d.getMonth() + 1).padStart(2, "0") + "-" + String(d.getDate()).padStart(2, "0"); }
    function on(d) { return byDay[key(d)] || []; }

    // The next timed event that has not started, and whether it is within the half hour.
    SystemClock { id: clock; precision: SystemClock.Minutes }
    readonly property var next: {
        const now = clock.date;
        for (const e of events) { if (!e.allDay && new Date(e.start) > now) return e; }
        return null;
    }
    readonly property var soon: next && new Date(next.start) - clock.date <= 30 * 60 * 1000 ? next : null
    // At the minute an event starts, the island says so, once.
    property string announced: ""
    onSoonChanged: if (!soon) announced = "";
    Connections {
        target: clock
        function onDateChanged() {
            const now = clock.date;
            for (const e of root.events) {
                if (e.allDay) continue;
                const s = new Date(e.start);
                if (s > now || now - s >= 60 * 1000) continue;
                if (root.announced === e.uid + e.start) continue;
                root.announced = e.uid + e.start;
                IslandEvents.show({ kind: "text", glyph: "calendar", text: e.title + (e.location ? "  ·  " + e.location : ""), duration: 8000 });
                break;
            }
        }
    }

    Process {
        id: fetcher
        stdout: StdioCollector {
            onStreamFinished: {
                root.busy = false;
                let r; try { r = JSON.parse(text); } catch (e) { r = null; }
                if (r) { root.events = r.events || []; root.status = r.calendars || []; root.refreshed = Date.now(); }
                if (root.pending) { root.pending = false; root.refresh(false); }
            }
        }
    }
    function refresh(cachedOnly) {
        if (fetcher.running) { pending = true; return; }
        busy = true;
        fetcher.command = ["python3", Quickshell.shellDir + "/scripts/calendars.py", JSON.stringify(subscriptions)].concat(cachedOnly ? ["--cached"] : []);
        fetcher.running = true;
    }
    onSubscriptionsChanged: refresh(false)
    Component.onCompleted: refresh(false)
    Timer { interval: 30 * 60 * 1000; running: root.subscriptions.length > 0; repeat: true; onTriggered: root.refresh(false) }

    function add(name, url) {
        const id = "c" + Date.now().toString(36);
        const used = subscriptions.map(s => s.color);
        const color = (palette.find(p => used.indexOf(p[0]) < 0) || palette[0])[0];
        Prefs.p.calendars = subscriptions.concat([{ id: id, name: name || url.replace(/^\w+:\/\//, "").split("/")[0], url: url, color: color }]);
    }
    function remove(id) { Prefs.p.calendars = subscriptions.filter(s => s.id !== id); }
    function update(id, patch) { Prefs.p.calendars = subscriptions.map(s => s.id === id ? Object.assign({}, s, patch) : s); }
    function statusOf(id) { return status.find(s => s.id === id) || null; }
}
