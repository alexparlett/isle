pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import qs.services

// The token file, as typed properties. Every colour and size in the shell comes from here.
Singleton {
    id: root

    property var t: ({})

    FileView {
        path: Qt.resolvedUrl("tokens.json")
        blockLoading: true
        watchChanges: true
        onFileChanged: reload()
        onLoaded: root.t = JSON.parse(text())
    }

    readonly property bool light: Theming.light
    function c(group, key, fallback) {
        if (group === "color" && light && root.t.light && root.t.light[key] !== undefined) return root.t.light[key];
        const g = root.t[group];
        return g && g[key] !== undefined ? g[key] : fallback;
    }

    // Colour
    readonly property color ink: c("color", "ink", "#0A0B0D")
    readonly property real glassAlpha: c("color", "glassAlpha", 0.78)
    readonly property real glassAlphaNoBlur: c("color", "glassAlphaNoBlur", 0.94)
    readonly property color glassBase: c("color", "glass", "#121316")
    readonly property color glass: Qt.alpha(glassBase, glassAlpha)
    readonly property color glassOpaque: Qt.alpha(glassBase, glassAlphaNoBlur)
    readonly property color raised: c("color", "raised", "#1B1C21")
    readonly property color pressed: c("color", "pressed", "#26272D")
    readonly property color text: c("color", "text", "#F2F2F3")
    readonly property color text2: Qt.alpha(text, c("color", "text2Alpha", 0.62))
    readonly property color text3: Qt.alpha(text, c("color", "text3Alpha", 0.38))
    readonly property color hairline: Qt.alpha(text, c("color", "hairlineAlpha", 0.08))
    readonly property color hairlineStrong: Qt.alpha(text, c("color", "hairlineStrongAlpha", 0.14))
    readonly property color accent: Prefs.p.accent ? Prefs.p.accent : c("color", "accent", "#7FA6FF")
    readonly property color onAccent: c("color", "onAccent", "#0A0B0D")
    readonly property color ok: c("color", "ok", "#6FCF97")
    readonly property color warn: c("color", "warn", "#F2C94C")
    readonly property color danger: c("color", "danger", "#FF6B6B")
    readonly property color live: c("color", "live", "#FF453A")

    // Geometry
    readonly property int radiusPanel: c("radius", "panel", 20)
    readonly property int radiusCard: c("radius", "card", 14)
    readonly property int radiusControl: c("radius", "control", 10)
    readonly property int radiusChip: c("radius", "chip", 8)
    readonly property int s1: c("space", "1", 4)
    readonly property int s2: c("space", "2", 8)
    readonly property int s3: c("space", "3", 12)
    readonly property int s4: c("space", "4", 16)
    readonly property int s5: c("space", "5", 24)
    readonly property int s6: c("space", "6", 32)
    readonly property int islandHeight: c("island", "height", 30)
    readonly property real glyphScale: c("icon", "scale", 1)
    readonly property real glyphWeight: c("icon", "weight", 1.75)
    readonly property int islandMargin: c("island", "margin", 12)
    readonly property int panelWidth: c("island", "panelWidth", 420)
    readonly property int headerHeight: c("island", "headerHeight", 44)

    // Type
    readonly property string fontUi: c("font", "ui", "Inter")
    readonly property string fontMono: c("font", "mono", "JetBrains Mono")
    readonly property var fontSize: c("font", "size", {})
    readonly property int sizeCaption: fontSize.caption ?? 11
    readonly property int sizeSmall: fontSize.small ?? 12
    readonly property int sizeBody: fontSize.body ?? 13
    readonly property int sizeHeading: fontSize.heading ?? 15
    readonly property int sizeTitle: fontSize.title ?? 20
    readonly property int sizeDisplay: fontSize.display ?? 28

    // Motion. Game mode and the reduced-motion preference set `motion` to 0.
    property real motion: 1
    readonly property int quick: c("motion", "quick", 140) * motion
    readonly property int move: c("motion", "move", 220) * motion
    readonly property int morph: c("motion", "morph", 360) * motion
    readonly property var spring: c("motion", "spring", { stiffness: 180, damping: 22 })
}
