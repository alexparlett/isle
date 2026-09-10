import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

// The widget library as a store: cards with a live preview or screenshot, the author and version, and what
// each widget touches; a sheet with the whole listing, its changelog, and the permissions as toggles. Browse
// lists the registries, and installing from one asks first: what it may reach, and whether to trust it.
Item {
    id: store
    property bool open: false
    // "installed" is everything the shell has; "yours" the user's own folder; "browse" the registries.
    property string tab: "installed"
    property string query: ""
    property string category: ""
    property string selected: ""
    property bool sourcesOpen: false
    // The name field for a new QML widget.
    property bool naming: false
    // The listing an install is being agreed to, with the grants and trust the dialog holds until Install.
    property var consent: null
    property var consentGrants: []
    property bool consentTrust: false
    signal place(string id)
    signal edit(var manifest)
    // Closing is asked of the owner, which holds `open`; assigning it here would break that binding.
    signal dismissed()
    readonly property var current: !selected ? null : tab === "browse" ? Widgets.catalogueEntry(selected) : Widgets.manifests[selected] || null
    // The installed manifest behind whatever is shown, when there is one.
    readonly property var installedOf: current ? Widgets.manifests[current.id] || null : null

    readonly property var rows: {
        const q = query.trim().toLowerCase();
        const src = tab === "browse" ? Widgets.catalogue : Widgets.library.filter(m => tab === "yours" ? m.user : true);
        return src.filter(m => !category || m.category === category)
            .filter(m => !q || m.name.toLowerCase().indexOf(q) >= 0 || (m.description || "").toLowerCase().indexOf(q) >= 0 || (m.author && String(m.author.name || m.author).toLowerCase().indexOf(q) >= 0));
    }
    function authorOf(m) { return m && m.author ? (m.author.name || String(m.author)) : ""; }
    function authorUrl(m) { return m && m.author && m.author.url ? m.author.url : ""; }
    function needsTrust(m) { return !!(m && m.restrictedIssues && m.restrictedIssues.length); }
    // An update starts from what the installed one already has; a first install from everything it asks.
    function askInstall(entry) {
        const inst = Widgets.manifests[entry.id] || null;
        consentGrants = inst ? (entry.permissions || []).filter(p => Widgets.grantsFor(inst.id).indexOf(p) >= 0) : (entry.permissions || []).slice();
        consentTrust = inst ? Widgets.trusted(inst) : false;
        Widgets.installError = "";
        consent = entry;
    }
    function confirmInstall() {
        if (!consent) return;
        Widgets.install(consent, consentGrants, consent.trust === true && consentTrust);
    }
    onTabChanged: { if (tab === "browse" && !Widgets.catalogueLoaded) Widgets.refreshCatalogue(false); sourcesOpen = false; }
    onOpenChanged: if (!open) { consent = null; sourcesOpen = false; naming = false; }
    // A widget the scaffold just made is selected as soon as the scan lists it.
    Connections {
        target: Widgets
        function onIdsChanged() { if (Widgets.scaffolded && Widgets.manifests[Widgets.scaffolded]) { store.tab = "yours"; store.selected = Widgets.scaffolded; Widgets.scaffolded = ""; } }
    }
    // The dialog closes itself once its install has landed.
    Connections {
        target: Widgets
        function onInstallingChanged() { if (Widgets.installing === "" && store.consent && Widgets.installError === "") store.consent = null; }
    }

    visible: open
    anchors.fill: parent
    Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.5) }
    MouseArea { anchors.fill: parent; onClicked: store.dismissed() }

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
                            id: tabChip
                            required property var modelData
                            readonly property bool sel: store.tab === modelData[0]
                            readonly property int badge: modelData[0] === "installed" ? Widgets.updates.length : 0
                            implicitHeight: 30; implicitWidth: tabRow.implicitWidth + Theme.s3 * 2
                            radius: 15; color: sel ? Theme.raised : "transparent"; border.width: 1; border.color: sel ? Theme.hairlineStrong : "transparent"
                            RowLayout {
                                id: tabRow
                                anchors.centerIn: parent
                                spacing: 6
                                Label { text: tabChip.modelData[1]; size: Theme.sizeSmall; weight: Font.DemiBold; color: tabChip.sel ? Theme.text : Theme.text2 }
                                Rectangle {
                                    visible: tabChip.badge > 0
                                    implicitWidth: Math.max(16, bl.implicitWidth + 8); implicitHeight: 16; radius: 8; color: Theme.accent
                                    Label { id: bl; anchors.centerIn: parent; text: tabChip.badge; size: 10; weight: Font.DemiBold; color: Theme.onAccent; tabular: true }
                                }
                            }
                            MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { store.tab = tabChip.modelData[0]; store.selected = ""; } }
                        }
                    }
                    Rectangle {
                        implicitWidth: 28; implicitHeight: 28; radius: 14; color: closeArea.containsMouse ? Theme.pressed : "transparent"
                        Glyph { anchors.centerIn: parent; name: "x"; size: 12; color: Theme.text2 }
                        MouseArea { id: closeArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: store.dismissed() }
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
                    Spinner { visible: store.tab === "browse" && Widgets.catalogueLoading; size: 14 }
                    Label { visible: !(store.tab === "browse" && Widgets.catalogueLoading); text: store.rows.length + (store.rows.length === 1 ? " widget" : " widgets"); size: Theme.sizeCaption; color: Theme.text3 }
                    Label {
                        visible: store.tab === "yours"
                        text: "New widget"; size: Theme.sizeCaption; color: Theme.accent
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: store.naming = true }
                    }
                    Label {
                        visible: store.tab === "browse"
                        text: "Sources"; size: Theme.sizeCaption; color: store.sourcesOpen ? Theme.text : Theme.accent
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { store.sourcesOpen = !store.sourcesOpen; store.selected = ""; } }
                    }
                    Rectangle {
                        visible: store.tab === "browse"
                        implicitWidth: 26; implicitHeight: 26; radius: 13; color: refreshArea.containsMouse ? Theme.pressed : "transparent"
                        Glyph { anchors.centerIn: parent; name: "refresh-cw"; size: 12; color: Theme.text2 }
                        MouseArea { id: refreshArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; enabled: !Widgets.catalogueLoading; onClicked: Widgets.refreshCatalogue(true) }
                    }
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
                        readonly property var installed: Widgets.manifests[modelData.id] || null
                        readonly property var update: Widgets.updateFor(modelData.id)
                        readonly property bool held: store.tab === "browse" && installed !== null && update === null
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
                                            Label { id: pl; anchors.centerIn: parent; text: Widgets.permissionPill(modelData); size: 10; weight: Font.DemiBold; color: Theme.accent }
                                        }
                                    }
                                    Rectangle {
                                        visible: cell.modelData.trust === true || store.needsTrust(cell.modelData)
                                        implicitHeight: 18; implicitWidth: tl.implicitWidth + 12
                                        radius: 9; color: Qt.alpha(cell.modelData.trust === true ? Theme.warn : Theme.danger, 0.16)
                                        Label { id: tl; anchors.centerIn: parent; text: cell.modelData.trust === true ? "full access" : "needs trust"; size: 10; weight: Font.DemiBold; color: cell.modelData.trust === true ? Theme.warn : Theme.danger }
                                    }
                                    Item { Layout.fillWidth: true }
                                    // The state at the right: an update waiting, installed, or on the dashboard.
                                    Rectangle {
                                        visible: cell.update !== null
                                        implicitHeight: 18; implicitWidth: ul.implicitWidth + 12
                                        radius: 9; color: Theme.accent
                                        Label { id: ul; anchors.centerIn: parent; text: "v" + (cell.update ? cell.update.version : ""); size: 10; weight: Font.DemiBold; color: Theme.onAccent; tabular: true }
                                    }
                                    Label { visible: cell.held; text: "installed"; size: Theme.sizeCaption; color: Theme.text3 }
                                    Label { visible: store.tab !== "browse" && Widgets.placed(cell.modelData.id) > 0; text: "on"; size: Theme.sizeCaption; color: Theme.accent }
                                    Glyph { visible: cell.held || (store.tab !== "browse" && Widgets.placed(cell.modelData.id) > 0); name: "check"; size: 11; color: cell.held ? Theme.text3 : Theme.accent }
                                }
                            }
                            MouseArea { id: cardArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { store.selected = cell.modelData.id; store.sourcesOpen = false; } onDoubleClicked: if (store.tab !== "browse") store.place(cell.modelData.id) }
                        }
                    }
                }
                Label {
                    visible: store.rows.length === 0 && !(store.tab === "browse" && Widgets.catalogueLoading)
                    text: store.tab !== "browse" ? "Nothing matches"
                        : Widgets.sourceStates.length && Widgets.sourceStates.every(s => s.error) ? "No source could be reached: " + Widgets.sourceStates[0].error
                        : Widgets.catalogue.length ? "Nothing matches" : "Nothing listed yet"
                    color: Theme.text3; Layout.alignment: Qt.AlignHCenter; wrapMode: Text.WordWrap; Layout.maximumWidth: 480; horizontalAlignment: Text.AlignHCenter
                }
            }

            // --- the sources ---------------------------------------------------------------------
            Rectangle {
                visible: store.sourcesOpen && store.tab === "browse"
                Layout.preferredWidth: 380
                Layout.fillHeight: true
                radius: Theme.radiusCard
                color: Theme.raised
                border.width: 1; border.color: Theme.hairline
                ColumnLayout {
                    anchors { fill: parent; margins: Theme.s3 }
                    spacing: Theme.s3
                    Label { text: "Sources"; size: Theme.sizeHeading; weight: Font.DemiBold }
                    Label { text: "A source is a git repository with an index.json listing widgets. The first is Isle's own."; size: Theme.sizeCaption; color: Theme.text3; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    Repeater {
                        model: Widgets.sources
                        RowLayout {
                            id: srcRow
                            required property string modelData
                            readonly property var state: Widgets.sourceStates.find(s => s.url === modelData) || null
                            Layout.fillWidth: true
                            spacing: Theme.s2
                            Glyph { name: srcRow.state && srcRow.state.error ? "x" : "globe"; size: 12; color: srcRow.state && srcRow.state.error ? Theme.danger : Theme.text3 }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Label { text: srcRow.state && srcRow.state.name !== srcRow.modelData ? srcRow.state.name : srcRow.modelData; size: Theme.sizeSmall; weight: Font.Medium; elide: Text.ElideMiddle; Layout.fillWidth: true }
                                Label { text: srcRow.state ? (srcRow.state.error || srcRow.state.widgets.length + " widgets  ·  " + srcRow.modelData) : srcRow.modelData; size: Theme.sizeCaption; color: srcRow.state && srcRow.state.error ? Theme.danger : Theme.text3; elide: Text.ElideMiddle; Layout.fillWidth: true }
                            }
                            Rectangle {
                                visible: srcRow.modelData !== Widgets.defaultSource
                                implicitWidth: 24; implicitHeight: 24; radius: 12; color: rmArea.containsMouse ? Theme.pressed : "transparent"
                                Glyph { anchors.centerIn: parent; name: "trash"; size: 12; color: Theme.text3 }
                                MouseArea { id: rmArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.removeSource(srcRow.modelData) }
                            }
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.s2
                        Field { id: srcField; Layout.fillWidth: true; implicitHeight: 32; placeholder: "https://github.com/…/widgets.git"; size: Theme.sizeSmall; onAccepted: { Widgets.addSource(text); text = ""; } }
                        Button { text: "Add"; glyph: "plus"; enabled: srcField.text.trim() !== ""; onClicked: { Widgets.addSource(srcField.text); srcField.text = ""; } }
                    }
                    Item { Layout.fillHeight: true }
                }
            }

            // --- the sheet ---------------------------------------------------------------------
            Rectangle {
                visible: store.current !== null && !store.sourcesOpen
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
                        readonly property var inst: store.installedOf
                        readonly property var update: sheetCol.m ? Widgets.updateFor(sheetCol.m.id) : null
                        readonly property bool busy: !!sheetCol.m && Widgets.installing === sheetCol.m.id
                        readonly property bool fromSource: !!(sheetCol.inst && sheetCol.inst.installed)
                        WidgetPreview { id: sheetPreview; Layout.fillWidth: true; Layout.preferredHeight: 180; manifest: sheetCol.m; large: true }
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
                            // Installed from a source: which one, and whether a newer version is listed.
                            Label {
                                visible: sheetCol.fromSource || !!(sheetCol.m && sheetCol.m.catalogue)
                                text: sheetCol.update ? "v" + (sheetCol.inst ? sheetCol.inst.version : "") + " installed  ·  v" + sheetCol.update.version + " available"
                                    : sheetCol.fromSource ? "Installed from " + (Widgets.sourceStates.find(s => s.url === sheetCol.inst.installed.source) || { name: sheetCol.inst.installed.source }).name
                                    : "From " + (sheetCol.m ? sheetCol.m.sourceName : "")
                                size: Theme.sizeCaption; color: sheetCol.update ? Theme.accent : Theme.text3; elide: Text.ElideMiddle; Layout.fillWidth: true
                            }
                        }
                        Label { visible: !!(sheetCol.m && (sheetCol.m.about || sheetCol.m.description)); text: sheetCol.m ? (sheetCol.m.about || sheetCol.m.description) : ""; size: Theme.sizeSmall; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        // Where it is from.
                        RowLayout {
                            visible: !!(sheetCol.m && (sheetCol.m.homepage || sheetCol.m.repo || sheetCol.m.origin))
                            spacing: Theme.s2
                            Glyph { name: "globe"; size: 12; color: Theme.text3 }
                            Label { text: sheetCol.m ? (sheetCol.m.homepage || sheetCol.m.repo || sheetCol.m.origin || "") : ""; size: Theme.sizeCaption; color: Theme.accent; elide: Text.ElideMiddle; Layout.fillWidth: true
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Compositor.exec("xdg-open " + JSON.stringify(sheetCol.m.homepage || sheetCol.m.repo || sheetCol.m.origin)) } }
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
                        // Permissions: toggles for an installed widget (it gets only what stays on); a list for one not yet installed.
                        ColumnLayout {
                            visible: !!(sheetCol.m && sheetCol.m.permissions && sheetCol.m.permissions.length)
                            Layout.fillWidth: true
                            spacing: 2
                            RowLayout {
                                Layout.fillWidth: true
                                Label { text: sheetCol.inst ? "Permissions" : "Asks for"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3; Layout.fillWidth: true }
                                Label { text: "Reset"; size: Theme.sizeCaption; color: Theme.accent; visible: !!sheetCol.inst && (Prefs.p.widgetGrants || {})[sheetCol.inst.id] !== undefined
                                    MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: Widgets.resetGrants(sheetCol.inst.id) } }
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
                                        Label { text: (modelData.indexOf("fetch:") === 0 ? "fetch " + modelData.slice(6) : modelData.replace(".write", " (control)")); size: Theme.sizeSmall; weight: Font.Medium }
                                        Label { text: Widgets.permissionLabel(modelData); size: Theme.sizeCaption; color: Theme.text3; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                    }
                                    Toggle { visible: sheetCol.inst !== null; checked: sheetCol.inst ? Widgets.grantsFor(sheetCol.inst.id).indexOf(modelData) >= 0 : false; onToggled: v => Widgets.setGrant(sheetCol.inst.id, modelData, v) }
                                }
                            }
                        }
                        // Full reach: asked by the author, granted by the user for one from a source.
                        RowLayout {
                            visible: !!(sheetCol.m && sheetCol.m.trust === true)
                            Layout.fillWidth: true
                            spacing: Theme.s2
                            Glyph { name: "shield"; size: 14; color: Theme.warn; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                Label { text: sheetCol.fromSource ? (Widgets.trusted(sheetCol.inst) ? "Full access granted" : "Full access withheld") : "Asks for full access"; size: Theme.sizeSmall; weight: Font.Medium; color: Theme.warn }
                                Label { text: "It runs with the shell's whole reach: every service, files, processes and the network. Only for code you have read or whose author you know." + (sheetCol.fromSource && !Widgets.trusted(sheetCol.inst) ? " It will not run until granted." : ""); size: Theme.sizeCaption; color: Theme.text3; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                            }
                            Toggle { visible: sheetCol.fromSource; checked: sheetCol.inst ? Widgets.trusted(sheetCol.inst) : false; onToggled: v => Widgets.setTrust(sheetCol.inst.id, v) }
                        }
                        Label {
                            visible: store.needsTrust(sheetCol.m) && !(sheetCol.m && sheetCol.m.trust === true)
                            text: "Reaches past the sandbox (" + (sheetCol.m ? (sheetCol.m.restrictedIssues || []).join(", ") : "") + ") without asking for full access, so it will not run."
                            size: Theme.sizeCaption; color: Theme.danger; wrapMode: Text.WordWrap; Layout.fillWidth: true
                        }
                        // The author's tools, for one of the user's own: check it, picture it, tag it.
                        ColumnLayout {
                            visible: !!(sheetCol.inst && sheetCol.inst.user) && !sheetCol.fromSource
                            Layout.fillWidth: true
                            spacing: Theme.s2
                            readonly property var check: Widgets.validation && sheetCol.inst && Widgets.validation.id === sheetCol.inst.id ? Widgets.validation : null
                            readonly property var shared: Widgets.shareResult && sheetCol.inst && Widgets.shareResult.id === sheetCol.inst.id ? Widgets.shareResult : null
                            Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: Theme.hairline }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: Theme.s1
                                Label { text: "Author"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3; Layout.fillWidth: true }
                                Spinner { visible: Widgets.authoring; size: 12 }
                                Button { text: "Check"; glyph: "shield-check"; variant: "text"; enabled: !Widgets.authoring; onClicked: Widgets.validate(sheetCol.inst.id) }
                                Button { text: "Picture"; glyph: "camera"; variant: "text"; enabled: sheetPreview.capturable; onClicked: sheetPreview.capture(sheetCol.inst.dir + "/screenshots/card.png", ok => { if (ok) { Widgets.rescan(); Widgets.validate(sheetCol.inst.id); } }) }
                                Button { text: "Tag"; glyph: "package"; variant: "text"; enabled: !Widgets.authoring; onClicked: Widgets.share(sheetCol.inst.id) }
                            }
                            Repeater {
                                model: parent.check ? parent.check.errors.map(e => ({ text: e, bad: true })).concat(parent.check.warnings.map(w => ({ text: w, bad: false }))) : []
                                RowLayout {
                                    required property var modelData
                                    Layout.fillWidth: true
                                    spacing: Theme.s2
                                    Glyph { name: modelData.bad ? "x" : "info"; size: 11; color: modelData.bad ? Theme.danger : Theme.warn; Layout.alignment: Qt.AlignTop; Layout.topMargin: 2 }
                                    Label { text: modelData.text; size: Theme.sizeCaption; color: modelData.bad ? Theme.danger : Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                }
                            }
                            Label { visible: !!parent.check && parent.check.ok && parent.check.warnings.length === 0; text: "Ready to share."; size: Theme.sizeCaption; color: Theme.ok }
                            Label { visible: !!parent.check && parent.check.ok && parent.check.warnings.length > 0; text: "Nothing stops it; the notes above would make a better listing."; size: Theme.sizeCaption; color: Theme.text3; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                            Label {
                                visible: !!parent.shared
                                text: parent.shared ? (parent.shared.ok ? parent.shared.note + (parent.shared.remote ? " Push it: git push origin " + parent.shared.tag : " Give it a remote and push the tag, then list it in a source. See docs/WIDGETS.md.") : parent.shared.error) : ""
                                size: Theme.sizeCaption; color: parent.shared && parent.shared.ok ? Theme.text2 : Theme.danger; wrapMode: Text.WordWrap; Layout.fillWidth: true
                            }
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
                            Label { visible: parent.openLog; text: (sheetCol.m && sheetCol.m.changelog) || ""; size: Theme.sizeCaption; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true; mono: true }
                        }
                        Label { visible: Widgets.installError !== "" && store.consent === null && Widgets.installing === ""; text: Widgets.installError; size: Theme.sizeCaption; color: Theme.danger; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                        Item { Layout.preferredHeight: Theme.s2 }
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: Theme.s1
                            Button { visible: !!(sheetCol.inst && sheetCol.inst.user); text: "Folder"; variant: "text"; onClicked: Compositor.exec("xdg-open " + JSON.stringify(sheetCol.inst.dir)) }
                            Button { visible: !!(sheetCol.inst && sheetCol.inst.user && sheetCol.inst.source) && !sheetCol.fromSource; text: "Edit"; variant: "text"; onClicked: store.edit(sheetCol.inst) }
                            Button { visible: !!(sheetCol.inst && sheetCol.inst.user) && !sheetCol.fromSource; text: "Delete"; variant: "text"; onClicked: { Widgets.deleteUser(sheetCol.inst.id); store.selected = ""; } }
                            Button { visible: sheetCol.fromSource; text: "Remove"; variant: "text"; enabled: !sheetCol.busy; onClicked: { Widgets.uninstall(sheetCol.inst.id); if (store.tab !== "browse") store.selected = ""; } }
                            Item { Layout.fillWidth: true }
                            Spinner { visible: sheetCol.busy; size: 14 }
                            Button { visible: sheetCol.update !== null; text: "Update"; glyph: "download"; variant: "accent"; enabled: !sheetCol.busy; onClicked: store.askInstall(sheetCol.update) }
                            Button { visible: !!(sheetCol.m && sheetCol.m.catalogue) && sheetCol.inst === null; text: "Install"; glyph: "download"; variant: "accent"; enabled: !sheetCol.busy; onClicked: store.askInstall(sheetCol.m) }
                            Button { visible: sheetCol.inst !== null && sheetCol.update === null; text: Widgets.placed(sheetCol.inst ? sheetCol.inst.id : "") > 0 && !(sheetCol.inst && sheetCol.inst.multiple) ? "On the dashboard" : "Place"; glyph: "plus"; variant: "accent"; enabled: !!sheetCol.inst && Widgets.canAdd(sheetCol.inst.id) && !Widgets.blocked(sheetCol.inst) && !sheetCol.busy; onClicked: store.place(sheetCol.inst.id) }
                        }
                    }
                }
                Scrollbar { target: sheet; anchors { top: parent.top; bottom: parent.bottom; right: parent.right; margins: 4 } }
            }
        }

        // --- a new QML widget: a name, then a folder with the whole shape of one ----------------
        Item {
            visible: store.naming
            anchors.fill: parent
            Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.5); radius: Theme.radiusPanel }
            MouseArea { anchors.fill: parent; onClicked: store.naming = false }
            Glass {
                anchors.centerIn: parent
                width: 420
                height: nameCol.implicitHeight + Theme.s4 * 2
                radius: Theme.radiusPanel
                MouseArea { anchors.fill: parent }
                ColumnLayout {
                    id: nameCol
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
                    spacing: Theme.s3
                    Label { text: "New widget"; size: Theme.sizeHeading; weight: Font.DemiBold }
                    Label { text: "A folder in your widgets with a manifest, a Widget.qml that draws, a README and a CHANGELOG. Edit it in any editor; the card redraws when the dashboard is next opened."; size: Theme.sizeSmall; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    Field { id: nameField; Layout.fillWidth: true; implicitHeight: 34; placeholder: "Name"; onAccepted: if (text.trim()) { Widgets.scaffold(text); store.naming = false; text = ""; } onVisibleChanged: if (visible) input.forceActiveFocus() }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.s2
                        Label { text: nameField.text.trim() ? "~/.config/isle/widgets/" + Widgets.slug(nameField.text) : ""; size: Theme.sizeCaption; color: Theme.text3; mono: true; elide: Text.ElideMiddle; Layout.fillWidth: true }
                        Button { text: "Cancel"; variant: "text"; onClicked: store.naming = false }
                        Button { text: "Create"; glyph: "plus"; variant: "accent"; enabled: nameField.text.trim() !== "" && !Widgets.authoring; onClicked: { Widgets.scaffold(nameField.text); store.naming = false; nameField.text = ""; } }
                    }
                }
            }
        }

        // --- consent: what the widget may reach, agreed before it lands -------------------------
        Item {
            visible: store.consent !== null
            anchors.fill: parent
            Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.5); radius: Theme.radiusPanel }
            MouseArea { anchors.fill: parent; onClicked: if (Widgets.installing === "") store.consent = null }
            Glass {
                anchors.centerIn: parent
                width: 440
                height: consentCol.implicitHeight + Theme.s4 * 2
                radius: Theme.radiusPanel
                MouseArea { anchors.fill: parent }
                ColumnLayout {
                    id: consentCol
                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
                    spacing: Theme.s3
                    readonly property var e: store.consent
                    readonly property bool updating: !!(consentCol.e && Widgets.manifests[consentCol.e.id])
                    readonly property bool busy: !!consentCol.e && Widgets.installing === consentCol.e.id
                    Label { text: (consentCol.updating ? "Update " : "Install ") + (consentCol.e ? consentCol.e.name : "") + "?"; size: Theme.sizeHeading; weight: Font.DemiBold }
                    Label { text: "v" + (consentCol.e ? consentCol.e.version : "") + " by " + (store.authorOf(consentCol.e) || "an unknown author") + ", from " + (consentCol.e ? consentCol.e.sourceName : "") + ". It goes into your widgets folder and runs inside the dashboard."; size: Theme.sizeSmall; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    ColumnLayout {
                        visible: !!(consentCol.e && consentCol.e.permissions && consentCol.e.permissions.length)
                        Layout.fillWidth: true
                        spacing: 2
                        Label { text: "It may"; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3 }
                        Repeater {
                            model: consentCol.e ? (consentCol.e.permissions || []) : []
                            RowLayout {
                                required property string modelData
                                Layout.fillWidth: true
                                spacing: Theme.s2
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 0
                                    Label { text: Widgets.permissionLabel(modelData); size: Theme.sizeSmall; weight: Font.Medium }
                                    Label { text: (modelData.indexOf("fetch:") === 0 ? "fetch " + modelData.slice(6) : modelData.replace(".write", " (control)")); size: Theme.sizeCaption; color: Theme.text3 }
                                }
                                Toggle { checked: store.consentGrants.indexOf(modelData) >= 0; onToggled: v => store.consentGrants = v ? store.consentGrants.concat([modelData]) : store.consentGrants.filter(p => p !== modelData) }
                            }
                        }
                    }
                    Label { visible: !!(consentCol.e && !(consentCol.e.permissions && consentCol.e.permissions.length) && consentCol.e.trust !== true); text: "It asks for nothing beyond drawing."; size: Theme.sizeSmall; color: Theme.text3 }
                    Rectangle {
                        visible: !!(consentCol.e && consentCol.e.trust === true)
                        Layout.fillWidth: true
                        implicitHeight: trustRow.implicitHeight + Theme.s3 * 2
                        radius: Theme.radiusControl
                        color: Qt.alpha(Theme.warn, 0.1); border.width: 1; border.color: Qt.alpha(Theme.warn, 0.4)
                        RowLayout {
                            id: trustRow
                            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s3 }
                            spacing: Theme.s3
                            Glyph { name: "shield"; size: 16; color: Theme.warn; Layout.alignment: Qt.AlignTop }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 2
                                Label { text: "Asks for full access"; size: Theme.sizeSmall; weight: Font.DemiBold; color: Theme.warn }
                                Label { text: "It would run with the shell's whole reach: every service, your files, processes and the network. Grant this only for code you have read or whose author you know. Without it the widget is installed but does not run."; size: Theme.sizeCaption; color: Theme.text2; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                                RowLayout {
                                    spacing: Theme.s2
                                    Layout.topMargin: 4
                                    Rectangle {
                                        implicitWidth: 18; implicitHeight: 18; radius: 4
                                        color: store.consentTrust ? Theme.warn : "transparent"; border.width: 1; border.color: store.consentTrust ? "transparent" : Theme.hairlineStrong
                                        Glyph { anchors.centerIn: parent; visible: store.consentTrust; name: "check"; size: 12; color: Theme.ink }
                                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: store.consentTrust = !store.consentTrust }
                                    }
                                    Label { text: "Grant full access"; size: Theme.sizeSmall; weight: Font.Medium
                                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: store.consentTrust = !store.consentTrust } }
                                }
                            }
                        }
                    }
                    Label { visible: Widgets.installError !== ""; text: Widgets.installError; size: Theme.sizeCaption; color: Theme.danger; wrapMode: Text.WordWrap; Layout.fillWidth: true }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.s2
                        Item { Layout.fillWidth: true }
                        Spinner { visible: consentCol.busy; size: 14 }
                        Button { text: "Cancel"; variant: "text"; enabled: !consentCol.busy; onClicked: store.consent = null }
                        Button { text: consentCol.updating ? "Update" : "Install"; glyph: "download"; variant: "accent"; enabled: !consentCol.busy; onClicked: store.confirmInstall() }
                    }
                }
            }
        }
    }
}
