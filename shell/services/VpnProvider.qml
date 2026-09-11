import QtQuick
import qs.services

// A VPN behind Vpn: a Provider whose list is the places it can connect to and whose status carries the
// connection that is up, if any.
Provider {
    id: root
    listKey: "choices"
    // [{ id, title, subtitle }]: countries, servers or connections; "" as a choice means the provider's default.
    readonly property var choices: listed
    // { name, detail } while connected, else null.
    readonly property var active: status && status.active ? status.active : null
    function connect(choice) { act("connect", choice || ""); }
    function disconnect() { act("disconnect", ""); }
}
