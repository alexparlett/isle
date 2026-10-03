# Menu bar

Batch 1, revised by revision 1. Answers to: `shell.md` Menu bar 1–10 and
App menus 1–9 with their "For the design process" items (status items and
the clock are `island.md`'s, `shell.md` Island 9–11); `apps.md`
Conventions (the menu set); `compositor.md` Shell surface hosting (the
bar's layer); `gaming.md` Game Mode (the top-edge reveal); `platform.md`
Updates (the system menu's restart line). Decisions B18, B20, B21, B24,
B27, B30.

## Purpose

The front app's menus on demand: macOS's auto-hidden menu bar, and at rest
not on screen at all (revision 1, brief P1). It appears over the top of
the screen when the pointer pushes the top edge outside the island's zone,
or from the keyboard, and carries the system's own menu, the front app's
name and menus, Window and Help with menu search (B18). It is how a Mac
user reaches every command of the front app and every system action by
pointer or keyboard. It is not a status area: the clock, status glyphs,
privacy marks and the entry to Control Centre are the island's
(`island.md`); toggles and sliders are Control Centre's; notifications are
Notification Centre's.

## Content

The bar is one band the full width of the screen it is revealed on, with
an edge: it is a place, not text floating over what is behind it
(`design-macos.md` §0 item 4, §3.2: 26's edgeless bar was the fault 27
fixed). It is no taller than the island's reserved strip (`island.md`), so
over ordinary windows it covers only the strip; its menus drop over
windows. Its hit targets run to the screen's top edge, and the first item
to the corner.

**Left, in order**

| Part | What it is | Data from |
|---|---|---|
| System menu | Isle's mark; the system's menu (below) | Isle |
| App menu | The front app's name, set apart from the menus by weight; About, Settings, Hide, Quit (below) | Desktop entry; `gaming.md` Games as apps for a game's name |
| App's menus | The front app's own top-level menus in the app's order | DBusMenu, `org_kde_kwin_appmenu`, `gtk_shell1` and GMenu, X11 properties (B18; `platform.md` Tray and global menu registrar) |
| Overflow » | Only when the app's menus do not fit, where the folded menus would start (Overflow, below) | |
| Window | The desktop's window menu, merged with the app's own Window menu if it has one | Compositor window list and actions |
| Help | Menu search at the top, then the app's own Help items, or Isle's for an app with none | Menu trees; AppStream for help and website links |

**Centre.** The island, in front of the bar and where it always is: the
bar comes down behind it and does not move it (brief P5). The bar reserves
the island's maximum width (`island.md` Overflow) and a guard gap each
side, and lays nothing out inside it; app menus that would reach it fold
into the overflow. Nothing in the bar is ever under the island (`shell.md`
Menu bar 7).

**Right.** Nothing. The bar has no status items, overflow strip, Control
Centre item or clock: those are the island's glyphs, tray and clock
(`island.md`). There is no Spotlight item: Cmd+Space reaches it.

**The system menu**

| Group | Item | Notes |
|---|---|---|
| 1 | About This Computer | Opens Settings › General › About |
| 2 | System Settings… | |
| 2 | Software Update… | Count at the trailing end when updates are waiting; opens Settings › Software Update |
| 2 | Restart to finish updating… | Only while a restart is owed; opens the session dialog's restart with its countdown |
| 3 | Force Quit… | ⌥⌘⎋; the Force Quit dialog (`prompts.md`, batch 2) |
| 4 | Sleep | |
| 4 | Restart… | The session dialog's countdown (`shell.md` System prompts 8) |
| 4 | Shut Down… | Same |
| 5 | Lock Screen | ⌃⌘Q |
| 5 | Log Out {user}… | ⇧⌘Q; the log-out confirmation |

macOS's Recent Items and App Store have no Isle source and are left out.

**The app menu** (every app, real or fallback)

| Group | Item | Notes |
|---|---|---|
| 1 | About {app} | The app's own About when its tree has one, else Isle's About panel: icon, name, version, the package it came from, the entry's description |
| 2 | Settings… | ⌘,; the app's own item when its tree has one, else its settings chord from the Cmd table, else shown disabled so the menu keeps its shape |
| 3 | Desktop-entry actions | Fallback only, and only if the entry has them: New Window, New Private Window, Library and the like |
| 4 | Hide {app} | ⌘H |
| 4 | Hide Others | ⌥⌘H |
| 4 | Show All | |
| 5 | Quit {app} | ⌘Q; kept-running apps quit too (B30) |

An app's own About, Settings and Quit items also stay where the app put
them (File, Edit, Help): Isle points the app menu at them by label and
chord, and never removes an item on a guess.

**The Window menu**

| Group | Item | Notes |
|---|---|---|
| 1 | Minimise | ⌘M |
| 1 | Zoom | |
| 1 | Fill, Centre | Their zone chords (`compositor.md` Snapping 5) |
| 1 | Move and resize ▸ | Halves, thirds, two-thirds, quarters, centred 60%, each with its chord; Return to previous size |
| 1 | Full screen tile ▸ | Split View left and right (`compositor.md` Split View) |
| 1 | Enter Full Screen | ⌃⌘F |
| 2 | Move to desktop ▸ | Each desktop of this screen, New desktop |
| 2 | Move to {display} | Only with a second output |
| 2 | Keep on top | Checked when on (`compositor.md` Stacking 9) |
| 3 | The app's own Window items | Only when the app exports a Window menu |
| 4 | Bring All to Front | |
| 5 | The app's windows | By title; a check on the front one, a diamond on a minimised one |

Groups 1 and 2 are the title bar's window menu too, in this order
(`window-chrome.md`, which adds Move, Resize and Close). They are here as
well because windows that draw their own title bar (libadwaita, Electron)
have no other pointer route to Move to and Keep on top.

**The fallback, as the common case.** Steam, Wine, every game, libadwaita
apps, Electron on Wayland and Firefox by default export nothing (B18;
`macos-model.md` §3 Notes). Their bar is built from what the desktop
knows, and every item in it works:

- the app menu above, with the entry's actions in group 3;
- **File**, **Edit**, **View** only when the per-app Cmd table lists
  commands for them (`shell.md` App menus 4): New Tab, Close Tab, Find,
  Reload and so on, each sending the app's own chord when chosen. File, when
  present, ends with Close Window (⌘W). An entry action the table also lists
  shows once, in the table's menu;
- **Window** as above;
- **Help** with menu search, then "{app} Help" and "{app} Website" where
  the app's AppStream data gives the links.

A game's fallback is the app menu, Window and Help only: closing a game's
window is quitting it, and over a game its chords are its own, so its menus
show no shortcut hints except the escape set (B30).

**A menu's rows.** No icons beside items (brief, Left out). Each row has a
leading mark column, reserved in every menu so labels align; the label; and
a trailing column for the shortcut hint or a submenu's chevron.

| Kind | Structure |
|---|---|
| Normal | Label; hint if any, as the Mac chord the user presses (`shell.md` App menus 7; `copy.md` rule 11) |
| Disabled | Dimmed; no hover highlight; skipped by arrow keys (Help's results excepted); still in the accessibility tree |
| Checked | A check in the mark column; a dash for the mixed state |
| Radio | A check in the mark column, as on macOS; the group is separated |
| Icon | The icon is not drawn; the label shows. An item with an icon and no label shows its accessible name, and only if it has none is the icon drawn, so nothing is blank |
| Submenu | Chevron in the trailing column; opens beside its row |
| Separator | A rule; consecutive and trailing separators collapse |
| Section label | GMenu's section labels as a small heading row, not selectable |

A menu taller than the screen scrolls, with an arrow row at the end that
has more. Labels too long for the menu's maximum width are cut in the
middle.

**Menu search.** A field at the top of Help (`shell.md` proposal). It
searches every item of the front app's menus, real or fallback, and Isle's
Help items. Results list below the field as the item's label with its path
("Edit › Find › Find and Replace"), disabled items included but marked.
Moving onto a result opens the menus down to that item in the bar and marks
it; Return activates it.

**Menu items in Spotlight.** The same index is a Spotlight source
(`shell.md` Spotlight 2; revision 1): the front app's enabled items, each
with its path and hint. Choosing one activates it in the front app without
revealing the bar; focus returns to the app first (shell runtime 11). The
result's layout is `spotlight.md`'s.

**Overflow of app menus.** When the app's menus do not fit between the
left edge and the island's reserve, the rightmost of the app's own menus
fold into one » title placed where they would start, each as a submenu in
order. The system menu, the app menu and Help never fold; Window folds
after the app's own menus. The app name has a maximum width and is cut at
its end past it; it never folds. Menus never continue to the right of the
island, so the right stays empty (revision 1).

## States

| State | What shows |
|---|---|
| Rest | Nothing: the bar is hidden on every screen. The island is at top centre in its strip. |
| Revealed | System menu, the front app's name and menus, Window, Help, on the left; the island in front at the centre; nothing on the right. |
| A menu open | Its title marked; the menu under it. Moving the pointer across titles switches menus without a click. |
| Menus loading | The app's tree is not cached: the fallback titles show at once and the app's own replace them once, when the tree arrives or the timeout (2 s) passes. An open menu whose items arrive takes them in place and keeps the marked row. |
| App not answering | The fallback stays; a submenu whose AboutToShow times out opens with what it had. An item activated in an app that does not answer does nothing visible; the menu has already closed. |
| Empty: no window focused | The front app is Files, as Finder on macOS (For the user 1, which `files.md` confirms): its name and fallback menus, without starting it until one is chosen. |
| Many menus | » on the left (Overflow, above). |
| Second screen | The bar reveals on the screen where the pointer pushed, with the focused app's menus wherever its window is; a menu chosen there acts on that app without moving focus (`shell.md` Menu bar 8). Ctrl+F2 reveals it on the screen of the focused window. Each screen overflows on its own width. |
| Full-screen window | The same reveal as at rest: a push at the top edge outside the island's zone brings the bar down over the content, without resizing it, with the island in front of it and, for a window with Isle's title bar, that bar beneath (`window-chrome.md`). It leaves as at rest. |
| Full-screen game | Revealed only by a dwell at the top edge outside the island's zone, never by a push alone, and never while the game holds the pointer locked or confined (`gaming.md` Game Mode 5). It appears in place with no transition and in its solid form (Game Mode 9), with the island and its live activities in front, and leaves as at rest. Ctrl+F2 is the game's; Cmd+Tab out reaches the bar. |
| Windowed game | As at rest (brief, interview); menus open without transitions and on the solid material while Game Mode is on. |
| Locked | Never shown; the lock owns the screen (`lock.md`, batch 2). |
| Shortcuts inhibited | Ctrl+F2 and Shift+Cmd+/ go to the window until the escape chord; the pointer reveal still works. |
| Update running, restart owed | The system menu's count and restart line; the glyph is the island's. |
| Reduce Motion | The bar and menus appear and leave by fading only. |
| Reduce Transparency | The bar and menus solid. |
| Increase Contrast | A border under the bar and around menus. |
| Largest text size | The bar grows with the text, the strip with it; more folds into the overflow; nothing is clipped. |

## Opening and closing

**Reveal.** The bar comes when:

- the pointer pushes the top edge outside the island's zone: it reaches
  the edge and keeps moving into it past a threshold (stage 4), so a
  pointer that only brushes the edge, or aims at a full-screen browser's
  tabs, does not reveal it;
- over a full-screen game, the pointer rests at the edge for a dwell
  instead (v1's hot zone: 400 ms as the start, inventory §4);
- Ctrl+F2 is pressed: the bar comes with keyboard focus and the system
  menu open (For the user 2);
- Shift+Cmd+/ is pressed: the bar comes with Help open and its search
  field focused.

It never reveals while a window is being dragged (the top edge is the fill
target, `window-chrome.md`), while a mouse button is held, while the
pointer is locked or confined, or over the lock.

**Leave.** The bar leaves when the pointer leaves the bar and its open
menus, after a grace (v1's 250 ms as the start), or when a menu closes
(a choice, Escape, a click elsewhere) with the pointer off the bar. Opened
from the keyboard, it leaves when Escape closes the top level or a choice
is made. Cmd+Tab, Spotlight, Mission Control and a system prompt close any
open menu and the bar with it. Focus returns to the window that had it
before the chosen item's action is delivered (`shell.md` Shell runtime 11).

**Character.** The bar slides down from the top edge over the strip (and
over content in full screen), critically damped, and leaves upward faster
(brief P5). Over a game it appears and leaves without motion. Menus open
in place, fast, and leave faster: a choice closes every level at once. On
a change of front app while revealed, the app name and titles are replaced
where they stand, anchored at the left, with no slide. Values are stage
4's.

## Keyboard

| Key | Action |
|---|---|
| Ctrl+F2 | Reveal the bar with focus, the system menu open; on a full-screen window too. Not over a game |
| Shift+Cmd+/ | Reveal the bar with Help open and its search field focused |
| Left, Right | Move between titles; with a menu open, open the neighbour's. In a menu, Right opens a submenu and Left closes it |
| Up, Down | Move between items, skipping separators and disabled rows, except Help's results |
| Typing | Marks the next item whose label starts with what was typed |
| Return, Space | Activate the marked item; on a title, open it |
| Escape | Close one level; at the top, hide the bar and return focus to the window |
| ⌘, ⌘H ⌥⌘H ⌘Q ⌘M ⌃⌘F ⌥⌘⎋ ⌃⌘Q ⇧⌘Q | The app menu, Window and system menu items, as their hints show, with the bar hidden |

Ctrl+F8, macOS's "move focus to status menus", opens Control Centre from
the island (`island.md`). Over a full-screen game only the escape set
reaches the desktop (B30), so the bar is reached by the dwell or by
Cmd+Tab out of the game. Ctrl+F2 and Shift+Cmd+/ are desktop chords; Isle
takes them before Cmd translation (B21).

## Pointer

| Input | On | Does |
|---|---|---|
| Push | The top edge outside the island's zone | Reveals the bar (dwell over a game) |
| Click | A title | Opens its menu; clicking it again closes it |
| Press, drag, release | Titles into a menu | Release on an item activates it, as on macOS |
| Hover | Titles, with a menu open | Switches menus. With no menu open, hover opens nothing |
| Hover | A submenu row | Opens it after a short dwell; moving diagonally toward an open submenu does not switch it |
| Click | » | Opens the menu overflow. Never on hover (`design-references.md` §1: Ice's hover reveal fires by accident) |
| Leave | The bar and its menus | Hides the bar after the grace |
| Secondary click | Empty bar | Nothing |

Guards on the reveal, as on the island's hover (v1 lessons §3.5): a push,
not a touch; a dwell over a game; a grace before leaving; no reveal during
a drag or with a button held; a peek of the island needs the pointer to
move after a reveal, so one gesture never opens both. While a menu is
open the island takes no hover, so moving across titles never opens
Control Centre (`island.md`).

## Shared places

- **The top edge: the island's zone against the reveal.** The island's
  zone (its shape, the strip above it, a guard gap each side) is the
  island's; the rest of the edge reveals the bar. During a window drag the
  edge is the fill target and neither fires. (`island.md`,
  `window-chrome.md`.)
- **Top centre: the island in front of the bar.** The island does not move
  when the bar comes; the bar reserves its maximum width plus the guard
  gaps, and app menus fold before reaching it. Neither covers the other's
  items.
- **The reserved strip.** The bar comes down into the island's strip, so
  over ordinary windows it covers no content; over a full-screen window it
  covers the top of the content without resizing it.
- **Below the bar: menus, Control Centre, Notification Centre and
  banners.** One top-layer surface open per screen: opening a menu closes
  Control Centre or Notification Centre, and either closes the bar.
  Proposal: a banner that arrives while a menu is open on that screen waits
  until it closes, as v1's island held events while its panel was open
  (`design-v1-lessons.md` §3.5); the notification still reaches
  Notification Centre. Banners' place is `notifications.md`'s (batch 2).
- **Top-left corner: a hot corner against the reveal and the system
  menu.** A hot corner fires after a rest in the corner and never while
  the bar is shown; the bar needs a push, so a rest alone never reveals
  it. Once the bar is shown, a click in the corner reaches the system menu
  (`shell.md` Mission Control 8).
- **The top edge over full screen.** One reveal: the bar with the island
  in front and, for a non-game window with Isle's title bar, that bar
  beneath; over a game the bar and the island only, by dwell, with no
  motion. Privacy marks and OSDs show without a reveal (`island.md`;
  `gaming.md` Game Mode 5, 7).
- **Keyboard focus.** An open menu holds focus; a system prompt, Spotlight,
  Mission Control and Cmd+Tab close it and take focus; the bar never takes
  focus from a prompt.
- **Input.** Ctrl+F2 and Shift+Cmd+/ are the bar's desktop chords, Ctrl+F8
  the island's; all are taken before Cmd translation (B21), given up under
  shortcuts-inhibit, and go in P5's chord table.
- **Window menu.** The bar's Window menu and the title bar's window menu
  carry the same actions (`window-chrome.md`).
- **Menu index.** Menu search and Spotlight's menu-item source read one
  index.
- **Lock.** The bar is never shown over the lock.

## Answers

1. **Order of the bar** (`shell.md`: system menu, app menu, app menus,
   Window, Help left; status items, Control Centre, clock right; island
   centre). **Changed** by revision 1: the left as proposed; nothing on the
   right; status items, Control Centre's entry and the clock are the
   island's (`island.md`); the island stays in front at the centre.
2. **System menu** (About This Computer, System Settings, Updates with
   count, Force Quit, Sleep, Restart, Shut Down, Lock Screen, Log Out, a
   restart-owed line). **Adopted**, grouped as macOS groups the Apple menu;
   the restart-owed line is an action, "Restart to finish updating…", in the
   updates group so Restart stays where it is expected.
3. **App menu** (About, Settings, Hide, Hide Others, Show All, Quit).
   **Adopted**, with the fallback's entry actions as its third group and
   the app's own items pointed at, not moved.
4. **Window menu** (Minimise, Zoom, zones, Bring All to Front, windows).
   **Changed**: adds Fill and Centre, Full screen tile, Enter Full Screen,
   Move to desktop, Move to {display} and Keep on top, shared with the
   title bar's window menu, because apps that draw their own title bar have
   no other pointer route to them.
5. **Menu search** (a field at the top of Help). **Adopted**, opened by
   Shift+Cmd+/, as on macOS; the same index feeds Spotlight (revision 1).
6. **Status items shown by default; reorder and remove; the clock.**
   Moved to `island.md`, as glyphs, the tray and the island's clock.
7. **Menus that do not fit** (one overflow menu). **Adopted** for app
   menus, with the fold rule above. macOS on a notched Mac lets menus
   continue past the notch; here they fold, so the right stays empty and
   Window and Help stay on the left. The status overflow is gone with the
   status items.
8. **An unfocused screen's bar** (inactive). **Changed**: the bar reveals
   on whichever screen the pointer pushes, with the focused app's menus,
   acting on that app without moving focus.
9. **Fallback arrangement** (app menu, Window and Help, entry actions under
   the app menu). **Changed**: File, Edit and View join when the Cmd table
   lists commands, so the fallback teaches the app's keys (`shell.md` App
   menus 4); Help carries AppStream links; a game gets no File and no hints.
10. **How disabled, checked, radio and icon items look.** **Changed** for
    icon items: icons are not drawn (brief, Left out); the rest as above.
11. **`apps.md` Conventions menu set** (app menu with About, Settings…,
    Hide, Hide Others, Quit; then File, Edit, View, Window, Help).
    **Adopted**, with Show All added to match `shell.md` and every app
    menu.
12. **Hiding and showing** (`shell.md` Menu bar 10: hidden over full
    screen, revealed at the top edge, a dwell over a game; "Automatically
    hide and show the menu bar" a setting). **Changed** by revision 1: the
    bar is hidden everywhere at rest and revealed by a push at the top edge
    outside the island's zone, Ctrl+F2 or Shift+Cmd+/; over a game by a
    dwell. The setting is dropped (For the user 3).
13. **The top-edge reveal** (`gaming.md` Game Mode 5). Structure here:
    dwell, in place, no motion, solid, the bar with the island, leaving
    after a grace, never while the pointer is locked or confined. Its look
    is stage 5's.
14. **Ctrl+F2** (`shell.md` Menu bar 9: reveals the bar with its first menu open).
    **Adopted**, and it reveals the bar (For the user 2).
15. **Restart owed** (`platform.md`: a status item and a system menu line).
    The system menu's line here; the glyph is `island.md`'s.
16. **Keyboard-shortcuts-inhibit, Game Mode, Focus and the rejected hand
    edit**, which batch 1 showed as status items: moved to `island.md`.
17. **The bar's layer** (`compositor.md` Shell surface hosting: top).
    **Adopted**: the bar in top, behind the island; its menus above it.

## Copy

| Where | String |
|---|---|
| System menu | "About This Computer", "System Settings…", "Software Update…", "Restart to finish updating…", "Force Quit…", "Sleep", "Restart…", "Shut Down…", "Lock Screen", "Log Out {user}…" |
| App menu | "About {app}", "Settings…", "Hide {app}", "Hide Others", "Show All", "Quit {app}" |
| Window menu | "Minimise", "Zoom", "Fill", "Centre", "Move and resize", "Return to previous size", "Full screen tile", "Enter Full Screen", "Move to desktop", "New desktop", "Move to {display}", "Keep on top", "Bring All to Front" |
| Fallback | "Close Window", "{app} Help", "{app} Website" |
| Help | Search field placeholder: "Search" |
| Isle's About panel | "Version {v}", "From {package}" |

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **What the bar shows when no window has focus.** Recommend Files as the
   front app, as Finder is on macOS: its name and its fallback menus (New
   Window starts it), without starting Files until one is chosen. The
   alternative is keeping the last app's menus, which acts on an app that is
   not in front.
2. **What Ctrl+F2 opens.** The system menu, as macOS and `shell.md` Menu
   bar 12; or the app menu, one key closer to the app's commands.
   *Recommended: the system menu*, as a Mac user expects; Right reaches the
   app menu.
3. **A bar that stays.** macOS keeps "Automatically hide and show the menu
   bar"; with the strip already reserved, a bar shown at all times would
   cost no window space. Keep the setting, on demand by default; or drop
   it. *Recommended: drop it for now*: revision 1 chose the bar on demand,
   and a second arrangement doubles the island's states (in front of a bar
   or alone) for a choice the user has already made.
