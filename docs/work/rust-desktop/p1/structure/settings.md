# Settings

Batch 2. Answers to: `apps.md` For the design process (Settings: the
window's layout; the pane list, names, order and grouping; whether Focus
is a pane; whether Game Mode folds into Gaming Focus; how a found row is
marked; how the Displays arrangement is drawn) and the presentation side of
`apps.md` Settings 1–17 and the Parity map's Settings rows; `platform.md`
For the design process (Settings: how the Settings app shows a rejected
hand edit, beside `island.md`'s glyph and notice row); `shell.md` Control
Centre 3 (a control's full page) and Spotlight 2 (Settings pages);
`gaming.md` Game Mode 2; B10, B17, B22, B27, B30.

## Purpose

macOS's System Settings for Isle: one window holding every setting the
desktop and the system have, in panes, searchable, with back and forward
(`apps.md` Settings). It is where a Mac user goes to change anything, and
the only full view of what Control Centre offers in quick form. Every v1
page has a home here (`apps.md` Settings 1). It is not where Isle's apps
keep their own preferences (each app's settings window, `conventions.md`),
not a place for live status (the island and Control Centre), and not a
second editor for `tokens.kdl`, which is the look's file and is never
written by Isle (`platform.md` Design tokens 2). This sheet is its
structure and the pane list; what each pane holds row by row is that
pane's packet.

## Content

**Name.** The app is "System Settings" in its desktop entry, the switcher,
Spotlight and the on-demand menu bar, matching the system menu's "System
Settings…" (`menu-bar.md`) and setting it apart from an app's own
"{app} Settings" window. In prose and paths it is "Settings"
("Settings › Displays"), as the batch 1 sheets write it.

### The window

```
┌───────────────────────┬──────────────────────────────────────────────┐
│ ● ● ●                 │  ‹  ›   Displays                             │  toolbar
│ ┌ Search ───────────┐ │                                              │
│ └───────────────────┘ │  [notice: settings.kdl has an error …]       │  only while it holds
│ ◯ Full Name           │                                              │
│   login             › │   Displays                                   │  pane header
│                       │   Arrange and set up the screens you use.    │
│ Wi-Fi                 │                                              │
│ Bluetooth             │   ┌──────────────────────────────────────┐   │
│ Network               │   │   arrangement, drawn to scale         │   │
│ …                     │   └──────────────────────────────────────┘   │
│                       │   ┌ group ───────────────────────────────┐   │
│ General               │   │ Label                      [control] │   │
│ Accessibility         │   │ Label                              › │   │
│ …                     │   └──────────────────────────────────────┘   │
└───────────────────────┴──────────────────────────────────────────────┘
```

- **Sidebar**, to the window's top, bottom and leading edge, the lights
  over its top (`conventions.md`, Window anatomy). In order: the **search
  field**; the **account row**; the **pane list**. It is not collapsible
  (macOS), scrolls on its own, and grows with the text size. Its width is
  fixed, as macOS's.
- **The account row**: the user's picture, full name and login name from
  AccountsService. It opens Users & Groups at the user's own account,
  where macOS's opens the Apple Account. Isle has no cloud account to put
  there.
- **The pane list**: each pane a row of its symbol and name, in groups
  separated by space, with no group headings (macOS). The selected pane is
  marked; Up and Down move the selection and open the pane at once.
- **Toolbar**: back and forward, then the pane's title, or the sub-page's
  within a pane (General › About). Nothing in the centre or trailing
  group: search is in the sidebar (`conventions.md`, Search). No sidebar
  toggle.
- **Content**: the pane. At its top, only while it holds, the
  configuration notice (Rejected hand edit, below). Then the **pane
  header**: the pane's name and a one-sentence subtitle (`copy.md` rule
  5); a pane whose subject is visual leads with it (the Displays
  arrangement, the Wallpaper preview, the Appearance choices). Then the
  pane's **groups of rows**. The column has a maximum width and centres in
  a wide window; the window resizes from every edge (`apps.md`
  Conventions 16), which only adds margin.

### Rows

Every pane is a list of grouped rows, macOS's form style. A row is a label
at the leading edge, an optional one-line description under it only where
the label leaves something to say (`copy.md` rule 5), and one control at
the trailing edge. Kinds of row:

| Row | Does |
|---|---|
| Control | Its control sets the value as it is changed |
| Navigation | A chevron; opens a sub-page within the pane (General's sections, a Focus mode, a device), with Back to return |
| Details | A "Details…" or "Options…" button; opens a sheet for one item's settings (a Wi-Fi network, a printer, Hot Corners…) |
| List | A table the user builds (login items, allowed apps, calendars), + and − at its foot, reordered by drag only where order matters |
| Provider | A provider's state sentence and its actions, drawn without knowing the tool behind it (`apps.md` Settings 11; D42) |
| Action | A button named for what it does ("Detect Displays", "Check now"); one that cannot be undone asks first |
| Live | A meter, the pad drawing, SMART figures: sampled only while the pane is shown (`apps.md` Settings 15) |

Changes apply as they are made; there is no Apply button (`apps.md`
Settings 6). A text field commits on Return or on leaving it. A slider
applies while it moves and is written once on release. A row that needs a
privileged change asks once, in the desktop's polkit dialog, when changed
(Conventions 25), as macOS has done since it dropped the padlock. A value
the compositor or a daemon refuses stays on its row with the reason under
it (`apps.md` Settings 6).

### Control types

By the kind of value, as v1 settled them (`972bbe7`; `design-v1-lessons.md`
§5 Settings):

| Value | Control | Example |
|---|---|---|
| On or off | A switch at the row's end | Reduce motion |
| One of two or three, all worth seeing | A segmented control; a row of pictures where the choice is visual | Appearance: Light, Dark, Auto |
| One of more, or long names | A pop-up button | Title-bar double-click action |
| A duration | A pop-up button of fixed steps, "Never" last | Turn display off after |
| A number with a unit, read exactly | A number field with a stepper, the unit after it | Tile margins |
| A continuous value, felt rather than read | A slider with a symbol or word at each end; tick marks where only steps are allowed | Volume, glass, pointer speed, text size |
| A colour | A row of swatches, the last opening a picker | Accent |
| A file or folder | Its name with "Choose…", which opens the Open panel (`files.md`) | Wallpaper folder |
| A chord | A shortcut recorder (Recording, below) | Keyboard Shortcuts |
| A list | A table with + and − | Login items |
| Text | A text field | Computer name |

A disabled control stays visible with its value; its reason is its
accessible description, never a line of instructions.

### The panes

Each pane takes the name and place of the macOS 26 pane that covers it; an
Isle-only pane sits beside its nearest macOS neighbour (`apps.md` proposal;
B17). macOS's second group is alphabetical after General, and so is
Isle's. What a pane holds is listed to place every v1 page and every
desktop setting; rows and their order are each pane's packet, and P13's
are under Each pane's rows.

| Group | Pane | macOS counterpart | Holds | From v1 |
|---|---|---|---|---|
| 1 | Wi-Fi | Wi-Fi | Wi-Fi on or off, networks, known networks, a network's Details… | Network |
| 1 | Bluetooth | Bluetooth | On or off, devices; pairing through the desktop's agent (`apps.md` Settings 16) | Bluetooth |
| 1 | Network | Network | Wired and other interfaces, their details | Network |
| 1 | VPN | VPN | NetworkManager VPNs and VPN providers (`apps.md` Settings 11) | Network › VPN providers |
| 1 | Phone | *Isle only*, after VPN | Phone providers; shown only while one is installed | Phone |
| 1 | Energy, or Battery on a machine with one | Energy, Battery | Power profile, sleep and wake, batteries | Power |
| 2 | General | General | Sub-pages: About, Software Update, Storage (each disk as a bar by kind, and Recommendations that open Cleaner, `cleaner.md` For the user 1) · Passwords, Date & Time, Language & Region, Login Items · Snapshots | About, Updates, Storage, Devices, Passwords, Date and time, Region, Apps (autostart, user services, start commands), Snapshots |
| 2 | Accessibility | Accessibility | Display (contrast, transparency, motion, text size), Zoom, the on-screen keyboard, pointer, Sticky Keys and Slow Keys (latched in the compositor, drawn with the on-screen keyboard's held and locked forms; with P5's chord table) | Accessibility |
| 2 | Appearance | Appearance | Light, Dark or Auto; the glass slider; accent; fonts (D52); other apps' title bars (`window-chrome.md`) | Appearance |
| 2 | Apps | *Isle only*, beside Desktop & Dock, which holds macOS's default browser | Default apps; per app: keep running after its last window (B20), Cmd translation (B21) | Apps (keep-running, default apps) |
| 2 | Control Centre | Control Centre (macOS 15; Menu Bar in 26) | The island: clock options, each glyph's tier (`island.md`); Control Centre's modules, shown or not, and Edit controls; which programs may show in the tray | |
| 2 | Desktop | Desktop & Dock, without the Dock | Windows: title-bar double-click, prefer tabs, close windows when quitting, session restore; tiling: edges, the pointer panel, margins; Mission Control; Hot Corners…; the widgets dashboard; click wallpaper to reveal desktop; the dropdown terminal | Apps (session restore) |
| 2 | Displays | Displays | The arrangement (below); per display: mode, scale, rotation, VRR, tearing, enabled, brightness; the main display; night light; the render GPU; Detect Displays | Displays, Modes (tearing) |
| 2 | Spotlight | Spotlight | Sources shown, clipboard history, web search | |
| 2 | Wallpaper | Wallpaper | As `wallpaper.md` | Wallpaper |
| 3 | Notifications | Notifications | Per app; previews; showing banners while a window is full screen or the screen is shared (`platform.md` Focus 3) | Notifications |
| 3 | Sound | Sound | Output and input with meters; alert sound; each system sound on or off (brief, interview) | Audio |
| 3 | Focus | Focus | Do Not Disturb, Game Mode and the user's modes, each a sub-page; Game Mode's settings (For the user 2) | Notifications (do not disturb, quiet hours), Modes (game auto) |
| 4 | Lock Screen | Lock Screen | Idle dim, display off, lock after, lock on sleep, the greeter | Power (idle timers) |
| 4 | Privacy & Security | Privacy & Security | The permissions Isle has granted: remembered screen-sharing choices; Providers, every provider of every kind with its grants as switches, one page each, read only with "Change…" everywhere else (B28, `widgets.md`) | |
| 4 | Login Password, or Fingerprint & Password with a reader | Login Password, Touch ID & Password | The password; fingerprints, and where a fingerprint is accepted (D33, D34) | Users (fingerprint, PAM switches) |
| 4 | Users & Groups | Users & Groups | Accounts; automatic login | Users |
| 5 | Calendars | *Isle only*, in Internet Accounts' place | Calendar subscriptions (D41) | Calendars |
| 6 | Keyboard | Keyboard | Per-device profiles with Detect (`apps.md` Settings 9, 10); the Alt and Super swap (B30); layouts; repeat; Keyboard Shortcuts… | Keyboard, Shortcuts |
| 6 | Mouse | Mouse | Speed, scrolling, buttons | Mouse |
| 6 | Trackpad | Trackpad | Gestures (`compositor.md` Pointer, touchpad and gestures 6); shown only while one is connected | |
| 6 | Game Controllers | Game Controllers | Each pad's live drawing and battery (`gaming.md` Controllers 6; B22) | Controllers |
| 6 | Printers | Printers & Scanners | Printers, the default printer | Printers |

Choices made in the table:

- **No Dock pane** (B19). macOS's Desktop & Dock becomes **Desktop**,
  holding everything there that is not the Dock, with Hot Corners… as a
  sheet that draws the screen with a pop-up at each corner (`shell.md`
  Mission Control 8; defaults `dock.md` For the user 3).
- **Control Centre** keeps the name macOS used until 26, since Isle has no
  menu bar to name a pane after, and the island's glyphs are what macOS
  26's Menu Bar pane sets. It is where `island.md` puts the glyph tiers.
- **Focus** is a pane of its own, as on macOS; Do Not Disturb and
  Game Mode (the Gaming Focus, B30) are modes within it.
- **VPN** is its own pane after Network, as on macOS, not a section of
  Network. **Phone** is Isle's own and shows only while a phone provider
  is installed, as macOS shows VPN only once one is set up: a pane that
  could only say "nothing here" is noise (brief P1). Trackpad likewise.
- **Passwords and Snapshots** are General's sub-pages, where macOS 26 has
  AutoFill & Passwords and Time Machine; both are provider lists
  (`apps.md` Settings 11).
- **Calendars** is named for what it holds: Isle's calendars are
  subscriptions (D41), so "Internet Accounts" would promise sign-ins Isle
  does not have. It sits where Internet Accounts does.
- **Apps** carries what the desktop applies to each app from outside
  (`conventions.md`, An app's own settings); `dock.md` already sends keep
  running here.
- **Game Controllers** and **Sound** take macOS's names for v1's
  Controllers and Audio; `gaming.md`'s "Settings › Controllers" follows
  at the gate.
- **Devices** (v1's Device Manager tree) becomes General › About's system
  report, as macOS's System Information.
- **Session restore** is in Desktop beside macOS's "Close windows when
  quitting an application" (`conventions.md` For the user 1); Login Items
  holds what starts at sign-in (`platform.md` Login Items 1).
- Nothing of macOS's account services (iCloud, Wallet, Game Center,
  Screen Time, Siri) is drawn as a placeholder.

### Search

The field at the top of the sidebar, on ⌘F (`conventions.md`).

- **Results replace the pane list** while a query stands: each result is a
  row's label with its path under it ("Displays › Night Light"), grouped
  by pane in the sidebar's order, with panes whose name matches first. It
  matches every row's label, description and keywords on every pane and
  sheet (`apps.md` Settings 3), and keywords carry other desktops' and
  Linux words, so "dark mode", "resolution", "tearing" or "XKB" find their
  rows without those words being shown (`copy.md` rule 2).
- **Moving onto a result shows its pane** in the content, as macOS does;
  Return or a click goes to the row. Escape clears the field and the pane
  list returns, keeping the pane shown.
- **The found row** is scrolled to a third of the way down the content,
  not to the edge, and marked by a ring in the accent around the row
  (brief P6: the accent marks the one thing pointed at). The ring holds
  until the user clicks, types, scrolls or changes pane, then fades; it
  never pulses. A row inside a sheet or sub-page opens it first. A row that
  is absent now (no trackpad connected) still lists, and opens its pane's
  empty state.
- **No results**: "No results for “{query}”" in the sidebar.
- **One index.** The pane and row index is data installed with Settings
  (`apps.md` Settings 4); Spotlight's settings results read the same
  index, and opening one marks the row the same way.

### Changed settings

KDE's "Highlight changed settings" (`design-references.md` §13), as
View › Show changed settings, off by default (For the user 1):

- a mark before every row whose value differs from its default, and on
  every pane in the sidebar that holds one;
- "Reset to default" in a marked row's context menu, which also works with
  the view off;
- desktop settings only, the keys of `settings.kdl`; system state a daemon
  holds (networks, accounts, printers) has no default and is not marked.

With `settings.kdl` hand-edited, it is the file's difference from the
defaults, read in the panes; after the one-time import from v1 (B27) it
shows what came over.

### The Displays arrangement

- **To scale.** Every enabled display is drawn as a rectangle of its
  logical size (resolution over scale) in its arranged place, named, with
  its resolution under the name. The shape is the display's, so the
  ultrawide reads as 21:9 beside a 16:9 screen. A disabled display is a
  row under the map, not drawn in it; a display that is not connected
  (behind the KVM) is not shown, and its settings are kept by its identity
  (`compositor.md` Outputs 1).
- **The main display** carries a small island at its top centre, as
  macOS marks it with the menu bar. Dragging the island to another display
  makes that one main; a pop-up offers the same, with "Follow focus"
  (`shell.md` Shell runtime 14).
- **Moving.** A display is dragged freely; on drop it snaps flush to the
  nearest edge of another display, aligns to that display's edge or centre
  when close, and is pushed out of any overlap, as v1 did (inventory §3.7).
  An arrangement applies at once; it needs no confirmation.
- **Selecting** a display shows its settings in the groups below the map.
  With one display the map shows it alone, selected, and does not drag.
- **Identifying.** While the pointer is over the map or a display is being
  dragged, each real screen shows its name, large, at its centre, so the
  map can be matched to the desk.
- **Keyboard.** The map is one Tab stop: arrows select a display; ⌥ with
  an arrow moves the selected display to the next snapped place in that
  direction; ⌘ with an arrow does the same for the island mark.
- **Mode, scale, rotation and enabled** apply at once, then a sheet asks
  "Keep these display settings?" with a countdown; with no answer they
  revert (`compositor.md` Outputs 2). The default button keeps them, so
  Return confirms on a screen that can be read.
- **The last enabled display** cannot be turned off: its switch is
  disabled (`apps.md` Settings 13).

### A rejected hand edit

`settings.kdl` holds an edit that does not parse, fails validation or
names an unknown key; the last good settings are in force (`platform.md`
Settings 6, 7; B27). The island shows its configuration glyph and the
notice row in Control Centre (`island.md`). Settings shows:

- **A notice at the top of every pane's content**, above the header, the
  same in every pane (`apps.md` Settings 7): `copy.md`'s rejected-edit
  string with line and column, the parser's message on a second line,
  and "Open in editor" and "Show in Files". It cannot be dismissed; it
  goes when a good edit replaces the bad one. The sidebar marks nothing:
  every pane carries the notice.
- **Rows bound to `settings.kdl` are disabled**, showing the values in
  force, so no control can be changed only to be refused. Rows a daemon
  owns (Wi-Fi, Bluetooth, accounts, the date, printers, updates) keep
  working, since they do not write the file. One schema drives the
  controls (`platform.md` Settings 8), so each row knows which it is.
- **A write already on its way** when the edit arrived is refused: its
  control returns to the value in force, and the notice is the reason
  (`apps.md` Conventions 19).
- **When fixed**, the notice leaves, the rows enable, and the values the
  good edit set appear in place (Conventions 21).
- **`tokens.kdl`.** Settings never writes it. A rejected edit there shows
  the same notice, naming `tokens.kdl`, on Appearance and Accessibility
  only, and disables nothing.
- **Isle's apps' settings windows** show the same notice and disable the
  same way, since their sections are in the same file (`conventions.md`).

Whether the first rejection also posts a banner is `notifications.md`'s.
How a safe start (settings ignored) is announced is batch 3's crash
notices; if it is shown in Settings, it takes the same notice slot.

### Recording a chord or detecting a keyboard

A shortcut recorder or Keyboard › Detect asks the compositor for a
recording grab (`apps.md` Settings 9; `compositor.md` Keyboards 6): while
it runs, desktop chords and Cmd translation are suspended and the pressed
keys reach Settings. The recorder shows "Type shortcut" in its field;
Escape cancels, Delete clears the binding, losing focus ends it. A chord
the desktop already uses is refused in the row, naming the action that
holds it ("Used by Mission Control"). Keys are named as the keyboard's
layout produces them, from XKB (`apps.md` Settings 10), with the Mac
symbols on a Mac-profile keyboard and words on a PC one (`copy.md` rule
11).

### Opening at a pane

Settings opens at a pane and row given by argument or D-Bus (`apps.md`
Settings 5). If it is not running it starts there; if it is, its window
raises at the new pane and the move goes on the back history. If a sheet
is open, the window raises and the request waits until the sheet closes,
so a half-filled sheet is never lost.

| From | Opens |
|---|---|
| System menu: About This Computer | General › About (`menu-bar.md`) |
| System menu: System Settings… | The pane last shown |
| System menu: Software Update… | General › Software Update |
| A Control Centre module's detail: "{pane} Settings…" | Its pane: Wi-Fi, Bluetooth, Sound, Displays, Focus, VPN, Energy, Keyboard, Phone, Accessibility (`shell.md` Control Centre 3; `control-centre.md`) |
| Control Centre's footer: Control Centre Settings… | Control Centre (`island.md`) |
| The wallpaper's menu: Wallpaper Settings… | Wallpaper (`wallpaper.md`) |
| Spotlight: a settings result | Its pane, the row marked (`spotlight.md`) |
| A notification's action from the desktop | The pane it names (an update finished: General › Software Update) |
| The configuration glyph | Not Settings: the notice row in Control Centre, whose "Open in editor" opens the file (`island.md`) |
| `isle settings open <pane> [row]` | As Spotlight |

### First run

- **No setup assistant.** Isle's defaults are the setup: a PC-layout
  keyboard gets the Alt and Super swap (B30), appearance follows Auto,
  the wallpaper is Isle's drawn backdrop (`wallpaper.md`).
- **v1's preferences are imported once and silently** (B27; `platform.md`
  Settings 10); keys with no new home are written to the log. Show
  changed settings then lists what came over.
- **Settings' first open** is on General, where About and Software Update
  are; after that it opens at the pane last shown (v1 D62; Conventions 6).

### Each pane's rows

Built in P13 from the rows gated on `p13/rows-1.html` (`p13/rows.md`, every
recommendation taken). Kinds: **C** control, **N** navigation, **D**
details, **L** list, **A** action, **V** live (Rows, above). Keys are
`settings.kdl`'s; a keyed entry is `<id>`. "Keep or revert" marks a change
the compositor holds until kept (B96). Built beyond the board: Identify in
Displays' foot (each screen's name on it for a few seconds), and the
Desktops and Spotlight categories of Keyboard Shortcuts… carry the desktop
and window-drag modifier pop-ups and the terminal's command.

#### Group 1

##### Wi-Fi

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

##### Bluetooth

*Connect headphones, keyboards and other wireless devices.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| — | Bluetooth — *Now discoverable as “{computer name}”.* while on and shown | C · switch | | BlueZ adapter `Powered`; `Discoverable` while the pane is shown (*crate addition*) |
| My devices | {device} with its glyph — *Connected* / *Not connected*; battery after it when reported | C · ⋯ menu: Connect or Disconnect, Rename…, Forget This Device… | | BlueZ `Connect`, `Disconnect`, `Alias`, `RemoveDevice` (rename and forget are *crate additions*) |
| Nearby devices | {device}; a spinner by the title while scanning | A · "Connect" | pairing is the compositor's prompt (B28) | BlueZ discovery, only while shown |
| Missing | "No Bluetooth adapter is connected." | state | | |

##### Network

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

##### VPN

*Turn your VPN connections on and off.* Shown only while a VPN exists
(`settings.md`, The panes); reached from search otherwise with "No VPNs".

| Group | Row | Kind · control | Bound to |
|---|---|---|---|
| — | {vpn} — *Not connected* / *Connected* / *Connecting*, its kind ("WireGuard") after | C · switch; ⋯ menu: Remove… | NM activate, deactivate, delete |

Providers and import are P17 and P20.

##### Energy

*Choose how the computer trades speed for power.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| — | Energy Mode — *High Power while Game Mode is on.* only during the hold | C · segmented | Low Power, Automatic, High Power | power-profiles-daemon (`isle-power` `set_profile`) |

#### Group 2

##### General

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

##### Accessibility

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

##### Appearance

*Choose light or dark, the accent colour and the glass.*

| Group | Row | Kind · control | Choices, range | Bound to |
|---|---|---|---|---|
| Lead | Appearance | C · pictures | Auto, Light, Dark | `appearance.mode` |
| — | Colour | C · swatches | Blue, Purple, Pink, Red, Orange, Yellow, Green, Graphite (the tokens' accents; no picker) | `appearance.accent` |
| — | Glass | C · slider, words | Clear to Tinted, 0 to 1 | `appearance.glass` |
| Windows | Show title bars on other apps' windows | C · switch | | `windows.title-bars` |

The `tokens.kdl` notice shows here and on Accessibility only.

##### Apps

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

##### Control Centre

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

##### Desktop

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

##### Displays

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

##### Spotlight

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

##### Wallpaper

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

#### Group 3

##### Notifications

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

##### Sound

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

##### Focus

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

#### Group 4

##### Lock Screen

*Set when the screen dims, turns off and locks.*

| Group | Row | Kind · control | Choices | Bound to |
|---|---|---|---|---|
| — | Dim the display after | C · pop-up | 1 min, 2 min, 5 min, 10 min, 15 min, 30 min, 1 hr, 2 hr, 4 hr, — Never | `lock-screen.dim-after` (minutes, 0 never) |
| — | Lock after | C · pop-up | as above | `lock-screen.lock-after` |
| — | Turn display off after | C · pop-up | as above | `lock-screen.display-off-after` |
| — | Lock when the computer sleeps | C · switch | | `lock-screen.lock-on-sleep` |

A hand-edited value off the steps shows as its own item in the menu.

##### Privacy & Security

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

#### Group 6

##### Keyboard

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

##### Mouse

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

#### Outside every pane

Read today and given no row: `keyboard.untranslated-apps`' defaults beyond
what an app's page shows (remote desktops and VMs, listed on their own
pages), `windows.not-games` beyond the same, `control-centre.tray`
(order, set by dragging in the panel), `capture.*` but `sound` (the
capture toolbar's Options), the `debug` section, and `output.<id>`'s
flipped transforms. They stay hand edits, shown by Show changed settings.

## States

| State | What shows |
|---|---|
| Rest | The sidebar with the shown pane selected; the pane |
| Searching | Results in place of the pane list; the selected result's pane in the content |
| Found row | The ring around the row until the user acts |
| Empty | A pane with nothing to list says what will appear ("No printers", "No calendars"), with its + to add one |
| Loading | The pane's header and labels at once; a row waiting on its daemon shows its label and no value (`conventions.md`); nothing waits on the compositor's values, which arrive with the window |
| Missing daemon or device | The pane says so in place of the rows that need it ("No Bluetooth adapter is connected."); a conditional pane (Phone, Trackpad) is absent from the list but reached from search |
| Refused value | The value stays on its row with the reason under it |
| Rejected hand edit | The notice on every pane; rows bound to `settings.kdl` disabled |
| Changed settings shown | Marks on changed rows and panes |
| Recording a chord | The recorder field waiting; desktop chords suspended |
| A privileged change | The desktop's polkit dialog above; the row shows its old value until granted, and "Permission was not given." under it if refused (`copy.md`) |
| A display change pending | The keep-or-revert sheet with its countdown |
| Many panes | The sidebar scrolls; the selected pane is scrolled into view |
| A long pane | The content scrolls under the toolbar, which turns solid |
| Second screen | Displays draws both; Settings itself is a window on either |
| Game in front | An ordinary window; no transitions while Game Mode is on. Live rows sample only while shown |
| Locked | Nothing shows |
| Focus on | No change |
| Reduce Motion | Pane changes cut; the found-row ring appears and fades without growing; the arrangement's snap is a cut |
| Reduce Transparency | The sidebar solid |
| Increase Contrast | Borders on groups, fields and controls (`conventions.md`) |
| Largest text size | Rows grow and labels wrap; the sidebar widens to hold pane names; the pane header's subtitle wraps |

## Opening and closing

Settings opens from its desktop entry, Spotlight, the system menu, a
Control Centre detail, a notification or the CLI, at the pane asked for
(Opening at a pane). One window; launching it again raises it (`apps.md`
Conventions 3). Closing the window quits it (Conventions 4); an update it
started keeps running (`apps.md` Settings 12) and reopening shows where it
is. Changing pane replaces the content in place with a short crossfade,
anchored, with no slide; a sub-page and Back do the same. Sheets and the
keep-or-revert sheet are `conventions.md`'s. Values are stage 4's.

## Keyboard

| Key | Action |
|---|---|
| ⌘F | Focus the search field |
| ⌘[, ⌘] and the mouse's back and forward buttons | Back, forward (`apps.md` Settings 8) |
| Up, Down in the sidebar | Select and open the previous or next pane |
| Down from the search field | Into the results |
| Return on a result | Go to the row |
| Escape | Clear the search; close a sheet; cancel a recording |
| Tab, Shift+Tab | Sidebar, then the pane's controls in order, then the toolbar (`conventions.md`) |
| Space | Toggle the focused switch, press the focused button |
| Arrows | Inside pop-ups, segmented controls, sliders, lists and the Displays map |
| ⌥ arrows, ⌘ arrows | Move the selected display, or the main-display mark, in the Displays map |
| ⌘W, ⌘Q | Close the window, which quits |

The View menu lists every pane, as macOS's does, with Show changed
settings. Every pane and row is reached from the keyboard; the Displays
map and the shortcut recorder are the two places where the pointer has a
gesture, and both have a key route above.

## Pointer

- **Click** a pane, a result, a row's control; a navigation row opens its
  sub-page.
- **Secondary click** on a row: "Reset to default" when its value differs
  from the default; "Copy" for values worth copying (the IP address, the
  serial number).
- **Drag**: displays and the main-display mark in the arrangement; sliders;
  reordering in lists whose order matters.
- **Hover** opens nothing; over the Displays map it shows each screen's
  name on the real screen. A truncated label shows in full in a tooltip.
- **Scroll** scrolls the sidebar or the pane under the pointer.

## Shared places

| Place or input | Competes with | Proposed winner |
|---|---|---|
| A setting's state | Control Centre's quick control for the same thing | One state. Control Centre acts quickly, Settings fully; each shows the other's change live (Conventions 21). A module's detail ends with "{pane} Settings…" to its pane; Settings has no link back |
| The rejected-edit notice | The island's glyph and notice row (`island.md`); a banner on first rejection (`notifications.md`) | All, from one source: the compositor's error. The island says it from anywhere; Settings says it in every pane and holds the rows; both offer "Open in editor" |
| Settings results | Spotlight's settings source | One index; the same row and mark either way |
| Focus over Settings | A polkit dialog for a privileged row; the Wi-Fi password prompt when joining from Wi-Fi; Bluetooth pairing from Bluetooth | The system prompt, above the window (`competition.md` Keyboard focus 2); Settings registers no agent and draws no password field (`apps.md` Settings 16; B12) |
| Keys while recording | Desktop chords and Cmd translation | The recorder, until Escape, a chord or losing focus. No island glyph: it is the user's own act in a focused window and ends with focus. The lock's idle timer still runs |
| Each screen's centre while arranging | Other surfaces | The display names, in the top layer, only while the pointer is over the map or a drag is in progress; they take no input. *Added for `competition.md`* |
| Glyph tiers | The island's secondary click on a glyph (`island.md`) and Settings › Control Centre | Two routes to one value |
| Module list | Control Centre's Edit controls (`control-centre.md`) and Settings › Control Centre | The pane shows which modules are on and opens Edit controls for their arrangement; the arrangement has one editor, in the panel |
| Hot corners | `dock.md` For the user 3's defaults | Set in Desktop › Hot Corners…; the defaults as accepted |
| Game Mode | Focus › Game Mode, and Displays for VRR and tearing per output | Folded (For the user 2) |
| An app's preferences | System Settings and the app's own settings window | The app's window; desktop policies about the app in System Settings (`conventions.md`) |

## Answers

1. **The window's layout** (proposal: a sidebar of panes, search at its
   top, the user's account above the panes, each pane a list of grouped
   rows). **Adopted**: the account row opens Users & Groups at the user's
   own account, there being no cloud account; the content column has a
   maximum width for the ultrawide; no sidebar toggle; each pane opens with
   a header of its name and one sentence.
2. **The pane list, names, order and grouping** (proposal: macOS's names
   and places, Isle-only panes beside their nearest neighbour). **Adopted**
   on macOS 26's six groups: no Dock pane; Desktop from Desktop & Dock with
   hot corners; Control Centre for the island's glyphs, the modules and the
   tray; Apps, Phone and Calendars as Isle's own; Passwords and Snapshots in
   General; VPN its own pane. Every v1 page is placed in the table.
3. **Focus as a pane of its own** (proposal: yes). **Adopted**, with Do Not
   Disturb and Game Mode as modes in it.
4. **Game Mode's settings** (fold into Focus › Game Mode, or a pane).
   **Folded**, proposed: For the user 2.
5. **How a found row is marked.** **New**: scrolled a third of the way
   down and ringed in the accent until the user acts, opening its sheet or
   sub-page first; the same from Spotlight.
6. **KDE's "highlight changed settings"** (`design-references.md` §13).
   **Recommended**: For the user 1.
7. **How the Displays arrangement is drawn** (proposal: to scale, snapping
   on drop). **Adopted**, with the main display marked by a small island
   that drags to another display, names shown on the real screens while
   arranging, keyboard moves, and a keep-or-revert sheet for mode changes.
8. **Control types by kind of value** (v1). **Adopted** as the table.
9. **Copy** (`copy.md`). **Adopted**: one-sentence subtitles, descriptions
   only where the label leaves something to say, no instructions under
   switches; disabled reasons as accessible descriptions.
10. **A rejected hand edit** (`platform.md`: shown by the shell with file,
    line, column and message until fixed; `island.md` already has the
    glyph and the notice row). **Answered** for Settings: a notice on every
    pane with the same actions, rows bound to the file disabled at the
    values in force, daemon rows working, `tokens.kdl` errors shown on
    Appearance and Accessibility only.
11. **First run.** **New**: no assistant, a silent one-time import, General
    first.
12. **Deep links** (`apps.md` Settings 5). **Adopted**, with the table of
    callers, the history entry, and a request waiting behind an open sheet.
13. **The app's name.** **New**: "System Settings" wherever the app is
    named, "Settings" in paths.

## Copy

| Where | String |
|---|---|
| App name | "System Settings" |
| Search | Placeholder "Search"; "No results for “{query}”" |
| Pane subtitles | One sentence each, under seventy characters: "Arrange and set up the screens you use." (Displays), "Choose what interrupts you, and when." (Focus), "Set what the island and Control Centre show." (Control Centre); the rest are each pane's packet |
| Rejected hand edit | `copy.md`'s string with the column: "settings.kdl has an error on line 12, column 4. Changes won't be saved until it's fixed.", then the parser's message; "Open in editor", "Show in Files" |
| Rejected token edit | "tokens.kdl has an error on line 12, column 4. The last good look is in use." |
| Changed settings | "Show changed settings", "Hide changed settings", "Reset to default" |
| Displays | "Keep these display settings?", "Reverting in {n} seconds.", buttons "Revert", "Keep"; "Detect Displays"; "Follow focus"; disabled reason "This is the only display that's on." |
| Recorder | "Type shortcut"; "Used by {action}"; Detect's "Press any key…" |
| Missing device, empty list | "No Bluetooth adapter is connected.", "No trackpad is connected.", "No printers", "No calendars", "No login items" |
| Refused value | "Couldn't change {setting}. {reason}" |
| Privileged change refused | "Permission was not given." (`copy.md`) |
| General › Storage | "{size} safe to clear", "Review", "Empty Trash…", "Large and untouched files" |
| Hot Corners | "Hot Corners…", the corner actions as `shell.md` Mission Control 8 names them, and "—" for none |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Showing what has been changed.** KDE marks every setting that differs
   from its default (`design-references.md` §13), which on Isle is also a
   readable diff of a hand-edited `settings.kdl`. (a) View › Show changed
   settings, off by default, marking changed rows and their panes, with
   Reset to default in a row's context menu; (b) always on; (c) not at all,
   as macOS. **Recommended: (a).** It costs nothing at rest (brief P1),
   gives back what a hand edit or the v1 import changed, and Reset to
   default is useful with the view off too.
2. **Game Mode's settings.** Game Mode is the Gaming Focus (B30), and its
   own effects (the performance profile held, night light off) are fixed
   behaviour rather than settings (`gaming.md` Game Mode 3). (a) Fold:
   Focus › Game Mode carries its automatic start for detected games and its
   allowed apps like any mode, with a line saying what Game Mode does; VRR
   and tearing, which are display settings, go to Displays; (b) a Game Mode
   pane of its own in the second group. **Recommended: (a).** One switch
   per thing in one place (`design-v1-lessons.md` §8), and a Mac user looks
   for Focus behaviour under Focus and display behaviour under Displays; a
   pane would hold a single switch.
