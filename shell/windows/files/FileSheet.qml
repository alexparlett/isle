import QtQuick
import QtQuick.Layouts
import qs.theme
import qs.ui

// The one thing Files asks over the folder: a name to give, or a question about what is already
// there. It takes the keyboard while it is up and hands it back when it goes.
Item {
    id: root
    // "name" for a field to fill in, "confirm" for a question with no field.
    property string mode: "name"
    property string title: ""
    property string message: ""
    property string acceptLabel: "OK"
    property bool danger: false
    // Shown under the buttons of a question that will be asked about several things.
    property bool offerAll: false
    property alias text: field.text

    // The accept label pressed, with whether "for all the rest" was ticked.
    signal accepted(string value, bool forAll)
    // A second, quieter way out of a question: skip this one, keep both, whatever the caller named.
    property string alternateLabel: ""
    signal alternate(bool forAll)
    signal rejected()

    visible: false
    anchors.fill: parent

    function ask(name) {
        field.text = name || "";
        visible = true;
        if (mode === "name") { field.input.forceActiveFocus(); field.input.selectAll(); }
        else card.forceActiveFocus();
    }
    function close() { visible = false; forAll.checked = false; }

    Rectangle { anchors.fill: parent; color: Qt.alpha(Theme.ink, 0.4) }
    MouseArea { anchors.fill: parent; onClicked: root.rejected() }

    Card {
        id: card
        width: 420
        anchors.centerIn: parent
        focus: true
        Keys.onEscapePressed: root.rejected()

        MouseArea { anchors.fill: parent }

        ColumnLayout {
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.s4 }
            spacing: Theme.s3

            Label { text: root.title; size: Theme.sizeHeading; Layout.fillWidth: true; wrapMode: Text.Wrap }
            Label {
                visible: root.message !== ""
                text: root.message
                color: Theme.text2
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }

            Field {
                id: field
                visible: root.mode === "name"
                Layout.fillWidth: true
                onAccepted: root.accepted(field.text.trim(), false)
            }

            RowLayout {
                visible: root.offerAll
                spacing: Theme.s2
                Toggle { id: forAll }
                Label { text: "Do the same with the rest"; color: Theme.text2 }
            }

            RowLayout {
                Layout.fillWidth: true
                Layout.bottomMargin: Theme.s4
                spacing: Theme.s2
                Button { text: "Cancel"; onClicked: root.rejected() }
                Item { Layout.fillWidth: true }
                Button {
                    visible: root.alternateLabel !== ""
                    text: root.alternateLabel
                    onClicked: root.alternate(forAll.checked)
                }
                Button {
                    variant: root.danger ? "danger" : "accent"
                    text: root.acceptLabel
                    enabled: root.mode !== "name" || field.text.trim() !== ""
                    onClicked: root.accepted(field.text.trim(), forAll.checked)
                }
            }
        }
    }
}
