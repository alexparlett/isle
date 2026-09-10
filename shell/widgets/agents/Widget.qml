import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// Terminal windows: where the coding agents live. Click one to go to it.
WidgetBase {
    title: "Agents"
    meta: Windows.terminals.length > 0 ? Windows.terminals.length + " running" : ""

    ColumnLayout {
        anchors.fill: parent
        spacing: 2
        Repeater {
            model: Windows.terminals
            ListRow {
                required property var modelData
                Layout.fillWidth: true
                implicitHeight: 34
                glyph: "terminal"
                glyphColor: modelData.focused ? Theme.ok : Theme.text3
                title: modelData.title || modelData.appId
                onClicked: { Windows.focus(modelData); Surfaces.dashboard = false; }
                Label { text: modelData.workspace > 0 ? "" + modelData.workspace : ""; size: Theme.sizeCaption; color: Theme.text3; tabular: true }
            }
        }
        Item { Layout.fillHeight: true }
    }

    Label { anchors.centerIn: parent; visible: Windows.terminals.length === 0; text: "No terminals open"; color: Theme.text3 }
}
