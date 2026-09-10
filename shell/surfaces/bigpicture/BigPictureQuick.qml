import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// The quick menu, the console's Guide-button panel: sound, display, network, controllers, power and the
// way home or out, on a slab at the right edge over whatever is in front. Up and down walk it, left and
// right adjust, A activates, B closes.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Games.quickOpen

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-quick"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; right: true }
    implicitWidth: 480
    color: "transparent"

    // Sinks and profiles cycle in place.
    function nextSink() { const s = Audio.sinks; if (s.length < 2) return; const i = s.indexOf(Audio.sink); Audio.setSink(s[(i + 1) % s.length]); }
    readonly property var profiles: ["power-saver", "balanced"].concat(Power.hasPerformance ? ["performance"] : [])
    function nextProfile(d) { const i = profiles.indexOf(Power.profile); Power.setProfile(profiles[(i + d + profiles.length) % profiles.length]); }
    function pair(d) { if (d.paired || d.trusted) Bluetooth.toggle(d); else d.pair(); }
    property string confirming: ""

    readonly property var pads: { Bluetooth.stamp; return Bluetooth.devices.filter(d => d.name && (d.connected || d.paired || d.trusted || Bluetooth.discovering)).slice(0, 6); }
    readonly property var items: [
        { head: "Sound and display" },
        { kind: "slider", glyph: Audio.muted ? "volume-x" : "volume-2", label: "Volume", sub: Audio.sink ? (Audio.sink.description || Audio.sink.name) : "", value: Audio.volume, set: v => Audio.setVolume(v), toggle: () => Audio.setMuted(!Audio.muted) },
        { kind: "cycle", glyph: "volume-2", label: "Output", sub: Audio.sink ? (Audio.sink.description || Audio.sink.name) : "None", run: () => root.nextSink(), left: () => root.nextSink() },
        { kind: "slider", glyph: "sun", label: "Brightness", value: Brightness.value, set: v => Brightness.set(v), hidden: !Brightness.available },
        { head: "Connect" },
        { kind: "toggle", glyph: "wifi", label: "Wi-Fi", sub: Network.summary, on: Network.wifiEnabled, set: v => Network.setWifiEnabled(v), hidden: !Network.wifiAvailable },
        { kind: "toggle", glyph: "bluetooth", label: "Bluetooth", sub: Bluetooth.discovering ? "Looking for devices" : Bluetooth.anyConnected ? Bluetooth.primaryName : "", on: Bluetooth.enabled, set: v => Bluetooth.setEnabled(v), hidden: !Bluetooth.available },
        { kind: "toggle", glyph: "search", label: "Pair a controller", sub: "Hold its pairing button", on: Bluetooth.discovering, set: v => Bluetooth.setDiscovering(v), hidden: !Bluetooth.enabled }
    ].concat(pads.map(d => ({ kind: "action", glyph: Bluetooth.glyphFor(d), label: Bluetooth.nameOf(d), sub: d.connected ? "Connected" : d.paired || d.trusted ? "Paired" : "Nearby", run: () => root.pair(d) }))).concat([
        { head: "System" },
        { kind: "cycle", glyph: "activity", label: "Power profile", sub: Power.profileLabel, run: () => root.nextProfile(1), left: () => root.nextProfile(-1) },
        { kind: "toggle", glyph: "bell-off", label: "Do not disturb", on: Notifications.dnd, set: v => Notifications.setDnd(v) },
        { kind: "action", glyph: "keyboard", label: "Keyboard", run: () => { Osk.toggle(); } },
        { kind: "action", glyph: "gamepad-2", label: "Isle home", run: () => Games.home(), hidden: Games.homeShown },
        { kind: "action", glyph: "moon", label: "Sleep", run: () => Session.sleep() },
        { kind: "action", glyph: "power", label: "Shut down", confirm: true, run: () => Session.shutdown() },
        { kind: "action", glyph: "monitor", label: "Desktop", sub: "Leave Big Picture", run: () => Games.desktop() }
    ]).filter(i => !i.hidden).filter((i, n, l) => !i.head || (n + 1 < l.length && !l[n + 1].head))

    property int cur: 1
    function step(d) { let i = cur; do { i = Math.max(0, Math.min(items.length - 1, i + d)); } while (items[i].head && i > 0 && i < items.length - 1); if (!items[i].head) cur = i; }
    function adjust(d) {
        const it = items[cur]; if (!it) return;
        if (it.kind === "slider") it.set(Math.max(0, Math.min(1, it.value + d * 0.05)));
        else if (it.kind === "cycle") (d > 0 ? it.run : it.left)();
        else if (it.kind === "toggle") it.set(d > 0);
    }
    function activate() {
        const it = items[cur]; if (!it) return;
        if (it.kind === "toggle") it.set(!it.on);
        else if (it.kind === "slider") { if (it.toggle) it.toggle(); }
        else if (it.confirm && root.confirming !== it.label) root.confirming = it.label;
        else { root.confirming = ""; it.run(); }
    }
    onCurChanged: confirming = ""
    onVisibleChanged: { Gamepad.listeners += visible ? 1 : -1; if (visible) { cur = 1; confirming = ""; keys.forceActiveFocus(); } }
    Connections {
        target: Gamepad
        function onPressed(b) {
            if (!root.visible || Osk.open) return;
            if (b === "up" || b === "rup") root.step(-1); else if (b === "down" || b === "rdown") root.step(1);
            else if (b === "left" || b === "rleft") root.adjust(-1); else if (b === "right" || b === "rright") root.adjust(1);
            else if (b === "a") root.activate();
            else if (b === "b" || b === "start") Games.quickOpen = false;
        }
    }
    Item {
        id: keys
        focus: true
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_Up: root.step(-1); break;
            case Qt.Key_Down: root.step(1); break;
            case Qt.Key_Left: root.adjust(-1); break;
            case Qt.Key_Right: root.adjust(1); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: root.activate(); break;
            case Qt.Key_Escape: Games.quickOpen = false; break;
            default: return;
            }
            event.accepted = true;
        }
    }

    Glass {
        anchors { fill: parent; margins: Theme.s4 }
        radius: Theme.radiusPanel
        ColumnLayout {
            anchors { fill: parent; margins: Theme.s4 }
            spacing: Theme.s2
            RowLayout {
                Layout.fillWidth: true
                Layout.bottomMargin: Theme.s2
                Label { text: "Quick menu"; size: Theme.sizeTitle; weight: Font.DemiBold; Layout.fillWidth: true }
                Label { text: Qt.formatTime(clock.date, DateTime.timeFormat); size: Theme.sizeHeading; color: Theme.text2; tabular: true }
            }
            SystemClock { id: clock; precision: SystemClock.Minutes }
            ListView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: root.items
                spacing: 2
                currentIndex: root.cur
                highlightRangeMode: ListView.ApplyRange
                preferredHighlightBegin: 0
                preferredHighlightEnd: height - 120
                highlightMoveDuration: Theme.quick
                delegate: Item {
                    id: rowItem
                    required property var modelData
                    required property int index
                    readonly property bool sel: index === root.cur
                    readonly property bool head: !!modelData.head
                    width: list.width
                    height: head ? 36 : 56
                    Label { visible: rowItem.head; anchors { left: parent.left; bottom: parent.bottom; bottomMargin: 6 } text: rowItem.modelData.head || ""; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3 }
                    Rectangle {
                        visible: !rowItem.head
                        anchors.fill: parent
                        radius: Theme.radiusControl
                        color: rowItem.sel ? Theme.pressed : "transparent"
                        border.width: rowItem.sel ? 1 : 0
                        border.color: Theme.accent
                        RowLayout {
                            anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                            spacing: Theme.s3
                            Glyph { name: rowItem.modelData.glyph || ""; size: 18; color: rowItem.sel ? Theme.text : Theme.text2 }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 1
                                Label { text: root.confirming === rowItem.modelData.label ? rowItem.modelData.label + "? Press again" : (rowItem.modelData.label || ""); size: Theme.sizeHeading; weight: Font.DemiBold; color: root.confirming === rowItem.modelData.label ? Theme.danger : Theme.text; elide: Text.ElideRight; Layout.fillWidth: true }
                                Label { visible: !!rowItem.modelData.sub; text: rowItem.modelData.sub || ""; size: Theme.sizeSmall; color: Theme.text2; elide: Text.ElideRight; Layout.fillWidth: true }
                            }
                            Slider { visible: rowItem.modelData.kind === "slider"; implicitWidth: 140; value: rowItem.modelData.value || 0; onMoved: v => rowItem.modelData.set(v) }
                            Toggle { visible: rowItem.modelData.kind === "toggle"; checked: !!rowItem.modelData.on; onToggled: v => rowItem.modelData.set(v) }
                            Label { visible: rowItem.modelData.kind === "cycle"; text: "‹ ›"; size: Theme.sizeHeading; color: Theme.text3 }
                            Glyph { visible: rowItem.modelData.kind === "action"; name: "chevron-right"; size: 16; color: Theme.text3 }
                        }
                        MouseArea { anchors.fill: parent; z: -1; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onEntered: root.cur = rowItem.index; onClicked: root.activate() }
                    }
                }
            }
            PadLegend { Layout.fillWidth: true; big: true; kind: Gamepad.kind; items: [{ keys: ["a"], label: "Select" }, { keys: ["b"], label: "Close" }, { keys: ["left", "right"], label: "Adjust" }] }
        }
    }
}
