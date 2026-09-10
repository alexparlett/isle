import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

WidgetBase {
    title: "Media"
    meta: Media.present && Media.active.identity ? Media.active.identity : ""

    RowLayout {
        anchors.fill: parent
        spacing: Theme.s3
        visible: Media.present
        Rectangle {
            width: 48; height: 48; radius: Theme.radiusControl
            color: Theme.raised
            clip: true
            Image { anchors.fill: parent; source: Media.artUrl; fillMode: Image.PreserveAspectCrop; asynchronous: true }
            Glyph { anchors.centerIn: parent; name: "music"; size: 14; visible: Media.artUrl === "" }
        }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Label { text: Media.title; weight: Font.DemiBold; Layout.fillWidth: true }
            Label { text: Media.artist; size: Theme.sizeSmall; color: Theme.text2; Layout.fillWidth: true }
        }
        RowLayout {
            spacing: Theme.s2 + 2
            Glyph { name: "skip-back"; size: 14; MouseArea { anchors.fill: parent; onClicked: Media.previous() } }
            Glyph { name: Media.playing ? "pause" : "play"; size: 14; weight: 1.6; color: Theme.text; MouseArea { anchors.fill: parent; onClicked: Media.togglePlaying() } }
            Glyph { name: "skip-forward"; size: 14; MouseArea { anchors.fill: parent; onClicked: Media.next() } }
        }
    }

    Label { anchors.centerIn: parent; visible: !Media.present; text: "Nothing playing"; color: Theme.text3 }
}
