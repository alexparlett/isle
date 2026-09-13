import QtQuick
import QtQuick.Window
import QtQuick.Layouts
import qs.theme
import qs.ui
import qs.services

SettingsPage {
    id: page
    title: "Drives"
    subtitle: "Cloud storage, mounted as folders."
    Component.onCompleted: Drives.refresh()

    // What the sheet is collecting: which service, a name for the account, and that service's own values.
    property string service: ""
    property string accountName: ""
    property var values: ({})
    readonly property var chosen: Drives.catalogue.find(c => c.id === page.service) || null

    SettingsGroup {
        heading: "Accounts"
        action: Button {
            text: "Add account"
            variant: "text"
            implicitHeight: 26
            enabled: Drives.catalogue.length > 0
            onClicked: {
                page.service = Drives.catalogue.length ? Drives.catalogue[0].id : "";
                page.accountName = "";
                page.values = ({});
                sheet.open();
            }
        }
        Repeater {
            model: Drives.accounts
            SettingsRow {
                id: acct
                required property var modelData
                property string armed: ""
                readonly property var service: Drives.provider(modelData.provider)
                label: modelData.title
                description: (service ? service.name : "") + (modelData.subtitle ? "  ·  " + modelData.subtitle : "")
                RowLayout {
                    spacing: Theme.s2
                    Repeater {
                        model: acct.modelData.actions || []
                        Button {
                            required property var modelData
                            text: acct.armed === modelData.id ? modelData.label + "?" : modelData.label
                            variant: acct.armed === modelData.id ? "danger" : modelData.primary ? "accent" : "text"
                            implicitHeight: 32
                            onClicked: {
                                if (modelData.danger && acct.armed !== modelData.id) { acct.armed = modelData.id; return; }
                                acct.armed = "";
                                Drives.act(acct.modelData.provider, modelData.id, acct.modelData.id);
                            }
                        }
                    }
                }
            }
        }
        SettingsRow { visible: Drives.accounts.length === 0; label: "No account yet" }
    }

    Sheet {
        id: sheet
        title: "Add an account"
        confirmLabel: page.chosen && page.chosen.browser ? "Sign in" : "Add"
        confirmEnabled: page.service !== "" && /^[A-Za-z0-9_-]+$/.test(page.accountName)
        onConfirmed: Drives.add(page.service, page.accountName, page.values)

        RowLayout {
            Layout.fillWidth: true
            Label { text: "Service"; Layout.preferredWidth: 110; color: Theme.text2 }
            Dropdown {
                Layout.fillWidth: true
                listWidth: 240
                options: Drives.catalogue.map(c => [c.id, c.name])
                value: page.service
                onPicked: v => { page.service = v; page.values = ({}); }
            }
        }
        RowLayout {
            Layout.fillWidth: true
            Label { text: "Name"; Layout.preferredWidth: 110; color: Theme.text2 }
            Field { Layout.fillWidth: true; implicitHeight: 34; placeholder: "letters and digits"; onTextChanged: page.accountName = text }
        }
        Repeater {
            model: page.chosen ? page.chosen.fields : []
            RowLayout {
                required property var modelData
                Layout.fillWidth: true
                Label { text: modelData.label; Layout.preferredWidth: 110; color: Theme.text2 }
                Field {
                    Layout.fillWidth: true
                    implicitHeight: 34
                    secret: !!modelData.password
                    placeholder: modelData.required ? "" : "optional"
                    onTextChanged: { const v = Object.assign({}, page.values); v[modelData.name] = text; page.values = v; }
                }
            }
        }
        Label {
            visible: !!page.chosen && page.chosen.browser
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            text: "A terminal opens for the consent page in your browser."
            size: Theme.sizeCaption
            color: Theme.text2
        }
    }
}
