# P20a · Windows

Planned 2026-11-04, for review.

The window model's last pieces. Keep on top is a stacking flag on the
window's record, set from the Window menu (P6 choice 23); it does not yet
follow the user across desktops and has no shortcut. A video's
picture-in-picture window opens as an ordinary window wherever places put
it. Nothing moves focus by direction. Every Isle app draws its menus inside
its own window (`isle_ui::ContextMenu`), so a menu near an edge is cut or
flipped inward. P15b left four compositor gaps: Quick Look cannot go full
screen, a panel wider than its parent hangs off it, a panel cannot hang
from an X11 window, and a drag ignores Escape and ⌘Tab.

The dropdown terminal is declined (the user, 2026-11-04): no chord, no
Desktop row, no app id of its own. `compositor.md`'s Dropdown terminal and
P1's `dropdown-terminal.md` are retired in 20a.0.

What P20a must get right: a PiP window is placed in its corner before its
first frame, never anywhere else first; a menu opens on the frame after the
press, wherever it falls on the screen, and a press outside it ends it
without reaching the window under it; and nothing kept on top or PiP ever
appears over a full-screen game.

## What it answers to

- Decisions: B8 (stacking and places), B30 (the game in front), B41
  (title bars and menus by windows are shell surfaces), B95
  (`isle_titlebar_v1`), B110 (`isle_float_v1`), B57 (centre surfaces).
- Intent: `compositor.md` Stacking and window places 4 and 9,
  Picture-in-picture 1–4 and its open point; `shell.md` Shell surface
  hosting 3; `apps.md` Conventions (menus).
- P1: `p1/structure/pip.md` and `p1/surfaces/pip.md` (the corners, sizes,
  states, keys and shared places); `p1/structure/window-chrome.md` (the
  Window menu's Keep on top, the shortcut table); `p1/structure/menu-bar.md`
  (Keep on top checked); `p1/surfaces/menus.md` (how a menu is drawn).
- Packets: P5 choice on chords (keep on top and focus by direction have no
  chord by default, the user can bind them); P15b's outcome and
  `docs/research/rust-desktop/p15-files.md` § The spec walk (the compositor
  cuts).
- Evidence read 2026-11-04: Smithay at `1d78177e` serves
  `xdg_toplevel_tag`. winit 0.30 has no `xdg_popup`; Freya's fork opens
  further windows at run time (`launch_window`, which `isle-app`'s hub
  uses), and Isle's apps draw in software unless `FREYA_RENDERER` asks for
  the GPU. The host has Chromium and Vivaldi, whose PiP window is titled
  "Picture in picture", and mpv, which has no PiP window. Firefox is not
  installed.

## Choices this plan makes

### Scope

1. **P20a holds** keep on top finished, picture-in-picture, focus by
   direction, menus as surfaces of their own, and P15b's compositor cuts:
   Quick Look's full screen, a panel's clamp to a narrow parent, a panel
   over an X11 parent, Escape and ⌘Tab during a drag. Not the dropdown
   terminal (declined).

### Keep on top

2. **On top and on every desktop,** as `compositor.md` Stacking 9 and v1's
   pin: the flag also assigns the window to every desktop of its output,
   through the assignment that the Apps grid's "All desktops" already uses.
   Turning it off leaves the window on the current desktop.
3. **Its place among layers.** From the bottom: windows, windows kept on
   top, PiP; all under the top layer and the overlay (P1 `pip.md` Shared
   places). A kept window is hidden while a full-screen game is in front
   (B30), shown over a full-screen app, and not raised over a full-screen
   Space that is not its own.
4. **Mission Control shows it** in the spread of every desktop, with the
   pin badge P1 draws for windows on every desktop; Show Desktop moves it
   aside with the rest.
5. **A `keep-on-top` shortcut,** with no chord by default (P5), toggling
   the focused window. The flag is not saved: a window opens unkept, as
   v1's pin.

### Picture-in-picture

6. **Recognised by title or tag.** "Picture-in-Picture" (Firefox) and
   "Picture in picture" (Chromium and its browsers) as the window's title
   when its first commit is answered, or an `xdg_toplevel_tag` of
   `picture-in-picture`. A title that arrives after the window is mapped
   moves it once into its corner with a fade. mpv and VLC have no PiP
   window; Keep on top serves them (the open point, answered).
7. **Placed at map,** by P1's rules: the corner last used on that screen,
   at its last size; first time the bottom-trailing corner at
   `size-pip-width` (480); inset from the work area so the top corners sit
   under the island's strip and the bottom ones stop short of the hot
   corners' points. A second PiP takes the next free corner, bottom ones
   first, then the last-used one.
8. **Kept in aspect** between `size-pip-min-width` (240) and
   `size-pip-max-scale` (a third of the work area's width, half its
   height), by the compositor's resize grab, with the corner it sits in
   fixed. A new aspect from the player keeps the corner.
9. **Snapping.** A drag ends in the nearest corner, or the one it was
   thrown toward when released fast, by a critically damped move; with
   Cmd held at release it stays where dropped and that place is kept.
   The drag shows the zone preview's shape at the corner, never a zone.
   Dropped on another screen, it snaps to that screen's corners. Players
   that start a move from their body do so through `xdg_toplevel.move`;
   any other, by the Cmd drag (Pointer 3).
10. **Undecorated,** with the system radius, the shadow and a resize band
    from the compositor. Not in the switcher, Mission Control's spread,
    places or session restore. Stays through Show Desktop and a desktop
    switch; on every desktop and over full-screen apps; hidden, still
    playing, while a full-screen game is in front.
11. **The keys** while it has focus: the quarter chords move it to a
    corner; the window menu holds Move to corner ▸, Resize and Close;
    Resize by arrows keeps aspect and corner, Return keeps, Escape undoes.
12. **A `pip` state domain:** the corner, size or free place by output
    identity, as P1's "opens in the corner it last used". When a screen
    goes, its PiP windows take the same corner of the screen with focus.

### Focus by direction

13. **Four shortcuts,** `focus-left`, `focus-right`, `focus-up` and
    `focus-down`, with no chord by default (P5). The next window is the
    nearest whose centre lies within 45° of the direction from the focused
    window's centre, among windows that can be seen on every screen's
    current desktop (kept and PiP ones included, hidden and minimised
    ones not), crossing to the next screen past the edge. It is focused
    and raised, as a click does. Nothing is found, nothing moves.

### Menus

14. **A menu is a window of its own, marked by Isle's protocol.** The
    titlebar manager gains `get_menu` (version 5): `isle_menu_v1` on an
    app's new toplevel, with the parent window, the press point or the
    control's rectangle in the parent's points, and the side it prefers.
    The compositor places it there, flipping and sliding it to fit the
    screen as an `xdg_positioner` would; draws it above its parent and
    below the top layer; gives it no bar, no places and no entry in the
    switcher, Mission Control or restore. Freya draws the menu and gets its
    input through winit, as for any window. Rejected: real `xdg_popup`
    surfaces, which winit cannot make and which would need a winit fork
    beside the Freya one; drawing menus into surfaces `isle-app` makes on
    its own connection, which would need Freya rendering off a window and
    input routed by hand.
15. **The compositor holds the menu's grab.** While a chain of menus is
    open, the keyboard is the newest menu's; the pointer goes to whichever
    menu it is over; a press anywhere else ends the chain with
    `dismissed` and is not delivered. A submenu is a menu whose parent is
    a menu. When the chain ends, focus returns to the parent window in the
    same frame (Shell surface hosting 3).
16. **Glass by the background-effect protocol** the compositor already
    serves, so a menu outside its window blurs what is under it, as the
    in-window menu blurs the window.
17. **Every `isle_ui::Menu` host moves:** context menus, pop-up buttons'
    menus, the toolbar's pull-downs and their submenus, in every Isle app.
    Without the protocol (another compositor, the headless tests) the
    in-window overlay is kept.
18. **Speed decides parking.** 20a.1 measures a press to the menu's first
    frame on screen with a new software window each time. Within a frame
    at the host's 144 Hz, each menu is a new window. Slower, `isle-app`
    keeps one menu window made at the first menu and `isle_menu_v1` gains
    `park`, with which the compositor neither draws it nor gives it input
    until it is placed again.

### P15b's compositor cuts

19. **Quick Look's full screen.** A window over its caller
    (`isle_float_v1`) may go full screen: in place on its screen, over its
    caller and every window, not on a Space of its own, and back over its
    caller when it leaves. Other windows with a parent are still refused.
20. **A panel wider than its parent** is centred on its parent and clamped
    whole onto the parent's screen, rather than hanging from its leading
    edge.
21. **A panel over an X11 window.** The portal's `x11:` parent handles
    resolve to the Xwayland window by its id, so an Open or Save panel
    from a full-screen X11 game floats over it as over a Wayland parent.
22. **Escape cancels a drag** and ⌘Tab carries one: the drag grab ends with
    a cancel to its source on Escape, and the switcher opens and commits
    while the pointer still holds the drag, which goes on into the app
    brought forward.

### Proof

23. **Headless tests** in `isle-compositor`: keep on top's layers, every
    desktop, the full-screen game and Mission Control; PiP recognised by
    each title and the tag, placed at map, the next free corner, aspect,
    floor and ceiling, the snap and the throw, the Cmd drop, another
    screen, a screen leaving, the game in front; focus by direction's
    cone, crossing screens, skipping hidden windows; a menu placed,
    flipped and slid at each screen edge, the chain, a press outside
    dismissed and not delivered, focus back in the frame, submenus;
    Quick Look's full screen, the clamp, an `x11:` parent, a drag's
    Escape and ⌘Tab. `isle-app` tests: a menu through the protocol and the
    fallback without it.
24. **Bars.** A menu's first frame within one frame at 144 Hz of the press
    (choice 18). A PiP window's first frame is in its corner. Nothing kept
    or PiP is in any frame while a full-screen game is in front.
25. **The VM walk** with real input: a Files context menu at each edge of
    the screen and across two screens; a Settings pop-up button near the
    bottom; a submenu; a press outside on another window; Keep on top from
    the menu and the chord, through desktop switches and Mission Control;
    Chromium's PiP dragged, thrown, resized, Cmd-dropped, over a
    full-screen app and under a full-screen game; focus by direction
    across two screens; Quick Look full screen; a Save panel from a
    narrow window; a drag cancelled by Escape and carried by ⌘Tab.

## The workspace

- `data/protocols/isle-titlebar-v1.xml`: `get_menu`, `isle_menu_v1`
  (version 5).
- `isle-compositor`: `wm/pip.rs` (recognition, corners, snapping, the
  domain), `menus/surface.rs` (the protocol, placement, the grab),
  changes in `wm/mod.rs` and `wm/desktops.rs` (keep on top), `wm/focus.rs`
  (by direction), `binds.rs` (the five shortcuts), `handlers/xdg_shell.rs`
  (full screen for floats, the clamp), `portal.rs` (`x11:` handles),
  `grabs.rs` (the drag's Escape and ⌘Tab), `mission.rs` and `switcher.rs`
  (PiP left out, the badge).
- `isle-state`: the `Pip` domain.
- `isle-app`: menu windows through `isle_menu_v1`, parking if choice 18
  needs it.
- `isle-ui`: `Menu` hosted in a window or in the overlay.
- `isle-system-settings`: the five shortcuts in Keyboard Shortcuts' list.

## Phases

### 20a.0 · The note and the record

The roadmap's P20a entry without the dropdown terminal; `compositor.md`'s
Dropdown terminal section and Places 4's mention retired, DESIGN.md's line,
P1's sheet marked declined, the Desktop row dropped from
`p1/structure/settings.md`, the `dropdown-terminal` shortcut taken out of
`isle-settings` and `isle-keys`; `state.md`, `progress.md`, README.

### 20a.1 · The menu spike

A software Freya window marked as a menu in the VM and on the host: the
press to its first frame, with and without a window already made. Decides
choice 18.

### 20a.2 · Menus in the compositor

`isle_menu_v1`: placement with flip and slide, stacking, the grab, dismiss,
focus back, submenus, glass, the left-outs. Tests.

### 20a.3 · Menus in the apps

`isle-app` and `isle-ui`: every menu host through the protocol, the
fallback, parking if needed. Files, Settings, Monitor, Keychain and Cleaner
checked. Tests.

### 20a.4 · Keep on top

Every desktop, the layers, the full-screen game, Mission Control's badge,
Show Desktop, the shortcut. Tests.

### 20a.5 · Picture-in-picture

Recognition, placement at map, corners and the next free one, aspect and
limits, snapping and the throw, the Cmd drop, screens, the game in front,
left out of lists, the keys, the domain. Tests.

### 20a.6 · Focus by direction

The four shortcuts and the cone. Tests.

### 20a.7 · P15b's cuts

Quick Look's full screen, the clamp, `x11:` parents, the drag's Escape and
⌘Tab. Tests.

### 20a.8 · The bars and the walk

Choice 24's bars; choice 25's walk in the VM, the journal read after each
step.

### 20a.9 · The host and close

Package and install. With the user: a day of menus in Files and Settings
on the host's screens; Chromium's or Vivaldi's PiP over work and under a
game; Keep on top. Then the decisions from the next free number,
ARCHITECTURE's How a context menu opens rewritten, this sheet's outcome.

## Not in P20a

The dropdown terminal (declined). Real `xdg_popup` for Isle's apps. PiP
for players without a PiP window. Document PiP windows whose title is the
page's. Keep on top saved across launches. A chord by default for keep on
top or focus by direction. P15b's Files cuts (P20g).

## Depends on, and feeds

- Needs P4 (stacking, places, the resize grab), P6 (the Window menu, the
  menu builder), P7 (the switcher), P12 (the game in front), P15 and P15b
  (Quick Look, the panels, the portal), P16 (Mission Control, Show
  Desktop, full-screen Spaces).
- Feeds P20b (PiP is among restore's left-outs), P20g (the audit reads the decline), P20f (the on-screen keyboard
  sits beside PiP in the bottom corners).

## Done when

- `cargo test` passes headless for choice 23.
- Choice 24's bars hold.
- Choice 25's walk passes in the VM.
- The host run passes with the user.

## Risks

- Chromium may set its PiP title after the first commit, so the window
  maps as an ordinary one and is moved; choice 6's fade covers it, and
  the walk says how often.
- A new window per menu may miss a frame on the host's GPU path
  (`FREYA_RENDERER`); choice 18's parking is the answer, measured first.
- Menus as toplevels depend on winit giving a new window the keyboard at
  once; a menu that opens without it is answered by the grab giving it
  focus from the compositor.
- A browser's Document PiP has the page's title and is not recognised.
