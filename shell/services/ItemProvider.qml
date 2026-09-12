import QtQuick
import qs.services

// A Provider whose list is items with the actions the tool allows on each: a snapshot tool's snapshots, a
// phone bridge's phones. An item's action runs with the item's id on stdin.
Provider {
    id: root
    listKey: "items"
    // [{ id, title, subtitle, actions: [{ id, label, danger, input }] }].
    readonly property var items: listed
}
