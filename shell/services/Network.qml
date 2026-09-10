pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Networking

// Wi-Fi and wired state through NetworkManager: what is connected, and the networks to connect to.
Singleton {
    id: root

    readonly property var devices: Networking.devices.values
    readonly property var wifiDevice: devices.find(d => d.type === DeviceType.Wifi) || null
    readonly property var wiredDevice: devices.find(d => d.type === DeviceType.Wired) || null

    readonly property bool wifiAvailable: wifiDevice !== null
    readonly property bool wifiEnabled: Networking.wifiEnabled
    readonly property bool wifiConnected: wifiDevice !== null && wifiDevice.connected
    readonly property bool wiredConnected: wiredDevice !== null && wiredDevice.connected
    readonly property bool connected: wifiConnected || wiredConnected
    readonly property bool online: Networking.connectivity === NetworkConnectivity.Full

    // Networks seen by the Wi-Fi device, strongest first, the connected one on top.
    readonly property var networks: {
        if (!wifiDevice) return [];
        const list = wifiDevice.networks.values.slice();
        list.sort((a, b) => (b.connected - a.connected) || (b.known - a.known) || (b.signalStrength - a.signalStrength));
        return list;
    }
    readonly property var activeNetwork: networks.find(n => n.connected) || null

    readonly property string summary: wifiConnected && activeNetwork ? activeNetwork.name
        : wiredConnected ? "Ethernet" : wifiEnabled ? "Not connected" : "Off"

    function setWifiEnabled(on) { Networking.wifiEnabled = on; }
    function scan() { if (wifiDevice) wifiDevice.scannerEnabled = true; }
    function stopScan() { if (wifiDevice) wifiDevice.scannerEnabled = false; }

    function needsPassword(n) { return !n.known && n.security !== WifiSecurityType.None; }
    function connect(n, psk) {
        if (n.known || n.security === WifiSecurityType.None) n.requestConnect();
        else n.requestConnectWithPsk(psk || "");
    }
    function disconnect(n) { n.requestDisconnect(); }
    function forget(n) { n.requestForget(); }

    function glyphFor(n) {
        return n.signalStrength > 0.66 ? "wifi" : n.signalStrength > 0.33 ? "wifi-high" : "wifi-low";
    }
    function securityLabel(n) {
        return n.security === WifiSecurityType.None ? "Open" : "Secured";
    }
}
