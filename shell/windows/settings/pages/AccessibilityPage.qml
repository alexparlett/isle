import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Accessibility"
    subtitle: "Seeing the screen and the shell more easily."

    SettingsGroup {
        heading: "Seeing"
        SettingsRow {
            label: "Magnifier"
            description: (Access.zoom > 1 ? "The screen at " + Math.round(Access.zoom * 100) + "% around the pointer." : "Off.") + "  Super+Ctrl and = or - steps it, Super+Ctrl+0 turns it off."
            RowLayout {
                spacing: Theme.s2
                Button { glyph: "zoom-out"; variant: "text"; enabled: Access.zoom > 1; onClicked: Access.zoomOut() }
                Label { text: Math.round(Access.zoom * 100) + "%"; tabular: true; size: Theme.sizeSmall; color: Theme.text2 }
                Button { glyph: "zoom-in"; variant: "text"; enabled: Access.zoom < 4; onClicked: Access.zoomIn() }
                Button { text: "Off"; variant: "text"; visible: Access.zoom > 1; onClicked: Access.zoomReset() }
            }
        }
        SettingsRow {
            label: "Text size"
            description: "Every size in the shell, together."
            Dropdown { listWidth: 160; options: [[1, "Normal"], [1.15, "Larger"], [1.3, "Large"], [1.5, "Largest"]]; value: Prefs.p.textScale || 1; onPicked: v => Prefs.p.textScale = Number(v) }
        }
        SettingsRow {
            label: "High contrast"
            description: "Quieter text and lines come up, and the glass loses its see-through."
            Toggle { checked: Prefs.p.highContrast; onToggled: v => Prefs.p.highContrast = v }
        }
        SettingsRow {
            label: "Reduce motion"
            description: "Surfaces appear in place rather than growing and sliding."
            Toggle { checked: Prefs.p.reducedMotion; onToggled: v => { Prefs.p.reducedMotion = v; Theme.motion = v || Modes.game ? 0 : 1; } }
        }
    }

    SettingsGroup {
        heading: "Typing"
        SettingsRow {
            label: "On-screen keyboard"
            description: "Super+Shift+K shows and hides it; a controller drives it in game mode."
            Button { text: Osk.open ? "Hide" : "Show"; variant: "text"; onClicked: Osk.toggle() }
        }
    }
}
