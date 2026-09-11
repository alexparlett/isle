import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Updates"
    subtitle: "The shell from its repository, and packages from the repositories through pacman" + (Updates.helper ? ", and the AUR through " + Updates.helper : "") + ". Permission is asked for once, in a dialog."

    Component.onCompleted: { Updates.check(); IsleUpdate.check(); }

    SettingsGroup {
        visible: Updates.restartNeeded
        heading: "Restart needed"
        SettingsRow {
            label: "An update is waiting for a restart"
            Button { text: "Restart"; glyph: "rotate-cw"; variant: "accent"; onClicked: Session.restart() }
        }
    }

    SettingsGroup {
        heading: "Isle"
        SettingsRow {
            label: IsleUpdate.running ? IsleUpdate.phase : IsleUpdate.checking ? "Checking" : IsleUpdate.error ? "Could not check" : IsleUpdate.behind > 0 ? "An update is available" : "Up to date"
            description: IsleUpdate.running ? (IsleUpdate.log[IsleUpdate.log.length - 1] || "")
                : IsleUpdate.failed ? IsleUpdate.failed
                : IsleUpdate.head ? "At " + IsleUpdate.head + " from " + IsleUpdate.headDate + (IsleUpdate.checkedAt ? ". Checked at " + IsleUpdate.checkedAt + "." : ".") + (IsleUpdate.error ? " " + IsleUpdate.error : "") + (IsleUpdate.viaHttps ? " Fetched over https; origin " + IsleUpdate.remote + " refused this machine." : "")
                : "This checkout has no git history, so it cannot pull. Clone the repository instead."
            RowLayout {
                spacing: Theme.s2
                Spinner { visible: IsleUpdate.running; Layout.rightMargin: Theme.s2 }
                Button { text: "Check now"; variant: "text"; enabled: !IsleUpdate.checking && !IsleUpdate.running; onClicked: IsleUpdate.check() }
                Button { text: "Update"; glyph: "refresh-cw"; variant: "accent"; enabled: IsleUpdate.behind > 0 && !IsleUpdate.running; onClicked: IsleUpdate.update() }
            }
        }
    }

    SettingsGroup {
        heading: "Packages"
        SettingsRow {
            label: Updates.running ? Updates.phase : Updates.checking ? "Checking" : Updates.count === 0 ? "Up to date" : Updates.count + (Updates.count === 1 ? " update" : " updates") + " available"
            description: Updates.running ? (Updates.log[Updates.log.length - 1] || "")
                : Updates.failed ? Updates.failed
                : Updates.checkedAt ? "Checked at " + Updates.checkedAt + ". Checks again every hour." : ""
            RowLayout {
                spacing: Theme.s2
                Spinner { visible: Updates.running; Layout.rightMargin: Theme.s2 }
                Button { text: "Check now"; variant: "text"; enabled: !Updates.checking && !Updates.running; onClicked: Updates.check() }
                Button { text: "Update everything"; glyph: "download"; variant: "accent"; enabled: Updates.count > 0 && !Updates.running; onClicked: Updates.update() }
            }
        }
        // Every pending package, and while a run goes, what is happening to each.
        Repeater {
            model: Updates.running || Updates.steps.length ? Updates.steps : Updates.pending.map(p => ({ name: p.name, state: "", from: p.from, to: p.to, aur: p.aur }))
            SettingsRow {
                id: pkg
                required property var modelData
                readonly property var pend: Updates.pending.find(p => p.name === modelData.name) || null
                label: modelData.name
                description: (pend ? pend.from + "  →  " + pend.to + (pend.aur ? "  ·  AUR" : "") : "")
                RowLayout {
                    spacing: Theme.s2
                    Label {
                        visible: !!pkg.modelData.state
                        text: ({ waiting: "Waiting", downloading: "Downloading", installing: "Installing", done: "Done", removed: "Removed" })[pkg.modelData.state] || pkg.modelData.state
                        size: Theme.sizeCaption; color: pkg.modelData.state === "done" ? Theme.ok : pkg.modelData.state === "waiting" ? Theme.text3 : Theme.accent
                    }
                    Glyph { visible: pkg.modelData.state === "done"; name: "check"; size: 14; color: Theme.ok }
                    Spinner { visible: pkg.modelData.state === "downloading" || pkg.modelData.state === "installing" }
                    Button { visible: !pkg.modelData.state && !Updates.running; text: "Update"; variant: "text"; onClicked: Updates.updateOne(pkg.modelData) }
                }
            }
        }
    }

    // What was said, for when a run stops short or someone wants to see.
    SettingsGroup {
        visible: Updates.log.length > 0 && (Updates.running || Updates.failed !== "")
        heading: "Output"
        SettingsRow {
            label: ""
            description: Updates.log.slice(-8).join("\n")
        }
    }

    SettingsGroup {
        heading: "Tools"
        SettingsRow { label: "Package cache"; description: "Old package versions pile up in /var/cache/pacman/pkg; the last two of each are kept."; Button { text: "Clean"; variant: "text"; enabled: !Updates.running; onClicked: Updates.cleanCache() } }
        SettingsRow { label: "Orphans"; description: "Packages nothing depends on any more are removed."; Button { text: "Remove"; variant: "text"; enabled: !Updates.running; onClicked: Updates.removeOrphans() } }
        SettingsRow { label: "Log"; description: "/var/log/pacman.log"; Button { text: "Open"; variant: "text"; onClicked: Compositor.exec("xdg-open /var/log/pacman.log") } }
    }
}
