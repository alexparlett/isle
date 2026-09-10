import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Storage"
    subtitle: "Filesystems and the drives behind them. Health is what udisks reads from SMART."

    Component.onCompleted: { Storage.listeners++; Drives.listeners++; Disks.refresh(); }
    Component.onDestruction: { Storage.listeners--; Drives.listeners--; }

    SettingsGroup {
        heading: "Filesystems"
        Repeater {
            model: Storage.mounts
            SettingsRow {
                required property var modelData
                label: Storage.label(modelData)
                description: modelData.target + "  ·  " + modelData.fstype + "  ·  " + Disks.sizeLabel(modelData.used) + " of " + Disks.sizeLabel(modelData.size)
                RowLayout {
                    spacing: Theme.s2
                    Rectangle {
                        implicitWidth: 140; implicitHeight: 6; radius: 3; color: Theme.pressed
                        Rectangle { width: parent.width * modelData.pct; height: parent.height; radius: 3; color: modelData.pct > 0.9 ? Theme.warn : Theme.accent }
                    }
                    Label { text: Math.round(modelData.pct * 100) + "%"; mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 36; horizontalAlignment: Text.AlignRight }
                    Button { text: "Open"; variant: "text"; onClicked: Compositor.exec("xdg-open " + JSON.stringify(modelData.target)) }
                }
            }
        }
    }

    SettingsGroup {
        heading: "Removable"
        SettingsRow {
            label: "Mount automatically"
            description: "A drive plugged in is mounted and announced; off, the announcement offers a Mount button."
            Toggle { checked: Prefs.p.autoMount; onToggled: v => Prefs.p.autoMount = v }
        }
        Repeater {
            model: Disks.volumes
            SettingsRow {
                required property var modelData
                label: modelData.label
                description: modelData.path + "  ·  " + modelData.fstype + "  ·  " + Disks.sizeLabel(modelData.size) + (modelData.mounted ? "  ·  " + modelData.mountpoint : "")
                RowLayout {
                    spacing: Theme.s2
                    Button { text: modelData.mounted ? "Open" : "Mount"; variant: "raised"; onClicked: modelData.mounted ? Disks.open(modelData) : Disks.mount(modelData) }
                    Button { text: "Unmount"; variant: "text"; visible: modelData.mounted; onClicked: Disks.unmount(modelData) }
                    Button { text: "Eject"; variant: "text"; onClicked: Disks.eject(modelData) }
                }
            }
        }
    }

    SettingsGroup {
        heading: "Drives"
        Repeater {
            model: Drives.drives
            SettingsRow {
                required property var modelData
                readonly property bool failing: modelData.smart && modelData.smart.failing
                label: modelData.model
                description: Drives.kind(modelData) + (modelData.size ? "  ·  " + Disks.sizeLabel(modelData.size) : "") + "  ·  " + modelData.devices.join(", ")
                           + (modelData.serial ? "  ·  " + modelData.serial : "") + (Drives.detail(modelData) ? "\n" + Drives.detail(modelData) : "")
                RowLayout {
                    spacing: Theme.s2
                    Rectangle { width: 8; height: 8; radius: 4; color: !modelData.smart ? Theme.text3 : failing ? Theme.warn : Theme.ok }
                    Label { text: Drives.health(modelData); size: Theme.sizeSmall; weight: Font.DemiBold; color: failing ? Theme.warn : Theme.text2 }
                    Button { text: "Refresh"; variant: "text"; visible: !!modelData.smart; onClicked: Drives.smartUpdate(modelData) }
                    Button { text: "Disks"; variant: "text"; visible: Drives.hasDisksApp; onClicked: Drives.openDisks(modelData) }
                }
            }
        }
        SettingsRow { visible: Drives.drives.length === 0; label: "No drives reported"; description: "udisks2 is not answering." }
        SettingsRow { visible: !Drives.hasDisksApp; label: "Partitioning and formatting"; description: "Install gnome-disk-utility for a Disks button here." }
    }
}
