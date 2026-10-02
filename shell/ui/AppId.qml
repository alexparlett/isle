import QtQuick

// Gives the shell window it sits in its own app id, through Isle.Files (D92).
Item {
    id: root
    required property string appId

    // Created at run time, so this file loads without the engine; the window then keeps the shell's id.
    Component.onCompleted: {
        try {
            const setter = Qt.createQmlObject("import Isle.Files; WindowAppId {}", root, "AppId");
            setter.appId = Qt.binding(() => root.appId);
            setter.window = Qt.binding(() => root.Window.window);
        } catch (e) {}
    }
}
