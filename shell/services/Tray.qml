pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.SystemTray

// Status notifier items, shown only in the dashboard's Session widget.
Singleton {
    id: root
    readonly property var items: SystemTray.items.values
    function activate(item) { item.activate(); }
    function secondary(item) { item.secondaryActivate(); }
}
