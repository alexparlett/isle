import QtQuick
import QtQuick.Layouts
import Quickshell.Io
import qs.theme
import qs.ui

// Open-Meteo, no account: the place is geocoded once per name, the forecast fetched every half hour.
WidgetBase {
    id: root
    title: "Weather"
    meta: place ? place.name : ""

    property var place: null
    property var now: null
    property var days: []
    property string error: ""
    readonly property string unit: root.settings.units === "fahrenheit" ? "°F" : "°C"
    readonly property string placeQuery: String(root.settings.place || "").trim()

    // WMO weather codes as a glyph and a word.
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
                if (!r) { root.error = "No place called " + root.placeQuery; return; }
                root.place = { query: root.placeQuery, name: r.name + (r.country_code ? ", " + r.country_code : ""), lat: r.latitude, lon: r.longitude };
                root.error = "";
                root.refresh();
            }
        }
    }
    Process {
        id: fetcher
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null; try { r = JSON.parse(text); } catch (e) {}
                if (!r || !r.current) { root.error = "Could not reach the forecast"; return; }
                root.now = { temp: Math.round(r.current.temperature_2m), code: r.current.weather_code, hi: Math.round(r.daily.temperature_2m_max[0]), lo: Math.round(r.daily.temperature_2m_min[0]) };
                const out = [];
                for (let i = 1; i < r.daily.time.length; i++)
                    out.push({ day: Qt.formatDate(new Date(r.daily.time[i] + "T12:00:00"), "ddd"), code: r.daily.weather_code[i], hi: Math.round(r.daily.temperature_2m_max[i]), lo: Math.round(r.daily.temperature_2m_min[i]) });
                root.days = out;
                root.error = "";
            }
        }
    }
    function locate() {
        if (!placeQuery) return;
        if (place && place.query === placeQuery) { refresh(); return; }
        geocoder.command = ["curl", "-sL", "--max-time", "15", "https://geocoding-api.open-meteo.com/v1/search?count=1&name=" + encodeURIComponent(placeQuery)];
        geocoder.running = true;
    }
    function refresh() {
        if (!place) return;
        fetcher.command = ["curl", "-sL", "--max-time", "20", "https://api.open-meteo.com/v1/forecast?latitude=" + place.lat + "&longitude=" + place.lon
            + "&current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min,weather_code&timezone=auto&forecast_days=6"
            + "&temperature_unit=" + (root.settings.units === "fahrenheit" ? "fahrenheit" : "celsius")];
        fetcher.running = true;
    }
    onPlaceQueryChanged: locate()
    onSettingsChanged: refresh()
    Component.onCompleted: locate()
    Timer { interval: 1800000; running: true; repeat: true; onTriggered: root.refresh() }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.s2
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.s3
            visible: !!root.now
            Glyph { name: root.now ? root.glyphFor(root.now.code) : "cloud"; size: root.rows >= 2 ? 34 : 24; color: Theme.text }
            Label { text: root.now ? root.now.temp + root.unit : ""; size: root.rows >= 2 ? 30 : 22; weight: Font.DemiBold; tabular: true }
            ColumnLayout {
                spacing: 0
                Layout.fillWidth: true
                Label { text: root.now ? root.describe(root.now.code) : ""; size: Theme.sizeSmall; elide: Text.ElideRight; Layout.fillWidth: true }
                Label { text: root.now ? "H " + root.now.hi + "  L " + root.now.lo : ""; size: Theme.sizeCaption; color: Theme.text3; tabular: true }
            }
        }
        // The days ahead, as many as the height allows.
        ColumnLayout {
            visible: root.rows >= 2 && root.days.length > 0
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 2
            Repeater {
                model: root.days.slice(0, Math.max(0, root.rows * 2 - 3))
                RowLayout {
                    required property var modelData
                    Layout.fillWidth: true
                    spacing: Theme.s2
                    Label { text: modelData.day; size: Theme.sizeCaption; color: Theme.text2; Layout.preferredWidth: 30 }
                    Glyph { name: root.glyphFor(modelData.code); size: 14; color: Theme.text2 }
                    Label { text: root.describe(modelData.code); size: Theme.sizeCaption; color: Theme.text3; elide: Text.ElideRight; Layout.fillWidth: true }
                    Label { text: modelData.hi + "  " + modelData.lo; size: Theme.sizeCaption; tabular: true; color: Theme.text2 }
                }
            }
            Item { Layout.fillHeight: true }
        }
        Label { visible: !root.now; Layout.alignment: Qt.AlignHCenter; Layout.fillHeight: true; verticalAlignment: Text.AlignVCenter; text: root.error || (root.placeQuery ? "Fetching…" : "Name a place in the settings"); color: Theme.text3; size: Theme.sizeSmall }
    }
}
