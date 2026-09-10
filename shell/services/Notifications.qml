pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Hyprland
import Quickshell.Services.Notifications

// The notification server. Every notification is kept until dismissed and logged to a history file that
// survives a restart. A new one toasts in the island unless something silences it: do not disturb, quiet
// hours, a fullscreen window, game mode, or the app being muted. Critical ones and allowed apps toast
// regardless; low-urgency ones never toast. What was silenced is counted and summarised on the way back.
Singleton {
    id: root

    readonly property bool dnd: Prefs.p.dnd
    function setDnd(on) { Prefs.p.dnd = on; }
    function isMuted(app) { return Prefs.p.mutedApps.indexOf(app) >= 0; }
    function setMuted(app, on) {
        const m = Prefs.p.mutedApps.filter(a => a !== app);
        Prefs.p.mutedApps = on ? m.concat([app]) : m;
    }
    // Apps that toast through do not disturb and quiet hours.
    function isAllowed(app) { return (Prefs.p.dndAllow || []).indexOf(app) >= 0; }
    function setAllowed(app, on) {
        const a = (Prefs.p.dndAllow || []).filter(x => x !== app);
        Prefs.p.dndAllow = on ? a.concat([app]) : a;
    }

    // --- what silences a toast -----------------------------------------------------------------
    // Quiet hours: from hour `dndQuietFrom` to hour `dndQuietTo`, wrapping past midnight.
    readonly property bool quietHours: {
        if (!Prefs.p.dndQuiet) return false;
        const h = clock.date.getHours(), from = Prefs.p.dndQuietFrom, to = Prefs.p.dndQuietTo;
        return from <= to ? (h >= from && h < to) : (h >= from || h < to);
    }
    SystemClock { id: clock; precision: SystemClock.Minutes }
    // A fullscreen window has focus: the compositor says so as it happens.
    property bool fullscreen: false
    Connections {
        target: Hyprland
        function onRawEvent(event) { if (event.name === "fullscreen") root.fullscreen = event.data === "1"; }
    }
    readonly property bool silenced: dnd || quietHours || (Prefs.p.dndFullscreen && fullscreen) || Modes.game
    readonly property string silencedBy: dnd ? "Do not disturb" : quietHours ? "Quiet hours" : Modes.game ? "Game mode" : fullscreen ? "Fullscreen" : ""
    // Toasts held back while silenced; summarised once the silence lifts.
    property int missed: 0
    onSilencedChanged: {
        if (silenced || missed === 0) return;
        const n = missed;
        missed = 0;
        if (Prefs.p.dndSummary && !Modes.game) IslandEvents.show({ kind: "text", glyph: "bell", duration: 6000,
            text: n === 1 ? "1 notification while you were away" : n + " notifications while you were away",
            actions: [{ label: "Show", run: () => root.openCentre() }] });
    }
    signal centreRequested
    function openCentre() { centreRequested(); }

    // Arrival times by notification id, for "12m" in lists.
    property var times: ({})

    NotificationServer {
        id: server
        keepOnReload: true
        imageSupported: true
        actionsSupported: true
        actionIconsSupported: false
        bodySupported: true
        bodyMarkupSupported: true
        persistenceSupported: true
        inlineReplySupported: true

        onNotification: n => {
            n.tracked = true;
            const t = root.times;
            t[n.id] = Date.now();
            root.times = t;
            root.remember(n);
            const critical = n.urgency === NotificationUrgency.Critical;
            const low = n.urgency === NotificationUrgency.Low;
            if (n.appName && Prefs.p.knownApps.indexOf(n.appName) < 0) Prefs.p.knownApps = Prefs.p.knownApps.concat([n.appName]).sort();
            if (low && !critical) return;
            if (root.isMuted(n.appName) && !critical) return;
            if (Modes.game && !critical) { Modes.suppressed++; return; }
            if (root.silenced && !critical && !root.isAllowed(n.appName)) { root.missed++; return; }
            root.toast(n);
            if (Prefs.p.notifySound) sound.running = true;
        }
    }
    Process {
        id: sound
        command: ["sh", "-c", "f=/usr/share/sounds/freedesktop/stereo/message.oga; [ -f \"$f\" ] || exit 0; command -v pw-play >/dev/null && exec pw-play \"$f\"; command -v paplay >/dev/null && exec paplay \"$f\""]
    }

    readonly property var list: server.trackedNotifications.values.slice().reverse()
    readonly property int count: server.trackedNotifications.values.length

    // --- history, across restarts ------------------------------------------------------------------
    // Every arrival is logged as a plain record; the live objects go with the shell, the records stay.
    // [{ id, app, summary, body, icon, time, urgency }], newest first, at most 300.
    readonly property string historyFile: Prefs.dir + "/notifications.json"
    property var history: []
    Process {
        command: ["sh", "-c", "cat \"$1\" 2>/dev/null || echo '[]'", "_", root.historyFile]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { root.history = JSON.parse(text); } catch (e) { root.history = []; } } }
    }
    Process { id: historyWriter }
    Timer {
        id: historySave
        interval: 800
        onTriggered: { historyWriter.command = ["sh", "-c", "printf '%s' \"$2\" > \"$1\"", "_", root.historyFile, JSON.stringify(root.history)]; historyWriter.running = true; }
    }
    function remember(n) {
        const rec = { id: n.id + ":" + Date.now(), app: n.appName, summary: n.summary, body: plain(n.body), icon: iconFor(n), time: Date.now(), urgency: n.urgency };
        history = [rec].concat(history).slice(0, 300);
        historySave.restart();
    }
    // History that is not among the live notifications: from before this shell started, or since dismissed.
    readonly property var past: {
        const live = {};
        for (const n of list) live[n.summary + " " + (times[n.id] || 0)] = true;
        return history.filter(r => !live[r.summary + " " + r.time]);
    }
    function clearHistory() { history = []; historySave.restart(); }

    function appIconFor(n) {
        if (n.appIcon) return Quickshell.iconPath(n.appIcon, "");
        const entry = n.desktopEntry ? DesktopEntries.byId(n.desktopEntry) : DesktopEntries.heuristicLookup(n.appName);
        return entry && entry.icon ? Quickshell.iconPath(entry.icon, "") : "";
    }
    function iconFor(n) { return appIconFor(n) || n.image || ""; }
    // The image hint is a picture when the app has an icon of its own; otherwise it stands in for the icon.
    function pictureFor(n) { return n.image && appIconFor(n) ? n.image : ""; }

    function plain(markup) {
        return (markup || "").replace(/<[^>]*>/g, "").replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&").replace(/\s+/g, " ").trim();
    }

    // Its default action, if it has one, and its app brought forward; the toast goes.
    function open(n) {
        if (!n) return;
        const d = n.actions.find(a => a.identifier === "default");
        if (d) d.invoke();
        Windows.focusApp(n.desktopEntry || n.appName);
        IslandEvents.dismissKind("notification");
    }

    // The toast stays as long as the app asked, within reason; critical ones longer by default.
    function toast(n) {
        const asked = n.expireTimeout > 0 ? Math.max(3000, Math.min(15000, n.expireTimeout)) : 0;
        IslandEvents.show({
            kind: "notification",
            duration: asked || (n.urgency === NotificationUrgency.Critical ? 8000 : 5000),
            height: pictureFor(n) ? 130 : 52,
            notification: n,
            app: n.appName,
            icon: iconFor(n),
            picture: pictureFor(n),
            summary: n.summary,
            body: plain(n.body),
            critical: n.urgency === NotificationUrgency.Critical,
        });
    }

    // The default action, else the first one; then the notification is done.
    function activate(n) {
        const a = n.actions.find(a => a.identifier === "default") || n.actions[0];
        if (a) a.invoke();
        n.dismiss();
    }

    function dismiss(n) { n.dismiss(); }
    function clearAll() { for (const n of server.trackedNotifications.values.slice()) n.dismiss(); clearHistory(); }

    function ageOf(ms) {
        const s = Math.max(0, (Date.now() - ms) / 1000);
        if (s < 60) return "now";
        if (s < 3600) return Math.floor(s / 60) + "m";
        if (s < 86400) return Math.floor(s / 3600) + "h";
        return Math.floor(s / 86400) + "d";
    }
    function age(n) { return ageOf(times[n.id] || Date.now()); }
}
