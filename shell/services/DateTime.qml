pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// The clock's settings through timedatectl: zone, network time, the RTC's convention. Changes ask polkit.
Singleton {
    id: root

    property string timezone: ""
    property bool ntp: false
    property bool canNtp: true
    property bool inSync: false
    property bool localRtc: false
    property var zones: []
    property string error: ""

    Process {
        id: status
        command: ["timedatectl", "show"]
        running: true
        stdout: StdioCollector {
            onStreamFinished: {
                for (const line of text.split("\n")) {
                    const [k, v] = line.split("=");
                    if (k === "Timezone") root.timezone = v;
                    else if (k === "NTP") root.ntp = v === "yes";
                    else if (k === "CanNTP") root.canNtp = v === "yes";
                    else if (k === "NTPSynchronized") root.inSync = v === "yes";
                    else if (k === "LocalRTC") root.localRtc = v === "yes";
                }
            }
        }
    }
    Process {
        command: ["timedatectl", "list-timezones"]
        running: true
        stdout: StdioCollector { onStreamFinished: root.zones = text.trim().split("\n").filter(z => z) }
    }
    function refresh() { status.running = true; }

    Process {
        id: actor
        onExited: root.refresh()
        stderr: StdioCollector { onStreamFinished: root.error = text.trim().split("\n").pop() || "" }
    }
    function run(args) { error = ""; actor.command = args; actor.running = true; }
    function setTimezone(z) { run(["timedatectl", "set-timezone", z]); }
    function setNtp(on) { run(["timedatectl", "set-ntp", on ? "true" : "false"]); }
    function setLocalRtc(on) { run(["timedatectl", "set-local-rtc", on ? "1" : "0"]); }
    // "YYYY-MM-DD HH:MM:SS"; only meaningful with network time off.
    function setTime(s) { run(["timedatectl", "set-time", s]); }

    // The shell's own clocks follow this.
    readonly property string timeFormat: Prefs.p.clock12 ? "h:mm AP" : "HH:mm"
}
