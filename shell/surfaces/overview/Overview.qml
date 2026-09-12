import QtQuick
import QtQuick.Layouts
import QtQuick.Effects
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import Quickshell.Hyprland
import qs.services

// Mission Control: the current space's windows spread out live, the spaces in a bar across the top, the
// hidden windows in a tray. Windows travel from their real rectangles to the stage and back.
PanelWindow {
    id: root
    screen: Compositor.shellScreen
    visible: Surfaces.overview || leaving

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-overview"
    WlrLayershell.keyboardFocus: Surfaces.overview ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    // false: every window at its real rectangle; true: at its stage rectangle. The morph between is the motion.
    property bool spreadOut: false
    property bool leaving: false
    readonly property var current: Windows.groups.find(g => g.focused) || null
    readonly property var wins: current ? current.apps : []
    property var placed: ({})
    property int cursor: -1
    property var dragged: null
    property real dragX: 0
    property real dragY: 0
    property int dropTarget: -1
    readonly property int newWorkspaceId: Windows.groups.reduce((m, g) => Math.max(m, g.id), 0) + 1

    // Geometry: the bar under the island's strip, the stage between it and the tray.
    readonly property int tileW: 160
    readonly property int tileH: Math.round(tileW * height / Math.max(1, width))
    readonly property int barTop: 56
    readonly property int stageTop: barTop + tileH + 40 + Theme.s5
    readonly property int stageBottom: height - (Windows.hidden.length ? 80 : Theme.s5)
    readonly property int stageX: 48

    Connections {
        target: Surfaces
        function onOverviewChanged() { if (Surfaces.overview) root.enter(); }
    }
    onWinsChanged: if (visible) placed = layout()
    onWidthChanged: if (visible) placed = layout()

    // The windows' places come from the compositor's toplevel list, which the shell refreshes on entry.
    function enter() {
        leaving = false;
        spreadOut = false;
        Hyprland.refreshToplevels();
        placed = layout();
        cursor = Math.max(0, wins.findIndex(w => w.focused));
        settle.restart();
    }
    Timer { id: settle; interval: 60; onTriggered: { root.placed = root.layout(); root.spreadOut = true; } }
    // Everything goes home first; the surface hides as it lands.
    property var afterLeave: null
    function leave(then) {
        if (leaving) return;
        leaving = true;
        afterLeave = then || null;
        spreadOut = false;
        dragged = null; dropTarget = -1;
        leaveTimer.restart();
    }
    Timer { id: leaveTimer; interval: Theme.morph; onTriggered: { root.leaving = false; Surfaces.overview = false; if (root.afterLeave) { root.afterLeave(); root.afterLeave = null; } } }
    function choose(e) { if (e) leave(() => Windows.focus(e)); }
    function closeWindow(e) { Windows.closeWindow(e); }

    // The window's rectangle on this screen, from the compositor.
    function realRect(e) {
        const ipc = e.toplevel ? e.toplevel.lastIpcObject : null;
        if (!ipc || !ipc.at || !ipc.size) return { x: width / 2 - 200, y: height / 2 - 120, w: 400, h: 240 };
        return { x: ipc.at[0] - screen.x, y: ipc.at[1] - screen.y, w: Math.max(40, ipc.size[0]), h: Math.max(30, ipc.size[1]) };
    }

    // The spread: rows by position, as many as the stage's shape wants, each row scaled to fit, never above 1:1.
    function layout() {
        const W = width - stageX * 2, H = stageBottom - stageTop, gap = Theme.s5;
        const items = wins.map(e => ({ e: e, r: realRect(e) }));
        const n = items.length, out = {};
        if (!n) return out;
        const rows = Math.max(1, Math.min(n, Math.round(Math.sqrt(n * H / W * 1.6)))), perRow = Math.ceil(n / rows);
        items.sort((a, b) => (a.r.y + a.r.h / 2) - (b.r.y + b.r.h / 2) || a.r.x - b.r.x);
        const grid = [];
        for (let i = 0; i < n; i += perRow) grid.push(items.slice(i, i + perRow).sort((a, b) => a.r.x - b.r.x));
        let s = 1;
        for (const row of grid) s = Math.min(s, (W - gap * (row.length - 1)) / row.reduce((t, it) => t + it.r.w, 0));
        const rowH = grid.map(row => Math.max(...row.map(it => it.r.h)));
        s = Math.min(s, (H - gap * (grid.length - 1)) / rowH.reduce((t, h) => t + h, 0));
        let y = stageTop + (H - (rowH.reduce((t, h) => t + h, 0) * s + gap * (grid.length - 1))) / 2;
        grid.forEach((row, ri) => {
            const rowW = row.reduce((t, it) => t + it.r.w, 0) * s + gap * (row.length - 1);
            let x = stageX + (W - rowW) / 2;
            for (const it of row) {
                out[it.e.address] = { x: x, y: y + (rowH[ri] - it.r.h) * s / 2, w: it.r.w * s, h: it.r.h * s };
                x += it.r.w * s + gap;
            }
            y += rowH[ri] * s + gap;
        });
        return out;
    }

    // The nearest window in a direction, by centre.
    function step(dx, dy) {
        const from = wins[cursor] ? placed[wins[cursor].address] : null;
        if (!from) { cursor = wins.length ? 0 : -1; return; }
        const cx = from.x + from.w / 2, cy = from.y + from.h / 2;
        let best = -1, bestD = Infinity;
        wins.forEach((w, i) => {
            const p = placed[w.address]; if (!p || i === cursor) return;
            const ox = p.x + p.w / 2 - cx, oy = p.y + p.h / 2 - cy;
            if (dx * ox + dy * oy <= 0) return;
            const d = Math.abs(dx ? oy : ox) * 3 + Math.abs(dx ? ox : oy);
            if (d < bestD) { bestD = d; best = i; }
        });
        if (best >= 0) cursor = best;
    }

    function targetAt(px, py) {
        for (let i = 0; i <= tiles.count; i++) {
            const item = i < tiles.count ? tiles.itemAt(i) : newTile;
            if (!item) continue;
            const p = item.mapToItem(null, 0, 0);
            if (px >= p.x && px <= p.x + item.width && py >= p.y && py <= p.y + item.height) return i;
        }
        return -1;
    }
    function drop() {
        const e = dragged, t = dropTarget;
        dragged = null; dropTarget = -1;
        if (!e || t < 0) return;
        const ws = t < tiles.count ? Windows.groups[t].id : newWorkspaceId;
        if (ws === e.workspace) return;
        // The window goes; the view stays on this space.
        Compositor.dispatch("hl.dsp.window.move({ workspace = " + ws + ", follow = false, window = \"address:" + e.address + "\" })");
    }

    // Backdrop: the desktop dimmed; the compositor blurs this surface.
    Rectangle {
        anchors.fill: parent
        color: Qt.alpha(Theme.ink, 0.55)
        opacity: root.spreadOut ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        MouseArea { anchors.fill: parent; onClicked: root.leave() }
        focus: true
        Keys.onPressed: event => {
            const n = root.wins.length, ctrl = event.modifiers & Qt.ControlModifier;
            if (event.key === Qt.Key_Escape) root.leave();
            else if (ctrl && event.key === Qt.Key_Left) Compositor.dispatch("hl.dsp.focus({ workspace = \"e-1\" })");
            else if (ctrl && event.key === Qt.Key_Right) Compositor.dispatch("hl.dsp.focus({ workspace = \"e+1\" })");
            else if (event.key === Qt.Key_Left) root.step(-1, 0);
            else if (event.key === Qt.Key_Right) root.step(1, 0);
            else if (event.key === Qt.Key_Up) root.step(0, -1);
            else if (event.key === Qt.Key_Down) root.step(0, 1);
            else if (event.key === Qt.Key_Tab) root.cursor = n ? (root.cursor + (event.modifiers & Qt.ShiftModifier ? n - 1 : 1)) % n : -1;
            else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) root.choose(root.wins[root.cursor]);
            else if (event.key === Qt.Key_W || event.key === Qt.Key_Delete) root.closeWindow(root.wins[root.cursor]);
            else if (event.key >= Qt.Key_1 && event.key <= Qt.Key_9) { Compositor.focusWorkspace(event.key - Qt.Key_0); root.leave(); }
            else return;
            event.accepted = true;
        }
    }

    // The spaces bar.
    Row {
        id: bar
        anchors.horizontalCenter: parent.horizontalCenter
        y: root.spreadOut ? root.barTop : -root.tileH - 60
        spacing: Theme.s4
        opacity: root.spreadOut ? 1 : 0
        Behavior on y { NumberAnimation { duration: Theme.morph; easing.type: Easing.OutQuint } }
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        Repeater {
            id: tiles
            model: Windows.groups
            Column {
                id: space
                required property var modelData
                required property int index
                readonly property bool hot: tileArea.containsMouse || root.dropTarget === index
                spacing: Theme.s2
                width: root.tileW + 40
                Rectangle {
                    id: tile
                    anchors.horizontalCenter: parent.horizontalCenter
                    width: space.hot ? root.tileW + 40 : root.tileW
                    height: Math.round(width * root.height / Math.max(1, root.width))
                    radius: 8
                    color: Theme.ink
                    border.width: space.modelData.focused || space.hot ? 2 : 1
                    border.color: space.modelData.focused || space.hot ? Theme.accent : Theme.hairlineStrong
                    clip: true
                    Behavior on width { NumberAnimation { duration: Theme.quick; easing.type: Easing.OutQuint } }
                    Image { anchors.fill: parent; anchors.margins: 1; source: Prefs.p.wallpaper ? "file://" + Prefs.p.wallpaper : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; opacity: 0.9 }
                    Repeater {
                        model: space.modelData.apps
                        Rectangle {
                            required property var modelData
                            readonly property var r: root.realRect(modelData)
                            readonly property real k: tile.width / Math.max(1, root.width)
                            x: r.x * k; y: r.y * k; width: Math.max(4, r.w * k); height: Math.max(3, r.h * k)
                            radius: 2
                            color: Theme.raised
                            border.width: 1; border.color: Theme.hairlineStrong
                            clip: true
                            ScreencopyView { anchors.fill: parent; anchors.margins: 1; captureSource: root.visible ? modelData.toplevel.wayland : null; live: root.visible; paintCursor: false }
                        }
                    }
                    MouseArea { id: tileArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { Compositor.focusWorkspace(space.modelData.id); root.leave(); } }
                }
                Label { anchors.horizontalCenter: parent.horizontalCenter; text: "Desktop " + space.modelData.id; size: Theme.sizeCaption; weight: Font.DemiBold; color: space.modelData.focused || space.hot ? Theme.text : Theme.text3 }
            }
        }
        Column {
            spacing: Theme.s2
            width: root.tileW + 40
            Rectangle {
                id: newTile
                anchors.horizontalCenter: parent.horizontalCenter
                width: root.tileW; height: Math.round(width * root.height / Math.max(1, root.width))
                radius: 8
                color: Qt.alpha(Theme.glass, 0.6)
                border.width: root.dropTarget === tiles.count || plusArea.containsMouse ? 2 : 1
                border.color: root.dropTarget === tiles.count || plusArea.containsMouse ? Theme.accent : Theme.hairlineStrong
                Glyph { anchors.centerIn: parent; name: "plus"; size: 18; color: Theme.text3 }
                MouseArea { id: plusArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { Compositor.focusWorkspace(root.newWorkspaceId); root.leave(); } }
            }
            Label { anchors.horizontalCenter: parent.horizontalCenter; text: " "; size: Theme.sizeCaption }
        }
    }

    // The stage.
    Repeater {
        model: root.wins
        Item {
            id: thumb
            required property var modelData
            required property int index
            readonly property var real: root.realRect(modelData)
            readonly property var target: root.placed[modelData.address] || null
            readonly property bool dragging: root.dragged !== null && root.dragged.address === modelData.address
            readonly property bool current: root.cursor === index
            // Dragged, the thumbnail shrinks to tile size and goes translucent, so the target shows through it.
            readonly property real dragW: Math.min(root.tileW, target ? target.w * 0.5 : root.tileW)
            opacity: dragging ? 0.6 : 1
            Behavior on opacity { NumberAnimation { duration: Theme.quick } }
            x: dragging ? root.dragX - dragW / 2 : (root.spreadOut && target ? target.x : real.x)
            y: dragging ? root.dragY - dragW * (target ? target.h / target.w : 0.6) / 2 : (root.spreadOut && target ? target.y : real.y)
            width: dragging ? dragW : (root.spreadOut && target ? target.w : real.w)
            height: dragging ? dragW * (target ? target.h / target.w : 0.6) : (root.spreadOut && target ? target.h : real.h)
            z: dragging ? 30 : current ? 2 : 1
            Behavior on x { enabled: !thumb.dragging; NumberAnimation { duration: Theme.morph; easing.type: Easing.OutQuint } }
            Behavior on y { enabled: !thumb.dragging; NumberAnimation { duration: Theme.morph; easing.type: Easing.OutQuint } }
            Behavior on width { enabled: !thumb.dragging; NumberAnimation { duration: Theme.morph; easing.type: Easing.OutQuint } }
            Behavior on height { enabled: !thumb.dragging; NumberAnimation { duration: Theme.morph; easing.type: Easing.OutQuint } }

            Item {
                id: picture
                anchors.fill: parent
                layer.enabled: true
                layer.effect: MultiEffect { maskEnabled: true; maskSource: mask; maskThresholdMin: 0.5; maskSpreadAtMin: 1 }
                Rectangle { anchors.fill: parent; color: Theme.raised; visible: !view.hasContent; AppIcon { anchors.centerIn: parent; size: Math.min(64, parent.height / 2); source: thumb.modelData.icon } }
                ScreencopyView { id: view; anchors.fill: parent; captureSource: root.visible ? thumb.modelData.toplevel.wayland : null; live: root.visible; paintCursor: false }
            }
            Item {
                id: mask
                anchors.fill: parent
                layer.enabled: true
                visible: false
                Rectangle { anchors.fill: parent; radius: 10 }
            }
            Rectangle {
                anchors.fill: parent
                radius: 10
                color: "transparent"
                border.width: thumb.current ? 2 : 1
                border.color: thumb.current ? Theme.accent : Theme.hairlineStrong
                opacity: root.spreadOut ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: Theme.quick } }
            }
            MouseArea {
                id: area
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                cursorShape: root.dragged ? Qt.ClosedHandCursor : Qt.PointingHandCursor
                property bool moved: false
                onEntered: root.cursor = thumb.index
                onPressed: mouse => { moved = false; }
                onPositionChanged: mouse => {
                    if (!pressed) return;
                    const p = mapToItem(null, mouse.x, mouse.y);
                    if (!root.dragged) { if (!moved && Math.abs(mouse.x) + Math.abs(mouse.y) < 6) return; moved = true; root.dragged = thumb.modelData; }
                    root.dragX = p.x; root.dragY = p.y;
                    root.dropTarget = root.targetAt(p.x, p.y);
                }
                onReleased: mouse => {
                    if (root.dragged) { root.drop(); return; }
                    if (mouse.button === Qt.MiddleButton) root.closeWindow(thumb.modelData); else root.choose(thumb.modelData);
                }
                onCanceled: { root.dragged = null; root.dropTarget = -1; }
            }
            // Close, at the corner, under the pointer.
            Rectangle {
                x: -9; y: -9
                width: 22; height: 22; radius: 11
                color: Theme.ink
                border.width: 1; border.color: Theme.hairlineStrong
                visible: root.spreadOut && !thumb.dragging && (area.containsMouse || closeArea.containsMouse)
                Glyph { anchors.centerIn: parent; name: "x"; size: 11; weight: 1.6; color: Theme.text }
                MouseArea { id: closeArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.closeWindow(thumb.modelData) }
            }
            // The title, in a pill beneath, under the pointer.
            Glass {
                anchors { top: parent.bottom; topMargin: Theme.s2; horizontalCenter: parent.horizontalCenter }
                width: pillRow.implicitWidth + Theme.s3 * 2
                height: 28
                radius: 14
                visible: root.spreadOut && !thumb.dragging && (area.containsMouse || closeArea.containsMouse)
                RowLayout {
                    id: pillRow
                    anchors.centerIn: parent
                    spacing: Theme.s2
                    AppIcon { size: 16; source: thumb.modelData.icon }
                    Label { text: thumb.modelData.title || Windows.nameFor(thumb.modelData.appId); size: Theme.sizeCaption; weight: Font.DemiBold; elide: Text.ElideMiddle; Layout.maximumWidth: 360 }
                }
            }
        }
    }

    // Hidden windows, along the bottom edge.
    Glass {
        visible: Windows.hidden.length > 0
        anchors { bottom: parent.bottom; bottomMargin: Theme.s4; horizontalCenter: parent.horizontalCenter }
        width: trayRow.implicitWidth + Theme.s4 * 2
        height: 48
        radius: 24
        opacity: root.spreadOut ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: Theme.quick } }
        RowLayout {
            id: trayRow
            anchors.centerIn: parent
            spacing: Theme.s3
            Label { text: "Hidden"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3; Layout.rightMargin: Theme.s1 }
            Repeater {
                model: Windows.hidden
                AppIcon {
                    required property var modelData
                    size: 28; source: modelData.icon; opacity: hiddenArea.containsMouse ? 1 : 0.55
                    MouseArea { id: hiddenArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: root.leave(() => Windows.restore(modelData)) }
                }
            }
        }
    }
}
