# P13 · Settings, core

Planned 2026-10-17. Reviewed by the user the same day: every
recommendation taken.

The roadmap's brief: the Settings app on the app conventions, with
displays, sound, network, Bluetooth, keyboard and shortcuts, appearance,
desktop and hot corners, notifications, Control Centre, Focus and Game
Mode, over the settings P8 added; the modules' "Settings…" links open it.
Privacy & Security lists and revokes the portal's `screenshot` and
`screencast` grants. Displays draws each output's VRR and tearing and the
`night-light` section, and a mode change reverts unless confirmed there.
Settings ships the row index Spotlight reads and opens at a pane and row
from `isle-settings://<pane>/<row>`; Settings › Spotlight edits the
`spotlight` section. An app is marked a game or never one. An app's memory
on the GPU is measured before B26's default is final.

Settings is the first of Isle's five apps, so P13 also builds what every
app stands on. Today nothing does: there is no app process, no desktop
entry or D-Bus service file, no single instance, no DBusMenu exporter, no
way for an app to read the tokens, no background effect, and Isle's apps
are not on the Cmd no-translation list. `isle-ui` already has the
primitives (button, switch, slider, field, search field, menu, glyphs) with
no compositor dependency, and `isle-preview` shows a Freya window running
on `freya-winit` against Isle.

Behind the panes, P2 to P12 left settings with no way to say them. The
store cannot empty a list or unset a key through `Settings1.Set`, so a
shortcut cannot be turned off, `windows.keep-running` cannot be cleared and
`output.mode` cannot go back to automatic; there is no reset and no way to
delete a Focus mode or an app's notification rule; `isle windows game`
fails on the empty default list. Displays reports no modes, rotation or
enabled state, and a mode, rotation or enabled change takes effect only at
Redetect. Brightness has no D-Bus. The network crate has no details,
forget or list of saved networks; Bluetooth cannot forget. Every
"Settings…" link in Control Centre, the system menu's three items and the
`settings` shortcut log "not built yet", and Spotlight's settings source
reads an index that is empty and opens a URI nothing handles.

What P13 must get right is that a change made in Settings is the same
change made anywhere else: written once by the compositor, shown live in
every place that shows it, refused with its reason, and never able to
leave the user with a dark screen. Around that, Settings is an ordinary
app in every way the conventions describe, so the four apps after it
inherit a platform rather than a precedent.

## What it answers to

- Decisions: B2, B7 (Freya apps as processes), B9, B10 (the compositor
  writes settings), B11 (apps read daemons through the service crates),
  B12 and B28 (agents are the desktop's; Settings registers none), B17,
  B18 (menus in the bar), B20, B21 (Super is Cmd; apps take Cmd natively),
  B22, B25 (the unified toolbar), B26 (GPU by default, measured here), B27
  (writes refused while the file does not parse), B30 (Game Mode, VRR's
  setting, the Alt and Super swap), B36, B45, B46, B64, B69, B73, B79, B83,
  B86–B91.
- Intent: `apps.md` Conventions 1–32 and its open points, Settings 1–19
  (bar 11 and 12, General's provider and update rows, wait for P20),
  acceptance bars 1–8 and 11 for Settings; `platform.md` Settings 6–10;
  `compositor.md` Outputs 1–2 (confirm or revert) and Keyboards 6 (the
  recording grab); `gaming.md` Game Mode 2 and Presentation 3–4 as
  settings; `shell.md` Control Centre 3, Spotlight 2, Mission Control 8
  (hot corners).
- Design, binding: `p1/structure/settings.md` and `p1/surfaces/settings.md`
  (the window, rows, controls, search, changed settings, the notice, the
  arrangement, sheets, states, keys, motion, copy, tokens);
  `structure/conventions.md` and `surfaces/conventions.md` (the toolbar,
  sheets, menus, focus); `focus.md`, `notifications.md`, `island.md`,
  `control-centre.md`, `lock.md`, `wallpaper.md`, `mission-control.md` and
  `dock.md` For the user 3 (hot corners' defaults), `capture.md` (Remember),
  `copy.md`.
- Evidence: `settings-2026.md` (new; § below), `ui-and-services.md` §1.4,
  `isle-v1-inventory.md` §5 and §7, strata's `docs/FREYA_STATE_ARCHITECTURE.md`
  §10 and `UI_CONVENTIONS.md`, cairn's `docs/design/concurrency.md`.

## Choices this plan makes

### Scope

1. **The panes P13 builds.** The roadmap's: Wi-Fi, Bluetooth, Network,
   Sound, Displays, Keyboard, Appearance, Desktop, Notifications, Control
   Centre, Focus, Spotlight, Privacy & Security. With them, the panes whose
   settings already exist and which Control Centre or the system menu
   already link to: VPN (NetworkManager's VPNs), Energy (the power
   profile), Lock Screen (P9's idle timers), Accessibility (the display
   and keyboard settings P5 and P8 added), Wallpaper, Mouse, Apps (keep
   running, Cmd translation, game or not) and General with About alone.
   Twenty-one panes. M2 moves the host onto this desktop after P14, and
   without these the user would hand-edit `settings.kdl` for things v1's
   window set. Left: General's other sub-pages, Users & Groups, Login
   Password, Calendars, Printers and Phone (P20), Trackpad (P16), Game
   Controllers (P18).
2. **A row appears with its feature.** A pane lists the rows whose
   settings something reads today; a row for a later packet's feature
   (session restore, Mission Control's group, widgets, the dropdown
   terminal, Zoom, the on-screen keyboard) is added by that packet, as
   Phone and Trackpad already come and go. An unbuilt pane is absent from
   the sidebar and the index, never a placeholder. General opens with
   About as its one sub-page.
3. **Each pane's rows pass a gate before they are built.** `settings.md`
   placed every setting in a pane and left rows to "that pane's packet".
   Phase 13.1 draws the 21 panes' rows, copy and subtitles on P1's drawing
   kit (`surfaces/scene.js`) as one board, the outline below as its
   starting point, and the user answers lgtm or names changes (the design
   gates workflow). Panes are built after it.

### The app platform

4. **A crate for what every app shares: `isle-app`.** The window and its
   toolbar's drag and double click, the theme from the compositor, single
   instance and activation, the command table with its DBusMenu export,
   Mac editing keys, in-window menus, sheets and popovers, the worker that
   hands results to the UI, and the Wayland extras bound beside winit. The
   form components (groups, row kinds, the controls of `surfaces/settings.md`)
   go into `isle-ui`, which the shell shares. Files, Monitor, Keychain and
   Cleaner then add only themselves.
5. **Names.** The app's id, desktop file id and bus name are
   `io.github.alexparlett.Isle.Settings`; its binary is `isle-settings`;
   its crate is `isle-system-settings`, since `isle-settings` is the store.
   `DBusActivatable` needs a reverse-DNS desktop id and `launch.rs` derives
   the object path from it (§3), so `apps.md` Conventions 1 (D92's
   `isle-settings` as id) changes to the reverse-DNS id with the short
   binary name, for all five apps.
6. **Software rendering by default, settled by the host measure.** The
   fork's software renderer draws a 55 MB PSS window where GL draws 132 MB
   and Vulkan 142 MB, almost all of it NVIDIA's driver mappings (§8); only
   software passes `apps.md` bar 2's 80 MB. Settings builds with
   `freya` `default-features = false` and the `winit` feature, so no GPU
   crate links, and `FREYA_RENDERER` still picks another for a test build.
   Translucency is the compositor's (choice 13), so software loses nothing
   that shows. Phase 13.14 measures a release build of both on the host
   and B26 is revised from that measure; if the GPU build somehow fits,
   the choice reverses there.
7. **One instance through `org.freedesktop.Application`.** The app
   requests its name with do-not-queue. Taking it, it serves
   `org.freedesktop.Application` (Activate, Open with `isle-settings://`
   URIs, ActivateAction `open` with a pane and row), which is what
   `launch.rs` already calls for a `DBusActivatable` entry. Finding it
   taken, it forwards its arguments and the activation token to the
   running instance and exits (§3). The desktop entry carries
   `DBusActivatable=true`, `MimeType=x-scheme-handler/isle-settings;` and
   `Exec=isle-settings %u`; a D-Bus service file starts it.
8. **The Wayland extras through the window's own surface.** winit cannot
   raise an open window with a token (`focus_window` is empty on Wayland)
   and binds neither the appmenu nor the background effect. `isle-app`
   opens a second event queue on winit's display
   (`Backend::from_foreign_display`) and binds `xdg_activation_v1`,
   `org_kde_kwin_appmenu_manager` and `ext_background_effect_manager_v1`
   against the window's `wl_surface` (§2). No winit fork.
9. **Menus: one command table, exported as DBusMenu.** Each app declares
   its commands once (label, chord, menu, enabled-when), strata's shape;
   menu items and keys are built from it and the test that every command
   with a chord is in exactly one menu runs per app (Conventions 10). A
   real DBusMenu exporter replaces `isle-menus`'s test fake, following
   ksni's server, and is tested against `isle-menus`' own client. The app
   sets its menu's address through the appmenu with its unique bus name, as
   Qt does. An item delivers its command to the focused window as the key
   would (Conventions 11). The app draws no menu bar of its own.
10. **The theme comes from the compositor.** New: `Shell1.Theme()` returns
    the token source in force (the shipped file with the user's over it)
    and the appearance values, and `ThemeChanged` fires when either moves,
    including a rejected `tokens.kdl` (with its status). The app builds the
    `Theme` with `isle-tokens`, as the shell does. No app opens
    `tokens.kdl` (Conventions 19); a token change re-renders every open
    window (Conventions 26, bar 8).
11. **Mac keys in the app.** Isle's apps join the no-translation list by
    the prefix `io.github.alexparlett.Isle.*` in `keyboard.untranslated-apps`'
    default, and bind Super as Cmd themselves. `isle-ui`'s field gains the
    Mac editing set (Cmd motions, Option by word, Ctrl's Emacs motions)
    rather than freya-edit's Ctrl defaults (Conventions 12).
12. **Menus, sheets and popovers draw inside the window.** winit has no
    `xdg_popup`, so a pop-up button's menu, a context menu and a popover
    are drawn in the window's own surface, flipped and clamped to fit, and
    sheets are in-window modals with Freya's modal focus boundary
    (Conventions 18). Settings' window is large enough that nothing it
    opens is cut. Popups as `xdg_popup` surfaces are left for the first app
    that needs a menu to leave its window.
13. **ext-background-effect-v1 is served.** The compositor serves it from
    the fork's Smithay module (#1860) and blurs behind a client's region
    with the glass pass the shell already uses; Settings' sidebar asks for
    it. Rule 17 needs it for every app; without it the sidebar is solid
    (as under Reduce Transparency), which is the fallback if the cost is
    too high.
14. **The lights are the compositor's, over the app, as on macOS.** Revised
    by the user on 2026-10-17, replacing apps drawing their own lights
    (B25, `apps.md` Conventions 15). An Isle app keeps server-side
    decorations with a transparent title bar: its content runs to the
    window's top and it names only the lights' inset, as a Mac app does with
    a full-size content view and a traffic-light inset. No standard
    protocol says this (`xdg-decoration` and KDE's only choose a side), so
    Isle serves its own, `isle_titlebar_v1`: `set_overlay(x, y)`, answered
    with the lights' group size. The compositor draws the lights alone at
    that inset, no fill, title or bar height, and takes the pointer over
    them first; Option, the active state and hover behave as on every other
    window. `isle-app` binds it beside the appmenu (choice 8); off Isle the
    app simply wears the compositor's ordinary decorations. The rest of the
    toolbar is the app's: a press on empty toolbar moves the window through
    `xdg_toplevel.move`, and a double, middle or secondary click goes to
    the compositor through `gtk_shell1`'s `titlebar_gesture`, which Isle
    already serves, so `windows.titlebar-action` is applied in one place.

### Settings over D-Bus

15. **The store says everything a pane needs.** `Settings1` gains
    `Reset(key)` (the node removed: its default, or unset for an optional
    key), `Remove(entry)` (a keyed entry deleted: a Focus mode, an app's
    notification rule, a forgotten display), and `Values()` (every key's
    value in force, defaults included). `Set` accepts an empty list, and
    `Get` returns one. Show changed settings is the app comparing
    `Values()` with `Schema()`'s defaults. `isle settings reset`, `isle
    settings open <pane> [row]`; `isle windows game` works on an empty
    list.
16. **The panes are the record of rows; the index is generated from
    them.** Each pane is declared in the app as data (id, name, glyph,
    hue, group, subtitle, rows with id, label, description, keywords and
    what each is bound to: a settings key, a daemon or nothing). Search
    reads it with `isle-search`'s matcher. `isle-settings --write-index`
    writes `data/settings/index.kdl` from it, and a test fails when the
    committed file differs. The schema keeps its developer's doc for the
    CLI; labels and keywords belong to the UI. Pane and row ids are
    kebab-case names (`displays`, `night-light`), and Control Centre's
    links move from pane names to ids.
17. **Opening at a pane is one call.** The compositor's `open_settings(pane,
    row)` activates the app by D-Bus with ActivateAction `open` and an
    activation token; Control Centre's footers and Set up…, the system
    menu's About This Computer and System Settings…, Spotlight's results,
    the `settings` shortcut and `isle settings open` all use it, and
    `isle-settings://` reaches the same action through the scheme handler.
    Software Update… stays disabled until its pane (P20). A request while a
    sheet is open waits for the sheet (`settings.md`).
18. **The app's own state is the app's.** The pane last shown and the
    window's history are written by the app to its own state file,
    `~/.local/state/isle/settings-app.json`, through `isle-state`; it is
    that file's only writer. Settings has no preferences of its own, so no
    `app` section yet; `example_app` stays the schema's test.

### Displays

19. **The compositor owns a display change and its revert.** New:
    `Displays1.Configure(changes, temporary)`. The compositor applies mode,
    refresh, rotation and enabled live, with a modeset rather than at
    Redetect, keeps the configuration it replaced, and for a temporary
    change starts its own `timing-display-revert-ms` (15 s) deadline;
    `Displays1.Keep()` writes the change to `settings.kdl`, while the
    deadline, `Displays1.Revert()` or the asking client leaving the bus
    puts the old one back. Settings draws the keep-or-revert sheet over
    the compositor's countdown. So a crash or a black screen reverts by
    itself, as mutter does and KDE does not (§6). Arrangement, scale, VRR
    and tearing apply at once through `Settings1` with no confirm, as
    `settings.md` has them; the last enabled display cannot be turned off,
    and the compositor refuses it too.
20. **Displays reports what the pane draws.** `Displays1.List` gains each
    output's modes (size and rates), current rotation, enabled, physical
    size, the EDID refresh range and brightness; `Displays1.SetBrightness`
    goes through the compositor's own brightness service, so its OSD and
    cached level stay right. `Displays1.Identify(on)` shows each screen's
    name at its centre while the pointer is over the map, as a shell
    surface in the top layer (added to the warm-up scene).
21. **Scale is a pop-up of steps.** 100% to 200% by 25 with the display's
    default marked, then "Other…" for a typed value, instead of macOS's
    Larger Text to More Space thumbnails, which assume Retina panels; the
    choice is made in 13.1's board.

### The rest of the backends

22. **Settings talks to the daemons through the service crates** (B11,
    Conventions 20), in its own process, with the missing calls added to
    the crates: the network crate's saved networks, forget, auto-join,
    hidden networks and a device's details (addresses, router, DNS, MAC,
    speed); Bluetooth's forget and rename; audio's balance. Settings
    registers no agent: joining a secured network or pairing raises the
    compositor's prompt (B12, B28).
23. **Each pane's missing compositor surface is small and typed.**
    `Notifications1.Apps()` lists every app that has posted, kept in the
    notifications state; `Island1.Modules()` exports Control Centre's
    module catalogue (ids, names, sizes, owning pane) and
    `Shell1.EditControls()` opens the panel's edit mode;
    `control-centre.tray-hidden` is the new list of tray programs kept out
    of the tray. Privacy & Security reads and revokes the permission
    store's `screenshot` and `screencast` tables directly, as `isle portal
    permissions` does.
24. **Keyboard layouts come from xkeyboard-config's own catalogue.** The
    app reads `evdev.xml` for layouts, variants and the options Modifier
    Keys… maps (Caps Lock as Control, Escape or nothing), shows names as
    XKB gives them, and names keys through `isle_keys::chord::key_name`
    (`apps.md` Settings 10). The recorder uses P5's `Keyboard1.Record`,
    which suspends desktop chords and translation, and refuses a chord the
    desktop holds with the holder's label.
25. **Cmd translation exceptions are editable, in Apps.** `apps.md` left it
    open. Each app's page has "Use Mac shortcuts" (its entry in
    `keyboard.untranslated-apps`), beside Keep running after its last
    window closes and Treat as a game (Automatic, Always, Never, writing
    `windows.games` and `windows.not-games`). Keyboard keeps the global
    switch.

### Hot corners

26. **The corner engine is built now, with the actions that exist.** A
    corner fires after the pointer rests in it, once per visit, never with
    a button held, over the lock, while the pointer is confined, or over a
    full-screen game (`mission-control.md`). A new `hot-corners` section
    holds the four corners and their modifiers. The actions offered are
    those built: Notification Centre, Apps, Lock Screen, Put Display to
    Sleep; Mission Control, Application Windows, Desktop and Widgets join
    the list with P16 and P17. The defaults are `dock.md`'s: bottom right
    the Apps view now, bottom left Mission Control once it exists.

### Proof

27. **Tested headless, run in the VM, measured on the host.** Each pane
    runs headless under `freya-testing` against the fake daemons on the
    harness's private system bus and a headless compositor, so a test can
    change a setting in the pane and read it back from the compositor, or
    stop a daemon and see the pane say so (bar 4). The VM runs every pane
    with real NetworkManager, BlueZ and PipeWire. The host run measures
    the app's memory both ways (§7: `smaps_rollup` and `nvidia-smi pmon`),
    its cold start, and a real mode change reverting on the AOC and the
    1080p screen; it is part of done, as P12's was, since D13 puts memory
    on the host.

## The rows

The board's starting point (choice 3), from `settings.md`, macOS 26 and 27
(§1) and the settings that exist. *New* marks a setting P13 adds.

| Pane | Rows |
|---|---|
| Wi-Fi | Wi-Fi switch; the joined network with Details… (Automatically join, IP address, Router, DNS, Wi-Fi address, Forget This Network…); Known Networks with a ⋯ menu (Auto-Join, Forget); Other Networks, Other… (name, security; the password from the agent's prompt) |
| Bluetooth | Bluetooth switch, "discoverable as {name}"; My Devices (Connect or Disconnect; ⋯ Rename, Forget); Nearby Devices (Connect, pairing by the agent) |
| Network | Ethernet, Wi-Fi and VPN rows with status; Ethernet's page: status, IP address, subnet, router, DNS, hardware address, speed |
| VPN | Each NetworkManager VPN with its switch; ⋯ Remove |
| Energy | Energy Mode (Low Power, Balanced, Performance); a line that Game Mode holds Performance |
| General › About | Computer name, the system, Isle's version and commit, processor, graphics, memory, displays |
| Accessibility | Display: Increase contrast, Reduce transparency, Reduce motion, Text size; Keyboard: Sticky Keys, Slow Keys with its delay |
| Appearance | Appearance (Auto, Light, Dark, pictures); Glass (a slider, Clear to Tinted, as macOS 27); Colour (the tokens' accents); Title bars on other apps' windows |
| Apps | Every app; an app's page: Keep running after its last window closes, Use Mac shortcuts, Treat as a game, Notifications… (to its page) |
| Control Centre | Clock Options… (24-hour, weekday, date, seconds); each status glyph: Always show, Show when active, Never show; modules shown with their sizes, Edit Controls…; Allow in the tray, a switch per program (*new* `tray-hidden`); Reset Control Centre… |
| Desktop | Windows: Title-bar double-click action (Fill, Minimise, No Action), Drag windows to screen edges to tile (*new*), Tiled windows have margins; Hot Corners… |
| Displays | The arrangement with the island mark and Follow focus; per display: Use as main, Resolution, Refresh rate, Scale, Rotation, Brightness, Variable refresh rate (Off, Full Screen, Always), Allow tearing in games, Enabled; Night Light… (Schedule, From, To, Warmth, Turn on until…); Detect Displays; Advanced… (the render GPU) |
| Spotlight | Results from System, a switch per source (*new* `sources-off`); Favourites in the Apps view; Clipboard history and Exclude these apps; Search Privacy… (folders in and out); Web search (DuckDuckGo, Google, Bing, Kagi, Custom…) |
| Wallpaper | As `wallpaper.md`: the picture, fit, colour, the dark picture, change picture, random order, per screen |
| Notifications | On the lock screen (Don't show, Show count, Show previews); Hold banners while a window is full screen; Hold banners while sharing your screen; Application Notifications → an app's page: Allow notifications, Banner style, Play sound, Show badges, Show over full-screen windows |
| Sound | Sound Effects: Alert sound (previews), Play sound for notifications, Screenshot sound; Output and Input: devices, Output volume with Mute, Balance; Input volume, Input level (live) |
| Focus | Do Not Disturb, Game Mode, the user's modes, Add Focus…; a mode's page: Allowed apps, Allow time-sensitive notifications, Turn on automatically (Time, App; Game Mode's When a game is in front and Off for these games), Delete Focus |
| Lock Screen | Dim the display after, Lock after, Turn display off after (each with Never), Lock when the computer sleeps |
| Privacy & Security | Screen & System Audio Recording and Screenshots: the apps remembered, each with a switch that revokes |
| Keyboard | Key repeat rate, Delay until repeat; Keyboards (each: Profile, Swap Alt and Super, Model), Detect…; Input Sources Edit…; Modifier Keys…; Num Lock on at start; Use Mac shortcuts in other apps; Keyboard Shortcuts… (the shortcut groups, a checkbox and recorder per action, Restore Defaults) |
| Mouse | Per pointing device: Tracking speed, Pointer acceleration, Natural scrolling, Primary button (Left, Right) |

## The workspace

```
crates/
  isle-app/               what every app shares: window, toolbar, theme,
                          single instance, activation, commands, menus
                          export, sheets and menus in the window, the
                          Wayland extras (new)
  isle-system-settings/   System Settings, binary `isle-settings`: the
                          frame, search, the panes (new)
  isle-ui/                the form: groups, row kinds, pop-up button,
                          segmented, stepper, swatches, pictures, recorder,
                          list with + and −; Mac editing in the field
  isle-menus/             the DBusMenu exporter (from the test fake)
  isle-settings/          Reset, Remove, Values, empty lists; `hot-corners`,
                          `control-centre.tray-hidden`, `spotlight.sources-off`,
                          `windows.edge-tiling`; the untranslated prefix
  isle-network/, isle-bluetooth/, isle-audio/
                          saved networks, forget, details; forget, rename;
                          balance
  isle-ipc/, isle-cli/    the new calls; `isle settings reset|open`,
                          `isle displays set`
  isle-compositor/src/
    titlebar_overlay.rs   `isle_titlebar_v1`: the lights over a client
                          at its inset (new)
    dbus.rs               Settings1, Displays1, Shell1, Island1,
                          Notifications1 additions
    displays.rs           live modesets, the temporary change and its revert
                          (new)
    hot_corners.rs        the corner engine (new)
    background_effect.rs  ext-background-effect-v1 and the blur behind
                          (new)
    settings_link.rs      `open_settings` and its callers (new)
  isle-shell/src/         the screens' names while arranging
data/
  apps/                   the desktop entry and D-Bus service file (new)
  settings/index.kdl      generated
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 13.0 · The store and the theme over D-Bus

`Settings1` Reset, Remove, Values; empty lists through Set and Get; `isle
settings reset`, `isle windows game` on an empty list. `Shell1.Theme` and
`ThemeChanged`. The no-translation prefix. Tests in `isle-settings` and the
compositor's D-Bus tests.

### 13.1 · The rows (gate)

The 21 panes' rows, copy and subtitles drawn on P1's kit as one board,
published as a private Artifact, with the scale pop-up, Spotlight's sources
and the new settings among the choices. The user answers lgtm or names
changes; `settings.md` gains each pane's rows. Runs beside 13.0 and 13.2.

### 13.2 · `isle-app`

The window on `freya-winit` with the software renderer and a transparent
title bar; `isle_titlebar_v1` served, with the lights drawn over the app at
its inset; drag, and clicks handed over by `gtk_shell1`, on empty toolbar; the theme from `Shell1`; single instance with
`org.freedesktop.Application`; the second event queue with activation,
appmenu and the background effect's client half; the command table, its
menus and the DBusMenu exporter; Mac editing in the field; in-window menus,
popovers and sheets; the worker. `freya-testing` and the `accessibility`
feature at the pinned revision. The form components in `isle-ui`.

### 13.3 · Settings' frame

The sidebar with the account row (AccountsService) and the pane list, back
and forward (Cmd+[, Cmd+], the mouse's buttons), the pane header and the
crossfade, search with results and the found row's ring, the rejected-edit
notice for both files, Show changed settings and Reset to default, opening
at a pane and row, the last pane kept. The generated index and its test;
the desktop entry, service file and scheme handler; `open_settings` and
every caller; `isle settings open`.

### 13.4 · Displays

`Displays1` List, Configure, Keep, Revert, SetBrightness and Identify; live
modesets; the compositor's deadline and the revert when the caller goes.
The pane: the arrangement with snapping and the island mark, the rows, the
keep-or-revert sheet, Night Light…, Detect Displays, Advanced….

### 13.5 · Sound

Devices, volumes, balance, the input meter while shown, alert sounds with
preview.

### 13.6 · Wi-Fi, Network, VPN and Bluetooth

The crates' additions and the four panes; joining and pairing through the
compositor's prompts; a missing adapter or daemon as the pane's state.

### 13.7 · Keyboard

Repeat, keyboards with Detect, Input Sources from `evdev.xml`, Modifier
Keys, Keyboard Shortcuts with the recorder and Restore Defaults.

### 13.8 · Appearance, Accessibility, Wallpaper and Mouse

The pictures, glass and accent; the display and keyboard accessibility
rows; Wallpaper's rows; each pointing device's rows.

### 13.9 · Desktop and hot corners

The Windows rows with `windows.edge-tiling` read by the tiling zones; the
corner engine, the `hot-corners` section and the Hot Corners… sheet.

### 13.10 · Notifications, Focus and Control Centre

`Notifications1.Apps`, `Island1.Modules`, `Shell1.EditControls`,
`tray-hidden`; the three panes, Focus's modes added and deleted.

### 13.11 · Spotlight, Privacy & Security and Apps

`spotlight.sources-off` read by Spotlight; the three panes; game marking
and Cmd translation per app.

### 13.12 · Lock Screen, Energy and General › About

The three panes.

### 13.13 · The background effect

`ext-background-effect-v1` served with the blur behind a client's region;
Settings' sidebar on it, solid under Reduce Transparency.

### 13.14 · The measure, the VM and the host

Headless: bars 3, 4, 5, 6, 7, 8 for Settings. The VM: every pane against
the real daemons. The host, the user's run: PSS and VRAM for the software
and GPU builds after 30 s idle (`settings-2026.md` §7), cold start, a real
mode change on each screen reverting and kept, Cmd chords on both
keyboards. Results in `docs/research/rust-desktop/p13-settings.md`.

### 13.15 · Close

ARCHITECTURE gains How an app is built and How Settings writes; B26
revised and P13's decisions recorded; `apps.md` Conventions 1 and its open
points answered; `settings.md` holds the rows; the roadmap's P13 and later
entries follow; this sheet ends with its outcome.

## Not in P13

General's Software Update, Storage, Passwords, Date & Time, Language &
Region, Login Items and Snapshots; Users & Groups, Login Password,
Calendars, Printers, Phone, the provider pages (P17, P20). Trackpad,
Mission Control's rows and the hot corners' Mission Control actions (P16).
Game Controllers (P18). Fonts, which theming installed apps settles (P14).
Default apps (P15). TCP/IP and DNS editing, VPN import (P20). Popups as
`xdg_popup`. Session restore and Close windows when quitting (P20).

## Depends on, and feeds

- Needs P2 (the settings store and its D-Bus), P3 and P6 (`isle-ui`, the
  menu registrar and appmenu), P5 (keyboards, the recording grab), P8 (the
  service crates, Control Centre, Focus, notifications), P9 (idle, the
  agents), P10 (Spotlight's settings source), P11 (the permission store),
  P12 (VRR, tearing, night light, the game lists).
- Files, Monitor, Keychain and Cleaner build on `isle-app`. P14 packages
  the desktop entry and service file and imports v1's preferences into the
  keys these panes show. P16 adds Mission Control's actions to the corners
  and Trackpad; P17 widgets' rows and Providers; P20 the remaining panes.

## Done when

- `cargo test` passes headless: the store's new calls, the display change
  and its revert (including the caller leaving), the corner engine, the
  index matching the panes, the command table, every pane against the
  fakes and a headless compositor.
- In the VM, every pane changes what it says through the real daemons and
  the compositor, and a value changed in Control Centre or by hand shows in
  the open window within a second.
- On the host, the app's memory is measured both ways and B26 is settled
  from it; a mode change reverts by itself and is kept on Keep, on both
  screens.

## Risks

- **A live modeset on NVIDIA.** Changing a mode or enabling a screen
  without Redetect may blank or fail on this driver. The compositor's
  revert is the guard; a refused modeset leaves the old mode and says so
  on the row. If live modesets prove unsafe, Configure falls back to the
  Redetect path behind the same deadline.
- **Clipped menus.** A pop-up near the window's edge is clamped inside it
  (choice 12). Settings is large enough; a narrow app later may need real
  popups.
- **Wallpaper's Choose….** The Open panel is Files' and comes with P15. Until
  then the FileChooser portal falls back to the GTK backend where it is
  installed, through `isle-portals.conf`; without it, the folder row takes
  a typed path.
- **The software renderer's frame cost.** It draws a whole frame per
  present. At Settings' size that is a few milliseconds of CPU per frame of
  a crossfade; the measure watches it, and the GPU build is the fallback.
- **Scope.** Twenty-one panes and a platform. The platform and the frame
  (13.0–13.4) are the packet's spine and land first; panes after 13.4 are
  independent and can be cut to P20 one at a time if the packet runs long,
  without changing a choice.
- **The DBusMenu exporter.** Isle's own client is the only one it is
  tested against; the menu bar is the only consumer, so that is enough.

## Reviewed

The user reviewed the plan on 2026-10-17 and took every recommendation:
21 panes, rows gated on a board in 13.1, `isle-app` for every app, the
reverse-DNS ids, software rendering pending the host measure, the
compositor owning a display change's revert, the hot-corner engine now, and
the host run part of done.
