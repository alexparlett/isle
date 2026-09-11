import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Calendars"
    subtitle: "Calendars you subscribe to by link: the Calendar widget shows their events and the island says when one starts."

    function ago(t) {
        if (!t) return "never";
        const m = Math.round((Date.now() / 1000 - t) / 60);
        return m < 1 ? "just now" : m < 60 ? m + "m ago" : m < 1440 ? Math.round(m / 60) + "h ago" : Math.round(m / 1440) + "d ago";
    }

    SettingsGroup {
        heading: "Subscribed"
        Repeater {
            model: Calendars.subscriptions
            SettingsRow {
                id: row
                required property var modelData
                readonly property var st: Calendars.statusOf(modelData.id)
                label: modelData.name
                description: (st ? (st.ok ? st.count + (st.count === 1 ? " event" : " events") + "  ·  fetched " + page.ago(st.fetched) : st.error) : "Waiting") + "  ·  " + modelData.url
                RowLayout {
                    spacing: Theme.s2
                    Dropdown { listWidth: 120; options: Calendars.palette; value: row.modelData.color || "accent"; onPicked: v => Calendars.update(row.modelData.id, { color: v }) }
                    Button { text: "Remove"; variant: "text"; onClicked: Calendars.remove(row.modelData.id) }
                }
            }
        }
        SettingsRow { visible: Calendars.subscriptions.length === 0; label: "No calendars yet"; description: "Proton Calendar: Share via link on the calendar, then paste the link here. Google, Outlook and any .ics address work the same way." }
        SettingsRow {
            label: "Add a calendar"
            description: "A link to an .ics file, webcal or https, or a file on this machine."
            RowLayout {
                spacing: Theme.s2
                Field { id: urlField; implicitWidth: 320; implicitHeight: 32; placeholder: "https://…/calendar.ics" }
                Field { id: nameField; implicitWidth: 150; implicitHeight: 32; placeholder: "Name (optional)" }
                Button {
                    text: "Add"; variant: "accent"; implicitHeight: 32
                    enabled: urlField.text.trim().length > 0
                    onClicked: { Calendars.add(nameField.text.trim(), urlField.text.trim()); urlField.text = ""; nameField.text = ""; }
                }
            }
        }
        SettingsRow {
            visible: Calendars.subscriptions.length > 0
            label: "Refresh"
            description: "Every half hour by itself; a shared link has nothing to push, so this is as fresh as it gets."
            Button { text: Calendars.busy ? "Fetching…" : "Fetch now"; variant: "text"; enabled: !Calendars.busy; onClicked: Calendars.refresh(false) }
        }
    }
}
