# Competition

Batch 1, rewritten for revision 1, with batch 2's shared places folded in;
kept current as later batches add surfaces. What wins each shared place on
screen and each shared input, collected from every sheet's "Shared
places". Where two sheets disagreed, the resolution is here and the sheets
follow it.

At rest each screen shows the wallpaper and the island (brief P1,
`revision-1.md`). Everything else opens on demand: the menu bar from the
top edge or Ctrl+F2, Control Centre from the island, the switcher, Spotlight
and Mission Control from their keys. There is no Dock (`dock.md`).

## Places on screen

| Place | Contenders | Winner |
|---|---|---|
| Top strip, centre | The island; windows | The island. Each screen's work area starts under a strip kept clear for it (v1's top gap, `window-chrome.md` Work area); fill and zones stop there and no title bar is dragged into it. The island grows only downward or within its reserve when peeked or open, over windows (`island.md`). |
| Top edge outside the island's zone | The on-demand menu bar's reveal; hot corners at its ends; a window's fill target during a drag | The pointer pushing the edge reveals the bar (`menu-bar.md`); during a drag nothing reveals and the edge is the fill target (below). |
| The revealed menu bar against the island | App menus on the left; the island at centre | The island keeps its place and is drawn above the bar. The bar carries app menus on the left and nothing on the right; menus that would reach the island's widest compact form plus a guard gap fold into the overflow, so a new activity never moves them (`menu-bar.md`). |
| The revealed menu bar against windows | The bar sliding down over windows' title bars and toolbars | The bar, while revealed. It reserves nothing and moves no window, as macOS's auto-hidden bar (`window-chrome.md`). |
| Below the island, centre | The island's expanded forms, Control Centre (hover), Notification Centre (the clock), the on-demand bar's open menus; window content | All are menu-bar surfaces: one open per screen, and one Control Centre across screens; opening one closes the other. They cover window content while open. Control Centre's gallery in edit mode sits beside the panel at its top and counts as part of it (`control-centre.md`). Where Notification Centre opens is `island.md`'s. |
| Top right of the main screen | Banners; a window's trailing toolbar controls and the right third's title bar; the revealed bar | Banners (`notifications.md`), in the top layer over windows, their top on the strip's lower edge, at most three tall. The revealed bar, no taller than the strip and empty on the right, never touches them. Main screen only (D49); the same place over a full-screen window, where banners are held unless allowed; with a free second screen (no full-screen window on it) they show there instead of being held. Over a game, solid and still. |
| Banners against other surfaces | Menu-bar surfaces, Notification Centre, the overlay, prompts, the lock, the island | A banner waits while a menu-bar surface is open on the main screen, or while Mission Control, the capture tool, the session dialog, the widgets dashboard or a system prompt is open; one that waits past its lifetime goes only to the centre. While Notification Centre is open, arrivals go into it and no banner shows. The switcher and Spotlight sit above banners, which keep showing beneath. None shows over the lock. The island and banners are separate places and may show at once; the island never shows a notification. |
| OSDs | The island; a capsule of their own near the top right (macOS 26's place, under the Sound item) | The island, the only OSD (`island.md`): with no status area there is no Sound item to hang a capsule from. Capture adds two OSD-class events that pre-empt and show over a full-screen window: the countdown, and "Screenshot saved" for a shot over a full-screen game (`capture.md`). |
| Top centre on the lock | The island's privacy form and OSDs; the lock's date and time | Both: the privacy form and OSDs keep their place, the date and time sit below the strip (`lock.md`). |
| Top edge over a full-screen window | The bar's reveal, the island's return, Isle's title bar, the island's privacy form, OSDs | Privacy marks and OSDs show without revealing. A push inside the island's zone brings the island back; outside it, the bar with the island, and Isle's title bar beneath for a non-game window. Over a game either takes a dwell, never while the game holds the pointer, with no motion. |
| Top edge during a window drag | The fill target; the bar's reveal and the island's hover | The drag. The whole top edge, the island's stretch included, is the fill target; the bar does not reveal and the island takes no hover until the drag ends. |
| Top corners | Hot corners; the bar's reveal and its system menu | A push into the edge reveals the bar; a hot corner fires only after a rest in the corner and never while a menu is open or over the lock, and a click in the revealed bar always reaches the system menu. |
| Bottom edge | Windows; a drag zone | Windows: no Dock and no reserve; fill and the lower zones reach the screen's edge. Whether a drag there tiles is `window-chrome.md` For the user 7 (recommended: no zone, as macOS). Nothing reveals from the bottom. |
| Bottom centre | The capture toolbar; windows | The toolbar, above windows, while open (`capture.md`). Nothing else claims it with no Dock. |
| Bottom corners | Hot corners; quarter zones during a drag | Quarter zones while a window is dragged; hot corners otherwise, after a rest, never over a full-screen game, and nothing over the lock. Default corner actions are `dock.md` For the user 3. |
| Bottom-trailing corner of the work area | Capture thumbnail; picture-in-picture; the hot corner | The thumbnail, in the top layer above a PiP window, which it does not move, since it is gone in seconds; whether PiP steps aside is the PiP sheet's (batch 3). Both click areas stop short of the corner point, so clicking them never rests in the hot corner (`spotlight.md`). Banners never go there. |
| Centre of the screen | Switcher, Spotlight (with its Apps grid), Mission Control, the capture tool, the session dialog, Force Quit, the widgets dashboard | One at a time: opening one closes the open one. Cmd+Tab closes any of them and switches. Spotlight and the others close any menu-bar surface; Spotlight sits below the island's strip, so the island and its privacy marks stay visible. The capture's freeze pick (Shift+Cmd+4) closes nothing: it draws over a still frame and gives focus back, so a shot can show any of them. Locking closes Spotlight. |
| Each screen's centre while arranging displays | Settings' display names; other surfaces | The names, in the top layer, only while the pointer is over the arrangement or a drag is in progress; they take no input (`settings.md`). |
| Over everything | System prompts and their scrim; the session dialog's scrim; the lock; the idle dim | The idle dim over all, the lock and a prompt included; it takes no input but the one that ends it. Then the lock, over every layer, system prompts included; only the island's privacy form, its OSDs and the idle dim draw over it. A prompt dims every screen with a scrim above windows and every other surface, and a click on it does nothing; only the privacy form, the lock and the idle dim are above it. A prompt waits as the island's activity behind a full-screen window or game (unless that window's app asked), while locked and once Cmd+Tab has set it aside; one on screen when the lock comes goes back to waiting. The session dialog and Force Quit never open over the lock. |

## Layers

From the bottom: wallpaper (`wallpaper.md`); the empty desktop layer;
windows with their title bars and shadows, the zone preview just under a
dragged window, and in-window menus, pop-up buttons and popovers, drawn as
popups just above their own window and under the top layer
(`conventions.md`); the top layer with the on-demand menu bar, the island
(above the bar where they meet), menus, the zoom and window menus, Control
Centre and its edit gallery, Notification Centre, the island's expanded
forms, banners, the capture thumbnail (above picture-in-picture) and
Settings' display names while arranging; the overlay with the switcher,
Spotlight, Mission Control, the capture tool (toolbar and picks), the
session dialog, Force Quit and the widgets dashboard; system prompts with
their scrim; the lock; the idle dim. The island's privacy form is drawn
above any other program's layer-shell surface, above prompts and over the
lock (`island.md`, `lock.md`). Other programs' layer-shell surfaces take
the layer they ask for, under Isle's prompts and lock.

Opening the on-demand bar, Control Centre or Notification Centre, or any
surface taking focus, closes a window's in-window menus and popovers.

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
6. A sheet, which holds its window's focus; the capture picker is one.
7. The focused window.

## Keys

| Keys | Contenders | Winner |
|---|---|---|
| Cmd+Tab, Cmd+` | Apps' own bindings | The compositor, before Cmd translation; apps never see them. Ctrl+Tab stays the app's. |
| Desktop chords (Ctrl+Option zones; Ctrl+F2, which reveals the menu bar with focus; Ctrl+F8 for the island, as `island.md` places it; Shift+Cmd+/) | Apps' Ctrl+Alt chords | The desktop, on the physical keys; apps keep their chords through Cmd+Option, translated. All go in P5's chord table. |
| Cmd+Space (Spotlight), Ctrl+Cmd+Space (Emoji), the Launchpad key | Apps' own bindings | The desktop, before Cmd translation (`spotlight.md`). Ctrl+Space and Cmd+F stay the app's. Over a game, windowed or full screen, they go to the game. |
| Shift+Cmd+3, 4, 5 (with Ctrl, to the clipboard); Cmd+Ctrl+Escape (stop a recording, cancel a countdown) | Apps' own bindings; a game | The desktop, before Cmd translation, and in the escape set: these are B30's "screenshot" (`capture.md`). On a PC keyboard Print and Shift+Print as well. |
| Shift+Cmd+6 (colour pick) | A game | The desktop; not in the escape set, so a game in front gets it. |
| Ctrl+Option+Cmd+D (Do Not Disturb), Ctrl+Option+Cmd+G (Game Mode) | Apps; a game | The desktop, before Cmd translation (*proposed*, `focus.md`); a game in front gets them, so Game Mode's chord acts only from the desktop. |
| Ctrl+Cmd+Delete (the session chooser); the power key | A game; logind | Ctrl+Cmd+Delete is the desktop's (*proposed*, `prompts.md`) and goes to a game in front. The compositor holds logind's power-key inhibitor, so the power key opens the chooser, over a game too; over the lock it goes to the lock's bottom row. |
| Cmd+Option+Esc (Force Quit) | A game | The desktop; over a full-screen game it is the game's, and Cmd+Tab comes out first (`prompts.md`). |
| Ctrl+F5 | Nothing on the desktop | The app: focus to its toolbar. Not in P5's table (`conventions.md`). |
| Ctrl+F3, Cmd+Option+D | The Dock's chords in batch 1 | Freed: there is no Dock, and both go to the app. |
| Any chord with a game in front | The game | The game, except the escape set (B30: Cmd+Tab, Mission Control, lock, screenshot). |
| Any chord under keyboard-shortcuts-inhibit | The inhibiting client (a VM, a remote desktop) | The client, except one escape chord that survives every inhibitor, games included (`window-chrome.md`). The island's glyph shows while it holds. The escape chord is still `compositor.md`'s to choose; it must not be Cmd+Ctrl+Escape, which stops a recording, or Cmd+Option+Esc. |
| Any chord while locked | Everything | The lock: only media, volume, brightness, layout and the power key act (`lock.md`). |
| Any chord while Settings records a shortcut | Desktop chords and Cmd translation | The recorder, until Escape, a chord or losing focus (`settings.md`). |
| Keys in Control Centre's grid | `island.md`'s Right or Return, as batch 1 had it | Arrows move between modules, Space acts, Return opens a module's page; Right still opens sub-pages and tray menus; E, + and − only with focus in the grid; volume, brightness and media keys keep their meaning (`control-centre.md`; `island.md` follows). |

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

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **A windowed game: should the shell drop transitions and blur, as
   Game Mode does for a full-screen one?** Recommend yes: the shell stays
   visible and works as normal, only cheaper to draw while the game runs.
