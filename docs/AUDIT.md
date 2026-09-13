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

Against Finder. The first list is what a hand does in the first minute,
because that is where this was found wanting: a list of features missed
every one of these.

Has: list, columns and grid; thumbnails from the shared cache; places with
bookmarks, devices, Recents and the Trash; tabs and as many windows as are
wanted; a menu bar and a context menu; a breadcrumb with back and forward
per tab; sortable columns; multiple selection; renaming and new folders on
the row; copy, cut, paste, duplicate, trash, put back, delete; pack and
unpack; rename many; one step of undo; go to folder; what is free; the file
chooser every application gets.

### What the hand does

| gap | |
|---|---|
| Space does nothing; there is no Quick Look | the fastest way to see a file without opening it |
| Enter opens rather than renames | Finder's way round, and the audit should say which Isle wants |
| No Get Info on a selection | size, kind, where, when, permissions |
| Dragging a file onto a folder has never been seen to work | |
| Dragging a folder to the sidebar has never been seen to work | |
| No arrow-key navigation into and out of folders in columns | left and right are how columns are walked |
| Double click on a folder's empty space does not go up | Finder does not either, but Windows hands expect it |
| No middle click to open in a new tab | |
| No Ctrl+click or long press on back for the trail | |
| Column widths cannot be dragged | |
| The sidebar cannot be reordered by dragging | |
| Selection is lost when the folder is rescanned | a file appearing elsewhere in the folder drops what was picked |
| No rubber-band selection by dragging over rows | |
| No Home/End/PageUp/PageDown in the list | |
| Type-ahead does not jump to a name | |

### What it does not have

| gap | |
|---|---|
| No preview pane | Shift+Cmd+P |
| Folders in the list do not open in place on a triangle | Finder's list nests |
| The grid has one icon size | Finder has a slider |
| No sorting or grouping menu beyond the column headers | by kind, date, size |
| No tags | the one Finder idea with no equivalent elsewhere |
| Search does not look under the folder, only in it | |
| No saved searches, no Recents beyond the list | |
| A folder on another volume trashes to the home volume's trash | |
| No Connect to server | |
| No Get Info on a folder's size | Finder counts on demand |
