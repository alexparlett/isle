# Competition

Batch 1, rewritten for revision 1; kept current as later batches add
surfaces. What wins each shared place on screen and each shared input,
collected from every sheet's "Shared places". Where two sheets disagreed,
the resolution is here and the sheets follow it.

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
| Below the island, centre | The island's expanded forms, Control Centre (hover), Notification Centre (the clock), the on-demand bar's open menus; window content | All are menu-bar surfaces: one open per screen, opening one closes the other. They cover window content while open. Where Notification Centre opens is `island.md`'s. |
| Top right | Banners; a window's trailing toolbar controls and the right third's title bar | No longer a status area: the on-demand bar puts nothing on the right and the island is centred, so the corner is free. Banners' place is `notifications.md`'s (batch 2); this sheet only notes that the top right is open to them. Banners sit above windows. |
| OSDs | The island; a capsule of their own near the top right (macOS 26's place, under the Sound item) | The island, the only OSD (`island.md`): with no status area there is no Sound item to hang a capsule from. |
| Top edge over a full-screen window | The bar's reveal, the island's return, Isle's title bar, the island's privacy form, OSDs | Privacy marks and OSDs show without revealing. A push inside the island's zone brings the island back; outside it, the bar with the island, and Isle's title bar beneath for a non-game window. Over a game either takes a dwell, never while the game holds the pointer, with no motion. |
| Top edge during a window drag | The fill target; the bar's reveal and the island's hover | The drag. The whole top edge, the island's stretch included, is the fill target; the bar does not reveal and the island takes no hover until the drag ends. |
| Top corners | Hot corners; the bar's reveal and its system menu | A push into the edge reveals the bar; a hot corner fires only after a rest in the corner and never while a menu is open, and a click in the revealed bar always reaches the system menu. |
| Bottom edge | Windows; a drag zone | Windows: no Dock and no reserve; fill and the lower zones reach the screen's edge. Whether a drag there tiles is `window-chrome.md` For the user 7 (recommended: no zone, as macOS). Nothing reveals from the bottom. |
| Bottom corners | Hot corners; quarter zones during a drag | Quarter zones while a window is dragged; hot corners otherwise, after a rest. Default corner actions are `dock.md` For the user 3. |
| Bottom-trailing corner of the work area | Capture thumbnail; picture-in-picture | Both inside the work area, which now reaches the bottom edge; their order is batch 2 and 3's (capture, PiP). |
| Centre of the screen | Switcher, Spotlight (with its Apps grid), Mission Control, the capture tool, the session dialog, the widgets dashboard | One at a time: opening one closes the open one. Cmd+Tab closes any of them and switches. |
| Over everything | System prompts; the lock | The lock over all. A prompt over every surface but the lock, the island's privacy form excepted. |

## Layers

From the bottom: wallpaper (`wallpaper.md`); the empty desktop layer;
windows with their title bars and shadows, and the zone preview just under
a dragged window; the top layer with the on-demand menu bar, the island
(above the bar where they meet), menus, the zoom and window menus, Control
Centre, Notification Centre, the island's expanded forms and banners; the
overlay with the switcher, Spotlight, Mission Control and the capture tool;
system prompts; the lock. The island's privacy form is drawn above any
other program's layer-shell surface (`island.md`). Other programs'
layer-shell surfaces take the layer they ask for, under Isle's prompts and
lock.

## Keyboard focus

In order, each above the ones after it:

1. The lock.
2. A system prompt. Cmd+Tab and the menu bar do not take focus from it.
3. The switcher while Cmd is held.
4. Spotlight, Mission Control, the capture tool, the session dialog, the
   widgets dashboard: one at a time.
5. A menu-bar surface: the on-demand menu bar and its menus, Control
   Centre, Notification Centre, the island's expanded forms. It takes focus
   on demand (Ctrl+F2 reveals the bar with focus on its first menu) and
   gives it back when it closes.
6. A sheet, which holds its window's focus.
7. The focused window.

## Keys

| Keys | Contenders | Winner |
|---|---|---|
| Cmd+Tab, Cmd+` | Apps' own bindings | The compositor, before Cmd translation; apps never see them. Ctrl+Tab stays the app's. |
| Desktop chords (Ctrl+Option zones; Ctrl+F2, which reveals the menu bar with focus; Ctrl+F8 for the island, as `island.md` places it; Shift+Cmd+/) | Apps' Ctrl+Alt chords | The desktop, on the physical keys; apps keep their chords through Cmd+Option, translated. All go in P5's chord table. |
| Ctrl+F3, Cmd+Option+D | The Dock's chords in batch 1 | Freed: there is no Dock, and both go to the app. |
| Any chord with a game in front | The game | The game, except the escape set (B30: Cmd+Tab, Mission Control, lock, screenshot). |
| Any chord under keyboard-shortcuts-inhibit | The inhibiting client (a VM, a remote desktop) | The client, except one escape chord that survives every inhibitor, games included (`window-chrome.md`). The island's glyph shows while it holds. |

## Between sheets

Where batch 1's sheets disagreed, and how it is settled:

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

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **A windowed game: should the shell drop transitions and blur, as
   Game Mode does for a full-screen one?** Recommend yes: the shell stays
   visible and works as normal, only cheaper to draw while the game runs.
