import QtQuick
import qs.services

// A snapshot tool behind Snapshots: a Provider whose list is the snapshots, each with the actions the tool
// allows on it (roll back, delete), run with the snapshot's id on stdin.
Provider {
    id: root
    listKey: "items"
    // [{ id, title, subtitle, actions: [{ id, label, danger }] }], newest first.
    readonly property var items: listed
}
