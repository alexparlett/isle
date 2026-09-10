import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// A centred dialog: what is asking, what for, a password field, cancel or authenticate.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Auth.active

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-auth"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    onVisibleChanged: if (visible) field.input.forceActiveFocus()

    Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.4) }

    Glass {
        anchors.centerIn: parent
        width: 400
        height: col.implicitHeight + Theme.s5 * 2
        radius: Theme.radiusPanel

        ColumnLayout {
            id: col
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s5 }
            spacing: Theme.s4

            RowLayout {
                spacing: Theme.s3
                Item {
                    implicitWidth: 40; implicitHeight: 40
                    id: authIcon
                    readonly property string icon: Auth.flow && Auth.flow.iconName ? Quickshell.iconPath(Auth.flow.iconName, "") : ""
                    // An action's icon the theme lacks resolves to a placeholder that fails to load: the key stands in.
                    readonly property bool shown: icon !== "" && appIcon.status === Image.Ready
                    AppIcon { id: appIcon; anchors.fill: parent; size: 40; source: authIcon.icon; visible: authIcon.shown }
                    Rectangle { anchors.fill: parent; radius: Theme.radiusControl; color: Theme.raised; visible: !authIcon.shown
                        Glyph { anchors.centerIn: parent; name: "key-round"; size: 14 } }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Label { text: "Authentication required"; size: Theme.sizeHeading; weight: Font.DemiBold; Layout.fillWidth: true }
                    Label { text: Auth.flow ? Auth.flow.message : ""; size: Theme.sizeSmall; color: Theme.text2; Layout.fillWidth: true; wrapMode: Text.WordWrap }
                }
            }

            Field {
                id: field
                Layout.fillWidth: true
                glyph: "key-round"
                placeholder: Auth.flow && Auth.flow.inputPrompt ? Auth.flow.inputPrompt.replace(/:\s*$/, "") : "Password"
                input.echoMode: Auth.flow && Auth.flow.responseVisible ? TextInput.Normal : TextInput.Password
                text: Auth.password
                onTextChanged: Auth.password = text
                onAccepted: Auth.submit()
                border.color: Auth.failed ? Theme.danger : (input.activeFocus ? Theme.accent : Theme.hairline)
                input.Keys.onEscapePressed: Auth.cancel()
            }

            RowLayout {
                Label { visible: Auth.failed; text: "Wrong password"; size: Theme.sizeCaption; color: Theme.danger }
                Label { visible: !Auth.failed && Auth.flow && Auth.flow.actionId; text: Auth.flow ? Auth.flow.actionId : ""; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true; elide: Text.ElideMiddle }
                Item { Layout.fillWidth: true }
                Button { text: "Cancel"; variant: "text"; onClicked: Auth.cancel() }
                Button { text: "Authenticate"; variant: "accent"; onClicked: Auth.submit() }
            }
        }
    }
}
