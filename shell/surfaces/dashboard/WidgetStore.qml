import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The widget library as a store: cards with a live preview or screenshot, the author and version, and what
// each widget touches; a sheet with the whole listing, its changelog, and the permissions as toggles.
Item {
    id: store
    property bool open: false
    // "installed" is everything the shell has; "yours" the user's own folder; "browse" the registries.
    property string tab: "installed"
    property string query: ""
    property string category: ""
    property string selected: ""
    signal place(string id)
    signal edit(var manifest)
    readonly property var current: selected ? Widgets.manifests[selected] || null : null

    readonly property var rows: {
        const q = query.trim().toLowerCase();
        return Widgets.library.filter(m => tab === "yours" ? m.user : tab === "installed" ? true : false)
            .filter(m => !category || m.category === category)
            .filter(m => !q || m.name.toLowerCase().indexOf(q) >= 0 || (m.description || "").toLowerCase().indexOf(q) >= 0 || (m.author && String(m.author.name || m.author).toLowerCase().indexOf(q) >= 0));
    }
    function authorOf(m) { return m && m.author ? (m.author.name || String(m.author)) : ""; }
    function authorUrl(m) { return m && m.author && m.author.url ? m.author.url : ""; }
    function needsTrust(m) { return !!(m && m.restrictedIssues && m.restrictedIssues.length); }

    visible: open
    anchors.fill: parent
    Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.5) }
    MouseArea { anchors.fill: parent; onClicked: store.open = false }

    Glass {
        anchors.centerIn: parent
        width: Math.min(parent.width - Theme.s5 * 2, 1180)
        height: parent.height - Theme.s5 * 2
        radius: Theme.radiusPanel
        MouseArea { anchors.fill: parent }

        RowLayout {
            anchors { fill: parent; margins: Theme.s4 }
            spacing: Theme.s4

            // --- the gallery ------------------------------------------------------------------
            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: Theme.s3
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.s3
                    Label { text: "Widget library"; size: Theme.sizeTitle; weight: Font.DemiBold }
                    Item { Layout.fillWidth: true }
                    Repeater {
                        model: [["installed", "Installed"], ["yours", "Yours"], ["browse", "Browse"]]
                        Rectangle {
                            required property var modelData
                            readonly property bool sel: store.tab === modelData[0]
                            implicitHeight: 30; implicitWidth: tabLabel.implicitWidth + Theme.s3 * 2
                            radius: 15; color: sel ? Theme.raised : "transparent"; border.width: 1; border.color: sel ? Theme.hairlineStrong : "transparent"
                            Label { id: tabLabel; anchors.centerIn: parent; text: modelData[1]; size: Theme.sizeSmall; weight: Font.DemiBold; color: parent.sel ? Theme.text : Theme.text2 }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { store.tab = modelData[0]; store.selected = ""; } }
                        }
                    }
                    Rectangle {
                        implicitWidth: 28; implicitHeight: 28; radius: 14; color: closeArea.containsMouse ? Theme.pressed : "transparent"
                        Glyph { anchors.centerIn: parent; name: "x"; size: 12; color: Theme.text2 }
                        MouseArea { id: closeArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: store.open = false }
                    }
                }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.s2
                    Field { Layout.preferredWidth: 260; implicitHeight: 32; glyph: "search"; placeholder: "Search widgets"; size: Theme.sizeSmall; onTextChanged: store.query = text }
                    Repeater {
                        model: [""].concat(Widgets.categoryOrder)
                        Rectangle {
                            required property string modelData
                            readonly property bool sel: store.category === modelData
                            implicitHeight: 26; implicitWidth: chipLabel.implicitWidth + Theme.s3 * 2
                            radius: 13; color: sel ? Theme.accent : Theme.raised; border.width: 1; border.color: sel ? "transparent" : Theme.hairline
                            Label { id: chipLabel; anchors.centerIn: parent; text: modelData || "All"; size: Theme.sizeCaption; weight: Font.DemiBold; color: parent.sel ? Theme.onAccent : Theme.text2 }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: store.category = modelData }
                        }
                    }
                    Item { Layout.fillWidth: true }
                    Label { text: store.rows.length + (store.rows.length === 1 ? " widget" : " widgets"); size: Theme.sizeCaption; color: Theme.text3 }
                }

                GridView {
                    id: grid
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    clip: true
                    cellWidth: Math.floor(width / Math.max(1, Math.floor(width / 280)))
                    cellHeight: 236
                    model: store.rows
                    boundsBehavior: Flickable.StopAtBounds
                    delegate: Item {
                        id: cell
                        required property var modelData
                        width: grid.cellWidth; height: grid.cellHeight
                        Rectangle {
                            anchors { fill: parent; margins: Theme.s1 + 2 }
                            radius: Theme.radiusCard
                            color: store.selected === cell.modelData.id ? Theme.pressed : cardArea.containsMouse ? Qt.alpha(Theme.text, 0.05) : Theme.raised
                            border.width: store.selected === cell.modelData.id ? 2 : 1
                            border.color: store.selected === cell.modelData.id ? Theme.accent : Theme.hairline
                            ColumnLayout {
                                anchors { fill: parent; margins: Theme.s3 }
                                spacing: Theme.s2
                                WidgetPreview { Layout.fillWidth: true; Layout.preferredHeight: 128; manifest: cell.modelData }
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.s2
                                    Glyph { name: cell.modelData.glyph || "layout-grid"; size: 14; color: Theme.text2 }
                                    Label { text: cell.modelData.name; weight: Font.DemiBold; size: Theme.sizeSmall; elide: Text.ElideRight; Layout.fillWidth: true }
                                    Label { visible: !!cell.modelData.version; text: "v" + cell.modelData.version; size: Theme.sizeCaption; color: Theme.text3; tabular: true }
                                }
                                Label { text: (store.authorOf(cell.modelData) || (cell.modelData.user ? "You" : "Isle")) + (cell.modelData.description ? "  ·  " + cell.modelData.description : ""); size: Theme.sizeCaption; color: Theme.text3; elide: Text.ElideRight; Layout.fillWidth: true }
                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 4
                                    Repeater {
                                        model: (cell.modelData.permissions || []).slice(0, 4)
                                        Rectangle {
                                            required property string modelData
                                            implicitHeight: 18; implicitWidth: pl.implicitWidth + 12
                                            radius: 9; color: Qt.alpha(Theme.accent, 0.14)
                                            Label { id: pl; anchors.centerIn: parent; text: modelData.replace(".write", " ✎"); size: 10; weight: Font.DemiBold; color: Theme.accent }
                                        }
                                    }
                                    Rectangle {
                                        visible: cell.modelData.trust === true || store.needsTrust(cell.modelData)
                                        implicitHeight: 18; implicitWidth: tl.implicitWidth + 12
                                        radius: 9; color: Qt.alpha(cell.modelData.trust === true ? Theme.warn : Theme.danger, 0.16)
                                        Label { id: tl; anchors.centerIn: parent; text: cell.modelData.trust === true ? "trusted" : "needs trust"; size: 10; weight: Font.DemiBold; color: cell.modelData.trust === true ? Theme.warn : Theme.danger }
                                    }
                                    Item { Layout.fillWidth: true }
                                    Label { visible: Widgets.placed(cell.modelData.id) > 0; text: "on"; size: Theme.sizeCaption; color: Theme.accent }
                                    Glyph { visible: Widgets.placed(cell.modelData.id) > 0; name: "check"; size: 11; color: Theme.accent }
                                }
                            }
                            MouseArea { id: cardArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: store.selected = cell.modelData.id; onDoubleClicked: store.place(cell.modelData.id) }
                        }
                    }
                }
                Label { visible: store.rows.length === 0; text: store.tab === "browse" ? "No registries yet. Add one in the sheet's Sources." : "Nothing matches"; color: Theme.text3; Layout.alignment: Qt.AlignHCenter }
            }

            // --- the sheet ---------------------------------------------------------------------
            Rectangle {
                visible: store.current !== null
                Layout.preferredWidth: 380
                Layout.fillHeight: true
                radius: Theme.radiusCard
                color: Theme.raised
                border.width: 1; border.color: Theme.hairline
                Flickable {
                    id: sheet
                    anchors { fill: parent; margins: Theme.s3 }
                    clip: true
                    contentHeight: sheetCol.implicitHeight
                    boundsBehavior: Flickable.StopAtBounds
                    ColumnLayout {
                        id: sheetCol
                        width: sheet.width
                        spacing: Theme.s3
                        readonly property var m: store.current
                        WidgetPreview { Layout.fillWidth: true; Layout.preferredHeight: 180; manifest: sheetCol.m; large: true }
                        ColumnLayout {
                            spacing: 2
                            Label { text: sheetCol.m ? sheetCol.m.name : ""; size: Theme.sizeTitle; weight: Font.DemiBold }
                            RowLayout {
                                spacing: Theme.s2
                                Label { text: store.authorOf(sheetCol.m) || (sheetCol.m && sheetCol.m.user ? "You" : "Isle"); size: Theme.sizeSmall; color: store.authorUrl(sheetCol.m) ? Theme.accent : Theme.text2
                                    MouseArea { anchors.fill: parent; enabled: store.authorUrl(sheetCol.m) !== ""; cursorShape: Qt.PointingHandCursor; onClicked: Compositor.exec("xdg-open " + JSON.stringify(store.authorUrl(sheetCol.m))) } }
                                Label { visible: !!(sheetCol.m && sheetCol.m.version); text: "·  v" + (sheetCol.m ? sheetCol.m.version : ""); size: Theme.sizeSmall; color: Theme.text3; tabular: true }
                                Label { visible: !!(sheetCol.m && sheetCol.m.license); text: "·  " + (sheetCol.m ? sheetCol.m.license : ""); size: Theme.sizeSmall; color: Theme.text3 }
                                Label { visible: !!(sheetCol.m && sheetCol.m.category); text: "·  " + (sheetCol.m ? sheetCol.m.category : ""); size: Theme.sizeSmall; color: Theme.text3 }
                            }
                        }
                        Label { visible: !!(sheetCol.m && (sheetCol.m.about || sheetCol.m.description)); text: sheetCol.m ? (sheetCol.m.about || sheetCol.m.description) : ""; size: Theme.sizeSmall; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        // Where it is from.
                        RowLayout {
                            visible: !!(sheetCol.m && (sheetCol.m.source || sheetCol.m.origin || sheetCol.m.homepage))
                            spacing: Theme.s2
                            Glyph { name: "globe"; size: 12; color: Theme.text3 }
                            Label { text: sheetCol.m ? (sheetCol.m.homepage || sheetCol.m.source || sheetCol.m.origin) : ""; size: Theme.sizeCaption; color: Theme.accent; elide: Text.ElideMiddle; Layout.fillWidth: true
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Compositor.exec("xdg-open " + JSON.stringify(sheetCol.m.homepage || sheetCol.m.source || sheetCol.m.origin)) } }
                        }
                        // Sizes.
                        RowLayout {
                            spacing: 4
                            Label { text: "Sizes"; size: Theme.sizeCaption; color: Theme.text3; Layout.rightMargin: Theme.s1 }
                            Repeater {
                                model: sheetCol.m ? (sheetCol.m.sizes || []) : []
                                Rectangle {
                                    required property string modelData
                                    implicitHeight: 20; implicitWidth: sl.implicitWidth + 12
                                    radius: 10; color: Theme.pressed
                                    Label { id: sl; anchors.centerIn: parent; text: modelData.replace("x", " × "); size: Theme.sizeCaption; mono: true }
                                }
                            }
                        }
                        // Permissions, as toggles: the widget gets only what stays on.
                        ColumnLayout {
                            visible: !!(sheetCol.m && sheetCol.m.permissions && sheetCol.m.permissions.length)
                            Layout.fillWidth: true
                            spacing: 2
                            RowLayout {
                                Layout.fillWidth: true
                                Label { text: "Permissions"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3; Layout.fillWidth: true }
                                Label { text: "Reset"; size: Theme.sizeCaption; color: Theme.accent; visible: sheetCol.m && (Prefs.p.widgetGrants || {})[sheetCol.m.id] !== undefined
                                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.resetGrants(sheetCol.m.id) } }
                            }
                            Repeater {
                                model: sheetCol.m ? (sheetCol.m.permissions || []) : []
                                RowLayout {
                                    required property string modelData
                                    Layout.fillWidth: true
                                    spacing: Theme.s2
                                    ColumnLayout {
                                        Layout.fillWidth: true
                                        spacing: 0
                                        Label { text: modelData.replace(".write", " (control)"); size: Theme.sizeSmall; weight: Font.Medium }
                                        Label { text: Widgets.permissionLabel(modelData); size: Theme.sizeCaption; color: Theme.text3; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                    }
                                    Toggle { checked: Widgets.grantsFor(sheetCol.m.id).indexOf(modelData) >= 0; onToggled: v => Widgets.setGrant(sheetCol.m.id, modelData, v) }
                                }
                            }
                        }
                        Label {
                            visible: store.needsTrust(sheetCol.m)
                            text: "Reaches past the sandbox (" + (sheetCol.m ? sheetCol.m.restrictedIssues.join(", ") : "") + "). It runs only with \"trust\": true in its widget.json, and then with the shell's full reach."
                            size: Theme.sizeCaption; color: Theme.danger; wrapMode: Text.WordWrap; Layout.fillWidth: true
                        }
                        // The changelog, folded.
                        ColumnLayout {
                            visible: !!(sheetCol.m && sheetCol.m.changelog)
                            Layout.fillWidth: true
                            spacing: 2
                            property bool openLog: false
                            Item {
                                id: logHead
                                Layout.fillWidth: true
                                implicitHeight: 20
                                Label { anchors { left: parent.left; verticalCenter: parent.verticalCenter } text: "What changed"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3 }
                                Glyph { anchors { right: parent.right; verticalCenter: parent.verticalCenter } name: logHead.parent.openLog ? "chevron-up" : "chevron-down"; size: 12; color: Theme.text3 }
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: logHead.parent.openLog = !logHead.parent.openLog }
                            }
                            Label { visible: parent.openLog; text: sheetCol.m ? sheetCol.m.changelog : ""; size: Theme.sizeCaption; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true; mono: true }
                        }
                        Item { Layout.preferredHeight: Theme.s2 }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.s1
                            Button { visible: !!(sheetCol.m && sheetCol.m.user); text: "Folder"; variant: "text"; onClicked: Compositor.exec("xdg-open " + JSON.stringify(sheetCol.m.dir)) }
                            Button { visible: !!(sheetCol.m && sheetCol.m.user && sheetCol.m.source); text: "Edit"; variant: "text"; onClicked: store.edit(sheetCol.m) }
                            Button { visible: !!(sheetCol.m && sheetCol.m.user); text: "Delete"; variant: "text"; onClicked: { Widgets.deleteUser(sheetCol.m.id); store.selected = ""; } }
                            Item { Layout.fillWidth: true }
                            Button { text: Widgets.placed(sheetCol.m ? sheetCol.m.id : "") > 0 && !(sheetCol.m && sheetCol.m.multiple) ? "On the dashboard" : "Place"; glyph: "plus"; variant: "accent"; enabled: sheetCol.m && Widgets.canAdd(sheetCol.m.id) && !Widgets.blocked(sheetCol.m); onClicked: store.place(sheetCol.m.id) }
                        }
                    }
                }
                Scrollbar { target: sheet; anchors { top: parent.top; bottom: parent.bottom; right: parent.right; margins: 4 } }
            }
        }
    }
}
