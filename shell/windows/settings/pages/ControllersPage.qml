import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Controllers"
    subtitle: "Every pad the kernel sees, and what it is doing."
    Component.onCompleted: Gamepad.watchers++
    Component.onDestruction: Gamepad.watchers--

    function faceLabel(kind, b) {
        if (kind === "sony") return ({ a: "✕", b: "○", x: "□", y: "△" })[b];
        if (kind === "nintendo") return ({ a: "B", b: "A", x: "Y", y: "X" })[b];
        return b.toUpperCase();
    }
    function battery(pad) { const d = Power.peripherals.find(d => Power.labelFor(d) === pad.name); return d ? "  ·  " + Power.percent(d) + "%" : ""; }

    SettingsGroup {
        heading: "Controllers"
        Repeater {
            model: Gamepad.pads
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: ({ sony: "PlayStation", xbox: "Xbox", nintendo: "Nintendo", generic: "Generic" })[modelData.kind] + "  ·  " + ({ usb: "USB", bluetooth: "Bluetooth", virtual: "Virtual" })[modelData.bus] + page.battery(modelData)
            }
        }
        SettingsRow { visible: Gamepad.pads.length === 0; label: "No controller connected" }
    }

    // A live picture per pad: the sticks in their rings, the dead zone the inner ring, the triggers as bars
    // (full for a pad whose triggers are buttons), and every button lit while it is down.
    Repeater {
        model: Gamepad.pads
        SettingsGroup {
            id: test
            required property var modelData
            heading: modelData.name
            component Key: Rectangle {
                property string name
                property string text: name
                property bool down: test.modelData.buttons.indexOf(name) >= 0
                width: 36; height: 36; radius: Theme.radiusControl
                color: down ? Theme.accent : Theme.raised
                border.width: 1; border.color: down ? Theme.accent : Theme.hairlineStrong
                Label { anchors.centerIn: parent; text: parent.text; size: Theme.sizeSmall; weight: Font.DemiBold; color: parent.down ? Theme.onAccent : Theme.text2 }
                Behavior on color { ColorAnimation { duration: Theme.quick } }
            }
            component Stick: Item {
                property real x1: 0
                property real y1: 0
                property bool down: false
                width: 96; height: 96
                Rectangle { anchors.fill: parent; radius: width / 2; color: Theme.raised; border.width: 1; border.color: Theme.hairlineStrong }
                Rectangle { anchors.centerIn: parent; width: parent.width * Gamepad.deadZone; height: width; radius: width / 2; color: "transparent"; border.width: 1; border.color: Theme.hairlineStrong }
                Rectangle {
                    width: 14; height: 14; radius: 7
                    color: parent.down ? Theme.accent : Math.hypot(parent.x1, parent.y1) > Gamepad.deadZone ? Theme.accent : Theme.text2
                    x: parent.width / 2 - 7 + parent.x1 * (parent.width / 2 - 8)
                    y: parent.height / 2 - 7 + parent.y1 * (parent.height / 2 - 8)
                }
            }
            component Trigger: Item {
                property string name
                property real amount: 0
                width: 36; height: 64
                Rectangle { anchors.fill: parent; radius: Theme.radiusControl; color: Theme.raised; border.width: 1; border.color: Theme.hairlineStrong }
                Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
                    height: parent.height * parent.amount; radius: Theme.radiusControl; color: Theme.accent }
                Label { anchors.centerIn: parent; text: parent.name.toUpperCase(); size: Theme.sizeCaption; weight: Font.DemiBold; color: parent.amount > 0.5 ? Theme.onAccent : Theme.text2 }
            }
            Item {
                Layout.fillWidth: true
                implicitHeight: body.implicitHeight + Theme.s4 * 2
                RowLayout {
                    id: body
                    anchors.centerIn: parent
                    spacing: Theme.s5
                    // Left: the shoulder, the d-pad, the stick.
                    ColumnLayout {
                        spacing: Theme.s3
                        RowLayout { spacing: Theme.s2
Trigger { name: "lt"; amount: Math.max(test.modelData.triggers.lt || 0, test.modelData.buttons.indexOf("lt") >= 0 ? 1 : 0) }
Key { name: "lb"; text: "LB"; Layout.alignment: Qt.AlignBottom } }
                        GridLayout {
                            columns: 3; rowSpacing: Theme.s1; columnSpacing: Theme.s1
                            Item { width: 36; height: 36 }
Key { name: "up"; text: "▲"; down: test.modelData.hat.y < 0 }
Item { width: 36; height: 36 }
                            Key { name: "left"; text: "◀"; down: test.modelData.hat.x < 0 }
Item { width: 36; height: 36 }
Key { name: "right"; text: "▶"; down: test.modelData.hat.x > 0 }
                            Item { width: 36; height: 36 }
Key { name: "down"; text: "▼"; down: test.modelData.hat.y > 0 }
Item { width: 36; height: 36 }
                        }
                        Stick { x1: test.modelData.axes.x || 0; y1: test.modelData.axes.y || 0; down: test.modelData.buttons.indexOf("ls") >= 0; Layout.alignment: Qt.AlignHCenter }
                    }
                    // Middle: the small buttons.
                    RowLayout {
                        spacing: Theme.s2
                        Layout.alignment: Qt.AlignVCenter
                        Key { name: "select"; text: "Select"; width: 60 }
Key { name: "guide"; text: "Guide"; width: 60 }
Key { name: "start"; text: "Start"; width: 60 }
                    }
                    // Right: the shoulder, the face, the stick.
                    ColumnLayout {
                        spacing: Theme.s3
                        RowLayout { spacing: Theme.s2; Layout.alignment: Qt.AlignRight
Key { name: "rb"; text: "RB"; Layout.alignment: Qt.AlignBottom }
Trigger { name: "rt"; amount: Math.max(test.modelData.triggers.rt || 0, test.modelData.buttons.indexOf("rt") >= 0 ? 1 : 0) } }
                        GridLayout {
                            columns: 3; rowSpacing: Theme.s1; columnSpacing: Theme.s1
                            Item { width: 36; height: 36 }
Key { name: "y"; text: page.faceLabel(test.modelData.kind, "y") }
Item { width: 36; height: 36 }
                            Key { name: "x"; text: page.faceLabel(test.modelData.kind, "x") }
Item { width: 36; height: 36 }
Key { name: "b"; text: page.faceLabel(test.modelData.kind, "b") }
                            Item { width: 36; height: 36 }
Key { name: "a"; text: page.faceLabel(test.modelData.kind, "a") }
Item { width: 36; height: 36 }
                        }
                        Stick { x1: test.modelData.axes.rx || 0; y1: test.modelData.axes.ry || 0; down: test.modelData.buttons.indexOf("rs") >= 0; Layout.alignment: Qt.AlignHCenter }
                    }
                }
            }
        }
    }

    SettingsGroup {
        heading: "Sticks"
        SettingsRow {
            label: "Dead zone"
            description: "How far a stick moves before Isle takes it as a direction. The inner ring above."
            RowLayout {
                spacing: Theme.s3
                Label { text: Math.round(Gamepad.deadZone * 100) + "%"; size: Theme.sizeSmall; color: Theme.text2 }
                Slider { implicitWidth: 180; value: (Gamepad.deadZone - 0.1) / 0.8; onMoved: f => Prefs.p.padDeadZone = Math.round((0.1 + f * 0.8) * 20) / 20 }
            }
        }
    }
}
