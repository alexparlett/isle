import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The form for a text widget of the user's own: a command or a file, an interval, and how to show it.
// Blank for a new one, or filled from the manifest being edited; Save writes the manifest.
ColumnLayout {
    id: form
    property string editingId: ""
    property string fName: ""
    property string fCommand: ""
    property string fFile: ""
    property int fInterval: 30
    property string fView: "text"
    property string fUnit: ""
    property string fMax: "100"
    property string fGlyph: "terminal"
    property string fSize: "3x1"
    signal done
    readonly property var glyphs: [["terminal", "Terminal"], ["activity", "Activity"], ["cpu", "Processor"], ["hard-drive", "Drive"], ["clock", "Clock"], ["timer", "Timer"], ["globe", "Globe"], ["wifi", "Wi-Fi"], ["shield", "Shield"], ["package", "Package"], ["target", "Target"], ["mail", "Mail"], ["music", "Music"], ["sun", "Sun"], ["rocket", "Rocket"], ["list", "List"]]
    readonly property bool valid: fName.trim() !== "" && (fCommand.trim() !== "" || fFile.trim() !== "")

    function startNew() { editingId = ""; fName = ""; fCommand = ""; fFile = ""; fInterval = 30; fView = "text"; fUnit = ""; fMax = "100"; fGlyph = "terminal"; fSize = "3x1"; }
    function startEdit(m) {
        editingId = m.id; fName = m.name; fCommand = m.source.command || ""; fFile = m.source.file || ""; fInterval = Number(m.source.interval) || 30;
        fView = m.view || "text"; fUnit = m.unit || ""; fMax = String(m.max || 100); fGlyph = m.glyph || "terminal"; fSize = m.default || "3x1";
    }
    function save() {
        const id = editingId || Widgets.slug(fName);
        const m = { id: id, name: fName.trim() || id, glyph: fGlyph, category: "Yours",
                    source: fFile.trim() ? { file: fFile.trim(), interval: fInterval } : { command: fCommand, interval: fInterval },
                    view: fView, sizes: ["3x1", "3x2", "6x1", "6x2"], default: fSize };
        if (fUnit.trim()) m.unit = fUnit.trim();
        if (fView === "gauge") m.max = Number(fMax) || 100;
        Widgets.saveUser(m);
        done();
    }

    spacing: Theme.s2
    Label { text: form.editingId ? "Edit " + form.fName : "New text widget"; size: Theme.sizeHeading; weight: Font.DemiBold }
    Label { text: "A command's output, or a file's contents, on a card."; size: Theme.sizeCaption; color: Theme.text3; wrapMode: Text.WordWrap; Layout.fillWidth: true }
    Label { text: "Name"; size: Theme.sizeCaption; color: Theme.text2; Layout.topMargin: Theme.s1 }
    Field { id: nameField; Layout.fillWidth: true; implicitHeight: 32; placeholder: "Uptime"; text: form.fName; onTextChanged: form.fName = text; next: cmdField }
    Label { text: "Command, run with sh"; size: Theme.sizeCaption; color: Theme.text2 }
    Field { id: cmdField; Layout.fillWidth: true; implicitHeight: 32; placeholder: "uptime -p"; text: form.fCommand; onTextChanged: form.fCommand = text; input.font.family: Theme.fontMono; next: fileField }
    Label { text: "Or a file to read"; size: Theme.sizeCaption; color: Theme.text2 }
    Field { id: fileField; Layout.fillWidth: true; implicitHeight: 32; placeholder: "/sys/class/thermal/thermal_zone0/temp"; text: form.fFile; onTextChanged: form.fFile = text; input.font.family: Theme.fontMono }
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.s2
        ColumnLayout {
            spacing: 4
            Label { text: "Every"; size: Theme.sizeCaption; color: Theme.text2 }
            NumberField { value: form.fInterval; from: 1; to: 3600; unit: "s"; onCommitted: v => form.fInterval = v }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4
            Label { text: "Show as"; size: Theme.sizeCaption; color: Theme.text2 }
            Dropdown { listWidth: 180; options: [["text", "Text"], ["number", "Number"], ["gauge", "Gauge"], ["sparkline", "Sparkline"], ["list", "List"]]; value: form.fView; onPicked: v => form.fView = v }
        }
    }
    Label {
        text: form.fView === "text" ? "The output as it is." : form.fView === "number" ? "The first number in the output, large." : form.fView === "gauge" ? "The first number as a bar against a maximum." : form.fView === "sparkline" ? "The first number, charted over the readings so far." : "A row per line; label: value splits into two columns."
        size: Theme.sizeCaption; color: Theme.text3; wrapMode: Text.WordWrap; Layout.fillWidth: true
    }
    RowLayout {
        visible: form.fView !== "text" && form.fView !== "list"
        Layout.fillWidth: true
        spacing: Theme.s2
        ColumnLayout {
            spacing: 4
            Label { text: "Unit"; size: Theme.sizeCaption; color: Theme.text2 }
            Field { implicitWidth: 90; implicitHeight: 32; placeholder: "%"; text: form.fUnit; onTextChanged: form.fUnit = text }
        }
        ColumnLayout {
            visible: form.fView === "gauge"
            spacing: 4
            Label { text: "Maximum"; size: Theme.sizeCaption; color: Theme.text2 }
            Field { implicitWidth: 90; implicitHeight: 32; placeholder: "100"; text: form.fMax; onTextChanged: form.fMax = text }
        }
        Item { Layout.fillWidth: true }
    }
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.s2
        ColumnLayout {
            spacing: 4
            Label { text: "Glyph"; size: Theme.sizeCaption; color: Theme.text2 }
            Dropdown { listWidth: 160; maxRows: 10; options: form.glyphs; value: form.fGlyph; onPicked: v => form.fGlyph = v }
        }
        ColumnLayout {
            spacing: 4
            Label { text: "Size"; size: Theme.sizeCaption; color: Theme.text2 }
            Dropdown { listWidth: 140; options: [["3x1", "3 × 1"], ["3x2", "3 × 2"], ["6x1", "6 × 1"], ["6x2", "6 × 2"]]; value: form.fSize; onPicked: v => form.fSize = v }
        }
        Item { Layout.fillWidth: true }
    }
    Item { Layout.fillHeight: true }
    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.s1
        Button { text: "Open the folder"; variant: "text"; onClicked: Widgets.openUserDir() }
        Item { Layout.fillWidth: true }
        Button { text: "Cancel"; variant: "text"; onClicked: form.done() }
        Button { text: form.editingId ? "Save" : "Create"; variant: "accent"; enabled: form.valid; onClicked: form.save() }
    }
}
