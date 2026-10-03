# Picture-in-picture

P1 stage 5, wave C. How a picture-in-picture window looks, sits and moves,
in Frost, from `structure/pip.md` (binding). Values are `tokens.kdl` names;
numbers not yet tokens are listed at the end.

## Anatomy

The player's own window, placed and drawn by the compositor in the windows
layer above windows kept on top. Isle draws nothing on it or over it.

- **Shape**: the video's rectangle, `radius-window` on all four corners,
  clipped by the compositor; `shadow-window` at all times, focused or not,
  since it floats over every window. No title bar, no lights, no icon, no
  edge but the shadow's own half-point ring. Keyboard focus shows as the
  focus ring (Accessibility).
- **Controls**: the player's (play, pause, close, back to tab), drawn by
  the player on its hover. Isle adds none, not even a drag handle or a
  close: the whole video is the drag area (Pointer in the sheet), and the
  player's close already sits there.
- **Place**: a corner of its screen's work area (the screen less the
  strip), inset `space-l` (16) from both edges: the top corners `space-l`
  under the strip's lower edge, the bottom ones `space-l` above the
  screen's foot, so neither reaches a hot corner's point. The first time,
  bottom trailing. With the on-screen keyboard open, the bottom corners
  are `space-l` above its panel.
- **Size**: the video's aspect. Default width `size-pip-width` (480: 480 ×
  270 for 16:9). Floor `size-pip-min-width` (240 wide: 240 × 135), where
  Firefox's and Chromium's controls still fit. Ceiling: a width of
  `size-pip-max-scale` (0.33) of the work area's width and a height of
  half the work area's height, whichever binds first (1135 × 638 at 16:9
  on the ultrawide, 634 × 357 on the 1920 screen). The size is remembered
  with the corner.
- **Corner target while dragging**: the zone preview's form
  (`window-chrome.md`): the rectangle the window would take in the nearest
  corner (or the one it is being thrown toward), `radius-window`,
  `colour-fill-strong`, no blur, an inner `stroke-hairline` of
  `colour-hairline-strong`, no shadow, under the dragged window. Never the
  accent.
- **Pointer**: the player's over the video; Bibata `grabbing` during a move
  Isle drives; the resize cursors on the `size-resize-band` (8) band that
  straddles its edges, which Isle gives it though it draws no bar.
- **More than one**: each the same form in its own corner; with every
  corner taken, the next covers the last-used one at the same place.

## States

| State | What changes |
|---|---|
| Rest | The video in its corner with its shadow. |
| Hover | The player's controls; nothing from Isle. |
| Dragging | The window under the pointer, `grabbing`; the corner target under it, moving between corners as the nearest one changes. |
| Dragging with the move modifier | No corner target: it will stay where dropped. |
| Resizing | Aspect kept; the corner it sits in fixed; it grows toward the screen's centre, between floor and ceiling. Nothing else drawn. |
| Resizing from the keyboard | The window menu's Resize: a ring of `stroke-focus-ring` in `colour-accent` at `stroke-focus-offset` outside the frame until Return or Escape. |
| Focused | By keyboard, the focus ring round the window at `radius-window` plus `stroke-focus-offset` while it holds focus; by a click, no change. The shadow stays `shadow-window` either way. |
| Aspect changes | The window takes the new shape, its corner fixed (Motion). |
| Second screen | The same sizes and inset on that screen's work area. |
| Full-screen app in front | Over it, in its corner, the same inset (the strip is not drawn, but the corner stays where the strip would end). |
| Windowed game in front | Over it, as normal; no shadow while Game Mode applies. |
| Full-screen game in front | Not drawn, still playing; back in its corner when the game leaves the front. |
| Mission Control | In its corner under the overlay, under its dim; not a tile. |
| Show Desktop, Space switch | Still in its corner; nothing moves it. |
| Stage Manager | Over the column where they meet. |
| Under the capture thumbnail or a banner | Covered for their lifetime; it does not move. |
| Locked | Under the lock with every window. |
| Focus on | No change. |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility. |

## Motion

| Transition | Form |
|---|---|
| Open | At its corner and size from the first frame (placed at map): scale from `motion-window-scale` (0.94) to 1 about its centre with its alpha on `motion-open`, as every window opens. |
| Close | Scale to 0.94 and fade on `motion-exit`, as every window closes. |
| Drag | The window follows the pointer one to one. The corner target fades in over `motion-fade-ms` at the nearest corner and springs between corners on `motion-move`, never replaced. |
| Snap | On release the window springs from where it was dropped to the target corner on `motion-move`, starting with the release velocity, so a throw carries into the move; a release faster than `motion-flick-speed` targets the corner it was thrown toward. Critically damped: it stops at the corner, no overshoot. The target fades over `motion-exit-fade-ms` as the window arrives. With the move modifier it stays where it was dropped. |
| Resize | Live with the pointer; at a floor or ceiling it stops, no rubber band. From the keyboard each step springs on `motion-move` with its corner fixed. |
| Aspect change | The frame springs to the new shape on `motion-move`, the corner fixed; the old frame's content held and scaled until the player commits, then a `motion-fade-ms` crossfade (`window-chrome.md`'s zoom). |
| Corner by chord | `motion-move` from corner to corner, straight. |
| Full-screen game comes to the front | Gone in that frame; back over `motion-fade-ms` in place when the game leaves the front. |
| Its screen goes | It appears in the same corner of the focused screen over `motion-fade-ms`; nothing travels between screens. |
| Reduce Motion | Open and close fade in place over `motion-reduced-fade-ms`. Snap and chord moves: out at the drop point and in at the corner, a crossfade of that length. The corner target jumps between corners. Resize unchanged (it follows the pointer). |
| Game Mode | Open, close, snap and moves are cuts; no shadow; the corner target in `material-solid`, jumping between corners, and the window jumps to its corner on release. |

## Accessibility

- **No text of Isle's.** The player owns every control and label on it.
- **Increase Contrast**: the `stroke-hairline` of `colour-hairline-strong`
  every window gets (`window-chrome.md`); the corner target's edge stays
  `colour-hairline-strong`.
- **Reduce Transparency**: the video is opaque; the corner target is
  already no glass and needs no form of its own.
- **Focus.** Reached by moving focus by direction or by clicking it. A
  keyboard arrival draws the focus ring (`stroke-focus-ring` in the accent
  at `stroke-focus-offset`, `radius-window` plus the offset) while it holds
  focus; a click does not, since it has no bar and already wears the
  active shadow. The window menu (Ctrl+Option+Space) opens at its top
  leading corner and draws its own selection.
- **Largest text**: no change; the window menu's rows grow as menus do.
- **Hit targets**: the floor of 240 × 135 keeps the players' own controls
  usable; the resize band is every window's.

## Board

At 3440 × 1440 with the 1920 × 1080 screen beside it, dark first, each in
light. Crops 1600 × 670 of the screen's bottom trailing corner unless noted.

1. **Rest**: a 16:9 video at 480 bottom trailing over a code editor;
   whole screen, then the crop.
2. **Each corner**: four frames, showing the inset under the strip and
   above the foot.
3. **Hover**: Firefox's own controls on it; nothing of Isle's.
4. **Drag**: mid-drag with the corner target at bottom trailing, then at
   top trailing; a modifier drag with no target.
5. **Snap**: five frames of a throw toward the top leading corner.
6. **Sizes**: floor, default and ceiling side by side, on both screens.
7. **Shared corners**: the capture thumbnail over it; a banner over it at
   top trailing; the on-screen keyboard open beneath it.
8. **Over a full-screen video app** and **Mission Control open** (under
   the dim).
9. **Increase Contrast** and **Reduce Transparency**; **keyboard focus**
   (the ring) and **keyboard resize**.
10. **Motion**: open, snap, aspect change, each with Reduce Motion and
    Game Mode.

## New tokens

| Group | Name | Value |
|---|---|---|
| `size` | `pip-width` | 480 (the first width; height from the aspect) |
| `size` | `pip-min-width` | 240 |
| `size` | `pip-max-scale` | 0.33 (of the work area's width; the height also stops at half the work area's) |

`pip-max-scale` relies on `scale` being among the schema's unitless
suffixes, as `window-chrome.md` asked.
