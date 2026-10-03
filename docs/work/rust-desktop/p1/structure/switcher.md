# App switcher

Batch 1, revised by revision 1. Answers to: `shell.md` § For the design
process (App switcher); `gaming.md` § For the design process (Games as
apps, the switcher's part); `platform.md` § For the design process
(Notifications and badges, the part the Dock had); `compositor.md` § For
the design process (Shell surface hosting, the switcher's layer). B13,
B17, B19, B20, B21, B30.

## Purpose

Cmd+Tab moves between apps, most recent first, and commits when Cmd is
released; Cmd+` moves between the front app's windows (B20). While Cmd is
held the switcher is also where an app is quit, hidden or has a window
closed without being brought forward. With no Dock (`dock.md`), it is
also where running apps are seen: which run, which are hidden, minimised
or running in the background, and their badges, progress and requests for
attention. It speaks in apps: it is not a window picker across every app
(Mission Control and App Exposé are), not a launcher (Spotlight is), and a
quick Cmd+Tab draws nothing at all.

## Content

A row of entries, one per app, centred on the screen it opens on:

| Part | What | Data |
|---|---|---|
| Entry | A card holding a live preview of the app's most recent window, with the app's icon over its lower edge | compositor's app list in recency order, window textures drawn only while shown (B13, `compositor.md` § Apps, hide, quit 2) |
| Window count | Beside the icon when the app has more than one window: all its windows on every Space, minimised and hidden included | compositor's window list |
| State mark | On an app that is hidden, whose only windows are minimised, that is not responding, or that runs in the background with no window | compositor |
| Badge | A count at the icon's top trailing corner; "999+" past three digits. A status mark, not the accent (brief P6) | `com.canonical.Unity.LauncherEntry` `count` with `count-visible`, kept per desktop entry (`platform.md` Notifications and badges 6) |
| Progress | A bar along the card's lower edge; no indeterminate form, since LauncherEntry has none | LauncherEntry `progress` with `progress-visible` |
| Attention mark | A still mark on the card while the app asks for attention; a stronger still form when urgent | an xdg-activation request without a valid token (`compositor.md` § Apps, hide, quit 11), the X11 urgency hint and `_NET_WM_STATE_DEMANDS_ATTENTION`, LauncherEntry's `urgent` |
| Name | The selected app's name, once, under the row (macOS) | desktop entry, or the game's name (`gaming.md` § Games as apps) |
| Window row | Opened into a selected app: one preview per window in the stable order Cmd+` walks, the selected window's title under it; minimised windows marked | compositor's window list |

- **Which apps.** Every app with at least one window, on any Space or
  screen, hidden and minimised windows included (`shell.md` App switcher
  5). Keep-running apps hidden on their last close (B20) still have their
  window and are listed. An app still launching joins when its first
  window maps.
- **Background apps.** An app with a live process and no window (running
  in the background after quit, macOS 27, `design-macos.md` § 3.3) is
  listed after every app with a window, past a gap, with the background
  mark and its icon in place of a preview (For the user 3). The compositor
  knows an app's processes from its clients and from what Isle launched;
  one started elsewhere is known only once it has had a window.
- **Previews.** Live while the switcher is drawn and never otherwise (v1
  D55). A preview keeps its window's aspect inside a fixed card, so an
  ultrawide window and a tall one sit in the same slot. Before a texture
  arrives the card shows the icon alone. A minimised-only app's card shows
  its minimised window's texture (`compositor.md` § Minimise 4).
- **Games** show their cover art as the icon and a live frame as the
  preview (Answers).
- **Order** does not change for badges or attention: an app asking for
  attention keeps its place and carries the mark, so a quick Cmd+Tab still
  goes to the previous app.

## States

| State | What shows |
|---|---|
| Rest | Nothing. The switcher exists only while Cmd is held past the delay. |
| Quick tap | Nothing drawn; the previous app comes forward (`shell.md` App switcher 1, acceptance bar 7). Never a background entry. |
| Held | The row, the second entry selected (Cmd+Tab) or the last app with a window (Shift+Cmd+Tab). |
| Window row open | The selected app's windows under its card; Q, W and H act on the window row's app. |
| One app | Its entry alone, with any background entries after it; a quick tap does nothing. |
| No apps with a window | A quick tap does nothing; held, the row shows only the background entries, or nothing if there are none. |
| Badges, progress | On the cards that have them; nothing else changes. |
| Attention | The mark on that card, held until the app is activated; the island's event is `island.md`'s. |
| Loading | Cards with icons appear at once; previews, badges and progress fade in as their sources answer. The selection never waits on a preview. |
| Error | A window whose texture cannot be had (a protected surface) shows the icon in its card. An app with no LauncherEntry simply has no badge. |
| App quitting (after Q) | Its entry stays until the app has gone, then closes its slot; the selection moves to the next entry. An app that asks to save keeps its entry, and its question is waiting when the switcher closes. An app that quits to the background moves to the background entries. |
| Not responding | Marked; Q still sends quit; Force Quit is Cmd+Option+Esc. |
| During a drag | Cmd+Tab works while a file or text is being dragged: committing brings the app forward and the drag carries on into its window, as macOS's (it replaces the Dock's spring loading). |
| Many apps | Cards shrink to fit down to a floor, then the row wraps into a second and further rows, centred, so every entry stays visible and reachable. Selection wraps from the last entry to the first and back. |
| Second screen | Opens on the screen with the focused window; lists apps per the answer under For the user. |
| Windowed game in front | As normal (brief, interview). |
| Full-screen game in front | Cmd+Tab still works (B30's escape set, `compositor.md` § Shortcuts and keybinds 7). A quick tap leaves the game's Space without drawing anything. Held, the switcher draws over the game without transitions or blur (`gaming.md` Game Mode 9); keys go to it, not the game, until release; Escape gives the game back its focus and its pointer lock (`gaming.md` § Full-screen Spaces 4). On a PC keyboard the Alt and Super swap is suspended in a game (B30), so Cmd+Tab is the key labelled Windows with Tab; Alt+Tab goes to the game. |
| Full-screen app in front | As normal; committing to another app leaves the full-screen Space. |
| Locked | Not available; the lock owns the keyboard. |
| Focus on | Unchanged. Badges and marks are silent, so they stay. |
| Reduce Motion | Appears and goes without movement; selection moves without sliding; the window row opens in place. |
| Reduce Transparency | The panel in its solid form (brief P4). |
| Increase Contrast | The panel gets an edge; the selection, the state marks and the attention mark take their stronger forms. |
| Largest text size | The name, window titles and badge text scale; cards do not. |

## Opening and closing

- **Cmd+Tab** switches at once to the previous app. Only if Cmd is still
  held after a short delay does the switcher draw, on the screen with the
  focused window (`shell.md` App switcher 1). A second Tab inside the
  delay draws it at once, so a fast double Tab never selects blind. The
  delay is stage 4's.
- **Appearing** is a quick fade in place, no growth or slide (brief P5).
- **Release of Cmd commits**, whatever was pressed in between
  (`compositor.md` § Shortcuts and keybinds 5): the switcher goes at once,
  faster than it came, and the selected app comes forward with its most
  recent window, or the window chosen in the window row, unhidden and
  restored if needed (`shell.md` App switcher 4); a restored window grows
  from the island (`window-chrome.md`). If it has no window on the current
  Space the view moves to one where it does (`compositor.md` § Apps, hide,
  quit 3). Committing to a background entry launches the app again
  through its desktop entry, which asks a single-instance app to show its
  window.
- **Escape cancels**: the switcher goes and the app that was in front
  keeps focus. Quits, hides and closes already done stand.
- Modifiers held through the switch are released cleanly in both apps
  (B21, `shell.md` App switcher 8).

## Keyboard

| Action | Key |
|---|---|
| Switch to the previous app, or open and move right | Cmd+Tab |
| Open from the end, or move left | Shift+Cmd+Tab |
| Move between entries while held | Tab and Shift+Tab, ← and → |
| Open the selected app's window row | ↓ |
| Leave the window row | ↑ |
| Step through the selected app's windows | ` and Shift+` (the key above Tab, whatever it is labelled); opens the window row |
| Quit the selected app (keep-running apps too, B30); on a background entry, stop it running in the background (a polite signal by process, `compositor.md` § Apps, hide, quit 7) | Q |
| Hide or show the selected app | H |
| Close the selected app's front window, or the selected window in the window row | W (the keep-running close path, `compositor.md` § Apps, hide, quit 8) |
| Commit without releasing | Return |
| Cancel | Escape |
| Commit | Release Cmd |
| Cycle the front app's windows without the switcher | Cmd+` and Shift+Cmd+`, in a stable order, skipping minimised windows (`shell.md` App switcher 6) |

H and W do nothing on a background entry. Focus: while held, the switcher
takes every key; the window in front keeps its focus until commit, and a
cancel leaves it untouched.

## Pointer

- **Hover** selects an entry, or a window in the window row, but only when
  the pointer moves, so a pointer resting where the switcher appears never
  takes the selection (`design-v1-lessons.md` § 3.5).
- **Click** commits that entry or window at once, Cmd still held or not.
- A click outside the switcher does nothing; nothing closes it but
  release, Return, a click on an entry, or Escape.
- No secondary click, drag or scroll.

## Shared places

| Contender | Proposal |
|---|---|
| Centre of the screen | The switcher sits in the overlay layer, above windows, the island, the on-demand menu bar and banners, centred on the focused screen; Spotlight and Mission Control are never open with it (below). |
| Spotlight, Mission Control, the capture tool, the session dialog | Cmd+Tab closes the open one and switches (`shell.md` Shell runtime 10). Inside Mission Control it could instead move the highlight; closing is simpler and matches the one-at-a-time rule. |
| The island's attention event | Both show: the island announces once, the card holds the mark until the app is activated. Activating the app clears both. |
| A drag in progress | Cmd+Tab works during it and the drag survives the commit. |
| A system prompt holding focus | Cmd+Tab sets the prompt aside into the island as a waiting activity, lifting the scrim, and switches; Show brings it back (`prompts.md`). No other chord takes focus from it. |
| Lock | The lock wins; no switcher. |
| A full-screen game | Cmd+Tab wins: it is in the escape set (B30). |
| Keyboard-shortcuts-inhibit (a VM or remote desktop) | Cmd+Tab goes to the inhibiting client; `compositor.md` § Keyboard-shortcuts-inhibit owns how inhibit is left. |
| Apps' own Cmd+Tab and Cmd+` | The compositor takes both before translation (`compositor.md` § Shortcuts and keybinds 3); apps never see them. Ctrl+Tab stays the app's. |

## Answers

- **What an entry shows** (proposal: icon, window count when over one,
  live preview of the most recent window). **Adopted**, pending For the
  user: a card with the live preview, the icon over its lower edge, the
  count beside the icon; changed in that the name shows once, for the
  selected app, under the row (macOS), rather than on every card, and the
  card also carries the state marks, the badge, progress and the attention
  mark the Dock carried (revision 1).
- **Order.** Most recent app first (`compositor.md` § Apps, hide, quit 2),
  the front app at the start and the selection opening on the second;
  background entries last.
- **Into an app's windows** (GNOME's arrow, `design-references.md` § 4).
  **Adopted**: ↓, or `, opens a row of the app's windows under its card
  in the stable order, so Cmd+` and the switcher show the same thing in
  the same order (v1 D59). This replaces macOS's hand-off to App Exposé
  and keeps commit on release. In macOS's switcher ` moves the selection
  back; Isle keeps ` for windows (`shell.md` App switcher 3) and leaves
  moving back to Shift+Tab.
- **Q, W, H on the held app** (`design-v1-lessons.md` § 5 Switcher).
  **Adopted**, with W closing the selected window when the window row is
  open, and Q stopping a background app.
- **Apps with no visible window.** Hidden and keep-running apps are listed
  with their hidden window's preview and the hidden mark; minimised-only
  apps with the minimised window's preview and its mark, committing
  restores the most recent one; apps running in the background with no
  window are listed last with the background mark (For the user 3).
- **Badges and progress** (`platform.md`, the Dock's part, moved here).
  A count at the icon's top trailing corner and a bar along the card's
  lower edge, from LauncherEntry. Their look is stage 4's.
- **Attention** (`shell.md` Dock 3, moved here). A still mark held until
  the app is activated, stronger when urgent, with an island event
  (`island.md`). No bounce (brief P5).
- **Games** (`gaming.md` Games as apps, proposal: its cover).
  **Adopted**: the cover stands in for the icon, the live frame is the
  preview, the game's name is the name, and the game is one entry however
  many windows it has (`gaming.md` § Games as apps 4).
- **Many apps.** Shrink to a floor, then wrap into rows (v1 wrapped at
  three quarters of the width, `isle-v1-inventory.md` § 4). Scrolling
  would hide entries the keyboard is moving towards.
- **Layer** (`compositor.md` Shell surface hosting, proposal: overlay).
  **Adopted**.

## Copy

- The selected app's name; in the window row, the selected window's
  title.
- State marks, as text where a mark alone is unclear: "Hidden",
  "Minimised", "Not responding", "Running in background".
- A badge past three digits: "999+".
- Accessible names: "{app}" then its states, "{n} windows",
  "{n} notifications", "{n}% done", "needs attention", "hidden",
  "minimised", "not responding", "running in background", "quitting".

Words follow `copy.md`.

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **What an entry shows.** (a) A card with the live preview of the app's
   most recent window, its icon and its window count, as the proposal and
   v1's final form after four content models (`design-v1-lessons.md`
   § 5 Switcher); (b) macOS's row of icons with the selected app's name,
   previews only in the window row. **Recommended: (a).** v1 settled
   there in use, the ultrawide has room for it, previews exist only
   while the switcher shows (v1 D55), and with no Dock the card now
   carries badges, progress and marks that a bare icon would crowd. (b)
   is calmer, fits many more apps in a row, and is the Mac answer (brief
   P3).
2. **Second screen.** (a) Every app, whichever screen its windows are on,
   with the switcher on the screen holding the focused window; (b) the
   same, plus a "current screen only" setting (AltTab, KDE,
   `design-references.md` § 4). **Recommended: (a).** An app is one entry
   wherever its windows are (B20), so a per-screen filter has no clean
   answer for an app on both, and the second screen is supported, not
   designed for (brief).
3. **Apps running in the background after quit.** (a) Every such app the
   compositor knows of, listed last with the background mark, Q stopping
   it; (b) only apps whose keep-running policy or tray item says they mean
   to stay; (c) not listed, as macOS's switcher, leaving tray items in
   Control Centre as the only sign. **Recommended: (a).** The switcher
   shows only on demand, so the entries add nothing at rest (brief P1), and
   with no Dock it is the one place left that answers "is Steam still
   running?" and can stop it.
