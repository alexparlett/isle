# Picture-in-picture

Batch 3. Answers to: `compositor.md` § For the design process
(Picture-in-picture); `competition.md`'s bottom-trailing corner row and
`capture.md`'s question whether PiP steps aside for the thumbnail;
`window-chrome.md` Shared places (undecorated windows). B8, B13, B30.

## Purpose

A video kept in view while the user works: a player's picture-in-picture
window, held small in a corner, above every window and on every Space,
full-screen ones included, as on macOS (`compositor.md` Picture-in-picture
1). It is the player's window; Isle places it, keeps it on top, snaps it
and sizes it. It is not a window to work in, so it is not in the switcher
(Picture-in-picture 3) or in Mission Control's spread, and nothing of
Isle's is drawn on it.

## Content

- **The window**: the player's own picture-in-picture window, recognised
  by title or `xdg-toplevel-tag` (Picture-in-picture 1; mpv and VLC are the
  packet's).
- **No decoration** (the proposal): no title bar, no lights, no app icon.
  The corner radius every window has, clipped by the compositor, and a
  shadow, so it reads as a window of this desktop (brief P2).
- **Controls**: the player's own, shown by the player on hover (Firefox
  and Chromium both draw play, pause, close and back-to-tab over the
  video). Isle adds none: a second set of controls over the player's would
  be the noise the brief rules out (brief P1), and the player's controls
  already reach the video, which Isle's could not.
- **Its place**: one of the four corners of its screen's work area, inset
  by a margin, so its top corners sit under the island's strip and its
  bottom corners stop short of the hot corners' points (`competition.md`).
  Opens in the corner it last used, on the screen it was on if that screen
  is present (Picture-in-picture 2); the first time, the bottom-trailing
  corner (v1, `isle-v1-inventory.md` § 3.3; `competition.md`).
- **Its size**: the video's aspect, kept through every resize
  (Picture-in-picture 2), between a floor that leaves the player's controls
  usable and a ceiling of a fraction of the work area (stage 4 sets both).
  The last size is remembered with the corner.
- **More than one** (Firefox allows several): each opens in the last-used
  corner if free, otherwise the next free corner, bottom ones first; with
  every corner taken, over the last-used one.

## States

| State | What shows |
|---|---|
| Rest | The video in its corner. |
| Hover | The player's controls. Nothing from Isle. |
| Dragging | The window follows the pointer; the corner it will settle in is marked by the zone preview's shape (`window-chrome.md`), never a tiling zone: a PiP window never tiles. |
| Resizing | Aspect kept; the corner it sits in stays fixed and the window grows or shrinks toward the screen's centre. |
| Focused | The video's keys are the player's (Space, arrows); the corner chords move it (Keyboard). Reached by the keyboard it shows the focus ring; by a click, no change of look. |
| The player changes aspect | The window takes the new aspect, its corner fixed. |
| Second screen | Snaps to the corners of the screen it is dropped on; remembered with that screen. When that screen goes, it moves to the same corner of the screen with focus (`compositor.md` Outputs 4). |
| Full-screen app in front | Over it, in its corner (Picture-in-picture 1). |
| Windowed game in front | Over it, as normal. |
| Full-screen game in front | Hidden while the game is in front, still playing; back in its corner when the game leaves the front (For the user 1). |
| Mission Control | Not in the spread; it stays in its corner under the overlay and is back when Mission Control closes. |
| Show Desktop | Stays: it is not one of the windows moved aside. |
| Space switch | Stays still while the Spaces move beneath it. |
| Stage Manager | Above the strip where they meet; not part of any set. |
| Locked | Under the lock, with every window. |
| Focus on | No change. |
| Reduce Motion | Snapping and opening fade in place rather than moving. |
| Reduce Transparency | No change: the video is opaque. |
| Increase Contrast | The outline every window gets (`window-chrome.md`). |
| Largest text | No change; the player's controls are the player's. |

## Opening and closing

- **Opens** in its corner at its size, placed at map so it never appears
  elsewhere and moves (v1's after-map mover was the bug,
  `compositor.md` Picture-in-picture, lessons). It fades in with the small
  scale every window opens with (`window-chrome.md` Choreography); the
  compositor does not know where the video was in its tab, so nothing
  flies out of it.
- **Closes** by the player's close or back-to-tab control, Cmd+W
  passed to the player, or the window menu's Close; it fades as any window
  closes.
- **Snapping**: on release after a drag, it moves to the nearest corner,
  or the one it was thrown toward when released moving fast, by a short,
  critically damped move: no bounce at the corner (brief P5). With the
  window-move modifier held as it is released (Cmd, `compositor.md` Pointer
  3) it stays where it was dropped and that place is remembered instead,
  as macOS lets Cmd-drag place it freely.

## Keyboard

Picture-in-picture is reached from the keyboard by moving focus by
direction (`compositor.md` Stacking 9), or by clicking it. While it has
focus:

| Action | Key |
|---|---|
| Play, pause, seek, mute | The player's keys |
| Move to a corner | Ctrl+Option+U, I, J, K, the quarter chords (`window-chrome.md`), read as corners |
| The window menu: Move to corner ▸, Resize, Close | Ctrl+Option+Space |
| Resize, from the window menu | Arrows grow and shrink it, keeping aspect and its corner; Return keeps, Escape undoes. The focus ring marks the mode |
| Close | Cmd+W (to the player), or Close in the window menu |

## Pointer

- **Hover**: the player's controls. Isle opens nothing on hover.
- **Drag** anywhere on the video: moves it, snapping on release. Players
  that start a move from their own body (Firefox, Chromium) do so through
  the protocol; for any other, the modifier drag moves it (Pointer 3).
- **Edges and corners**: resize, keeping aspect. Isle gives a PiP window
  its resize band even though it draws no bar.
- **Secondary click**: the player's.
- **Scroll**: the player's.

## Shared places

| Place or input | Contenders | Proposed winner |
|---|---|---|
| Bottom-trailing corner | The capture thumbnail; PiP; the hot corner | The thumbnail, above PiP in the top layer, as `capture.md` has it. PiP does not step aside: the thumbnail is gone in seconds, and moving a video the user is watching at every screenshot is motion for nothing (brief P5). Both stop short of the corner point. *Settles `competition.md`'s open question* |
| Top-trailing corner | Banners; PiP | Banners, in the top layer, over PiP for their lifetime; PiP does not move for them. |
| Top corners | The island's strip; the menu bar's reveal | The strip: PiP's top corners sit below it, so a revealed bar never meets it |
| The windows layer | Windows, windows kept on top, the dropdown terminal | From the bottom: windows, windows kept on top, PiP, the dropdown while shown (`dropdown-terminal.md`). Under the top layer and the overlay. *Adds to `competition.md` Layers* |
| A full-screen game | PiP over the game's Space | The game: PiP is hidden while it is in front, still playing (For the user 1) |
| Zones and the zone preview | A PiP drag | PiP never tiles; the preview's shape marks the corner it will settle in |
| Ctrl+Option+U, I, J, K | Quarter zones for windows | Corners for a focused PiP window; quarters for any other |
| Mission Control, the switcher, places | PiP | Excluded from the switcher (Picture-in-picture 3) and Mission Control's spread; its corner and size are kept by this sheet's own rule, not as a window place (`compositor.md` Stacking 4) |

## Answers

- **Whether it is decorated** (`compositor.md`, proposal: no title bar).
  **Adopted**: no bar and no lights, the system radius and a shadow; the
  player's own controls on hover.
- **Corner placement.** The bottom-trailing corner of the work area the
  first time (v1; `competition.md`), then the last-used corner, inset by
  a margin under the strip and short of the hot corners.
- **Snapping between corners** (Picture-in-picture 2). **Adopted**: the
  nearest corner on release, or the one it is thrown toward; free placement
  with the move modifier held, as macOS.
- **Resizing.** From its edges and corners and from the keyboard, keeping
  aspect, with its corner fixed, between a floor and a ceiling.
- **Controls on hover.** The player's; Isle draws none.
- **Whether PiP steps aside for the capture thumbnail.** No.

## Copy

- Window menu: "Move to corner" with "Top left", "Top right", "Bottom
  left", "Bottom right" (`window-chrome.md`'s zone names); "Resize";
  "Close".

## For the user

Decided at gate 2 on 2026-10-03: every recommendation below was accepted.

1. **Picture-in-picture over a full-screen game.** (a) It stays in its
   corner over the game, as on macOS; (b) it is hidden while a full-screen
   game is in front, still playing, and comes back in its corner when the
   game leaves the front. **Recommended: (b).** A game in front owns the
   screen (brief P7): a corner of a game is often its minimap or HUD, and
   a window over it keeps the game from being scanned out directly
   (`gaming.md` acceptance bar 1). A video to watch while playing can sit
   on a second screen, where nothing hides it. (a) suits a guide watched
   over a slow game.
