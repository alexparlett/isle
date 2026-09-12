# Audit

Each surface against what macOS and Windows users expect of it. "Has"
is verified in the VM; "Gap" is what a daily user would miss; "Wave"
is when it lands. Ticks mark gaps closed since the audit was written.

## Island and control panel

Has: rest pill, hover unfold, event morphs (notification, volume, media,
recording, drive, focus, auth), toggles, output picker, volume,
brightness (backlight), media, batteries, power profile, Wi-Fi and
Bluetooth drill-downs, output drill-down.

| gap | wave |
|---|---|
| Connecting to a secured Wi-Fi network has no password prompt | ✓ 1 |
| No per-app volume mixer and no input device or microphone level | ✓ 1 |
| Desktop monitors have no brightness control (DDC/CI) | ✓ 2 |
| No privacy glyph when the microphone or camera is in use | ✓ 1 |
| No VPN glyph at rest and no VPN toggle (the DND glyph is there) | ✓ 2 |
| Bluetooth pairing gives no confirmation code | ✓ 3 |

## Notifications

Has: server, toast, click for default action, do not disturb, critical
override, list in the dashboard, clear all, age.

| gap | wave |
|---|---|
| Action buttons and inline reply are not shown on the toast | ✓ 1 |
| No per-app muting | ✓ 2 |
| Images in the body are not shown | ✓ 3 |

## Dashboard and widgets

Has: 12-column grid, ten widgets, polling only while open, tray icons.

| gap | wave |
|---|---|
| No edit mode: widgets cannot be moved, resized, added or removed | ✓ 2 |
| Tray icons have no right-click menu | ✓ 2 |
| Widgets have no settings | ✓ 3 |

## Launcher and clipboard

Has: apps, windows, run, calculator, files, clipboard, shell actions,
power actions, keyboard navigation.

| gap | wave |
|---|---|
| No ranking by how often an app is launched | ✓ 1 |
| No web search fallback when nothing matches | ✓ 1 |
| No secondary action (run in terminal, open the file's folder) | ✓ 1 |
| Clipboard images show no thumbnail; entries cannot be deleted or pinned | ✓ 1, pins ✓ 3 |

## Switcher and overview

Has: app switcher on held modifier, same-app cycling, minimise to
hidden, overview with live thumbnails, hidden windows.

| gap | wave |
|---|---|
| Windows cannot be closed from the overview | ✓ 2 |
| Windows cannot be dragged between workspaces | ✓ 3 |
| No keyboard navigation in the overview | ✓ 2 |

## Terminal

| gap | wave |
|---|---|
| The dropdown terminal in the keymap is not implemented | ✓ 1 |

## Capture

Has: region, window, screen; screenshot to file and clipboard; record
with a stop in the island; colour picker; annotate.

| gap | wave |
|---|---|
| No countdown timer, no cursor toggle, no audio in recordings | ✓ 2 |
| No way to open the folder or the last capture from the event | ✓ 2 |

## Lock, idle, power

Has: lock over PAM, logind and idle triggers, power menu, polkit dialog.

| gap | wave |
|---|---|
| Idle "dim" is a flag that dims nothing | ✓ 1 |
| No caps-lock warning and no failed-attempt count on the lock | ✓ 2 |
| No hibernate; no confirmation with a countdown for restart or shut down | ✓ 2 |

## Settings

Has: appearance (accent, wallpaper, motion), keyboard (profiles, table),
displays (read-only), audio, network, bluetooth, notifications, power,
modes, about.

| gap | wave |
|---|---|
| Displays cannot be edited (resolution, scale, position, refresh, VRR) | ✓ 2 |
| No mouse or touchpad page (speed, acceleration, natural scroll) | ✓ 2 |
| No rebinding by pressing a chord (layout, variant, options and repeat are in) | ✓ 3 |
| No light theme, though the tokens define one | ✓ 2 |
| No storage page (drives, SMART, open in Disks) | ✓ 3 |
| No default apps or startup apps | ✓ 3 |

## Keychain, disks, monitor, modes, Big Picture

| gap | wave |
|---|---|
| Keychain has no password generator and no categories | ✓ 3 |
| SSH keys and the agent have no home | ✓ 3 |
| Big Picture cannot be driven by a real gamepad without Steam's lizard mode | ✓ 3 |
| Per-process network, DDC/CI brightness and tray menus unverified in the VM (no NIC mapping, no i2c, no tray apps) | hardware |

## Against KDE Plasma and macOS, September 2026

The first audit measured each surface against what a Windows or macOS
user expects of it, and every gap in it has closed. This one takes the
two desktops whole, Plasma 6 and macOS 15, and asks what a daily user of
either would reach for and not find. "Has" is what Isle does today;
"Gap" is what both of them do and Isle does not; "Worth" is a judgement:
**now** fits the shell's shape and its two workloads, **later** is real
but not urgent, **no** is a thing Isle should not grow. Unverified means
the gap is inferred from the docs and code, not tried.

### Accessibility

Has: reduced motion, an on-screen keyboard, caps-lock warning on the lock.

| gap | worth |
|---|---|
| No screen magnifier (Plasma's zoom effect, macOS Zoom); Hyprland has `misc:cursor_zoom_factor`, which is the whole mechanism | ✓ |
| No large-text or high-contrast toggle; the tokens could scale type and swap the palette in one place | ✓ |
| No sticky, slow or bounce keys, no mouse keys (xkb has them all; one page) | later |
| No screen reader hookup (Orca, AT-SPI) | later |
| No colour filters for colour blindness (a shader on the wallpaper layer is not the screen; needs a compositor effect) | no |

### Windows and workspaces

Has: floating everything with edge snapping, Mission Control with drag
between spaces, an app switcher, hidden stack, kept-in-tray apps, per-app
places, a dropdown terminal, hyprbars with the client's own frame honoured.

| gap | worth |
|---|---|
| Fullscreen apps do not take a space of their own the way macOS does; a game or a video sits on the desktop's space with everything else | ✓ |
| Named or reordered spaces: Plasma names them, macOS keeps their order; Isle numbers them | later |
| No window rules page (Plasma's "special window settings": always on this space, size, no title bar, per app) | later |
| No Stage Manager or window grouping, no tabbed windows (Plasma's tabs went, macOS keeps its own) | no |
| The island lives on one monitor and the rest show nothing at all; both desktops put a bar or a dock on every screen | ✓, one screen here and in the VM, so two is unverified |

### Launcher and search

Has: apps, windows, files by name, clipboard, calculator with units,
shell actions, settings pages, vault items, web fallback, ranking by use.

| gap | worth |
|---|---|
| No file content search (Baloo, Spotlight index); `fd` is names only, and `rga` or `ripgrep-all` would be the whole engine | ✓ |
| No emoji and symbol picker (Plasma has one, macOS the Character Viewer); a `:` prefix or a keymap action over the Unicode tables | ✓ |
| No dictionary or definitions (macOS), no currency at live rates (qalc does units, not rates without a fetch) | later |
| No Quick Look: a space bar preview of a file result, images and PDFs at least | later |
| No Spotlight-style suggestions from the web or apps' own providers (KRunner plugins) | no |

### Files and devices

Has: disks and drives in the island and a widget, mount and eject,
SMART, printers, devices tree, keyboard configurators, Bluetooth pairing.

| gap | worth |
|---|---|
| No phone integration: KDE Connect and macOS Continuity (notifications from the phone, clipboard, file drop, remote input). `kdeconnectd` exists and would be a provider | ✓ 7 |
| No Bluetooth file transfer (OBEX) and no AirDrop-like drop to a nearby machine | later |
| No file sharing or remote login switches (Plasma's SMB share, macOS Sharing); one page over `smb.conf` and `sshd` | later |
| No firewall page (Plasma's firewalld/ufw module, macOS's app firewall) | later |
| No backup page beyond snapshots (Time Machine, Plasma's KBackup, Vorta); a restic or borg provider would fit the providers framework | later |

### Look and feel

Has: dark, light, auto by sunset, accent, wallpaper by folder, cursor
size, title bars on or off, motion, every toolkit themed from one set of
tokens, night light.

| gap | worth |
|---|---|
| No font settings: family and size for the UI, the mono face; the tokens carry them, the page does not | ✓ 8 |
| No wallpaper slideshow, no per-space wallpaper, no dynamic (time-of-day) wallpaper (macOS dynamic desktops, Plasma's slideshow) | later |
| No cursor theme choice (Bibata is the one) and no icon theme choice (Papirus is the one); both are a preference plus a re-render | later |
| No sound theme: event sounds beyond the one notification chime | no |
| No desktop icons or files on the desktop | no |

### Input

Has: keyboard layouts, options, repeat, profiles, mouse and touchpad
speed, acceleration, natural scroll, tap, disable while typing; the keymap
with Mac and Windows chords.

| gap | worth |
|---|---|
| No touchpad gesture settings (three and four finger swipes are fixed in the compositor config) | later |
| No hot corners or screen edges (Plasma's edges, macOS's corners) | later |
| No text replacement or expansion (macOS), no compose key page | later |
| No per-device settings for a second mouse or keyboard | no |

### Notifications and focus

Has: server, toasts with actions and reply, do not disturb with quiet
hours and a fullscreen rule, per-app muting, allow-through list, history,
summary when silence lifts, centre in the panel.

| gap | worth |
|---|---|
| No focus modes beyond DND: macOS's Work and Personal with their own allow lists and schedules; Plasma has DND only, so Isle already matches Plasma | later |
| No notification grouping by app in the centre (macOS stacks) | later |
| No lock-screen notifications (macOS shows them, Plasma can) | later |

### Sessions and accounts

Has: users page, greeter, lock, idle, power menu with countdowns,
hibernate, apps at login, services, kept-running apps.

| gap | worth |
|---|---|
| No session restore: the windows open at logout do not come back (Plasma restores, macOS reopens); the window places are half of it | now |
| No online accounts (Plasma's KAccounts, macOS's Internet Accounts); the calendar and vault providers are the honest version of this and should stay the pattern | no |
| No screen time or usage stats | no |
| No first-run welcome (Plasma's welcome centre, macOS setup) | later |

### Media and capture

Has: media in the island and a widget, output picker, per-app mixer,
microphone level, capture of region, window and screen, recording with
audio and cursor, picker, annotate, privacy glyphs.

| gap | worth |
|---|---|
| No screen sharing indicator with a stop for a portal cast (macOS's purple dot menu, Plasma's KPipeWire indicator); the recording dot is Isle's own captures only | ✓ |
| No casting to a TV or speaker (AirPlay, Plasma's none either) | no |
| No system-wide equaliser (neither ships one) | no |

### Gaming

Has: game mode, Big Picture with the pad, Steam interop, an on-screen
keyboard, controller battery, tearing and VRR, the pad bridge.

| gap | worth |
|---|---|
| No controller page: buttons test, dead zones, which pad is which (Plasma 6.x has one) | now |
| No performance overlay switch (MangoHud on or off, a keymap action) | later |
| No per-game settings (Proton version, launch options) beyond Steam's own | no |

### What to build first

The island on every monitor, fullscreen apps on their own space, file
content search, an emoji picker, a screen-sharing indicator, a magnifier
and large-text toggle, phone integration as a provider, fonts in
Appearance, session restore, and a controller page. Everything marked
**no** is a line Isle draws on purpose: it is a shell for one person at a
desk with games and agents, not a platform.
