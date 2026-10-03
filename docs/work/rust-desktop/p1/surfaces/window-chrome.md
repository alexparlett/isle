# Window chrome

P1 stage 5, wave A. How the chrome looks and moves, in Frost. What it is,
its states, keys and copy are `structure/window-chrome.md` (binding), with
`structure/conventions.md` for Isle's window anatomy and the Split View
entries from `structure/mission-control.md`. Values are `tokens.kdl` names;
numbers not yet tokens are listed at the end.

## Anatomy

**Frame.** Every window: `radius-window` on all four corners, clipped by
the compositor; `shadow-window` while active, `shadow-window-inactive`
otherwise (each carries its own half-point edge ring). The resize band is
8 wide, straddling the visible edge (4 out, 4 in); at a corner it follows
the arc, 16 along each edge from the corner. A client-side window gets the
compositor's shadow only when its geometry is not inset from its surface.

**Server-side title bar.** Height `size-titlebar`, full width, part of the
window's surface (same clip, no seam but a `stroke-hairline` of
`colour-hairline` along its lower edge). Fill `colour-titlebar` active,
`colour-titlebar-inactive` inactive.

- Lights: leading inset `space-l`, centred on the bar's height (below).
- Title: `type-style-headline`, `colour-text` active, `colour-text-3`
  inactive; one line, centred on the window. When the centred title would
  come within `space-m` of the lights group's trailing edge, it is laid
  from that point instead (leading-aligned after the lights), then
  truncates at the end with "…", keeping `space-m` trailing padding; below
  48 of room it is not drawn. Nothing else is in the bar.
- Over a client-side app: no bar, no lights, nothing drawn in the window's
  top; the compositor's shadow and corner clip only as above.

**Traffic lights.** Three circles of 12 (close, minimise, zoom), `space-s`
apart, so centres are 20 apart and the group is 52 wide. Hit area: each
light owns 20 across (itself and half of each gap; the close light's also
runs to the window edge) by the bar's full height, meeting
`size-hit-minimum`. The lights' red, yellow and green are the one fixed
exception to "roles only": they are the Mac's signal and do not follow the
accent, the appearance or the glass. Values (new group `traffic`):

| | Fill | Pressed | Glyph |
|---|---|---|---|
| Close | `#ff5f57` | `#c9443e` | `#7a0c08` |
| Minimise | `#febc2e` | `#c8921d` | `#8a5200` |
| Zoom | `#28c840` | `#1f9a32` | `#0a5c12` |

- Edge: an inner half-point ring of `rgba(0,0,0,0.25)` on each coloured
  light (`traffic-edge`).
- Inactive and not hovered: `colour-fill-strong`, no ring, no glyph.
- Disabled: `colour-fill` with a half-point `colour-hairline-strong` ring,
  on an active window too; never coloured, never a glyph.
- Glyphs are drawn, not Phosphor: at 12 Phosphor's line falls under half a
  point. Drawn on Phosphor's round caps and joins, in an 8 box centred on
  the light, 1.25 line: close a 6 × 6 cross; minimise a 6 bar; zoom two
  filled right triangles of 4.5 legs pointing out along the diagonal, 1
  apart (full screen); with Option held, zoom a 7 plus (fill). Shipped with
  the Isle-drawn glyphs (`icons.md`).

**Unified toolbar (Isle's apps).** Height `size-toolbar`, spanning the
window's top. At rest it has no fill of its own: it shows the colour
beneath it (`colour-sidebar` over the sidebar, the content's `colour-window`
or `colour-view` beyond). Once content scrolls under it, a solid
`colour-titlebar` bar with a `colour-hairline` lower edge appears over the
content region only (`colour-titlebar-inactive` when inactive).

- Lights: leading inset `space-l`, centred on the toolbar's height: the
  same inset, size, gap and order as the title bar, so the group's leading
  edge is in one place on every window.
- Sidebar: `size-sidebar` wide by default, `colour-sidebar`, from the
  window's top to its bottom and leading edge, a `colour-hairline` at its
  trailing edge; its rows start under the toolbar. The lights sit over its
  top; the sidebar toggle sits at its trailing edge less `space-s`.
- Toolbar items: icon buttons `size-control-large` square, `radius-control`,
  glyph `size-glyph-large` in `colour-text-2` (Phosphor regular), no fill
  at rest, `colour-control-hover` on hover, `colour-control-pressed`
  pressed; segmented controls and the search field at
  `size-control-large`. `space-xs` between items in a group, `space-l`
  between groups, `space-l` trailing padding. Content's first item starts
  `space-l` after the sidebar (or after the lights group when there is
  none).
- Title: `type-style-headline` in `colour-text`; a subtitle under it in
  `type-style-subheadline`, `colour-text-2`. Inactive: title `colour-text-3`,
  glyphs `colour-text-3`.

**Zoom menu.** A panel on the shell's glass (`material-glass`, blur and
saturation, `shadow-panel`), `radius-panel`, padding `space-m`. In the top
layer, its top leading corner `space-xs` below the bar under the zoom light,
flipped up or leading to stay on the output.

- Section labels: `type-style-subheadline` semibold, `colour-text-2`,
  `space-xs` above their row; `space-m` between sections.
- Thumbnails: 44 tall, as wide as the output's shape gives (105 on the
  3440×1440 screen, 78 on 1920×1080), `radius-control`, fill `colour-fill`,
  `space-s` apart, four to a row: halves, thirds, two-thirds and a third,
  its mirror; then quarters, centre, fill, beside (when offered). Slots
  inside each thumbnail are `space-xxs` apart and inset `space-xxs`,
  `radius-small`, fill `colour-fill-strong`. Beside draws the real windows
  as `colour-hairline-strong` outlines and each gap as a slot.
- Full screen section: three thumbnails (full screen, left of screen,
  right of screen), slots edge to edge with no strip, each carrying
  Phosphor `arrows-out-simple` at `size-glyph-small` in `colour-text-2`.
- Hovered or keyboard-selected slot: `colour-accent` fill. Current zone:
  its slot filled `colour-text-2`, so it reads apart from both.
- Name line: under the grid, `type-style-callout` in `colour-text-2`, the
  name of the hovered slot ("Left third"), else the current zone's.
- Text rows (Return to previous size; Move to <display>; in Split View,
  Swap sides, Replace window…, Exit Full Screen): menu rows as the window
  menu's, inside the panel.

**Zone preview.** The target zone's real rectangle (inside the margins
when on), `radius-window`, in the windows layer under the dragged window:
`material-glass-clear` with the material's blur, an inner
`stroke-hairline` of `colour-hairline-strong`, no shadow. Neutral, never
the accent: the dragged window is the subject, the preview a place.

**Pointer panel.** The zoom menu's Move and resize section alone (its
thumbnails, name line, no text rows) on the same panel. Placed with its
leading edge `space-xxl` beyond the pointer and centred on it vertically,
flipped to the other side near an output edge; it stays where it opened
so the pointer can travel to it.

**Window menu.** A menu on the menu material, as every in-window menu
(`conventions.md`): `colour-window-raised`, `shadow-menu`, `radius-card`,
padding 5, minimum width 220. Rows 22, `radius-row`, `type-style-body`;
the leading mark column (Phosphor `check` at `size-glyph-small` for Keep
on top); chords trailing in `colour-text-2`; submenus by `caret-right` at
`size-glyph-small`; separators `colour-hairline` inset `space-s`. Chosen
row `colour-accent` with `colour-on-accent`; disabled `colour-text-disabled`.
At the pointer, or under the lights group when opened by chord.

**Sheet.** Attached flush to the lower edge of the title bar or toolbar,
centred across the window: `colour-window`, square top corners, bottom
corners `radius-window`, `shadow-window`, padding `space-l` `space-xl`; at
least 320 wide and at most the window's width less `space-xxxl` each
side. The window's content below the bar takes `colour-scrim`; the bar
and its lights do not. Buttons at the foot, `size-control`, `space-s`
apart, default trailing in `colour-accent`.

**Alert.** A sheet 320 wide (more at large text), its content centred:
title `type-style-headline` in `colour-text`, one line of informative text
in `type-style-callout`, `colour-text-2`, `space-xs` under it; buttons
`space-l` below, side by side and equal width, default trailing. A
destructive button uses `colour-danger-text` and is never default. A
caution glyph (Phosphor `warning`, 32, `colour-warn`) only above the title
of an alert about what cannot be undone.

- Not responding, on close: drawn by the compositor as this alert on the
  frozen window: "{App} is not responding", "You will lose any unsaved
  changes." (`copy.md`, Force Quit), Force Quit (destructive, leading),
  Wait (default). Until then the window shows only Bibata's `wait` pointer
  while the pointer is over it.

**Inhibit notice.** A capsule on `material-glass`, height
`size-control-large`, radius half its height, padding `space-m` across,
`shadow-menu`, top centre: `space-m` under the bar in a window, `size-strip`
from the output's top in a full-screen one (clear of the island's privacy
form). Isle's keyboard-slash glyph at `size-glyph` then the notice in
`type-style-callout`, `colour-text`, the chord in semibold, `space-s`
between.

**Show Desktop slivers.** Each window keeps its size and corners; 24 of it
stays on its output edge, shadow drawn.

## States

| State | What changes |
|---|---|
| Rest, active | `colour-titlebar`; lights coloured, no glyphs; title `colour-text`; `shadow-window` |
| Inactive | `colour-titlebar-inactive`; lights `colour-fill-strong`; title and toolbar glyphs `colour-text-3`; `shadow-window-inactive` |
| Hover over the lights | All three coloured with glyphs, inactive windows too; a disabled light stays as disabled; Option turns zoom's glyph to the plus |
| Pressed light | The light takes its pressed fill; the glyph stays; acts on release inside |
| Disabled light | Disabled form above, on any window |
| Loading | Bar drawn in the window's first frame, inside the open motion |
| Error | No bar or lights; corner clip and shadow stay |
| With a sheet | Content under `colour-scrim`; bar and lights clear; close's press does nothing |
| Not responding | `wait` pointer over the window; close brings the alert |
| Tiled or filled | Frame unchanged, inside the tile with margins on |
| Dragging | Zone preview in a region; pointer panel with Option |
| Full screen | No bar, corners square, no shadow; the revealed bar brings the window's bar under it |
| Narrow window, long title | Centred, then laid after the lights, then truncated, then gone; lights stay |
| Title bars off | No bar; frame and shadow only |
| Keys inhibited | The notice once per gain; the island's glyph is `island.md`'s |
| Show Desktop | Slivers at the edges |
| Second screen | Same sizes at that output's scale; thumbnails 78 wide; preview on the output under the pointer |
| Game windowed, Game Mode on | Chrome as normal, no shadows, no motion; zoom menu, panel and preview solid |
| Game full screen | No chrome; the reveal shows the menu bar only |
| Locked, Focus on | Nothing / no change |
| Reduce Motion, Transparency, Contrast, largest text | See Motion and Accessibility |

## Motion

Content fades with growth: an opacity rides its spring's progress, never
its own clock. Exits use `motion-exit` and `exit-fade-ms`.

| Transition | Spring or fade | From → to, anchor | Reduce Motion |
|---|---|---|---|
| Open | `motion-open` | Its box at scale 0.94, opacity 0 → its box; about the box's centre, no translation | Fade in place, `reduced-fade-ms` |
| Close, hide | `motion-exit` | Its box → scale 0.94, opacity 0; centre | Fade out in place |
| Minimise | `motion-exit` | Its box → a 96-wide rectangle (its aspect) `space-s` under the island, at top centre of its output; opacity reaches 0 at arrival; the island does not move | Fade out in place |
| Restore | `motion-open` | That rectangle, or its tile in App Exposé or Mission Control → its box, set before the move | Fade in place |
| Zoom, fill, tile, return, drop, unsnap | `motion-move` | Old frame → new frame; old content held, scaled to the frame, until the client commits, then a `fade-ms` crossfade | Cut to the new box with the crossfade |
| Enter full screen | bar `motion-exit`, frame `motion-move` | Bar's height to 0 with its opacity; frame to the output, radius and shadow to 0 as it lands; the Space motion is Mission Control's | Crossfade to the full-screen window |
| Leave full screen | `motion-move`, bar `motion-open` | The reverse; the bar grows back once the frame reaches its box | Crossfade |
| Revealed bar in full screen | The menu bar's reveal | The window's bar rides under the menu bar on its spring | As the menu bar's |
| Zoom menu | `motion-open` in, `motion-exit` out | Scale 0.96 → 1 from the zoom light's centre, opacity with it; out the reverse | Fade |
| Zone preview | `motion-move`; `fade-ms` in, `exit-fade-ms` out | Enters from the dragged window's frame to the zone; between zones its rectangle springs from one to the next, never replaced; on leaving it fades in place; on drop it fades as the window moves in | Jumps between zones; fades |
| Pointer panel | `fade-ms` in, `exit-fade-ms` out | In place, no movement | Same |
| Window menu | The menu motion of `menu-bar.md` | | As there |
| Sheet | `motion-open` in, `motion-exit` out | Grows down from the bar's lower edge (height reveal, content fading with it); scrim on `fade-ms` | Fade, scrim with it |
| Inhibit notice | `motion-open`, then `exit-fade-ms` | Scale 0.96 → 1 about its top centre; holds `timing-event-ms`; fades out | Fade |
| Show Desktop | `motion-move` | Box → edge and back, straight | Fade out and back |
| Light hover, press | None | Glyphs and fills change at once | Same |

Zoom menu timings: hover opens after 600 ms over the zoom light (and a
press held 600 ms), so a pass never opens it; `timing-hover-grace-ms`
covers the crossing to the panel; disarmed after a click until the pointer
leaves. Game Mode: while its profile applies, every row above is a cut
with no shadows; the zoom menu, panel and preview use `material-solid`.

## Accessibility

- **Surfaces.** Title-bar and toolbar text sits on `colour-titlebar`,
  `colour-titlebar-inactive`, `colour-sidebar`, `colour-window` or
  `colour-view`: window roles, so `colour-text-3` is allowed there. On the
  zoom menu, pointer panel and inhibit notice (glass) only `colour-text`
  and `colour-text-2`. Sheets and alerts are on `colour-window`, the window
  menu on `colour-window-raised`. The lights' glyphs are marks, not text.
- **Increase Contrast.** Every window, the zone preview and each light
  (inactive and disabled included) get a `stroke-hairline` of
  `colour-hairline-strong`; glass surfaces take `material-glass-tinted`;
  the toolbar's lower edge, sheets and controls take borders
  (`conventions.md`); slots in thumbnails get the same ring so they hold
  3:1 against their thumbnail; Phosphor steps to bold.
- **Reduce Transparency.** Bars and toolbars are opaque already; the
  toolbar shows its solid form at all times; the zoom menu, pointer panel,
  inhibit notice and zone preview use `material-solid` (the preview then
  hides what is under the zone, which the dragged window is not).
- **Focus.** The zoom menu's keyboard selection is the hovered slot's
  accent fill and shows the preview; no separate ring, as menus. Sheets,
  alerts and toolbar controls draw the focus ring (`stroke-focus-ring`,
  offset `stroke-focus-offset`) outside the control. Title bars and the
  lights take no keyboard focus; their actions have chords.
- **Largest text.** The bar's height becomes the title's line height plus
  16 when that is more than `size-titlebar`; the toolbar grows likewise;
  the lights, their gap and hit areas keep their size and stay centred.
  Menu rows, the name line and alerts grow with the text; thumbnails keep
  their size.
- **Hit targets.** Lights 20 by the bar's height; thumbnail slots at least
  `size-hit-minimum` in both directions (quarters are 20 tall in a 44
  thumbnail); a beside gap narrower than that in the thumbnail keeps a 20
  hit area centred on it; menu rows 22.

## Board

At 3440×1440 with the 1920×1080 screen beside it, dark first, each scene
also in light. Crops about 1600 wide at actual size.

1. **Side by side**: an Isle Settings window (toolbar, sidebar to the
   edge, active) and a Qt app with Isle's bar (inactive). Crop: their top
   400.
2. **Lights strip**: active, inactive, hover active, hover inactive,
   pressed close, Option over zoom, disabled zoom and minimise, Increase
   Contrast. Crop 1600 wide, with a 400% inset of one hovered group.
3. **Narrow title**: one window at four widths: centred, laid after the
   lights, truncated, dropped. Crop: the four bars stacked.
4. **Client-side app**: a libadwaita window beside an Isle-barred one;
   nothing added to its top. Crop: both tops.
5. **Zoom menu, ultrawide**: open over a Qt window, a third hovered with
   its preview on the screen behind, current zone (left half) marked,
   Return to previous size and Move to display rows. Crops: the menu at
   actual size; the whole screen at fit.
6. **Zoom menu, 1080p**: 16:9 thumbnails with beside offered. Crop: menu.
7. **Zoom menu in Split View**: the Full screen section with Swap sides,
   Replace window…, Exit Full Screen. Crop: menu.
8. **Zone preview**: a drag into the left half, then the top-left corner
   (two frames), then the top edge's fill. Crop: the left 1600.
9. **Pointer panel**: Option held mid-drag in the screen's middle, a slot
   hovered, the preview shown. Crop 1600 round the pointer.
10. **Window menu** with Move and resize open. Crop: menu and bar.
11. **Sheet** on Files with the dimmed content; **not-responding alert**
    on a Qt window. Crops: each window.
12. **Inhibit notice**: windowed VM and full-screen VM. Crop: top 400.
13. **Show Desktop**: slivers at the edges. Full screen at fit.
14. **Motion**: open, close, minimise to the island, restore, tile, enter
    full screen, run live on the harness's spring and as five-frame strips.
15. **Settings**: scenes 1, 5, 8 and 11 under Increase Contrast and under
    Reduce Transparency; scene 1 at the largest text.

## New tokens

| Group | Token | Value |
|---|---|---|
| `traffic` (new; no appearance split) | `close`, `close-pressed`, `close-glyph` | `#ff5f57`, `#c9443e`, `#7a0c08` |
| | `minimise`, `minimise-pressed`, `minimise-glyph` | `#febc2e`, `#c8921d`, `#8a5200` |
| | `zoom`, `zoom-pressed`, `zoom-glyph` | `#28c840`, `#1f9a32`, `#0a5c12` |
| | `edge` | `rgba(0,0,0,0.25)` |
| `size` | `traffic-light` | 12 |
| | `zone-thumbnail` (height) | 44 |
| | `alert` (alert width, sheet minimum) | 320 |
| | `title-minimum` | 48 |
| | `resize-band` | 8 |
| | `minimise-target` (width) | 96 |
| | `sliver` | 24 |
| `timing` | `zoom-dwell-ms` | 600 |
| `motion` | `window-scale`, `panel-scale` | 0.94, 0.96 |

`motion-*-scale` needs `scale` added to the schema's unitless suffixes.
