# Dropdown terminal

P1 stage 5, wave C. How the dropdown terminal looks and moves, in Frost,
from `structure/dropdown-terminal.md` (binding). Values are `tokens.kdl`
names; numbers not yet tokens are listed at the end.

## Anatomy

The user's terminal in a frame the compositor draws, at the top of the
windows layer while shown, under the top layer.

- **Frame**: no title bar, no lights; `radius-window` on all four corners
  at rest, clipped by the compositor; `shadow-window` (the active window's)
  while shown, focused or not, since it sits above every window. No edge
  of its own beyond the shadow's half-point ring.
- **Content**: the terminal edge to edge inside the clip, its padding and
  colours its own, themed from the tokens as any installed terminal
  (`theming.md`: kitty opaque on `colour-window`). Isle draws nothing in
  it.
- **Place**: centred on the work area of the screen with focus, its top
  edge on the strip's lower edge (`size-strip`, 48), on the ultrawide and
  over a full-screen window alike.
- **Size**: the setting's proportions of the work area. Default: the
  centred zone's width (60%) by half the height: 2064 × 696 on the
  ultrawide, 1152 × 516 on the 1920 screen. "Full width": the work area's
  width less `space-s` each side, so its corners and shadow still read
  (3424 × 696).
- **Clip line**: the strip's lower edge. Nothing of the dropdown is drawn
  above it at any moment, so it never passes over the island, its panels
  or the revealed bar, which are in the top layer anyway.
- **Resize handles**: the bottom edge and both side edges carry
  `size-resize-band` (8), straddling the edge; the corners at its foot
  carry both directions along their arcs, 16 along each edge, as every
  window's. Pointer: Bibata `ns-resize`, `ew-resize`, and the diagonal at
  the foot's corners. The top edge has none. Nothing is drawn on the
  handles: the cursor is the affordance, as on any window.
- **Resizing**: a side edge moves both sides about the centre; the bottom
  edge moves the foot, the top fixed. Live, as a window resize: the old
  content held and scaled to the frame until the terminal commits. No size
  readout; the terminal's own rows and columns are its business.
- **Bounds**: at least the terminal's minimum (its own) and
  `size-dropdown-min-width` (320) by `size-dropdown-min-height` (160); at
  most the work area less `space-s` at the
  sides and foot.

## States

| State | What changes |
|---|---|
| Hidden | Nothing drawn; the process runs. |
| First use | Nothing until the terminal's first frame; the pointer shows its starting form meanwhile; then it opens (Motion). |
| Open, focused | The frame and shadow; the terminal's own focused caret. |
| A shell surface holds focus | Unchanged in look; the terminal draws its unfocused caret. |
| Another window takes focus | It closes (Motion), unless the setting keeps it; kept, it stays drawn above, unfocused. |
| The process exits | It closes. |
| Error | Nothing drawn; the island's event (`island.md`) carries the message. |
| Second screen | Its place and size from that screen's work area; moving screens is a close on one and an open on the other. |
| Full-screen app in front | The same place and frame; the strip is not drawn, the clip line stays at 48. |
| Game in front | Hidden; the chord goes to the game. |
| Mission Control opens | It closes. |
| Spotlight, the switcher or another overlay opens | Unchanged under it. |
| Locked | Under the lock. |
| Focus on | No change. |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility. |

## Motion

| Transition | Form |
|---|---|
| Open | It slides down from behind the strip on `motion-open`: from its top at 48 less its height (wholly above the clip line, so nothing is visible) to its top at 48, clipped at the line throughout. While the clip cuts it, its visible top is square; its top corners round only as they pass the line at the end. Its content rides with it, fully opaque: the clip reveals it, nothing fades. Focus is the terminal's from the first frame. |
| Close | The reverse on `motion-exit`, quicker: up behind the line. Focus returns to the window that had it in the first frame of the close, not at its end. |
| Resize | Live with the pointer, no spring. Released at a bound, it stays at the bound. |
| Moving screens | Close on the old screen and open on the new from the same frame; nothing travels between screens. |
| Mission Control opening | Its close, from the same frame as Mission Control's open. |
| Chord with an overlay open | The overlay's exit and the dropdown's open from the same frame. |
| Reduce Motion | Open and close fade in place at full size over `motion-reduced-fade-ms`; focus moves at the start of each. |
| Game Mode, or over a full-screen window while Game Mode applies | Cuts: drawn whole on the first frame, gone on the next; no shadow. |

## Accessibility

- **Text** is the terminal's, on its own background; its colours are the
  `term` palette, checked by `contrast.py` against `colour-window`.
- **Increase Contrast**: a `stroke-hairline` of `colour-hairline-strong`
  inside its edge, as every window's (`window-chrome.md`).
- **Reduce Transparency**: no change; it is opaque already.
- **Focus**: the terminal's caret; no ring, as on any window. The chord
  and the Settings rows reach everything the pointer does, size included.
- **Largest text**: the terminal's own font size; the frame's size is the
  setting's proportions and does not change with it.
- **Hit targets**: the resize band is every window's 8; its corners at the
  foot add the 16 along each edge, as `window-chrome.md` gives windows.

## Board

At 3440 × 1440 with the 1920 × 1080 screen beside it, dark first, each in
light. Crops 1600 × 760 at the top centre `[920, 0, 2520, 760]` unless
noted.

1. **Open** over a code editor and a browser: whole screen, then the crop,
   showing the island above the strip and the frame's top on 48.
2. **Full width** on the ultrawide; crop of its trailing end with a banner
   over it.
3. **Over a full-screen window**: the same place, no strip drawn.
4. **With Control Centre open** over it, and **with the bar revealed**,
   neither covering the first line.
5. **Resizing**: the bottom edge and a side edge mid-drag, cursors shown.
6. **Second screen**: open on the 1920 screen; crop `[160, 0, 1760, 600]`.
7. **Increase Contrast**.
8. **Motion**: open and close as five-frame strips (0, 25, 50, 75, 100%)
   showing the clip line and the corners rounding at the end, on the
   harness's spring; with Reduce Motion and Game Mode.

## New tokens

| Group | Name | Value |
|---|---|---|
| `size` | `dropdown-min-width` | 320 |
| `size` | `dropdown-min-height` | 160 |
