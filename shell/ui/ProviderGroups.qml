import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The provider pages' shape: the providers found, each with its switch and its own options and actions, then
// each ready provider's items with the actions the provider allows on one; a dangerous one asks twice.
ColumnLayout {
    id: root
    required property var service
    property string heading: "Providers"
    property string none: "Nothing has a provider"
    property string empty: "Nothing yet"
    spacing: Theme.s4

    SettingsGroup {
        heading: root.heading
        Repeater {
            model: root.service.providers
            SettingsRow {
                id: row
                required property var modelData
                readonly property var impl: modelData.impl
                readonly property bool on: root.service.enabled(modelData)
                label: modelData.name
                description: !impl.installed ? "Not installed.  " + modelData.note : !on ? modelData.note : impl.state + (impl.said ? "  ·  " + impl.said : "") + (impl.error ? "  ·  " + impl.error : "")
                RowLayout {
                    spacing: Theme.s2
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
                    Toggle { enabled: row.impl.installed; checked: row.on; onToggled: v => root.service.setEnabled(row.modelData.id, v) }
                }
            }
        }
        SettingsRow { visible: root.service.providers.length === 0; label: root.none }
    }

    Repeater {
        model: root.service.shown.filter(p => p.impl.ready)
        SettingsGroup {
            id: group
            required property var modelData
            heading: modelData.name + (modelData.impl.items.length ? "  ·  " + modelData.impl.items.length : "")
            Repeater {
                model: group.modelData.impl.items
                SettingsRow {
                    id: item
                    required property var modelData
                    property string armed: ""
                    label: modelData.title
                    description: modelData.subtitle || ""
                    RowLayout {
                        spacing: Theme.s2
                        Repeater {
                            model: item.modelData.actions || []
                            RowLayout {
                                id: iact
                                required property var modelData
                                property string input: ""
                                spacing: Theme.s1
                                Field { visible: !!modelData.input; implicitWidth: 200; implicitHeight: 32; placeholder: modelData.input || ""; onTextChanged: iact.input = text
                                        onAccepted: { group.modelData.impl.act(modelData.id, item.modelData.id + "\\n" + text); text = ""; } }
                                Button {
                                    text: item.armed === modelData.id ? modelData.label + "?" : modelData.label
                                    variant: item.armed === modelData.id ? "danger" : modelData.primary ? "accent" : "text"
                                    implicitHeight: 32
                                    enabled: !group.modelData.impl.busy && (!modelData.input || iact.input !== "")
                                    onClicked: {
                                        if (modelData.danger && item.armed !== modelData.id) { item.armed = modelData.id; return; }
                                        item.armed = "";
                                        group.modelData.impl.act(modelData.id, modelData.input ? item.modelData.id + "\\n" + iact.input : item.modelData.id);
                                        iact.input = "";
                                    }
                                }
                            }
                        }
                    }
                }
            }
            SettingsRow { visible: group.modelData.impl.items.length === 0; label: root.empty }
        }
    }
}
