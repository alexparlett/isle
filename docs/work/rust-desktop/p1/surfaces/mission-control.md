# Mission Control

P1 stage 5, wave C. How Mission Control, App Exposé, the Space switch,
Split View, Stage Manager and hot corners look and move, in Frost, from
`structure/mission-control.md` (binding). Values are `tokens.kdl` names;
numbers not yet tokens are listed at the end.

## Anatomy

Mission Control is drawn in the overlay of every screen at once, under the
island's strip, which stays above it with the pill at rest (`island.md`).
Per screen, top to bottom: the Spaces bar, the spread, the tray.

**Backdrop.** The screen's wallpaper, plain (`wallpaper.md`: not the
softened form, which belongs to the lock and the dashboard), under a dim
of `colour-veil` across the whole screen, strip included. The dim tells
the mode apart from the desktop and lifts the tiles off a busy picture; it
carries no text, so it promises no contrast. Windows of the current
desktop are not drawn in their places while open: they are the tiles.

**Spaces bar**

- A glass panel (`material-glass`, blur, saturation, noise;
  `shadow-panel`; `radius-panel`), centred on the screen, its top on the
  strip's lower edge (`size-strip`). Padding `space-m`; width to content,
  at most the screen's width less `space-xxxl` each side.
- **Miniature**: `size-desktop-miniature` (112) tall, as wide as the
  screen's aspect gives (268 on the ultrawide, 199 on the 1920 screen),
  `radius-control` (concentric: 18 less 12). Live: the wallpaper with that
  desktop's windows at their real places, scaled, shadows dropped.
  Miniatures `space-l` apart.
- **Label**: `space-xs` under the miniature, centred, one line cut at its
  end inside the miniature's width, `type-style-callout`: "Desktop 1" in
  `colour-text-2`; the current Space's in `colour-text`. A full-screen
  Space leads with its app's icon at `size-app-icon-small` (16), `space-xs`,
  then the name; a game's leads with its cover fitted to 16 tall (a 2:3
  cover is 11 × 16), `radius-small` floored to 2, `colour-hairline` edge.
  A Split View Space: both icons `space-xxs` apart, then "{app} and {app}".
- **Split View miniature**: the two windows at their split, a `space-xxs`
  gap of `colour-window-sunken` between them, edge to edge otherwise.
- **Current Space**: a ring of `stroke-found` (2) in `colour-text`,
  `stroke-focus-offset` outside the miniature, its radius the miniature's
  plus the offset. No accent: the accent ring is only the selection
  (spread, below), so one accent mark is on screen.
- **Selected or hovered Space** (and a Space under a drag): the
  switcher's plate, `colour-fill-strong`, `space-xs` round the miniature
  and its label, `radius-card` (6 + 4). A place takes the plate; a window
  takes the ring (spread, below); the current Space keeps its ring over
  the plate.
- **+ tile**: after the last miniature, `space-l` on: a well
  `size-desktop-miniature` square, `colour-fill`, `radius-control`, `plus`
  at `size-glyph-large` (20) in `colour-text-2`; its label slot empty but
  kept. Hover and drag-over: `colour-fill-strong`, the glyph `colour-text`.
  Hidden at the compositor's limit; nothing takes its place.
- **Remove**: on hover over a miniature, a badge centred on its top
  leading corner: a `size-control-small` (20) circle of `material-solid`,
  `shadow-control`, a `stroke-hairline` edge in `colour-hairline`, `x` at
  `size-glyph-small` in `colour-text`; hit box 24.
  A full-screen or Split View Space shows `arrows-in-simple` instead (Exit
  Full Screen). Never on a screen's only desktop. Hover
  `colour-control-hover` over the solid, pressed `colour-control-pressed`.
- **Bar height**: 12 + 112 + 4 + 15 + 12 = 155 (y 48 to 203).
- **Many desktops**: past the screen's width, miniatures shrink together
  to `size-desktop-miniature-min` (64 tall), then the row scrolls sideways
  inside the panel's padding with the overlay scroller; labels keep their
  size.

**Spread**

- The area from the bar's foot plus `space-xxl` to the tray's top less
  `space-xxl` (or the screen's foot less `space-xxxl` with no tray),
  `space-xxxl` in from the sides.
- **Tile**: the window's live texture, title bar and lights included, its
  aspect kept, never larger than real; `radius-window` scaled with the
  tile, never below `radius-small`; `shadow-window-inactive`, the hovered
  or selected tile `shadow-window`. A dialog is drawn on its parent's tile
  at its relative place.
- **Icon**: Papirus at `size-app-icon-medium` (48), centred on the tile's
  width with its centre on the tile's lower edge, as the switcher's cards;
  a game's cover in its place, fitted to 48 tall, `radius-small`,
  `colour-hairline` edge.
- **Spacing**: tiles `space-xxl` apart side to side; rows `space-xxl`
  apart beneath a reserve of 52 under each tile (the icon's 24 overhang,
  `space-xs`, the title capsule's 24), so a title never covers the row
  below.
- **Title capsule** (hovered or selected tile only): `space-xs` under the
  icon, centred on the tile, `size-control` (24) tall, radius 12, padding
  `space-s`, `material-glass`, `shadow-control`; the window's title in
  `type-style-body`, `colour-text`, one line, cut in the middle (as the
  switcher's window titles), at most the tile's width or `size-menu-max`
  (480), whichever is wider.
- **Selection ring**: the hovered or keyboard-selected tile takes a ring
  of `stroke-focus-ring` in full `colour-accent`, `stroke-focus-offset`
  outside the tile: the screen's one accent mark. This is the keyboard
  focus: no second ring is drawn. While a tile's menu is open it keeps its
  ring.
- **Close**: on pointer hover only, the Remove badge (above) with `x`,
  centred on the tile's top leading corner, where the close light sits.
  Keyboard selection shows no badge; W, Delete and Cmd+W close.
- **Group windows by application**: each app's tiles laid out as one
  cluster, `space-l` apart inside it, with no icons on its tiles; under the
  cluster, `space-s` on, a glass capsule `size-control-large` (30) tall,
  radius 15, padding `space-s`: the icon at `size-app-icon-small`,
  `space-xs`, the name in `type-style-headline`, `colour-text`. A Stage
  Manager desktop's sets use the same cluster.
- **Loading**: a tile appears at once as an opaque frame of `colour-window`
  at its place and size, the icon centred in it as well as on its edge;
  the texture fades in over it. **Protected surface**: the frame and its
  centred icon stay.

**Minimised tray**

- A glass panel as the bar's, centred, its foot `space-l` above the
  screen's foot, width to content, at most the screen's width less
  `space-xxxl` each side, so its ends are clear of the bottom corners.
  Padding `space-m`.
- Groups, leading to trailing: "Minimised", then "Hidden". Each opens with
  its heading in `type-style-subheadline` semibold, `colour-text-2`,
  `space-xs` above its first thumbnail; between groups a gap of
  `space-xxl` with a vertical `colour-hairline` centred in it, the
  thumbnails' height. A group with nothing is absent, heading and rule
  with it.
- **Thumbnail**: live, fitted inside `size-tray-tile-width` ×
  `size-tray-tile-height` (192 × 80), `radius-small`, `space-l` apart; the
  app's icon at `size-app-icon` (32) centred on its lower edge. A hidden
  app's thumbnail at 50% over a `colour-fill` well (the switcher's hidden
  form). Hover: the ring and the title capsule, the capsule's foot
  `space-xs` above the tray panel's top edge, over the dim, never on the
  tray (no glass on glass).
- Height: 12 + 14 + 4 + 80 + 16 + 12 = 138.

**App Exposé**

- No Spaces bar. A header capsule centred, its top `space-xxl` under the
  strip: `material-glass`, `size-row-large` (40) tall, radius 20, padding
  `space-xs` leading and `space-m` trailing; the icon at `size-app-icon`
  (32), `space-s`, the name in `type-style-title-3`, `colour-text`.
- The spread as above, starting `space-xxl` under the header.
- The set-apart row in the tray's form and place: "Minimised", "Hidden",
  then one group per other desktop headed by its name ("Desktop 2").
- Other apps' windows are not drawn while it is open.
- Nothing to show: the header, then a glass capsule `size-control` tall at
  the spread's centre, "No windows" in `type-style-body`, `colour-text-2`.
- On a second screen: the header and that screen's spread; the set-apart
  row only on the screen with focus.

**Space switch.** Nothing is drawn but the windows moving (Motion): no
OSD, no dots, no label.

**Split View**

- **Panes**: the two windows edge to edge across the output, either side
  of the divider; square corners, no shadows, as full screen.
- **Divider**: `space-s` (8) wide, the output's height, `colour-window-sunken`;
  a grip at its centre, 4 × `space-xxxl` (32), radius 2,
  `colour-hairline-strong`, `colour-text-2` while hovered or dragged.
  Pointer `col-resize`. Hit area `size-hit-minimum` wide, centred on it.
- **Chooser**: the first window in its half; the free half shows the
  wallpaper under `colour-veil`, with the candidates laid out as the
  spread inside it (`space-xxxl` from the half's edges and the divider,
  `size-strip` from the top), with the spread's tiles, icons, ring and
  title capsule. A window too wide for the half: texture and icon at 40%,
  no ring, pointer `not-allowed`; hovered, its capsule reads "Doesn't fit
  in Split View" in `colour-text-2`. With no candidates, or none that fits,
  the free half holds App Exposé's empty capsule at its centre, "No other
  windows fit" in `type-style-body` `colour-text-2`.
- **Drop on a full-screen Space** in the bar: the miniature shows the half
  the window would take as the zone preview's form (`window-chrome.md`):
  `colour-fill-strong`, no blur, an inner `stroke-hairline` of
  `colour-hairline-strong`, `radius-small`, over the half under the
  pointer.
- **Swap by drag**: a pane's revealed bar dragged onto the other: the
  other pane takes the zone preview's form over its half; on drop both
  move on `motion-move`.

**Stage Manager**

- **Column**: at the screen's leading edge, inset `space-l`, the first
  entry's top `space-l` under the strip; no panel behind it.
- **Entry**: a box `size-stage-entry-width` × `size-stage-entry-height`
  (160 × 100). The set's front window fitted in it, `radius-small`,
  `shadow-window-inactive`; up to two more of its windows behind,
  each offset `space-xs` up and `space-xs` toward the trailing side, at
  the same scale. The app's icon at `size-app-icon` (32) centred on the
  stack's lower edge. Entries `space-xxl` apart below the icon's overhang:
  a pitch of 140, so 9 fit on the ultrawide and 7 on the 1920 screen.
- **Hover**: the front window takes `shadow-window`; the title capsule
  with the app's name `space-xs` under the icon. No ring: the column is
  not a selection.
- **Stage**: the front set in the work area beside the column, which
  reserves its width plus `space-l` each side (192) while shown.
- **Drop on an entry**: the switcher's plate round the stack and icon,
  `radius-card`; the window shrinks toward it on `motion-move`.
- **Entry to the stage**: the stack follows the pointer; the stage takes
  the zone preview's form; on drop its windows grow on `motion-move`.

**Hot corners.** No feedback is drawn: no glyph, no glow, no mark at
rest or during the rest in the corner. The corner's action is its own
feedback, starting on the frame the rest ends
(`timing-corner-dwell-ms`). A corner that does nothing draws nothing.

## States

| State | What changes |
|---|---|
| Rest | Nothing; no surface exists. |
| Open | Backdrop, bar, spread, tray when it holds anything, on every screen. No ring until the pointer moves or a key is pressed (v1 D9). |
| App Exposé | Header capsule, the app's tiles, the set-apart row; no bar. |
| Empty desktop | Bar and backdrop; no text. |
| One window | Its tile at real size and place. |
| Many windows | Tiles shrink to fit the spread; titles only on the ringed tile. |
| Many desktops | Miniatures shrink to 64 tall, then the bar scrolls; the selection kept in view. |
| Minimised or hidden windows | The tray's groups; in App Exposé the set-apart row. |
| Dragging a window | The tile follows the pointer with `shadow-window`, its place left empty. A desktop or the + tile under it takes the plate; a full-screen Space shows the half it would take; the other screen's spread shows the zone preview's form over its spread area, `radius-panel`. A refusing Space (Split View, too small) takes nothing and the pointer is `not-allowed`. |
| A window opens or closes while open | The spread re-lays out (Motion). |
| Close asks to save | The tile stays, the dialog drawn on it; the ring stays. |
| Loading, error | Opaque frames with the icon; textures fade in; a protected one keeps its frame. |
| Second screen | Each screen its own bar (199-wide miniatures on the 1920 screen), spread and tray. |
| Full-screen app in front | The bar with every Space; the spread holds its one tile, or both panes of a Split View. |
| Windowed game in front | As Open, every material `material-solid`, no transitions. |
| Full-screen game in front | Drawn over the game, `material-solid`, no blur, no motion; the backdrop is the wallpaper under `colour-veil`. |
| Split View: choosing | The first window in its half; candidates in the other; misfits at 40%; with none that fits, "No other windows fit". |
| Stage Manager desktop | The column at the leading edge when shown; in Mission Control every window in clusters by set. |
| A system prompt | Mission Control stays under the prompt's scrim, unchanged. |
| Locked | Gone in the lock's first frame; never drawn over the lock. |
| Focus on | No change. |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility. |

## Motion

The compositor draws all of it (B7). Every opacity rides its spring's
progress p; nothing fades on its own clock. Exits run on `motion-exit`.

| Transition | Form |
|---|---|
| Open | Each window travels from its real rectangle to its tile on `motion-move`, scale and position together, its radius and shadow blending to the tile's by p. The dim, the bar and the tray fade in at their places by p; tile icons by 2p − 1. Nothing grows from a corner or drops. |
| Close | The reverse on `motion-exit`: each tile back to its real rectangle; bar, tray and icons out by 1 − 2p, the dim by 1 − p. The chosen window is drawn last, on top, and lands with the rest. A chosen Space adds the switch (below) from the same frame. |
| Gesture | p follows the fingers; on release it settles on `motion-move` (open) or `motion-exit` (closed) from the fingers' speed, by distance or past `motion-flick-speed`. |
| App Exposé | As Open for the app's windows; other apps' windows fade by 1 − p in place; the set-apart row rises from below the screen's foot to its place by p, on the same spring. Tab to the next app: the old app's tiles return to their rectangles on `motion-exit` while the new app's travel to theirs on `motion-move`; the header crossfades by p. |
| Hover, selection | Instant: ring, shadow, title capsule and badge appear in the next frame and go in one. |
| Space (look at full size) | The selected tile springs to its real rectangle on `motion-open`, over the spread; released or pressed again, back on `motion-exit`. |
| Re-layout (open, close, move) | Tiles move to their new places on `motion-move`; an arriving window grows from `motion-window-scale` with its alpha on `motion-open`; a closed one shrinks to `motion-window-scale` and fades on `motion-exit` before the rest close the gap. |
| Space switch | On `motion-switch`: the old desktop's windows travel `motion-switch-distance` (80) away from the new one's side with alpha 1 − p, while the new desktop's arrive from 80 on the other side with alpha p. Wallpaper, island and the top layer stay still. At either end of the row nothing moves. A gesture drives p; a cancelled one returns on `motion-exit`. |
| Switch while open | The bar's ring jumps to the new miniature in one frame; the spread's tiles leave and arrive as the switch, inside the spread. |
| Enter, leave full screen | The window's frame to the output on `motion-move` (`window-chrome.md`) while the rest of its desktop leaves by the switch; the window never slides. Leaving reverses it, its frame on `motion-move` and the desktop arriving on `motion-switch`. |
| Drag a tile | It follows the pointer at its size. Entering the bar it shrinks on `motion-move`, about the grab point, to the scale a window takes in a miniature times two; leaving the bar it grows back. Dropped on a Space it shrinks into the miniature on `motion-exit` and appears there; the spread closes the gap on `motion-move`. Refused or dropped on nothing, it returns to its place on `motion-move`. |
| Desktop added, removed | The new miniature fades in by p as the + tile moves along on `motion-move`; a removed one fades out over `motion-exit-fade-ms`, then the rest close up and the panel narrows about its centre on `motion-move`. Reordering by drag: neighbours make room on `motion-move`. |
| Restore from the tray | The window grows from its thumbnail to its box on `motion-open` (`window-chrome.md`); the tray narrows on `motion-move`, and goes by `motion-exit-fade-ms` when empty. |
| Split View: enter | The window to its half on `motion-move` with the switch; the free half's dim and candidates fade in by its p, the candidates travelling from their rectangles on the origin desktop as in Open. |
| Split View: choose, cancel | The chosen tile grows to fill the half on `motion-move`; the other candidates and the dim fade by 1 − 2p. Cancel: the first window back to its box on `motion-move` and the switch back on `motion-switch`. |
| Divider | Both panes follow the pointer live, or the window menu's Resize from the keyboard; a double-click returns them to halves on `motion-move`. |
| Stage Manager swap | The chosen entry's windows grow from their places in the stack to their boxes on `motion-move`; the front set shrinks into a new entry at the column's top on `motion-exit`; entries below move down on `motion-move`. |
| Stage Manager reveal | A push of `size-edge-push` into the leading edge, held `timing-hover-dwell-ms`: the column slides in from beyond the edge on `motion-open`, its shadows with it; leaving it, after `timing-hover-grace-ms`, back out on `motion-exit`. With Show recent apps on, covered by a staged window: out past the edge on `motion-exit`; uncovered, back on `motion-open`. |
| Tray thumbnail dragged out | It grows to its spread size leaving the tray, on `motion-move`, then drags as a tile. |
| Ctrl+Shift moves | A window or desktop moved by key runs the drop and reorder motions above without a pointer. |
| Hot corner | None of its own. |
| Reduce Motion | Open and close: the spread at its places and the real windows crossfade over `motion-reduced-fade-ms`, the dim, bar and tray with it; a gesture drives the crossfade. The switch, Split View, Stage Manager swaps and the column's reveal are crossfades of the same length at their end places. Re-layout and drags of a tile jump; the dragged tile itself still follows the pointer. The column's hide and return when covered fade. |
| Game Mode | No transitions: each state replaces the last in one frame; the dragged tile follows the pointer; no blur sampled. |

## Accessibility

- **Text roles.** Labels, titles, headings and the header sit on glass
  (bar, tray, capsules): `colour-text` and `colour-text-2` only, never
  `colour-text-3`. Nothing is written on the dim. The badges' glyphs are
  `colour-text` on `material-solid`.
- **Increase Contrast.** Every tile, thumbnail, miniature and stack gets a
  `stroke-hairline` of `colour-hairline-strong`; glass takes
  `material-glass-tinted` with the same edge; the rings gain a 1-point
  `colour-hairline-strong` outer edge so they read on any wallpaper; the
  plate becomes `colour-control-pressed` with a `colour-hairline-strong`
  inner edge (the switcher's); the tray's rule and the divider's grip
  `colour-hairline-strong`; glyphs step to Bold.
- **Reduce Transparency.** Bar, tray, capsules and header
  `material-solid`. The Split View drop half and the other screen's drop
  form are the zone preview's form, not glass, and stay. The dim and the
  wallpaper are unchanged.
- **Focus.** The ring (spread, tray, Split View) or the plate (bar) is the
  keyboard selection, drawn from the first key; nothing else rings. The
  selection crosses between screens with the arrows. While its menu is
  open a miniature keeps the plate.
- **Names.** Tiles "{title}, {app}"; tray and set-apart thumbnails
  "{title}, minimised", "{title}, hidden" or "{title}, Desktop 2";
  miniatures "Desktop {n}", with "current" as their state.
- **Largest text.** Labels, titles, headings and the header scale, and the
  bar and tray grow by the extra line height; miniatures, tiles,
  thumbnails, icons and badges keep their size. Labels cut at their end.
- **Hit targets.** Miniatures at least 64 tall; the + tile its full well;
  badges 20 with a 24 hit box; tiles their whole rectangle; the divider
  `size-hit-minimum`. All at or above `size-hit-minimum`.
- **Never colour alone.** The current Space's ring is neutral and it is
  also the only label in `colour-text`; a refused drop changes the pointer;
  a misfit says why.

## Board

At 3440 × 1440 with the 1920 × 1080 screen beside it, dark first, each in
light, on Frost and on a bright picture. Crops about 1600 wide at actual
size: the bar `[920, 0, 2520, 260]`, the spread's centre `[920, 220, 2520, 900]`,
the tray `[920, 1240, 2520, 1440]`.

1. **Open, seven windows** on Desktop 2 of three with a full-screen Firefox
   Space and a game's Space; whole screen, then the bar crop.
2. **Bar states**: current Space's neutral ring, plate on a selected Space,
   the plate under the ring, Remove on hover, Exit Full Screen on a
   full-screen Space, + tile hovered, a Split View pair label, a game's
   cover label.
3. **Spread hover**: ring, title capsule, close badge; and keyboard
   selection without the badge.
4. **Tray**: three minimised and two hidden, the rule, headings; a hovered
   thumbnail with its capsule above.
5. **App Exposé** for kitty: header, four tiles, the set-apart row with
   Minimised, Hidden and Desktop 3; and "No windows".
6. **Group windows by application**: three clusters with name capsules.
7. **Many**: 24 windows; 14 desktops shrunk, then 20 scrolling.
8. **Drag**: a tile shrunk over the bar onto Desktop 3 (plate), onto the +
   tile, onto a full-screen Space with the half marked, refused by a Split
   View Space; the other screen's spread marked.
9. **Loading and protected**: frames with icons; one texture at 50%.
10. **Split View chooser**: Zed in the left half, five candidates in the
    right, one misfit hovered with its line; the divider at rest and
    hovered.
11. **Stage Manager**: the column of five stacks with the front set
    beside; the hidden column revealed over windows; Mission Control on
    that desktop, clusters by set.
12. **Second screen**: both screens open at once; crop of the 1920 bar.
13. **Over a full-screen game**: solid, no blur.
14. **Increase Contrast, Reduce Transparency, largest text**: scenes 1
    and 4.
15. **Motion**, on the harness's spring and as five-frame strips: open and
    close, App Exposé, the Space switch (and at the row's end), a drag to
    a Space, the Split View choose and cancel, a Stage Manager swap and
    reveal; each with Reduce Motion and Game Mode.

## Choices

1. **The backdrop.** (a) The plain wallpaper under `colour-veil`; (b) the
   plain wallpaper alone; (c) the softened form. **Recommended: (a).** It
   reads as a mode the way macOS's dimmed desktop does and lifts the tiles
   off any picture; (b) leaves windows fighting a bright wallpaper, and
   (c) is the lock's and the dashboard's look and blurs the miniatures'
   wallpaper match away.
2. **Hot corner feedback.** (a) None: the action is the feedback, as on
   macOS; (b) a quarter-circle of glass filling the corner over the rest.
   **Recommended: (a)** (brief P1): the corner is reached on purpose, and
   a mark there would flash on every pass of the pointer.

## New tokens

| Group | Name | Value |
|---|---|---|
| `size` | `desktop-miniature` | 112 (height; the width follows the screen's aspect) |
| `size` | `desktop-miniature-min` | 64 |
| `size` | `tray-tile-width` | 192 |
| `size` | `tray-tile-height` | 80 |
| `size` | `stage-entry-width` | 160 |
| `size` | `stage-entry-height` | 100 |
| `motion` | `switch-distance` | 80 (points the Space switch moves windows) |
| `timing` | `corner-dwell-ms` | 250 (the rest in a hot corner before it fires) |
