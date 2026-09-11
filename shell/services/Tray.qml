pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.SystemTray
import qs.services

// Status notifier items, shown only in the dashboard's Session widget; any can be put away (Ctrl-click, or
// the Apps page), and comes back from the Apps page.
Singleton {
    id: root
    readonly property var all: SystemTray.items.values
    readonly property var hidden: Prefs.p.trayHidden || []
    readonly property var items: all.filter(i => hidden.indexOf(i.id) < 0)
    function setHidden(id, on) { const l = hidden.filter(x => x !== id); Prefs.p.trayHidden = on ? l.concat([id]) : l; }
    function activate(item) { item.activate(); }
    function secondary(item) { item.secondaryActivate(); }
}
