# No Dock

Batch 1, revision 1. Answers to: `shell.md` § For the design process
(Dock; Shell runtime, the Dock's part of which screens carry what);
`gaming.md` § For the design process (Games as apps, the Dock's part);
`platform.md` § For the design process (Notifications and badges, the
Dock's part); `compositor.md` § For the design process (Stacking, the
minimise and restore part; Shell surface hosting, the Dock's layer). B19,
B20, revised at the gate.

## Purpose

Isle has no Dock (`revision-1.md`). At rest the screen is the wallpaper
and the island (brief P1), as v1 was. This sheet records where each duty
batch 1's Dock had now lives, so nothing B19 asked for is lost. Every
duty moves to a surface that already exists and opens on demand: the
switcher, Spotlight's Apps grid, App Exposé and Mission Control, the
on-demand menu bar, Files and Settings. Nothing new is drawn at rest to
replace it.

## Where each duty goes

| Dock duty | New home | Owner |
|---|---|---|
| Launching an app | Spotlight (Cmd+Space, type the name) and its Apps grid (Cmd+Space then Cmd+1, the Launchpad key, a hot corner), as macOS 26 replaced Launchpad (`design-macos.md` § 3.3, 3.12) | `spotlight.md`, batch 2 |
| Pinned apps | No pins. Spotlight ranks apps by launch count and recency (`shell.md` Spotlight 4); a row of favourites at the top of the Apps grid is For the user 1 | `spotlight.md` |
| Launch feedback | The pointer's starting form, from the xdg-activation token or startup notification, until the first window maps or the launch times out. No bouncing icon and no island event | compositor (cursor) |
| Running indicator | The switcher: every app with a window is an entry; hidden and minimised-only apps carry their marks. The on-demand menu bar names the front app | `switcher.md` |
| Running in background after quit | The switcher, at the end of the row with a background mark; Q there stops it (`switcher.md` For the user 3) | `switcher.md` |
| Badges | The switcher's cards, on the icon's top trailing corner; also the app's tile in the Apps grid, which `spotlight.md` confirms | `switcher.md`, `spotlight.md` |
| Progress | The switcher's cards. Whether a long job's progress also shows in the island as a live activity is `island.md`'s; its finish is a notification either way | `switcher.md`, `island.md` |
| Attention | A held mark on the app's switcher card, and an island event naming the app, whose form and hold are `island.md`'s | `switcher.md`, `island.md` |
| Minimised windows | App Exposé (Ctrl+↓), where they are set apart (`shell.md` Mission Control 5); in Mission Control, a tray of the current desktop's minimised windows, which is v1's hidden-window tray back again now that no Dock holds them (`design-v1-lessons.md` § 5 Mission Control); in the switcher, the minimised mark and the window row; in the Window menu by title | `mission-control.md`, batch 3; `switcher.md`; `menu-bar.md` |
| Minimise and restore motion | The window shrinks toward the island and restores from there or from its tile in App Exposé or Mission Control | `window-chrome.md` |
| The app's windows | The switcher's window row (↓ or `), App Exposé, the Window menu | `switcher.md`, `menu-bar.md` |
| Desktop-entry actions and the LauncherEntry quicklist | Spotlight: listed under the app's result and in its tile's menu in the Apps grid | `spotlight.md` |
| Hide, Hide others, Quit, Force quit | The switcher's held-app keys (H, Q), the app menu (Cmd+H, Cmd+Option+H, Cmd+Q), the Force Quit dialog (Cmd+Option+Esc) | `switcher.md`, `menu-bar.md` |
| Stop running in background | Q on the app's background entry in the switcher; the Force Quit dialog lists background apps too | `switcher.md` |
| Show all windows | App Exposé (Ctrl+↓), or ↓ in the switcher | `mission-control.md`, `switcher.md` |
| Open at login, Show in Files, Assign to desktop | The app tile's menu in the Apps grid; Open at login also in Settings › General › Login items | `spotlight.md`, Settings (batch 2) |
| Keep running after last window (the B20 policy) | Settings › Apps, per app; and the app's menu in the on-demand menu bar, as an item Isle adds, if `menu-bar.md` takes it | Settings, `menu-bar.md` |
| Dropping files on an app | Files' Open With; or Cmd+Tab while dragging brings the app forward and the drop lands in its window (`switcher.md`, During a drag) | `files.md`, batch 3; `switcher.md` |
| Trash | Files: the sidebar's Trash, Cmd+Delete, Empty Trash, ejecting volumes. Spotlight finds "Trash". The trash sound plays from Files | `files.md` |
| Games' cover art | The switcher (icon and name), Spotlight's results and Apps grid, Mission Control | `switcher.md`, `spotlight.md`, `mission-control.md` |
| Recent apps | Spotlight's recency ranking | `spotlight.md` |
| Stacks (Downloads) | Not built; Files covers them | |
| A Mission Control item | Ctrl+↑, the Mission Control key, a hot corner, a three-finger swipe | `mission-control.md` |
| Position, size, magnification, auto-hide, indicators settings | Gone | |
| The Dock's layer and screens | Gone; the island and the on-demand menu bar are on every screen (`island.md`, `menu-bar.md`) | |

What goes with it:

- **The bottom edge** belongs to windows. The work area reaches the
  screen's bottom edge; whether it gives a drag zone is `window-chrome.md`
  For the user 7.
- **Ctrl+F3** (move focus to the Dock) and **Cmd+Option+D** (turn hiding
  on or off) are unbound and go back to the app.
- **"Minimise windows into application icon"** has no meaning and is not a
  setting.
- **A pointer route to switching.** With no Dock, switching apps by
  pointer alone goes through Mission Control: a hot corner, a gesture, or
  the Mission Control key. Which corners carry what by default is For the
  user 3.

## Answers

- **Order of sections, Apps and Mission Control items, default position
  and size, the app menu's grouping** (`shell.md` Dock). **Replaced**:
  there is no Dock; the app menu's items are spread as the table says.
- **How attention is shown** (proposal: one bounce, or until clicked if
  urgent). **Replaced**: a held mark on the switcher card and an island
  event. Nothing bounces (brief P5).
- **Badges and progress** (`platform.md`, how they look in the Dock).
  **Replaced**: on the switcher's cards, with badges also on the Apps grid's
  tiles. Badges and progress stay kept per desktop entry (`platform.md`
  Notifications and badges 6); only their place changes.
- **Games** (`gaming.md`, the Dock's part). **Replaced**: the cover stands
  in for the icon in the switcher, Spotlight and Mission Control.
- **Running in the background after quit** (`shell.md` Dock, open).
  Moved to `switcher.md`.
- **Which screens carry the Dock** (`shell.md` Shell runtime). Falls away.
- **Minimise and restore choreography** (`compositor.md` Stacking, the
  Dock's part). Moved to `window-chrome.md`: toward the island.
- **Layer** (`compositor.md` Shell surface hosting, the Dock's part).
  Falls away.

`shell.md` § Dock, `compositor.md` Stacking 7 and Minimise 1, 4, `gaming.md`
and `platform.md`'s Dock lines, and B19 are revised at the gate to match.

## Copy

Strings that move with their duties, in the rules of `copy.md`:

- "Running in background" and "Stop running in background": the switcher.
- "Open at login", "Show in Files", "Assign to" with "All desktops",
  "This desktop", "None": the Apps grid's tile menu.
- "Keep running after last window": Settings › Apps, and the app menu if
  `menu-bar.md` takes it.
- "Empty Trash…", "Eject": Files.

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Favourites in the Apps grid.** (a) A row of favourites at the top of
   Spotlight's Apps grid, in the user's order, added from a tile's menu or
   by dragging a tile into the row; the rest stays alphabetical with
   categories; (b) no arranging at all, as macOS 26 and 27. **Recommended:
   (a).** It keeps the one thing the Dock's pins gave, a short list in the
   user's order, without a surface at rest, and answers the complaint that
   the Apps view cannot be arranged (`design-macos.md` § 3.12, brief
   Takeaways). Folders and pages, as Launchpad had, stay out.
2. **Badges at rest.** With no Dock, nothing at rest shows an app's count.
   (a) Nothing does: the notification is the signal at rest, and counts
   show when the switcher or Apps grid is open; (b) an island glyph while
   any app has a badge. **Recommended: (a).** A count that is always
   somewhere is the noise the user dislikes (brief P1), and the island's
   glyphs are for the system's state, not apps'.
3. **Default hot corners.** (a) None, as macOS ships; (b) bottom left
   Mission Control and bottom right Spotlight's Apps grid, so a pointer
   alone can switch and launch. **Recommended: (b).** The bottom corners
   were the Dock's neighbours and are now empty; a corner fires only after
   a rest and never over a full-screen game (`shell.md` Mission Control 8),
   and the top corners stay clear for the on-demand menu bar.
