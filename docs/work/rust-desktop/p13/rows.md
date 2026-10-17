# P13 · The rows

Phase 13.1's gate (`p13-settings.md` choice 3): every pane's subtitle,
groups and rows, and every sheet and sub-page, for the 21 panes of choice
1. Drawn on `rows-1.html`. Binding: `p1/structure/settings.md` (row and
control kinds, copy), `p1/surfaces/settings.md` (sizes), `copy.md`.

A row is here only when something reads its setting today or P13 adds it
(choice 2). Keys are `settings.kdl`'s, kebab-case as the file writes them;
a keyed section's entry is `<id>` (`output.<id>.mode`). *New* marks a key
P13 adds. A daemon row names the service crate or D-Bus call it goes
through. "Keep or revert" marks a change that applies at once and then
asks (choice 19); every other change applies at once and stays.

Kinds: **C** control, **N** navigation, **D** details, **L** list, **A**
action, **V** live (`settings.md`, Rows). Read-only rows are **N** without a
chevron: label and value, Copy in their context menu.

## Group 1

### Wi-Fi

*Join wireless networks and manage the ones you know.*

| Group | Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|---|
| — | Wi-Fi | C · switch | | NetworkManager `WirelessEnabled` (`isle-network`) |
| — | {network}, "Connected" under it; Wi-Fi glyph in an accent circle | D · "Details…" | | the active connection |
| Known networks | {network}, with lock and signal glyphs | C · ⋯ menu: Auto-Join (checked), Forget This Network… | | NM saved connections: `autoconnect`, delete (*crate addition*) |
| Other networks | {network}, lock and signal; "Connect" on hover | A | the joining prompt is the compositor's (B12) | NM scan, only while shown |
| Other networks (foot) | "Join other network…" | D · button under the group | | the sheet below |
| Missing | "No Wi-Fi device is connected." | state | | |

**Wi-Fi Details…** (sheet, 520; header: the network's name and signal)

| Row | Kind · control | Bound to |
|---|---|---|
| Automatically join this network | C · switch | the connection's `autoconnect` |
| IP address | N, read only | device details (*crate addition*) |
| Router | N, read only | same |
| DNS servers | N, read only, one per line | same |
| Wi-Fi address | N, read only | the device's MAC |
| Security | N, read only ("WPA3 Personal") | the connection |
| Forget This Network… (leading, `colour-danger-text`), Done (default) | A · buttons | delete the connection, after an alert |

**Join other network…** (sheet, 400)

| Row | Kind · control | Choices | Bound to |
|---|---|---|---|
| Network name | C · text field | | the new connection's SSID |
| Security | C · pop-up | None, WPA2/WPA3 Personal, WPA3 Personal, WPA2 Enterprise | its key management |
| Cancel, Join (default) | A | the password is asked by the compositor's prompt, never here | NM `AddAndActivateConnection` |

### Bluetooth

*Connect headphones, keyboards and other wireless devices.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| — | Bluetooth — *Now discoverable as “{computer name}”.* while on and shown | C · switch | | BlueZ adapter `Powered`; `Discoverable` while the pane is shown (*crate addition*) |
| My devices | {device} with its glyph — *Connected* / *Not connected*; battery after it when reported | C · ⋯ menu: Connect or Disconnect, Rename…, Forget This Device… | | BlueZ `Connect`, `Disconnect`, `Alias`, `RemoveDevice` (rename and forget are *crate additions*) |
| Nearby devices | {device}; a spinner by the title while scanning | A · "Connect" | pairing is the compositor's prompt (B28) | BlueZ discovery, only while shown |
| Missing | "No Bluetooth adapter is connected." | state | | |

### Network

*See how this computer is connected, and each connection's details.*

| Group | Row | Kind · control | Bound to |
|---|---|---|---|
| — | Ethernet — status dot and *Connected* / *Cable unplugged* | N › its page | NM device |
| — | Wi-Fi — dot and *Connected* / *Off* | N › Wi-Fi pane | NM device |
| — | VPN — dot and *Connected* / *Not connected*; only while a VPN exists | N › VPN pane | NM VPN connections |

Wired devices are listed by NM's own name ("Ethernet", "Ethernet 2");
bridges, loopback and virtual devices are not listed.

**Ethernet** (sub-page: "Ethernet", *The wired connection's addresses and speed.*)

| Row | Kind | Bound to |
|---|---|---|
| Status ("Connected", green dot) | N, read only | NM device state |
| Speed ("1 Gb/s") | N, read only | device details (*crate addition*) |
| Configure IPv4 ("Using DHCP") | N, read only; editing is P20 | the connection's `ipv4.method` |
| IP address, Subnet mask, Router, DNS servers | N, read only | device details |
| Hardware address | N, read only | the device's MAC |

### VPN

*Turn your VPN connections on and off.* Shown only while a VPN exists
(`settings.md`, The panes); reached from search otherwise with "No VPNs".

| Group | Row | Kind · control | Bound to |
|---|---|---|---|
| — | {vpn} — *Not connected* / *Connected* / *Connecting*, its kind ("WireGuard") after | C · switch; ⋯ menu: Remove… | NM activate, deactivate, delete |

Providers and import are P17 and P20.

### Energy

*Choose how the computer trades speed for power.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| — | Energy Mode — *High Power while Game Mode is on.* only during the hold | C · segmented | Low Power, Automatic, High Power | power-profiles-daemon (`isle-power` `set_profile`) |

## Group 2

### General

*About this computer and the system it runs.*

| Group | Row | Kind | Bound to |
|---|---|---|---|
| — | About | N › About | |

**About** (sub-page: "About", *This computer's name, its system and its parts.*)

| Group | Row | Kind · control | Bound to |
|---|---|---|---|
| — | Name | C · text field; asks polkit | hostnamed `SetStaticHostname` |
| System | System ("CachyOS") | N, read only | `/etc/os-release` `PRETTY_NAME` |
| System | Kernel ("7.2.8-2-cachyos") | N, read only | `uname` |
| System | Isle ("0.1.0 (125e453)") | N, read only | build version and commit |
| Hardware | Computer ("Micro-Star International MS-7E59") | N, read only | hostnamed hardware vendor and model |
| Hardware | Processor | N, read only | `/proc/cpuinfo` |
| Hardware | Graphics (one per GPU) | N, read only | DRM devices and their PCI names |
| Hardware | Memory ("64 GB", rounded) | N, read only | `/proc/meminfo` |
| Displays | {display} — size and resolution ("34-inch, 3440 × 1440") | N, read only | `Displays1.List` |

### Accessibility

*Make the screen and keyboard easier to see and use.*

| Group | Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|---|
| Display | Increase contrast | C · switch | | `appearance.increase-contrast` |
| Display | Reduce motion | C · switch | | `appearance.reduce-motion` |
| Display | Reduce transparency | C · switch | | `appearance.reduce-transparency` |
| Display | Text size | C · slider, ticks | Default to Largest, 100% to 150% by 10% (`type.scale-max`) | `appearance.text-scale` |
| Keyboard | Sticky Keys — *Modifier keys stay down until the next key.* | C · switch | | `keyboard.sticky-keys` |
| Keyboard | Slow Keys — *A key counts only once held.* | C · switch | | `keyboard.slow-keys` > 0 |
| Keyboard | Acceptance delay (only while Slow Keys is on) | C · pop-up | 0.25 s, 0.5 s, 0.75 s, 1 s, 1.5 s, 2 s | `keyboard.slow-keys` in ms; turning on writes 500 |

### Appearance

*Choose light or dark, the accent colour and the glass.*

| Group | Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|---|
| Lead | Appearance | C · pictures | Auto, Light, Dark | `appearance.mode` |
| — | Colour | C · swatches | Blue, Purple, Pink, Red, Orange, Yellow, Green, Graphite (the tokens' accents; no picker) | `appearance.accent` |
| — | Glass | C · slider, words | Clear to Tinted, 0 to 1 | `appearance.glass` |
| Windows | Show title bars on other apps' windows | C · switch | | `windows.title-bars` |

The `tokens.kdl` notice shows here and on Accessibility only.

### Apps

*Set how the desktop treats each app's windows and keys.*

| Group | Row | Kind | Bound to |
|---|---|---|---|
| — | A field, "Search apps", filtering the list | C · search field | |
| — | Every app with a desktop entry and each library game, A to Z: icon, name, a value where something is set ("Keeps running", "Game", "Not a game", "Sends keys as typed") | N › the app's page | the app index |

**An app's page** (sub-page: the app's icon at 32, its name, *What the desktop does with {app}'s windows and keys.*)

| Row | Kind · control | Choices | Bound to |
|---|---|---|---|
| Keep running after last window | C · switch; not for games | | `windows.keep-running` |
| Use Mac shortcuts — *Cmd shortcuts become {app}'s own.* | C · switch; disabled while the global switch is off | | absent from `keyboard.untranslated-apps` when on |
| Treat as a game | C · segmented | Automatic, Always, Never | `windows.games`, `windows.not-games`; Automatic is in neither |
| Notifications (value "Temporary banners", "Off"; only once the app has posted) | N › Notifications › the app | | `notification-app.<id>` |

### Control Centre

*Set what the island and Control Centre show.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| Island | Clock | D · "Clock Options…" | | the sheet below |
| Island | Wi-Fi, Bluetooth, Sound, VPN, Focus, Updates; Battery only on a machine with one | C · pop-up each | Always show, Show when active, Never show | `island-glyphs.<glyph>` |
| Control Centre | One row per module with its glyph: Wi-Fi, Now Playing, Bluetooth, Focus, VPN, Sound, Display, Night Light, Energy Mode, Updates, Batteries, Dark Mode, Keyboard layout, Accessibility, Screenshot | C · pop-up each | Don't show, then the sizes it allows: Small, Medium, Large, Wide | `control-centre.modules` (a newly shown module goes last) |
| Control Centre (foot) | "Edit controls…" | A | | `Shell1.EditControls()` |
| Allow in the tray | Each program with a tray item now or hidden before | C · switch each | | `control-centre.tray-hidden` *new* |
| (foot) | "Reset Control Centre…" | A · asks first | | `Settings1.Reset` of `island`, `island-glyphs`, `control-centre` |

**Clock Options…** (sheet, 440)

| Row | Kind · control | Choices | Bound to |
|---|---|---|---|
| Time format | C · segmented | Automatic, 24-hour, 12-hour | `island.clock-24-hour` (auto, on, off) |
| Show the day of the week | C · switch | | `island.clock-weekday` |
| Show date | C · switch | | `island.clock-date` |
| Display the time with seconds | C · switch | | `island.clock-seconds` |
| Done | A · default | | |

### Desktop

*Set how windows tile and fill, and what the screen corners do.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| Windows | Double-click a window's title bar to | C · pop-up | Fill, Minimise, No Action | `windows.titlebar-action` (fill, minimise, none) |
| Windows | Drag windows to screen edges to tile | C · switch | | `windows.edge-tiling` *new*, read by the tiling zones |
| Windows | Tiled windows have margins | C · switch | | `windows.tile-margins` |
| (foot) | "Hot Corners…" | D · button under the group | | the sheet below |

**Hot Corners…** (sheet, 664): the main display's wallpaper at 240 wide in
its shape, a pop-up at each corner outside it, "Done".

| Row | Kind · control | Choices | Bound to |
|---|---|---|---|
| Top left, top right, bottom left, bottom right | C · pop-up each; a modifier held while the menu is open is required to fire, shown before the action ("⌘ Apps") | Notification Centre, Apps, Lock Screen, Put Display to Sleep, — (Mission Control, Application Windows, Desktop, Widgets join with P16, P17) | `hot-corners.top-left` … `hot-corners.bottom-right` *new*, each `none`, `notification-centre`, `apps`, `lock-screen` or `display-sleep`; `hot-corners.<corner>-modifiers` *new* ("Cmd", "Option+Shift") |

Defaults: bottom right Apps, the rest none; bottom left becomes Mission
Control with P16 (`dock.md` For the user 3).

### Displays

*Arrange and set up the screens you use.*

| Group | Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|---|
| Lead | The arrangement, to scale, the island on the main display | C · map | | `output.<id>.x`, `.y`; applies at once |
| — | Main display | C · pop-up | each enabled display by name, then Follow focus | `shell.main-screen` (unset follows focus) |
| {display} (the selected one) | Use this display | C · switch; disabled on the only one on | | `output.<id>.enabled` · keep or revert |
| {display} | Resolution | C · pop-up | the output's sizes, the preferred one marked "(default)" | `output.<id>.mode` (`WxH`) · keep or revert |
| {display} | Refresh rate | C · pop-up | the rates at that size ("144 Hz", "120 Hz", "60 Hz") | `output.<id>.mode` (`WxH@HZ`) · keep or revert |
| {display} | Scale | C · pop-up | 100% to 200% by 25%, the display's default marked, then Other… (a field, 50% to 400%) | `output.<id>.scale`; applies at once |
| {display} | Rotation | C · pop-up | Standard, 90°, 180°, 270° | `output.<id>.transform` · keep or revert |
| {display} | Brightness — *Brightness can't be set on this display.* as the refused line without DDC | C · slider, glyphs | 0% to 100% | `Displays1.SetBrightness` |
| {display} | Variable refresh rate — *This display supports 48–144 Hz.* | C · segmented | Off, Full screen, Always | `output.<id>.vrr` |
| {display} | Allow tearing in games — *A game that asks is shown without waiting for the display.* | C · switch | | `output.<id>.tearing` |
| Off | {display}, "Off" (each disabled display) | C · switch | | `output.<id>.enabled` · keep or revert |
| (foot) | "Detect Displays", "Advanced…", "Night Light…" | A, D, D | | `isle displays redetect`; the sheets below |

**Keep these display settings?** (alert, 320): "Reverting in {n}
seconds." counting `timing-display-revert-ms`; Revert, Keep (default).
`Displays1.Revert` / `Displays1.Keep`; the compositor's deadline reverts
without either.

**Night Light…** (sheet, 520)

| Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|
| Schedule | C · pop-up | Off, Sunset to Sunrise, Custom | `night-light.schedule` |
| From, To (only for Custom) | C · time fields | `HH:MM` | `night-light.from`, `night-light.to` |
| Turn on until sunrise ("until {time}" for Custom; "Turn on" for Off) | C · switch | | the compositor's manual toggle, as Control Centre's |
| Warmth | C · slider, words | Less warm to More warm, 6000 K to 2700 K | `night-light.warmth` |
| Done | A · default | | |

**Advanced…** (sheet, 480)

| Row | Kind · control | Choices | Bound to |
|---|---|---|---|
| Graphics card — *Takes effect the next time you log in.* | C · pop-up | Automatic (the boot GPU), then each render node by name | `render-device` |
| Done | A · default | | |

### Spotlight

*Choose what Spotlight searches and what it keeps.*

| Group | Row | Kind · control | Bound to |
|---|---|---|---|
| Results | Apps, Actions, Calculator, Files, Menu items, Settings, Windows, Web search | C · switch each | `spotlight.sources-off` *new* (`apps`, `actions`, `calculator`, `files`, `menu-items`, `settings`, `windows`, `web`); a scope (⌘1 to ⌘4) still searches its own source |
| Favourites | The Apps view's favourites, in order | L · + −, drag to reorder | `spotlight.favourites` |
| Clipboard | Don't keep copies from | L · + − | `spotlight.clipboard-exclude` |
| Search | Web search | C · pop-up: DuckDuckGo, Google, Bing, Kagi, Custom… (a field taking an address with `{query}`) | `spotlight.web-search` |
| Search (foot) | "Search Privacy…" | D · button under the group | the sheet below |

**Search Privacy…** (sheet, 520)

| Group | Row | Kind | Bound to |
|---|---|---|---|
| Also search these folders | folders besides home | L · + − (the Open panel) | `spotlight.files-include` |
| Don't search these folders | folders left out | L · + − | `spotlight.files-exclude` |
| — | Done | A · default | |

### Wallpaper

*Choose the picture behind your windows.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| Lead | The chosen wallpaper on the main display, in its shape | preview | | |
| — | Show on all screens | C · switch | | `wallpaper` when on; `wallpaper-output.<id>` per screen when off |
| — | Screen (only while off) | C · pop-up | each display by name | which section the rows below set |
| — | Wallpaper | C · pictures | Frost (the drawn backdrop), Colour, Picture | `wallpaper.source` (`frost`, `#rrggbb`, a path) |
| — | Picture or folder (Picture only) — "Missing" under it when gone | C · file, "Choose…" | | `wallpaper.source` |
| — | Fit (Picture only) | C · pop-up | Fill screen, Fit to screen, Centre | `wallpaper.fit` |
| — | Colour (Colour, and around Fit and Centre) | C · swatches, the last a picker | | `wallpaper.colour` (unset: the picture's mean) |
| Dark Mode | Use a different picture in Dark Mode | C · switch | | `wallpaper.dark-source` set or unset |
| Dark Mode | Dark Mode picture (only while on) | C · file, "Choose…" | | `wallpaper.dark-source` |
| Folder | Change picture (only for a folder) | C · pop-up | Every hour, Every day, When logging in, Never | `wallpaper.change` |
| Folder | Random order (only for a folder) | C · switch | | `wallpaper.shuffle` |

## Group 3

### Notifications

*Choose which apps can notify you, and how.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| — | On the lock screen | C · pop-up | Don't show, Show count, Show previews | `notifications.lock-screen` |
| — | Hold banners while a window is full screen — *Banners still show on a screen without one.* | C · switch | | `notifications.hold-full-screen` |
| — | Hold banners while sharing your screen | C · switch | | `notifications.hold-sharing` |
| Application notifications | Every app that has posted, A to Z: icon, name, its summary under it ("Temporary banners, sound, badges", "Off") | N › the app's page | | `Notifications1.Apps()` |

**An app's Notifications page** (sub-page: icon, name, *How {app}'s notifications reach you.*)

| Row | Kind · control | Choices | Bound to |
|---|---|---|---|
| Allow notifications | C · switch; the rest disabled while off | | `notification-app.<id>.allow` |
| Banner style | C · segmented | Temporary, Persistent | `notification-app.<id>.banner-style` |
| Play sound | C · switch | | `notification-app.<id>.sound` |
| Show badges | C · switch | | `notification-app.<id>.badges` |
| Show over full-screen windows | C · switch | | `notification-app.<id>.over-full-screen` |

### Sound

*Choose alert sounds, speakers and microphones.*

| Group | Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|---|
| Sound effects | Alert sound | C · pop-up; choosing plays it | the freedesktop theme's message sounds by name | `notifications.alert-sound` |
| Sound effects | Play sound for notifications | C · switch | | `notifications.sounds` |
| Sound effects | Play sound when taking a screenshot | C · switch | | `capture.sound` |
| Output | The output devices, the default selected; kind after the name | L, no foot; choosing one makes it the default | | `isle-audio` `set_default_output` |
| Output | Output volume | C · slider, speaker glyphs | 0% to 100% | `set_volume` |
| Output | Mute | C · switch | | `set_mute` |
| Output | Balance | C · slider, words, centre tick | Left to Right | output balance (*crate addition*) |
| Input | The input devices, the default selected | L, no foot | | `set_default_input` |
| Input | Input volume | C · slider, microphone glyphs | 0% to 100% | `set_volume` |
| Input | Input level | V · meter, sampled while shown | | `isle-audio` `meter` |

### Focus

*Choose what interrupts you, and when.*

| Group | Row | Kind | Bound to |
|---|---|---|---|
| — | Do Not Disturb, Game Mode, then the user's modes: symbol in a circle (accent while on), name, value ("On", "When a game is in front", "Weekdays 09:00–17:00", "Off") | N › the mode's page | `focus-mode.<name>`; on and off is the Focus service's |
| (foot) | "Add Focus…" | D · button under the group | the sheet below |

**A mode's page** (sub-page: the symbol at 32, the name, *What gets through while {focus} is on.*)

| Group | Row | Kind · control | Bound to |
|---|---|---|---|
| Allowed notifications | Apps whose banners and sounds still come through ("No apps") | L · + − | `focus-mode.<name>.allow-apps` |
| — | Allow time-sensitive notifications | C · switch | `focus-mode.<name>.allow-critical` |
| Turn on automatically | Each schedule: "Weekdays 09:00–17:00" (Time), "While {app} is in front" (App) | L · + opens "Time" and "App"; double-click edits (From, To, Every day, Weekdays, Weekends) | `focus-mode.<name>.schedules`, `.while-apps` |
| (foot) | "Delete Focus" (`colour-danger-text`, asks first); the user's own modes only | A | `Settings1.Remove` |

Game Mode's page replaces Turn on automatically with:

| Group | Row | Kind · control | Bound to |
|---|---|---|---|
| Turn on automatically | When a game is in front | C · switch | `focus.game-mode-auto` |
| Turn on automatically | Off for these games | L · + − ("Not installed" after a missing one) | `focus.games-off` |

People is absent until a source names senders (`focus.md`).

**Add Focus…** (sheet, 400)

| Row | Kind · control | Bound to |
|---|---|---|
| Name | C · text field | the new `focus-mode.<name>` |
| Symbol | C · a grid of 12 glyphs | `focus-mode.<name>.symbol` |
| Cancel, Add (default; disabled until named) | A | opens the new mode's page |

## Group 4

### Lock Screen

*Set when the screen dims, turns off and locks.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| — | Dim the display after | C · pop-up | 1 min, 2 min, 5 min, 10 min, 15 min, 30 min, 1 hr, 2 hr, 4 hr, — Never | `lock-screen.dim-after` (minutes, 0 never) |
| — | Lock after | C · pop-up | as above | `lock-screen.lock-after` |
| — | Turn display off after | C · pop-up | as above | `lock-screen.display-off-after` |
| — | Lock when the computer sleeps | C · switch | | `lock-screen.lock-on-sleep` |

A hand-edited value off the steps shows as its own item in the menu.

### Privacy & Security

*See what apps may do without asking, and take it back.*

| Group | Row | Kind | Bound to |
|---|---|---|---|
| Privacy | Screen & System Audio Recording (value: "{n} apps") | N › its page | permission store `screencast` |
| Privacy | Screenshots (value) | N › its page | permission store `screenshot` |

**Screen & System Audio Recording** (sub-page: *Apps that may share your screen again without asking.*)
and **Screenshots** (*Apps that may take screenshots without asking.*)

| Row | Kind · control | Bound to |
|---|---|---|
| Each app remembered: icon, name | C · switch; off revokes, and the row stays off until the app asks again | the table's entry, deleted as `isle portal permissions --revoke` does |
| Empty | "No apps" | |

## Group 6

### Keyboard

*Set key repeat, layouts, shortcuts and each keyboard's kind.*

| Group | Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|---|
| — | Key repeat rate | C · slider, words, ticks | Slow to Fast: 2, 5, 10, 15, 20, 25, 30, 40, 60 keys a second | `keyboard.repeat-rate` |
| — | Delay until repeat | C · slider, words, ticks | Long to Short: 2000, 1500, 1000, 800, 600, 400, 250, 150 ms | `keyboard.repeat-delay` |
| Keyboards | Each physical keyboard: name, value "Mac" or "PC" (with "(automatic)" when guessed) | N › its page | `keyboard-device.<name>` |
| Keyboards (foot) | "Detect…" | A · "Press any key…" while waiting; opens the keyboard pressed | `Keyboard1.Record` |
| Text input | Input sources ("British – Mac") | D · "Edit…" | `keyboard.layouts`, `keyboard.variants` |
| — | Turn on Num Lock at login | C · switch | | `keyboard.numlock` |
| — | Use Mac shortcuts in other apps — *Cmd shortcuts become each app's own, as Cmd+C does Ctrl+C.* | C · switch | | `keyboard.translate` |
| (foot) | "Modifier Keys…", "Keyboard Shortcuts…" | D · buttons under the group | | the sheets below |

**A keyboard's page** (sub-page: keyboard glyph at 32, its name, *How this keyboard's modifier keys work.*)

| Row | Kind · control | Choices | Bound to |
|---|---|---|---|
| Kind | C · segmented | Automatic, Mac, PC | `keyboard-device.<name>.profile` |
| Swap Alt and Super — *Cmd sits beside the space bar.*; disabled on a Mac keyboard | C · switch | | `keyboard-device.<name>.swap-alt-super` |
| Keyboard type — *ISO has a key left of Z.* | C · pop-up | Automatic, ISO, ANSI | `keyboard-device.<name>.model` |

**Input Sources…** (sheet, 480)

| Row | Kind · control | Bound to |
|---|---|---|
| The sources in order, the first the default ("British – Mac") | L · + (a step listing `evdev.xml`'s layouts and variants, with Back), −, drag to reorder | `keyboard.layouts`, `keyboard.variants` |
| Switch with {chord} (read only, `shortcuts.next-layout`), only with two or more | N | |
| Done | A · default | |

**Modifier Keys…** (sheet, 440)

| Row | Kind · control | Choices | Bound to |
|---|---|---|---|
| Caps Lock key | C · pop-up | Caps Lock, Control, Escape, No Action | `keyboard.options` (`caps:ctrl_modifier`, `caps:escape`, `caps:none`); applies to every keyboard |
| Done | A · default | | |

**Keyboard Shortcuts…** (sheet, 720 × 480): categories down the side,
each action a row of checkbox, name and recorder; "Restore Defaults"
leading, "Done" trailing. Only actions the compositor carries out today
(`binds::built`) and P13's Open System Settings; the rest join with their
packets. A checkbox off writes an empty list.

| Category | Actions (`shortcuts.<id>`) |
|---|---|
| Windows | Close window, Minimise, Minimise all of the app's windows, Hide, Hide others, Quit, Enter or leave full screen, Next window of the app, Previous window of the app, Window menu, Zoom menu |
| Tiling | Left half, Right half, Centre, Fill, Restore, Top left, Top right, Bottom left, Bottom right, Fill beside, Move to next display, Move to previous display |
| Desktops | Desktop to the left, Desktop to the right, Carry window left, Carry window right; "Switch to a desktop with" (pop-up of modifiers, `shortcuts.desktop-jump`); "Move and resize windows with" (`shortcuts.window-drag`) |
| Spotlight and apps | Spotlight, Apps, Emoji, App switcher, App switcher, from the end, Open a terminal, Open System Settings, Force Quit; "Terminal" (text field, `shortcuts.terminal-command`) |
| Menus and panels | Control Centre, Notification Centre, Menu bar, Search menus |
| Screenshots | Screenshot of the screen, of a portion, the capture toolbar, each also "to the clipboard"; Pick a colour; Stop recording |
| Focus | Do Not Disturb, Game Mode |
| Session | Lock Screen, Log Out…, Log Out, Restart, sleep or shut down |
| Keyboard and displays | Next input source, Take keys back, Detect Displays |
| Media | Volume up, Volume down, Mute, Microphone mute, Brightness up, Brightness down, Play or pause, Next track, Previous track |

### Mouse

*Set pointer speed, scrolling and the primary button.* One group per
physical pointing device; with one, it reads as one set.

| Group | Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|---|
| {device} | Tracking speed | C · slider, words | Slow to Fast, -1 to 1 | `pointer.<name>.speed` |
| {device} | Pointer acceleration | C · switch | | `pointer.<name>.accel-profile` (adaptive on, flat off) |
| {device} | Natural scrolling — *Content moves with your fingers.* | C · switch | | `pointer.<name>.natural-scroll` |
| {device} | Primary mouse button | C · segmented | Left, Right | `pointer.<name>.left-handed` |
| Missing | "No mouse is connected." | state | | |

`tap` and `scroll-method` are touchpad settings and come with Trackpad
(P16).

## Outside every pane

Read today and given no row: `keyboard.untranslated-apps`' defaults beyond
what an app's page shows (remote desktops and VMs, listed on their own
pages), `windows.not-games` beyond the same, `control-centre.tray`
(order, set by dragging in the panel), `capture.*` but `sound` (the
capture toolbar's Options), the `debug` section, and `output.<id>`'s
flipped transforms. They stay hand edits, shown by Show changed settings.

## For the user

1. **Scale.** (a) A pop-up of steps, 100% to 200% by 25% with the
   display's default marked and Other… for a typed value; (b) macOS's
   thumbnails from Larger Text to More Space. **Recommended: (a).** The
   thumbnails assume a Retina panel whose default is 200%; at 100% on the
   ultrawide every thumbnail but one is "larger text", and the pop-up says
   the number a user who knows fractional scaling wants.
2. **Glass.** (a) A slider from Clear to Tinted, as macOS 27 and the
   schema's 0 to 1; (b) two choices, Clear and Tinted, as macOS 26.
   **Recommended: (a).** The floor already keeps text readable at any
   point (B58), so the middle values are safe and worth having.
3. **Spotlight's sources.** (a) A switch per source in an unscoped
   search (Apps, Actions, Calculator, Files, Menu items, Settings,
   Windows, Web search), with the new `spotlight.sources-off`; (b) no
   switches. **Recommended: (a).** macOS has them, it is the only way to
   keep, say, menu items out of the top hit, and a scope still reaches a
   source that is off.
4. **Energy.** (a) The pane with Energy Mode alone and the Game Mode
   line; (b) no Energy pane until sleep and wake settings exist, the mode
   left to Control Centre. **Recommended: (a).** Control Centre and
   search already point at it, and a desktop has no battery rows to add.
5. **Network's rows.** (a) Every connection with its status: Ethernet
   opening its page, Wi-Fi and VPN opening their panes, as macOS; (b)
   wired connections only. **Recommended: (a).** It is the one place the
   whole connection state reads at a glance, and the Wi-Fi and VPN rows
   hold no settings of their own, so nothing is set in two places.
6. **Mouse.** (a) Per device, a group per connected mouse, as the schema
   keys `pointer`; (b) one set written to every mouse, as macOS.
   **Recommended: (a).** It costs nothing with one mouse and keeps a
   gaming mouse's flat profile apart from anything else plugged in.
7. **Virtual input devices.** xremap's keyboard and the Steam
   Controller's puck show up as a keyboard and a mouse. (a) Leave out
   every device on the virtual bus and every gamepad's emulated keys; (b)
   list everything. **Recommended: (a).** None of them can be set
   usefully, and a Detect that finds xremap would be wrong.
8. **Control Centre's modules in Settings.** (a) A pop-up per module,
   Don't show or a size, with order left to Edit controls; (b) a read-only
   list and Edit controls…. **Recommended: (a).** It is the keyboard route
   `control-centre.md` asks for, and one pop-up per module says shown and
   size together.
9. **Privacy & Security.** (a) A sub-page per kind, as macOS; (b) both
   lists on the pane itself. **Recommended: (a).** Providers (P17) add a
   kind per page, and the pane then reads the same as on a Mac.
10. **Scale's confirmation.** Choice 19 applies scale at once with no
    keep-or-revert, where `settings.md` listed it with the mode.
    **Recommended: keep choice 19.** A wrong scale leaves the screen
    readable and the row in reach; only a mode, rotation or an enabled
    change can leave nothing on screen.

**Answered** 2026-10-17: the user took every recommendation.
