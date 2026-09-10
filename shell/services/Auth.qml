pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.Polkit

// The polkit agent: apps asking for privileges get the auth dialog.
Singleton {
    id: root

    // The agent's current flow, or null.
    readonly property var flow: agent.flow || null
    readonly property bool active: agent.isActive && flow !== null && !flow.isCompleted && !flow.isCancelled
    property string password: ""
    property bool failed: false

    PolkitAgent {
        id: agent
        onAuthenticationRequestStarted: {
            root.password = "";
            root.failed = false;
            if (agent.flow) IslandEvents.show({ kind: "text", duration: 3000, glyph: "key-round", text: "Authentication required", detail: agent.flow.message || "" });
        }
    }

    Connections {
        target: root.flow
        function onAuthenticationFailed() { root.failed = true; root.password = ""; }
    }

    function submit() {
        if (!flow || password === "") return;
        failed = false;
        flow.submit(password);
        password = "";
    }
    function cancel() { if (flow) flow.cancelAuthenticationRequest(); }
}
