# Control Centre

Batch 2. Answers to: `shell.md` Control Centre (modules, names, order and
defaults; whether controls appear anywhere else; how a control's detail
opens) and Control Centre 2–9; `gaming.md` Controllers (where pad batteries
show) and Games as apps (a game's art in per-app volume); `apps.md`
Settings (what moves there from the panel, in part). Decisions B11, B12,
B14, B17, B18, B28, B30.

`island.md` fixes how the panel opens (hover with v1's guards, a click
pins with focus, Ctrl+F8), that its top row is the pill's own row, and its
sections: Now, modules, tray, footer. This sheet designs the modules and
what follows from them.

## Purpose

The quick controls: the few switches, sliders and choices the user reaches
for many times a day, one hover from anywhere (brief P3: macOS's Control
Centre, opened from the island as v1's panel was). Each module shows one
thing's state, changes it in one click, and leads to a page with the
choices behind it (networks, devices, outputs, players). It is not
Settings: anything set once and left (schedules, profiles, arrangements,
forgetting a device) is a Settings pane that a module links to. It is not
a status area at rest: the island's glyphs show state and open modules,
and nothing here shows until the panel opens (brief P1).

## Content

### The grid

Four columns of equal cells (`design-macos.md` §3.7). A module takes one of
four sizes:

| Size | Cells | Shows |
|---|---|---|
| Small | 1 | The module's glyph, which is its switch or action; nothing else |
| Medium | 2 side by side | Glyph, name, a state line |
| Large | 2 × 2 | The module's content: art and transport, a list of devices |
| Wide | 4 in a row | Glyph, name and a slider, or a short row of content |

The arrangement is an ordered list. Modules are packed in reading order,
each into the first place it fits, so a module that is absent (no Wi-Fi
adapter, no VPN set up, no battery anywhere) leaves no hole and the grid
closes up. The same arrangement serves every screen. There is one grid,
not macOS 26's extra pages: the panel scrolls before it pages
(Panel height below).

### The modules

| Module | Sizes (default) | In the grid by default | Click on the glyph | Page | Glyph that opens it | Source |
|---|---|---|---|---|---|---|
| Wi-Fi | S, **M** | Yes, when a Wi-Fi device exists | Wi-Fi on or off | Networks | Wi-Fi | network |
| Bluetooth | S, **M** | Yes, when an adapter exists | Bluetooth on or off | Devices and pairing | Bluetooth | bluetooth |
| Now Playing | S, M, **L**, W | Yes | Play or pause | Players | (the island's media face) | media |
| Focus | S, **M** | Yes | The shown Focus on or off | Focus modes, Game Mode included | Focus | Focus service (`focus.md`) |
| VPN | S, **M** | Yes, when a VPN provider or connection exists | Connect the last VPN, or disconnect | Providers and connections | VPN | providers (`vpns`), network |
| Sound | S, M, **W** | Yes | Mute | Outputs, per-app volume, input | Sound | audio |
| Display | M, **W** | Yes | (none; the slider) | Brightness per screen | | brightness |
| Night Light | **S**, M | Yes | Night light on or off | None | | compositor (`compositor.md` Outputs 8) |
| Energy Mode | **S**, M | Yes | (none; opens its page) | Low Power, Automatic, High Power | | power |
| Updates | S, **M** | Yes | (none; opens its page) | Waiting, installing, restart owed | Updates | packages |
| Batteries | S, **M**, L | Yes, when any battery is reported | (none; opens its page) | Every battery | Battery | power, bluetooth |
| Dark Mode | **S**, M | No | Dark or light | None | | appearance setting |
| Keyboard layout | S, **M** | No | (none; opens its page) | The focused keyboard's layouts | | compositor keyboards (B30) |
| Accessibility | S, **M** | No | (none; opens its page) | Zoom, On-screen keyboard, Increase Contrast, Reduce Motion, Reduce Transparency | | settings, compositor |
| Screenshot | **S** | No | Opens the capture toolbar | None | | capture (`capture.md`) |
| Phone | S, **M** | No, offered when a phone provider exists | The provider's primary action, if it has one | The provider's state sentence and actions | | providers (`phones`) |

Notes on the set:

- **Game Mode is not a module.** It is the Gaming Focus (B30), so its
  switch is the Game Mode row on the Focus page and the Focus module shows it
  while it is on. A second Game Mode module would be a second switch for
  one thing (brief, v1 lessons §8). See For the user 3.
- **Display is brightness only.** Night light and dark mode are their own
  modules, one click each, instead of macOS's switches inside Display's
  page: one switch per thing, in one place, at the grid's level where v1
  had night light.
- **Batteries** is macOS's Battery module widened to every battery the
  system reports: the machine's on a laptop, and here, on a desktop, the
  mouse, keyboard, headset and pads (UPower and BlueZ). It answers
  `gaming.md` Controllers below.
- **Energy Mode** is power-profiles-daemon's three profiles under macOS's
  words (copy.md rule 2): Low Power is power-saver, Automatic is balanced,
  High Power is performance.
- **Provider modules** (VPN, Phone) draw the provider's state sentence and
  actions and know nothing of the tool (`shell.md` Control Centre 5). There
  are no third-party modules: other programs reach the panel through the
  tray, and widget providers (B14) stay on the dashboard.
- **Microphone** has no module: its device, level and mute are on the
  Sound page, and the mute key raises an OSD.

### The default arrangement

In order: Wi-Fi, Now Playing, Bluetooth, Focus, VPN, Sound, Display, Night
Light, Energy Mode, Updates, Batteries. Packed, on this machine with every
module present:

```
┌──────────┬──────────┐
│ Wi-Fi    │ Now      │
├──────────┤ Playing  │
│ Bluetooth│          │
├──────────┼──────────┤
│ Focus    │ VPN      │
├──────────┴──────────┤
│ Sound ───────────── │
├─────────────────────┤
│ Display ─────────── │
├────┬────┬──────────┤
│ NL │ EM │ Updates  │
├────┴────┼──────────┘
│Batteries│
└─────────┘
```

Connectivity and media on top, as on macOS; the two sliders together; the
rarely touched last. On a wired desktop with no Wi-Fi device, Now Playing
moves to the top left and the rest close up behind it.

### A module's parts

- **Glyph.** The module's switch or action. On means accent (brief P6);
  off is neutral. A module with no switch (Display, Energy Mode, Updates,
  Batteries, Keyboard layout, Accessibility) has a glyph that is only a
  mark, and its whole surface opens its page.
- **Name and state line** at Medium and larger. The state reads as a state
  (copy.md rule 10): the network, the device, the mode, "Off".
- **Body.** At Medium and larger, a click on the name and state opens the
  module's page. At Small, a click is the glyph's action; the page is
  reached from Return and from the secondary-click menu.
- **Content** at Large and Wide: the slider, the art and transport, the
  devices.

### Pages

A module's detail is a page in the panel, as v1's drill-downs (`island.md`;
`shell.md` Control Centre 3). The page replaces the Now, grid and tray
sections; the pill's row stays above it and the footer below.

- **Head.** A back control, then the module itself at full width: its
  name, state and switch or slider. The grid is not on screen while a page
  is, so this is the same control moved up, not a second one; nothing
  else on the page toggles what it toggles (For the user 1).
- **Body.** Rows that choose (a network, an output, a player, a Focus) or
  act on one thing (connect a device, set one screen's brightness). A row
  with more behind it has a chevron to its own sub-page. No row repeats the
  head's switch (v1 `49a415d`, `921f032`).
- **Foot.** One link to the matching Settings pane, "{Pane} Settings…",
  which folds the panel and disarms hover (`island.md` Pointer).

What each page holds:

| Page | Rows |
|---|---|
| Wi-Fi | A wired connection's state when there is one ("Connected", "Cable unplugged"); the current network with signal and security; known networks in range; "Other Networks", collapsed; "Join Other Network…" (Settings). Scans only while open. A row connects; a secured network NetworkManager has no secret for raises the Wi-Fi prompt (`prompts.md`; `shell.md` Control Centre 3), never a field in the panel. The row shows "Connecting", then the result or the error in the row |
| Bluetooth | Connected devices with battery; "My devices" (paired); "Nearby devices", discovered only while the page is open. A paired row connects or disconnects; a nearby row pairs, and a passkey or PIN is the pairing prompt (`prompts.md`; B28). Forgetting a device is Settings' |
| Now Playing | The chosen player in full: art, title, artist, scrubber, transport, shuffle and repeat where offered, and its name, which raises it; then every other MPRIS player as a row, choosing it makes it the one shown |
| Focus | Do Not Disturb, Game Mode and the user's modes, each with its symbol; the one on is marked with what turned it on (by hand, a schedule, a game in front). A row turns that mode on, or off if on; its chevron offers durations. What wins when modes overlap, and what turning Game Mode off over a detected game does, are `focus.md`'s |
| VPN | Per provider: its state sentence, its connections or choices as rows that connect (v1 `921f032`), its actions (Sign in, a terminal action). NetworkManager VPN connections under their own heading. A VPN secret is the VPN prompt (`prompts.md`) |
| Sound | Output devices, the default marked; a Bluetooth headset with its battery. Then each app playing sound, by its icon (a game's cover art, as in the switcher: `gaming.md` Games as apps) and name, with its own slider and mute; absent when nothing plays. Then input devices, the input level with its mute and a level meter that samples only while shown (`shell.md` Shell runtime 7) |
| Display | One brightness slider per screen, named by the monitor (EDID, or the user's name for it in Displays), the screen the panel is on first. DDC shows the requested value at once and writes debounced (`shell.md` Control Centre 6); a monitor that answers no DDC says so in its row |
| Energy Mode | Low Power, Automatic, High Power as choices; while Game Mode holds High Power, the row says so, and a choice made during Game Mode stands (`gaming.md` Game Mode 4) |
| Updates | "Up to date" with the last check and "Check now"; or the count by source (repositories, AUR, Isle) with "Install updates…", which opens Settings › Software Update; or the run's progress ("Installing 4 of 14"); or the restart owed with its reasons (`island.md`) and "Restart…", which opens the session dialog |
| Batteries | Every battery: the machine's with time left on a laptop, then devices by kind (mouse, keyboard, headset, pads), each with its name, level and charging. A pad is named as `gaming.md` Controllers 2 names it; a virtual pad carries no battery and is not listed |
| Keyboard layout | The focused keyboard's name and its layouts (B30 per-device profiles); a row switches to it |
| Accessibility | Switches for Zoom, On-screen keyboard, Increase Contrast, Reduce Motion and Reduce Transparency. These are the panel's only switches for them; Settings › Accessibility holds the rest |
| Phone | The provider's state sentence and its actions (ring, send a file, the phone's battery), as the provider lists them |
| Tray item | The item's DBusMenu as rows, submenus as sub-pages (`island.md`) |

### Now

`island.md` fixes Now's content. Its structure in the panel:

- Order: live activities in the queue's order (the leader in full, the
  others one line each, expanding in place), then privacy rows, then the
  notice rows.
- A collapsed row keeps its one action (Stop, Show, Cancel) visible, so
  stopping a share never needs an expand first.
- A screen privacy row is left out while a recording or sharing row names
  the same app.
- Media is not a Now row while the Now Playing module is in the grid: the
  module is media's one place in the panel; Now shows media only when the
  module has been removed (For the user 2).
- Now never scrolls on its own; with more than the cap of rows (stage 4),
  the rest collapse to a count that expands in place.

### Tray

As `island.md`: other programs' StatusNotifierItems as a row of icons in
the user's order, wrapping to a second row rather than scrolling, those
asking for attention marked. A menu opens as a page. Absent when no
program has an item.

### Footer

"Edit controls" and "Control Centre Settings…" (`island.md`). In edit mode
the footer holds "Done" instead.

### Edit mode and the gallery

macOS 26's Edit Controls (`design-macos.md` §3.7), with v1's grid editor's
live outline (v1 D24, D35):

- **Entering.** "Edit controls" in the footer, or "Edit controls" from a
  module's secondary-click menu. The panel pins. Now and the tray fold away
  so the grid is what is edited; live activities stay in the pill's row.
- **The grid in edit mode.** Each module shows a remove badge. Drag moves a
  module, with a live outline of where it lands, the others reflowing round
  it. Secondary click offers the sizes the module supports and Remove.
- **The gallery** opens beside the panel, on its leading side, anchored at
  the same top: a search field, then every module not in the grid, grouped
  (Connectivity; Sound and display; Focus; System; Accessibility;
  Providers). A module that cannot show (no provider, no adapter) is listed
  disabled with why and "Set up…", which opens its Settings pane. Drag
  from the gallery into the grid adds a module; dragging out of the grid
  removes it.
- **Leaving.** Done, Escape, or the panel closing. Changes apply as they
  are made (`apps.md` Settings 6); "Reset to default" is in Settings.

### Not in Control Centre

What moves to Settings (pane names are `settings.md`'s):

| What | Where |
|---|---|
| The island's glyph tiers; the module list, order and sizes as a keyboard-first alternative to edit mode; Reset to default | Settings › Control Centre |
| Night light's schedule and warmth | Displays |
| Display arrangement and Redetect; per display, mode, scale, HDR, VRR and tearing | Displays (`apps.md` Settings 13). VRR and tearing are per output and apply to any full-screen window, so they are not Game Mode's (`focus.md`) |
| Focus modes: allowed apps, schedules, Game Mode's automatic start and the games it is off for | Focus (`focus.md`) |
| Forgetting a Wi-Fi network or Bluetooth device; hidden networks; network details | Wi-Fi, Bluetooth, Network |
| VPN and phone providers: adding, grants, sign-in details | Network › VPN, Phone (`apps.md` Settings 11) |
| Alert sounds and their volume | Sound |
| Keyboard profiles and layouts | Keyboard |
| Update runs, history and logs | Software Update |
| Idle timers, battery thresholds | Energy, Lock Screen |
| Text size and the other accessibility settings | Accessibility |

Considered and left out: a "keep awake" switch (`design-references.md`
§8): not in the intent, and Monitor lists inhibitors; macOS 26's extra
pages; tabs as in Noctalia (§8), since one grid that scrolls keeps the
keyboard path short.

### Panel height and scrolling

The panel grows downward from the pill to fit its content, up to a maximum
height: the work area's height less a margin. Past it, the sections
between the pill's row and the footer scroll as one; the pill's row and
the footer stay. A page scrolls under its head. Long lists on a page
(networks, nearby devices, players, per-app streams) scroll within the
page; nothing is cut off. On the 3440×1440 screen the default arrangement
with Now and a tray fits without scrolling; the 1080p screen may scroll.

## States

| State | What shows |
|---|---|
| Closed | Nothing. Control Centre has no rest form; the island's glyphs are its state at rest. |
| Peeked | The panel as above, without keyboard focus (`island.md`). |
| Pinned | The same, with keyboard focus on the first module. |
| Page | The module's page, head, rows and foot. |
| Sub-page | A row's or a tray submenu's page, with back to its page. |
| Edit mode | The grid with remove badges, the gallery beside it, Done in the footer. |
| Empty | Nothing live and no tray: the pill's row, the grid and the footer. The grid is never empty: removing every module leaves one line, "No controls", and Edit controls. |
| Loading | A module whose service has not answered yet shows its name and its last known state, controls disabled until the first answer; nothing waits (`shell.md` Shell runtime 8). A page that scans (Wi-Fi, Bluetooth) shows what it has and adds rows as they come, without reordering under the pointer. |
| Error | A dead service's module reads "Not available" with its controls disabled; its page says what is missing and links to Settings. A failed action shows its error in the row it came from, in copy.md's form, and the control returns to the true state. A provider that times out shows its error as its state sentence. |
| A device absent | No Wi-Fi device, no Bluetooth adapter, no VPN, no battery: that module leaves the grid and is listed in the gallery as unavailable. |
| Many items | Pages scroll; the tray wraps; Now collapses past its cap. |
| Overflow | The panel scrolls past its maximum height (above). |
| Second screen | Control Centre opens on the screen whose island was used, with the same arrangement; Display shows that screen's brightness first; Now as `island.md` (Now included on every screen). Opening it on one screen closes it on the other. |
| Game in front, windowed | As normal (brief), drawn in the solid form without transitions (`competition.md` For the user 1). Focus shows Game Mode; Energy Mode shows the hold; Night Light shows "Off during Game Mode". |
| Full screen, game or not | Reached only through the island's return at the top edge (`island.md`); over a game, after the dwell, never while the game holds the pointer. Solid, no transitions over a game. Ctrl+F8 does not reach it over a game (B30's escape set). |
| Locked | Nothing. The lock has its own media controls and battery (`lock.md`). |
| Focus on | The Focus module shows the mode and its glyph. Nothing else changes. |
| Reduce Motion | No growth from the module into its page and no reflow motion in edit mode: content changes at once or by a short fade. |
| Reduce Transparency | The panel and the gallery are solid. |
| Increase Contrast | Modules, rows and sliders get edges; on and off differ by more than colour (a filled glyph against an outlined one). |
| Largest text size | Cells scale with the text, so the arrangement holds; the panel grows wider and taller, up to the screen, and scrolls sooner. Medium and larger modules let the state line take a second line; Small modules carry no text, and their name is in the tooltip and the accessible name. |

## Opening and closing

- **The panel** opens and closes as `island.md` sets: peek on hover,
  pinned by a click, Return or Ctrl+F8, from the island's own row.
- **A glyph's click** opens the panel pinned at that module's page, with
  back leading to the grid.
- **A page** grows out of its module: the panel's shape morphs to the
  page's height while the grid fades and the page comes in, anchored at the
  top, never sliding sideways (brief P5). Back reverses it, faster. A
  sub-page does the same from its row.
- **Edit mode** fades the remove badges in and opens the gallery beside the
  panel, growing from the panel's edge; Done reverses it.
- **Opened from elsewhere.** The CLI and D-Bus open the panel at a module's
  page (`shell.md` Shell runtime 16), which the island's glyphs use.
- **Closed by** Escape at the grid, a click outside, Ctrl+F8 again, the
  pointer leaving a peek, a top-layer surface opening, or anything in
  `island.md`'s list (Spotlight, Mission Control, the capture tool, the
  session dialog), and the widgets dashboard. A system prompt raised from a
  row (a Wi-Fi password, a pairing code) closes nothing: it takes focus
  over the pinned panel and gives it back when it closes, so the row's
  result is seen.
- **Character.** Critically damped, no bounce; out faster than in; over a
  game and under Reduce Motion, none.

## Keyboard

This refines `island.md`'s map for the grid, where the arrows move.

| Action | Key |
|---|---|
| Open pinned, focus on the first module | Ctrl+F8 |
| Move between sections: pill's row, Now, modules, tray, footer | Tab, Shift+Tab |
| Move between modules | Arrows, by position in the grid; Home, End for the first and last |
| The module's switch or action (toggle, play or pause, mute) | Space |
| Open the module's page | Return; on a module with no switch, Return or Space |
| Adjust a slider module from the grid | + and − (= and -), a step at a time; Shift for a larger step |
| On a page: move between rows and controls | Up, Down; Tab into a row's own controls |
| Choose or act on a row | Return or Space |
| Open a row's sub-page | Right |
| Back | Escape, Cmd+[, or Left on a row; the mouse's back button |
| A focused slider on a page | Left, Right, Home, End |
| Find a row by name on a long page | Typing, as in a menu |
| Tray: move between items | Left, Right |
| Tray: the item's click (menu page or Activate) | Return or Space |
| Tray: the item's menu page | Right, or the menu key |
| Tray: SecondaryActivate | Option+Return |
| Tray: reorder | Cmd+Left, Cmd+Right |
| A module's secondary-click menu | Shift+F10, or the menu key |
| Edit controls | E on the grid, or the footer's button |
| Edit mode: move the selected module | Cmd+arrows; the others reflow |
| Edit mode: remove it | Delete |
| Edit mode: its sizes | Return opens the size menu |
| Edit mode: to the gallery and back | Tab; Return on a gallery module adds it at the end |
| Edit mode: leave | Escape, or Done |
| Close | Escape at the grid; focus returns to the window that had it |

Volume, brightness and media keys move the matching module's control in
place while the panel is open (`island.md`).

## Pointer

- **Hover** over the island peeks the panel (`island.md`). Inside, hover
  highlights a module or row only when the pointer moves, and opens
  nothing: pages open on a click.
- **Click** on a glyph acts; on a Medium or larger module's name and state,
  opens its page; on a row, chooses or acts. A click inside a peek also
  pins it.
- **Secondary click** on a module: Show details, its sizes, Remove, Edit
  controls, "{Pane} Settings…". On a page's row: the row's own menu
  (Disconnect, "{network} settings…"). On a tray item: its menu page.
- **Drag**: sliders and the scrubber; modules in edit mode; tray items with
  Cmd held. A drag that starts in the panel holds it open until release,
  wherever the pointer goes.
- **Scroll** scrolls the panel or the page; it never moves a slider, so
  scrolling the panel cannot change the volume.
- **Middle click** on a tray item: SecondaryActivate (`island.md`).

## Shared places

What this sheet adds to `competition.md`, beside `island.md`'s entries,
which it keeps:

- **Beside the panel: the gallery.** In edit mode the gallery sits beside
  Control Centre at the same top, over window content, and leaves with
  edit mode. Nothing else opens there; it is part of the panel for the
  "one menu-bar surface per screen" rule.
- **The media row in Now against the Now Playing module.** Changes
  `island.md`'s "two views of one player": the module is media's place in
  the panel, and Now shows media only when the module is not in the grid.
  The island's compact face is unchanged (For the user 2).
- **Glyph against module.** Agrees with `island.md`: a glyph opens its
  module's page; switches and sliders exist only in Control Centre. A
  page's head carries its module's own control, so a page opened from a
  glyph is complete (For the user 1).
- **Control Centre against the widgets dashboard.** The dashboard closes
  the panel. If the dashboard keeps v1's Controls widget (`shell.md`
  Widgets 2), it draws these modules with the same state, never a second
  set of switches; the two are never on screen together (`widgets.md`).
- **Control Centre against Settings.** Settings is a window with its own
  rows; a module's "settings…" link folds the panel and opens the pane.
  The panel holds the quick control, Settings the configuration
  (Not in Control Centre above).
- **Control Centre against prompts.** A row that needs a secret or a
  pairing code raises the prompt, which takes focus over the pinned panel
  and returns it (`competition.md` keyboard focus 2 and 5).
- **Control Centre against the capture tool.** Screenshot opens the capture
  toolbar, which closes the panel (`shell.md` Shell runtime 10).
- **Second screen.** One Control Centre open at a time across screens.
- **Keys.** Inside the grid the arrows move and Space and Return act and
  open, refining `island.md`'s "Right or Return" for modules; Right still
  opens sub-pages and tray menus. E (edit) and + and − apply only with
  focus in the grid. Volume, brightness and media keys keep their meaning.

## Answers

- **Control Centre: modules, names, order, defaults** (proposal: Wi-Fi,
  Bluetooth, Focus, Sound, Display, Now Playing, Game Mode, VPN, Power
  profile, Battery, Updates, Keyboard layout, Phone, Accessibility, Screen
  capture): **changed**. Game Mode folds into Focus as a row of its page (B30;
  one switch per thing). Display keeps brightness, and night light and dark
  mode become modules of their own. Power profile is Energy Mode and
  Battery is Batteries, in macOS's words widened to peripherals. Screen
  capture is Screenshot. In the grid by default: Wi-Fi, Now Playing,
  Bluetooth, Focus, VPN, Sound, Display, Night Light, Energy Mode, Updates,
  Batteries; in the gallery: Dark Mode, Keyboard layout, Accessibility,
  Screenshot, Phone. v1's toggles, sliders and footer all have a module.
- **Control Centre: whether toggles and sliders appear anywhere else**
  (answered by `island.md`: no; the glyphs only open modules):
  **adopted**, with two notes. A page's head carries its own module's
  control, which is the same control, not another (For the user 1). The
  dashboard's Controls widget, if kept, draws these modules rather than a
  second set (`widgets.md`).
- **Control Centre: how a control's detail opens** (answered by
  `island.md`: a page in the panel with a way back): **adopted**: the page
  grows out of its module, its head is the module, its foot links to
  Settings, and a glyph opens it directly.
- **Control Centre 4: add, remove, reorder**: macOS 26's Edit controls and
  gallery, with v1's live outline (D24, D35), and a list in Settings for
  the keyboard.
- **Pad batteries** (`gaming.md` Controllers; proposal: Control Centre and
  the Devices widget): **adopted** and widened: the Batteries module and
  page with every peripheral; a pad connected over Bluetooth also in the
  Bluetooth page's row; the island's Battery glyph when one is low
  (`island.md`); the Devices widget (`widgets.md`). A low-battery
  notification is `notifications.md`'s. Virtual pads have no battery and
  are not listed.
- **A game's art in per-app volume** (`gaming.md` Games as apps; proposal:
  its cover): **adopted**: the same icon the switcher uses, so a game looks
  the same everywhere (`switcher.md`).
- **Wi-Fi network list with password** (v1's drill-down): **adopted**; the
  password is the Wi-Fi prompt (`prompts.md`, B12), never a field in the
  panel.
- **Bluetooth devices and pairing**: **adopted**; discovery runs only while
  the page is open, and the codes are the pairing prompt (`prompts.md`,
  B28).
- **Sound output and per-app volume** (v1's output page): **adopted** on
  the Sound page, with input and its mute there too.
- **Brightness per monitor over DDC** (`design-references.md` §8):
  **adopted**: this screen's in the module, every screen's on the page.
- **VPN providers** (v1's providers page): **adopted**: rows connect, no
  switch beside them; the module's glyph connects the last one.
- **Tray as icons with menus as pages** (`island.md`): **adopted**, with a
  full keyboard path.
- **Tabbed Control Centre, detail cards in place, keep awake**
  (`design-references.md` §8): **not taken**; one grid and pages, as
  `island.md` set, and keep awake is not in the intent.

## Copy

| Where | String |
|---|---|
| Module names | "Wi-Fi", "Bluetooth", "Now Playing", "Focus", "VPN", "Sound", "Display", "Night Light", "Energy Mode", "Updates", "Batteries", "Dark Mode", "Keyboard layout", "Accessibility", "Screenshot", "Phone" |
| Wi-Fi states | "{network}", "Not connected", "Off", "Connecting" |
| Bluetooth states | "On", "Off", "{device}", "{n} devices" |
| Now Playing | "Not playing"; the player's name |
| Focus states | "Off", "{Focus}", "{Focus} until {time}" |
| VPN states | "{name}", "Not connected", "Connecting" |
| Sound | the output's name; "Muted" |
| Display | the monitor's name; "Brightness can't be set on this display" |
| Night Light | "On until {time}", "Off until {time}", "Off during Game Mode" |
| Energy Mode | "Low Power", "Automatic", "High Power", "High Power while Game Mode is on" |
| Updates | "Up to date", "1 update", "{n} updates", "Installing {n} of {m}", "Restart to finish updating", "Checked {time}", "Check now", "Install updates…", "Restart…" |
| Batteries | "{device} {n}%", "Charging", "{time} left" |
| Wi-Fi page | "Other Networks", "Join Other Network…", "Cable unplugged", "Couldn't connect to {network}. Check the password and try again." |
| Bluetooth page | "My devices", "Nearby devices", "Connecting", "Pairing", "Couldn't connect to {device}. Make sure it's on and nearby." |
| Sound page | "Output", "Input", "Apps", "No apps playing sound" |
| Display page | "Couldn't set brightness on {monitor}." |
| Pages' foot | "{Pane} Settings…" |
| Errors and empty | "Not available", "No networks found", "No devices", "No controls" |
| Footer | "Edit controls", "Control Centre Settings…", "Done" |
| Edit mode | "Search controls", "Remove", "Small", "Medium", "Large", "Wide", "Set up…", "Show details"; gallery groups "Connectivity", "Sound and display", "Focus", "System", "Accessibility", "Providers" |
| Accessible names | "{Module}: {state}"; a switch adds "on" or "off"; "Back to Control Centre" |

Feature names (Now Playing, Night Light, Dark Mode, Energy Mode, Game Mode,
Do Not Disturb) keep their capitals as names, as Control Centre does
(copy.md rule 1).

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **A page's head and its switch.** v1 took the switches off its
   drill-down pages (`49a415d`). Here a page is also opened straight from a
   glyph, with the grid not on screen. (a) The page's head is the module
   itself, switch included: one control, moved up, so "Bluetooth is off"
   is fixed where it is read. (b) No switch on pages, as v1; a glyph's page
   that needs the switch sends the user back to the grid. *Recommended:
   (a)*: it is macOS's form, keeps one switch on screen at a time, and
   makes a glyph's click complete.
2. **Media in Now and in Now Playing.** `island.md` shows a playing player
   twice in the open panel: a Now row and the module. (a) The module is
   media's one place; Now shows media only when the module is removed.
   (b) Both, as `island.md` has it. *Recommended: (a)*: two transports for
   one player in one panel is the duplicate v1 removed elsewhere, and the
   module sits right under Now. This changes `island.md` at the gate.
3. **Game Mode.** (a) No module: Game Mode is a row on the Focus
   page, and the Focus module shows it while on. (b) A Game Mode module as
   well, a second switch for the Gaming Focus, as v1's panel had a game
   mode tile. *Recommended: (a)*: it comes on by itself for any detected
   game (B30), so a manual switch is rare, and one switch per thing holds.
