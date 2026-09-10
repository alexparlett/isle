import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// One screen of the lock: the wallpaper blurred and dimmed, the clock in glass; the password field and the power
// actions appear on any movement or key and fade again when nothing happens for a while.
WlSessionLockSurface {
    id: root
    color: Theme.ink

    Backdrop { anchors.fill: parent; image: Prefs.p.wallpaper; blur: true; dim: 0.45 }

    SystemClock { id: clock; precision: SystemClock.Minutes }

    readonly property bool typing: Lock.password !== "" || Lock.failed || Lock.checking || Lock.capsLock
    property bool awake: false
    function wake() { awake = true; doze.restart(); }
    Timer { id: doze; interval: 12000; onTriggered: if (!root.typing) root.awake = false }
    HoverHandler { onPointChanged: root.wake() }
    TapHandler { onTapped: root.wake() }

    ColumnLayout {
        anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: Math.round(root.height * 0.28) }
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
                Label { Layout.alignment: Qt.AlignHCenter; text: Qt.formatTime(clock.date, DateTime.timeFormat); size: 56; weight: Font.DemiBold; tabular: true; font.letterSpacing: -1 }
                Label { Layout.alignment: Qt.AlignHCenter; text: Qt.formatDate(clock.date, "dddd d MMMM"); color: Theme.text2 }
            }
        }

        Glass {
            id: fieldCard
            Layout.alignment: Qt.AlignHCenter
            implicitWidth: 300
            implicitHeight: 44
            radius: 22
            opacity: root.typing || root.awake ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.quick } }
            border.color: Lock.failed ? Theme.danger : Theme.hairline

            // Wrong password: shake.
            transform: Translate { id: shake }
            SequentialAnimation {
                id: shakeAnim
                NumberAnimation { target: shake; property: "x"; to: -8; duration: 40 }
                NumberAnimation { target: shake; property: "x"; to: 8; duration: 70 }
                NumberAnimation { target: shake; property: "x"; to: -5; duration: 60 }
                NumberAnimation { target: shake; property: "x"; to: 0; duration: 50 }
            }
            Connections { target: Lock; function onFailedChanged() { if (Lock.failed) shakeAnim.restart(); } }

            RowLayout {
                anchors { fill: parent; leftMargin: Theme.s4; rightMargin: Theme.s4 }
                spacing: Theme.s2 + 2
                Glyph { name: "lock"; size: 14; color: Lock.failed ? Theme.danger : Theme.text2 }
                Label { visible: Lock.password === ""; text: "Password"; size: Theme.sizeSmall; color: Theme.text3 }
                Row {
                    Layout.fillWidth: true
                    spacing: 5
                    Repeater {
                        model: Lock.password.length
                        Rectangle { width: 7; height: 7; radius: 4; color: Theme.text; anchors.verticalCenter: parent.verticalCenter }
                    }
                }
                Label { text: Lock.checking ? "Checking" : Lock.failed ? Lock.message : Lock.capsLock ? "Caps Lock is on" : Quickshell.env("USER"); size: Theme.sizeCaption; color: Lock.failed ? Theme.danger : Lock.capsLock ? Theme.warn : Theme.text3 }
            }
        }
    }

    // Power, at the bottom, while awake. Sleep and lock again; the rest end the session.
    Glass {
        anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: Theme.s6 * 2 }
        width: powerRow.implicitWidth + Theme.s4 * 2
        height: 44
        radius: 22
        opacity: root.awake ? 1 : 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        RowLayout {
            id: powerRow
            anchors.centerIn: parent
            spacing: Theme.s4
            Repeater {
                model: [
                    { glyph: "moon", label: "Sleep", run: () => Session.sleep() },
                    { glyph: "rotate-cw", label: "Restart", run: () => Session.restart() },
                    { glyph: "power", label: "Shut down", run: () => Session.shutdown() },
                    { glyph: "log-out", label: "Log out", run: () => Session.logout() },
                ]
                RowLayout {
                    required property var modelData
                    spacing: Theme.s1 + 2
                    Glyph { name: modelData.glyph; size: 14; color: Theme.text2 }
                    Label { text: modelData.label; size: Theme.sizeSmall; color: Theme.text2 }
                    MouseArea { anchors { fill: parent; margins: -6 } cursorShape: Qt.PointingHandCursor; onClicked: modelData.run() }
                }
            }
        }
    }

    // Keys go straight to the password; nothing is echoed.
    TextInput {
        id: input
        anchors.fill: parent
        focus: true
        opacity: 0
        echoMode: TextInput.Password
        text: Lock.password
        onTextChanged: Lock.password = text
        Keys.onPressed: root.wake()
        onAccepted: Lock.submit()
        Keys.onEscapePressed: { text = ""; Lock.failed = false; }
        Connections { target: Lock; function onPasswordChanged() { if (Lock.password === "") input.text = ""; } }
    }
}
