import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The widget library, and the user's own text widgets: a command or a file read on an interval, shown as
// text, a number, a gauge, a sparkline or a list. A manifest each, in ~/.config/isle/widgets/<id>/.
SettingsPage {
    id: page
    title: "Widgets"
    subtitle: "What the dashboard can hold. Edit the dashboard (E) to place them; make your own from a command here."

    Component.onCompleted: Widgets.rescan()

    // The form: blank for a new widget, or the manifest being edited.
    property bool composing: false
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
    function startNew() { editingId = ""; fName = ""; fCommand = ""; fFile = ""; fInterval = 30; fView = "text"; fUnit = ""; fMax = "100"; fGlyph = "terminal"; fSize = "3x1"; composing = true; }
    function startEdit(m) {
        editingId = m.id; fName = m.name; fCommand = m.source.command || ""; fFile = m.source.file || ""; fInterval = Number(m.source.interval) || 30;
        fView = m.view || "text"; fUnit = m.unit || ""; fMax = String(m.max || 100); fGlyph = m.glyph || "terminal"; fSize = m.default || "3x1"; composing = true;
    }
    function save() {
        const id = editingId || Widgets.slug(fName);
        const m = { id: id, name: fName.trim() || id, glyph: fGlyph, category: "Yours",
                    source: fFile.trim() ? { file: fFile.trim(), interval: fInterval } : { command: fCommand, interval: fInterval },
                    view: fView, sizes: ["3x1", "3x2", "6x1", "6x2"], default: fSize };
        if (fUnit.trim()) m.unit = fUnit.trim();
        if (fView === "gauge") m.max = Number(fMax) || 100;
        Widgets.saveUser(m);
        composing = false;
    }
    readonly property var glyphs: [["terminal", "Terminal"], ["activity", "Activity"], ["cpu", "Processor"], ["hard-drive", "Drive"], ["clock", "Clock"], ["timer", "Timer"], ["globe", "Globe"], ["wifi", "Wi-Fi"], ["shield", "Shield"], ["package", "Package"], ["target", "Target"], ["mail", "Mail"], ["music", "Music"], ["sun", "Sun"], ["rocket", "Rocket"], ["list", "List"]]

    SettingsGroup {
        heading: "Yours"
        Repeater {
            model: Widgets.library.filter(m => m.user)
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: (modelData.source ? (modelData.source.file ? modelData.source.file : modelData.source.command) + " · every " + (modelData.source.interval || 30) + " s · " + (modelData.view || "text") : "QML widget in " + modelData.dir)
                             + (Widgets.placed(modelData.id) ? " · on the dashboard" : "")
                glyph: modelData.glyph || "terminal"
                RowLayout {
                    spacing: Theme.s1
                    Button { text: "Edit"; variant: "text"; visible: !!modelData.source; onClicked: page.startEdit(modelData) }
                    Button { text: "Delete"; variant: "text"; onClicked: Widgets.deleteUser(modelData.id) }
                }
            }
        }
        SettingsRow {
            visible: !page.composing
            label: Widgets.library.some(m => m.user) ? "Another one" : "Nothing of yours yet"
            description: "A command's output, or a file's contents, on the dashboard."
            RowLayout {
                spacing: Theme.s1
                Button { text: "Open the folder"; variant: "text"; onClicked: Widgets.openUserDir() }
                Button { text: "New text widget"; glyph: "plus"; variant: "accent"; onClicked: page.startNew() }
            }
        }
    }

    SettingsGroup {
        visible: page.composing
        heading: page.editingId ? "Editing " + page.fName : "New text widget"
        SettingsRow { label: "Name"; Field { implicitWidth: 300; implicitHeight: 32; placeholder: "Uptime"; text: page.fName; onTextChanged: page.fName = text } }
        SettingsRow {
            label: "Command"
            description: "Run with sh; its output is what shows. Leave empty to read a file instead."
            Field { implicitWidth: 300; implicitHeight: 32; placeholder: "uptime -p"; text: page.fCommand; onTextChanged: page.fCommand = text; input.font.family: Theme.fontMono }
        }
        SettingsRow { label: "File"; description: "Read instead of running a command."; Field { implicitWidth: 300; implicitHeight: 32; placeholder: "/sys/class/thermal/thermal_zone0/temp"; text: page.fFile; onTextChanged: page.fFile = text; input.font.family: Theme.fontMono } }
        SettingsRow { label: "Every"; NumberField { value: page.fInterval; from: 1; to: 3600; unit: "s"; onCommitted: v => page.fInterval = v } }
        SettingsRow {
            label: "Show as"
            description: page.fView === "text" ? "The output as it is" : page.fView === "number" ? "The first number in the output, large" : page.fView === "gauge" ? "The first number as a bar against a maximum" : page.fView === "sparkline" ? "The first number, charted over the readings so far" : "A row per line; label: value splits into two columns"
            Dropdown { listWidth: 200; options: [["text", "Text"], ["number", "Number"], ["gauge", "Gauge"], ["sparkline", "Sparkline"], ["list", "List"]]; value: page.fView; onPicked: v => page.fView = v }
        }
        SettingsRow { visible: page.fView !== "text" && page.fView !== "list"; label: "Unit"; Field { implicitWidth: 120; implicitHeight: 32; placeholder: "%"; text: page.fUnit; onTextChanged: page.fUnit = text } }
        SettingsRow { visible: page.fView === "gauge"; label: "Maximum"; Field { implicitWidth: 120; implicitHeight: 32; placeholder: "100"; text: page.fMax; onTextChanged: page.fMax = text } }
        SettingsRow { label: "Glyph"; Dropdown { listWidth: 180; maxRows: 10; options: page.glyphs; value: page.fGlyph; onPicked: v => page.fGlyph = v } }
        SettingsRow { label: "Size"; description: "Cells across by cells down; the card can be resized later."; Dropdown { listWidth: 160; options: [["3x1", "3 × 1"], ["3x2", "3 × 2"], ["6x1", "6 × 1"], ["6x2", "6 × 2"]]; value: page.fSize; onPicked: v => page.fSize = v } }
        SettingsRow {
            label: ""
            RowLayout {
                spacing: Theme.s1
                Button { text: "Cancel"; variant: "text"; onClicked: page.composing = false }
                Button { text: page.editingId ? "Save" : "Create"; variant: "accent"; enabled: page.fName.trim() !== "" && (page.fCommand.trim() !== "" || page.fFile.trim() !== ""); onClicked: page.save() }
            }
        }
    }

    SettingsGroup {
        heading: "Built in"
        Repeater {
            model: Widgets.library.filter(m => !m.user)
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: (modelData.description || "") + "  ·  " + modelData.category + "  ·  " + (modelData.sizes || []).map(s => s.replace("x", "×")).join(", ")
                glyph: modelData.glyph || "layout-grid"
                Label { text: Widgets.placed(modelData.id) ? "On the dashboard" + (Widgets.placed(modelData.id) > 1 ? " ×" + Widgets.placed(modelData.id) : "") : ""; size: Theme.sizeCaption; color: Theme.accent }
            }
        }
    }
}
