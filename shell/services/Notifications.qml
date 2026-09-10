pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.Notifications

// The notification server. Every notification is kept until dismissed; new ones toast in the island unless
// do not disturb is on, and critical ones toast regardless.
Singleton {
    id: root

    readonly property bool dnd: Prefs.p.dnd
    function setDnd(on) { Prefs.p.dnd = on; }
    function isMuted(app) { return Prefs.p.mutedApps.indexOf(app) >= 0; }
    function setMuted(app, on) {
        const m = Prefs.p.mutedApps.filter(a => a !== app);
        Prefs.p.mutedApps = on ? m.concat([app]) : m;
    }

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
            const t = root.times; t[n.id] = Date.now(); root.times = t;
            const critical = n.urgency === NotificationUrgency.Critical;
            if (n.appName && Prefs.p.knownApps.indexOf(n.appName) < 0) Prefs.p.knownApps = Prefs.p.knownApps.concat([n.appName]).sort();
            if (root.isMuted(n.appName) && !critical) return;
            if (Modes.game && !critical) { Modes.suppressed++; return; }
            if (root.dnd && !critical) return;
            if (!root.dnd || critical) root.toast(n);
        }
    }

    readonly property var list: server.trackedNotifications.values.slice().reverse()
    readonly property int count: server.trackedNotifications.values.length

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

    function toast(n) {
        IslandEvents.show({
            kind: "notification",
            duration: n.urgency === NotificationUrgency.Critical ? 8000 : 5000,
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
    function clearAll() { for (const n of server.trackedNotifications.values.slice()) n.dismiss(); }

    function age(n) {
        const s = Math.max(0, (Date.now() - (times[n.id] || Date.now())) / 1000);
        if (s < 60) return "now";
        if (s < 3600) return Math.floor(s / 60) + "m";
        if (s < 86400) return Math.floor(s / 3600) + "h";
        return Math.floor(s / 86400) + "d";
    }
}
