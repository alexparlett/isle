import QtQuick
import qs.services
import qs.ui

SettingsPage {
    title: "Snapshots"
    subtitle: "Snapshots of the system and rolling back to one."
    Component.onCompleted: Snapshots.refresh()
    ProviderGroups { service: Snapshots; heading: "Snapshot tools"; none: "No snapshot tool has a provider"; empty: "No snapshots yet" }
}
