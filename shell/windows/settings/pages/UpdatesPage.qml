import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    title: "Updates"
    subtitle: "The shell from its repository, and packages from the repositories through pacman" + (Updates.helper ? ", and the AUR through " + Updates.helper : "") + ". Updating runs in a terminal so it can ask."

    Component.onCompleted: { Updates.check(); IsleUpdate.check(); }

    SettingsGroup {
        heading: "Isle"
        SettingsRow {
            label: IsleUpdate.checking ? "Checking" : IsleUpdate.error ? "Could not check" : IsleUpdate.behind > 0 ? IsleUpdate.behind + (IsleUpdate.behind === 1 ? " commit" : " commits") + " behind" : "Up to date"
            description: IsleUpdate.head ? "At " + IsleUpdate.head + " from " + IsleUpdate.headDate + (IsleUpdate.checkedAt ? ". Checked at " + IsleUpdate.checkedAt + "." : ".") + (IsleUpdate.error ? " " + IsleUpdate.error : "") + (IsleUpdate.viaHttps ? " Fetched over https; origin " + IsleUpdate.remote + " refused this machine." : "") : "This checkout has no git history, so it cannot pull. Clone the repository instead."

            RowLayout {
                spacing: Theme.s2
                Button { text: "Check now"; variant: "text"; enabled: !IsleUpdate.checking; onClicked: IsleUpdate.check() }
                Button { text: "Update"; glyph: "refresh-cw"; variant: "accent"; enabled: IsleUpdate.behind > 0; onClicked: IsleUpdate.update() }
            }
        }
        Repeater {
            model: IsleUpdate.incoming.slice(0, 8)
            SettingsRow { required property string modelData; label: modelData; description: "" }
        }
    }

    SettingsGroup {
        heading: "Packages"
        SettingsRow {
            label: Updates.checking ? "Checking" : Updates.count === 0 ? "Up to date" : Updates.count + (Updates.count === 1 ? " update" : " updates") + " available"
            description: Updates.checkedAt ? "Checked at " + Updates.checkedAt + ". Checks again every hour." : ""
            RowLayout {
                spacing: Theme.s2
                Button { text: "Check now"; variant: "text"; enabled: !Updates.checking; onClicked: Updates.check() }
                Button { text: "Update everything"; glyph: "download"; variant: "accent"; enabled: Updates.count > 0; onClicked: Updates.update() }
            }
        }
    }

    SettingsGroup {
        heading: "Pending"
        visible: Updates.count > 0
        Repeater {
            model: Updates.pending
            SettingsRow {
                required property var modelData
                label: modelData.name
                description: modelData.from + "  →  " + modelData.to + (modelData.aur ? "  ·  AUR" : "")
                Button { text: "Update"; variant: "text"; onClicked: Updates.updateOne(modelData) }
            }
        }
    }

    SettingsGroup {
        heading: "Tools"
        SettingsRow { label: "Package cache"; description: "Old package versions pile up in /var/cache/pacman/pkg."; Button { text: "Clean"; variant: "text"; onClicked: Compositor.exec("kitty --class isle-windows -e sh -c 'sudo paccache -rk2; echo; echo Done. Press Enter.; read x'") } }
        SettingsRow { label: "Orphans"; description: "Packages nothing depends on any more."; Button { text: "Review"; variant: "text"; onClicked: Compositor.exec("kitty --class isle-windows -e sh -c 'o=$(pacman -Qtdq); if [ -n \"$o\" ]; then echo \"$o\"; echo; sudo pacman -Rns $o; else echo No orphans.; fi; echo; echo Done. Press Enter.; read x'") } }
        SettingsRow { label: "Log"; description: "/var/log/pacman.log"; Button { text: "Open"; variant: "text"; onClicked: Compositor.exec("kitty --class isle-windows -e sh -c 'tail -n 200 /var/log/pacman.log | less +G'") } }
    }
}
