import QtQuick
import qs.services
import qs.ui

SettingsPage {
    title: "Phone"
    subtitle: "A phone beside the desktop: its notifications and battery here, the clipboard and files both ways, a ring to find it."
    Component.onCompleted: Phones.refresh()
    ProviderGroups { service: Phones; heading: "Bridges"; none: "No phone bridge has a provider"; empty: "No phone yet; open the app on the phone, on this network" }
}
