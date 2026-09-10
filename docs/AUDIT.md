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
