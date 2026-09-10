pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// Lock, sleep, restart, shut down, log out.
Singleton {
    id: root

    Process { id: proc }
    function run(cmd) { proc.command = cmd; proc.running = true; }

    function lock() { Lock.lock(); }
    function sleep() { run(["systemctl", "suspend"]); }
    function hibernate() { run(["systemctl", "hibernate"]); }
    function restart() { run(["systemctl", "reboot"]); }
    function shutdown() { run(["systemctl", "poweroff"]); }
    function logout() { Compositor.dispatch("hl.dsp.exit()"); }
}
