pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Bluetooth as Bt

// Adapter power, the devices currently connected, and the paired ones to connect to.
Singleton {
    id: root

    readonly property var adapter: Bt.Bluetooth.defaultAdapter
    readonly property bool available: adapter !== null
    readonly property bool enabled: available && adapter.enabled
    readonly property bool discovering: available && adapter.discovering

    readonly property var devices: Bt.Bluetooth.devices.values
    // A list is only recomputed when the list of devices changes, not when one of them connects or pairs,
    // so every device's state changes bump `stamp` and the lists depend on it.
    property int stamp: 0
    Instantiator {
        model: Bt.Bluetooth.devices
        delegate: Connections {
            required property var modelData
            target: modelData
            function onConnectedChanged() { root.stamp++; }
            function onPairedChanged() { root.stamp++; }
            function onTrustedChanged() { root.stamp++; }
            function onNameChanged() { root.stamp++; }
        }
    }
    readonly property var connected: { stamp; return devices.filter(d => d.connected); }
    readonly property var paired: { stamp; return devices.filter(d => d.paired || d.trusted); }
    readonly property var idle: { stamp; return devices.filter(d => !d.connected && (d.paired || d.trusted)); }
    // What can be paired: named, not yet paired, and not a device's low-energy side ("LE_…", no audio or
    // input on it) when that is all bluez has seen of it.
    readonly property var nearby: { stamp; return devices.filter(d => !d.paired && !d.trusted && d.name !== "" && d.name.indexOf("LE_") !== 0); }
    readonly property bool anyConnected: connected.length > 0
    readonly property string primaryName: anyConnected ? connected[0].name : ""

    function setEnabled(on) { if (available) adapter.enabled = on; }
    function setDiscovering(on) { if (available) adapter.discovering = on; }
    function toggle(device) { if (device.connected) device.disconnect(); else device.connect(); }

    // --- the pairing agent ---------------------------------------------------------
    // bluetoothctl answers BlueZ's agent calls; its prompts are read here and the answer written back.
    // request: { kind: "confirm" | "pin" | "passkey" | "display", code, device }
    property var request: null
    readonly property var pairingDevice: devices.find(d => d.pairing) || null
    property string agentBuffer: ""

    Process {
        id: agent
        command: ["bluetoothctl"]
        running: root.available
        stdinEnabled: true
        onStarted: agent.write("agent KeyboardDisplay\ndefault-agent\n")
        stdout: SplitParser {
            splitMarker: ""
            onRead: chunk => {
                const text = (root.agentBuffer + chunk).replace(/\x1b\[[0-9;]*[A-Za-z]/g, "");
                let m;
                if ((m = text.match(/Confirm passkey (\d+) \(yes\/no\)/))) root.ask("confirm", m[1]);
                else if ((m = text.match(/(?:Passkey|PIN code):?\s+(\d+)/))) root.ask("display", m[1]);
                else if (text.match(/Enter PIN code:/)) root.ask("pin", "");
                else if (text.match(/Enter passkey \(number in 0-999999\):/)) root.ask("passkey", "");
                else if (text.match(/Authorize service [0-9a-f-]+ \(yes\/no\)/)) { agent.write("yes\n"); root.agentBuffer = ""; return; }
                else { root.agentBuffer = text.slice(-400); return; }
                root.agentBuffer = "";
            }
        }
    }
    Timer { id: expire; interval: 60000; onTriggered: root.request = null }
    function ask(kind, code) {
        const d = pairingDevice;
        request = { kind: kind, code: code, device: d ? (d.name || d.address) : "a device" };
        expire.restart();
        if (kind === "confirm") IslandEvents.show({ kind: "text", duration: 30000, glyph: "bluetooth", text: "Pair " + request.device, detail: "Code " + code,
            actions: [{ label: "Pair", run: () => root.answer(true) }, { label: "Cancel", run: () => root.answer(false) }] });
        else if (kind === "display") IslandEvents.show({ kind: "text", duration: 30000, glyph: "bluetooth", text: "Type " + code + " on " + request.device, detail: "then Enter" });
    }
    function answer(yes) { agent.write(yes ? "yes\n" : "no\n"); request = null; IslandEvents.dismissKind("text"); }
    function reply(text) { agent.write(text + "\n"); request = null; }

    // A device's icon is a freedesktop name ("audio-headset"); map the common ones to glyphs.
    function glyphFor(device) {
        const i = device.icon || "";
        if (i.indexOf("headset") >= 0 || i.indexOf("headphone") >= 0) return "headphones";
        if (i.indexOf("gaming") >= 0 || i.indexOf("joystick") >= 0) return "gamepad-2";
        if (i.indexOf("keyboard") >= 0) return "keyboard";
        if (i.indexOf("mouse") >= 0) return "mouse";
        if (i.indexOf("phone") >= 0) return "smartphone";
        return "bluetooth";
    }
}
