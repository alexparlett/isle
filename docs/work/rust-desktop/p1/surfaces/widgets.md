# Widgets

P1 stage 5, wave C. The widgets dashboard: its backdrop, the grid, each
built-in widget in its sizes, providers' widgets, edit mode and the soft
push, the gallery, the consent panel, grants, and the provider row that
every provider surface shares. Structure is binding from
`structure/widgets.md`. The backdrop is `wallpaper.md`'s softened form with
`lock.md`'s wash; plates are `lock.md`'s; modules are `control-centre.md`'s;
the consent panel is `prompts.md`'s frame. Values are `tokens.kdl` names;
numbers not yet tokens are listed at the end.

## Anatomy

### Backdrop and plates

The dashboard covers its screen, strip included: the softened wallpaper
(`wallpaper.md`), then `material-lock-wash` over it, opaque over the
windows. With the wash the backdrop meets the glass rule, so the few lines
drawn straight on it (the empty dashboard, a widget's title while it loads)
use `colour-text` and `colour-text-2`. Every widget's content sits on a
**plate**, `lock.md`'s: `material-glass` at the user's glass position,
`material-noise-opacity`, `shadow-menu`, no blur pass (the backdrop is
already blurred and holds no shapes), `radius-panel`. Padding `space-m`, so
inner corners are `radius-control`; Controls and Session, which hold tiles,
pad `space-s` so their tiles keep `radius-card`. The island's privacy form
and OSDs draw above it at their anchor (`island.md`).

### The grid

| Part | Look |
|---|---|
| Cell | `size-widget-cell` (72) square, the same on every screen; gaps `space-l` (16), so a pitch of 88. A widget of n cells is 88n − 16: 72, 160, 248, 336, 512 |
| Rows hold | One row's inner 48 holds two lines of reading (headline over subheadline), two battery rows of 18, or two volumes' bars |
| Extent | Columns as many as fit the screen's width less `space-xxxl` each side; rows as many as fit between `size-strip` + `space-l` and `space-xxl` above the controls; the grid centred in both directions. Ultrawide 38 × 14 (3328 × 1216, from x 56, y 101); 1920×1080 21 × 10 (1832 × 864) |
| Controls | Bottom centre, outside the grid, `space-s` apart, their foot `space-xxxl` above the screen's bottom: "Edit widgets" as a plate capsule (`size-control-large`, radius 15, padded `space-m`, `type-style-body` `colour-text`) and the close control, a 30 plate circle holding `x` at `size-glyph` in `colour-text`. In edit mode, "Add widgets" (the capsule, `colour-fill-strong` over it while the gallery shows) and "Done", the default button (`colour-accent`, `colour-on-accent`) at the same height |
| Default layout | Clock 2×2, Calendar 4×2, Weather 2×2, Media 4×1 over Storage 4×1, System 2×2, Devices 2×1 over Notifications 2×1: one block 16 × 2 (1392 × 160), centred on the ultrawide at columns 11 to 26, rows 6 and 7 (x 1024 to 2416, y 629 to 789), inside the middle half. The same block on the 1920 screen |
| Hover | A widget the pointer moves over: `colour-fill` over its plate. Nothing opens |

### The one-line forms

- **Empty.** A widget with nothing to show draws a strip at the top of its
  place: a plate the widget's width by `size-row-large` (40), `radius-panel`,
  matching the plates beside it,
  holding its line ("Not playing") in `type-style-body` `colour-text-2`,
  inset `space-m`, and a trailing small button where the sheet gives one
  ("Add Calendar…"). No glyph; the rest of its place is the backdrop.
- **Reading.** Every widget has a one-line form its content falls to at
  large text, and 2×1 is that form drawn at rest; a widget defaults to the
  smallest size that reads more than one value (Devices 2×1, the others
  2×2 or larger): its essential value on one line ("18° Partly cloudy",
  "14:05 Tokyo"), the plate unchanged.
- **Title only.** Before its first data a widget draws its plate and its
  name in `type-style-subheadline` semibold `colour-text-2` at the top
  leading inset; no spinner.

### The built-in widgets

Sizes are columns × rows; the default is in bold. Text styles named once
per widget hold at every size unless a size says otherwise. Figures are
tabular throughout. Glyphs are Phosphor in `colour-text`; data colours
are `series`, never the accent, except a running timer's live ring.

| Widget | Sizes | Content |
|---|---|---|
| Clock | 2×1, **2×2**, 4×2 | 2×1: the city (headline) over its weekday (subheadline `colour-text-2`) leading; the time trailing in `type-style-title`. 2×2 digital: city top leading (subheadline semibold `colour-text-2`); at the foot the time in `type-style-figure` over weekday and date (subheadline `colour-text-2`). 2×2 analogue: a dial the inner height less the city's line, centred, the city under it. 4×2: the face leading, city in `type-style-title-3` and date trailing it |
| Dial | — | A circle of `colour-fill` (`colour-fill-strong` between 18:00 and 06:00 in its zone); twelve ticks, the quarters 2 × 6 in `colour-text`, the rest 1 × 4 in `colour-text-2`; hour hand 4 wide to half the radius, minute hand 3 wide to 0.8, round caps, `colour-text`; a 6 centre cap. A seconds hand, 1 wide in `colour-text-2`, only when Date & Time shows seconds |
| Calendar | 2×2, **4×2**, 4×4 | 2×2: weekday (subheadline semibold `colour-text-2`), the day's number in `type-style-figure`, then the next event: its mark, title (callout), time (subheadline `colour-text-2`). 4×2: that column 104 wide leading; trailing, event rows as Notification Centre's (`notification-centre.md`: mark, title, time and place, Now, Join), up to three, then "{n} more"; rows past today lead with the day ("Tomorrow", "Tuesday") in subheadline semibold `colour-text-2`, taking a row. 4×4: Notification Centre's month at 44-wide columns and 28-high rows, then two event rows. An event's details open in a popover in the settings popover's form (`material-solid`) |
| Weather | 2×1, **2×2**, 4×2, 4×4 | Glyphs Regular, monochrome: `sun`, `moon`, `cloud`, `cloud-sun`, `cloud-moon`, `cloud-rain`, `cloud-snow`, `cloud-lightning`, `cloud-fog`, `wind`. 2×1: glyph 24, temperature in `type-style-title`, the place (subheadline `colour-text-2`). 2×2: place top leading; temperature in `type-style-figure`; glyph 16 and condition (callout); high and low (subheadline `colour-text-2`). 4×2: that column 120 wide, then six hours: hour (subheadline `colour-text-2`), glyph 20, temperature (callout), in equal columns. 4×4: then five days as `size-row` rows: weekday, glyph 16, low, a range bar (4 tall, `colour-fill-strong` track for the week's span, the day's span in `colour-text`), high |
| Media | 2×2, **4×1**, 4×2 | `control-centre.md`'s Now Playing parts. 4×1: art 48 at `radius-control`, `space-m`, title (headline) over artist (subheadline `colour-text-2`), transport trailing (`skip-back`, play or pause, `skip-forward` as plain icon buttons `size-control-large`, Fill, glyphs 20, 24, 20). 2×2: art `size-media-art` (64) top leading, the player's icon 16 top trailing, title and artist, transport at the foot. 4×2: art 136 leading; title (`type-style-title-3`), artist, the player button, the scrubber with times, transport. Empty: "Not playing" |
| Timer | 2×1, **2×2** | Idle: the label (subheadline `colour-text-2`), the duration in `type-style-figure` (2×1: `type-style-title`), presets as small buttons, Start. Running: a ring the inner height less 40, 4 wide, track `colour-fill-strong`, arc `colour-accent` from twelve o'clock (live, as the Updates arc); the time left centred in `type-style-title-3`; Pause and Reset (`arrow-counter-clockwise`) as plain icon buttons 30 under it. Paused, the arc `colour-text-2`. 2×1: `timer` 16, label over time leading, play or pause trailing |
| System | 2×1, **2×2**, 4×1, 4×2, 6×2 | Each series as a number, ring or graph (its editor): number, the label (subheadline `colour-text-2`) over the value (`type-style-title-3`); ring, 48, 4 wide, track `colour-fill-strong`, arc in the series' colour, value centred (callout), label under; graph, label and value over a 1.5 line in the series' colour on a `colour-fill` area at 20%. Colours: the widget's first four graphed or ringed series take `series-a` to `series-d` in order (by default CPU, memory, GPU, temperature); numbers take none. 2×1 one series; 2×2 four numbers in quarters; 4×1 four numbers across; 4×2 four rings; 6×2 four graphs |
| Devices | **2×1**, 2×2, 4×2 | `control-centre.md`'s Batteries rows (18 tall, kind glyph 12, name callout, `lightning`, "{n}%"; low by `battery-warning` and weight), `space-xs` apart: two at 2×1, six at 2×2, two columns of six at 4×2; the last row "{n} more" past that. Empty: "No devices" |
| Storage | **4×1**, 4×2 | Rows 16 tall, `space-s` apart: `hard-drive` 12 `colour-text-2`, the name (callout, a 96 column, cut), a bar (6 tall, radius 3, track `colour-fill-strong`, used in `colour-text`) filling the middle, free space trailing (subheadline tabular `colour-text-2`). Nearly full: `warning` 12 for the drive glyph and the figure semibold `colour-text`. Two at 4×1, five at 4×2 |
| Desktops | **4×1**, 4×2, 6×2 | One row per desktop, 20 tall, `space-s` apart: its name in a 72 column (subheadline `colour-text-2`), its windows' app icons at `size-app-icon-row` (20), `space-xs` apart, then "+{n}". The current desktop's row on a `colour-fill-strong` band 24 tall at `radius-row`, its name `colour-text` semibold. Two rows at 4×1, five at 4×2 |
| Controls | **4×2**, 4×3, 4×4 | `control-centre.md`'s modules as tiles (`colour-fill`, `radius-card`, 64 tall, `space-s` apart), their widths from the inner 320: Wide the whole, Medium half (156), Small a quarter (74). Default Sound and Display Wide. Same component, same state |
| Session | 4×1, **4×2** | Lock Screen `lock`, Sleep `moon-stars`, Restart… `arrow-clockwise`, Shut Down… `power`, Log Out… `sign-out`: each a `size-lock-button` (40) circle of `colour-fill-strong` with its glyph at 20, Regular, `colour-text`, spread across the inner width. 4×2: each name `space-xs` under its circle (subheadline `colour-text-2`, centred, one line), then the Energy Mode module as a Medium tile the inner width. 4×1: circles alone, names in tooltips |
| Notifications | 2×1, **2×2**, 2×4 | Rows `size-row` (28), `space-xs` apart: the app's icon at 20, its name (body `colour-text`, cut), the count trailing as the quiet badge. One, four and eight rows; then "{n} more". Empty: "No notifications" |

A list's last row reads "{n} more" in `type-style-callout` `colour-text-2`
when the size cannot hold the rest; its click goes where the list belongs.

### Providers' widgets

The plate, the provider's name as a title line (subheadline semibold
`colour-text-2`, `space-xs` above the template) unless turned off, then
the template filling the inner area:

| Template | Look |
|---|---|
| Text | `type-style-body` `colour-text`, wrapping, cut at the plate's foot |
| Number | The value in `type-style-figure` (`type-style-title` under two rows), its unit and label in subheadline `colour-text-2` |
| Gauge | System's ring, the arc in `colour-text` (one value has no series) |
| Sparkline | A 1.5 line in `colour-text` over its value |
| Graph | System's graph, series a to d in order |
| List | Notifications' rows: optional glyph or icon at 16 or 20, label, trailing value in `colour-text-2` |
| Card | Title in `type-style-headline`, body in callout, an optional image at `radius-control` leading |

| State | Look |
|---|---|
| Stale | "Updated {time}" in `type-style-footnote` `colour-text-2`, trailing in the title line |
| Failing, no data | The strip: `warning-circle` 16 then the reason, `colour-text`, cut, the whole reason in its tooltip and accessible name; "Try Again" in its menu |
| Not allowed, not installed | The strip: "“{widget}” isn't allowed to run." with "Allow…", "“{widget}” isn't installed." with "Remove", as small buttons trailing |

### Edit mode

| Part | Look |
|---|---|
| Cells | Every free cell a `size-widget-cell` square of `colour-fill`, `radius-control` |
| A collapsed widget's place | Its whole extent as `colour-fill` at `radius-panel`, so its size can be read and grabbed |
| Remove badge | On every widget: `notifications.md`'s close form (20, `material-solid`, `shadow-control`, `colour-hairline` edge) holding `minus` 12, centred on the top leading corner's curve, 5 in from each edge; hit box 24 |
| Resize handles | On every widget in edit mode, `colour-text` on the hovered or selected one and `colour-text-2` elsewhere: a capsule 4 × `size-control-small` centred on each edge, 4 inside it; the trailing bottom corner a 3-wide arc on the corner's curve, 4 inside. Hit bands `size-resize-band` along the edges and 24 square at the corner |
| Selected | The focus ring's form, also for pointer selection, as Control Centre's edit mode |
| Lifted | The widget at its size under the pointer, full opacity, `shadow-panel`; no scale. Over the gallery, 50% opacity: release removes it |
| Ghost | Where it will land: `colour-fill` at `radius-panel` with a `stroke-hairline` `colour-hairline-strong` edge. Refused: no fill, a `colour-danger` edge and `prohibit` 24 in `colour-text` centred |
| Settings popover | `conventions.md`'s popover on `material-solid` (opened from a plate), `radius-card`, `shadow-menu`, padding `space-m`, at least 200 wide, `space-s` beside the widget's trailing edge, flipped to fit; rows built from `settings.md`'s control types without a group card. The anchor keeps its selected ring |
| System's editor | The consent panel's frame at `size-form-narrow` (480), `material-solid` (it sits over plates), its series as `settings.md` rows on `colour-fill` groups |

### The gallery

A sidebar `size-panel` (376) wide: inset `space-l` from the leading edge,
its top at `size-strip` + `space-l`, its foot `space-l` above the screen's
bottom; `radius-panel`, `shadow-panel`, padding `space-l` (inner 344). It is
over the plates, so it is `material-solid` with `shadow-panel` and no blur
pass, whatever the glass position, never glass on glass. Its menus are
`material-solid`.

| Part | Look |
|---|---|
| Search | Control Centre's gallery search: `size-control`, `colour-fill`, `radius-control`, `magnifying-glass` 12, "Search widgets" in `colour-text-2` |
| Sections | `space-s` under: the segmented component full width at `size-control`, its plate `colour-fill`, the chosen segment `material-solid` with `shadow-control`; "Installed" carries updates waiting as the quiet badge after its label |
| Groups | Headings in `notification-centre.md`'s heading style |
| Item | The widget's picture at its default size, scaled down to 344 wide when wider, live, drawn on `colour-fill` instead of a plate (as a banner becomes a card in the centre); `space-xs`; its name (body `colour-text`) over its sizes, "2 × 1, 2 × 2, 4 × 2" (subheadline `colour-text-2`). Two-column pictures pair across the 344; wider ones take a row. Placed and single: "Added" after `check` 12 in place of the sizes. Hovered: `notifications.md`'s close form holding `plus` at the picture's top leading corner |
| No room | The sizes line becomes "No room for “{widget}”. Remove or resize a widget to make space." in callout `colour-text-2`, wrapping |
| Yours | "New widget…" as a regular button at the top; each item adds "Check widget" and "Update picture" as small buttons under its name; "Tag release…" and "Show in Files" in its menu |
| Browse | "Sources…" as a regular button at the top, as "New widget…" in Yours |
| Browse item | Its picture, name, "{author} · {version}" (subheadline `colour-text-2`), then a permission summary: each declared kind's glyph at 12 in `colour-text-2` (`globe`, `folder`, `pencil-simple`, `plugs-connected`, `terminal-window`), the first in words, cut, then "+{n}" as the quiet badge; trailing Install or Update as a small button |
| Panels over it | Details and Sources replace the gallery's contents in the same shape, with a Back button (`caret-left`, plain, `size-control`) at the top leading corner and their title in `type-style-title-3` |
| Details | Icon 48 or the picture; name; the description in body `colour-text`; "{author} · {version} · {licence}" and the source in subheadline `colour-text-2`; Install, Update or Remove as regular buttons; sizes; pictures in a row; the README and changelog in body; "What it runs" in `prompts.md`'s details box (`colour-fill`, `radius-small`, `type-mono` `colour-text-2`); permissions as grant rows, read only (below) |
| Sources | A card per source, rows edge to edge: the address in body `colour-text` cut in the middle over "Last updated {time}" or the error (`warning-circle` 12 then the sentence, `colour-text`), subheadline; then "Allow widgets from this source to run" with the switch component trailing. Under the cards, `settings.md`'s list foot (+ and −); + opens a popover with an address field (`size-row-field`) and Add as the default |

### The consent panel

`prompts.md`'s frame (376, `radius-panel`, `shadow-panel`, padding
`space-xl`) on `material-solid`, since it sits over plates, centred on the
dashboard's screen, its top at a fifth of the height, over `colour-scrim`
on that screen alone. Top to bottom: the provider's icon at 48 or `plug` at
48 Light; the title, "Install “{widget}”?", in `type-style-title-3`
centred; "It runs {command}." in callout `colour-text-2` with the command
in `type-mono` `colour-text`; `space-l`; "It asks to:" in the heading
style; a box of `colour-fill`, `radius-small`, holding one grant row per
permission; the source under it in subheadline `colour-text-2`; for a
source not yet allowed, "This also allows other widgets from {source} to
run." in callout `colour-text`; then Cancel and Install with the prompts'
armed moment (`timing-prompt-arm-ms`), Install taking the accent once
armed. Focus is on Install from the first frame; until armed, Return and
Space on it do nothing, and arming moves no focus. Updating with new
permissions and allowing one's own use the same frame with their titles and
buttons.

### Grant rows

One row per permission, in words: a glyph at 16 in `colour-text-2`
(`globe` Connect to, `folder` Read, `pencil-simple` Read and change,
`plugs-connected` Use, `terminal-window` Run), `space-s`, the sentence in
`type-style-body` `colour-text`, a host or path cut in the middle.

- **Settings › Privacy & Security › Providers**: `settings.md`'s control
  rows with the switch component trailing; on is granted.
- **Read only** (gallery details, Settings' provider panes, Keychain): the
  state, "Allowed" or "Not allowed", trailing in `colour-text-2`, then
  "Change…" as a plain button.
- **Consent panel**: `size-row` rows, nothing trailing.

### The provider row

One layout in two densities: on glass (Control Centre's VPN and Phone
pages, Spotlight's locked-vault row) and in windows (Settings' provider
panes, Keychain's providers).

| Part | On glass | In a window |
|---|---|---|
| Icon | `size-app-icon-row` (20) | `size-app-icon` (32) |
| Name | `type-style-headline` `colour-text`, `space-s` after the icon | the same, `space-m` after |
| Sentence | Under the name: `type-style-subheadline` `colour-text-2`, one line, cut at the end, whole in the tooltip; kept while a call runs | the same |
| Switch | Trailing at inset `space-m`, centred on name and sentence, where the place has one | the same |
| Actions | While on, `space-s` under the sentence at the text column, in the provider's order: up to two as small buttons, `space-s` apart; any more under "Options", a pop-up button (`caret-down` 12) after them. Never trailing. `terminal` actions and sign-ins end in "…". A running action is disabled; after `timing-progress-delay-ms`, the spinner at 12 leads its label | the same as regular buttons at `size-control` |
| List | Under it, `size-row` rows indented to the text column, `control-centre.md`'s device circles and `check` | `settings.md`'s list rows |
| Error | The sentence's place: `warning-circle` 12 then the reason in `colour-text`, one line, cut; actions kept; "{provider} didn't answer. Try again later." past its time limit | the same |
| Height | Name and sentence plus `space-m` above and below (40 at least), plus the actions' line | likewise |

## States

| State | What changes |
|---|---|
| Closed | Nothing drawn |
| Open | Backdrop, plates, the two controls; the ring on the first widget only after a key |
| Edit mode | Cells, badges, the gallery, "Add widgets" and "Done" |
| Dragging, resizing | The lifted widget or growing frame, the ghost, neighbours flowing; the gallery faded while one is dragged from it |
| Empty | "No widgets" in `type-style-title-3` `colour-text-2`, centred on the screen; the controls |
| Full | The gallery's "No room for “{widget}”…" line; a drag's ghost refused |
| Loading | Last data in the first frame; title only where none |
| Error | Per widget, its strip; Sources' card shows a failed pull |
| Second screen | Its own grid (21 × 10) and layout, same cells |
| Windowed game, Game Mode | Plates, gallery, panels `material-solid`; the backdrop stays (a picture); no transitions |
| Full-screen game, locked | Not shown. A prompt or consent: its scrim over the dashboard, which stays beneath |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility |

## Motion

Exits run quicker than entrances; content rides its plate's alpha, never
its own clock. Nothing slides in from an edge and nothing rises.

| Transition | Spring or fade | From → to |
|---|---|---|
| Open | `motion-fade-ms`; `motion-open` | The backdrop's alpha 0 → 1 over the windows; from its halfway point every plate together scales from `motion-panel-scale` about its own centre, alpha min(1, 2p). The controls fade with the plates. No stagger |
| Close | `motion-exit-fade-ms`; `motion-exit` | Plates scale to `motion-panel-scale` about their own centres on `motion-exit` as they and the controls fade to 0; from their half the backdrop to 0. Focus returns in the first frame |
| Edit mode in | `motion-open` | The gallery revealed by a clip from its leading edge, laid out in place, content by 2p − 1; cells and badges by the same p. The controls' capsules spring their widths, labels crossfading by p |
| Edit mode out | `motion-exit` | The reverse |
| Drag | Direct; `motion-move` | The lifted widget follows the pointer 1:1. The ghost moves cell to cell on `motion-move`; neighbours flow on `motion-move` from the layout before the drag, all at once; dragging back returns them on the same spring |
| Drop | `motion-move` | The widget from the pointer into its ghost; its shadow eases to `shadow-menu` with p. Refused: back to its old place on `motion-move` |
| Resize | `motion-move` | The frame a cell at a time; where the size's layout changes, old content 1 − 2p, new 2p − 1 |
| Add by click | `motion-open` | At its place, as Open's plates; selected |
| Remove | `motion-exit-fade-ms` | Out in place; neighbours stay, since nothing moves by itself |
| To and from empty | `motion-exit`; `motion-open` | The plate's height to the strip's and back, top fixed; content crossfades by p |
| Data arrives, track changes | `motion-fade-ms` | Crossfade in place; figures change in one frame |
| Gallery during a drag | `motion-exit-fade-ms`; back `motion-fade-ms` | Fades out as the drag leaves it; returns after the drop |
| Popover, consent panel, System's editor | as `conventions.md`, `prompts.md` | Popover from 0.96 on `motion-open`; panels from the top centre with the scrim |

**Reduce Motion.** Open and close are fades of `motion-reduced-fade-ms`, the
backdrop and plates together, no scale. The soft push's moves become cuts
while the ghost still shows where things land; the lifted widget still
follows the pointer. Edit mode, resizes and panels crossfade in place.

**Game Mode.** No transitions: every change in one frame; `material-solid`.

## Accessibility

- **Colour roles.** On the washed backdrop and on plates, only `colour-text`
  and `colour-text-2`; `colour-text-3` nowhere, the gallery and panels
  included. `series` colours and the Timer's accent are graphics beside
  their figures, never the only reading. Low batteries and full disks show
  by glyph and weight. `contrast.py` already checks the lock's backdrop.
- **Names.** The dial is named for the time and city ("14:05, Tokyo"); each
  weather glyph by its condition ("Light rain"); a ring or graph by its
  series and value ("CPU 34%").
- **Increase Contrast.** The wash steps to `material-lock-wash-contrast`;
  plates `material-glass-tinted` with a `stroke-hairline` edge of
  `colour-hairline-strong`; the same edge on cells, the ghost, handles,
  badges, gallery items, the controls and every tile; glyphs Bold; the
  scrubber's knob at rest.
- **Reduce Transparency.** The backdrop stays; plates, the gallery and
  panels `material-solid`; tiles and pictures keep `colour-fill` on them.
- **Focus ring.** `stroke-focus-ring` at `stroke-focus-offset`, keyboard
  only: round a plate at `radius-panel` plus the offset, held by the 16
  gaps; round a control inside a widget at its own radius; round the
  capsules and circle; round a gallery picture.
- **Largest text size.** Cells keep their size; content falls through its
  parts to the one-line form (Media drops the art, then the artist, and the
  transport stays; Weather the condition; Calendar shows fewer rows). The
  figure style grows with the text. The gallery, popovers and panels grow;
  the gallery's pictures keep their size.
- **Hit targets.** Plates whole; transport 30; small buttons 20 with a 24
  hit box; badges 20
  with a 24 box; handles' bands 8 and the corner 24; Session's circles 40.
  All at or above `size-hit-minimum`.

## Board

At 3440×1440 with the 1920×1080 screen beside it, dark first and each in
light, over Frost, Ridge and a near-white photograph. Whole screens scaled;
crops at actual size: the default block `[920, 440, 2520, 980]`, the
gallery `[0, 0, 1600, 1440]`, the controls `[920, 1100, 2520, 1440]`.

1. **Open, default layout**: whole screen and the block crop.
2. **Widget sheet**: every built-in in each size with live content, on a
   stage crop of 1600 wide; Clock digital and analogue, day and night.
3. **Empty forms**: Media, Devices, Calendar with no calendars, Notifications
   as strips beside full widgets.
4. **Providers**: each template; stale, failing, not allowed, not installed.
5. **A full canvas**: the ultrawide's outer columns in use.
6. **Edit mode**: cells, badges, a selected widget's handles, the settings
   popover, the gallery on Installed.
7. **The soft push**: a drag mid-way with the ghost and three neighbours
   flowing; dragging back; a refused place at the screen's edge; a resize.
8. **Gallery**: Installed with "Added" and an update count; Yours with the
   author tools; Browse with permission summaries; "No room"; Details;
   Sources with a failed pull.
9. **Consent**: install from a new source; an update asking for one more
   permission; allowing one's own.
10. **Provider row**: on glass (VPN page) and in a window (Settings),
    signed out with actions, on with a list, an error, an action running.
11. **Grants**: the Privacy & Security page with switches; the read-only
    rows in Details.
12. **Empty dashboard**; **second screen**; **windowed game** (solid);
    scene 1 under Increase Contrast, Reduce Transparency, largest text.
13. **Motion**: open and close; edit in and out; a drag with the push; a
    resize; a widget falling to its strip; each with Reduce Motion and
    Game Mode.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `widget-cell` | 72 (the dashboard's cell; gaps are `space-l`) |
| `type` | `style.figure` | size 40, weight 600, line 44 (a widget's one large figure: time, temperature, date, timer) |

`material-lock-wash` and its contrast form now serve the dashboard too.
Phosphor glyphs to vendor: the weather set above, `hard-drive`, `warning`,
`prohibit`, `globe`, `pencil-simple`, `terminal-window`, `sign-out`,
`arrow-counter-clockwise`, `calendar-blank`.
