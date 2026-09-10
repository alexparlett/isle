import QtQuick
import QtQuick.Layouts
import Quickshell
import qs.theme
import qs.ui
import qs.services

// The Settings window: a sidebar of sections, the section on the right.
FloatingWindow {
    id: root
    title: "Settings"
    visible: Surfaces.settings
    implicitWidth: 980
    implicitHeight: 720
    minimumSize: Qt.size(760, 520)
    color: Theme.light ? "#FFFFFF" : "#131417"
    onVisibleChanged: if (!visible) Surfaces.settings = false

    // Grouped the way KDE, macOS and Windows do: connect, look and feel, hardware, people and system, then about.
    readonly property var sections: [
        { id: "network", glyph: "wifi", label: "Network" },
        { id: "bluetooth", glyph: "bluetooth", label: "Bluetooth" },
        { group: "Look and feel" },
        { id: "appearance", glyph: "palette", label: "Appearance" },
        { id: "wallpaper", glyph: "image", label: "Wallpaper" },
        { id: "notifications", glyph: "bell", label: "Notifications" },
        { id: "modes", glyph: "gamepad-2", label: "Modes" },
        { group: "Hardware" },
        { id: "displays", glyph: "monitor", label: "Displays" },
        { id: "audio", glyph: "volume-2", label: "Audio" },
        { id: "keyboard", glyph: "keyboard", label: "Keyboard" },
        { id: "shortcuts", glyph: "command", label: "Shortcuts" },
        { id: "mouse", glyph: "mouse", label: "Mouse" },
        { id: "printers", glyph: "printer", label: "Printers" },
        { id: "storage", glyph: "hard-drive", label: "Storage" },
        { id: "power", glyph: "power", label: "Power" },
        { id: "devices", glyph: "plug-zap", label: "Devices" },
        { group: "System" },
        { id: "apps", glyph: "layout-grid", label: "Apps" },
        { id: "users", glyph: "user", label: "Users" },
        { id: "datetime", glyph: "calendar-clock", label: "Date & time" },
        { id: "region", glyph: "languages", label: "Language & region" },
        { id: "updates", glyph: "download", label: "Updates" },
        { id: "about", glyph: "info", label: "About" },
    ]

    // After a search hit's page loads, scroll its row into view.
    property string query: ""
    // The search hit the arrow keys have reached; Enter opens it.
    property int hit: 0
    function reveal() {
        const item = SettingsIndex.rows[SettingsIndex.highlight];
        if (!item || !pageLoader.item) return;
        const y = item.mapToItem(pageLoader, 0, 0).y;
        flick.contentY = Math.max(0, Math.min(flick.contentHeight - flick.height, pageLoader.y + y - flick.height / 3));
    }
    Connections { target: SettingsIndex; function onHighlightChanged() { if (SettingsIndex.highlight) revealTimer.restart(); } function onRowsChanged() { if (SettingsIndex.highlight) revealTimer.restart(); } }
    Timer { id: revealTimer; interval: 60; onTriggered: root.reveal() }

    // A click on nothing in particular takes the focus off the search field.
    MouseArea { anchors.fill: parent; onPressed: mouse => { search.focus = false; mouse.accepted = false; } }
    // The mouse's back and forward buttons, and Alt with the arrows, walk the trail.
    MouseArea { anchors.fill: parent; acceptedButtons: Qt.BackButton | Qt.ForwardButton; onClicked: mouse => mouse.button === Qt.BackButton ? Surfaces.settingsBack() : Surfaces.settingsForward() }
    Shortcut { sequences: ["Alt+Left", StandardKey.Back]; onActivated: Surfaces.settingsBack() }
    Shortcut { sequences: ["Alt+Right", StandardKey.Forward]; onActivated: Surfaces.settingsForward() }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        // Sidebar
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 210
            color: "transparent"
            Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.hairline }
            // The section list scrolls when the window is shorter than it.
            Flickable {
                anchors { fill: parent; margins: Theme.s3; topMargin: Theme.s4 }
                contentHeight: sideCol.implicitHeight
                clip: true
            ColumnLayout {
                id: sideCol
                width: parent.width
                spacing: 2
                // The title, with back and forward at the right, as a toolbar has them.
                RowLayout {
                    Layout.fillWidth: true
                    Layout.leftMargin: Theme.s2 + 2
                    Layout.bottomMargin: Theme.s2
                    spacing: Theme.s1
                    Label { text: "Settings"; size: Theme.sizeHeading; weight: Font.DemiBold; Layout.fillWidth: true }
                    Repeater {
                        model: [{ glyph: "arrow-left", flip: false, can: Surfaces.settingsCanBack, go: () => Surfaces.settingsBack() },
                                { glyph: "arrow-left", flip: true, can: Surfaces.settingsCanForward, go: () => Surfaces.settingsForward() }]
                        Rectangle {
                            required property var modelData
                            width: 26; height: 26; radius: 13
                            color: navArea.containsMouse && modelData.can ? Theme.pressed : Theme.raised
                            border.width: 1; border.color: Theme.hairline
                            opacity: modelData.can ? 1 : 0.35
                            Glyph { anchors.centerIn: parent; name: modelData.glyph; size: 13; rotation: modelData.flip ? 180 : 0; color: Theme.text }
                            MouseArea { id: navArea; anchors.fill: parent; hoverEnabled: true; cursorShape: modelData.can ? Qt.PointingHandCursor : Qt.ArrowCursor; onClicked: modelData.go() }
                        }
                    }
                }
                Field {
                    id: search
                    Layout.fillWidth: true
                    Layout.bottomMargin: Theme.s2
                    implicitHeight: 32
                    size: Theme.sizeSmall
                    glyph: "search"
                    placeholder: "Search settings"
                    onTextChanged: { root.query = text; root.hit = 0; }
                    input.Keys.onPressed: event => {
                        if (event.key === Qt.Key_Escape) { text = ""; event.accepted = true; }
                        else if (event.key === Qt.Key_Down && results.count > 0) { root.hit = (root.hit + 1) % results.count; event.accepted = true; }
                        else if (event.key === Qt.Key_Up && results.count > 0) { root.hit = (root.hit + results.count - 1) % results.count; event.accepted = true; }
                        else if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && results.count > 0) { SettingsIndex.goTo(results.model[Math.min(root.hit, results.count - 1)]); text = ""; event.accepted = true; }
                    }
                }
                // Hits replace the section list while there is a query.
                Repeater {
                    id: results
                    model: root.query ? SettingsIndex.search(root.query) : []
                    Rectangle {
                        required property var modelData
                        required property int index
                        Layout.fillWidth: true
                        implicitHeight: 40
                        radius: Theme.radiusControl
                        color: index === root.hit ? Theme.raised : hitArea.containsMouse ? Qt.alpha(Theme.raised, 0.5) : "transparent"
                        border.width: 1
                        border.color: index === root.hit ? Theme.hairline : "transparent"
                        ColumnLayout {
                            anchors { fill: parent; leftMargin: Theme.s2 + 2; rightMargin: Theme.s2; topMargin: 5 }
                            spacing: 0
                            Label { text: modelData.label; size: Theme.sizeSmall; weight: Font.DemiBold; Layout.fillWidth: true; elide: Text.ElideRight }
                            Label { text: modelData.pageLabel; size: Theme.sizeCaption; color: Theme.text3; Layout.fillWidth: true; elide: Text.ElideRight }
                        }
                        MouseArea { id: hitArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { SettingsIndex.goTo(modelData); search.text = ""; } }
                    }
                }
                Label { visible: root.query !== "" && results.count === 0; text: "Nothing matches"; size: Theme.sizeSmall; color: Theme.text3; Layout.leftMargin: Theme.s2 + 2 }
                Repeater {
                    model: root.query ? [] : root.sections
                    Rectangle {
                        required property var modelData
                        readonly property bool isGroup: !!modelData.group
                        readonly property bool sel: !isGroup && Surfaces.settingsPage === modelData.id
                        Layout.fillWidth: true
                        implicitHeight: isGroup ? 26 : 34
                        Label { visible: parent.isGroup; anchors { left: parent.left; bottom: parent.bottom; leftMargin: Theme.s2 + 2; bottomMargin: 4 } text: modelData.group || ""; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text3 }
                        radius: Theme.radiusControl
                        color: sel ? Theme.raised : area.containsMouse && !isGroup ? Qt.alpha(Theme.raised, 0.5) : "transparent"
                        border.width: 1
                        border.color: sel ? Theme.hairline : "transparent"
                        RowLayout {
                            visible: !parent.isGroup
                            anchors { fill: parent; leftMargin: Theme.s2 + 2; rightMargin: Theme.s2 }
                            spacing: Theme.s2 + 2
                            Glyph { name: modelData.glyph || ""; size: 14; color: sel ? Theme.text : Theme.text2 }
                            Label { text: modelData.label || ""; weight: sel ? Font.DemiBold : Font.Medium; color: sel ? Theme.text : Theme.text2; Layout.fillWidth: true }
                        }
                        MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; enabled: !parent.isGroup; cursorShape: Qt.PointingHandCursor; onClicked: Surfaces.settingsPage = modelData.id }
                    }
                }
            }
            }
        }

        // Page
        Flickable {
            id: flick
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentHeight: pageLoader.height + Theme.s5 * 2
            clip: true
            Loader {
                id: pageLoader
                x: Theme.s5 + 4; y: Theme.s5
                width: parent.width - (Theme.s5 + 4) * 2
                source: Qt.resolvedUrl("pages/" + Surfaces.settingsPage.charAt(0).toUpperCase() + Surfaces.settingsPage.slice(1) + "Page.qml")
                onLoaded: if (SettingsIndex.highlight) revealTimer.restart()
            }
        }
    }
}
