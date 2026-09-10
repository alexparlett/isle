//@ pragma IconTheme Papirus-Dark
import QtQuick
import Quickshell
import qs.services
import qs.surfaces.wallpaper
import qs.surfaces.island
import qs.surfaces.dashboard
import qs.surfaces.launcher
import qs.surfaces.switcher
import qs.surfaces.overview
import qs.surfaces.capture
import qs.surfaces.lock
import qs.surfaces.auth
import qs.surfaces.windows
import qs.surfaces.power
import qs.surfaces.bigpicture
import qs.surfaces.keyboard
import qs.surfaces.idle
import qs.windows.settings
import qs.windows.keychain
import qs.windows.monitor

ShellRoot {
    // Singletons are created on first reference; these must exist from the start.
    Component.onCompleted: { Notifications.count; Media.present; Audio.ready; Surfaces.dashboard; Widgets.ids; Clipboard; Switcher.open; Capture.mode; Auth.active; Idle.dimmed; Keyboard.devices; Theming.script; Games.detected; Disks.volumes; KeychainService.available; Startup.entries; Gamepad.active; Osk.open; SshKeys.agent; SettingsIndex.entries; Users.users; DateTime.timezone; Updates.count; SystemLocale.lang; Pip.placed; Terminal.exists; Vpn.available; Displays.monitors; Input.p; Lock.surfaceComponent = lockSurface; }

    Wallpaper {}
    Island { id: island }
    HotZone { onPeek: { island.peek = true; island.peekTimer.restart(); } }
    Dashboard {}
    Launcher {}
    Switcher {}
    Overview {}
    CaptureBar {}
    FreezeFrame {}
    AuthDialog {}
    PowerMenu {}
    BigPicture {}
    OnScreenKeyboard {}
    Dimmer {}
    SnapGhost {}
    Settings {}
    Keychain {}
    Monitor {}
    Component { id: lockSurface; LockSurface {} }
}
