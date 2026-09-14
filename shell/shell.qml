//@ pragma IconTheme Papirus-Dark
import QtQuick
import Quickshell
import Quickshell.Io
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
import qs.windows.files

ShellRoot {
    // Singletons are created on first reference; these must exist from the start.
    Component.onCompleted: { Notifications.count; Media.present; Audio.ready; Surfaces.dashboard; Widgets.ids; Clipboard; Switcher.open; Capture.mode; Auth.active; Idle.dimmed; Keyboard.devices; Theming.script; Games.detected; Disks.volumes; KeychainService.available; Startup.entries; Gamepad.active; Osk.open; SshKeys.agent; SettingsIndex.entries; Users.users; DateTime.timezone; Updates.count; IsleUpdate.behind; SystemLocale.lang; Pip.placed; Terminal.exists; Vpn.available; Displays.monitors; Input.p; Calendars.events; Snapshots.providers; Phones.providers; Access.zoom; Restore.enabled; Windows.apps; AppImages.folder; Lock.surfaceComponent = lockSurface; }

    Wallpaper {}
    // An island and its hot zone on every screen.
    Variants {
        id: islands
        model: Quickshell.screens
        Scope {
            id: perScreen
            required property var modelData
            readonly property alias island: isl
            Island { id: isl; screen: perScreen.modelData }
            HotZone { screen: perScreen.modelData; onPeek: { isl.peek = true; isl.peekTimer.restart(); } }
        }
    }
    // The island's IPC speaks to the one on the shell's screen; events are shared by all.
    readonly property var primaryIsland: { const s = islands.instances.find(i => i.island.primary); return s ? s.island : (islands.instances[0] ? islands.instances[0].island : null); }
    IpcHandler {
        target: "island"
        function text(text: string): void { IslandEvents.show({ kind: "text", text: text, glyph: "check" }); }
        function pin(on: bool): void { if (primaryIsland) primaryIsland.pinned = on; }
        function notifications(): void { if (primaryIsland) primaryIsland.showPage("notifications"); }
        function clear(): void { IslandEvents.queue = []; IslandEvents.dismiss(); }
        function dnd(action: string): void { Notifications.setDnd(action === "on" ? true : action === "off" ? false : !Notifications.dnd); }
        // Sample events for design review: osd, media, text.
        function demo(kind: string): void {
            const d = 6000;
            if (kind === "osd") IslandEvents.show({ kind: "osd", duration: d, glyph: "volume-2", value: 0.64, label: "64" });
            else if (kind === "media") IslandEvents.show({ kind: "media", duration: d, title: "Nightcall", artist: "Kavinsky", artUrl: "" });
            else IslandEvents.show({ kind: "text", duration: d, text: "Copied to clipboard", glyph: "check" });
        }
    }
    Dashboard {}
    Launcher {}
    Switcher {}
    Overview {}
    CaptureBar {}
    FreezeFrame {}
    AuthDialog {}
    PowerMenu {}
    BigPictureHome {}
    BigPictureQuick {}
    OnScreenKeyboard {}
    Dimmer {}
    SnapGhost {}
    Settings {}
    Keychain {}
    Monitor {}
    // Files needs the compiled browsing engine, built at install time and able to be absent or stale.
    // The component is made at run time so a module that will not load is Files failing to open, not
    // the shell failing to start; the import above only registers the directory, which is what lets
    // Files.qml name its own sibling.
    LazyLoader { loading: true; component: Qt.createComponent(Qt.resolvedUrl("windows/files/Files.qml")) }
    // The portal chooser needs the same engine, and is held the same way for the same reason.
    LazyLoader { loading: true; component: Qt.createComponent(Qt.resolvedUrl("surfaces/filechooser/FileChooser.qml")) }
    Component { id: lockSurface; LockSurface {} }
}
