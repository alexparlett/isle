# Capture

P1 stage 5, wave B. How the capture tool looks and moves, in Frost, from
`structure/capture.md` (binding): the pick, the toolbar, the countdown, the
thumbnail, the colour loupe, Markup's tool row and the screen-share
picker. Values are `tokens.kdl` names; numbers not yet tokens are listed at
the end.

## Anatomy

Nothing the capture tool draws ever reaches a shot: the compositor composes
shots and streams without these parts (the sheet).

### The pick (Shift+Cmd+4)

| Part | Look |
|---|---|
| Frozen frame | Each screen's last composed frame, drawn unchanged |
| Veil | `colour-veil` over every screen: a dim, not a material, so it reads the same over any content and in every glass setting |
| Pointer | Region mode: Bibata's `crosshair`. Window mode: `isle-camera`, an Isle-drawn cursor in Bibata's style (black camera, white edge) at `icon-cursor-size` |
| Selection (region, while dragging) | The veil cut away inside the rectangle. Its edge a capture line: 1 point of `colour-capture-line` inside 1 point of `colour-capture-line-shadow`, so it holds on white and black alike. Square corners: it is a pixel rectangle. No handles while dragging; the release takes the shot |
| Size readout | "1280 × 720" in device pixels: `type-style-callout`, tabular, `colour-text`, on a `material-solid` capsule `size-control-small` (20) tall, padding `space-s`, `shadow-control`. `space-s` below the selection's trailing foot, its trailing edge on the selection's; flipped inside the selection, then above it, where the screen ends |
| Window mode | The window under the pointer lifted clear: the veil cut to its shape (its corners, not its shadow), the capture line along that shape. An open menu, popover or shell panel lifts the same way. Tab moves the lift in one frame |

### The toolbar (Shift+Cmd+5)

| Part | Look |
|---|---|
| Bar | `size-capture-bar` (48) tall, width to content (about 560 with every group), `radius-panel`, `material-glass` and its blur, `shadow-panel`. Padding `space-s` at the ends, controls centred on its height |
| Place | Centred on the screen with the pointer, its foot `space-xxxl` above the work area's foot (y 1360 on the ultrawide); where it was dragged to once moved. Above the on-screen keyboard when that is open |
| Handle | The bar itself: any point between controls drags it, pointer `grab` |
| Groups | Leading to trailing, `space-m` apart with a 1 × 20 `colour-hairline` rule centred in each gap: Close; the modes; Pick colour; Options; the action |
| Close | Plain icon button `size-control-large` (30) square, `radius-card` (concentric: 18 less 9, to the token), `x` at `size-glyph` (16), `colour-text-2` |
| Modes | One segmented control (component) holding all six, `size-control-large` tall: track `colour-fill`, `radius-card`, padding `space-xxs`; segments 36 wide, radius 8 (concentric); glyphs at `size-glyph-large` (20), Regular, `colour-text-2`; the chosen segment `colour-window-raised` with `shadow-control`, its glyph `colour-text`. A 1 × 16 `colour-hairline` between the third and fourth segments marks shots from recordings |
| Mode glyphs | Shots: `monitor`, `app-window`, `selection`. Recordings: Isle-drawn `isle-record-screen`, `isle-record-window`, `isle-record-portion` (the shot glyph with a 6-point filled dot at its foot trailing). Pick colour: `eyedropper`, a plain icon button |
| Options | A plain button, `size-control-large`, "Options" in `type-style-body`, `colour-text`, then `caret-down` at `size-glyph-small`, `colour-text-2` |
| Options menu | A menu as `menus.md` draws it, on `material-solid` (opened from glass). Its foot `space-xs` above the bar, leading edge on the button's. Section labels as menu section labels; checks in the mark column |
| Action | The default button (component) at `size-control-large`: `colour-accent`, label in `type-style-body`, `colour-on-accent`, padding `space-l`: "Capture", "Record", "Stop recording", "Share". Absent in window mode; the bar narrows (Motion) |
| Request line | For a portal screenshot: after Close and its rule, the asking app's icon at `size-app-icon-small` (16), `space-xs`, "{app} wants a screenshot" in `type-style-body`, `colour-text`; only the shot modes follow |
| Tooltips | The tooltip component, `space-s` above the control, after `timing-tooltip-ms` (700): the control's name and chord |

**Over the screen while the toolbar is open** (live, not frozen):

- **Screen mode**: nothing on one screen. With two, the veil on the screen
  not under the pointer and `isle-camera` as the pointer; the one under it
  clear with a capture line inset along its edges.
- **Window mode**: the veil, and the window under the pointer lifted as in
  the pick.
- **Portion mode**: the veil outside the selection; the capture line round
  it; eight handles at its corners and mid-edges, circles of
  `size-capture-handle` (8) in `colour-capture-line` ringed 1 point in
  `colour-capture-line-shadow`, each with a 24-square hit area; the size
  readout as in the pick. Its body drags with `grab`; handles take the
  resize pointers.

### The countdown

The island's Line event form (`island.md`), pre-empting as an OSD and in
the OSD's place over a full-screen window: `camera` (a shot) or `record`
(a recording) at `size-glyph` (16), `space-s`, the seconds left in
`type-style-clock`, tabular, in a box as wide as "10", so the pill keeps
its width. A click on it cancels. "Screenshot saved" and "Screenshot
copied" are Line events too: `camera`, and `check` as Copied.

### The colour loupe

| Part | Look |
|---|---|
| Loupe | A circle `size-loupe` (120) across, centred on the hotspot, standing in for the pointer: fifteen by fifteen frozen pixels at 8 points each, unsmoothed, a 1-point `colour-capture-line-shadow` grid between them; the centre pixel ringed in the capture line. Its rim 2 points of `colour-capture-line` with `shadow-menu` |
| Value | Under the loupe, `space-s`: a `material-solid` capsule `size-control` (24) tall, padding `space-s`; a 12 circle of the colour ringed in `colour-hairline-strong`, `space-xs`, the value ("#1E1F23") in `type-mono`, `colour-text`. Flipped above the loupe at a screen's foot |
| Ground | The frozen frame without the veil |

### The thumbnail

| Part | Look |
|---|---|
| Place | The bottom-trailing corner of the work area of the screen shot, inset `space-l` from both edges, in the top layer above picture-in-picture; its hit area is its own rectangle, so it never reaches the corner point |
| Frame | The shot fitted inside `size-thumbnail-width` × `size-thumbnail-height` (240 × 160) keeping its shape (a full ultrawide shot is 240 × 100; a portrait window 107 × 160), in a `colour-window-raised` mat `space-xs` wide, `radius-card` outside, the image at `radius-control` (concentric), `shadow-panel`. Opaque: an image, not glass |
| Recording | Its first frame, with a `material-solid` capsule `size-control-small` tall, `space-xs` in from the image's foot leading: `play` 12 in Fill and the duration in `type-style-callout`, tabular, `colour-text` |
| Hover | No change of look; the timer holds |
| Drag | The thumbnail itself follows the pointer at 0.9 opacity; its place empties |
| Duration | `timing-thumbnail-ms` (5000), restarting from the full length when the pointer leaves |
| Menu | Secondary click: a menu on `colour-window-raised` (it sits on no glass), the sheet's items, at the pointer |

### Markup (in Quick Look)

Quick Look's window (`files.md`) with a tool row under its title bar:
`size-row-large` (40) tall, `colour-titlebar`, a `colour-hairline` lower
edge; window roles, so its text and glyphs follow the window's.

- **Tools**, leading from `space-l`, `space-xs` apart: icon buttons
  `size-control-large` square, `radius-control`, glyphs at
  `size-glyph-large` in `colour-text-2`: pen `pencil-simple`, highlighter
  `highlighter`, line `line-segment`, arrow `arrow-up-right`,
  rectangle `rectangle`, ellipse `circle`, text `text-t`, redact
  `selection-slash`, crop `crop`; `space-l`; undo `arrow-counter-clockwise`.
  The chosen tool `colour-control-pressed` with its glyph `colour-text`.
- **Colour**, `space-l` after the tools: a button holding a 16 circle of
  the current colour ringed in `colour-hairline-strong`, opening a popover
  (`colour-window-raised`, `radius-card`, padding `space-s`) of the eight
  `annotation-*` swatches as 20 circles, `space-xs` apart, the current one
  ringed by the focus ring's geometry in `colour-text`.
- **Palette**: a fixed annotation set, not the data series: marks are
  written into the image, so they must not change with the appearance or
  the accent, and must read on any screenshot. Red is the default. The
  highlighter lays the colour at 40% opacity.
- **Done and Revert** at the trailing end, `space-s` apart, inset
  `space-l`: Revert a regular button, Done the default, at `size-control`.
- **On the image**: a chosen mark shows the capture line and handles of
  portion mode; crop shows the veil outside its rectangle with the same
  line and handles.

### The screen-share picker

A sheet (`window-chrome.md`): `colour-window`, square top corners, bottom
corners `radius-window`, `shadow-window`, padding `space-l` `space-xl`, the
window's content under `colour-scrim`. Width `size-picker` (680), or the
window's width less `space-xxxl` each side when that is narrower. Without a
window it stands centred on the screen as an alert does, all corners
`radius-window`, over `colour-scrim` on that screen. Window roles throughout.

| Part | Look |
|---|---|
| Title | The app's icon at `size-app-icon` (32), `space-m`, "Choose what to share with {app}" in `type-style-headline`, `colour-text` |
| Tabs | `space-m` below: the segmented control component at `size-control`, centred: "Screens", "Windows", "Portion of screen" |
| Filter | In Windows, once typing starts: the search field component (`size-control`, 180 wide) at the tabs' row trailing end, holding the typed text |
| Grid well | `space-m` below: `colour-window-sunken`, `radius-card`, padding `space-s`; three columns of tiles `space-m` apart; two rows show, then it scrolls |
| Tile | 200 wide: a preview area 200 × 125, `radius-small`, `colour-fill`, the live preview fitted and centred; `space-s`; a screen's name in `type-style-body` `colour-text` over its size ("3440 × 1440") in `type-style-subheadline` `colour-text-2`; a window's app icon at 16, `space-xs`, its title in `type-style-body`, cut in the middle |
| Window groups | Before each app's tiles, a header: the app's icon at 16 and name in `type-style-subheadline` semibold, `colour-text-2`, `space-xs` below |
| Chosen | The preview ringed `stroke-focus-ring` in `colour-accent` at `stroke-focus-offset`; the name line unchanged |
| Several allowed | The checkbox component at the preview's top trailing corner, inset `space-xs`, on a `material-solid` 20 circle; a checked tile also carries the ring |
| Portion | The well holds "Select portion…" (a regular button) centred; once chosen, one tile with the portion's live preview and its size, the button under it |
| Options | `space-m` below the well, leading, `space-s` apart: checkbox rows "Share audio", "Let {app} share this again without asking", `type-style-body` |
| Buttons | `space-l` below, trailing, `space-s` apart, `size-control`: "Cancel", then "Share" (default; disabled form until a source is chosen; "Share 2 sources") |
| Empty, loading | "No windows" in `type-style-body`, `colour-text-2`, centred in the well. A tile waiting for its preview shows the app's icon at `size-app-icon-medium` (48), or the screen's name, centred on `colour-fill` |

## States

| State | Visually |
|---|---|
| Rest | Nothing drawn. |
| Pick, region | Frozen screens, veil, crosshair; while dragging, the cut, line and readout. |
| Pick, window | Frozen screens, veil, the lifted window with its line, the camera pointer. |
| Toolbar open | The bar; the mode's drawing over the live screen (Anatomy). |
| Countdown | The island's countdown; the bar and any selection gone. |
| Recording | The island's face and privacy marks only. |
| Thumbnail | The thumbnail in its corner. |
| Colour pick | Frozen screens, no veil, the loupe and value. |
| Picker | The sheet over the dimmed window, previews live. |
| Picker, empty | "No windows" in the well; Share disabled. |
| Loading | Picker tiles with icon or name until the preview fades in; Share live. |
| Error | Protected preview: the icon stays. Write and start failures: banners (`notifications.md`'s look). |
| Many sources | The well scrolls; the filter field in the tabs' row. |
| Second screen | The pick and veil on both; the bar on the pointer's screen; the thumbnail on the screen shot. |
| Windowed game in front | Every glass part `material-solid`; no blur, no shadows on bar and thumbnail; no transitions. |
| Full-screen game in front | The pick and bar as the windowed game; no thumbnail; "Screenshot saved" in the island. |
| Full-screen app, not a game | As normal. |
| Under shortcuts-inhibit, locked, Focus on | No change of look. |
| Reduce Motion, Reduce Transparency, Increase Contrast, largest text | Motion and Accessibility. |

## Motion

The compositor draws all of it (B7). Content rides its shape's progress,
never its own clock; exits are quicker than entrances.

| Transition | Form |
|---|---|
| Pick in | The freeze is instant and invisible (the frozen frame is the live one); the veil fades in over `motion-fade-ms`. The crosshair is there from the first frame |
| Dragging, window lift | The cut, line and readout follow the pointer every frame, no spring; the lift moves between windows in one frame |
| Pick out (shot, Escape) | The live screen returns in the first frame; the veil fades over `motion-exit-fade-ms` |
| Toolbar in, out | A fade in place over `motion-fade-ms`, out over `motion-exit-fade-ms`; no slide from the edge, no scale |
| Mode change | The segment changes in one frame; the veil, selection or lift fade in over `motion-fade-ms`. Entering window mode the bar narrows about its centre on `motion-move`, the action button out by 1 − 2p; leaving, it widens and the button comes in by 2p − 1 |
| Countdown | The island's morph (`island.md`); each second's digits change in place, crossfading over `motion-fade-ms` |
| Thumbnail in | On `motion-open`: opacity min(1, 2p), scale from `motion-panel-scale` (0.96) about its bottom-trailing corner, in place (the sheet: no slide) |
| Thumbnail leaves on time | Opacity to 0 over `motion-exit-fade-ms` |
| Flick | The thumbnail tracks the pointer from the press. A release moving toward the screen's trailing or bottom edge faster than `motion-flick-speed` carries on at the release velocity until it is off the screen, on `motion-exit`; slower releases spring back on `motion-move` |
| Drag cancelled | Back to its place on `motion-move` |
| Replaced by a new shot | The old fades out over `motion-exit-fade-ms` while the new comes in as above, in the same place |
| Loupe | Follows the pointer every frame; in over `motion-fade-ms`, out over `motion-exit-fade-ms` |
| Picker | The sheet's motion (`window-chrome.md`): grows down from the bar on `motion-open`, content with the reveal; out on `motion-exit`; scrim over `motion-fade-ms`. The centred form: as Spotlight's open, and a fade out. Previews fade in over `motion-fade-ms` on arrival |
| Reduce Motion | Veil, bar, thumbnail, loupe and picker fade in place over `motion-reduced-fade-ms`; a flicked thumbnail fades instead of travelling; the bar's width jumps |
| Game Mode | No transitions: each part appears and goes in one frame; `material-solid`, no blur |

## Accessibility

- **Text roles.** On the bar's glass, `colour-text` (Options, the request
  line) and `colour-text-2` (glyphs off, the caret); never `colour-text-3`.
  Readouts, the loupe's value and the recording capsule are `colour-text`
  on `material-solid`. The picker and Markup are window roles; the
  picker's secondary text stays `colour-text-2`. The action button is
  `colour-on-accent` on `colour-accent`.
- **The veil and the capture line** are not text and are not affected by
  Reduce Transparency; the line's two tones keep 3:1 against any content.
- **Increase Contrast.** The bar at `material-glass-tinted` with a
  `colour-hairline-strong` edge; the thumbnail, readout, loupe capsule and
  picker with the same edge; the capture line 2 points of each tone; the
  handles 10; glyphs step to Bold.
- **Reduce Transparency.** The bar on `material-solid`; nothing else
  changes (the rest is opaque already).
- **Focus.** The bar's controls take the focus ring outside themselves (the
  bar's 9 points hold it); in the modes, the ring is on the focused
  segment. The thumbnail never takes focus. In the picker the well takes
  the ring when the grid has focus, and the chosen tile's ring is the
  selection; tabs, options and buttons take their own rings.
- **Largest text size.** The readout, the loupe's value, tooltips, the
  Options menu, the request line and the picker grow; the bar's icons,
  height and the action button keep their size (the sheet). Picker tiles
  keep their preview size; their text lines grow.
- **Hit targets.** Bar controls 30; handles 24; the thumbnail its size;
  picker tiles their whole area; checkboxes their row. All above
  `size-hit-minimum`.
- **Never colour alone.** Modes differ by glyph, recordings by their dot,
  the chosen tile by its ring and checkbox.

## Board

At 3440×1440 with the 1920×1080 screen beside it, dark first, then light,
over windows on the Haze wallpaper. Crops (`[x0, y0, x1, y1]`): pick
`[920, 300, 2520, 1100]`; toolbar `[920, 1000, 2520, 1440]`; thumbnail
`[1840, 1040, 3440, 1440]`; island `[920, 0, 2520, 300]`.

1. **Pick, region**: the veil, crosshair, a drag with its readout; over a
   white and a black window (the capture line's contrast).
2. **Pick, window**: a covered window lifted; an open menu lifted.
3. **Toolbar**: each mode chosen in turn, Options open, a tooltip, the
   portion selection with handles and readout; window mode's narrower bar.
4. **Stop recording**: the toolbar reopened while recording.
5. **Portal screenshot**: the request line and Share.
6. **Countdown**: 5, then 1, in the island; "Screenshot saved" over a game.
7. **Colour pick**: the loupe over text and over a gradient.
8. **Thumbnail**: a shot and a recording; the menu; the flick in three
   frames; a drag over Files.
9. **Markup**: Quick Look with the tool row, the arrow chosen, the colour
   popover open; a redaction and a crop in progress.
10. **Picker**: Screens on Firefox's window; Windows with groups, one
    chosen; several allowed with two checked; Portion before and after;
    loading and "No windows"; the centred form.
11. **Second screen**: the veil on both, a region on the 1920 screen; the
    bar on it.
12. **Game**: the solid bar and pick over a full-screen game.
13. **Settings**: scenes 1, 3, 8 and 10 under Increase Contrast, Reduce
    Transparency and the largest text size.
14. **Motion** on the harness: pick in and out; toolbar in, mode change,
    out; thumbnail in, timeout, flick, replace; picker sheet; each under
    Reduce Motion and Game Mode.

## Choices

1. **Markup's colours.** (a) A fixed annotation palette of eight, the
   same in both appearances; (b) the data series. **Recommended: (a).**
   The series are muted, built for graphs on a window and change with the
   appearance; marks are written into a file and must shout on anything.
2. **The lifted window's edge.** (a) The two-tone capture line, as the
   region; (b) an accent ring. **Recommended: (a).** One line for every
   capture target, and it holds on any content, which an accent may not.

## New tokens

| Group | Token | Value |
|---|---|---|
| `colour` | `veil` | dark "rgba(0,0,0,0.32)"; light "rgba(0,0,0,0.22)" |
| `colour` | `capture-line` | "#ffffff" (both appearances) |
| `colour` | `capture-line-shadow` | "rgba(0,0,0,0.50)" (both appearances) |
| `annotation` (new group: fixed, never by appearance) | `red`, `orange`, `yellow`, `green`, `blue`, `purple`, `black`, `white` | "#f2392b", "#ff8a00", "#ffd60a", "#2fbf5b", "#2f7cf6", "#8e5cf7", "#111214", "#ffffff"; `default` "red" |
| `size` | `capture-bar` | 48 |
| `size` | `capture-handle` | 8 |
| `size` | `loupe` | 120 |
| `size` | `thumbnail-width` | 240 |
| `size` | `thumbnail-height` | 160 |
| `size` | `picker` | 680 (the share picker's width) |
| `timing` | `thumbnail-ms` | 5000 |
| `motion` | `flick-speed` | 600 (points per second, as `schema.md` reads it) |

Isle-drawn for `icons.md`: `isle-record-screen`, `isle-record-window`,
`isle-record-portion`, and the cursor `isle-camera` (Bibata has no camera).
