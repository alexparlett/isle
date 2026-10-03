# Files

P1 stage 5, wave C. How Files, Quick Look and the Open and Save panels look
and move, in Frost. What they are, their states, keys and copy are
`structure/files.md` (binding); the window's anatomy, toolbar, sidebar rows,
tables, states in place, popovers and focus are `surfaces/conventions.md`'s,
sheets and alerts `surfaces/window-chrome.md`'s, Markup's tool row
`surfaces/capture.md`'s, and are reused here, not redrawn. Values are
`tokens.kdl` names; numbers not yet tokens are listed at the end.

## Anatomy

### The window

- **Default size** `size-files-width` by `size-files-height` (1400 by 860);
  the first time on the 3440×1440 screen the compositor centres it in the
  work area: x 1020, y 314. Minimum 560 by 400, which holds the widest
  sheet (Rename many, `size-form-narrow` plus `space-xxxl` each side).
- **Regions** as `conventions.md`: toolbar 52; sidebar 220, `colour-sidebar`;
  content `colour-view`; the preview inspector 260, `colour-window`. Under
  the toolbar, top to bottom over the content and preview only: the tab bar
  (two or more tabs), the scope bar (a search standing). At the content's
  foot, over the content only: the path bar, then the status bar, when on.

### The toolbar

| Part | Look |
|---|---|
| Back, forward | `conventions.md`'s pair, `caret-left` and `caret-right`; with no history the glyph is `colour-text-disabled` and the button takes no hover |
| Title | The proxy icon (the folder's own icon at `size-app-icon-small`, 16), `space-xs`, the folder's name in `type-style-headline`; "{n} items" under the name in `type-style-subheadline` `colour-text-2`, tabular, aligned with the name, the pair centred on 52. Hover over icon or name: a `colour-control-hover` plate `radius-control`, padding `space-xs`, which names it as a target. The Trash: `trash` glyph, "Trash"; a search: `magnifying-glass`, "Searching “{folder}”" or the saved search's name |
| Path menu | ⌘-click or secondary click on the title: the in-window menu (`menus.md`), its top `space-xs` under the plate, leading edges aligned; one row per folder from this one up to the computer's name, names only (no icons on menu items, brief), the current folder checked |
| View switcher | `conventions.md`'s segmented control, three segments: Icons `squares-four`, List `list-bullets`, Columns `columns`; 112 wide |
| Group and Sort | `space-xs` after it: an icon button 48 by 30 holding `rows` at 20 and `caret-down` at 12, `space-xxs` apart, both `colour-text-2`; its menu is the in-window menu: Use Groups, Group By and Sort By as View's, checks in the mark column |
| Jobs button | First in the trailing group, present only while a job runs, waits or has failed. A 30 icon button holding a ring `size-progress-ring` (18) across, `stroke-progress` (2): track `colour-fill-strong`, the running total's arc in `colour-accent` from twelve o'clock. Waiting: the ring holds, and a `size-mark` (6) dot of `colour-warn` sits on its top trailing edge. Failed with nothing running: `warning-circle` Fill at 20 in `colour-danger-text` instead of the ring. Open: the pressed plate |
| Empty | In the Trash only: `conventions.md`'s primary action, "Empty", disabled (`colour-text-disabled` label) while the Trash is empty |
| More, preview, search | `dots-three-circle`; the inspector toggle (`sidebar-simple` mirrored), its glyph Fill while the preview shows; the search field |

Narrowing as `conventions.md`: the centre group into », the title cut at its
end (the subtitle with it), search into its button, then the preview and
the sidebar collapse.

### The tab bar

`size-tab-bar` (30) tall over the content and preview, `colour-window-sunken`,
a `colour-hairline` under it. Tabs share the width equally down to
`size-tab-min` (140), then the bar scrolls sideways. A tab: its folder's
name in `type-style-callout`, centred, cut in the middle, `colour-text-2`;
1 by 14 `colour-hairline` rules between unselected tabs. The selected tab
fills `colour-view`, has no line under it (it joins the content) and its
label is `colour-text`. Hover shows `x` at 12 in a 16 square `radius-small`
at the tab's leading edge, inset `space-xs`, `colour-control-hover` under
the pointer. The + is a 30 square at the bar's trailing end, `plus` at 16
`colour-text-2`, outside the scrolling part. A tab under a drag takes the
drop ring (below).

### The sidebar

`conventions.md`'s rows and headers. Glyphs at 16, `colour-accent-text`:

| Row | Glyph |
|---|---|
| Recents | `clock-counter-clockwise` |
| Home, Applications, Desktop, Documents, Downloads | `house`, `app-window`, `desktop`, `file-text`, `download-simple` |
| Music, Pictures, Videos, Templates, Public | `music-notes`, `image`, `film-strip`, `file-dashed`, `users` |
| A bookmark; a saved search | `folder-simple`; `list-magnifying-glass` |
| The computer; an internal volume; removable; a network share | `desktop-tower`; `hard-drive`; `usb`; `network` |
| Trash, empty and full | `trash`; Isle-drawn `isle-trash-full` (the same can with paper over its rim) |

- **Headers** (Favourites, Locations): on hover over the header,
  `caret-down` (open) or `caret-right` (collapsed) at 12 in `colour-text-2`
  at its trailing edge less `space-s`, in a 20 hit square.
- **Volumes.** Mounted: as any row, Eject at the trailing accessory, `eject`
  at 16 in `colour-text-2` in a `size-control-small` square, pressed
  `colour-control-pressed`. Unmounted: glyph and label `colour-text-3`, no
  Eject. Mounting: Eject's place holds a 16 spinner in `colour-text-2`.
- **Nearly full** (past nine-tenths): a capacity mark before the
  accessory, `space-s` from it: a bar `size-capacity-mark` (20) by 4,
  radius 2, track `colour-fill-strong`, the used share in `colour-warn`. On
  the selected row, track `colour-on-accent` at 35% and fill
  `colour-on-accent`. Free space is the row's tooltip.
- Drop and insertion: the drop ring of `conventions.md`; between two
  Favourites, an insertion line 2 points of `colour-accent` across the
  rows' inset width with a `size-mark` circle ringed 2 at its leading end.

### Icons view

| Part | Look |
|---|---|
| Icon size | `size-app-icon-large` (64) by default, 16 to 256 by 8 from View Options' slider, the status bar's or ⌘+ ⌘− (steps 16, 32, 48, 64, 96, 128, 192, 256) |
| Grid | Inset `space-l` from the content's edges; cells `size-icon-cell-pad` (48) wider than the icon (112 at 64, the Apps grid's tile); columns as many as fit, the remainder shared between the gaps, so the grid fills the width as Finder's; rows to content |
| Cell | `space-s`, the icon centred in its box (thumbnails fitted, never cropped, with a `colour-hairline` edge and `radius-small`; documents Papirus's mimetype icon; folders Isle's folder, `icons.md`); `space-xs`; the name in `type-style-body` `colour-text`, centred, at most the cell less `space-xs` each side, two lines, then cut in the middle |
| Item info | When on, a line under the name in `type-style-subheadline` `colour-text-2` ("12 items", "1920 × 1080") |
| Selection | Finder's, as Spotlight's Apps grid: a plate the icon plus `space-xs` round it, `radius-card`, `colour-fill-strong`; the name in a `colour-accent` capsule (padding `space-xs`, its lines' height plus 2, `radius-small`) in `colour-on-accent`. Unfocused or inactive: capsule `colour-selection-unfocused`, text unchanged |
| Band | From empty space: a rectangle of `colour-accent` at 15% with a 1-point `colour-accent` edge, square corners; items inside select as it passes |
| Group heading | With Use Groups: `size-row` (28), the group's name in `type-style-subheadline` semibold `colour-text-2` at the grid's inset, a `colour-hairline` under it across the content; `space-s` above the next cells |

### List view

`conventions.md`'s table: header 24, rows 28, stripes, joined selection.

- **Columns** at their defaults: Name (the rest, at least 240), Date
  Modified 168, Size 80, Kind 140; Date Deleted and Original Location in the
  Trash, Where in a search, Date Created and Date Last Opened from the
  header's menu.
- **Name cell**: indent `space-l` per level; a disclosure `caret-right` at
  12 in a 16 box `colour-text-2` (a 20 hit square) for folders, nothing for
  files; `space-xxs`; the icon at `size-app-icon-small`; `space-s`; the name
  in `type-style-body` `colour-text`, cut in the middle.
- **Other cells** in `colour-text-2`: dates in `type-style-body` with
  tabular figures, leading-aligned ("Today 14:02", "2 Oct 2026 at 09:15");
  Size in `type-style-mono`, right-aligned ("4 KB", "1.2 GB"); folders "--".
- **Groups**: a heading row as Icons' group heading; the stripes restart
  under each.

### Columns view

- **Columns** `size-column` (240) wide by default, at least
  `size-column-min` (160), `colour-view`, a `colour-hairline` between
  them with a `size-resize-band` drag area; Right Size fits the longest name
  plus the row's padding.
- **Rows** `size-row`, inset `space-xs`: icon 16, `space-s`, name
  `type-style-body` cut in the middle, and for a folder `caret-right` at 12
  in `colour-text-2` at the trailing edge less `space-s`. The selection in
  the focused column is `colour-accent`; the path's selections in the
  columns before it are `colour-selection-unfocused`.
- **The preview column**: last, `size-inspector` wide, `colour-window`,
  holding the preview's content (below); the view scrolls sideways so it is
  always whole in view.

### The preview

Padding `space-l`, content top-aligned and scrolling:

| Part | Look |
|---|---|
| Thumbnail | Quick Look's renderer fitted into the pane's width (228 at 260) and at most as tall, centred: images and video frames at `radius-control` with a `colour-hairline` edge; anything drawn as an icon (apps, documents without a renderer, folders) at `size-app-icon-hero` (128) |
| Name, kind | `space-m`; the name in `type-style-title-3` `colour-text`, centred, wrapping to three lines then cut in the middle; under it, `space-xxs`, "Markdown document, 4 KB" in `type-style-callout` `colour-text-2` |
| Information | `space-l`; "Information" in `type-style-subheadline` semibold `colour-text-2`; a `colour-hairline`; rows of `size-control-small` (20): the label leading in `type-style-callout` `colour-text-2`, the value trailing in `type-style-callout` `colour-text`, tabular, cut in the middle. Created, Modified, Last opened, then the kind's facts; a folder's size reads "Calculating…" in `colour-text-2` until counted |
| Show More | `space-xs` under the rows, trailing: a plain button in `colour-accent-text`, `type-style-callout` |
| Action | `space-l` below, centred: a regular button at `size-control`, `pencil-simple` at 16, `space-xs`, "Markup" (images); "Put Back" in the Trash |
| Several selected | Up to three thumbnails stacked, each `space-s` down and across from the one behind, the back ones at 60% and 80%; "{n} items" in `type-style-title-3`; the total size in `type-style-callout` `colour-text-2` |

### Item states

| State | Look, in each view |
|---|---|
| Being written | Icon and name at 50%; a bar `size-item-progress` (4) tall, `radius-small` 2, track `colour-fill-strong`, fill `colour-accent`: in Icons across the icon's box at its foot, inset an eighth of the box each side; in List and Columns in the Date Modified cell (or the row's trailing end in Columns), 96 wide, centred on the row |
| Cut, awaiting Paste | Icon at 50%, ringed by a dashed `stroke-hairline` outline in `colour-text-2` round its box at `radius-small` (dashes and gaps of 3); name unchanged |
| Hidden (shown by ⇧⌘.) | Icon at 50%, name `colour-text-2` |
| Alias | A badge on the icon's bottom leading corner: a square a quarter of the icon (at least 10), `colour-window-raised`, `radius-small`, `shadow-control`, holding `arrow-bend-up-right` in `colour-text` at three quarters of it; a broken alias's icon at 50% |
| Excluded by a panel's filter | Icon at 40%, name `colour-text-disabled`, no hover, no selection |

### Drag and drop

- **Drag image**: the dragged icons at their view's size, at 70%, names
  under them in Icons, stacked as the preview's stack past three; a count
  in the badge component's shape in `colour-accent` with `colour-on-accent`
  on the top trailing corner for two or more. The pointer carries Bibata's
  `copy` while the drop copies.
- **Drop targets** (only while accepting): a folder in Icons takes the
  selection plate in `colour-fill-strong` ringed 2 points (`stroke-found`)
  in `colour-accent`; a folder row in List or Columns the ring inside the
  row, `radius-row`; the folder itself (the view's empty space) the ring
  inset `space-xxs` round the content region, `radius-card`; a tab, a path
  bar segment or a sidebar row the ring inside it.
- **Spring-loading**: after `timing-spring-ms` (500) over a folder or tab
  the ring blinks off and on twice at `motion-blink-ms`, then the folder
  opens in place (Space opens it at once).

### Rename in place

The name becomes a field where it stood: `colour-view`, a 1-point
`colour-accent` inner edge, `radius-small`, padding `space-xxs`, text
unchanged in size and place; the base name selected (the `selection`
mix), the extension not. In Icons it is the cell's label width and grows to
three lines; in List it runs from the name's start to the column's end, 22
tall; in Columns, the row less `space-xs`. A taken name: the refused-value
form (`conventions.md`) in a callout under the field, `space-xs` below it,
`colour-window-raised`, `shadow-menu`, `radius-small`, padding 3 by
`space-s`. The dot and extension questions are alerts.

### The jobs popover

`conventions.md`'s popover from the jobs button, `size-jobs` (320) wide,
rows `colour-hairline` apart, five rows then scrolling. A row, padding
`space-s` 0: "Copying 3 items to “Backup”" in `type-style-body`
`colour-text`, cut in the middle; `space-xs`; the window's bar (6,
`conventions.md`) with a stop button `space-s` after it, `x-circle` Fill at
16 in `colour-text-2` in a 20 square; `space-xs`; "1.2 GB of 4.8 GB, about
2 minutes" in `type-style-subheadline` `colour-text-2`, tabular. Waiting:
the bar's fill `colour-text-3`, the third line "Waiting" in
`colour-warn-text` and a plain "Show" in `colour-accent-text` trailing,
which raises the question. Failed: `warning-circle` Fill 12 in
`colour-danger-text` before the first line, the third line the reason,
"Show". Done: the bar full, the third line "Done", gone after
`timing-job-done-ms` (2000).

### Questions and alerts

All `window-chrome.md` alerts on the job's window. **Name taken**: title "An
item named “{name}” already exists in “{folder}”.", the newer-item line
under it; the "Apply to all" checkbox centred on its own line `space-m`
above the buttons (only while more items remain); Stop, Keep Both (default,
accent), Replace (`colour-danger-text`), equal widths; with ⌥ held Keep Both
reads Merge in the same place. **Not enough space**, **eject in use** (Force
Eject destructive, leading), **Empty Trash** (the `warning` glyph: it cannot
be undone) and **failures** (the items as a list of 20 rows, at most five
showing, icon 16 and name cut in the middle, in a `colour-window-sunken`
well `radius-control` `space-s` under the informative line).

### Path bar and status bar

Each `size-control` (24), `colour-window`, a `colour-hairline` above, over
the content region. **Path bar**: from `space-s`, segments of icon 16,
`space-xxs`, name in `type-style-callout` `colour-text-2`, `caret-right` at
12 in `colour-text-3` between, the last segment `colour-text`; segments cut
in the middle from the root first. **Status bar**: "41 items, 212 GB
available" ("3 of 41 selected, 1.2 GB" with a selection) in `type-style-callout`
`colour-text-2`, tabular, centred on the content; in Icons the icon-size
slider (the component, small, no ticks) `size-icon-slider` (96) wide at the
trailing edge less `space-s`, `image` at 12 either side.

### Search

`conventions.md`'s scope bar with "This folder", "Everything under
“{folder}”", "Home"; trailing, a `plus` icon button at `size-control-small`
and "Save" a regular button at `size-control-small`. Criteria rows below it,
each `size-row-large` (40), `colour-window`, hairline under: pop-up buttons
at `size-control` (Kind, Last modified, Contents contain with a field
`size-row-field` wide), `space-s` apart; `minus-circle` and `plus-circle`
at 16 trailing. Results take a Where column: the enclosing folder's icon 16
and name in `colour-text-2`.

### Get Info

A window `size-info` (300) wide, height to content up to the work area,
then scrolling; `colour-window`; a server-style bar of `size-titlebar` with
the lights (zoom disabled) and "{name} Info" as its title.

- **Header**, padding `space-l`: the icon at `size-app-icon-medium` (48),
  `space-m`, the name in `type-style-headline` wrapping to two lines, size
  and Modified under it in `type-style-subheadline` `colour-text-2`.
- **Sections**: a `colour-hairline` inset `space-l` above each; a header row
  of `size-row`: `caret-down` or `caret-right` at 12 `colour-text-2`,
  `space-xs`, the name in `type-style-headline`. Inside, padding `space-l`
  sides and `space-s` under: label and value pairs in `type-style-callout`,
  labels trailing-aligned in a 96 column in `colour-text-2`, values
  `colour-text`, selectable, `space-xs` between lines.
- **Controls**: Name & Extension a text field at `size-control`; Comments a
  text view 64 tall, `colour-view`, `colour-hairline` edge, `radius-control`;
  Open with a full-width pop-up and "Change All…" a regular button under
  it, trailing; Sharing & Permissions a two-column table (Name, Privilege)
  with a pop-up at `size-control-small` per row and the "Allow running as a
  program" checkbox under it; disabled forms on items not the user's.
- **Preview**: the preview pane's thumbnail at the window's width less
  `space-l` each side.

## Quick Look

| Part | Look |
|---|---|
| Size | The content's own size, fitted inside `size-quick-look-max-width` (1280) by `size-quick-look-height-scale` (two thirds) of the work area's height (928 on the ultrawide), at least `size-quick-look-min-width` (480) by `size-quick-look-min-height` (320); text, code and PDF `size-quick-look-text` (720) wide; a file with no renderer the minimum. The frame is kept while moving between items (content refits inside it); the size the user gave it is kept for the next time |
| Place | Centred over its caller's window (Spotlight: the screen's work area), clamped inside the work area |
| Window | `radius-window`, `shadow-window`; content `colour-view` |
| Toolbar | `size-toolbar` (52), solid `colour-titlebar`, a `colour-hairline` under it. Leading: the lights (minimise disabled); with several items, `space-l`, `caret-left` and `caret-right` buttons, `space-s`, "3 of 12" in `type-style-subheadline` `colour-text-2`, tabular, and the index button `squares-four`. The file's name centred on the window in `type-style-headline`, laid after the leading items and cut in the middle as window titles are. Trailing, `space-xs` apart: the kind's icon buttons (Rotate Left `arrow-arc-left`, Markup `pencil-simple`, Trim `scissors`, chosen ones in the pressed plate), `space-l`, then "Open with {App}" as the primary action. No share button |
| Full screen | The toolbar shows on pointer movement and fades after `timing-controls-ms` (2000) at rest; content on `colour-view` edge to edge |
| Images | Fitted, no margin. Text and code: `type-style-mono` `colour-text`, padding `space-l`, wrapping off. PDF: pages on `colour-window-sunken`, `space-l` apart, each with `shadow-control`. Fonts: "The quick brown fox" at `type-style-large-title`, then 22, 17, 13 and the alphabet in `type-style-body`. Archives: the List view's table, read-only. A folder or unknown file: icon 128, `space-m`, name `type-style-title-3`, kind, size and dates in `type-style-callout` `colour-text-2`, centred |
| Audio and video | Controls on a capsule `size-control-large` (30) tall, inset `space-l` from the content's foot and sides (at most 480 wide, centred), `colour-window-raised`, `radius-card`, `shadow-menu`: `play` or `pause` Fill at 16, the slider component (accent fill), the time "0:42 / 3:10" in `type-style-callout` tabular `colour-text-2`, `speaker-high` at 16. Over video it fades after `timing-controls-ms` at rest; audio keeps it, under its cover or icon at 128 |
| Index sheet | The content becomes the Icons grid at 128 on `colour-view`, the shown item selected in Finder's form |
| Markup | `capture.md`'s tool row under the toolbar; the Markup button pressed |
| Trim bar | Replaces the controls: a band `size-trim-bar` (56) at the content's foot, `colour-titlebar`, a `colour-hairline` above, padding `space-m`: `play` button 30; `space-s`; the strip `size-trim-strip` (40) tall, `radius-control`, frames fitted end to end (audio: its waveform in `colour-text-2` on `colour-fill`); the kept range framed 3 points in `colour-accent` with handles `size-trim-handle` (10) wide at its ends (a 1 by 12 `colour-on-accent` grip each), the frames outside it under `colour-scrim`; the playhead a 2-point `colour-text` line with `shadow-control`; the kept length in `type-style-callout` tabular over the range's centre, above the strip; `space-l`; Revert and Done at `size-control`, as Markup's |

## Open and Save panels

| Part | Look |
|---|---|
| Sheet | `window-chrome.md`'s sheet with no padding of its own, the parts below edge to edge. Open and expanded Save at `size-chooser-width` by `size-chooser-height` (760 by 480) or the size remembered for the app, clamped to the parent's width less `space-xxxl` each side; at least `size-chooser-min-width` by `size-chooser-min-height` (600 by 380). Too small a parent: centred over it, `radius-window` all round, `shadow-window`, the parent under `colour-scrim`. No parent: a window of its own centred on the focused screen, a `size-titlebar` bar with the lights (minimise and zoom disabled) and the request's title |
| Title line | When the request has a title and the panel is a sheet: `size-titlebar` (32), the title in `type-style-headline` centred |
| Navigation row | `size-row-large` (40), padding `space-m` across, `colour-window`, `colour-hairline` under: back and forward at `size-control` (24) squares; `space-s`; the view switcher at `size-control` (segments 32); `space-s`; the folder pop-up (the component, 200 wide, its menu the path menu's rows); search at `size-toolbar-search` and `size-control`, trailing |
| Browser | Sidebar at `size-sidebar-min` (160), `colour-sidebar`, rows as Files'; the content in the window's own views (Icons, List, Columns with the preview column), at the same sizes |
| Foot | `colour-window`, `colour-hairline` above, padding `space-m` `space-l`. A row of "Show:" (`type-style-body`) and its pop-up, then the request's choices, `space-l` apart, leading; `space-s` under it, New Folder (regular, `size-control`) leading and Cancel then Open (default, accent; "Choose", "Save" or the request's label) trailing, `space-s` apart. One row when there is no filter or choice. The default is disabled until a choice is made |
| Save, collapsed | `size-chooser-save` (520) wide, padding `space-l` `space-xl`: rows `space-s` apart of a label column `size-chooser-label` (80) trailing-aligned in `type-style-body` `colour-text` and controls at `size-control`: "Save As:" a text field 320 wide, base name selected; "Where:" a pop-up 320 wide (current folder, recent folders, then Favourites, each section under a menu section label) and, `space-s` after it, the expand button: a 24 square `colour-control` `radius-control`, `caret-down` at 12; "Format:" a pop-up when the request has filters. No Tags row. Choices, then New Folder, Cancel and Save, `space-l` below |
| Save, expanded | Save As alone on a row of `size-row-large` centred, field 280; then the navigation row (Where's pop-up becomes its folder pop-up), the browser, the foot with Format and choices; the expand button shows `caret-up` at the Save As row's trailing end |
| Replace step | The sheet's content becomes an alert's (title, "Replacing it overwrites it.", Cancel default, Replace in `colour-danger-text`) and the sheet takes `size-alert` width |

## States

| State | What changes |
|---|---|
| Rest | The window as Anatomy, at its folder; no jobs button |
| Loading | Frame and arriving rows; the subtitle's count grows; `conventions.md`'s spinner after `timing-progress-delay-ms` |
| Many items | As loading; kinds' icons until thumbnails arrive, which replace them in place with no fade |
| Empty folder | `conventions.md`'s Empty line: "No items"; Recents "No recent files"; Trash "No items in Trash" |
| No results | "No results for “{query}”"; the scope bar and criteria kept |
| No permission | The sentence in `type-style-body` `colour-text-2`, centred, wrapping at 400 |
| Folder gone | The same, with "Go to Enclosing Folder" (regular) `space-m` below |
| Slow volume | As loading; the window keeps answering |
| Work in flight | Writing items (Item states); the jobs button; the switcher card's bar |
| A job waiting | The jobs button's warn dot; the name-taken alert on the window; its row "Waiting" |
| A job failed part way | The failures alert; the jobs button's `warning-circle` until the popover is seen |
| No window, job running | No look of its own; activating Files opens a window with the jobs popover open |
| No window focused | `menus.md`'s bar with Files' name |
| Narrow window | Toolbar folds; preview then sidebar collapse; a panel clamps, then centres |
| Trash | The Trash's title, columns, Empty; Open disabled; the preview's Put Back |
| Quick Look open | Its window over the caller; the caller's selection stays in the accent (it keeps focus) |
| Second screen | No change; Quick Look and panels on their caller's screen |
| Game in front | No change of look; no transitions |
| Locked, Focus on, configuration error | No change of their own (`conventions.md`) |
| Reduce Motion, Transparency, Contrast, largest text | Motion and Accessibility |

## Motion

Content rides its spring's progress; exits on `motion-exit` and
`motion-exit-fade-ms`. Within the window, moving is instant: Files is a
tool for many quick steps.

| Transition | How | Reduce Motion |
|---|---|---|
| Folder navigation, back, forward, path menu | Cut: the new folder's frame in the next frame, its scroll position and selection restored | Same |
| View switch | Cut; the selection kept and scrolled into view; the segment's plate moves on `motion-move` | Cut |
| Icon size | The grid lays out every frame with the slider or step; no spring | Same |
| Disclosure | The caret turns 90° on `motion-move`; rows below move down on `motion-move`, children fading in by 2p − 1; closing on `motion-exit`, children 1 − 2p | Cut |
| A new column (Columns) | Appears at once; the view scrolls to keep the last column whole on `motion-move` | Jump |
| Tab bar appearing, a tab opening or closing | Bar height 0 to 30 on `motion-move`, content riding it; a new tab's width from 0, the others giving way, on `motion-move`; a closed tab out on `motion-exit`, its label fading 1 − 2p | Cut |
| Preview, sidebar | `conventions.md` | As there |
| Items arriving, new folder | In place at once; the rename field open in the same frame | Same |
| Items leaving (trashed, moved) | Out over `motion-exit-fade-ms`; the rest close up on `motion-move` | Cut |
| Writing finishes | Bar out over `motion-exit-fade-ms`; icon and name to full over `motion-fade-ms` | Same |
| Jobs button | In over `motion-fade-ms` after `timing-job-delay-ms` (1000), out over `motion-exit-fade-ms`; the trailing group's items slide over on `motion-move`; the ring follows the total with no spring | Fade only |
| Drag cancelled or refused | The image flies back to its source on `motion-move` | Fades out over `motion-reduced-fade-ms` |
| Drop ring, insertion line | At once | Same |
| Spring-loading | Two blinks, then a cut | No blink: the ring holds, then a cut |
| Popovers, menus, sheets, alerts | `conventions.md`, `menus.md`, `window-chrome.md` | As there |
| Quick Look in | From the item's rectangle (the icon's box, a row's icon or a Spotlight row's icon) to its frame on `motion-open`: the item's image scaled with the frame; the window's fill and toolbar by 2p − 1; the rendered content crossfades over the image in the last half. With no rectangle to hand (scrolled out, a D-Bus caller without one): the window's own open, from `motion-window-scale` about its centre | Fade in place, `motion-reduced-fade-ms` |
| Quick Look out | Back to the item's current rectangle on `motion-exit`, toolbar and fill out by 1 − 2p; none to hand, the window's close | Fade out in place |
| Between items | Content swaps in one frame inside the kept frame; title and count change with it | Same |
| Index sheet | Crossfade over `motion-fade-ms` | Same |
| Markup row, trim bar | Height from 0 on `motion-move`, content riding it | Cut |
| Video controls, full-screen toolbar | In over `motion-fade-ms`, out over `motion-exit-fade-ms` | Same |
| Panel | The sheet's: down from the parent's toolbar on `motion-open`, out on `motion-exit`; centred and windowed forms as the window's open and close | Fade in place |
| Save expand, collapse | The sheet's size on `motion-open` (expand) or `motion-exit` (collapse), hung from its top centre; the browser clipped by the reveal and fading in by 2p − 1, out by 1 − 2p | Crossfade at the new size |
| Replace step | Content crossfades over `motion-fade-ms` while the sheet's width moves on `motion-move` | Cut with the crossfade |

**Quick Look's zoom against brief P5.** Kept, as `structure/files.md`
fixed it: the window opens from the thing it shows, so the motion says
where it came from; it does not slide in from elsewhere, and its
fallback is the in-place open.

**Game Mode:** every row is a cut; nothing fades.

## Accessibility

- **Surfaces.** All window roles: text on `colour-view`, `colour-window`,
  `colour-window-sunken`, `colour-window-raised`, `colour-sidebar`,
  `colour-titlebar`; `colour-text-3` only for placeholders, unmounted
  volumes, the path bar's separators and an inactive window's title and
  glyphs. On `colour-accent`, `colour-on-accent` only. Nothing in Files,
  Quick Look or the panels is glass.
- **Never by colour or opacity alone.** Dimmed states carry their meaning
  elsewhere too: writing items their bar, cut items their dashed outline,
  aliases their badge, hidden files their leading dot, excluded items
  their refusal to select (and their accessible description); the waiting
  job its word.
- **Increase Contrast.** `conventions.md`'s borders, plus: a
  `colour-hairline-strong` edge round icon selections' plates and capsules,
  thumbnails, the tab bar's tabs, the drag image, the item progress track,
  the capacity mark, Quick Look's window and controls capsule and the trim
  strip; dimmed items at 65% rather than 50%; the drop ring at 3.
- **Reduce Transparency.** No change: Files has no translucent part.
- **Focus.** Lists, grids, columns and the sidebar show focus as the
  accent selection; with nothing selected, the first item's ring is inside
  its row or round its plate. Tab and toolbar controls take the ring outside
  themselves. Quick Look takes no ring until Tab moves into its toolbar;
  then its controls do. In a panel, Tab's order is name field, sidebar,
  content, choices, buttons, each with its own form.
- **Largest text size.** Rows, the tab bar, path and status bars and Get
  Info's rows grow to their line plus 12; Icons' cells widen by the text
  factor, names keep two lines then a middle cut; List keeps its column
  widths and cuts in the middle; Save's label column grows; glyph and icon
  boxes keep their size.
- **Targets.** Toolbar items 30; rows 28; disclosure, tab close, Eject,
  stop and the header caret 20 squares (`size-hit-minimum`); trim handles 20
  wide centred on their 10; column dividers and edges 8.

## Board

Wave C's Files board at 3440×1440 with the 1920×1080 screen beside it, dark
first and each scene in light. Crops `[x0, y0, x1, y1]`: the window
`[920, 264, 2520, 1224]`; the toolbar `[920, 300, 2520, 480]`; Quick Look
`[920, 200, 2520, 1240]`.

1. **Default window**: List in `~/Development/hyprland`, sidebar complete
   (saved search, Backup nearly full, an unmounted volume, Trash full), the
   preview on a Markdown file. Whole screen at fit; the window crop.
2. **Toolbar strip**: rest; the title's hover and path menu; Group and Sort
   open; the jobs button running, waiting and failed; Trash with Empty; a
   search with the scope bar and two criteria; the four narrowings.
3. **Icons**: at 64 with a focused selection of three, unfocused, item
   info, an alias, hidden files, a cut item, a writing item, a band in
   progress; the same folder at 32 and 128; groups by Kind.
4. **List**: two levels disclosed, folders first, sizes, stripes, a joined
   selection, a writing row; groups by Date Modified.
5. **Columns**: four levels and an image in the preview column; a
   divider's menu.
6. **Preview**: an image with Markup, a folder calculating, an app, three
   selected, a Trash item with Put Back.
7. **Tabs**: three tabs, hover close, twelve scrolled, a drop on a tab.
8. **Drag and drop**: three items with their count over a folder in
   Icons, a row in List, a sidebar place, between Favourites, the whole
   view; a spring-load's two blinks.
9. **Rename**: in Icons and List, the taken-name callout, the dot alert,
   the extension alert, Rename many's sheet.
10. **Jobs**: the popover with running, waiting, failed and done rows; the
    name-taken alert with Apply to all; Merge under ⌥; not enough space;
    the failures alert.
11. **Trash**: its columns and Empty; the Empty alert; "No items in Trash".
12. **States in place**: empty, no results, no permission, folder gone,
    loading with the spinner, Recents empty. Six windows tiled.
13. **Bars and Get Info**: path and status bars in Icons with the slider;
    Get Info for a file with every section open, a volume, an AppImage's
    permissions.
14. **Quick Look**: an image over Files (Open with, Rotate, Markup), code,
    a PDF, a video with its controls, an unknown file, "3 of 12" and the
    index sheet; from Spotlight over Spotlight; full screen.
15. **Markup and Trim**: the tool row on a screenshot; a recording with the
    trim bar and a shortened range.
16. **Open panel**: a sheet on a Zed window, List with a filter dimming
    items and a choice; Columns with the preview; on the 1080p screen.
17. **Save panel**: collapsed with Where open; expanded; the Replace step;
    the parentless window; centred over a small parent.
18. **Accessibility**: scenes 1, 3 and 17 under Increase Contrast and at the
    largest text size.
19. **Motion**, on the harness's spring and as five-frame strips: Quick
    Look in and out from an Icons cell and a List row, its fallback; a
    disclosure; a tab opening; an item trashed; the jobs button arriving; a
    drag cancelled; Save expanding; each under Reduce Motion and Game Mode.

## New tokens

| Group | Token | Value |
|---|---|---|
| `size` | `files-width`, `files-height` | 1400, 860 |
| | `tab-bar`, `tab-min` | 30, 140 |
| | `progress-ring` | 18 |
| | `capacity-mark` | 20 (its width; 4 tall) |
| | `icon-cell-pad` | 48 (an Icons cell's width beyond its icon) |
| | `column`, `column-min` | 240, 160 |
| | `item-progress` | 4 |
| | `jobs` | 320 (the jobs popover's width) |
| | `icon-slider` | 96 |
| | `info` | 300 (Get Info's width) |
| | `quick-look-max-width`, `quick-look-min-width`, `quick-look-min-height`, `quick-look-text` | 1280, 480, 320, 720 |
| | `quick-look-height-scale` | 0.667 (of the work area's height) |
| | `trim-bar`, `trim-strip`, `trim-handle` | 56, 40, 10 |
| | `chooser-width`, `chooser-height` | 760, 480 |
| | `chooser-min-width`, `chooser-min-height` | 600, 380 |
| | `chooser-save`, `chooser-label` | 520, 80 |
| `timing` | `spring-ms` | 500 (spring-loading's dwell) |
| | `job-delay-ms`, `job-done-ms` | 1000, 2000 |
| | `controls-ms` | 2000 (media controls and Quick Look's full-screen toolbar at rest) |

Isle-drawn for `icons.md`: `isle-trash-full`, and Isle's folder icons
(plain and the special folders), which `icons.md` already gives to Files.
