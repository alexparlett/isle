import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Snapshots"
    subtitle: "Snapshots of the system and rolling back to one."

    Component.onCompleted: Snapshots.refresh()

    SettingsGroup {
        heading: "Snapshot tools"
        Repeater {
            model: Snapshots.providers
            SettingsRow {
                id: row
                required property var modelData
                readonly property var impl: modelData.impl
                readonly property bool on: Snapshots.enabled(modelData)
                label: modelData.name
                description: !impl.installed ? "Not installed" : !on ? modelData.note : impl.state + (impl.said ? "  ·  " + impl.said : "") + (impl.error ? "  ·  " + impl.error : "")
                RowLayout {
                    spacing: Theme.s2
                    // The tool's own options and actions: a toggle, a choice, a field with its button, a button.
                    Repeater {
                        model: row.on ? row.impl.actions : []
                        RowLayout {
                            id: act
                            required property var modelData
                            property string input: ""
                            spacing: Theme.s1
                            Label { visible: modelData.on !== undefined || !!modelData.options; text: modelData.label; size: Theme.sizeCaption; color: Theme.text2 }
                            Toggle { visible: modelData.on !== undefined; checked: !!modelData.on; enabled: !row.impl.busy; onToggled: row.impl.act(modelData.id, "") }
                            Dropdown { visible: !!modelData.options; listWidth: 220; options: modelData.options || []; value: modelData.value || ""; enabled: !row.impl.busy; onPicked: v => row.impl.act(modelData.id, v) }
                            Field { visible: !!modelData.input; implicitWidth: 180; implicitHeight: 32; placeholder: modelData.input || ""; onTextChanged: act.input = text; onAccepted: { row.impl.act(modelData.id, text); text = ""; } }
                            Button { visible: modelData.on === undefined && !modelData.options; text: modelData.label; variant: modelData.primary ? "accent" : "text"; implicitHeight: 32; enabled: !row.impl.busy; onClicked: { row.impl.act(modelData.id, act.input); act.input = ""; } }
                        }
                    }
                    Toggle { enabled: row.impl.installed; checked: row.on; onToggled: v => Snapshots.setEnabled(row.modelData.id, v) }
                }
            }
        }
        SettingsRow { visible: Snapshots.providers.length === 0; label: "No snapshot tool has a provider" }
    }

    // Each tool's snapshots, newest first, with the actions it allows on one; a dangerous one asks twice.
    Repeater {
        model: Snapshots.shown.filter(p => p.impl.ready)
        SettingsGroup {
            id: group
            required property var modelData
            heading: modelData.name + (modelData.impl.items.length ? "  ·  " + modelData.impl.items.length : "")
            Repeater {
                model: group.modelData.impl.items
                SettingsRow {
                    id: snap
                    required property var modelData
                    property string armed: ""
                    label: modelData.title
                    description: modelData.subtitle || ""
                    RowLayout {
                        spacing: Theme.s2
                        Repeater {
                            model: snap.modelData.actions || []
                            Button {
                                required property var modelData
                                text: snap.armed === modelData.id ? modelData.label + "?" : modelData.label
                                variant: snap.armed === modelData.id ? "danger" : "text"
                                implicitHeight: 32
                                enabled: !group.modelData.impl.busy
                                onClicked: {
                                    if (modelData.danger && snap.armed !== modelData.id) { snap.armed = modelData.id; return; }
                                    snap.armed = "";
                                    group.modelData.impl.act(modelData.id, snap.modelData.id);
                                }
                            }
                        }
                    }
                }
            }
            SettingsRow { visible: group.modelData.impl.items.length === 0; label: "No snapshots yet" }
        }
    }
}
