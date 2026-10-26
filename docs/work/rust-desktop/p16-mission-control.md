# P16 · Mission Control

Planned 2026-10-26. Reviewed by the user the same day: every
recommendation taken, then trackpad gestures deferred (choice 2).

The roadmap's brief: the Spaces overview, Mission Control's actions in the
hot corners (the engine, its four actions and Settings' Hot Corners… sheet
came in P13, B99), trackpad gestures and Settings' Trackpad pane, Split
View; Stage Manager as a layout module, off by default (B25).

P1 designed all of it. `p1/structure/mission-control.md` and
`p1/surfaces/mission-control.md` fix Mission Control's Spaces bar, spread
and tray, App Exposé, the Space switch, Split View with its chooser and
divider, Stage Manager's column, the hot corners' actions, the gestures and
the mouse routes, with every state, key, string, token and motion;
`window-chrome.md` fixes Show Desktop's slivers and the zoom menu's Full
screen section. What is open is how it is built, and the rows P1 named but
never laid out: Settings › Desktop's Mission Control and Stage Manager
groups, the shortcuts the Keyboard Shortcuts sheet gains and the mouse
buttons. The gestures and the Trackpad pane wait for a trackpad (choice 2).

Today the window model is ready for it and nothing of it exists. The
registry (B40) keeps every window with its desktop, mode and flags;
`wm/desktops.rs` keeps a row of up to sixteen desktops per output with
full-screen desktops right of their origin; the switch is a 120-point slide
of snapshots, hardcoded rather than the token's 80; `wm::draw::place` draws
any window's pieces moved and scaled onto any rectangle; the switcher's
previews (B54) are compositor drawings copied into Skia images. Ctrl+↑,
Ctrl+↓ and Cmd+F3 are bound and swallowed as `Action::Reserved`. libinput's
gesture events fall through `process_input_event`'s `_ => {}`;
pointer-gestures is not served; touchpads are left out of `pointer_names`,
and there is no Trackpad pane. The hot-corner engine has four actions and
no Mission Control. Half-and-half is two independent tiles; there is no
pairing, divider or layout trait.

What P16 must get right is that Mission Control is the desktop's way to
switch with the pointer, so it must be instant and stay inside bar 5 with
twenty live windows on the ultrawide: its motion starts on the frame after
the key or the corner's rest, and the windows are the tiles, so they
cannot be redrawn as copies each frame. And every desktop rule P4 and P12
built (focus, the constraint's release, VRR, the island over full screen)
must hold through everything Mission Control and Split View do to desktops.

## What it answers to

- Decisions: B8 (layout behind a per-desktop trait), B13 and B54 (previews
  are compositor drawings), B17, B18 (the top edge), B19 (no Dock; the
  default corners; minimised windows in App Exposé and Mission Control),
  B20, B24, B25 (Stage Manager later, off), B30 (the escape set), B33
  (glass on the bars, solid forms), B37 and B39 (surfaces, glass is the
  compositor's), B40–B42, B46 and B89 (games, paced when out of view), B53
  and B57 (keys without focus; centre surfaces close the bar), B60, B73
  (the compositor owns a surface's model and keys), B94 and B97 (Settings'
  rows), B99 (the corner engine).
- Intent: `shell.md` Mission Control, hot corners and gestures 1–9 and its
  open points, No Dock 2, 4, 7 and 8, Shell runtime 10, 11 and 14, bars 5
  and 13; `compositor.md` Stacking 3, 9 and 10, Spaces 1–8, Split View 1–5,
  Stage Manager 1–2, Minimise 1 and 4, Server-side decorations 3–4, PiP 3,
  Dropdown terminal 3, Pointer, touchpad and gestures 5 (6 and 7 deferred,
  choice 2), Shortcuts 2 and 7, Shell surface hosting 4; `gaming.md`
  Full-screen Spaces; `apps.md` Settings' Desktop pane; `platform.md`'s
  Windows and Spaces interfaces.
- Design, binding: `p1/structure/mission-control.md` and
  `p1/surfaces/mission-control.md`; `structure/window-chrome.md` and
  `surfaces/window-chrome.md` (Show Desktop, the zoom menu's Full screen
  section, the divider's keys); `dock.md`, `competition.md`, `pip.md`,
  `dropdown-terminal.md`, `menu-bar.md` (Full screen tile), `island.md`,
  `copy.md`; `structure/settings.md` and `surfaces/settings.md` (Desktop,
  the Hot Corners… sheet, Keyboard Shortcuts);
  `review/critique.md` MC1–MC9 and C12, `review/accessibility.md` MC-01–03.
- Evidence: `mission-control-2026.md` (new), `isle-v1-inventory.md` §3.6
  and its Mission Control row, `design-v1-lessons.md` § Mission Control,
  `p7-switcher.md` (the preview path's measures), `gaming-2026.md` (paced
  windows).

## Choices this plan makes

### Scope

1. **One packet, Stage Manager last.** Mission Control, App Exposé, Show
   Desktop and Split View share one model of a desktop's windows and one
   way of drawing them moved, so they are built together. Stage Manager is
   the one part that needs B8's layout trait, is off by default and is
   least used here; it is the last two phases and can be cut to a packet of
   its own after P17 without touching the rest, if the rest runs long. That
   would be the user's call at the time, not this plan's.
2. **Trackpad gestures are deferred to a packet of their own.** The host is
   a desktop with a mouse and no trackpad (`mission-control-2026.md` §7),
   and `shell.md` puts mouse routes first. Gestures would be code that never
   runs here, proved only through a fake touchpad in the VM, and they would
   force the riskiest change in the plan: a desktop switch the fingers can
   hold halfway and take back, which moves the moment the current desktop
   changes, on which focus, the constraint's release, VRR and the island
   over full screen all hang. Deferred with them: libinput's gesture
   events, pointer-gestures served to clients, the Trackpad pane, and the
   touchpad's `tap` and `scroll-method` rows. The roadmap gains the packet
   (16.0), started when a trackpad is used; `mission-control-2026.md` §2
   and §4 are its evidence. What P16 builds keeps the way open: the switch
   and Mission Control's opening are each one progress on one spring
   (choices 5 and 11), so following fingers is a change in one place.
   `shell.md` Mission Control 9 and `compositor.md` Pointer 6–7 stay as
   intent, marked deferred.
3. **A small gate for the rows P1 named but did not lay out** (16.1):
   Desktop's Mission Control and Stage Manager groups, the Keyboard
   Shortcuts sheet's new rows, the mouse buttons, and the choices this plan
   proposes in P1's terms (choices 13, 18, 21, 23). One board, recommended
   choices drawn, the user answering as at earlier gates. Everything else
   in P1's sheets is binding and complete; a gap found in building gets the
   smallest fix in P1's terms, listed in the outcome.
4. **The record's disagreements settled for P1's sheets**, which were
   decided later and in more detail, in 16.0:
   - the current desktop's ring is `colour-text`, not the accent
     (DESIGN.md's Surfaces line and the `surfaces-c.html` note are wrong;
     C12);
   - App Exposé is not opened from the switcher (`shell.md` Mission
     Control 1, `dock.md` line 39): the switcher's window row replaced it;
   - the thumb-and-three pinch opens the Apps view (P1 added it; into
     `shell.md` and `compositor.md` with the deferred gestures);
   - Mission Control opens on every screen, an exception to Shell runtime
     14, written there;
   - a Split View desktop refuses drops (into `compositor.md` Split View 1);
   - the drop forms keep the zone preview's form (MC-02 answered).
   The hot corner's dwell is choice 18.

### The desktop switch

5. **The switch keeps its slide, from its token.** `slide_out` and
   `slide_in` stay as P4 built them, the current desktop changing at once,
   with `motion-switch-distance` (80) in place of the hardcoded 120 and the
   `switch` spring. A desktop chosen in Mission Control closes into this
   switch (choice 11). Nothing bounces past the first or last desktop.
6. **Full screen joins the switch.** Entering and leaving full screen leave
   the growing window out of the slide and let its `Morph` carry it, so a
   full-screen window grows and never slides (`structure/mission-control.md`
   Space switch).

### Mission Control

7. **The compositor owns Mission Control's model and keys** (as B73 for
    Spotlight). A pure model in `isle-compositor/src/mission/`: the mode
    (Mission Control or App Exposé of an app), each output's bar (its
    desktops with kind, label and miniature size, the + tile while under
    sixteen), its spread (tiles with the window, the target rectangle, the
    group), its tray (minimised then hidden), the selection and where it
    moves for each key, the drag and its target. It is computed from the
    registry and `Desktops` and re-laid when they change. The keys of
    `structure/mission-control.md`'s table are read by the compositor while
    it is open; its surfaces take the keyboard (it is level 4 of
    `competition.md`, unlike the switcher), so typing reaches nothing else.
8. **The spread's layout is GNOME's row strategy, kept near the real
    places.** Windows sorted by centre y fill rows towards an ideal width,
    each row sorted by centre x; every row count from one to the count
    that makes tiles smaller than the switcher's cards is tried and scored
    by scale and space used; no tile is larger than its window, small
    windows are not boosted (P1: never larger than real). Each row is
    centred, then each tile is shifted towards its window's real centre
    as far as its neighbours' gaps allow, which keeps a window on the left
    on the left. Group by application lays each app's windows out as one
    cluster, the clusters by the same rows. Plain functions in
    `mission/layout.rs`, tested at 3440×1440 and 1920×1080 with one to forty
    windows, as `wm::geometry` is.
9. **The windows are drawn by the compositor, the rest by two shell
    surfaces.** B54 says Mission Control takes the switcher's path: windows
    drawn into textures and copied into Skia images. For the spread that
    path does not fit: the tiles are the windows at up to their real size,
    travelling from their real rectangles and back every frame of the
    motion, and twenty of them copied each frame, or re-rendered at each
    size, would miss bar 5; the switcher already had to batch four at a
    time. So the spread is drawn in the compositor's own pass, each window's
    pieces through `place` from its real rectangle to its tile by the
    progress, live and with no copy, as niri draws its overview
    (`mission-control-2026.md` §5). Everything around the windows is Freya,
    in two surfaces per output on the Overlay layer:
    - `mission-back`: the Spaces bar's glass panel and its miniatures, the
      tray's panel and thumbnails, App Exposé's set-apart row;
    - `mission-front`: the icons on the tiles' lower edges, the title
      capsule, the close badges, the selection ring, App Exposé's header,
      the labels of the empty states.
    The compositor draws, back to front: wallpaper, `colour-veil`,
    `mission-back`, the tiles, `mission-front`, the dragged tile. The
    miniatures and tray thumbnails are small and change slowly, so they
    keep B54's path: textures copied into Skia images, a few at a time, at
    the switcher's rate, a parked window's last buffer never woken. B54 is
    revised to say so at the close.
10. **Every screen at once.** Both surfaces are `Scope::EveryOutput` and
    open together; each output's model is its own row and its own windows.
    The arrows move past an edge onto the next screen; a tile dragged onto
    another screen's bar or spread moves the window there. App Exposé's
    set-apart row is on the focused screen only (P1).
11. **Opening is one progress.** Every window travels from its real
    rectangle to its tile on `motion-move`, the veil, bar and tray fade by
    p and the icons by 2p−1; closing is the reverse on `motion-exit`, and
    a desktop chosen closes into that desktop's switch (choice 5). The
    progress is one `Motion`, so a later gesture can hold it (choice 2).
    While open, windows keep committing and
    are drawn live; windows of other desktops stay parked. With a
    full-screen game in front, only Ctrl+↑ and the Mission Control key open
    it, drawn without motion or blur, the pointer constraint released until
    it closes (P1 States). Locking closes it; a system prompt draws over it.
    Opening closes Control Centre, Notification Centre, the bar and its
    menus (B57), Spotlight and the switcher, and hides the dropdown terminal
    when there is one (P20).
12. **What Mission Control does is the registry's, said once.** Choose a
    tile (focus and raise, the switch if needed), close (the window's close
    with the keep-running policy, `wm::apps::close_window`), move to a
    desktop, a new desktop, another screen, reorder and remove desktops,
    exit a full-screen desktop, enter Split View by a drop on a full-screen
    desktop, restore from the tray, Space to look (the selected tile springs
    to its real rectangle on `motion-open`), the window menu on a secondary
    click. Each is a `State` method that `Desktops1`, `Windows1`, the CLI and
    the window menu reach too. Moving a desktop moves its row entry, so the
    saved state becomes the order of desktops and their counts, not counts
    alone.
13. **The two Mission Control settings P1 copies, as a group in Desktop.**
    "Automatically rearrange desktops based on most recent use" (off): a
    desktop left for another moves to just after it in the row, never past
    a full-screen desktop's origin. "Group windows by application" (off).
    Proposed on the gate's board with the shortcuts and mouse buttons.

### Show Desktop

14. **Show Desktop moves the windows, really.** Every shown window on the
    output (keep-on-top included, PiP and the dropdown not) is moved to its
    nearest of the left, right or bottom edge, keeping its size, until 24
    points of it show (`surfaces/window-chrome.md`), through the registry's
    `set_frame` on `motion-move`, its box held to put it back. Moving them
    rather than offsetting the drawing keeps hit-testing, the slivers'
    clicks and X11's stacking true. A click on a sliver, the chord again,
    any window activated or a new window mapped puts every one back. The output remembers that it shows the desktop until
    then; a switch away keeps it, so coming back shows it still.

### The routes

15. **The shortcuts are built.** `mission-control` (Ctrl+↑, XF86LaunchA),
    `app-expose` (Ctrl+↓) and `show-desktop` (Cmd+F3) leave
    `Action::Reserved`. The Mission Control key with Ctrl is App Exposé and
    with Cmd Show Desktop, as P1 proposes (the Launchpad key already opens
    the Apps view, P10). Inside Mission Control, Ctrl+Shift+←/→ and
    Ctrl+Shift+1…9 move the selected window or desktop (P1's proposal).
    Mission Control joins the escape set over a game (B30). The Keyboard
    Shortcuts sheet's Desktops group gains them. Spotlight's actions gain
    Mission Control, App Exposé and Show Desktop.
16. **Mouse buttons as routes.** The shortcut table learns mouse buttons
    (`BTN_SIDE`, `BTN_EXTRA`, `BTN_FORWARD`, `BTN_BACK`, `BTN_TASK`) as
    chords with optional modifiers, matched on press in the pointer path,
    never the primary, secondary or middle button. Mission Control,
    Application Windows and Show Desktop can take one in Settings › Desktop
    › Shortcuts…; none by default (P1 Mouse routes). A bound button is
    withheld from clients as a bound key is.
17. **The hot corners gain Mission Control, Application Windows and
    Desktop.** `Corner` gains the three; bottom left defaults to Mission
    Control (B19, `dock.md`), the top corners stay none for the bar. Firing
    a corner again while its own surface is open closes it (P1). Widgets
    join with P17.
18. **The corner's dwell becomes `timing.corner-dwell-ms`** (250), as the
    surface sheet and the token say; B99 recorded `timing.hover-dwell-ms`
    (120), which P13 built when the corners held nothing a stray pass could
    open by mistake. With Mission Control in the bottom-left corner of an
    ultrawide a stray pass costs a screen-wide motion; the longer rest is
    P1's own answer. B99 revised at the close. On the gate's board as a
    recommendation.
19. **An app can be assigned to a desktop** (`compositor.md` Spaces 4,
    `shell.md` No Dock 5). The Apps view's tile menu and an app's
    Spotlight result gain Options › Assign To: All Desktops, This Desktop,
    None; its new windows open on that desktop of their output, and an
    app on all desktops shows on every one of the output's desktops as a
    keep-on-every-desktop window does (`compositor.md` Stacking 9). Kept in
    the `windows` section by app id.
20. **`Desktops1` and the CLI grow.** `Desktops1` gains `MissionControl`,
    `AppExpose(app)`, `ShowDesktop(output)`, `Reorder(output, from, to)`,
    `Split(window, side)` and `Assign(app, where)`; `isle desktops`
    gains `mission-control`, `expose`, `show-desktop`, `reorder`, `split`
    and `assign`. `Test1` gains `mission()`, the model as JSON, as
    `switcher()` is.

### Settings

21. **Desktop's groups.** Mission Control (choice 13, Shortcuts… with
    choice 16's buttons, Hot Corners… moved into it) and, with Stage
    Manager, its group (choice 26). No Trackpad pane (choice 2). P1's
    "Click wallpaper to reveal desktop" is proposed as Stage Manager's own behaviour with no row,
    since a click on the wallpaper in a stacking desktop does nothing today
    and macOS's "Always" is the setting people turn off.

### Split View

22. **A Split View desktop is a full-screen desktop of two windows.**
    `Kind::FullScreen { window }` becomes `Kind::FullScreen(Panes)`, one
    window or two with the divider's fraction, so every full-screen rule
    (a desktop right of its origin, the bar and island hidden, VRR and
    scan-out for one window, the return at its end) holds for both. Each
    pane's window is `Mode::FullScreen` with its origin and is configured
    to its half with the tiled states, not the full-screen state, so a
    client keeps its own chrome as it would at half the screen.
23. **Entered from the zoom menu, the Window menu, a drop or a chord.**
    The zoom menu's Full screen section gains Left of screen and Right of
    screen; the Window menu's Full screen tile ▸ is enabled (shown disabled
    since P6); dropping a window on a full-screen desktop in Mission
    Control pairs it. Chords: `split-left` and `split-right`, unbound by
    default since the menus and the drop reach it and every free chord is
    someone's; recorded in Keyboard Shortcuts. On the gate's board.
24. **The chooser is Mission Control's spread in the free half.** The
    origin desktop's other windows, laid out by choice 8 in the half under
    the veil, a window whose minimum width does not fit dimmed with
    "Doesn't fit in Split View" and skipped by the keys; none fits: "No
    other windows fit". Escape or a click on the empty half cancels: the
    first window returns to its box and the desktop goes.
25. **The divider is the compositor's.** An 8-point strip drawn as an
    element with its grip, a pointer grab that moves both panes and stops at
    either's minimum, a double click back to halves, the half chords cycling
    the focused pane through ½, ⅔ and ⅓, the Window menu's Resize moving it
    by keys (`window-chrome.md`). With a pane's bar revealed, the zoom menu
    offers Swap sides, Replace window… (the chooser again) and Exit Full
    Screen; a pane's bar dragged onto the other swaps them. Leaving full
    screen for one pane returns it to its origin and leaves the other full
    screen alone. Two panes only.

### Stage Manager

26. **B8's layout trait, with two modules.** `wm::layout::Layout` is what a
    desktop's arrangement answers: where a mapping window goes, what
    activating an app does to the others, the work area, what Mission
    Control groups by. The stacking code moves behind it unchanged as
    `Stacking`; `Stage` is the second. The module is a field of the
    desktop, chosen from the desktop's menu in Mission Control's bar
    (Stage Manager, checked when on) and saved with the desktops. No Control
    Centre module: P1 offered one only "if `control-centre.md`'s gallery
    takes it", and that gallery has none.
27. **Stage keeps sets and parks the rest.** A set is an app's windows and
    whatever the user drags into it; the front set has the work area less
    the column's reserve (192) and keeps the user's arrangement, centred
    only the first time it is staged. The other sets' windows are parked
    (B54: last buffer, not woken). Activating an app, by any route, swaps
    its set in on `motion-move`. "Show windows from an application": all
    at once (default) or one at a time, the most recent, a click again
    cycling.
28. **The column is a shell surface fed images.** A `stage` surface at the
    leading edge, `Scope::EveryOutput` on a Stage desktop only, its entries
    the front window and up to two behind at 160×100 through B54's path,
    redrawn once a second a few at a time and at once when a set changes.
    Hidden by default ("Show recent apps" off), shown by a push of
    `size-edge-push` held `timing.hover-dwell-ms` with the grace after, as
    the top edge's reveal is (`top_edge`). A window dropped on an entry joins
    that set; an entry dragged onto the stage adds its windows to the front
    set. Mission Control on a Stage desktop spreads every window, grouped by
    set.

### Proof

29. **Tested headless, run in the VM, used on the host.** Headless: the
    spread layout at both sizes; the switch with the token's distance and
    full screen growing through it, the existing desktop tests unchanged;
    Mission Control's keys, drags and every action through
    `Test1.mission()`; Show Desktop and its four ways back; Split View's
    entry, chooser, divider, swap and exit; the corners' new actions and
    dwell; mouse-button chords; Stage's sets. Bar 5 measured with Mission
    Control open over twenty live windows on a 3440×1440 headless output.
    The VM: kitty, Firefox, a GTK 4 and a Qt 6 app in the spread, drags
    between desktops and screens, Split View with Firefox and kitty. The
    host, the user's: the key and mouse routes, the corners and Split View
    on the installed package.

## The workspace

```
crates/
  isle-compositor/src/
    mission/              the model, layout, keys, drag, drawing (new)
    wm/desktops.rs        the switch's token, order saved, panes, the
                          layout per desktop, assignment
    wm/show_desktop.rs    (new)
    wm/split.rs           the chooser, the divider (new)
    wm/layout/            the trait, stacking, stage (new)
    hot_corners.rs        three actions, the corner dwell
    binds.rs              the built actions, mouse buttons
    render.rs             the spread between the two surfaces
    dbus.rs               Desktops1, Test1
  isle-shell/src/
    mission/              mission-back, mission-front (new)
    stage/                the column (new)
    zones.rs              Left and Right of screen, Swap, Replace
  isle-keys/              mouse-button chords; the split shortcuts
  isle-settings/          corners, desktop, windows.assign
  isle-state/             desktops: order and layout
  isle-system-settings/   Desktop's groups; shortcuts
  isle-cli/               isle desktops' new commands
data/
  settings/index.kdl      the new rows
  spotlight/actions.kdl   Mission Control, App Exposé, Show Desktop
```

## Phases

Each phase ends with its tests passing headless and a line in
`progress.md`.

### 16.0 · The note and the record

`docs/research/rust-desktop/mission-control-2026.md` (written with this
plan); the roadmap's P16 entry and the deferred gestures packet; choice 4's
corrections to DESIGN.md, `shell.md`, `compositor.md` and `dock.md`, and
the gestures marked deferred there.

### 16.1 · The rows and chords (gate)

Two boards, published as private Artifacts. `p16/rows-1.html`: Desktop's
Mission Control and Stage Manager groups, the Keyboard Shortcuts rows, the
mouse buttons, the Split View chords; the answers go into
`p1/structure/settings.md`. `p16/mission-control-1.html`: the surfaces
themselves, P1's board scenes drawn for the first time with the spread laid
out by choice 8's algorithm, added at the user's ask since wave C drew only
three states.

### 16.2 · The switch

The token's distance and spring; full screen growing through the switch.
The existing desktop tests pass unchanged.

### 16.3 · Mission Control's model

`mission/`: the bars, the spread's layout, the tray, App Exposé's rows,
the keys and selection, the drag's targets; `Test1.mission()`. Pure, tested
at both sizes.

### 16.4 · Mission Control drawn

`mission-back` and `mission-front` on every output, the spread in the
compositor's pass between them, the veil; opening and closing; miniatures
and thumbnails through the preview path; game, lock, prompt, Reduce Motion
and Reduce Transparency states.

### 16.5 · Acting in Mission Control

Choose, close, Space to look, the window menu; drags to desktops, +,
another screen; reordering and removing desktops with the order saved;
group by application; automatic rearranging.

### 16.6 · App Exposé and the tray

App Exposé by Ctrl+↓ and from Mission Control; Tab between apps; the
set-apart row; the tray's minimised and hidden windows and restoring from
it.

### 16.7 · Show Desktop

The move to the edges and back, the four ways back.

### 16.8 · The routes

The three shortcuts built and the key combinations; mouse buttons in the
shortcut table; the corners' three actions, the default and the dwell;
Spotlight's actions; Assign To; `Desktops1` and `isle desktops`.

### 16.9 · Split View

The panes' desktop kind; the zoom menu, Window menu and drop entries; the
chooser; the divider, its keys and double click; swap, replace and exit.

### 16.10 · Settings

Desktop's Mission Control group, Shortcuts… and Hot Corners… with the new
actions; Keyboard Shortcuts' rows; the index.

### 16.11 · The layout trait

`wm::layout` with `Stacking` moved behind it unchanged; the module saved
per desktop. Every window test passes unchanged.

### 16.12 · Stage Manager

`Stage`, sets, parking and the swap; the column surface and its reveal;
drops into sets; Mission Control grouped by set; the desktop's menu and
Desktop's Stage Manager group.

### 16.13 · Measures and the VM

Bar 5 with twenty windows; the VM's run of choice 29; results in
`docs/research/rust-desktop/p16-mission-control.md`.

### 16.14 · The host and close

The user's run on the installed package. ARCHITECTURE gains How Mission
Control draws and How a desktop's layout is chosen; decisions recorded
(B54 and B99 revised, the gestures deferred); `shell.md`'s and
`compositor.md`'s open points answered or marked deferred; the roadmap's
P16; this sheet ends with its outcome.

## Not in P16

Trackpad gestures, pointer-gestures for clients and the Trackpad pane
(choice 2), to their own packet. Within that packet's reach but not
buildable there either: the two-finger swipe in from the trackpad's right
edge for Notification Centre, since libinput's gestures carry no position
and two fingers are scrolling (`mission-control-2026.md` §4). Widgets in the
hot corners (P17). Dragging a window to the top edge to open Mission
Control (P1 declined: the top edge is the fill target). Named desktops (P1
declined). A Control Centre module for Stage Manager. A third pane in Split
View. Spaces shared across displays: each output has its own row (B40).
The dropdown terminal and PiP themselves (P20); Mission Control only leaves
them out.

## Depends on, and feeds

- Needs P3 (surfaces, transitions, glass), P4 (the registry, desktops,
  zones, the zoom and window menus), P5 (the shortcut table, the escape
  set), P6 (the top edge, the bar), P7 (the preview path), P9 (the lock,
  prompts), P10 (Spotlight's actions, the Apps view), P12 (pacing, the
  constraint's release, VRR on full-screen desktops), P13 (the corner
  engine, Settings' Desktop pane and Hot Corners… sheet), P15 (panels under
  the asking app).
- The gestures packet drives the switch's and Mission Control's progress
  with the fingers. P17's Widgets corner and dashboard join the corner
  list; P20's dropdown
  terminal and PiP are left out of the spread by the rules here; P14b's
  import brings v1's desktops' counts.

## Done when

- `cargo test` passes headless with the layout, Mission Control, Show
  Desktop, Split View and Stage tests, and every desktop test from P4 and
  P12 unchanged.
- Mission Control open over twenty live windows on a 3440×1440 output stays
  within bar 5 (content draw ≤2 ms p95, no dropped frame at 144 Hz).
- In the VM, windows move between desktops and screens in Mission Control
  and Split View pairs Firefox and kitty.
- Every Mission Control row of `shell.md`'s and `compositor.md`'s Parity
  maps works.
- On the host, the user opens Mission Control by Ctrl+↑ and the bottom-left
  corner, moves windows between desktops in it, and uses Split View, on
  the installed package.

## Risks

- **Twenty live windows in one pass.** Drawing each window's full texture
  smaller costs sampling, not copies; on the 5070 Ti it should be cheap,
  but a debug build in the VM will not show it. 16.14 measures headless at
  the host's size; the fallback for a window over a size is B54's texture
  at tile size once the opening motion settles.
- **Two surfaces around the compositor's tiles.** A Freya surface has never
  been drawn with compositor content between two of them. The order is the
  compositor's own render list, so it should be a matter of inserting the
  tiles between the two surfaces' elements; 16.5 proves it first with one
  window.
- **Stage Manager's size.** It is a second arrangement of every window
  rule; if it runs long it moves to its own packet (choice 1).

## Reviewed

The user reviewed the plan on 2026-10-26 and took every recommendation: one
packet with Stage Manager last and cuttable to its own packet, Mission
Control's windows drawn by the compositor between two shell surfaces with
the miniatures on B54's path (B54 revised at the close), the corners' dwell
at `timing.corner-dwell-ms` (B99 revised), and a board at 16.1 for the rows
and chords P1 did not lay out. Asked afterwards, the user deferred trackpad
gestures, since the host is a desktop with no trackpad (choice 2): the
gestures, pointer-gestures, the Trackpad pane and the finger-held switch
leave P16 for a packet of their own. Gate 16.1 passed the same day: both
boards (`p16/rows-1.html`, `p16/mission-control-1.html`) taken as drawn.

## Outcome

Built 2026-10-26 in phases 16.0–16.13; 16.14's host run, part of done, is
the user's, with its checklist in `docs/research/rust-desktop/p16-mission-control.md`.
B112–B118 record the decisions, B54 and B99 are revised; ARCHITECTURE
gains How Mission Control draws, and How windows are kept its desktops'
layouts, Split View and Show Desktop; `shell.md`'s and `compositor.md`'s
points are settled or marked for the Trackpad packet. Differences from the
plan:

- **The model is in `isle-shell`** beside the switcher's, not in the
  compositor (choice 7): pure and tested without one, as B53's is.
- **Split View's chooser is a Mission Control mode** (`Mode::Chooser`), so
  the free half reuses the spread, the misfits and the empty capsule.
- **A desktop's saved state** gains which plain desktops Stage Manager
  arranges; the order of desktops is not saved, since windows are not.
- **The Shortcuts… sheet shows each action's keys as text** and edits only
  the mouse button; the keys are recorded in Keyboard Shortcuts, which now
  keeps a button beside a recorded key.
- **Full screen tile is disabled** for a window too wide for half the
  screen, so the menu offers only what will work.

Found and fixed on the way: going full screen and leaving it never
animated (the window moved to a desktop not yet shown before its morph);
icons, rings and badges sat under clipped images, which draw in a layer of
their own.

Not built, carried in `progress.md`: dragging a pane's bar onto the other
to swap (Swap sides does it); the divider's resize pointer; windows dropped
into Stage Manager's sets and entries dragged onto the stage; the column's
once-a-second redraw (entries are redrawn as sets change); a dialog drawn
on its parent's tile; Mission Control's keyboard routes inside the
Shortcuts… sheet (Keyboard Shortcuts edits them).
