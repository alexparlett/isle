import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// Big Picture's home: rows of tiles in the consoles' manner, the library across every store, the stores
// themselves, and the system; a stick or the pad walks them, A opens, Y searches through the on-screen
// keyboard, Start or Guide opens the quick menu. Steam's own UI and games take over in front of it.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Games.homeShown

    // Top, not overlay: the keyboard and the quick menu sit above it, and it shows only when nothing is fullscreen.
    WlrLayershell.layer: WlrLayer.Top
    WlrLayershell.namespace: "isle-bigpicture"
    // On demand, not exclusive: an exclusive layer takes every pointer event, and the on-screen keyboard
    // above it needs the clicks.
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: Theme.ink

    SystemClock { id: clock; precision: SystemClock.Minutes }

    // --- what is on the screen ---------------------------------------------------------------
    property string search: ""
    function game(g) { return { label: g.name, art: g.art, glyph: "", bg: Theme.raised, sub: g.source === "steam" ? "Steam" : g.source === "heroic" ? "Heroic" : "Lutris", enabled: true, run: () => Games.play(g) }; }
    readonly property var recent: Games.library.filter(g => g.last > 0).slice(0, 12).map(game)
    readonly property var all: (search ? Games.library.filter(g => g.name.toLowerCase().indexOf(search.toLowerCase()) >= 0) : Games.library).map(game)
    readonly property var stores: [
        { label: "Steam", glyph: "gamepad-2", bg: "#1B2838", sub: "Big Picture", enabled: Games.tools.steam, run: () => Games.openSteam() },
        { label: "Heroic", glyph: "rocket", bg: "#2B2358", sub: "Console mode", enabled: Games.tools.heroic, run: () => Games.openHeroic() },
        { label: "Lutris", glyph: "swords", bg: "#5A3A16", sub: "", enabled: Games.tools.lutris, run: () => Games.openLutris() }
    ]
    readonly property var system: [
        { label: "Quick menu", glyph: "settings-2", bg: Theme.raised, sub: "Sound, display, network", enabled: true, run: () => Games.quickOpen = true },
        { label: "Controllers", glyph: "bluetooth", bg: Theme.raised, sub: Bluetooth.anyConnected ? Bluetooth.primaryName : "Pair one", enabled: Bluetooth.available, run: () => { Games.quickOpen = true; } },
        { label: "Keyboard", glyph: "keyboard", bg: Theme.raised, sub: "", enabled: true, run: () => Osk.toggle() },
        { label: "Sleep", glyph: "moon", bg: Theme.raised, sub: "", enabled: true, run: () => Session.sleep() },
        { label: "Desktop", glyph: "monitor", bg: Theme.raised, sub: "Leave Big Picture", enabled: true, run: () => Games.desktop() }
    ]
    readonly property var rows: {
        const out = [];
        if (recent.length && !search) out.push({ title: "Continue", kind: "game", items: recent });
        if (all.length || search) out.push({ title: search ? "Results for \"" + search + "\"" : "Library", kind: "game", items: all });
        out.push({ title: "Stores", kind: "app", items: stores });
        out.push({ title: "System", kind: "app", items: system });
        return out;
    }

    // --- the focus -------------------------------------------------------------------------
    property int row: 0
    property var cols: []
    readonly property int col: Math.max(0, Math.min((rows[row] ? rows[row].items.length : 1) - 1, cols[row] || 0))
    readonly property var focused: rows[row] && rows[row].items[col] ? rows[row].items[col] : null
    function setCol(c) { const l = cols.slice(); l[row] = Math.max(0, Math.min(rows[row].items.length - 1, c)); cols = l; }
    function move(dx, dy) {
        if (dy) row = Math.max(0, Math.min(rows.length - 1, row + dy));
        else setCol(col + dx);
    }
    function activate() { if (focused && focused.enabled !== false) focused.run(); }
    onRowsChanged: if (row >= rows.length) row = Math.max(0, rows.length - 1)
    onVisibleChanged: {
        Gamepad.listeners += visible ? 1 : -1;
        if (visible) { row = 0; cols = []; search = ""; keys.forceActiveFocus(); }
        else Osk.hide();
    }
    Connections {
        target: Gamepad
        function onPressed(b) {
            if (!root.visible) return;
            // Guide held is home from anywhere: the top of the rows, the search cleared.
            if (b === "guide-hold") { root.search = ""; root.row = 0; return; }
            if (Games.quickOpen || Osk.open) return;
            if (b === "left" || b === "rleft") root.move(-1, 0); else if (b === "right" || b === "rright") root.move(1, 0);
            else if (b === "up" || b === "rup" || b === "lb") root.move(0, -1); else if (b === "down" || b === "rdown" || b === "rb") root.move(0, 1);
            else if (b === "lt") root.setCol(root.col - 5); else if (b === "rt") root.setCol(root.col + 5);
            else if (b === "a") root.activate();
            else if (b === "y") { root.search = ""; Osk.show(); }
            else if (b === "b") { if (root.search) root.search = ""; }
            else if (b === "start") Games.quickOpen = true;
        }
    }
    // Search: the on-screen keyboard types into this window, since it holds the focus.
    Item {
        id: keys
        focus: true
        Keys.onPressed: event => {
            switch (event.key) {
            case Qt.Key_Left: root.move(-1, 0); break;
            case Qt.Key_Right: root.move(1, 0); break;
            case Qt.Key_Up: root.move(0, -1); break;
            case Qt.Key_Down: root.move(0, 1); break;
            case Qt.Key_Return: case Qt.Key_Enter: if (Osk.open) Osk.hide(); else root.activate(); break;
            case Qt.Key_Escape: if (Osk.open) { Osk.hide(); root.search = ""; } else if (root.search) root.search = ""; else Games.desktop(); break;
            case Qt.Key_Backspace: root.search = root.search.slice(0, -1); break;
            case Qt.Key_F1: case Qt.Key_Menu: Games.quickOpen = true; break;
            default:
                if (event.text && event.text >= " " && !(event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))) { root.search += event.text; if (!Osk.open) root.row = Math.min(root.row, 1); break; }
                return;
            }
            event.accepted = true;
        }
    }

    // The focused game's art, faint, behind everything.
    Image {
        anchors.fill: parent
        source: root.focused && root.focused.art ? "file://" + root.focused.art : ""
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        opacity: status === Image.Ready ? 0.16 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.quick * 2 } }
    }
    Rectangle { anchors.fill: parent; gradient: Gradient { GradientStop { position: 0; color: Qt.alpha(Theme.ink, 0.2) } GradientStop { position: 1; color: Theme.ink } } }

    readonly property int tileW: 176
    readonly property int gapX: Theme.s4
    readonly property int margin: Theme.s6 + Theme.s4

    ColumnLayout {
        anchors { fill: parent; margins: root.margin; bottomMargin: Theme.s5 }
        spacing: Theme.s5

        // The top bar: the time, and what is connected.
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
                        Glyph { name: Power.glyphFor(modelData); size: 18 }
                        Label { text: Power.percent(modelData) + "%"; size: Theme.sizeHeading; tabular: true; color: Power.low(modelData) ? Theme.warn : Theme.ok }
                    }
                }
                RowLayout { spacing: Theme.s2; visible: Bluetooth.anyConnected; Glyph { name: "headphones"; size: 18 } Label { text: Bluetooth.primaryName; size: Theme.sizeHeading; color: Theme.text2 } }
                Glyph { name: Network.wifiConnected ? "wifi" : Network.wiredConnected ? "plug-zap" : "wifi-off"; size: 18; color: Network.connected ? Theme.text : Theme.text3 }
                Glyph { name: Audio.muted ? "volume-x" : "volume-2"; size: 18 }
                Glyph { name: "gamepad-2"; size: 18; color: Gamepad.kind ? Theme.text : Theme.text3 }
            }
        }

        // The focused tile's name, large, as the consoles put it above the rows.
        ColumnLayout {
            spacing: 2
            Layout.preferredHeight: 64
            Label { text: root.focused ? root.focused.label : ""; size: Theme.sizeDisplay + 8; weight: Font.DemiBold; elide: Text.ElideRight; Layout.maximumWidth: root.width * 0.6 }
            Label { text: root.focused ? (root.focused.sub || "") : ""; size: Theme.sizeHeading; color: Theme.text2 }
        }

        ListView {
            id: rowsView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: root.rows
            spacing: Theme.s5
            currentIndex: root.row
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: 0
            preferredHighlightEnd: height * 0.55
            highlightMoveDuration: Theme.quick * 2
            highlightFollowsCurrentItem: true
            delegate: ColumnLayout {
                id: rowItem
                required property var modelData
                required property int index
                readonly property bool isGame: modelData.kind === "game"
                readonly property int tileH: isGame ? Math.round(root.tileW * 1.5) : 104
                width: rowsView.width
                spacing: Theme.s3
                Label { text: rowItem.modelData.title; size: Theme.sizeTitle; weight: Font.DemiBold; color: rowItem.index === root.row ? Theme.text : Theme.text2 }
                ListView {
                    id: tiles
                    Layout.fillWidth: true
                    Layout.preferredHeight: rowItem.tileH + 44
                    orientation: ListView.Horizontal
                    clip: false
                    model: rowItem.modelData.items
                    spacing: root.gapX
                    currentIndex: rowItem.index === root.row ? root.col : (root.cols[rowItem.index] || 0)
                    highlightRangeMode: ListView.ApplyRange
                    preferredHighlightBegin: 0
                    preferredHighlightEnd: width - root.tileW * 2
                    highlightMoveDuration: Theme.quick * 2
                    delegate: Item {
                        id: tile
                        required property var modelData
                        required property int index
                        readonly property bool sel: rowItem.index === root.row && index === root.col
                        width: root.tileW
                        height: rowItem.tileH + 44
                        opacity: modelData.enabled === false ? 0.35 : 1
                        Rectangle {
                            id: face
                            width: root.tileW; height: rowItem.tileH
                            radius: Theme.radiusPanel
                            color: tile.modelData.bg || Theme.raised
                            border.width: tile.sel ? 3 : 0
                            border.color: Theme.accent
                            scale: tile.sel ? 1.06 : 1
                            Behavior on scale { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
                            clip: true
                            Image { anchors.fill: parent; source: tile.modelData.art ? "file://" + tile.modelData.art : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; visible: !!tile.modelData.art; sourceSize.width: root.tileW * 2 }
                            Glyph { anchors.centerIn: parent; name: tile.modelData.glyph || ""; size: rowItem.isGame ? 48 : 36; weight: 1.5; color: "#FFFFFF"; visible: !!tile.modelData.glyph && !tile.modelData.art }
                            Label { anchors.centerIn: parent; visible: !tile.modelData.glyph && !tile.modelData.art; text: (tile.modelData.label || "").split(" ").slice(0, 2).map(w => w.charAt(0)).join(""); size: 40; weight: Font.DemiBold; color: Theme.text2 }
                        }
                        Label { anchors { top: face.bottom; topMargin: Theme.s3; left: face.left; right: face.right } text: tile.modelData.label; size: Theme.sizeHeading; weight: Font.DemiBold; color: tile.sel ? Theme.text : Theme.text2; elide: Text.ElideRight; horizontalAlignment: Text.AlignHCenter }
                        MouseArea { anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onEntered: { root.row = rowItem.index; root.setCol(tile.index); } onClicked: root.activate() }
                    }
                }
                Label { visible: rowItem.modelData.items.length === 0; text: root.search ? "Nothing matches" : Games.tools.steam || Games.tools.heroic || Games.tools.lutris ? "Nothing installed yet" : "Install Steam, Heroic or Lutris to fill this"; size: Theme.sizeHeading; color: Theme.text3 }
            }
        }

        // The legend.
        RowLayout {
            Layout.fillWidth: true
            PadLegend {
                Layout.fillWidth: true
                big: true
                kind: Gamepad.kind
                items: [{ keys: ["a"], label: "Open" }, { keys: ["y"], label: "Search" }, { keys: ["lb", "rb"], label: "Rows" }, { keys: ["start"], label: "Quick menu" }, { keys: ["guide"], label: "Hold: home" }, { keys: ["select"], label: "Hold: keyboard" }]
            }
            Label { text: Prefs.p.gamescope && Games.tools.gamescope ? "Steam through gamescope" : ""; size: Theme.sizeCaption; color: Theme.text3 }
        }
    }
}
