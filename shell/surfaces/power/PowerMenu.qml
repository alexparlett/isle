import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// Pills in a row. Arrows or a first letter select; Enter runs; Escape closes. Restart, shut down and log out count down first.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Surfaces.power

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-power"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    // Hibernate only when the kernel can and there is swap to hold the image.
    property bool canHibernate: false
    Process {
        command: ["sh", "-c", "grep -q disk /sys/power/state && [ $(wc -l < /proc/swaps) -gt 1 ] && echo yes || echo no"]
        running: true
        stdout: StdioCollector { onStreamFinished: root.canHibernate = text.trim() === "yes" }
    }

    readonly property var actions: [
        { glyph: "lock", label: "Lock", key: "l", confirm: false, run: () => Session.lock() },
        { glyph: "moon", label: "Sleep", key: "s", confirm: false, run: () => Session.sleep() },
        { glyph: "hard-drive", label: "Hibernate", key: "h", confirm: false, run: () => Session.hibernate(), hidden: !canHibernate },
        { glyph: "rotate-cw", label: "Restart", key: "r", confirm: true, verb: "Restarting", run: () => Session.restart() },
        { glyph: "power", label: "Shut down", key: "p", confirm: true, verb: "Shutting down", run: () => Session.shutdown() },
        { glyph: "log-out", label: "Log out", key: "o", confirm: true, verb: "Logging out", run: () => Session.logout() },
    ].filter(a => !a.hidden)
    property int selected: 0
    // A pending confirmed action and the seconds left on its countdown.
    property var pending: null
    property int left: 0
    onVisibleChanged: { if (visible) { selected = 0; keys.forceActiveFocus(); } pending = null; }

    function run() {
        const a = actions[selected];
        if (!a) return;
        if (!a.confirm) { Surfaces.power = false; Qt.callLater(a.run); return; }
        if (pending === a) { Surfaces.power = false; Qt.callLater(a.run); return; }
        pending = a; left = 5;
    }
    Timer {
        interval: 1000; repeat: true; running: root.pending !== null
        onTriggered: { root.left--; if (root.left <= 0) { const a = root.pending; root.pending = null; Surfaces.power = false; Qt.callLater(a.run); } }
    }

    Rectangle {
        anchors.fill: parent
        color: Qt.alpha(Theme.ink, 0.4)
        MouseArea { anchors.fill: parent; onClicked: Surfaces.power = false }
    }

    Item {
        id: keys
        focus: true
        Keys.onPressed: event => {
            if (event.key === Qt.Key_Escape) { if (root.pending) root.pending = null; else Surfaces.power = false; }
            else if (event.key === Qt.Key_Left) root.selected = (root.selected + root.actions.length - 1) % root.actions.length;
            else if (event.key === Qt.Key_Right || event.key === Qt.Key_Tab) root.selected = (root.selected + 1) % root.actions.length;
            else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) root.run();
            else {
                const i = root.actions.findIndex(a => a.key === event.text.toLowerCase());
                if (i < 0) return;
                root.selected = i;
                root.run();
            }
            event.accepted = true;
        }
    }

    ColumnLayout {
        anchors.centerIn: parent
        spacing: Theme.s3

        Glass {
            Layout.alignment: Qt.AlignHCenter
            // In a layout the implicit size is the one that counts; a plain width is overridden.
            implicitWidth: row.implicitWidth + Theme.s2 * 2
            implicitHeight: row.implicitHeight + Theme.s2 * 2
            radius: Theme.radiusPanel + 4

            RowLayout {
                id: row
                anchors.centerIn: parent
                spacing: Theme.s2
                Repeater {
                    model: root.actions
                    Rectangle {
                        required property var modelData
                        required property int index
                        readonly property bool sel: index === root.selected
                        implicitWidth: 92; implicitHeight: 80
                        radius: Theme.radiusCard + 2
                        color: sel ? Theme.raised : "transparent"
                        border.width: 1
                        border.color: sel ? Theme.hairlineStrong : "transparent"
                        Behavior on color { ColorAnimation { duration: Theme.quick } }
                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: Theme.s2
                            Glyph { Layout.alignment: Qt.AlignHCenter; name: modelData.glyph; size: 17; color: sel ? Theme.text : Theme.text2 }
                            Label { Layout.alignment: Qt.AlignHCenter; text: modelData.label; size: Theme.sizeSmall; weight: sel ? Font.DemiBold : Font.Medium; color: sel ? Theme.text : Theme.text2 }
                        }
                        MouseArea { anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onEntered: root.selected = index; onClicked: root.run() }
                    }
                }
            }
        }

        // The countdown, while a confirmed action waits.
        Glass {
            Layout.alignment: Qt.AlignHCenter
            visible: root.pending !== null
            implicitWidth: countRow.implicitWidth + Theme.s4 * 2
            implicitHeight: 36
            radius: 18
            RowLayout {
                id: countRow
                anchors.centerIn: parent
                spacing: Theme.s2
                Label { text: root.pending ? root.pending.verb + " in " + root.left : ""; tabular: true; weight: Font.DemiBold }
                Label { text: "·  Enter now  ·  Esc to cancel"; color: Theme.text3; size: Theme.sizeSmall }
            }
        }
    }
}
