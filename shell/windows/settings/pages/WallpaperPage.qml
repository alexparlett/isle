import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.theme
import qs.ui
import qs.services

// The wallpaper: a grid of what the wallpaper folders hold, the one in use ringed, and a way to bring in more.
SettingsPage {
    id: page
    title: "Wallpaper"
    subtitle: "Pick one, or choose any image. Folders under Pictures and the system's wallpapers are shown."

    property var images: []
    Process {
        id: lister
        command: ["python3", Quickshell.shellDir + "/scripts/wallpapers.py"].concat(Prefs.p.wallpaperFolders || [])
        running: true
        stdout: StdioCollector { onStreamFinished: { try { page.images = JSON.parse(text); } catch (e) { page.images = []; } } }
    }
    function choose(what) {
        Compositor.exec("sh -c " + JSON.stringify((what === "folder" ? "f=$(zenity --file-selection --directory 2>/dev/null)" : "f=$(zenity --file-selection --file-filter=\"Images | *.png *.jpg *.jpeg *.webp *.avif\" 2>/dev/null)")
            + "; [ -n \"$f\" ] && qs -p " + Quickshell.shellDir + " ipc call prefs " + (what === "folder" ? "add wallpaperFolders" : "set wallpaper") + " \"$f\""));
    }
    Connections { target: Prefs.p; function onWallpaperFoldersChanged() { lister.running = true; } }

    SettingsGroup {
        heading: "In use"
        SettingsRow {
            label: Prefs.p.wallpaper ? Prefs.p.wallpaper.split("/").pop() : "The drawn backdrop"
            description: Prefs.p.wallpaper ? Prefs.p.wallpaper.replace(/\/[^/]*$/, "").replace(Quickshell.env("HOME"), "~") : "Isle's own gradient, with the accent."
            RowLayout {
                spacing: Theme.s2
                Rectangle {
                    width: 64; height: 36; radius: 6; color: Theme.ink; clip: true
                    border.width: 1; border.color: Theme.hairlineStrong
                    Image { anchors.fill: parent; source: Prefs.p.wallpaper ? "file://" + Prefs.p.wallpaper : ""; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 128; visible: Prefs.p.wallpaper !== "" }
                    Backdrop { anchors.fill: parent; visible: Prefs.p.wallpaper === "" }
                }
                Button { text: "Choose an image"; onClicked: page.choose("image") }
                Button { text: "Add a folder"; variant: "text"; onClicked: page.choose("folder") }
                Button { text: "Drawn backdrop"; variant: "text"; visible: Prefs.p.wallpaper !== ""; onClicked: Prefs.p.wallpaper = "" }
            }
        }
    }

    Repeater {
        model: page.images.map(i => i.folder).filter((v, i, a) => a.indexOf(v) === i)
        SettingsGroup {
            id: folderGroup
            required property string modelData
            readonly property var items: page.images.filter(i => i.folder === modelData)
            heading: modelData
            Flow {
                Layout.fillWidth: true
                Layout.margins: Theme.s2
                spacing: Theme.s2
                Repeater {
                    model: folderGroup.items
                    Rectangle {
                        required property var modelData
                        readonly property bool current: Prefs.p.wallpaper === modelData.path
                        width: 176; height: 120
                        radius: 8; color: Theme.ink; clip: true
                        border.width: current ? 2 : 1
                        border.color: current ? Theme.accent : thumbArea.containsMouse ? Theme.hairlineStrong : Theme.hairline
                        Image { anchors.fill: parent; anchors.margins: 1; source: "file://" + modelData.path; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize.width: 352; cache: true }
                        Rectangle { anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom; height: 24; color: Qt.alpha(Theme.ink, 0.6) }
                        Label { anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom; anchors.margins: 6; text: modelData.name; size: Theme.sizeCaption; elide: Text.ElideRight; color: Theme.text }
                        MouseArea { id: thumbArea; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: Prefs.p.wallpaper = modelData.path }
                    }
                }
            }
        }
    }
    Label { visible: page.images.length === 0; text: "No images found in the wallpaper folders."; color: Theme.text3 }
}
