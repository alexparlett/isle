import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.theme
import qs.ui

// Time and date, with the weather for a named place beside them; the week strip needs two rows, the days
// ahead four. Open-Meteo, no account: geocoded once per place, refreshed every half hour.
WidgetBase {
    id: root
    title: ""

    SystemClock { id: clock; precision: root.settings.seconds ? SystemClock.Seconds : SystemClock.Minutes }
    readonly property string timeFormat: (root.settings.format === "12h" ? "h:mm" : "HH:mm") + (root.settings.seconds ? ":ss" : "") + (root.settings.format === "12h" ? " AP" : "")
    // Monday of this week, for the strip.
    readonly property var monday: { const d = new Date(clock.date); const day = (d.getDay() + 6) % 7; d.setDate(d.getDate() - day); return d; }

    // --- weather -----------------------------------------------------------------------
    readonly property bool wantWeather: root.settings.weather !== false
    readonly property string placeQuery: wantWeather ? String(root.settings.place || "").trim() : ""
    readonly property string unit: root.settings.units === "fahrenheit" ? "°F" : "°C"
    property var place: null
    property var now: null
    property var days: []
    function glyphFor(code) {
        if (code === 0) return "sun";
        if (code <= 2) return "cloud-sun";
        if (code === 3) return "cloud";
        if (code <= 48) return "cloud-fog";
        if (code <= 57) return "cloud-drizzle";
        if (code <= 67 || (code >= 80 && code <= 82)) return "cloud-rain";
        if (code <= 77 || (code >= 85 && code <= 86)) return "cloud-snow";
        return "cloud-lightning";
    }
    function describe(code) {
        if (code === 0) return "Clear";
        if (code <= 2) return "Partly cloudy";
        if (code === 3) return "Overcast";
        if (code <= 48) return "Fog";
        if (code <= 57) return "Drizzle";
        if (code <= 67) return "Rain";
        if (code <= 77) return "Snow";
        if (code <= 82) return "Showers";
        if (code <= 86) return "Snow showers";
        return "Thunder";
    }
    Process {
        id: geocoder
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null; try { r = JSON.parse(text).results[0]; } catch (e) {}
                if (!r) { root.now = null; return; }
                root.place = { query: root.placeQuery, name: r.name, lat: r.latitude, lon: r.longitude };
                root.refresh();
            }
        }
    }
    Process {
        id: fetcher
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null; try { r = JSON.parse(text); } catch (e) {}
                if (!r || !r.current) return;
                root.now = { temp: Math.round(r.current.temperature_2m), code: r.current.weather_code, hi: Math.round(r.daily.temperature_2m_max[0]), lo: Math.round(r.daily.temperature_2m_min[0]) };
                const out = [];
                for (let i = 1; i < r.daily.time.length; i++)
                    out.push({ day: Qt.formatDate(new Date(r.daily.time[i] + "T12:00:00"), "ddd"), code: r.daily.weather_code[i], hi: Math.round(r.daily.temperature_2m_max[i]), lo: Math.round(r.daily.temperature_2m_min[i]) });
                root.days = out;
            }
        }
    }
    function locate() {
        if (!placeQuery) { now = null; days = []; return; }
        if (place && place.query === placeQuery) { refresh(); return; }
        geocoder.command = ["curl", "-sL", "--max-time", "15", "https://geocoding-api.open-meteo.com/v1/search?count=1&name=" + encodeURIComponent(placeQuery)];
        geocoder.running = true;
    }
    function refresh() {
        if (!place || !wantWeather) return;
        fetcher.command = ["curl", "-sL", "--max-time", "20", "https://api.open-meteo.com/v1/forecast?latitude=" + place.lat + "&longitude=" + place.lon
            + "&current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min,weather_code&timezone=auto&forecast_days=6"
            + "&temperature_unit=" + (root.settings.units === "fahrenheit" ? "fahrenheit" : "celsius")];
        fetcher.running = true;
    }
    onPlaceQueryChanged: locate()
    onUnitChanged: refresh()
    Component.onCompleted: locate()
    Timer { interval: 1800000; running: root.wantWeather; repeat: true; onTriggered: root.refresh() }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.s2

        // Time and date at the left, today's weather at the right.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.s3
            ColumnLayout {
                spacing: 0
                Label { text: Qt.formatTime(clock.date, root.timeFormat); size: 34; weight: Font.DemiBold; tabular: true; font.letterSpacing: -1 }
                Label { text: Qt.formatDate(clock.date, root.cols >= 3 ? "dddd d MMMM" : "ddd d MMM"); color: Theme.text2; size: Theme.sizeSmall }
            }
            Item { Layout.fillWidth: true }
            RowLayout {
                visible: root.wantWeather && !!root.now
                spacing: Theme.s2
                Glyph { name: root.now ? root.glyphFor(root.now.code) : "cloud"; size: 24; color: Theme.text }
                ColumnLayout {
                    spacing: 0
                    Label { text: root.now ? root.now.temp + root.unit : ""; size: Theme.sizeHeading; weight: Font.DemiBold; tabular: true }
                    Label { text: root.now ? root.describe(root.now.code) : ""; size: Theme.sizeCaption; color: Theme.text2 }
                    Label { text: root.place ? root.place.name + (root.now ? "  ·  " + root.now.hi + " / " + root.now.lo : "") : ""; size: Theme.sizeCaption; color: Theme.text3 }
                }
            }
        }

        // The days ahead, when there is height for them.
        ColumnLayout {
            visible: root.rows >= 4 && root.wantWeather && root.days.length > 0
            Layout.fillWidth: true
            spacing: 2
            Repeater {
                model: root.days.slice(0, root.rows >= 6 ? 5 : 3)
                RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: Theme.s2
                    Label { text: modelData.day; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 28 }
                    Glyph { name: root.glyphFor(modelData.code); size: 13; color: Theme.text2 }
                    Label { text: root.describe(modelData.code); size: Theme.sizeCaption; color: Theme.text3; elide: Text.ElideRight; Layout.fillWidth: true }
                    Label { text: modelData.hi + "  " + modelData.lo; size: Theme.sizeCaption; tabular: true; color: Theme.text2 }
                }
            }
        }

        Item { Layout.fillHeight: true }

        // The week, when the card is two rows tall.
        RowLayout {
            Layout.fillWidth: true
            visible: root.settings.week !== false && root.rows >= 2
            spacing: Theme.s1
            Repeater {
                model: 7
                Rectangle {
                    required property int index
                    readonly property var day: { const d = new Date(root.monday); d.setDate(d.getDate() + index); return d; }
                    readonly property bool today: day.toDateString() === clock.date.toDateString()
                    readonly property bool past: day < clock.date && !today
                    Layout.fillWidth: true
                    implicitHeight: 44
                    radius: Theme.radiusControl
                    color: today ? Theme.raised : "transparent"
                    border.width: 1
                    border.color: today ? Theme.hairlineStrong : "transparent"
                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: 2
                        Label { text: Qt.formatDate(day, "ddd").charAt(0); size: 10; weight: Font.DemiBold; color: Theme.text3; Layout.alignment: Qt.AlignHCenter }
                        Label { text: day.getDate(); tabular: true; weight: today ? Font.DemiBold : Font.Medium; color: past ? Theme.text3 : Theme.text; Layout.alignment: Qt.AlignHCenter }
                    }
                }
            }
        }
    }
}
