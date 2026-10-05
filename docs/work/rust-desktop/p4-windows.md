# P4 · Windows

Planned and done 2026-10-05, but for the user's host run. The record is
`docs/ARCHITECTURE.md` (How windows are kept) with B40–B42; the
measurements are in `docs/research/rust-desktop/p4-windows.md`.

The roadmap's brief: stacking with snapping and the ultrawide zones (B8);
app-centric switching mechanics and hide-on-last-close policy, with Cmd+Q
quitting keep-running apps (B20, B30); minimise, maximise, full-screen
Spaces, window places; server-side title bars drawn in Freya (B25); the
Xwayland window manager with v1's X11 lessons (B5).

P2 left windows centred and floating, every client drawing its own frame,
and no X11. P4 makes the compositor a window manager: every toplevel,
Wayland or X11, belongs to an app and a desktop, opens where it was last,
tiles to the ultrawide's zones by drag, chord or the zoom button, hides,
minimises, fills and goes full screen into a desktop of its own, and wears
Isle's title bar when it lets the compositor draw one. What it must get
right is the model the switcher (P7), Spotlight (P10), Mission Control
(P16) and Game Mode (P12) all read: one registry of windows with their app,
desktop, state and boxes, of which Smithay's `Space` is only the part shown
now.

## What it answers to

- Decisions: B5 (Smithay's X11 WM, X11 policy Isle's), B8 (stacking,
  zones, real minimise and maximise, layout behind a per-desktop trait),
  B17, B20 (app-centric windows, keep-running), B25 (Freya title bars),
  B30 (Cmd+Q quits keep-running apps), B33 (windows opaque; the zoom
  menu's glass), B35 (the loop and Test interface), B37 (surfaces, which
  this packet extends with the window-attached kind).
- Intent: `compositor.md` Stacking 1–9, Snapping 1–8, Apps 1–11,
  Minimise and maximise 1–5, Spaces 1, 2, 5–8, Server-side decorations
  1–6, Xwayland 1–9, Shell surface hosting 4 (the window list and
  actions), Pointer 2–3; its acceptance bars 2, 3, 8 (Steam's client and
  menus, X11 Chromium moving by its own controls) and 13. `gaming.md`
  Full-screen Spaces where it is mechanism. `platform.md` Settings for the
  new `windows` section.
- Design: `p1/structure/window-chrome.md` and `p1/surfaces/window-chrome.md`
  (binding: the bar, the lights, the zoom menu, the zone preview, the
  pointer panel, the work area, the choreography, states, keys, copy),
  `p1/structure/mission-control.md` (desktops, the switch motion, full
  screen), DESIGN.md (Motion, Materials, the `traffic` and `size` tokens
  already in `tokens.kdl`).
- Evidence: `smithay-and-gaming.md` §1.4 (Steam menus under
  xwayland-satellite, the failure B5 avoids), `isle-v1-inventory.md` §3.3
  to §3.6 (the X11 rules, `isle-windows`, places, keep-running),
  `macos-model.md` §3 (app-centric windows, the chords). niri and
  cosmic-comp, both on Smithay, are the models for window transitions,
  close snapshots, rounded clipping and Xwayland.

## Choices this plan makes

The user took every recommendation at review (2026-10-05).

1. **One registry; the `Space` is its view.** A `wm` module in
   `isle-compositor` keeps every toplevel, Wayland or X11, from its first
   commit to its destruction: its id, app, PID, parent, desktop, mode,
   boxes and flags. Smithay's `Space<Window>` holds only what is on
   screen now, in stacking order; a hidden, minimised or other-desktop
   window leaves the `Space` and keeps its record. The geometry (zones,
   edge regions, placement, clamping, edge snapping, cascade) is a module
   of plain functions over rectangles, unit-tested at 3440×1440 and
   1920×1080, with no Smithay state in it. Layout is a trait chosen per
   desktop (B8); stacking is its one implementation.
2. **A window's state.** One mode: floating, tiled (a zone and the box
   before it), filled (the box before it; this is maximise, and the
   client is told it is maximised) or full screen (its origin desktop and
   box). Two flags beside it: minimised (the window's) and hidden (its
   app's). Both park the window off the `Space`; they are two flags, not
   one parked state, so a window minimised inside a hidden app comes back
   minimised when the app is shown (macOS; answers `compositor.md`
   Minimise and maximise, Open). A tiled window is sent the `tiled_*`
   states for its edges. A parked window, and one on a desktop not shown,
   gets no frame callbacks and is sent xdg-shell's `suspended` state
   (answers Apps, Open).
3. **Desktops.** Each output has its own row, keyed by the output's
   identity, starting with one. Ctrl+←/→ moves along the row with no
   wrap; Ctrl+1…9 jumps, and a number past the end adds desktops up to it
   (v1's numbered habit) until Mission Control (P16) adds them with its +;
   at most sixteen (answers Spaces, Open: dynamic, chords for the first
   nine). `isle desktops add|remove|list` covers the rest. The count per
   output is kept in a `desktops` state domain. A window keeps its
   desktop by index; when its output goes it joins the current desktop of
   the output it moves to and goes back to its own when that output
   returns (answers Spaces, Open). The switch is the gate's short slide
   with a crossfade; the wallpaper, the strip and the top layer stay.
4. **Full-screen desktops.** A window going full screen
   (`set_fullscreen`, `_NET_WM_STATE_FULLSCREEN`, the zoom light,
   Ctrl+Cmd+F) gets a new desktop right of its origin; the view follows
   only if it has focus. A window covering its output exactly with no
   decoration is given one the same way without being told it is full
   screen (D48). Ending full screen or closing removes the desktop and
   returns the window and the view to the origin. While one is shown,
   layer-shell's top layer and the shell's top surfaces stay drawn only if
   they ask to be (the island's reveal is P6's).
5. **Places.** A `places` state domain keyed by app and output identity
   holds the main window's box relative to that output's work area. It is
   recorded when a user move or resize of the app's only titled toplevel
   ends (a grab, a drop into a zone, a chord), never polled; applied as
   the size of the first configure and the location at map, so the first
   frame is at the place. A place for an output that is absent is clamped
   onto the output with focus. Dialogs, untitled and second titled
   windows, full-screen windows and override-redirect windows are never
   recorded or placed (D40). A window with no place opens at the size it
   asks for, or 50% of the work area's width by 70% of its height when it
   asks for none, centred on the output with focus; a second window of the
   same app is cascaded down and right by the bar's height (answers
   Stacking, Open: per app and output; xdg-toplevel-tag waits until
   Smithay serves it).
6. **Dialogs.** A toplevel with a parent opens centred over it and moves
   with it while it is modal. A modal dialog smaller than its parent is
   attached flush under the parent's bar and the parent's content takes
   `colour-scrim` (window-chrome For the user 4); larger ones stay centred.
   A modal dialog keeps its parent from taking focus; a click on the
   parent focuses the dialog.
7. **Title bars are attached surfaces.** B37 gains a third scope,
   attached: an instance the compositor makes, sizes, places and removes,
   one per decorated window, drawn in that window's place in the stack
   rather than in a layer. Its tree reads the window's model (title,
   active, lights enabled, mode, Option held, hover) from a context the
   compositor updates, and asks for window actions (close, minimise, zoom,
   fill, the title-bar action, start a move, open the zoom menu) through
   `Actions`. It sits behind the panic boundary like every surface; a
   panicking bar leaves its window undecorated until it is rebuilt. The
   frame is the bar plus the client's geometry; the client is configured
   with the frame less the bar. The cost of a tree per window is measured
   in 4.6 and given a budget; if it is too high, the fallback is one tree
   drawing every bar on an output into an atlas, recorded rather than
   built.
8. **Decoration negotiation.** xdg-decoration and KDE server-decoration
   prefer server side; a client that asks for client side gets it. Isle's
   own apps ask for client side. X11 windows are decorated unless Motif
   hints turn decorations off, `_GTK_FRAME_EXTENTS` is set, the window is
   override-redirect, untitled, transient-for a menu, or full screen. The
   decision is taken again on every change of mode, hints or full screen
   (D48). With title bars off in settings, server-side windows get none.
9. **The window's drawing.** Every frame is clipped to `radius-window`
   (an SSD client's bottom corners; a CSD client only when its geometry
   is not inset from its surface, so a client that draws its own shadow
   keeps it) and gets the compositor's shadow, `shadow-window` active and
   `shadow-window-inactive` otherwise, under the same rule. Clipping is a
   texture program over each surface element; shadows are a pixel-shader
   element. Nothing is blurred under a window (B33), X11 included.
10. **Window transitions.** Every window is drawn through a transform
    (offset, scale about an anchor, opacity) driven by P3's springs from
    the motion tokens, so the choreography of window-chrome's Motion table
    applies: open, close and hide, show after hide, minimise toward the
    strip's top centre (the island's own rectangle comes with P6) and
    restore, the frame move for tiling and fill with the old content held
    scaled until the client commits and then crossfaded, full screen, and
    the desktop switch. Close and the held content need the window's last
    frame: the compositor renders the window into a texture when a close
    or a resize begins. Reduce Motion and the no-transitions switch apply
    as for surfaces.
11. **Moves, resizes and zones.** One move grab and one resize grab serve
    every route (Isle's bar, a client's `move`/`resize`, `_NET_WM_MOVERESIZE`,
    the Cmd modifier drag, which P5's shortcut set may move). During a
    move, edges snap to other windows and to the work area, and the
    pointer in an edge or corner region shows the zone preview (a
    compositor element under the dragged window, moving between zones on
    `motion-move`); release tiles. Holding Option shows the pointer panel.
    Dragging a tiled or filled window away restores its box under the
    pointer at the same relative place. The bottom edge gives no zone.
    Region sizes: 8 points at the edges, corners 64 points along each edge
    (answers Snapping, Open; tuned on the host).
12. **The zoom menu and the pointer panel.** Shell surfaces in the top
    layer opened at a box the compositor gives them, so B37 gains surfaces
    placed at open. They draw window-chrome's grid of thumbnails in the
    output's shape on glass, with keyboard selection; hovering a slot shows
    its zone preview. Arrangements of several windows (Snapping, Open) are
    not built: the menu has no section for them.
13. **Apps.** A crate `isle-apps` reads desktop entries (a small parser of
    `[Desktop Entry]`) into an index the worker builds at start and on
    change and hands the loop as a snapshot, so resolution never touches
    the disk on the loop. An app id resolves by desktop id, then
    `StartupWMClass`, then case-insensitively, then by the last dotted
    part; `WM_CLASS` for X11; `steam_app_<id>` and gamescope windows stay
    apps of their own, named by P12. An id that resolves to nothing is its
    own app (answers Apps, Open). Grouping is by app, never by PID.
14. **Hide, close and quit.** The close path (Cmd+W, the close light, and
    later the switcher and Mission Control) hides a keep-running app on its
    last window and closes otherwise; `windows.keep-running` lists apps by
    desktop id. Cmd+Q sends a close to every toplevel of the app and, once
    they are gone, the polite signal to each of its PIDs still running; a
    window that stays is left to the not-responding path. P5's
    translation table later swaps in an app's own quit chord. Cmd+H, Cmd+
    Option+H, Cmd+M, Cmd+Option+M and Cmd+` (the front app's windows in a
    stable order) are built; Cmd+Tab's switcher is P7's, on this model's
    app recency.
15. **Not responding.** A window is pinged when the user acts on it (a
    click, a close, a quit), not on a timer, so nothing wakes at rest; no
    pong within three seconds marks it not responding, shown by the busy
    pointer over it, and a later pong clears it. The alert offered on close
    and the Force Quit dialog are P9's prompts.
16. **Chords until P5.** `binds` becomes a keyed table of action ids to
    chords (`fill "Ctrl+Option+F"`), with the defaults in code from
    window-chrome's Keyboard table and `compositor.md`'s Shortcuts 2, so
    P4's thirty-odd chords exist without a field each. P5 turns it into
    the shortcut set (labels, clash refusal, translation, the escape set).
    Chords match the physical keys.
17. **Xwayland.** Started at session start with Smithay's `XWayland`, its
    `DISPLAY` reported to the supervisor on the ready pipe beside
    `WAYLAND_DISPLAY` and exported; restarted with a backoff if it dies,
    its windows gone with it. X11 windows are `Window`s in the same
    registry and `Space`. The WM's policy is window-chrome's and
    `compositor.md` Xwayland 2–9: override-redirect drawn at its own
    place and never placed, focused, decorated, animated or shadowed; an
    untitled window treated as a menu; a window with no class, title or
    full-screen state never focused; `_NET_WM_MOVERESIZE` starting Isle's
    grabs; state requests honoured as their Wayland twins; transient
    windows over their parent; `_NET_WM_PING` feeding not responding;
    `_NET_WM_SYNC_REQUEST` pacing resize. X11 clients draw at scale 1 and
    are scaled on a fractional output (answers Xwayland, Open; the host's
    outputs are at 1).

## The workspace

```
crates/
  isle-compositor/src/
    wm/               the registry, modes and flags, desktops, focus and MRU,
                      places, apps' close, hide and quit, the layout trait
    wm/geometry.rs    zones, edge regions, placement, snapping (pure)
    wm/render.rs      clip, shadow, window transforms, snapshots
    xwayland.rs       the X server, the WM and its policy
    grabs.rs          one move and one resize grab for every route
  isle-apps/          desktop entries, the index, app-id resolution (new)
  isle-shell/         the attached scope and surfaces placed at open
  isle-shell/src/chrome/   the title bar, the zoom menu, the pointer panel
  isle-ui/            the traffic lights and zone thumbnails as components
  isle-ipc/           Windows1 grows, Desktops1 (new)
  isle-cli/           isle windows …, isle desktops …
```

The test client grows commands on its standard input (title, full
screen, maximise, minimise, a parent, minimum and maximum sizes, a
decoration request, a move on press, ignoring pings) and prints every
configure; a second test client, `isle-test-x11-client` on x11rb, opens X11
windows with the properties the WM's rules read.

## Phases

Each phase ends in a check run by script or in the VM and lands as its
own commits.

### 4.0 · The registry

- `wm`: the records, ids, app (by app id for now), PID, parent, mode and
  flags; the `Space` as the shown part; MRU of apps and of windows;
  focus and raise through it. P2's centring, focus and output rescue moved
  onto it unchanged.
- `Windows1.List` reports app, desktop, mode, flags and parent; actions by
  id: focus, close, minimise, unminimise, hide app, show app, fill,
  unfill, full screen, tile to a zone, move to a desktop, set box.
  `isle windows list|focus|close|…`.
- The test client's command channel and configure printing.

**Check:** headless: every existing test passes on the registry; a
window minimised by id leaves the screenshot and the `Space`, gets no
frame callbacks and is listed minimised; restored, it is back in its box
and stacking order.

### 4.1 · Placement and places

- The work area: the output less `size-strip` along its top and any
  exclusive zones. The first configure carries the place's size; map puts
  the window at its place or centres it; cascade; dialogs over their
  parent, modal ones attached and the parent scrimmed; modal focus.
- `places` recorded at the end of user moves and resizes of a main
  window, keyed by app and output identity; never for dialogs, untitled,
  full-screen or off-output boxes.

**Check:** headless: a client closed after a move reopens at that box in
its first presented frame (the first screenshot after map shows it
there, with no intermediate box ever reported); a dialog opens centred
over its parent and is not recorded; a place on an absent output is
clamped onto the present one; a place partly off its output is never
recorded.

### 4.2 · Moves, resizes and zones

- `wm::geometry` with its unit tests: every zone at 3440×1440 and
  1920×1080 with and without tile margins, edge and corner regions,
  cycling chords (halves, two-thirds, thirds; 60%, centre third), edge
  snapping, unsnapping, clamping.
- One move grab and one resize grab for every route; the zone preview
  element; tiling and fill as modes with their pre-boxes and `tiled_*`
  and `maximized` states; Cmd-drag moving an inactive window without
  raising it.
- The `binds` table (choice 16) with the tiling, fill, return, centre,
  quarter, beside and display chords.

**Check:** headless: a drag released in each edge region tiles to its
zone; the bottom edge does not; a tiled window dragged away returns to
its old size under the pointer; each chord gives its zone, repeated
chords cycle; the client is configured with the zone's size and the
right states; a resize from the left edge keeps the right edge still.

### 4.3 · Apps: hide, close, quit, minimise

- `isle-apps` and the index on the worker; app resolution at map.
- The close path with keep-running; Cmd+Q's close-all and signal; Cmd+H,
  Cmd+Option+H, Cmd+M, Cmd+Option+M, Cmd+`; activating an app raises its
  windows on the desktop in their order; xdg-activation with a fresh token
  focuses, otherwise marks attention.
- Pings on interaction and the not-responding mark with the busy pointer.
- The `windows` settings section: `keep-running`, `titlebar-action`
  (fill, minimise, none), `tile-margins`, `title-bars`.

**Check:** headless with two clients of one app id and one of another:
Cmd+W on a keep-running app's last window hides the app and Cmd+Q on it
ends the process; Cmd+H then activating the app shows its windows in
their order; Cmd+` walks the app's windows in a stable order; a client
that ignores pings is marked not responding within three seconds of a
click and cleared by a pong. Unit tests resolve ids against a fixture
set of desktop entries.

### 4.4 · Desktops and full screen

- Desktops per output (choice 3), the chords, move to a desktop, the
  `desktops` state domain, `Desktops1` and `isle desktops`.
- Full-screen desktops (choice 4) from every route; borderless
  detection; the return on leaving or closing; minimise disabled in full
  screen.
- Windows following their output away and back now keep their desktop.

**Check:** headless: Ctrl+→ shows the second desktop's windows only and
Ctrl+← brings the first back; a full-screen request makes a desktop right
of the origin and ends with the window back in its box on the origin; a
borderless window the size of the output gets a desktop and is not told
it is full screen; a removed and re-added output's windows return to
their desktops.

### 4.5 · Drawing and motion

- The rounded clip and the shadow (choice 9), active and inactive.
- Window transforms on springs (choice 10): open, close from a snapshot,
  hide and show, minimise and restore toward the strip, the frame move
  with held content and crossfade, full screen, the desktop slide and
  crossfade. Reduce Motion and the no-transitions switch.

**Check:** headless: screenshots of a window's corners show the clip at
`radius-window` and the shadow's falloff, deeper while active; a closed
window's snapshot is drawn for the exit's frames and none after; a
still desktop after any transition presents no frames; with transitions
off every move is a cut.

### 4.6 · Title bars

- The attached scope and placed-at-open surfaces in `isle-shell`
  (choices 7, 12); the bar's tree in `isle-shell`'s `chrome` with the
  lights and zone thumbnails as `isle-ui` components, from window-chrome's
  spec: lights, glyphs on hover of the group, Option's plus, disabled and
  inactive forms, the title's centring, leading fallback, truncation and
  drop, Increase Contrast's outlines, the largest text.
- Decoration negotiation (choice 8); the frame; input: lights, drag to
  move, double-click to the title-bar action, the resize band on every
  edge and corner of a decorated window; inactive lights acting without
  raising.
- The cost of a bar per window measured: RSS and VRAM with thirty bars;
  the budget written into `shell.md`.

**Check:** headless: a server-side client gets a bar, a client-side one
does not, and switching modes adds and removes it; a click on each light
closes, minimises and makes a full-screen desktop; Option-click on zoom
fills; a drag on the bar moves the window and tiles at an edge; a
double-click fills and again returns; the bar's accessibility tree names
the lights; a panicking bar is rebuilt and the window keeps working.
Screenshots of active, inactive and hovered bars compared against the
P1 board's crops by eye.

### 4.7 · The zoom menu and the pointer panel

- The zoom menu from a dwell on the zoom light, a press held, its chord;
  its grid in the output's shape, the beside slots from the real
  arrangement, Return to previous size, Move to {display}; keyboard
  selection; the slot's preview on the screen.
- The pointer panel with Option during a drag.

**Check:** headless: a dwell of `timing-zoom-dwell-ms` opens the menu and
a pass does not; choosing a slot tiles the window there and closes the
menu, focus back on the window; Option during a drag shows the panel and
a release over a slot tiles there.

### 4.8 · Xwayland

- Smithay's `XWayland` and `X11Wm` (choice 17), `DISPLAY` through the
  supervisor, restart with a backoff; X11 windows in the registry with
  their policy; `isle-test-x11-client`.

**Check:** headless: an X11 window maps, takes keys and gets Isle's bar;
an override-redirect window is drawn at its own place and never focused;
`_NET_WM_MOVERESIZE` moves the window and tiles at an edge; a Motif
no-decorations window and one with `_GTK_FRAME_EXTENTS` get no bar;
minimise and full-screen requests act as their Wayland twins; a transient
opens over its parent; Xwayland killed comes back and `DISPLAY` still
works.

### 4.9 · The VM and the host

- The VM: kitty with Isle's bar, a GTK 4 app client-side, a Qt app, X11
  apps; tiling by drag and chord; desktops; full screen; screenshots.
- The host, in the user's run on tty8 as in P2: Firefox and Chromium,
  VS Code, Steam's client with its context menus opening where clicked
  and staying open, X11 Chromium moving by its own controls, a full-screen
  game on its own desktop.
- `docs/research/rust-desktop/p4-windows.md`: the bar's cost, the X11
  findings, region sizes as tuned.

**Check:** the VM screenshots; the host run's notes from the user.

### 4.10 · Close

- ARCHITECTURE: how windows are kept, the attached scope, Xwayland.
- DECISIONS for what later packets must keep (the registry and its view,
  the two parked flags, places, attached surfaces).
- `compositor.md`'s open points answered; `state.md`, `progress.md`, the
  roadmap's P4 entry.

## Not in P4

The window menu and keyboard move and resize from it (P6, on the shell's
menu component, with `show_window_menu`); Cmd+Tab's switcher (P7); the
not-responding alert and Force Quit (P9); the inhibit notice and Cmd
translation (P5); Show Desktop, Split View, Stage Manager, Mission
Control and gestures (P16); picture-in-picture and the dropdown terminal
(P20); keep on top and focus by direction (P20, with the pin's chord);
the island's rectangle as minimise's target and the top-edge reveal over
full screen (P6); session restore (P20); Game Mode's profile (P12);
xdg-toplevel-icon and -tag, and the global menu's X11 properties (P6).

## Depends on, and feeds

- Needs P2 and P3 (surfaces, springs, tokens, glass).
- P5 replaces the `binds` table with the shortcut set and translates
  Cmd. P6 opens the window menu, uses the island's rectangle and reveals
  bars over full screen. P7 reads app recency and minimised and hidden
  windows. P12 detects games among the registry's windows and turns
  places and motion off. P16 draws desktops and the spread from the
  registry and its live textures.

## Done when

- `cargo test` passes headless with Wayland and X11 clients: places on
  the first frame, every zone and region, hide, close, quit and minimise,
  desktops and full-screen desktops, title bars with their lights, the
  zoom menu, the X11 rules.
- In the VM, kitty, a GTK 4 app, a Qt app and an X11 app behave as
  window-chrome says, screenshots kept.
- On the host, Steam's client and menus and X11 Chromium pass
  `compositor.md`'s bar.
- A bar per window has a measured cost and a budget in `shell.md`.

## Risks

- **Scope.** This is the largest packet so far. The phases are ordered so
  that each leaves the desktop usable; 4.5's motion and 4.7's menu can
  slip to a later packet without blocking P5–P7 if they run long.
- **A tree per bar.** Freya's runtime per window may cost more than a bar
  is worth; measured in 4.6, with the atlas fallback recorded.
- **Snapshots on NVIDIA.** Rendering a window into a texture as it closes
  is the path niri uses on NVIDIA; it is checked on the host in 4.9.
- **Xwayland and Steam.** Steam's popups are the failure B5 was chosen to
  avoid; if Smithay's WM shows the same faults, they are fixed here or
  upstream before P12.
- **Chords before P5.** Desktop chords on Super intercept the keys apps
  would get as Ctrl after P5's translation; until then Cmd+W, Cmd+Q and
  the rest are the desktop's, which is their meaning anyway.

## Outcome

Phases 4.0 to 4.9 passed their checks headless and in the VM, with these
differences from the plan:

- **Chords until P5** are a field each in `binds`, not a keyed table: the
  settings derive already types, validates and introspects fields, and P5
  replaces them anyway. Ctrl+1…9 is one field naming the modifiers.
- **The modifier drag** is Ctrl+Cmd, macOS's own drag gesture, not Cmd:
  a Cmd+click is a link opened in a new tab once P5 translates it.
- **The bar's cost** is small (0.27 MB and 0.47 MiB a bar), so the atlas
  is not needed; the budget is in `shell.md`.
- **The zone grid** hit-tests from its own layout function in the tree as
  well as in the compositor; Freya sent no enter to absolutely placed slots.
- **The ready pipe**: Xwayland, started before the compositor reported its
  socket, inherited the pipe's write end, so the supervisor never exported
  the session. The fd is close-on-exec from the start. Headless tests do not
  use the pipe; the VM found it.
- **Not built, for later packets:** the window menu and keyboard move and
  resize (P6, with `show_window_menu` and `gtk_shell1`); X11 pings and
  `_NET_WM_SYNC_REQUEST` pacing; reading `_GTK_FRAME_EXTENTS`; Split View's
  entries in the zoom menu (P16); closing the hover-opened zoom menu when the
  pointer leaves it (it closes on a choice, Escape or a click elsewhere);
  the bar's crossfade when entering full screen.
- **Waiting on the user:** the host run on tty8 (Firefox, Chromium, VS Code,
  Steam's client and its menus, X11 Chromium moving by its own controls, a
  full-screen game on its own desktop).

Tests: `cargo test --workspace --features isle-compositor/test-hooks`, 118
in all, with 32 headless window tests and 6 X11 tests on Xwayland; the bar's
cost in `tests/bench.rs` by hand.

