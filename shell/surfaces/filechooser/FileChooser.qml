import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Isle.Files
import qs.theme
import qs.ui
import qs.services
import qs.windows.files

// The file chooser every application reaches through the portal: the same folder view as Files,
// in a modal card, answering the request that brought it up.
PanelWindow {
    id: root
    screen: Compositor.shellScreen
    visible: FileChooserPortal.current !== null

    readonly property var request: FileChooserPortal.current
    // Which of the request's filters is chosen; the dropdowns below are two placements of one choice.
    property int filterIndex: 0

    readonly property bool canAccept: request
        && (request.directory || (request.save ? nameField.text.trim() !== "" : list.hasSelection))

    // One choice, two placements: beside the name when saving, at the foot of the card when opening.
    component FileTypes: Dropdown {
        listWidth: 200
        value: root.filterIndex
        options: root.request ? root.request.filters.map((f, i) => [i, f.label]) : []
        onPicked: v => root.filterIndex = v
    }

    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "isle-filechooser"
    WlrLayershell.keyboardFocus: visible ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    // A new request starts where it asked to, and on the filter it asked for.
    onRequestChanged: {
        if (!request) return;
        dir.path = request.folder;
        dir.foldersOnly = request.directory;
        filterIndex = request.filters.length ? 0 : -1;
        nameField.text = request.currentName;
        if (request.save) nameField.input.forceActiveFocus();
        else list.forceActiveFocus();
    }

    function accept() {
        if (!canAccept) return;
        if (request.directory) request.accept([dir.path], filterIndex);
        else if (request.save) request.accept([Engine.join(dir.path, nameField.text.trim())], filterIndex);
        else request.accept([list.currentPath], filterIndex);
    }

    Directory {
        id: dir
        path: Engine.home
        patterns: {
            if (!root.request || root.filterIndex < 0) return [];
            const f = root.request.filters[root.filterIndex];
            return f ? f.patterns : [];
        }
    }

    Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.4) }
    MouseArea { anchors.fill: parent; onClicked: if (root.request) root.request.reject() }

    Glass {
        id: card
        width: Math.min(880, root.width - Theme.s6 * 2)
        height: Math.min(620, root.height - Theme.s6 * 2)
        anchors.centerIn: parent
        radius: Theme.radiusPanel
        focus: true
        opacity: root.visible ? 1 : 0
        scale: root.visible ? 1 : 0.96
        Behavior on opacity { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }
        Behavior on scale { NumberAnimation { duration: Theme.move; easing.type: Easing.OutQuint } }

        // The card is not the backdrop; a click inside it must not answer the request.
        MouseArea { anchors.fill: parent }
        Keys.onEscapePressed: if (root.request) root.request.reject()

        ColumnLayout {
            anchors { fill: parent; margins: Theme.s4 }
            spacing: Theme.s3

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.s2
                Label {
                    Layout.fillWidth: true
                    size: Theme.sizeHeading
                    elide: Text.ElideRight
                    text: root.request ? (root.request.title || (root.request.save ? "Save file" : "Open file")) : ""
                }
                Label {
                    visible: root.request && root.request.appId !== ""
                    size: Theme.sizeCaption
                    color: Theme.text3
                    text: root.request ? root.request.appId : ""
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.s2
                Button { variant: "text"; glyph: "arrow-up"; enabled: dir.path !== "/"; onClicked: dir.path = Engine.parentOf(dir.path) }
                Flickable {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 34
                    contentWidth: crumbs.implicitWidth
                    flickableDirection: Flickable.HorizontalFlick
                    clip: true
                    onContentWidthChanged: contentX = Math.max(0, contentWidth - width)
                    RowLayout {
                        id: crumbs
                        height: parent.height
                        spacing: 0
                        Repeater {
                            model: Engine.crumbs(dir.path)
                            delegate: RowLayout {
                                id: crumb
                                required property var modelData
                                required property int index
                                spacing: 0
                                Glyph { visible: crumb.index > 0; name: "chevron-right"; size: 12; color: Theme.text3 }
                                Button {
                                    variant: "text"
                                    text: crumb.modelData.name
                                    enabled: crumb.modelData.path !== dir.path
                                    onClicked: dir.path = crumb.modelData.path
                                }
                            }
                        }
                    }
                }
                Field {
                    Layout.preferredWidth: 170
                    glyph: "search"
                    placeholder: "Search"
                    onTextChanged: dir.filter = text
                }
            }

            FolderView {
                id: list
                Layout.fillWidth: true
                Layout.fillHeight: true
                directory: dir
                onActivated: path => dir.path = path
                // A double click on a file is the choice, not an application launch.
                openFiles: false
                onChosen: root.accept()
            }

            RowLayout {
                Layout.fillWidth: true
                visible: root.request && root.request.save
                spacing: Theme.s3
                Label { text: "Save as"; color: Theme.text2 }
                Field {
                    id: nameField
                    Layout.fillWidth: true
                    placeholder: "File name"
                    onAccepted: root.accept()
                }
                // The type belongs beside the name it decides the ending of, not down by Cancel.
                FileTypes { visible: root.request && root.request.filters.length > 0 }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.s2
                // Opening has no name field for the type to sit beside, so it keeps the foot of the card.
                FileTypes { visible: root.request && !root.request.save && root.request.filters.length > 0 }
                Item { Layout.fillWidth: true }
                Button { text: "Cancel"; onClicked: if (root.request) root.request.reject() }
                Button {
                    variant: "accent"
                    enabled: root.canAccept
                    text: root.request ? (root.request.acceptLabel || (root.request.save ? "Save" : "Open")) : "Open"
                    onClicked: root.accept()
                }
            }
        }
    }
}
