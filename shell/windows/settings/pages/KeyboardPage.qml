import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Keyboard"
    subtitle: "Each keyboard gets a profile; layouts, keys and repeat are set here. The shortcuts have their own page."

    Component.onCompleted: Keyboard.refreshDevices()

    // Each keyboard as Plasma's Hardware tab has it: what it is, its XKB model (guessed from the vendor and the
    // first layout, or chosen), and the profile its chords follow.
    SettingsGroup {
        heading: "Keyboards"
        Repeater {
            model: Keyboard.groups
            SettingsRow {
                id: devRow
                required property var modelData
                readonly property var hw: Keyboard.hardwareFor(modelData)
                readonly property bool guessed: !(Prefs.p.keyboardModels || {})[modelData.name]
                label: hw && hw.vendor && modelData.label.toLowerCase().indexOf(hw.vendor.toLowerCase()) < 0 ? hw.vendor + " " + modelData.label : modelData.label
                description: [hw ? hw.bus : "", hw ? hw.size : "", modelData.profile === "mac" ? "Cmd reaches apps as Ctrl; the shell's chords stay Cmd" : "keys passed through as is"].filter(Boolean).join(" · ")
                RowLayout {
                    spacing: Theme.s2
                    Dropdown {
                        listWidth: 280; maxRows: 12
                        options: Xkb.models.map(m => [m.name, m.description + (m.vendor && m.vendor !== "Generic" ? "  ·  " + m.vendor : "")])
                        value: Keyboard.modelFor(devRow.modelData)
                        onPicked: v => Keyboard.setGroupModel(devRow.modelData, v)
                    }
                    Label { text: devRow.guessed ? "guessed" : ""; size: Theme.sizeCaption; color: Theme.text3; Layout.preferredWidth: 52 }
                    Rectangle {
                        height: 28; radius: Theme.radiusChip; color: Theme.pressed
                        width: segRow.implicitWidth + 4
                        RowLayout {
                            id: segRow
                            anchors.centerIn: parent
                            spacing: 2
                            Repeater {
                                model: ["win", "mac"]
                                Rectangle {
                                    required property string modelData
                                    readonly property bool sel: modelData === devRow.modelData.profile
                                    implicitHeight: 24; implicitWidth: segLabel.implicitWidth + Theme.s3 * 2
                                    radius: Theme.radiusChip - 2
                                    color: sel ? Theme.raised : "transparent"
                                    Label { id: segLabel; anchors.centerIn: parent; text: modelData === "win" ? "Windows" : "Mac"; size: Theme.sizeSmall; weight: Font.DemiBold; color: parent.sel ? Theme.text : Theme.text3 }
                                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Keyboard.setGroupProfile(devRow.modelData, modelData) }
                                }
                            }
                        }
                    }
                    Label { text: Prefs.p.keyboardProfiles[modelData.members[0]] ? "" : "auto"; size: Theme.sizeCaption; color: Theme.text3; Layout.preferredWidth: 30 }
                }
            }
        }
        SettingsRow { visible: Keyboard.groups.length === 0; label: "No keyboards seen"; description: "The compositor reports none yet." }
        SettingsRow {
            label: "NumLock at start"
            description: "The keypad types numbers when the session begins."
            Toggle { checked: Input.get("numlock", false); onToggled: v => Input.set("numlock", v) }
        }
        SettingsRow {
            label: "Test area"
            description: "Type here to try the layout, model and options."
            Field { implicitWidth: 300; implicitHeight: 32; placeholder: "Type here" }
        }
    }

    // Layouts the way Plasma lays them out: a table of the layouts in use, buttons that act on the selected
    // row, and a picker with the layouts on the left and the chosen one's variants on the right.
    property int selectedLayout: 0
    property bool adding: false
    property string pickQuery: ""
    property string pickLayout: ""
    property string pickVariant: ""
    readonly property var pickLayouts: {
        const q = pickQuery.trim().toLowerCase();
        return Xkb.layouts.filter(l => l.name !== "custom" && (!q || l.description.toLowerCase().indexOf(q) >= 0 || l.name.indexOf(q) >= 0));
    }
    function moveLayout(from, to) {
        if (to < 0 || to >= Input.layouts.length) return;
        const l = Input.layouts.slice();
        const [x] = l.splice(from, 1);
        l.splice(to, 0, x);
        Input.setLayouts(l);
        selectedLayout = to;
    }
    SettingsGroup {
        heading: "Layouts"
        Card {
            Layout.fillWidth: true
            padding: 0
            ColumnLayout {
                anchors { left: parent.left; right: parent.right }
                spacing: 0
                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: Theme.s2
                    Layout.leftMargin: Theme.s3
                    Layout.rightMargin: Theme.s3
                    spacing: Theme.s3
                    Label { text: "Layout"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3; Layout.fillWidth: true }
                    Label { text: "Variant"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3; Layout.preferredWidth: 240 }
                    Label { text: "Code"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3; Layout.preferredWidth: 70 }
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.hairline }
                Repeater {
                    model: Input.layouts
                    Rectangle {
                        required property var modelData
                        required property int index
                        readonly property bool sel: page.selectedLayout === index
                        Layout.fillWidth: true
                        implicitHeight: 36
                        color: sel ? Theme.pressed : layoutArea.containsMouse ? Qt.alpha(Theme.text, 0.04) : "transparent"
                        RowLayout {
                            anchors { fill: parent; leftMargin: Theme.s3; rightMargin: Theme.s3 }
                            spacing: Theme.s3
                            Label { text: Xkb.layoutName(modelData.layout) + (index === 0 ? "  ·  default" : ""); elide: Text.ElideRight; Layout.fillWidth: true }
                            Label { text: modelData.variant ? Xkb.variantName(modelData.layout, modelData.variant) : "Default"; color: Theme.text2; elide: Text.ElideRight; Layout.preferredWidth: 240 }
                            Label { text: modelData.layout + (modelData.variant ? "(" + modelData.variant + ")" : ""); mono: true; size: Theme.sizeSmall; color: Theme.text3; Layout.preferredWidth: 70 }
                        }
                        MouseArea { id: layoutArea; anchors.fill: parent; hoverEnabled: true; onClicked: page.selectedLayout = index }
                    }
                }
                Rectangle { Layout.fillWidth: true; height: 1; color: Theme.hairline }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.margins: Theme.s2
                    spacing: Theme.s1
                    Button { text: "Add…"; variant: "raised"; onClicked: { page.adding = !page.adding; page.pickQuery = ""; page.pickLayout = ""; page.pickVariant = ""; } }
                    Button { text: "Remove"; variant: "text"; enabled: Input.layouts.length > 1; onClicked: { Input.setLayouts(Input.layouts.filter((x, i) => i !== page.selectedLayout)); page.selectedLayout = Math.max(0, Math.min(page.selectedLayout, Input.layouts.length - 1)); } }
                    Button { text: "Move up"; variant: "text"; enabled: page.selectedLayout > 0; onClicked: page.moveLayout(page.selectedLayout, page.selectedLayout - 1) }
                    Button { text: "Move down"; variant: "text"; enabled: page.selectedLayout < Input.layouts.length - 1; onClicked: page.moveLayout(page.selectedLayout, page.selectedLayout + 1) }
                    Item { Layout.fillWidth: true }
                    Label { text: "The first layout is the one at start; the others are a switch away."; size: Theme.sizeCaption; color: Theme.text3 }
                }
            }
        }
        Card {
            visible: page.adding
            Layout.fillWidth: true
            padding: Theme.s3
            ColumnLayout {
                anchors { left: parent.left; right: parent.right }
                spacing: Theme.s2
                Field {
                    Layout.fillWidth: true
                    implicitHeight: 32
                    glyph: "search"
                    placeholder: "Search layouts"
                    text: page.pickQuery
                    onTextChanged: page.pickQuery = text
                    onVisibleChanged: if (visible) input.forceActiveFocus()
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.s3
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 280
                        radius: Theme.radiusControl; color: Theme.pressed; border.width: 1; border.color: Theme.hairline
                        ListView {
                            id: layoutList
                            anchors { fill: parent; margins: Theme.s1 }
                            clip: true
                            model: page.pickLayouts
                            boundsBehavior: Flickable.StopAtBounds
                            delegate: Rectangle {
                                required property var modelData
                                readonly property bool sel: page.pickLayout === modelData.name
                                width: layoutList.width
                                height: 30
                                radius: Theme.radiusControl
                                color: sel ? Theme.raised : pickArea.containsMouse ? Qt.alpha(Theme.text, 0.04) : "transparent"
                                RowLayout {
                                    anchors { fill: parent; leftMargin: Theme.s2; rightMargin: Theme.s2 }
                                    Label { text: modelData.description; elide: Text.ElideRight; Layout.fillWidth: true }
                                    Label { text: modelData.name; mono: true; size: Theme.sizeSmall; color: Theme.text3 }
                                }
                                MouseArea { id: pickArea; anchors.fill: parent; hoverEnabled: true; onClicked: { page.pickLayout = modelData.name; page.pickVariant = ""; } }
                            }
                        }
                        Scrollbar { target: layoutList; anchors { top: parent.top; bottom: parent.bottom; right: parent.right } }
                    }
                    Rectangle {
                        Layout.preferredWidth: 300
                        Layout.preferredHeight: 280
                        radius: Theme.radiusControl; color: Theme.pressed; border.width: 1; border.color: Theme.hairline
                        ListView {
                            id: variantList
                            anchors { fill: parent; margins: Theme.s1 }
                            clip: true
                            model: page.pickLayout ? [{ name: "", description: "Default" }].concat(Xkb.variantsOf(page.pickLayout)) : []
                            boundsBehavior: Flickable.StopAtBounds
                            delegate: Rectangle {
                                required property var modelData
                                readonly property bool sel: page.pickVariant === modelData.name
                                width: variantList.width
                                height: 30
                                radius: Theme.radiusControl
                                color: sel ? Theme.raised : varArea.containsMouse ? Qt.alpha(Theme.text, 0.04) : "transparent"
                                Label { anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: Theme.s2; rightMargin: Theme.s2 } text: modelData.description; elide: Text.ElideRight }
                                MouseArea { id: varArea; anchors.fill: parent; hoverEnabled: true; onClicked: page.pickVariant = modelData.name }
                            }
                        }
                        Label { anchors.centerIn: parent; visible: !page.pickLayout; text: "Pick a layout for its variants"; size: Theme.sizeSmall; color: Theme.text3 }
                        Scrollbar { target: variantList; anchors { top: parent.top; bottom: parent.bottom; right: parent.right } }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.s1
                    Label { text: page.pickLayout ? Xkb.layoutName(page.pickLayout) + (page.pickVariant ? "  ·  " + Xkb.variantName(page.pickLayout, page.pickVariant) : "") : ""; color: Theme.text2; Layout.fillWidth: true; elide: Text.ElideRight }
                    Button { text: "Cancel"; variant: "text"; onClicked: page.adding = false }
                    Button { text: "Add"; variant: "accent"; enabled: page.pickLayout !== ""; onClicked: { Input.setLayouts(Input.layouts.concat([{ layout: page.pickLayout, variant: page.pickVariant }])); page.selectedLayout = Input.layouts.length - 1; page.adding = false; } }
                }
            }
        }
        SettingsRow {
            visible: Input.layouts.length > 1
            label: "Switch layouts with"
            Dropdown { listWidth: 260; maxRows: 12; options: [["", "Nothing"]].concat(Xkb.optionsOf("grp").map(o => [o.name, o.description])); value: (Input.get("optionSet", []) || []).find(o => o.indexOf("grp:") === 0) || ""; onPicked: v => { for (const o of Xkb.optionsOf("grp")) Input.setOption(o.name, false); if (v) Input.setOption(v, true); } }
        }
    }

    SettingsGroup {
        heading: "Keys"
        SettingsRow { label: "Caps Lock is Control"; Toggle { checked: Input.hasOption("ctrl:nocaps"); onToggled: v => Input.setOption("ctrl:nocaps", v) } }
        SettingsRow { label: "Caps Lock is Escape"; Toggle { checked: Input.hasOption("caps:escape"); onToggled: v => Input.setOption("caps:escape", v) } }
        SettingsRow { label: "Swap Alt and Win"; Toggle { checked: Input.hasOption("altwin:swap_alt_win"); onToggled: v => Input.setOption("altwin:swap_alt_win", v) } }
        SettingsRow {
            label: "Compose key"
            description: "Types accented and special characters in sequence."
            Dropdown { listWidth: 220; maxRows: 12; options: [["", "None"]].concat(Xkb.optionsOf("Compose key").map(o => [o.name, o.description])); value: (Input.get("optionSet", []) || []).find(o => o.indexOf("compose:") === 0) || ""; onPicked: v => { for (const o of Xkb.optionsOf("Compose key")) Input.setOption(o.name, false); if (v) Input.setOption(v, true); } }
        }
        SettingsRow {
            label: "Other options"
            description: "XKB option names, separated by commas."
            Field { implicitWidth: 260; implicitHeight: 32; placeholder: "terminate:ctrl_alt_bksp"; text: Input.get("options", ""); onAccepted: Input.set("options", text) }
        }
    }

    // Every XKB option group, as Plasma's Advanced tab lists them; one choice per group where the
    // group is a position or a key, any number where it is a set of behaviours.
    property var openGroups: ({})
    readonly property var multiGroups: ({ compat: 1, japan: 1, korean: 1, esperanto: 1, custom: 1, fkeys: 1, solaris: 1 })
    SettingsGroup {
        heading: "Advanced"
        Repeater {
            model: Xkb.groups.filter(g => g.name !== "grp" && g.name !== "Compose key")
            ColumnLayout {
                id: optGroup
                required property var modelData
                readonly property bool open: page.openGroups[modelData.name] === true
                readonly property int chosen: modelData.options.filter(o => Input.hasOption(o.name)).length
                Layout.fillWidth: true
                spacing: 0
                Item {
                    Layout.fillWidth: true
                    implicitHeight: header.implicitHeight
                    SettingsRow {
                        id: header
                        anchors.fill: parent
                        label: optGroup.modelData.description
                        description: optGroup.chosen ? optGroup.modelData.options.filter(o => Input.hasOption(o.name)).map(o => o.description).join(", ") : ""
                        Glyph { name: optGroup.open ? "chevron-down" : "chevron-right"; size: 14; color: Theme.text3 }
                    }
                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { const o = Object.assign({}, page.openGroups); o[optGroup.modelData.name] = !optGroup.open; page.openGroups = o; } }
                }
                Repeater {
                    model: optGroup.open ? optGroup.modelData.options : []
                    SettingsRow {
                        required property var modelData
                        label: modelData.description
                        description: modelData.name
                        Toggle {
                            checked: Input.hasOption(modelData.name)
                            onToggled: v => {
                                if (v && !page.multiGroups[optGroup.modelData.name]) for (const o of optGroup.modelData.options) if (o.name !== modelData.name) Input.setOption(o.name, false);
                                Input.setOption(modelData.name, v);
                            }
                        }
                    }
                }
            }
        }
        SettingsRow {
            label: "Key repeat"
            description: "Delay before a held key repeats, then repeats per second."
            RowLayout {
                spacing: Theme.s2
                NumberField { value: Input.get("repeatDelay", 400); from: 100; to: 2000; unit: "ms"; onCommitted: v => Input.set("repeatDelay", v) }
                NumberField { value: Input.get("repeatRate", 25); from: 1; to: 100; unit: "/s"; onCommitted: v => Input.set("repeatRate", v) }
            }
        }
    }
}
