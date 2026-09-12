import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Audio"

    SettingsGroup {
        heading: "Output"
        Repeater {
            model: Audio.sinks
            SettingsRow {
                required property var modelData
                label: modelData.description
                description: modelData === Audio.sink ? "Default" : ""
                RowLayout {
                    spacing: Theme.s2
                    Glyph { name: "check"; size: 14; color: Theme.accent; visible: modelData === Audio.sink }
                    Button { text: "Use"; visible: modelData !== Audio.sink; onClicked: Audio.setSink(modelData) }
                }
            }
        }
        SettingsRow {
            label: "Volume"
            RowLayout {
                spacing: Theme.s2
                Slider { implicitWidth: 200; value: Audio.muted ? 0 : Audio.volume; onMoved: v => { Audio.setMuted(false); Audio.setVolume(v); } }
                Label { text: Audio.muted ? "0" : Math.round(Audio.volume * 100); mono: true; tabular: true; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 28; horizontalAlignment: Text.AlignRight }
            }
        }
    }

    SettingsGroup {
        heading: "Input"
        visible: Audio.sources.length > 0
        Repeater {
            model: Audio.sources
            SettingsRow {
                required property var modelData
                label: modelData.description
                description: modelData === Audio.source ? "Default" : ""
                RowLayout {
                    spacing: Theme.s2
                    Glyph { name: "check"; size: 14; color: Theme.accent; visible: modelData === Audio.source }
                    Button { text: "Use"; visible: modelData !== Audio.source; onClicked: Audio.setSource(modelData) }
                }
            }
        }
        SettingsRow {
            label: "Microphone level"
            description: Audio.micInUse ? "In use by " + Audio.captures.map(c => Audio.streamName(c)).join(", ") : Audio.sourceMuted ? "Muted" : ""
            RowLayout {
                spacing: Theme.s2
                Slider { implicitWidth: 200; value: Audio.sourceMuted ? 0 : Audio.sourceVolume; onMoved: v => { Audio.setSourceMuted(false); Audio.setSourceVolume(v); } }
                Toggle { checked: !Audio.sourceMuted; onToggled: v => Audio.setSourceMuted(!v) }
            }
        }
    }

    SettingsGroup {
        heading: "Apps"
        Repeater {
            model: Audio.apps
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: modelData.detail
                RowLayout {
                    spacing: Theme.s2
                    Slider { implicitWidth: 200; value: modelData.node.audio && !modelData.node.audio.muted ? modelData.node.audio.volume : 0; onMoved: v => { Audio.setAppMuted(modelData, false); Audio.setAppVolume(modelData, v); } }
                    Toggle { checked: modelData.node.audio ? !modelData.node.audio.muted : true; onToggled: v => Audio.setAppMuted(modelData, !v) }
                }
            }
        }
        SettingsRow { visible: Audio.apps.length === 0; label: "Nothing playing"; description: "Apps appear here while they play sound." }
    }
}
