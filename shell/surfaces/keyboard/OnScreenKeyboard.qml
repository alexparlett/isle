import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import qs.theme
import qs.ui
import qs.services

// The on-screen keyboard: a glass slab along the bottom, keys for a mouse, a finger or a controller, in the
// manner of the consoles: a suggestion strip, two cursors (one a stick each, the triggers pressing their
// half) and a legend of the pad's buttons. It never takes keyboard focus, so what it types goes to the
// window that had it.
PanelWindow {
    id: root

    screen: Compositor.shellScreen
    visible: Osk.open

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-keyboard"
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.None
    // Windows make room for it on the desktop; over a game it simply sits on top.
    exclusionMode: Modes.game ? ExclusionMode.Ignore : ExclusionMode.Auto
    anchors { bottom: true }
    color: "transparent"

    readonly property bool big: Modes.game
    readonly property int unit: big ? 72 : 54
    readonly property int keyH: big ? 60 : 46
    readonly property int gap: 6
    implicitWidth: Math.min(screen ? screen.width - Theme.s4 * 2 : 1000, unit * 10 + gap * 9 + Theme.s4 * 2)
    implicitHeight: rowsCol.implicitHeight + Theme.s3 * 2

    // Each key: `t` is what it shows, `v` what it types or does, `w` its width in units.
    function k(t, v, w) { return { t: t, v: v, w: w || 1 }; }
    readonly property var utility: [k("esc", "esc"), k("tab", "tab"), k("←", "left"), k("↑", "up"), k("↓", "down"), k("→", "right"), k("", "", 3), k("⌄", "hide")]
    readonly property var pages: ({
        letters: [
            utility,
            "qwertyuiop".split("").map(c => k(c, c)),
            [k("", "", 0.5)].concat("asdfghjkl".split("").map(c => k(c, c)), [k("", "", 0.5)]),
            [k("⇧", "shift", 1.5)].concat("zxcvbnm".split("").map(c => k(c, c)), [k("⌫", "backspace", 1.5)]),
            [k("?123", "page:symbols", 1.5), k(",", ","), k("", "space", 5), k(".", "."), k("↵", "enter", 1.5)]
        ],
        symbols: [
            utility,
            "1234567890".split("").map(c => k(c, c)),
            "@#£$%&*()".split("").map(c => k(c, c)).concat([k("-", "-")]),
            [k("=\\<", "page:more", 1.5)].concat("_+'\":;!".split("").map(c => k(c, c)), [k("⌫", "backspace", 1.5)]),
            [k("abc", "page:letters", 1.5), k(",", ","), k("?", "?"), k("", "space", 4), k(".", "."), k("↵", "enter", 1.5)]
        ],
        more: [
            utility,
            "~`|•√π÷×€¥".split("").map(c => k(c, c)),
            "[]{}<>^°=\\".split("").map(c => k(c, c)),
            [k("?123", "page:symbols", 1.5)].concat("/©®™✓¶§".split("").map(c => k(c, c)), [k("⌫", "backspace", 1.5)]),
            [k("abc", "page:letters", 1.5), k(",", ","), k("", "space", 5), k(".", "."), k("↵", "enter", 1.5)]
        ]
    })
    // The suggestion strip is a row of its own above the keys, when there is anything to offer.
    readonly property var suggestionRow: Osk.suggestions.map(w => ({ t: w, v: "suggest:" + w, w: 10 / Math.max(1, Osk.suggestions.length), fill: true }))
    readonly property var rows: (suggestionRow.length ? [suggestionRow] : []).concat(pages[Osk.page] || pages.letters)
    function isAction(v) { return v.length > 1 && v !== ","; }

    // Two cursors, as on the Deck: the left stick and pad move one over the left half, the right stick the
    // other over the right half; LT and RT press each, A presses the one last moved. B deletes, X is space,
    // Y shift, LB and RB move the caret, Start enters; Select held toggles the keyboard (Osk).
    property var cur: [{ row: 1, col: 0 }, { row: 1, col: 9 }]
    property int lastCur: 0
    property bool padUsed: false
    function units(row) { return rows[row].reduce((a, key) => a + key.w, 0); }
    function centre(row, col) { let x = 0; for (let i = 0; i < col; i++) x += rows[row][i].w; return x + rows[row][col].w / 2; }
    // The half a key belongs to, by where its centre sits.
    function half(row, col) { return centre(row, col) < units(row) / 2 ? 0 : 1; }
    function cols(row, side) { const out = []; for (let i = 0; i < rows[row].length; i++) if (rows[row][i].v !== "" && half(row, i) === side) out.push(i); return out; }
    function nearest(row, x, side) {
        const c = cols(row, side).length ? cols(row, side) : cols(row, 1 - side);
        let best = c[0], d = 1e9;
        for (const i of c) { const dd = Math.abs(centre(row, i) - x); if (dd < d) { d = dd; best = i; } }
        return best;
    }
    function place(side, row, x) { const c = cur.slice(); c[side] = { row: row, col: nearest(row, x, side) }; cur = c; lastCur = side; padUsed = true; }
    function move(side, dx, dy) {
        const p = cur[side];
        if (dy !== 0) place(side, Math.max(0, Math.min(rows.length - 1, p.row + dy)), centre(p.row, p.col));
        else {
            const c = cols(p.row, side), i = c.indexOf(p.col);
            place(side, p.row, centre(p.row, c[Math.max(0, Math.min(c.length - 1, (i < 0 ? 0 : i) + dx))]));
        }
    }
    // A page change or the suggestion strip coming and going re-places the cursors; a refresh of the strip does not.
    readonly property string shape: rows.length + ":" + Osk.page
    onShapeChanged: cur = cur.map((p, i) => { const r = Math.min(p.row, rows.length - 1); return { row: r, col: nearest(r, i === 0 ? units(r) / 4 : units(r) * 3 / 4, i) }; })
    onVisibleChanged: { Gamepad.listeners += visible ? 1 : -1; if (visible) { padUsed = false; cur = [{ row: 1, col: 0 }, { row: 1, col: 9 }]; } }
    Connections {
        target: Gamepad
        function onPressed(b) {
            if (!root.visible) return;
            if (b === "left") root.move(0, -1, 0); else if (b === "right") root.move(0, 1, 0);
            else if (b === "up") root.move(0, 0, -1); else if (b === "down") root.move(0, 0, 1);
            else if (b === "rleft") root.move(1, -1, 0); else if (b === "rright") root.move(1, 1, 0);
            else if (b === "rup") root.move(1, 0, -1); else if (b === "rdown") root.move(1, 0, 1);
            else if (b === "a") root.pressCursor(root.lastCur);
            else if (b === "lt") root.pressCursor(0);
            else if (b === "rt") root.pressCursor(1);
            else if (b === "b") Osk.key("backspace");
            else if (b === "x") Osk.key("space");
            else if (b === "y") Osk.key("shift");
            else if (b === "lb") Osk.key("left");
            else if (b === "rb") Osk.key("right");
            else if (b === "start") Osk.key("enter");
        }
    }
    function pressCursor(side) { padUsed = true; lastCur = side; const p = cur[side]; press(rows[p.row] ? rows[p.row][p.col] : null); }
    function press(key) {
        if (!key || key.v === "") return;
        if (isAction(key.v)) Osk.key(key.v); else Osk.type(key.v);
    }

    readonly property bool legendShown: Gamepad.kind !== "" || Modes.game
    readonly property var legend: [
        { keys: ["a"], label: "Select" }, { keys: ["b"], label: "Delete" }, { keys: ["x"], label: "Space" }, { keys: ["y"], label: "Shift" },
        { keys: ["lt", "rt"], label: "Halves" }, { keys: ["lb", "rb"], label: "Caret" }, { keys: ["start"], label: "Enter" }, { keys: ["select"], label: "Hold: hide" }
    ]

    Glass {
        anchors.fill: parent
        radius: Theme.radiusCard
        ColumnLayout {
            id: rowsCol
            anchors { fill: parent; margins: Theme.s3 }
            spacing: root.gap
            Repeater {
                model: root.rows
                RowLayout {
                    id: rowItem
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    spacing: root.gap
                    Repeater {
                        model: rowItem.modelData
                        Rectangle {
                            id: cap
                            required property var modelData
                            required property int index
                            readonly property bool blank: modelData.v === ""
                            readonly property bool action: root.isAction(modelData.v)
                            readonly property bool suggestion: modelData.v.indexOf("suggest:") === 0
                            readonly property bool lit: modelData.v === "shift" && Osk.shift > 0
                            readonly property int cursorSide: !root.padUsed ? -1 : root.cur[0].row === rowItem.index && root.cur[0].col === index ? 0 : root.cur[1].row === rowItem.index && root.cur[1].col === index ? 1 : -1
                            Layout.preferredWidth: root.unit * modelData.w + root.gap * (modelData.w - 1)
                            Layout.fillWidth: modelData.v === "space" || !!modelData.fill
                            Layout.preferredHeight: suggestion ? root.keyH - 10 : root.keyH
                            radius: Theme.radiusControl
                            opacity: blank ? 0 : 1
                            color: area.pressed ? Theme.accent : lit ? Qt.alpha(Theme.accent, 0.35) : suggestion ? "transparent" : action ? Theme.pressed : Theme.raised
                            border.width: cursorSide >= 0 ? 2 : suggestion ? 0 : 1
                            border.color: cursorSide === 0 ? Theme.accent : cursorSide === 1 ? Theme.ok : Theme.hairlineStrong
                            Behavior on color { ColorAnimation { duration: Theme.quick } }
                            Label {
                                anchors.centerIn: parent
                                text: Osk.shift > 0 && !cap.action && cap.modelData.t.length === 1 ? cap.modelData.t.toUpperCase() : cap.modelData.t
                                size: cap.action && !cap.suggestion && cap.modelData.t.length > 1 && cap.modelData.t.charCodeAt(0) < 128 ? Theme.sizeSmall : (root.big ? Theme.sizeTitle : Theme.sizeHeading)
                                weight: cap.suggestion ? Font.Normal : Font.DemiBold
                                color: area.pressed ? Theme.onAccent : cap.action && !cap.suggestion ? Theme.text2 : Theme.text
                            }
                            // Shift locked shows a mark under the arrow.
                            Rectangle { visible: cap.modelData.v === "shift" && Osk.shift === 2; anchors { bottom: parent.bottom; bottomMargin: 6; horizontalCenter: parent.horizontalCenter } width: 14; height: 3; radius: 1.5; color: Theme.accent }
                            MouseArea {
                                id: area
                                anchors.fill: parent
                                enabled: !cap.blank
                                cursorShape: Qt.PointingHandCursor
                                onPressed: { root.padUsed = false; root.press(cap.modelData); if (["backspace", "left", "right", "up", "down"].indexOf(cap.modelData.v) >= 0) repeat.start(); }
                                onReleased: repeat.stop()
                                onCanceled: repeat.stop()
                            }
                            // Held deletes and arrows repeat, as a real key does.
                            Timer { id: repeat; interval: 350; repeat: true; onTriggered: { interval = 45; root.press(cap.modelData); } onRunningChanged: if (!running) interval = 350 }
                        }
                    }
                }
            }
            // The legend, when a pad is about.
            Item {
                visible: root.legendShown
                Layout.fillWidth: true
                Layout.topMargin: 2
                implicitWidth: 0
                implicitHeight: legendFlow.implicitHeight
                PadLegend { id: legendFlow; width: parent.width; items: root.legend; kind: Gamepad.kind }
            }
        }
    }
}
