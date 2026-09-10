import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Language and region"
    subtitle: "The system locale through localectl. Apps pick the change up when they next start; the session, at the next login."

    Component.onCompleted: SystemLocale.refresh()
    readonly property var options: SystemLocale.available.map(l => [l, SystemLocale.label(l)])
    readonly property var formatOptions: [["", "Same as language"]].concat(options)

    SettingsGroup {
        heading: "Language"
        SettingsRow {
            label: "Language"
            description: SystemLocale.lang ? SystemLocale.label(SystemLocale.lang) : "not set"
            Dropdown { listWidth: 280; maxRows: 12; options: page.options.length ? page.options : [[SystemLocale.lang, SystemLocale.lang]]; value: SystemLocale.lang; onPicked: v => SystemLocale.apply(v, SystemLocale.time, SystemLocale.numeric, SystemLocale.monetary) }
        }
        SettingsRow {
            label: "More languages"
            description: "Only generated locales are offered. Uncomment lines in /etc/locale.gen and run locale-gen to add one."
            Button { text: "Edit locale.gen"; variant: "text"; onClicked: Compositor.exec("kitty --class isle-windows -e sh -c 'sudo ${EDITOR:-nano} /etc/locale.gen && sudo locale-gen; echo; echo Done. Press Enter.; read x'") }
        }
    }

    SettingsGroup {
        heading: "Formats"
        SettingsRow {
            label: "Dates and times"
            description: SystemLocale.time ? SystemLocale.label(SystemLocale.time) : "follow the language"
            Dropdown { listWidth: 280; maxRows: 12; options: page.formatOptions; value: SystemLocale.time; onPicked: v => SystemLocale.apply(SystemLocale.lang, v, SystemLocale.numeric, SystemLocale.monetary) }
        }
        SettingsRow {
            label: "Numbers"
            description: SystemLocale.numeric ? SystemLocale.label(SystemLocale.numeric) : "follow the language"
            Dropdown { listWidth: 280; maxRows: 12; options: page.formatOptions; value: SystemLocale.numeric; onPicked: v => SystemLocale.apply(SystemLocale.lang, SystemLocale.time, v, SystemLocale.monetary) }
        }
        SettingsRow {
            label: "Currency"
            description: SystemLocale.monetary ? SystemLocale.label(SystemLocale.monetary) : "follow the language"
            Dropdown { listWidth: 280; maxRows: 12; options: page.formatOptions; value: SystemLocale.monetary; onPicked: v => SystemLocale.apply(SystemLocale.lang, SystemLocale.time, SystemLocale.numeric, v) }
        }
        SettingsRow {
            label: "Example"
            description: "How a date, a number and an amount look in the chosen formats."
            Label { text: Qt.locale((SystemLocale.time || SystemLocale.lang).split(".")[0]).toString(new Date(), Locale.ShortFormat) + "   " + Number(1234567.89).toLocaleString(Qt.locale((SystemLocale.numeric || SystemLocale.lang).split(".")[0])) + "   " + Number(42.5).toLocaleCurrencyString(Qt.locale((SystemLocale.monetary || SystemLocale.lang).split(".")[0])); mono: true; size: Theme.sizeSmall; color: Theme.text2 }
        }
        SettingsRow { visible: SystemLocale.error !== ""; label: "That did not work"; description: SystemLocale.error }
    }

    SettingsGroup {
        heading: "Keyboard"
        SettingsRow {
            label: "Layout"
            description: "Lives with the keyboard settings."
            Button { text: "Open"; variant: "text"; onClicked: Surfaces.settingsPage = "keyboard" }
        }
    }
}
