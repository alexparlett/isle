# Competition

Batch 1, rewritten for revision 1, with batches 2 and 3's shared places
folded in; kept current as later batches add surfaces. What wins each shared place on
screen and each shared input, collected from every sheet's "Shared
places". Where two sheets disagreed, the resolution is here and the sheets
follow it.

At rest each screen shows the wallpaper and the island (brief P1,
`revision-1.md`). Everything else opens on demand: the menu bar from the
top edge or Ctrl+F2, Control Centre and Notification Centre from the
island, the switcher, Spotlight, Mission Control, the widgets dashboard and
the dropdown terminal from their keys. There is no Dock (`dock.md`).

## Places on screen

| Place | Contenders | Winner |
|---|---|---|
| Top strip, centre | The island; windows | The island. Each screen's work area starts under a strip kept clear for it (v1's top gap, `window-chrome.md` Work area); fill and zones stop there and no title bar is dragged into it. The island grows only downward or within its reserve when peeked or open, over windows (`island.md`). |
| The island while Notification Centre is open | The island's hover; clicks on the pill | The centre. On its screen the pill takes no hover, so a peek never replaces it; a click on the clock closes the centre, a click elsewhere on the pill turns it into Control Centre, pinned (`notification-centre.md`, `island.md`). |
| The top strip while Mission Control is open | The Spaces bar; the island | The island keeps its strip, drawn above the overlay, and the bar starts under it. It shows its rest form, privacy marks and OSDs and takes no hover or click until Mission Control closes. The menu bar does not reveal over it; a top corner set to an action fires after its rest, closing Mission Control first (`mission-control.md`). |
| Top centre, under the strip | The dropdown terminal; the island, its panels and the revealed bar | The top layer draws over the dropdown, which slides down from behind the island's strip, clipped at its lower edge, so it never passes over the island or the bar. The bar is no taller than the strip and never covers the terminal's first line (`dropdown-terminal.md`). |
| Top edge outside the island's zone | The on-demand menu bar's reveal; hot corners at its ends; a window's fill target during a drag | The pointer pushing the edge reveals the bar (`menu-bar.md`); during a drag nothing reveals and the edge is the fill target (below). |
| The revealed menu bar against the island | App menus on the left; the island at centre | The island keeps its place and is drawn above the bar. The bar carries app menus on the left and nothing on the right; menus that would reach the island's widest compact form plus a guard gap fold into the overflow, so a new activity never moves them (`menu-bar.md`). |
| The revealed menu bar against windows | The bar sliding down over windows' title bars and toolbars | The bar, while revealed. It reserves nothing and moves no window, as macOS's auto-hidden bar (`window-chrome.md`). |
| Below the island, centre | The island's expanded forms, Control Centre (hover or click), Notification Centre (the clock), the on-demand bar's open menus; window content | All are menu-bar surfaces in the top layer: one open per screen, opening one closes the other, and one Control Centre and one Notification Centre across all screens. Notification Centre unfolds from the pill under the clock as Control Centre does from the rest of it, and between the two is one morph, never a close and an open (`notification-centre.md`, replacing B24's column at the right edge). They cover window content while open. Control Centre's gallery in edit mode sits beside the panel at its top and counts as part of it (`control-centre.md`). |
| Top right of the main screen | Banners; a window's trailing toolbar controls and the right third's title bar; the revealed bar | Banners (`notifications.md`), in the top layer over windows, their top on the strip's lower edge, at most three tall. The revealed bar, no taller than the strip and empty on the right, never touches them. Main screen only (D49); the same place over a full-screen window, where banners are held unless allowed; with a free second screen (no full-screen window on it) they show there instead of being held. Over a game, solid and still. Banners draw over picture-in-picture, which does not move for them (`pip.md`), and over a full-width dropdown terminal. |
| Banners against other surfaces | Menu-bar surfaces, Notification Centre, the overlay, prompts, the lock, the island | A banner waits while a menu-bar surface is open on the main screen, or while Mission Control, the capture tool, the session dialog, the widgets dashboard or a system prompt is open; one that waits past its lifetime goes only to the centre. While Notification Centre is open on any screen, no banner shows on any screen and arrivals go into it. The switcher and Spotlight sit above banners, which keep showing beneath. None shows over the lock. The island and banners are separate places and may show at once; the island never shows a notification. |
| OSDs | The island; a capsule of their own near the top right (macOS 26's place, under the Sound item) | The island, the only OSD (`island.md`): with no status area there is no Sound item to hang a capsule from. Capture adds two OSD-class events that pre-empt and show over a full-screen window: the countdown, and "Screenshot saved" for a shot over a full-screen game (`capture.md`). |
| Top centre on the lock | The island's privacy form and OSDs; the lock's date and time | Both: the privacy form and OSDs keep their place, the date and time sit below the strip (`lock.md`). |
| Top edge over a full-screen window | The bar's reveal, the island's return, Isle's title bar, the island's privacy form, OSDs | Privacy marks and OSDs show without revealing. A push inside the island's zone brings the island back; outside it, the bar with the island, and Isle's title bar beneath for a non-game window. Over a game either takes a dwell, never while the game holds the pointer, with no motion. |
| Top edge during a window drag | The fill target; the bar's reveal and the island's hover | The drag. The whole top edge, the island's stretch included, is the fill target; the bar does not reveal and the island takes no hover until the drag ends. |
| Top corners | Hot corners; the bar's reveal and its system menu | A push into the edge reveals the bar; a hot corner fires only after a rest in the corner and never while a menu is open or over the lock, and a click in the revealed bar always reaches the system menu. |
| Bottom edge | Windows; a drag zone; the on-screen keyboard | Windows: no Dock and no reserve; fill and the lower zones reach the screen's edge. The on-screen keyboard is the one thing that takes it: while open, the work area ends above it and windows reflow, nothing covered; over a full-screen window it overlays the bottom instead (`keyboard-osk.md`). A drag there gives no zone, as on macOS (`window-chrome.md`, gate 2). Nothing reveals from the bottom. |
| Bottom centre | The capture toolbar; windows; the on-screen keyboard | The toolbar, above windows and above the keyboard, while open (`capture.md`, `keyboard-osk.md`). Nothing else claims it with no Dock. |
| Bottom corners | Hot corners; quarter zones during a drag; Mission Control's minimised tray; the on-screen keyboard | Quarter zones while a window is dragged; hot corners otherwise, after a rest, never over a full-screen game, and nothing over the lock. Mission Control's tray runs along the bottom with its ends short of the corners, and the keyboard's keys stop short of the corner points. The corner actions are Mission Control, Application Windows, Desktop, Notification Centre, Apps, Widgets, Lock Screen and Put Display to Sleep (`mission-control.md`); the defaults stay gate 2's (`dock.md` For the user 3): bottom left Mission Control, bottom right Apps, the top corners none. |
| Bottom-trailing corner of the work area | Capture thumbnail; picture-in-picture; the hot corner | The thumbnail, in the top layer above a PiP window. PiP does not step aside: the thumbnail is gone in seconds (`pip.md`). With the on-screen keyboard open, both sit at the work area's corner above it. Both click areas stop short of the corner point, so clicking them never rests in the hot corner (`spotlight.md`). Banners never go there. |
| Centre of the screen | Switcher, Spotlight (with its Apps grid), Mission Control, the capture tool, the session dialog, Force Quit, the widgets dashboard | One at a time: opening one closes the open one. Cmd+Tab closes any of them and switches. Spotlight and the others close any menu-bar surface; Spotlight sits below the island's strip, so the island and its privacy marks stay visible. The capture's freeze pick (Shift+Cmd+4) closes nothing: it draws over a still frame and gives focus back, so a shot can show any of them. Locking closes Spotlight. Quick Look draws above Spotlight as its child; Spotlight stays open beneath with the keyboard, and Escape closes Quick Look first (`files.md`). Picture-in-picture and the dropdown terminal are not in Mission Control's spread: PiP stays in its corner under the overlay and the dropdown hides; windows kept on top are in the spread like any window. |
| The whole screen under the widgets dashboard | The dashboard; the island, Control Centre, Notification Centre, banners | The dashboard, over its whole screen, strip included. Only the island's privacy form and OSDs draw above it, as on the lock; Control Centre and Notification Centre close when it opens and cannot open over it; other island events wait and are dropped past their duration; banners wait. A system prompt sits above it with its scrim and the dashboard stays open beneath (`widgets.md`). |
| The asking app's window | An Open or Save panel from Files | The panel, as a sheet on that window (a modal child through the portal's parent handle), moving with it; centred over it when it is too small; a centred window when there is no parent; one per parent. It is the asking app's dialog in the switcher, Mission Control and the Window menu, never a Files window, and the on-demand bar shows the app's name with the panel's menus (`files.md`). |
| A long job's progress | The window; the switcher card; the island | The window's jobs button while open and the switcher card always; never the island (`island.md` For the user 3). Its finish shows in the window while Files is in front, else as a notification; its question is a sheet on the job's window with an attention event (`files.md`). |
| Each screen's centre while arranging displays | Settings' display names; other surfaces | The names, in the top layer, only while the pointer is over the arrangement or a drag is in progress; they take no input (`settings.md`). |
| Over everything | System prompts and their scrim; the session dialog's scrim; the on-screen keyboard; the lock; the idle dim | The idle dim over all, the lock and a prompt included; it takes no input but the one that ends it. Then the lock, over every layer, system prompts included; only the island's privacy form, its OSDs and the idle dim draw over it. A prompt dims every screen with a scrim above windows and every other surface, and a click on it does nothing; only the privacy form, the lock and the idle dim are above it. A prompt waits as the island's activity behind a full-screen window or game (unless that window's app asked), while locked and once Cmd+Tab has set it aside; one on screen when the lock comes goes back to waiting. The session dialog and Force Quit never open over the lock. The on-screen keyboard is drawn above the overlay and a prompt's scrim, so it can type into either, and under the lock (which draws its own) and the idle dim; its clicks are never outside clicks (`keyboard-osk.md`). |

Across screens, one menu-bar surface is open at a time: opening Control
Centre on one screen closes Notification Centre on another, and the same
for the revealed bar's menus.

## Layers

From the bottom: wallpaper (`wallpaper.md`); the empty desktop layer; the
windows layer: windows with their title bars and shadows, the zone preview
just under a dragged window, and in-window menus, pop-up buttons and
popovers, drawn as popups just above their own window (`conventions.md`),
then windows kept on top, picture-in-picture, and the dropdown terminal
while shown (`pip.md`, `dropdown-terminal.md`); the top layer with the
on-demand menu bar, the island (above the bar where they meet), menus, the
zoom and window menus, Control Centre and its edit gallery, Notification
Centre, the island's expanded forms, banners, the capture thumbnail (above
picture-in-picture) and Settings' display names while arranging; the
overlay with the switcher, Spotlight and Quick Look above it as its child,
Mission Control (with the island's strip above it), the capture tool
(toolbar and picks), the session dialog, Force Quit and the widgets
dashboard (with the island's privacy form and OSDs above it); system
prompts with their scrim; the on-screen keyboard; the lock; the idle dim.
The island's privacy form is drawn above any other program's layer-shell
surface, above prompts and over the lock (`island.md`, `lock.md`). Other
programs' layer-shell surfaces take the layer they ask for, under Isle's
prompts and lock.

Opening the on-demand bar, Control Centre or Notification Centre, or any
surface taking focus, closes a window's in-window menus and popovers. A
Space switch closes the revealed bar and its menus, since the front app
changes, and in-window menus and popovers with their windows; Control
Centre, Notification Centre and banners stay (`mission-control.md`).

## Keyboard focus

In order, each above the ones after it:

1. The lock. It owns the keyboard: only the media, volume, brightness,
   layout and power keys act, and the escape set does nothing. Locking
   takes the pointer and keys from a game and gives them back at unlock.
2. A system prompt. The menu bar and every other chord do not take focus
   from it; Cmd+Tab sets it aside into the island as a waiting activity,
   lifting the scrim, and switches, and Show brings it back (`prompts.md`).
   Return answers it only once armed, so a key meant for a window or the
   lock never does. It takes focus from a sheet, Spotlight, a pinned
   Control Centre or a banner's reply field (which keeps its text), and
   gives it back on closing.
3. The switcher while Cmd is held.
4. Spotlight, Mission Control, the capture tool, the session dialog, Force
   Quit, the widgets dashboard: one at a time.
5. A menu-bar surface: the on-demand menu bar and its menus, Control
   Centre, Notification Centre, the island's expanded forms, and a banner's
   reply field. It takes focus on demand (Ctrl+F2 reveals the bar with
   focus on its first menu) and gives it back when it closes.
6. A sheet, which holds its window's focus; the capture picker and an
   Open or Save panel on the asking app's window are sheets.
7. The focused window, the dropdown terminal included.

Outside the order: the on-screen keyboard never takes focus and types into
whoever holds it (`keyboard-osk.md`). Quick Look leaves the keyboard with
its caller (a Files window, a panel, Spotlight) unless Markup or Trim is on
or Tab has moved focus into it (`files.md`).

## Keys

| Keys | Contenders | Winner |
|---|---|---|
| Cmd+Tab, Cmd+` | Apps' own bindings | The compositor, before Cmd translation; apps never see them. Ctrl+Tab stays the app's. |
| Desktop chords (Ctrl+Option zones; Ctrl+Option+` for the dropdown terminal, *proposed*; Ctrl+F2, which reveals the menu bar with focus; Ctrl+F8 for the island, as `island.md` places it; Shift+Cmd+/) | Apps' Ctrl+Alt chords | The desktop, on the physical keys; apps keep their chords through Cmd+Option, translated. All go in P5's chord table. Ctrl+Option+` goes to a game in front (`dropdown-terminal.md`). |
| Ctrl+Option+U, I, J, K with picture-in-picture focused | The quarter zones | Corners for a focused PiP window; quarters for any other (`pip.md`). |
| Cmd+Space (Spotlight), Ctrl+Cmd+Space (Emoji), the Launchpad key | Apps' own bindings | The desktop, before Cmd translation (`spotlight.md`). Ctrl+Space and Cmd+F stay the app's. Over a game, windowed or full screen, they go to the game. |
| Shift+Cmd+3, 4, 5 (with Ctrl, to the clipboard); Cmd+Ctrl+Escape (stop a recording, cancel a countdown) | Apps' own bindings; a game | The desktop, before Cmd translation, and in the escape set: these are B30's "screenshot" (`capture.md`). On a PC keyboard Print and Shift+Print as well. |
| Shift+Cmd+6 (colour pick) | A game | The desktop; not in the escape set, so a game in front gets it. |
| Ctrl+Option+Cmd+D (Do Not Disturb), Ctrl+Option+Cmd+G (Game Mode) | Apps; a game | The desktop, before Cmd translation (*proposed*, `focus.md`); a game in front gets them, so Game Mode's chord acts only from the desktop. |
| Ctrl+Option+Cmd+N (Notification Centre), Ctrl+Option+Cmd+W (the widgets dashboard) | Apps; a game | The desktop, before Cmd translation (Ctrl+Option+Cmd+N *proposed*, `notification-centre.md`; Ctrl+Option+Cmd+W decided at gate 2, `widgets.md`); not in the escape set, so a game in front gets them. v1's Super+N and Super+A cannot carry over: Super is Cmd, and Cmd+N and Cmd+A are the app's New and Select All (B21). |
| Cmd+F3 (Show Desktop), Option+Cmd+F5 (the on-screen keyboard) | Apps; a game | The desktop, before Cmd translation (*proposed*, `mission-control.md`, `keyboard-osk.md`); not in the escape set, so a game in front gets them. |
| Ctrl+↑ and the Mission Control key; Ctrl+↓, Ctrl+← / →, Ctrl+1…9 | Apps' Ctrl bindings; Option-translated word motion; a game | The desktop, from the physical Ctrl key only (`mission-control.md`). With a game in front only Ctrl+↑ and the Mission Control key act, as the escape set's Mission Control; the rest go to the game. |
| Ctrl+Shift+← / →, Ctrl+Shift+1…9 | Apps | Mission Control's while it is open (*proposed*), moving the selected window or desktop; the app's otherwise. |
| Ctrl+Cmd+Delete (the session chooser); the power key | A game; logind | Ctrl+Cmd+Delete is the desktop's (*proposed*, `prompts.md`) and goes to a game in front. The compositor holds logind's power-key inhibitor, so the power key opens the chooser, over a game too; over the lock it goes to the lock's bottom row. |
| Cmd+Option+Esc (Force Quit) | A game | The desktop; over a full-screen game it is the game's, and Cmd+Tab comes out first (`prompts.md`). |
| Ctrl+F5 | Nothing on the desktop | The app: focus to its toolbar. Not in P5's table (`conventions.md`). |
| Ctrl+F3, Cmd+Option+D | The Dock's chords in batch 1 | Freed: there is no Dock, and both go to the app. |
| Any chord with a game in front | The game | The game, except the escape set (B30: Cmd+Tab; Mission Control, which is Ctrl+↑ and the Mission Control key only; lock; screenshot). |
| Any chord under keyboard-shortcuts-inhibit | The inhibiting client (a VM, a remote desktop) | The client, except one escape chord that survives every inhibitor, games included (`window-chrome.md`). The island's glyph shows while it holds. The escape chord is still `compositor.md`'s to choose; it must not be Cmd+Ctrl+Escape, which stops a recording, or Cmd+Option+Esc. |
| Any chord while locked | Everything | The lock: only media, volume, brightness, layout and the power key act (`lock.md`). |
| Any chord while Settings records a shortcut | Desktop chords and Cmd translation | The recorder, until Escape, a chord or losing focus (`settings.md`). |
| Keys in Control Centre's grid | `island.md`'s Right or Return, as batch 1 had it | Arrows move between modules, Space acts, Return opens a module's page; Right still opens sub-pages and tray menus; E, + and − only with focus in the grid; volume, brightness and media keys keep their meaning (`control-centre.md`; `island.md` follows). In Notification Centre the arrows, Page Up and Down, Cmd+T and Delete are the centre's only while it has focus. |
| Escape while a Control Centre peek shows and the pointer rests in the island's zone | The focused window | The peek: Escape closes it and disarms hover until the pointer leaves the zone, and that one Escape does not reach the window; the window keeps every other key (`control-centre.md`). |
| ⌘1 to ⌘6 (Monitor's resources, Cleaner's views), ⌘E (Eject in Files), ⌥⌘I (Finder's Inspector in Files), ⌥⌘Q (Quit Process in Monitor) | `conventions.md`'s ⌘1 to ⌘4 for an app's views, ⌘E Use Selection for Find, ⌥⌘I Show Inspector | The app's own: an app's command table may extend or replace the conventions'. None is a desktop chord. |

## Between sheets

Where sheets disagreed, and how it is settled:

- **The island's reserve.** `menu-bar.md` reserved the rest width and let
  the island grow into free width; `island.md` reserved the widest compact
  form. Settled on the widest compact form, so a new activity never moves
  the app menus of the revealed bar.
- **Game Mode and a windowed game.** The brief says the shell stays as
  normal with a windowed game in front; `gaming.md` Game Mode 9 drops
  transitions and background blur for any shell surface during Game Mode,
  windowed games included. Read together: a windowed game leaves the shell
  shown and working as normal, and its surfaces use their solid form and
  open without transitions (For the user 1).
- **The top edge: menu reveal and fill.** The on-demand bar reveals when
  the pointer pushes the top edge outside the island's zone; `window-chrome.md`
  makes the whole top edge the fill target during a drag. Settled: a drag
  in progress suppresses the reveal and the island's hover.
- **Minimise's target.** Batch 1 sent windows into the Dock; with no Dock,
  `window-chrome.md` shrinks them toward the island and `island.md`
  supplies its rectangle without reacting to it.
- **Revealing a hidden Dock.** Falls away with the Dock; `compositor.md`
  Pointer 5 keeps only the top edge's reveal, updated at the gate.
- **Game Mode's name and settings.** The Gaming Focus is "Game Mode"
  wherever the user reads it, a row of Control Centre's Focus page and
  Settings › Focus › Game Mode. VRR and tearing are per output in
  Settings › Displays and apply to any full-screen window, not a Game Mode
  place (`focus.md`).
- **One Focus at a time, and the other holds.** Game Mode over every other
  Focus while a game is in front; a Focus turned on by hand over schedules
  and app triggers; among automatic starts, the latest. A full-screen
  window, a screen share or recording, and the user away also hold
  banners, as situations, not modes: no switch, no glyph, and one summary
  afterwards with a Focus's (`focus.md`, `notifications.md`).
- **A banner and its app's attention.** A banner from an app suppresses
  that app's non-urgent attention event in the island for the banner's
  lifetime; the switcher's mark still holds and an urgent request still
  shows (`notifications.md`, `island.md`).
- **Capture while locked.** A recording or share of a whole screen carries
  the lock screen as drawn, never the session; one of a window holds its
  last frame until unlock, as macOS (`lock.md`, `capture.md`).
- **The safe start.** Announced by the island's configuration-error glyph
  and notice row, with "Use my settings", plus one banner at start; the
  glyph means "settings not in force", a rejected edit or a safe start
  (`prompts.md`, `island.md`).
- **Where Notification Centre opens.** B24 put it in a column at the right
  edge and `island.md` left it open. Settled: from the pill under the
  clock, one morph with Control Centre, one centre across screens
  (`notification-centre.md` For the user 1).
- **Banners while Notification Centre is open.** `notifications.md` said
  "that screen"; settled on any screen: while the centre is open anywhere,
  no banner shows and arrivals go into it.
- **The calendar notification.** `notifications.md` sent "{event} starts
  now" to the dashboard's Calendar widget; its click opens Notification
  Centre's agenda with the event expanded. The Calendar widget stays.
- **Show Desktop and "Show widgets".** `window-chrome.md` shared one window
  move between them and left the rest to `widgets.md`: the dashboard moves
  no window, so Show Desktop is the only window move and reveals the plain
  wallpaper. The dashboard has its own chord, not v1's (`wallpaper.md`).
- **PiP and the capture thumbnail.** PiP does not step aside (`pip.md`).
- **The bottom edge.** Batch 2 gave it to windows with no reserve; the
  on-screen keyboard is the one exception, while open.
- **Files as the front app** with no window focused (`menu-bar.md` For
  the user 1): confirmed by `files.md`.
- **Ending things.** Force Quit ends an app and every process it owns;
  Monitor's Quit Process ends one process; neither opens the other
  (`monitor.md`, `prompts.md`).
- **The Trash.** Files and Cleaner trash and put back through the same
  operations, and Empty Trash is one alert from either; Cleaner's ⌘Z undoes
  only its own last move (`cleaner.md`, `files.md`).
- **Providers.** Their grants are in Settings › Privacy & Security ›
  Providers, read only with "Change…" everywhere else (`widgets.md`);
  Settings adds and removes providers, Keychain signs in to and uses them
  (`keychain.md`).
- **Settings › General › Storage.** Shows each disk as a bar by kind and
  Recommendations that open Cleaner at the view; only Cleaner removes
  anything but the Trash (`cleaner.md` For the user 1, `settings.md`).

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **A windowed game: should the shell drop transitions and blur, as
   Game Mode does for a full-screen one?** Recommend yes: the shell stays
   visible and works as normal, only cheaper to draw while the game runs.
