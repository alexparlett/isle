import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

WidgetBase {
    title: "Devices"

    // Batteries UPower knows, then connected Bluetooth devices UPower does not.
    readonly property var rows: {
        const out = Power.peripherals.map(d => ({ glyph: Power.glyphFor(d), name: Power.labelFor(d), sub: Power.percent(d) + "%", low: Power.low(d) }));
        for (const b of Bluetooth.connected) {
            if (out.find(r => r.name === b.name)) continue;
            out.push({ glyph: Bluetooth.glyphFor(b), name: Bluetooth.nameOf(b), sub: b.batteryAvailable ? Math.round(b.battery * 100) + "%" : "Connected", low: b.batteryAvailable && b.battery < 0.2 });
        }
        return out;
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Repeater {
            model: rows
            RowLayout {
                required property var modelData
                Layout.fillWidth: true
                implicitHeight: 28
                spacing: Theme.s2 + 2
                Glyph { name: modelData.glyph; size: 14 }
                Label { text: modelData.name; size: Theme.sizeSmall; Layout.fillWidth: true }
                Label { text: modelData.sub; size: Theme.sizeCaption; tabular: true; color: modelData.low ? Theme.warn : Theme.text2 }
            }
        }
        Item { Layout.fillHeight: true }
    }

    Label { anchors.centerIn: parent; visible: rows.length === 0; text: "Nothing connected"; color: Theme.text3 }
}
