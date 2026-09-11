pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Renders the token file into the installed apps' theme files whenever the accent changes.
Singleton {
    id: root

    readonly property string script: Quickshell.shellDir + "/../theme/render.py"
    Process { id: renderer; command: ["python3", root.script] }

    // The flags files of Chromium-based launchers follow what is installed: the pass runs when the set of
    // desktop entries changes, which covers an app installed while the shell is up, and once at start.
    Process { id: flags; command: ["python3", root.script, "--flags-only"] }
    Timer { id: flagsLater; interval: 3000; onTriggered: flags.running = true }
    Connections { target: DesktopEntries.applications; function onValuesChanged() { flagsLater.restart(); } }

    // Auto is light between today's sunrise and sunset, minutes after midnight, from the timezone's tzdata coordinates.
    property int sunrise: 420
    property int sunset: 1140
    Process {
        id: sun
        command: ["python3", Quickshell.shellDir + "/../theme/sun.py"]
        running: true
        stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text); root.sunrise = o.rise; root.sunset = o.set; } catch (e) {} } }
    }
    Connections { target: DateTime; function onTimezoneChanged() { sun.running = true; } }
    property int minute: 0
    Timer {
        interval: 30000; running: true; repeat: true; triggeredOnStart: true
        onTriggered: { const d = new Date(); const m = d.getHours() * 60 + d.getMinutes(); if (m < root.minute) sun.running = true; root.minute = m; }
    }
    readonly property bool daytime: minute >= sunrise && minute < sunset
    readonly property bool light: Prefs.p.theme === "light" || (Prefs.p.theme === "auto" && daytime)
    onLightChanged: apply()

    // After the preference file has been written, which lands a moment after the property changes.
    Timer { id: later; interval: 400; onTriggered: renderer.running = true }
    function apply() { later.restart(); }

    Connections { target: Prefs.p; function onAccentChanged() { root.apply(); } function onThemeChanged() { root.apply(); } function onTitleBarsChanged() { root.apply(); } }
}
