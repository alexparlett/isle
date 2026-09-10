import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// The launcher: one field, results as rows. Prefixes pick a source; Enter runs the selected row.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Surfaces.launcher

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-launcher"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    property int selected: 0

    onVisibleChanged: {
        if (visible) { field.text = Surfaces.launcherPrefix; Launcher.query = field.text; selected = 0; field.input.forceActiveFocus(); }
        else { field.text = ""; Launcher.query = ""; }
    }

    // which: 0 Enter, 1 Shift+Enter, 2 Ctrl+Enter. Clipboard edits keep the list open.
    function run(which) {
        const r = Launcher.results[selected];
        if (!r || r.kind === "hint") return;
        const fn = which === 1 ? r.alt : which === 2 ? r.extra : r.run;
        if (!fn) return;
        fn();
        if (which && r.kind === "clip") return;
        Surfaces.launcher = false;
    }

    MouseArea { anchors.fill: parent; onClicked: Surfaces.launcher = false }

    Glass {
        id: card
        width: 640
        anchors { horizontalCenter: parent.horizontalCenter; top: parent.top; topMargin: Math.round(root.height * 0.28) }
        height: column.implicitHeight + Theme.s2 * 2
        radius: Theme.radiusPanel
        opacity: root.visible ? 1 : 0
        scale: root.visible ? 1 : 0.96
        Behavior on opacity { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
        Behavior on scale { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
        Behavior on height { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }

        MouseArea { anchors.fill: parent }

        ColumnLayout {
            id: column
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s2 }
            spacing: Theme.s1 + 2

            Field {
                id: field
                Layout.fillWidth: true
                implicitHeight: 48
                size: Theme.sizeHeading
                placeholder: "Search"
                onTextChanged: { Launcher.query = text; root.selected = 0; }
                onAccepted: root.run(0)
                input.Keys.onPressed: event => {
                    const n = Launcher.results.length;
                    if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && (event.modifiers & Qt.ShiftModifier)) { root.run(1); event.accepted = true; return; }
                    if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && (event.modifiers & Qt.ControlModifier)) { root.run(2); event.accepted = true; return; }
                    if (event.key === Qt.Key_Down || (event.key === Qt.Key_N && event.modifiers & Qt.ControlModifier)) { root.selected = n ? (root.selected + 1) % n : 0; event.accepted = true; }
                    else if (event.key === Qt.Key_Up || (event.key === Qt.Key_P && event.modifiers & Qt.ControlModifier)) { root.selected = n ? (root.selected + n - 1) % n : 0; event.accepted = true; }
                    else if (event.key === Qt.Key_Escape) { Surfaces.launcher = false; event.accepted = true; }
                    else if (event.key === Qt.Key_Tab) { event.accepted = true; }
                }
                Label {
                    anchors { right: parent.right; rightMargin: Theme.s3; verticalCenter: parent.verticalCenter }
                    visible: field.text === ""
                    text: "> run   = maths   / files   : clipboard   @ windows"
                    mono: true; size: Theme.sizeCaption; color: Theme.text3
                }
            }

            // At most eight rows tall; more scroll, with the selection kept in view.
            ListView {
                id: list
                Layout.fillWidth: true
                Layout.preferredHeight: count ? Math.min(count, 8) * (52 + spacing) - spacing : 0
                spacing: Theme.s1 + 2
                clip: true
                model: Launcher.results
                boundsBehavior: Flickable.StopAtBounds
                currentIndex: root.selected
                highlightFollowsCurrentItem: true
                highlightMoveDuration: 0
                preferredHighlightBegin: 0; preferredHighlightEnd: height
                highlightRangeMode: ListView.ApplyRange
                Scrollbar { target: list; anchors { top: parent.top; bottom: parent.bottom; right: parent.right } }
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: list.width
                    height: 52
                    radius: Theme.radiusControl + 2
                    color: index === root.selected ? Theme.raised : "transparent"
                    border.width: 1
                    border.color: index === root.selected ? Theme.hairlineStrong : "transparent"

                    RowLayout {
                        anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                        spacing: Theme.s3
                        Item {
                            implicitWidth: 36; implicitHeight: 36
                            AppIcon { anchors.fill: parent; size: 36; source: modelData.icon || ""; visible: !!modelData.icon }
                            Rectangle {
                                anchors.fill: parent; radius: 10; color: Theme.pressed; visible: !modelData.icon
                                clip: true
                                Image { anchors.fill: parent; source: modelData.thumb || ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: !!modelData.thumb; cache: false }
                                Glyph { anchors.centerIn: parent; name: modelData.glyph || "search"; size: 14; visible: !modelData.thumb }
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 1
                            Label { text: modelData.title; weight: Font.DemiBold; Layout.fillWidth: true; mono: modelData.kind === "calc" || modelData.kind === "run" }
                            Label { text: modelData.subtitle || ""; size: Theme.sizeCaption; color: Theme.text2; Layout.fillWidth: true }
                        }
                        Label { visible: !!modelData.chord; text: modelData.chord || ""; mono: true; size: Theme.sizeCaption; color: Theme.text3 }
                        Label { visible: index === root.selected && modelData.kind !== "hint"; text: (modelData.alt ? "⇧↵ " + modelData.altLabel + "   " : "") + (modelData.extra ? "⌃↵ " + modelData.extraLabel + "   " : "") + "↵"; mono: true; size: Theme.sizeCaption; color: Theme.text3 }
                    }
                    // Hover picks a row only when the pointer moved: a list scrolling under a still pointer must not.
                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onPositionChanged: root.selected = index
                        onClicked: { root.selected = index; root.run(false); }
                    }
                }
            }
        }
    }
}
