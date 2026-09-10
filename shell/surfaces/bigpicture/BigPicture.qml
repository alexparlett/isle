import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// Big Picture: 10-foot tiles for the launchers and the library, driven by arrows and Enter (the controller's pad and A).
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Modes.current === "bigpicture"

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-bigpicture"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: Theme.ink

    SystemClock { id: clock; precision: SystemClock.Minutes }

    readonly property var launchers: [
        { key: "steam", glyph: "gamepad-2", label: "Steam", bg: "#1B2838", enabled: Games.tools.steam, run: () => Games.launchSteam() },
        { key: "heroic", glyph: "rocket", label: "Heroic", bg: "#2B2358", enabled: Games.tools.heroic, run: () => Games.launch("heroic") },
        { key: "lutris", glyph: "swords", label: "Lutris", bg: "#5A3A16", enabled: Games.tools.lutris, run: () => Games.launch("lutris") },
        { key: "desktop", glyph: "monitor", label: "Desktop", bg: Theme.raised, enabled: true, run: () => Modes.set("normal") },
    ]
    readonly property int perRow: Math.max(4, Math.floor((width - Theme.s6 * 2) / 232))
    // Focus runs over the launcher row, then the library grid.
    property int focus: 0
    readonly property int total: launchers.length + Games.library.length

    onVisibleChanged: { Gamepad.active = visible; if (visible) { focus = 0; Games.refresh(); keys.forceActiveFocus(); } }
    Connections {
        target: Gamepad
        function onPressed(b) {
            if (!root.visible) return;
            if (b === "left") root.move(-1, 0); else if (b === "right") root.move(1, 0);
            else if (b === "up") root.move(0, -1); else if (b === "down") root.move(0, 1);
            else if (b === "a" || b === "start") root.activate();
            else if (b === "b") Modes.set("normal");
        }
    }

    function activate() {
        if (focus < launchers.length) { const l = launchers[focus]; if (l.enabled) l.run(); }
        else Games.play(Games.library[focus - launchers.length]);
    }
    function move(dx, dy) {
        let f = focus;
        if (dy !== 0) {
            if (f < launchers.length) f = dy > 0 ? launchers.length + Math.min(f, Math.max(0, Games.library.length - 1)) : f;
            else {
                const i = f - launchers.length;
                const ni = i + dy * perRow;
                f = ni < 0 ? Math.min(i, launchers.length - 1) : ni < Games.library.length ? launchers.length + ni : f;
            }
        } else f = Math.max(0, Math.min(total - 1, f + dx));
        focus = f;
    }

    Item {
        id: keys
        focus: true
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_Left: root.move(-1, 0); break;
            case Qt.Key_Right: root.move(1, 0); break;
            case Qt.Key_Up: root.move(0, -1); break;
            case Qt.Key_Down: root.move(0, 1); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: root.activate(); break;
            case Qt.Key_Escape: case Qt.Key_Backspace: Modes.set("normal"); break;
            default: return;
            }
            event.accepted = true;
        }
    }

    component Tile: Item {
        id: tile
        property string label
        property string glyph: ""
        property string art: ""
        property color bg: Theme.raised
        property bool enabled: true
        property bool sel: false
        property var run
        implicitWidth: 200
        implicitHeight: 200 + 44
        opacity: enabled ? 1 : 0.35
        Rectangle {
            id: face
            width: 200; height: 200
            radius: Theme.radiusPanel + 4
            color: tile.bg
            border.width: tile.sel ? 3 : 0
            border.color: Theme.accent
            scale: tile.sel ? 1.04 : 1
            Behavior on scale { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
            clip: true
            Image { anchors.fill: parent; source: tile.art ? "file://" + tile.art : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: tile.art !== "" }
            Glyph { anchors.centerIn: parent; name: tile.glyph; size: 64; weight: 1.5; color: "#FFFFFF"; visible: tile.glyph !== "" && tile.art === "" }
            Label { anchors.centerIn: parent; visible: tile.glyph === "" && tile.art === ""; text: tile.label.split(" ").slice(0, 2).map(w => w.charAt(0)).join(""); size: 48; weight: Font.DemiBold; color: Theme.text2 }
        }
        Label { anchors { top: face.bottom; topMargin: Theme.s3; horizontalCenter: face.horizontalCenter } text: tile.label; size: Theme.sizeTitle; weight: Font.DemiBold; color: tile.sel ? Theme.text : Theme.text2; width: 200; horizontalAlignment: Text.AlignHCenter }
        MouseArea { anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onEntered: root.focus = tile.index; onClicked: root.activate() }
        property int index: 0
    }

    ColumnLayout {
        anchors { fill: parent; margins: Theme.s6 + Theme.s4 }
        spacing: Theme.s6

        RowLayout {
            Layout.fillWidth: true
            ColumnLayout {
                spacing: 2
                Label { text: Qt.formatTime(clock.date, DateTime.timeFormat); size: Theme.sizeDisplay; weight: Font.DemiBold; tabular: true }
                Label { text: Qt.formatDate(clock.date, "dddd d MMMM"); size: Theme.sizeHeading; color: Theme.text2 }
            }
            Item { Layout.fillWidth: true }
            RowLayout {
                spacing: Theme.s5
                Repeater {
                    model: Power.peripherals
                    RowLayout {
                        required property var modelData
                        spacing: Theme.s2
                        Glyph { name: Power.glyphFor(modelData); size: 16 }
                        Label { text: Math.round(modelData.percentage) + "%"; size: Theme.sizeHeading; tabular: true; color: modelData.percentage < 20 ? Theme.warn : Theme.ok }
                    }
                }
                RowLayout { spacing: Theme.s2; visible: Bluetooth.anyConnected; Glyph { name: "headphones"; size: 16 } Label { text: Bluetooth.primaryName; size: Theme.sizeHeading; color: Theme.text2 } }
                Glyph { name: Audio.muted ? "volume-x" : "volume-2"; size: 16 }
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: Theme.s6
            Repeater {
                model: root.launchers
                Tile {
                    required property var modelData
                    required property int index
                    label: modelData.label; glyph: modelData.glyph; bg: modelData.bg; enabled: modelData.enabled
                    sel: root.focus === index
                    Component.onCompleted: this.index = index
                }
            }
        }

        Label { visible: Games.library.length > 0; text: "Library"; size: Theme.sizeHeading; weight: Font.DemiBold; color: Theme.text2 }
        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentHeight: grid.height
            clip: true
            Grid {
                id: grid
                width: parent.width
                columns: root.perRow
                columnSpacing: Theme.s6
                rowSpacing: Theme.s5
                Repeater {
                    model: Games.library
                    Tile {
                        required property var modelData
                        required property int index
                        label: modelData.name; art: modelData.art
                        sel: root.focus === root.launchers.length + index
                        Component.onCompleted: this.index = root.launchers.length + index
                    }
                }
            }
            Label { anchors.centerIn: parent; visible: Games.library.length === 0; text: Games.tools.steam ? "Nothing installed yet" : "Install Steam, Heroic or Lutris to fill this"; size: Theme.sizeHeading; color: Theme.text3 }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.s4
            component Hint: RowLayout {
                property string btn
                property string text
                spacing: Theme.s2
                Rectangle { width: 28; height: 28; radius: 14; color: "transparent"; border.width: 1.5; border.color: Theme.text3
                    Label { anchors.centerIn: parent; text: parent.parent.btn; size: Theme.sizeSmall; weight: Font.DemiBold; color: Theme.text3 } }
                Label { text: parent.text; size: Theme.sizeHeading; color: Theme.text3 }
            }
            Hint { btn: "A"; text: "Launch" }
            Hint { btn: "B"; text: "Desktop" }
            Item { Layout.fillWidth: true }
            Label { text: Prefs.p.gamescope && Games.tools.gamescope ? "Steam through gamescope" : ""; size: Theme.sizeHeading; color: Theme.text3 }
        }
    }
}
