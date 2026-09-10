import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// The island. Rest is a pill; hover unfolds it into the control panel; events morph it for a moment.
PanelWindow {
    id: root

    screen: Compositor.shellScreen

    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "isle-island"
    // Keys only while an event asks for them (an inline reply).
    WlrLayershell.keyboardFocus: IslandEvents.current && IslandEvents.current.kind === "notification" ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors.top: true
    margins.top: Theme.islandMargin
    color: "transparent"

    // Enter after a short dwell, leave after a short grace, so a passing pointer does not flap the panel.
    property bool hovered: false
    Timer { id: enterDelay; interval: 120; onTriggered: root.hovered = true }
    Timer { id: leaveGrace; interval: 250; onTriggered: root.hovered = false }
    // A click that opens something folds the panel and disarms the hover until the pointer has left, so the
    // island does not grow back under a pointer that has not moved; nor does it unfold under a full-screen surface.
    property bool armed: true
    readonly property bool unfolded: (hovered && armed && !Surfaces.busy) || pinned
    property bool pinned: false
    function dismiss() { enterDelay.stop(); hovered = false; pinned = false; armed = false; }
    Connections {
        target: Surfaces
        function onBusyChanged() { if (Surfaces.busy && hover.hovered) root.armed = false; }
        // A window opened from the island: fold, and stay folded under the pointer.
        function onSettingsChanged() { if (Surfaces.settings && hover.hovered) root.dismiss(); }
        function onMonitorChanged() { if (Surfaces.monitor && hover.hovered) root.dismiss(); }
        function onKeychainChanged() { if (Surfaces.keychain && hover.hovered) root.dismiss(); }
    }
    // An event keeps the pill while the pointer is on it, so its buttons stay reachable; pinning still wins.
    // An open panel is not taken away by an event: an OSD raised from its own sliders is dropped, anything
    // else waits, held, until the pointer leaves.
    property bool eventHeld: false
    readonly property string mode: pinned ? "panel" : (IslandEvents.current && !eventHeld) ? "event" : unfolded ? "panel" : "rest"
    onUnfoldedChanged: if (!unfolded && eventHeld) { eventHeld = false; IslandEvents.resume(); }
    Connections {
        target: IslandEvents
        function onCurrentChanged() {
            if (!IslandEvents.current) { root.eventHeld = false; return; }
            // Only when the panel would otherwise be showing; a fresh event on the rest pill is left alone.
            if ((!root.unfolded && !root.pinned) || root.eventHeld) return;
            if (IslandEvents.current.kind === "osd") IslandEvents.dismissKind("osd");
            else { root.eventHeld = true; IslandEvents.hold(); }
        }
    }

    // Game mode unmaps the island; a 4px hot zone at the top edge peeks it for a few seconds.
    property bool peek: false
    property alias peekTimer: peekTimer
    Timer { id: peekTimer; interval: 4000; onTriggered: root.peek = false }
    visible: !Modes.game || peek || pinned

    // The window stays the panel's size through every morph, so the compositor never re-places a surface that is
    // changing size frame by frame; the body sits centred at its top, and only the body takes input.
    implicitWidth: Theme.panelWidth
    implicitHeight: Math.ceil(Math.max(body.height, Theme.headerHeight + panelBody.implicitHeight + Theme.s3 * 2))
    mask: Region { item: body }

    SystemClock { id: clock; precision: SystemClock.Minutes }

    IpcHandler {
        target: "island"
        function text(text: string): void { IslandEvents.show({ kind: "text", text: text, glyph: "check" }); }
        function pin(on: bool): void { root.pinned = on; }
        function clear(): void { IslandEvents.queue = []; IslandEvents.dismiss(); }
        function dnd(action: string): void { Notifications.setDnd(action === "on" ? true : action === "off" ? false : !Notifications.dnd); }
        // Sample events for design review: osd, media, text.
        function demo(kind: string): void {
            const d = 6000;
            if (kind === "osd") IslandEvents.show({ kind: "osd", duration: d, glyph: "volume-2", value: 0.64, label: "64" });
            else if (kind === "media") IslandEvents.show({ kind: "media", duration: d, title: "Nightcall", artist: "Kavinsky", artUrl: "" });
            else IslandEvents.show({ kind: "text", duration: d, text: "Copied to clipboard", glyph: "check" });
        }
    }

    Glass {
        id: body
        anchors { horizontalCenter: parent.horizontalCenter; top: parent.top }

        // Events get a little more air at the sides; their corners stay at the panel's radius rather than growing with height.
        readonly property int padX: root.mode === "event" ? Theme.s4 : Theme.s3
        width: root.mode === "panel" ? Theme.panelWidth
             : root.mode === "event" ? eventLoader.contentWidth + padX * 2
             : rest.implicitWidth + padX * 2
        height: root.mode === "panel" ? Theme.headerHeight + panelBody.implicitHeight + Theme.s3 * 2
              : root.mode === "event" ? Math.max(IslandEvents.current.height || Theme.islandHeight, eventLoader.contentHeight + Theme.s3)
              : Theme.islandHeight
        radius: root.mode === "panel" ? Theme.radiusPanel + 2 : Math.min(height / 2, Theme.radiusPanel + 2)

        Behavior on width { NumberAnimation { duration: Theme.morph; easing.type: Easing.OutQuint } }
        Behavior on height { NumberAnimation { duration: Theme.morph; easing.type: Easing.OutQuint } }
        Behavior on radius { NumberAnimation { duration: Theme.morph; easing.type: Easing.OutQuint } }

        // A click can shrink the panel out from under the pointer; a hover lost while the body is still
        // changing size is geometry, not intent, so the leave waits until it has settled.
        onWidthChanged: settle.restart()
        onHeightChanged: settle.restart()
        Timer { id: settle; interval: 500; onTriggered: if (!hover.hovered && root.hovered) leaveGrace.restart() }
        HoverHandler {
            id: hover
            onHoveredChanged: {
                if (hovered) { if (!root.armed) return; leaveGrace.stop(); enterDelay.restart(); }
                else { root.armed = true; enterDelay.stop(); if (!settle.running) leaveGrace.restart(); }
            }
        }

        // Rest
        RestContent {
            id: rest
            clock: clock
            anchors.centerIn: parent
            visible: opacity > 0
            opacity: root.mode === "rest" ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        }

        // A click on a notification's body: its default action, then the app.
        MouseArea {
            anchors.fill: eventLoader
            enabled: root.mode === "event" && IslandEvents.current !== null && IslandEvents.current.kind === "notification"
            cursorShape: Qt.PointingHandCursor
            onClicked: Notifications.open(IslandEvents.current.notification)
        }
        // Event
        Loader {
            id: eventLoader
            anchors.centerIn: parent
            active: IslandEvents.current !== null
            visible: opacity > 0
            opacity: root.mode === "event" ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.quick } }
            sourceComponent: EventContent { ev: IslandEvents.current ?? ({ kind: "text" }) }
            readonly property real contentWidth: item ? item.implicitWidth : 0
            readonly property real contentHeight: item ? item.implicitHeight : 0

            // Click acts on the event (a notification's default action); right click only dismisses.
            MouseArea {
                anchors.fill: parent
                z: -1
                acceptedButtons: Qt.LeftButton | Qt.RightButton | Qt.MiddleButton
                cursorShape: Qt.PointingHandCursor
                onClicked: mouse => {
                    const ev = IslandEvents.current;
                    if (!ev) return;
                    if (ev.kind === "recording") { Capture.stop(); return; }
                    if (ev.kind === "drive" || (ev.kind === "text" && ev.actions)) return;
                    if (mouse.button === Qt.LeftButton && ev.kind === "notification") Notifications.activate(ev.notification);
                    else if (mouse.button === Qt.LeftButton && ev.action) ev.action();
                    IslandEvents.dismiss();
                }
            }
        }

        // Panel: header band, then the body
        Item {
            anchors.fill: parent
            visible: opacity > 0
            opacity: root.mode === "panel" ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: Theme.move } }

            HeaderBand {
                clock: clock
                panel: panelBody
                anchors { left: parent.left; right: parent.right; top: parent.top; leftMargin: Theme.s4; rightMargin: Theme.s4 + 2 }
                height: Theme.headerHeight
            }
            Rectangle {
                anchors { left: parent.left; right: parent.right; top: parent.top; topMargin: Theme.headerHeight }
                height: 1
                color: Theme.hairline
            }
            PanelBody {
                id: panelBody
                onDismiss: root.dismiss()
                anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s3; topMargin: Theme.headerHeight + Theme.s3 }
            }
        }
    }
}
