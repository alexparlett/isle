//@ pragma IconTheme Papirus-Dark
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Services.Greetd
import qs.theme
import qs.ui

// The greeter, in the same material: the clock, a user line, a password field. greetd launches the session.
// Runs as its own Quickshell config under greetd's user; see greeter/README.md.
ShellRoot {
    id: root

    property string user: Quickshell.env("ISLE_GREETER_USER") || "user"
    property string command: Quickshell.env("ISLE_GREETER_COMMAND") || "start-hyprland"
    property string password: ""
    property bool failed: false
    property string message: ""

    Connections {
        target: Greetd
        function onAuthMessage(message, error, responseRequired, echoResponse) {
            if (responseRequired) Greetd.respond(root.password);
            else if (error) root.message = message;
        }
        function onAuthFailure(message) { root.failed = true; root.message = "Wrong password"; root.password = ""; }
        function onReadyToLaunch() { Greetd.launch([root.command], ["XDG_SESSION_TYPE=wayland"]); }
    }

    function submit() {
        if (password === "" || Greetd.state !== GreetdState.Inactive) return;
        failed = false;
        Greetd.createSession(user);
    }

    Variants {
        model: Quickshell.screens
        PanelWindow {
            required property var modelData
            screen: modelData
            WlrLayershell.layer: WlrLayer.Overlay
            WlrLayershell.namespace: "isle-greeter"
            WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
            exclusionMode: ExclusionMode.Ignore
            anchors { top: true; bottom: true; left: true; right: true }
            color: Theme.ink

            Backdrop { anchors.fill: parent; dim: 0.2 }
            SystemClock { id: clock; precision: SystemClock.Minutes }

            ColumnLayout {
                anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: Math.round(parent.height * 0.28) }
                spacing: Theme.s5
                Glass {
                    Layout.alignment: Qt.AlignHCenter
                    implicitWidth: clockCol.implicitWidth + Theme.s6 * 2 + Theme.s4
                    implicitHeight: clockCol.implicitHeight + Theme.s5 * 2
                    radius: Theme.radiusPanel + 12
                    ColumnLayout {
                        id: clockCol
                        anchors.centerIn: parent
                        spacing: 2
                        Label { Layout.alignment: Qt.AlignHCenter; text: Qt.formatTime(clock.date, "HH:mm"); size: 56; weight: Font.DemiBold; tabular: true; font.letterSpacing: -1 }
                        Label { Layout.alignment: Qt.AlignHCenter; text: Qt.formatDate(clock.date, "dddd d MMMM"); color: Theme.text2 }
                    }
                }
                Glass {
                    Layout.alignment: Qt.AlignHCenter
                    implicitWidth: 300; implicitHeight: 44; radius: 22
                    border.color: root.failed ? Theme.danger : Theme.hairline
                    RowLayout {
                        anchors { fill: parent; leftMargin: Theme.s4; rightMargin: Theme.s4 }
                        spacing: Theme.s2 + 2
                        Glyph { name: "lock"; size: 15; color: root.failed ? Theme.danger : Theme.text2 }
                        Row {
                            Layout.fillWidth: true
                            spacing: 5
                            Repeater { model: root.password.length; Rectangle { width: 7; height: 7; radius: 4; color: Theme.text; anchors.verticalCenter: parent.verticalCenter } }
                        }
                        Label { text: root.failed ? root.message : root.user; size: Theme.sizeCaption; color: root.failed ? Theme.danger : Theme.text3 }
                    }
                }
            }

            TextInput {
                anchors.fill: parent
                focus: true
                opacity: 0
                echoMode: TextInput.Password
                onTextChanged: root.password = text
                onAccepted: root.submit()
                Connections { target: root; function onPasswordChanged() { if (root.password === "") parent.text = ""; } }
            }
        }
    }
}
