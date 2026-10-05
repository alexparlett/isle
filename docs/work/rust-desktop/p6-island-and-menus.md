# P6 · Island and menus

Planned 2026-10-05. Reviewed by the user the same day: every
recommendation taken, P6 kept as one packet.

The roadmap's brief: the island on every screen (B18): the clock, the
status glyphs, live activity as P1 designs it (B24), and Control Centre's
panel opening from it, whose modules and tray fill in with P8. The menu bar
on demand from the top edge and Ctrl+F2, with the three menu sources and the
registrar owned from session start, the fallback app menu and menu search.
The wallpaper.

P5 left a desktop with no picture behind it (the backends clear to a grey),
nothing in the strip P4 keeps clear at the top of each screen, and four
reserved chords that only log: the window menu, the menu bar, Help's search
and Control Centre. `isle-ui` holds the theme and glass and no components;
the Phosphor glyphs in `data/icons/` are read by nothing; no surface yet
runs on every output. P6 makes the screen at rest what B18 promises, the
wallpaper and the island, and puts every command of the front app and of
the system one push of the top edge away. What it must get right is the
top edge: one pointer gesture, one key, or one game in front must never
open two things, hide the island's privacy marks, or leave an app without
a menu.

## What it answers to

- Decisions: B17, B18 (the island at rest, the menu bar on demand, the three
  menu sources owned from session start), B19 (attention moves to the
  island), B20 (the bar speaks in apps), B21 (hints show the Mac chord;
  desktop chords before translation), B24, B30 (the escape set over a game;
  Game Mode solid and still), B32–B34 (attention, glass, the principles'
  readings), B35 (the loop, the worker, Test1), B37 (surfaces), B39 (glass is
  the compositor's), B40 (the registry), B44–B46 (the seat's keys, the
  tables as data, the game flag).
- Intent: `shell.md` Shell runtime 10 and 14 (left by P3), Menu bar 1–10,
  App menus 1–9 and their open points, Island 1–12, Control Centre 1, 3 and 4
  as the frame, OSDs as the island's forms, Wallpaper 1–2; acceptance bars 2,
  3, 5, 8, 11, 13, 14. `platform.md` Tray and global menu registrar 1–6 and 8
  (the tray, 7, is P8's) and its open points; App index and launching where
  P6 needs to start an app or open a link. `compositor.md` Shell surface
  hosting 1–3, Wire protocols (`org_kde_kwin_appmenu`, `gtk_shell1`),
  Xwayland 10, Pointer 5 (the top-edge push and dwell), Minimise 1 (toward
  the island), Server-side decorations' open point on `gtk_shell1`.
- Left to P6 by earlier packets: the window menu with keyboard move and
  resize and `show_window_menu` (P4), the island's rectangle as minimise's
  target and the reveal of title bars over full screen (P4), the global
  menu's X11 properties (P4), the inhibit glyph, notice row and the
  window's inhibit notice and hints through the inverse tables (P5).
- Design, binding: `p1/structure/island.md`, `menu-bar.md`, `wallpaper.md`,
  `competition.md` (the top edge and keys), `control-centre.md` (the grid,
  its packing, the page model); `p1/surfaces/island.md`, `menus.md`,
  `wallpaper.md`, `window-chrome.md` (the window menu, the inhibit notice,
  the bar beneath over full screen); `review/accessibility.md` and
  `copy.md`; DESIGN.md Components, Motion, Materials. Tokens are already in
  `data/tokens/tokens.kdl`.
- Evidence: `macos-model.md` §1 (the menu sources per toolkit, what the
  compositor must implement, the lookup order, existing implementations),
  `design-v1-lessons.md` §3.5 and §5 (hover guards, the island's morph),
  `isle-v1-inventory.md` §4 (the pill, the hot zone, wallpaper),
  `docs/research/rust-desktop/global-menu-2026.md` (this plan's research:
  the protocols' state today, crates, Electron and GTK 4).

## Choices this plan makes

1. **The wallpaper is the compositor's, not a tree.** Each output draws a
   background render element: a texture for a picture or a solid colour,
   and Frost's drawn backdrop as a GLES shader from the tokens (base,
   overhead light, falloff, grain), which needs no decode and can be the
   first frame. It is not a Freya surface: it has no text, takes no focus
   and changes a few times a day, so a tree would only cost a texture and a
   context. A click on it clears keyboard focus, so the front app becomes
   the no-window case (choice 13). Over a full-screen window it adds no
   element (acceptance bar 13).
2. **Pictures are decoded on the worker.** `wallpaper` and
   `wallpaper-output "identity"` sections: `source` (`frost`, a file, a
   folder or `#rrggbb`), `dark-source` for the dark variant, `fit` (`fill`,
   `fit`, `centre`), `colour`, `change` (`never`, `hourly`, `daily`,
   `login`) and `shuffle`. The worker decodes with the `image` crate (JPEG,
   PNG, WebP; AVIF and JPEG XL if the crate's features build on Arch
   without trouble, else left out and refused with a message), scales to
   each output's pixel size with its fit, computes the mean colour in
   linear light, and hands the loop the pixels to upload. A `wallpaper`
   state domain keeps each output's mean colour and the folder's place.
   The change is a 600 ms crossfade between two textures, a cut in Game
   Mode or when covered. A missing or broken file keeps the last good
   picture, else Frost, and is reported on the bus for Settings (P13).
3. **Not in P6 for the wallpaper:** the softened form (its readers are the
   lock, P9, and the dashboard, P17, which build it on P3's blur when they
   need it), the bundled pictures (an art task with its own sources and
   licences, tracked for before P14; until then Frost is the only shipped
   choice), the wallpaper's secondary-click menu (both its items open
   later packets' surfaces; it comes with Settings in P13).
4. **The island is one surface per output whose shape the compositor
   owns.** `island` is a `Top`, `EveryOutput` surface. Its shape, the
   rounded rectangle with its fill, shadow and blur, is drawn by the
   compositor, which springs its width and height (B7) with the radius
   following the height (half, capped at `radius-panel`), anchored at the
   top centre; the tree draws content only, on a clear ground, clipped to
   the shape. Each spring publishes its progress p to the tree every frame
   through a context value, and the tree lays out the outgoing and incoming
   forms with their opacities by p (1 − 2p, 2p − 1), so the pill's row,
   present in both, neither fades nor moves (`surfaces/island.md` Motion).
   The fill blends from `colour-island` to `material-glass` and the shadow
   from `shadow-menu` to `shadow-panel` by the same p. The texture is sized
   to the larger of the two forms while a spring runs. A snapshot crossfade
   was rejected: the shared row would vanish at p = ½.
5. **One model of what the island shows, pure and tested.** A module
   `island` in `isle-shell` holds the queue as `structure/island.md` sets
   it: glyph states with their tiers, privacy marks, live activities with
   their ranks and one per kind, events with their forms, durations,
   replacement, pre-emption by OSDs, waiting while a panel is open and
   dropping past their time, suppression over full screen, a game and the
   lock. It takes a clock from its caller and returns what each output's
   island should show and when it next changes, so it is unit-tested with
   a fake clock and the loop arms one calloop timer for the next change.
   Sources post to it on the loop. Built from real sources in P6: the
   clock, shortcuts inhibited, the configuration error and the safe start,
   the keyboard layout OSD (P5's layout chord), attention (the registry's
   flag from xdg-activation and X11 urgency), the Focus glyph's Gaming
   Focus stand-in (choice 22). The rest (Wi-Fi, Bluetooth, sound, VPN,
   updates, battery, the privacy marks, recording, sharing, calls, timers,
   media, prompts, every level OSD, copied) are posted by their packets
   (P8, P9, P11, P17) and by `Test1.Island` in tests and the preview.
6. **The clock wakes once a minute.** The `jiff` crate, which reads the
   system's time-zone database, formats the time; the loop arms a timer to
   the next minute's boundary (a second's with seconds on) and nothing
   else ticks (acceptance bar 2). The worker watches `timedate1`'s
   `Timezone`, a clock change (a `timerfd` with cancel-on-set) and
   logind's `PrepareForSleep`, and any of them re-reads the zone and
   re-arms. An `island` section holds `clock-24-hour` (`auto` from the
   locale's time format, which is 24-hour here), `clock-weekday`,
   `clock-date`, `clock-seconds` and each glyph's tier (`always`,
   `when-active`, `never`). The clock's box is the format's widest string,
   measured once per format and text scale.
7. **The top edge is decided in one place.** A `top_edge` module in the
   compositor's pointer path owns the island's zone (its shape, the strip
   above it to the edge, `size-island-guard` each side, never over a
   revealed bar's titles), the push (upward relative motion accumulated
   while the pointer sits on the top row, past `size-edge-push`, reset when
   it leaves the row), the dwell over a game (`timing-edge-dwell-ms`), and
   the guards: nothing during a move or resize grab, with a button held,
   with the pointer locked or confined, or over the lock; and a peek needs
   the pointer to move after a reveal, so one gesture never opens both.
   The island's hover guards (dwell, grace, settle, disarm after a click
   that opens elsewhere, no hover while a menu is open or a window is
   dragged) are timers here, not in the tree. `Test1` gains relative
   pointer motion to test the push, which the absolute injector cannot
   express.
8. **One top-layer panel per screen.** The shell gains exclusive groups:
   Control Centre, Notification Centre (P17) and the bar's menus are one
   group per output, and opening one closes the other; Spotlight, Mission
   Control, the capture toolbar, the session dialog and Force Quit are the
   other group (shell runtime 10) and close the first. `Shell` closes a
   group's open member before opening the next, and the island's queue is
   told when its panel opens or closes.
9. **The main screen follows focus.** Events, faces and the start row show
   on the main screen only (shell runtime 14, D49): the output of the
   focused window, else the output under the pointer. A `shell.main-screen`
   setting can pin it to an output by identity instead. Every output's
   island has the clock, its glyphs and the privacy marks; a monitor's
   brightness OSD shows on that monitor.
10. **Control Centre is built as its frame and two modules.** The panel
    unfolds from the pill by choice 4's morph: the pill's row, Now, the
    grid, the footer; peek on hover without focus, pinned by a click,
    Return or Ctrl+F8 with focus, closed by Escape, a click outside, Ctrl+F8
    again or another panel. Now carries the two notice rows with real
    sources (configuration error with Open in editor, Show in Files, Use my
    settings; shortcuts inhibited with Take keys back) and every activity
    row in its forms, fed by `Test1` until their packets. The grid packs
    the arrangement in reading order as `control-centre.md` sets it, from a
    `control-centre` section's `modules` list (ids and sizes), absent
    modules closing up; module frames in the four sizes; the page and
    sub-page frame with back, and keyboard navigation through all of it.
    Two modules prove the frames with sources that exist now: **Dark Mode**
    (Small, the `appearance.mode` switch) and **Keyboard layout** (Medium,
    with its page of the typing keyboard's layouts, from P5's `Keyboard1`).
    Until P8 they follow the design's default arrangement so the panel is
    not empty; P8 returns the default to the design's. The tray, edit mode
    and the gallery, and every other module, are P8's; "Edit controls" and
    "Control Centre Settings…" are disabled until P8 and P13.
11. **The clock opens Control Centre until Notification Centre exists.**
    `structure/island.md` has the clock open Notification Centre, which is
    P17's. Until then a click on the clock pins Control Centre, as a click
    elsewhere on the pill does, so the clock is never dead. P17 changes it.
12. **A component set in `isle-ui`, as P6 needs it.** The menu (every row
    kind of `menus.md`, the mark and shortcut columns with macOS's symbol
    order and aligned keys or PC words by the typing keyboard's profile,
    submenus with a dwell and the diagonal aim kept, scroll arrows,
    type-to-select, the chosen row's blink, keyboard), buttons (plain,
    default, small, icon), the search field, a level bar, the module tile
    in its four frames, rows and cards, and `Glyph`: Phosphor from
    `data/icons/phosphor` as SVG, the two-layer level glyphs, Fill and
    Regular weights, plus the Isle-drawn glyphs `menus.md` and
    `island.md` name (`isle-mark`, `isle-keyboard-slash`, `isle-vpn`,
    `isle-equaliser`) added to the set. The zoom menu's private row moves
    onto the menu. `isle-preview` gains a page per component, so the
    later packets' surfaces start from a set already seen. The rest of
    DESIGN.md's components (switches, sliders, steppers, tabs, sheets) come
    with the packet that first needs them.
13. **The bar is one surface per output, menus inside it.** `menu-bar` is a
    `Top` surface on each output, opened on one at a time, its tree
    covering the output so the band, every open menu, Help's search and a
    found item's open path are laid out together. It reports its hit shapes
    as it reports glass shapes, and a pointer outside them goes to what is
    below; with a menu open a click outside closes it and is not delivered,
    as on macOS. The island stays above it (it is registered after it in
    the same layer). Its texture costs the output's size while it is open,
    which 6.6 measures against the budget; separate surfaces per menu were
    rejected because the found-item path and menus switching across titles
    need one layout. The front app is the registry's `front_app()`; with no
    window focused it is Files, named from its entry and with its fallback
    menus, not started until an item is chosen.
14. **Menus are one tree from four sources, in a crate.** `isle-menus`, with
    no Smithay state, holds the menu tree (items with id, label with
    mnemonics stripped, enabled, visible, toggle and state, shortcut,
    children, and where to send an activation), and on the worker: the
    registrar, a DBusMenu client and a GMenuModel client (`org.gtk.Menus`
    and `org.gtk.Actions`, written for Isle) producing that tree, the
    fallback builder and the search index. The loop holds snapshots per
    window. Calls into apps are asynchronous with a 2 s limit. Before a
    submenu opens Isle sends both `AboutToShow` and `Event "opened"`, and
    `"closed"` after, then fetches again whatever the answer, since Qt,
    Chromium and Firefox each fill on a different one; a GMenu item's
    `submenu-action` is set around its submenu the same way. A menu with no
    answer opens with what it had. LayoutUpdated and `Changed` bursts are
    batched for 100 ms, and a LayoutUpdated for a submenu replaces that
    submenu only. Items are found again by their label path, never kept by
    id, because Electron renumbers its tree on every change. Defaults left
    out of a DBusMenu item are filled in; `_` mnemonics are stripped (`__`
    is an underscore); `toggle-state` −1 is mixed; consecutive radio items
    are a group. GMenu actions resolve `app.`, `win.` and appmenu-gtk-module's
    `unity.` prefixes to their paths. `ItemActivationRequested` (an app's
    Alt and mnemonic) reveals the bar with that menu open. The DBusMenu
    proxy is Isle's own: `system-tray`'s replaces the whole tree on a
    submenu's update and stops watching after its first timeout, so only
    its property parsing is a reference (`global-menu-2026.md`). A tree is
    fetched when its window first takes focus, so it is cached by the time
    the bar is revealed (acceptance bar 8's 100 ms).
15. **The registrar is owned before the first client.** The worker owns
    `com.canonical.AppMenu.Registrar` on its own session connection before
    the compositor opens its socket, as P5 learnt to order the GameMode
    watch; the supervisor holds back the ready pipe until it does, so Qt,
    which decides once per process, and Chromium, LibreOffice and Firefox,
    which export only while the name has an owner, always see it. The name
    is requested without queueing or allowing replacement, so whoever owns
    it first keeps it; Plasma's kded registrar asks the same way and only
    when its applet runs, and vala-panel's is activatable but not
    installed. If the name is taken, Isle logs the owner, reports it on the
    bus and in the configuration error's notice row, shows the fallback,
    and asks again when the owner leaves. It records `(window id, sender, path)`,
    drops a sender's entries when it leaves the bus, answers
    `GetMenuForWindow` and `GetMenus`, and takes the name again after a
    restart.
16. **The Wayland and X11 halves on the loop.** The compositor serves
    `org_kde_kwin_appmenu_manager` v2 from `wayland-protocols-plasma`'s
    server bindings, storing `(service, path)` on the toplevel, and
    `gtk_shell1` (choice 17), storing the GTK surface's bus name and its
    application, window and menubar paths. Isle's window manager reads
    `_KDE_NET_WM_APPMENU_SERVICE_NAME` and `_OBJECT_PATH` (which Qt sets
    itself on X11), the `_GTK_*` menu properties and `_UNITY_OBJECT_PATH`,
    with their changes, through a patch on the Smithay fork, as
    `STEAM_GAME` was read, and maps `RegisterWindow` ids to X11 windows.
    `xdg_dbus_annotation_v1` is not served: it is still a merge request and
    no exporter uses it for menus yet; the lookup has a place for it.
    A window's menu is found in `macos-model.md` §1.3's order: its
    surface's own address; for X11, the registrar's entry by XID, then its
    properties; then a registration or `/MenuBar/<n>` from a connection
    whose PID is the window's client's; else the fallback. The app id never
    selects a menu.
17. **`gtk_shell1` at version 5, advertising the menu bar only once read.**
    Bindings are generated from GTK's protocol file with
    `wayland-scanner`'s macros in the compositor. `global_menu_bar` is
    advertised from the build where the GMenuModel client passes its tests,
    since GTK hides its own menubar on the capability and a failed read
    would leave the app with no menu. v5's `titlebar_gesture` is handled:
    a double click on a GTK client's header bar does the title-bar action
    setting, a right click opens the window menu, a middle click lowers
    the window, so libadwaita apps get the window menu from their own bars.
    Binding v5 and ignoring the gesture would break double-click to zoom in
    every GTK 4 app, the trap a Smithay compositor serving v7 fell into; the
    alternative, v4, gives up the window menu on header bars. v6 and v7 add
    nothing P6 uses.
18. **The fallback is built from what Isle knows, and every item works.**
    The app menu from the desktop entry (About, Settings…, the entry's
    actions, Hide, Hide Others, Show All, Quit); File, Edit and View only
    from labelled rules in the translation tables, so `translation.kdl`
    gains an optional `menu "Edit" "Find…"` on a rule and the shipped
    tables are given labels for the groups that have commands worth a menu
    (browsers, terminals, VS Code, JetBrains, file managers); Window; Help
    with search and the AppStream help and homepage links read from
    `/usr/share/metainfo`. A game's fallback is the app menu, Window and
    Help, with no hints but the escape set's. An app's real tree is merged
    with Isle's own parts: the app menu points at the app's own About,
    Settings and Quit by label and chord and never removes them from where
    the app put them; the app's Window items join Isle's Window menu;
    Help gets the search field first.
19. **Hints are the chord the user presses.** A DBusMenu shortcut, in
    Chromium's keysym names (`[["Control","c"]]`) or Qt's portable text
    (`C`, `Del`, `PgDown`), or a GMenu `accel` (`<Primary>c`), is turned
    into a stroke and mapped back through P5's inverse (`Tables::mac_chord`) for
    that app; one with no Mac chord is shown as sent, and over a game only
    the escape set's hints show. On a PC profile the words are used
    (`copy.md` rule 11). An item is activated by its source's call, never by
    typing its chord into the app, so an item works whatever the tables
    say.
20. **Search crawls when it opens.** Help's search reads the front app's
    tree as cached; when the search field opens, Isle fills the submenus
    not yet fetched, enabled ones only, to a depth of four and at most fifty
    menus, as Unity's HUD did, and closes each it opened. A crawl runs the
    app's own code (Firefox's handlers, LibreOffice building menus), so it
    never runs in the background. A chosen result is found again by its
    label path before it is activated. The same index, with its paths and
    hints, is on the bus for Spotlight (P10).
21. **The system menu has its final shape now.** About This Computer,
    System Settings…, Software Update…, Force Quit…, Restart…, Shut Down…,
    Lock Screen and Log Out… open surfaces later packets build (P13, P8's
    updates, P9's prompts, lock and session dialog); until then they are
    shown disabled, so the menu does not change shape under the user, and
    each is enabled by the packet that builds its target. Sleep works now
    (logind `Suspend`). The restart-owed line waits for its source.
22. **Game Mode is one switch in the shell's model.** Over a game nothing
    transitions, glass is solid, the bar is revealed by a dwell and only
    privacy and OSDs show over a full-screen one. The shell reads one flag,
    on while the focused window is a game (P5's flag), which P12 replaces
    with Game Mode proper; the Focus glyph shows `game-controller` while it
    is on, standing in for the Gaming Focus until P8's Focus service.
23. **The window menu, on the menu component.** The title bar's window
    menu (groups 1 and 2 of the Window menu plus Move, Resize and Close) and
    the bar's Window menu are one builder over P4's actions. Opened by a
    secondary click on Isle's bar, a client's `show_window_menu`, an X11
    client's `_GTK_SHOW_WINDOW_MENU`, `gtk_shell1`'s gesture, and the
    `window-menu` shortcut (no default chord). Keyboard Move and Resize
    take the arrows until Return or Escape, as P4 deferred. **Keep on top**
    is built here, as a stacking flag on the record, because the Window
    menu shows it; its chord stays P20's. Full screen tile (Split View) is
    shown disabled until P16.
24. **A launch path, small and shared.** The app menu's entry actions,
    Help's links, Open in editor and the fallback's New Window must start
    things, and nothing in Isle can yet. `isle-apps` gains `launch(entry,
    action, uris)`: the entry's `Exec` with field codes expanded, in a
    transient systemd user scope named by the xdg convention
    (`app-isle-<id>-<random>.scope`), with an xdg-activation token in
    `XDG_ACTIVATION_TOKEN` so the new window takes focus; links go to the
    default handler from `mimeapps.list` the same way. It runs on the worker
    through systemd's D-Bus API. P10 builds Spotlight on it. Icon lookup by
    the freedesktop theme spec (Papirus first, then hicolor) is added for
    the faces, the attention event, Isle's About panel and P7's cards.
25. **Isle's About panel** for an app whose tree has no About: a small
    placed shell surface centred on the main screen, on `material-solid`
    over the window, with the app's icon, name, version and package
    (`pacman -Qo` on the worker, cached per entry) and the entry's
    comment; Escape or its close button closes it.
26. **The inhibit notice in the window.** The window's brief notice from
    `window-chrome.md` (glass capsule inside the top edge, once per gain)
    is an attached surface like the title bar, shown when a window gains
    an inhibitor; the island's glyph and notice row read the same state,
    which P5 kept only on the bus and P6 puts in the shell's model.
27. **The island is minimise's target.** P4's minimise shrinks toward the
    strip's top centre; it now aims at the island's current rectangle on
    that output.
28. **Not served or not built:** menus for libadwaita apps with no menubar
    (no export; the fallback covers them, and building menus from
    `org.gtk.Actions` alone or AT-SPI is left as research); appmenu-gtk-module
    per launch (left until launching from Spotlight in P10, with the
    exclusion list `macos-model.md` asks for); Firefox's two prefs (the
    user sets them; not offered); Spotlight's menu-item source (P10 reads
    the index P6 builds); hot corners (P16).

## The workspace

```
crates/
  isle-menus/             the menu tree, registrar, DBusMenu and GMenuModel
                          clients, fallback builder, search index (new)
  isle-ui/                menu, buttons, search field, level bar, tile, rows,
                          glyphs
  isle-apps/              launch, icon lookup, AppStream links, package
  isle-shell/src/
    island/               the queue model, the pill and its forms, Control
                          Centre's frame, its modules and pages
    menubar/              the bar, its menus, Help's search
    about.rs              Isle's About panel
  isle-compositor/src/
    wallpaper/            the backdrop shader, textures, crossfade
    top_edge.rs           the island's zone, the push, dwells, hover guards
    menus/                org_kde_kwin_appmenu, gtk_shell1, X11 lookup,
                          the window-to-menu map
    wm/window_menu.rs     the window menu's builder, keyboard move and resize
  isle-settings/          wallpaper, island, control-centre, shell sections
  isle-ipc/               Menus1, Island1 (new); Test1's island and pointer
                          additions
  isle-cli/               isle wallpaper …, isle menus …, isle island …
```

`Test1` gains `Island(json)` to post glyph states, activities, events and
privacy marks, `PointerRelative(dx, dy)`, and a clock override for the
island's model. The test client gains `--appmenu` (exports a DBusMenu tree
from a JSON file over zbus and announces it through
`org_kde_kwin_appmenu`), `--gtk-menu` (exports `org.gtk.Menus` and
`org.gtk.Actions` and announces them through `gtk_shell1`) and
`--register` (calls `RegisterWindow` with a given id, for the X11 and PID
paths); the X11 test client gains the KDE and GTK menu properties. Every
activation the test clients receive is printed.

## Phases

Each phase ends in a check run by script or in the VM and lands as its own
commits.

### 6.0 · Wallpaper

- The background element per output: Frost's shader from the tokens, solid
  colour, textures; the backends' grey clear goes.
- `wallpaper` sections, decode and fit on the worker, the mean colour and
  folder place in state, light and dark variants with the appearance, the
  crossfade and its cut, nothing under full screen, the click clearing
  focus. `isle wallpaper set|status`.

**Check:** headless screenshots: Frost dark and light against the board's
crop within a tolerance; a 16:9 picture filling a 21:9 output crops top and
bottom, Fit draws mean-colour bars; a change crossfades over 600 ms (frame
counts) and cuts with a game focused; a missing file keeps the last
picture; a full-screen window leaves no background element; idle frames
stay zero with a still wallpaper.

### 6.1 · Components and glyphs

- `Glyph` with the Phosphor set and the Isle-drawn glyphs; the menu
  component; buttons, search field, level bar, tile, rows and cards; the
  zoom menu moved onto the menu; preview pages for each.
- `isle-apps`: icon lookup, launch in a scope with a token, AppStream
  links, the package query.

**Check:** `isle-preview` pages by eye against `p1/surfaces-a.html`; unit
tests of the menu's keyboard (skip disabled, type-to-select, submenu by
Right and Left, scroll); a headless test launching a test entry's action
into a scope and the window taking focus by its token.

### 6.2 · The island

- The `island` model with its unit tests (every rule of The queue); the
  surface on every output; the compositor's shape and spring with p fed to
  the tree; the clock and its re-arming; glyphs with tiers; faces, marks,
  the count and overflow; events in their three forms; the privacy form
  over full screen; the main screen; minimise toward the island.
- Real sources: clock, inhibited, configuration error and safe start,
  layout OSD, attention, the game flag. `Test1.Island` for the rest.
  `isle island status`.

**Check:** unit tests of the queue with a fake clock; headless: an idle
desktop with the island draws one frame a minute (traced over five
minutes), with seconds one a second; a posted OSD pre-empts an event and
restarts on repeat; a privacy mark shows on both outputs and stays over a
full-screen window, which otherwise hides the island and adds no element;
an event posted while the panel is open waits; a second output's island
has the clock and no event; screenshots of rest, glyphs, faces, events and
the privacy form against the board.

### 6.3 · Control Centre and the top edge

- `top_edge`: the zone, the push, the dwell over a game, the guards and the
  hover timers; peek and pin, Ctrl+F8, Escape's exception, focus return.
- The panel's frame, Now's rows, the grid's packing, the four module
  frames, pages and back; Dark Mode and Keyboard layout; exclusive groups;
  the configuration error's actions; the inhibit notice in the window.

**Check:** headless: a pointer resting on the island peeks after the dwell
and keeps typing in the window; a click pins with focus and Escape returns
it; a push at the edge inside the zone over a full-screen window brings the
island back and a peek needs movement after it; the grid packs a list with
absent modules with no holes (unit); Dark Mode switches `appearance.mode`
and every surface follows; Keyboard layout's page lists both layouts and
switches; a window gaining an inhibitor shows its notice and the glyph,
and Take keys back gives the keys back.

### 6.4 · The menu bar and the fallback

- The bar surface, its hit shapes, the reveal by push, dwell, Ctrl+F2 and
  Shift+Cmd+/, the leave and its grace, the slide and Game Mode's cut, the
  title highlight, menus switching across titles, overflow into », the
  island's reserve, the second screen.
- The system menu (disabled items until their packets, Sleep live), the app
  menu, the Window menu and the title bar's window menu with keyboard move
  and resize and keep on top, Help with AppStream links, Files when no
  window has focus, the fallback's File, Edit and View from labelled
  rules, Isle's About panel.

**Check:** headless: every built action in the system, app and Window menus
does what it says on a test client; Ctrl+F2 opens the system menu with
focus and Escape returns it to the window before anything is delivered; a
push outside the zone reveals the bar and inside it does not; no reveal
during a drag or with a button held; opening Control Centre closes an
open menu and the bar; a narrow output folds menus into » and
Help never folds; the fallback for an app in the browser group carries New
Tab with ⌘T; screenshots against the board's scenes 1–7, 9, 10 and 13.

### 6.5 · Menu sources

- `isle-menus`: the registrar owned before the socket opens, the DBusMenu
  client, the GMenuModel client, the tree, batching and timeouts, the
  merge with Isle's parts, hints through the inverse.
- The compositor: `org_kde_kwin_appmenu` v2, `gtk_shell1` v5 with the
  gestures and the capability, the fork's X11 properties, `RegisterWindow`
  ids to X11 windows, the PID fallback, the lookup order. `isle menus
  show <window>` prints a window's tree and its source.

**Check:** headless, one test per source: a test client exporting a tree
through each of the four paths shows its menus in the bar within 100 ms of
the reveal once cached, its Ctrl+C item hinted ⌘C, toggles and radios
marked, a submenu fetched through AboutToShow, an activation reaching the
client; a client that never answers gives the fallback after 2 s and a
submenu that times out opens with what it had; a burst of a hundred
LayoutUpdated signals refetches once; a sender leaving the bus drops its
menu; the registrar is owned before the first client connects (the client
checks the name at start).

### 6.6 · Menu search and the measure

- The index over the front app's tree and Isle's Help items; Help's search
  field with paths, disabled results, no results, the found item opened
  down its path; the index on the bus for P10.
- Measures: the bar's and the island's texture memory and draw times at
  3440×1440, against acceptance bars 3 and 5.

**Check:** headless: "find" in a test tree lists the nested item with its
path and Return activates it; moving onto a result opens its titles; the
bar's open cost and the island's draw p95 recorded in
`docs/research/rust-desktop/p6-island-and-menus.md`.

### 6.7 · The VM and the host

- The VM: the acceptance bar 8 matrix (a Qt 6.9+ app, Chromium on
  Wayland, LibreOffice on GTK 3, a `GtkApplication` with a menubar), plus
  an X11 Qt app, kitty and a libadwaita app on the fallback; the island and
  bar over a full-screen window; two outputs.
- The host, in the user's run on tty8: the wallpaper on the ultrawide, the
  island at rest for an hour (frames traced), Control Centre's peek and
  pin, Chromium's menus, VS Code's and Steam's fallbacks, a game with the dwell.

**Check:** the VM's screenshots and `isle menus show` per app; the host
run's notes from the user.

### 6.8 · Close

- ARCHITECTURE: how the top of the screen works (the wallpaper element,
  the island's shape and model, the top edge, the bar and the menu
  sources).
- DECISIONS for what later packets must keep (the island's model as the
  one place sources post; the shape is the compositor's; one top-layer
  panel per screen; menus as one tree from four sources with the registrar
  before the first client; the main screen; the launch path).
- `shell.md`'s and `platform.md`'s open points answered, `compositor.md`'s
  `gtk_shell1` point; `state.md`, `progress.md`, the roadmap's P6 entry.

## Not in P6

Every Control Centre module but Dark Mode and Keyboard layout, the tray,
edit mode and the gallery, and the services behind the glyphs, the
privacy marks and the level OSDs (P8); the prompt, lock and session dialog
behind the system menu's disabled items, and the island's locked form
(P9); Notification Centre and the clock opening it (P17); Spotlight and its
menu-item source (P10); the capture activities and the screen privacy mark
(P11); Game Mode proper (P12); Settings' Wallpaper, Control Centre and
clock pages (P13); the softened wallpaper (P9, P17); the bundled pictures
(an art task before P14); hot corners and Show Desktop (P16); keep on
top's chord (P20); menus for apps that export none beyond the fallback.

## Depends on, and feeds

- Needs P3 (surfaces, glass, springs, tokens), P4 (the registry, apps,
  the actions the Window menu runs, the title bars) and P5 (the inverse
  tables, the inhibit and game state, the reserved chords).
- P7's switcher closes the bar and reads attention and icons. P8 posts
  every service glyph, privacy mark, OSD and media activity to the
  island's model and fills Control Centre. P9 enables the system menu's
  items, draws the locked island and the softened wallpaper. P10 takes the
  menu index and the launch path. P11 posts recording, sharing and the
  screen mark. P12 replaces the Game Mode flag. P13 draws Settings' pages
  over the new sections. P17 opens Notification Centre from the clock.

## Done when

- `cargo test` passes headless: the island's queue, the wallpaper's fits
  and changes, the top edge's push, dwell and guards, Control Centre's
  frame and two modules, the bar's reveal and menus, every menu source,
  the fallback, hints, search, the window menu.
- In the VM, the four-app matrix shows real menus, the rest fall back, and
  nothing in the strip moves when the bar comes.
- On the host, the island costs one frame a minute at rest, the wallpaper
  and island read as the board, and Chromium's menus are in the bar.

## Risks

- **The morph's cost.** Re-laying out the tree every frame of a spring at
  full panel size may approach the 2 ms budget; if it does, the outgoing
  form is drawn once into its own texture and only the incoming one
  re-renders.
- **GTK's hidden menubar.** Advertising `global_menu_bar` takes the menubar
  out of GTK windows; a reader bug would strand them. The capability is a
  setting (`debug.gtk-global-menu`, on) so it can be turned off without a
  build.
- **Electron.** Until its fix ships, an Electron app with a native menu
  registers a window id nothing matches; the PID path may find it or may
  pick the wrong window of a multi-window app, which 6.7 checks on the
  host with an app that exports. VS Code exports nothing by default, so its
  bar is the fallback, which its table's labels make useful.
- **A registrar already running.** A leftover registrar from another
  desktop would take Isle's menus; choice 15 queues and reports it.
- **The bar's texture.** A full-output surface while revealed costs about
  20 MB of VRAM on the ultrawide; if 6.6 finds it too much the menus move
  to placed surfaces of their own and the band shrinks to its height.
