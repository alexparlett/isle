import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The readings Monitor samples: the ones that matter first (CPU, GPU, drives, board), every sensor on request.
WidgetBase {
    id: root
    title: "Sensors"
    meta: hottest ? Math.round(hottest.value) + "°" : ""

    Component.onCompleted: Monitor.listeners++
    Component.onDestruction: Monitor.listeners--

    // Temperatures worth a row: one per chip unless everything is asked for, named by what the chip is.
    readonly property var temps: {
        const all = Monitor.temps || [], out = [], seen = {};
        const nameOf = t => t.chip.indexOf("k10temp") === 0 || t.chip.indexOf("coretemp") === 0 || t.chip.indexOf("zenpower") === 0 ? "CPU"
            : t.chip.indexOf("nvme") === 0 ? "Drive" : t.chip.indexOf("amdgpu") === 0 ? "GPU" : t.chip.indexOf("nct") === 0 || t.chip.indexOf("it87") === 0 ? "Board"
            : t.chip.indexOf("spd") === 0 ? "Memory" : t.chip.indexOf("iwlwifi") === 0 ? "Wi-Fi" : t.chip;
        if (Monitor.gpu && Monitor.gpu.temp !== null && Monitor.gpu.temp !== undefined) out.push({ name: "GPU", detail: Monitor.gpu.name, value: Monitor.gpu.temp });
        for (const t of all) {
            const n = nameOf(t);
            if (t.value === null || t.value === undefined || t.value <= 0) continue;
            if (n === "GPU" && out.some(o => o.name === "GPU")) continue;
            if (root.settings.everything) { out.push({ name: n, detail: t.label, value: t.value }); continue; }
            // One row per kind: the hottest reading of the chip.
            const key = n + (n === "Drive" ? t.chip + t.label.slice(0, 0) : "");
            if (!seen[n] || seen[n].value < t.value) seen[n] = { name: n, detail: t.label, value: t.value };
        }
        if (!root.settings.everything) for (const k of ["CPU", "GPU", "Board", "Drive", "Memory", "Wi-Fi"]) if (seen[k]) out.push(seen[k]);
        return out;
    }
    readonly property var hottest: temps.reduce((m, t) => (!m || t.value > m.value) ? t : m, null)
    readonly property var fans: {
        const out = (Monitor.fans || []).filter(f => f.rpm > 0).map(f => ({ name: f.label.replace(/^fan(\d+)$/, "Fan $1"), rpm: f.rpm, pct: f.pct }));
        if (Monitor.gpu && Monitor.gpu.fan > 0) out.unshift({ name: "GPU fan", rpm: null, pct: Monitor.gpu.fan });
        return out;
    }
    function tone(v) { return v >= 85 ? Theme.danger : v >= 70 ? Theme.warn : Theme.text; }

    ColumnLayout {
        anchors.fill: parent
        spacing: 2
        Repeater {
            model: root.temps.slice(0, Math.max(2, root.rows * 3 - (root.settings.fans !== false && root.fans.length ? 2 : 0)))
            RowLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: Theme.s2
                Glyph { name: "thermometer"; size: 13; color: root.tone(modelData.value) }
                Label { text: modelData.name; size: Theme.sizeSmall; Layout.preferredWidth: 56 }
                Label { text: modelData.detail; size: Theme.sizeCaption; color: Theme.text3; elide: Text.ElideRight; Layout.fillWidth: true; visible: root.cols >= 3 }
                Label { text: Math.round(modelData.value) + "°"; size: Theme.sizeSmall; tabular: true; weight: Font.DemiBold; color: root.tone(modelData.value) }
            }
        }
        Repeater {
            model: root.settings.fans !== false ? root.fans.slice(0, Math.max(1, root.rows)) : []
            RowLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: Theme.s2
                Glyph { name: "fan"; size: 13; color: Theme.text2 }
                Label { text: modelData.name; size: Theme.sizeSmall; Layout.preferredWidth: 56 }
                Item { Layout.fillWidth: true }
                Label { text: modelData.rpm !== null ? modelData.rpm + " rpm" : Math.round(modelData.pct) + "%"; size: Theme.sizeSmall; tabular: true; color: Theme.text2 }
            }
        }
        Item { Layout.fillHeight: true }
        Label { visible: root.temps.length === 0; Layout.alignment: Qt.AlignHCenter; text: "No sensors found"; color: Theme.text3; size: Theme.sizeSmall }
    }
}
