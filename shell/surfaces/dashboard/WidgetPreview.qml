import QtQuick
import QtQuick.Effects
import qs.theme
import qs.ui
import qs.services

// A widget's picture for the store: its first screenshot when it ships one, otherwise the widget itself,
// drawn live at its default size inside a card's chrome and scaled to fit.
Item {
    id: preview
    property var manifest: null
    property bool large: false
    readonly property string shot: manifest && manifest.screenshots && manifest.screenshots.length ? "file://" + manifest.screenshots[0] : ""
    readonly property bool blocked: Widgets.blocked(manifest)
    // A listing that is not installed has no code here to draw live.
    readonly property bool absent: !!(manifest && manifest.catalogue && !Widgets.manifests[manifest.id])
    // The default span, in a 3-column-wide card's proportions: the same 12-column grid at a 1280 width.
    readonly property var span: manifest ? Widgets.spanOf(manifest.default || (manifest.sizes && manifest.sizes[0]) || "3x1") : { w: 3, h: 1 }
    readonly property real frameW: span.w * 100 + (span.w - 1) * 12
    readonly property real frameH: span.h * 96 + (span.h - 1) * 12

    Rectangle {
        anchors.fill: parent
        radius: Theme.radiusControl
        color: Theme.ink
        border.width: 1; border.color: Theme.hairline
        clip: true
        Image {
            visible: preview.shot !== ""
            anchors.fill: parent
            source: preview.shot
            fillMode: Image.PreserveAspectFit
            asynchronous: true
        }
        Item {
            id: frame
            visible: preview.shot === "" && !preview.blocked && !preview.absent && !!preview.manifest
            anchors.centerIn: parent
            width: preview.frameW; height: preview.frameH
            scale: Math.min((preview.width - 16) / preview.frameW, (preview.height - 16) / preview.frameH)
            Rectangle {
                anchors.fill: parent
                radius: Theme.radiusPanel - 2
                color: Theme.glass
                border.width: 1; border.color: Theme.hairline
                Column {
                    anchors { fill: parent; margins: Theme.s3 }
                    spacing: Theme.s2
                    Label { visible: (loader.item ? loader.item.title : "") !== ""; text: loader.item ? loader.item.title : ""; size: Theme.sizeCaption; weight: Font.DemiBold; color: Theme.text2 }
                    Loader {
                        id: loader
                        width: parent.width
                        height: parent.height - ((loader.item && loader.item.title) ? 20 : 0)
                        source: preview.manifest ? Widgets.componentUrl(preview.manifest.id) : ""
                        onLoaded: {
                            if (item.hasOwnProperty("manifest")) item.manifest = Qt.binding(() => preview.manifest);
                            if (item.hasOwnProperty("size")) item.size = Qt.binding(() => preview.span.w + "x" + preview.span.h);
                            if (item.hasOwnProperty("settings")) item.settings = Qt.binding(() => Widgets.settingsFor(preview.manifest.id));
                            if (item.hasOwnProperty("host")) item.host = host;
                        }
                    }
                }
            }
            // The preview only looks; clicks go to the card beneath.
            MouseArea { anchors.fill: parent; onClicked: mouse => mouse.accepted = false; onPressed: mouse => mouse.accepted = false }
        }
        Glyph { visible: preview.shot === "" && preview.absent; anchors.centerIn: parent; name: preview.manifest ? (preview.manifest.glyph || "layout-grid") : "layout-grid"; size: preview.large ? 40 : 28; color: Theme.text3 }
        Label {
            visible: preview.blocked && !preview.absent
            anchors.centerIn: parent
            width: parent.width - Theme.s4 * 2
            text: "Needs trust to run"
            size: Theme.sizeCaption; color: Theme.danger; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap
        }
    }
    WidgetHost { id: host; permissions: preview.manifest ? Widgets.grantsFor(preview.manifest.id) : [] }
    // Only the live drawing can be captured: the card frame at twice its size, for a crisp listing.
    readonly property bool capturable: shot === "" && !blocked && !absent && loader.status === Loader.Ready
    function capture(path, done) {
        if (!capturable) { done(false); return; }
        frame.grabToImage(result => done(result.saveToFile(path)), Qt.size(Math.round(preview.frameW * 2), Math.round(preview.frameH * 2)));
    }
}
