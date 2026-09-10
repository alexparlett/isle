import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// The capture toolbar at the bottom centre: what to capture, then shoot, record, or pick a colour.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Surfaces.capture

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-capture"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors.bottom: true
    margins.bottom: Theme.s5
    color: "transparent"

    implicitWidth: bar.width
    implicitHeight: bar.height

    function go(action) {
        Surfaces.capture = false;
        // Let the surface unmap before slurp and grim look at the screen.
        Qt.callLater(() => action());
    }

    Glass {
        id: bar
        width: row.implicitWidth + Theme.s2 * 2
        height: 44
        radius: 22
        focus: true
        Keys.onEscapePressed: Surfaces.capture = false
        Keys.onReturnPressed: root.go(() => Capture.shoot(Capture.mode))
        Keys.onPressed: event => {
            if (event.key === Qt.Key_1) Capture.mode = "region";
            else if (event.key === Qt.Key_2) Capture.mode = "window";
            else if (event.key === Qt.Key_3) Capture.mode = "screen";
            else if (event.key === Qt.Key_R) root.go(() => Capture.record(Capture.mode));
            else if (event.key === Qt.Key_P) root.go(() => Capture.pick());
            else return;
            event.accepted = true;
        }

        component Seg: Rectangle {
            property string glyph
            property string label
            property string mode
            readonly property bool on: Capture.mode === mode
            implicitWidth: segRow.implicitWidth + Theme.s3 * 2
            implicitHeight: 32
            radius: 16
            color: on ? Theme.raised : "transparent"
            border.width: 1
            border.color: on ? Theme.hairlineStrong : "transparent"
            RowLayout {
                id: segRow
                anchors.centerIn: parent
                spacing: Theme.s1 + 2
                Glyph { name: parent.parent.glyph; size: 14; color: parent.parent.on ? Theme.text : Theme.text2 }
                Label { text: parent.parent.label; size: Theme.sizeSmall; weight: Font.DemiBold; color: parent.parent.on ? Theme.text : Theme.text2 }
            }
            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Capture.mode = parent.mode }
        }
        component Act: Rectangle {
            property string glyph
            property var action
            property bool stay: false
            property color tint: Theme.text2
            implicitWidth: 32; implicitHeight: 32
            radius: 16
            color: area.containsMouse ? Theme.raised : "transparent"
            Glyph { anchors.centerIn: parent; name: parent.glyph; size: 14; color: area.containsMouse ? Theme.text : parent.tint }
            MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: parent.stay ? parent.action() : root.go(parent.action) }
        }

        RowLayout {
            id: row
            anchors.centerIn: parent
            spacing: 2
            Seg { glyph: "square"; label: "Region"; mode: "region" }
            Seg { glyph: "app-window"; label: "Window"; mode: "window" }
            Seg { glyph: "monitor"; label: "Screen"; mode: "screen" }
            Rectangle { width: 1; height: 20; color: Theme.hairlineStrong; Layout.leftMargin: Theme.s1 + 2; Layout.rightMargin: Theme.s1 + 2 }
            Act { glyph: "camera"; action: () => Capture.shoot(Capture.mode) }
            Act { glyph: "video"; action: () => Capture.record(Capture.mode); tint: Capture.recording ? Theme.live : Theme.text2 }
            Act { glyph: "pipette"; action: () => Capture.pick() }
            Rectangle { width: 1; height: 20; color: Theme.hairlineStrong; Layout.leftMargin: Theme.s1 + 2; Layout.rightMargin: Theme.s1 + 2 }
            // Options: a delay, the cursor, audio in recordings.
            Rectangle {
                implicitHeight: 32; implicitWidth: delayLabel.implicitWidth + Theme.s3 * 2
                radius: 16; color: Prefs.p.captureDelay > 0 ? Theme.raised : "transparent"; border.width: 1; border.color: Prefs.p.captureDelay > 0 ? Theme.hairlineStrong : "transparent"
                RowLayout { id: delayLabel; anchors.centerIn: parent; spacing: 4
                    Glyph { name: "timer"; size: 14; color: Prefs.p.captureDelay > 0 ? Theme.text : Theme.text2 }
                    Label { text: Prefs.p.captureDelay > 0 ? Prefs.p.captureDelay + "s" : ""; size: Theme.sizeSmall; weight: Font.DemiBold; visible: text !== "" } }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Prefs.p.captureDelay = Prefs.p.captureDelay === 0 ? 3 : Prefs.p.captureDelay === 3 ? 5 : 0 }
            }
            Act { glyph: "mouse-pointer"; tint: Prefs.p.captureCursor ? Theme.text : Theme.text3; stay: true; action: () => { Prefs.p.captureCursor = !Prefs.p.captureCursor; } }
            Act { glyph: "mic"; tint: Prefs.p.captureAudio ? Theme.text : Theme.text3; stay: true; action: () => { Prefs.p.captureAudio = !Prefs.p.captureAudio; } }
        }
    }
}
