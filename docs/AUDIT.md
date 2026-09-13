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

## Files

Against Finder and against what a hand reaches for. The "has" list is read
off the code — every shortcut, every menu item, every gesture that is
actually wired — rather than remembered, because every earlier version of
this audit was written from memory of Finder's feature list and missed the
faults that using it found in a minute: no deselecting, a right click
collapsing a selection, a new folder asking in a card, an AppImage opening
in the browser.

### Wired

Places: Recents, the person's own folders, bookmarks that can be dragged
into the order they are wanted, mounted devices with eject, the Trash of
every volume at once, saved searches. Tabs and as many windows as are
wanted. Menu bar (File, Edit, View, Go), context menu on a row and on the
folder, Menu/Shift+F10. List, columns and grid; a preview beside the
folder; folders that open in place on a triangle; thumbnails from the
shared cache; group headings by kind, date or size; sort by name, size,
date or kind on the headers or the menu; column widths dragged on the
header; icon size on Ctrl+plus and Ctrl+minus.

Keys: Alt with the arrows and Backspace to walk; Ctrl+H hidden; F5 refresh;
Ctrl+F search, Enter to search underneath; Ctrl+1/2/3 view; Ctrl+Shift+P
preview; F2 rename; Ctrl+Shift+N folder, Ctrl+Shift+F file; Ctrl+C/X/V;
Delete to trash, Shift+Delete for good; Ctrl+Z; Ctrl+A; Ctrl+T tab, Ctrl+W
close, Ctrl+N window, Ctrl+Tab between tabs; Ctrl+D duplicate; Ctrl+L go to
folder; Space Quick Look; Ctrl+I info; Home/End/PageUp/PageDown;
type-ahead; the arrows to walk in and out of a nested folder and, in
columns, between and within the columns.

Mouse: click to pick, Ctrl to add, Shift for a run, a band drawn across
rows, click on nothing to let go, double click to open, middle click for a
tab, right click keeping a selection it is already part of; dragging rows
and grid cells onto a folder, onto another window, out to other
applications, and onto the sidebar to bookmark.

Acts: copy, cut, paste, duplicate, rename on the row, rename many at once,
new file and folder on the row, move to trash, put back, empty trash,
delete, pack, unpack, open with, make an app the standing one for the type,
open in terminal, one step of undo. Searching narrows by kind and by how
lately something changed, and a search can be saved.

### What a hand still reaches for and does not find

| gap | |
|---|---|
| No tags, and no colour on a row | out of the slice's scope |
| No Connect to server | out of the slice's scope: no SMB or NFS yet |
| Sorting and grouping are not remembered per folder | remembered for all folders at once |
| A saved search is a term and its narrowings, not a live query | there is no index to query |
