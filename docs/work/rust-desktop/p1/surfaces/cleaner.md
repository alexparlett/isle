# Cleaner

P1 stage 5, wave C. How Cleaner looks and moves, in Frost. What it is, its
views, states, keys and copy are `structure/cleaner.md` (binding); the
window's anatomy, toolbar, sidebar rows, tables, states in place, the
inspector and focus are `surfaces/conventions.md`'s, sheets and alerts
`surfaces/window-chrome.md`'s, the System card's authorisation
`surfaces/prompts.md`'s, and are reused here, not redrawn. Values are
`tokens.kdl` names; numbers not yet tokens are listed at the end.

## Anatomy

### The window

- **Default size** `size-cleaner-width` by `size-cleaner-height` (1200 by
  760), minimum 720 by 480; first placed centred in the work area (x 1120,
  y 340 on the ultrawide).
- **Regions**: the toolbar (52); the sidebar (220, `colour-sidebar`); the
  content, `colour-window` for Overview and Cleanup (a column of
  `size-cleaner-column`, 760, centred, `space-xxl` at the sides at least,
  `space-xl` from the toolbar) and `colour-view` for Space and the tables;
  the basket, `conventions.md`'s inspector (260, `colour-window`), while it
  holds anything.
- **Toolbar.** Leading: the lights, the sidebar toggle, back and forward,
  the title (the view) in `type-style-headline` over the subtitle in
  `type-style-subheadline` `colour-text-2`, tabular ("All disks · 412 GB
  scanned", "Scanning… 61%", "Partial scan"). Centre: the scope, a pop-up
  button at `size-control-large`, at least 140 wide; `space-l`; the view's
  own controls: Space's "Map" and "List" as a text segmented control
  (`monitor.md`'s form); Duplicates, Large files and Untouched a Kind
  pop-up; Untouched the period pop-up after it. Trailing: "Rescan", the
  primary action (a regular button, "Stop" while scanning); More; the basket
  toggle (`sidebar-simple` mirrored, Fill while shown); search.
- **Scan progress.** While a scan runs, a `stroke-progress` (2) line of
  `colour-accent` along the toolbar's lower edge over the content region,
  from its leading edge, its length the scanned share of the scope's used
  space; no track. It is the one place the scan shows as live.

### Kinds

Each kind is a colour, a symbol and a name, together everywhere (D88). The
colours are data tokens, `kind-*` per appearance, never on a control.

| Kind | Token | Symbol | Dark | Light |
|---|---|---|---|---|
| Games | `kind-games` | `game-controller` | `#6a94f4` | `#3054ae` |
| Video | `kind-video` | `film-strip` | `#ffa659` | `#b36300` |
| Images | `kind-images` | `image` | `#2cdcaf` | `#048f70` |
| Audio | `kind-audio` | `music-notes` | `#cb73c4` | `#873482` |
| Documents | `kind-documents` | `file-text` | `#69caff` | `#0584b8` |
| Archives and disk images | `kind-archives` | `file-zip` | `#ff9bba` | `#be4d77` |
| Developer | `kind-developer` | `code` | `#79aa3b` | `#426801` |
| Apps and system | `kind-apps` | `package` | `#c9acff` | `#8962c5` |
| Caches and logs | `kind-caches` | `stack` | `#e6705f` | `#9e2e21` |
| Other | `kind-other` | `shapes` | `#ababab` | `#6f6f6f` |

Nine hues and a neutral Other, in this fixed order, which every bar and
legend follows. Chosen with the dataviz validator: adjacent pairs in this
order clear the colour-blind target (worst ΔE 9.9 dark, 10.4 light) and the
normal-vision floor (16.2, 16.6); `system/data-colours.md`.

| Form | Look |
|---|---|
| Kind tile | `settings.md`'s pane tile: `size-pane-tile` (20), `radius-row`, filled with the kind's colour, its symbol Fill at 16 in `colour-view` (dark glyph on the dark appearance's light fills, white on the light's) |
| Bar segment | The kind's colour, full; segments `space-xxs` (2) apart, the gap the surface behind |
| Map tile | The kind's colour mixed into `colour-view` at `kind-mix` (0.22), with a `stroke-kind` (3) band of the full colour along its top edge, clipped by its corners |
| Legend row | `size-row` (28): the kind tile, `space-s`, the name in `type-style-body`, the size in `type-style-mono` `colour-text-2` trailing |

### The sidebar

- **Views**: `conventions.md`'s rows, glyphs in `colour-accent-text`:
  Overview `chart-pie-slice`, Cleanup `broom`, Space `layout`, Duplicates
  `copy`, Large files `arrows-out-simple`, Untouched
  `clock-counter-clockwise`.
- **Disks**, fixed at the foot, `colour-sidebar`, a `colour-hairline` above,
  padding `space-m`. Each disk: `hard-drive` at 16 in `colour-text-2`,
  `space-s`, the name in `type-style-body`, cut at its end, the free space in
  `type-style-subheadline` `colour-text-2` trailing, tabular; `space-xs`
  under, a bar the row's width, 6 tall, `radius-small` 3: track
  `colour-fill-strong`, the used share `colour-text-2`, past nine-tenths
  `colour-warn` (as Files' capacity mark). `space-s` between disks. Then
  the Trash: `trash` at 16, "Trash", its size; "Empty Trash…" a small
  regular button (`size-control-small`) trailing.

### Overview

Top to bottom in the column, `space-xl` between sections:

1. **The scope**: its name in `type-style-title-2`; under it, `space-xxs`,
   used, free and scanned as figures in `type-style-body` `colour-text-2`,
   tabular, `space-l` apart.
2. **The bar**: `size-kind-bar` (20) tall, the column's width,
   `radius-small` at its ends: the kinds' segments in order, then Not
   scanned in `colour-fill-strong`, then free space in `colour-fill`. A kind
   under 2 points is drawn at 2.
3. **The legend**: two columns of legend rows, `space-xxl` apart, empty
   kinds left out; Not scanned and Free close it with plain tiles of their
   fills, ringed `colour-hairline`. Hover on a segment or a row shows the
   other ringed in `colour-hairline-strong`.
4. **Safe to clear**: a group (`settings.md`) with one row: `broom` tile in
   `pane-green`, "{size} safe to clear" in `type-style-headline`, "Review"
   a regular button trailing.
5. **Disks**: a group of rows of `size-row-large`: `hard-drive` at 20, the
   name over used of total in `type-style-subheadline` `colour-text-2`, the
   sidebar's bar at `size-row-slider` (200) trailing. "{n} folders couldn't
   be read" is the group's last row, `info` at 16 in `colour-text-2`.
6. **Largest folders** and **Largest files**: a group title in
   `type-style-headline`, then up to `limit-overview-rows` (5) navigation
   rows of `size-row-large`: kind tile, the name over its path (cut in the
   middle) in `type-style-subheadline` `colour-text-2`, the size in
   `type-style-mono` trailing, `caret-right` at 12.

### Cleanup

Three cards, `settings.md`'s groups, `space-xl` apart.

| Part | Look |
|---|---|
| Card head | 44 tall, padding `space-m`: a checkbox carrying the card's state (on, off or mixed), `space-s`, the name in `type-style-headline`, its total in `type-style-mono` `colour-text-2` trailing. System has no checkbox and "These can't be put back." under its name in `type-style-callout` `colour-text-2` |
| Row | `size-row-large` (40), padding `space-m`: checkbox, `space-s`, kind tile, `space-s`, the find's name (the app that made it) in `type-style-body` over a second line in `type-style-subheadline` `colour-text-2`: the path, cut in the middle; in Worth a look the reason, " · ", then the path. The size in `type-style-mono` trailing |
| More | Past `limit-cleanup-rows` (5): a row "{n} more" in `colour-accent-text`, a plain button, then "Show less" when open |
| System row | `size-row-large`: the name in `type-style-body`, its size in `type-style-mono` `colour-text-2`, then "Clear…" or "Remove…" a regular button in `colour-danger-text` trailing |

### Space

| Part | Look |
|---|---|
| Path | `size-row` (28) at the content's top, `colour-view`, a `colour-hairline` under it, padding `space-l`: the scope's `hard-drive` at 16 and name, then each folder, `caret-right` at 12 in `colour-text-2` between; crumbs are plain buttons in `type-style-body` `colour-text-2`, the current one `type-style-headline` `colour-text`; the current folder's size trailing in `type-style-mono` `colour-text-2` |
| Map | Inset `space-l` under the path; a squarified treemap of the folder's children, tiles `space-xxs` (2) apart, `radius-small` corners |
| Tile | The kind's map tile (Kinds), by the kind most of it is. At `size-tile-symbol` (32) square or more, the kind tile at its top leading corner inset `space-xs`; at `size-tile-label-width` by `size-tile-label-height` (96 by 44) or more, the name in `type-style-callout` semibold `colour-text`, cut at its end, over the size in `type-style-subheadline` `colour-text-2`, tabular, beside the kind tile |
| Smaller items | Children whose tile would be under `size-tile-min` (8) on a side share one tile, "{n} smaller items", in `colour-fill-strong` with no band; it does not open |
| Hover | A 1-point `colour-hairline-strong` inner ring at once; after `timing-tooltip-ms`, `conventions.md`'s tooltip: name, size, kind |
| Selected | `stroke-found` (2) ring in `colour-accent` inside the tile, inset `space-xxs`; unfocused, the same in `colour-hairline-strong` |
| In the basket | `check-circle` Fill at 16 in `colour-text` at the tile's top trailing corner, inset `space-xs`; in a tile under 32, the band thickens to 6 |
| List | `conventions.md`'s table: Name (disclosure `caret-right` 12, a 16 slot holding `check-circle` Fill in `colour-accent-text` when in the basket, the kind tile, the name), Size, Share (a bar 64 by 6, `radius-small`, track `colour-fill-strong`, the share of its parent in the kind's colour), Kind |

### Duplicates, Large files, Untouched

`conventions.md`'s tables with stripes, a leading checkbox column (16 wide),
the kind tile before each name.

- **Large files and Untouched**: Name, Folder (`colour-text-2`, cut in the
  middle), Kind, Last opened, Size (Large files sorts by it; Untouched by
  size too).
- **Duplicates**: each group a head row of `size-row-large` with no stripe
  and no checkbox: the kind tile, the name in `type-style-headline`, the size
  each and the count in `type-style-mono` `colour-text-2`; its copies under
  it as rows indented 36: checkbox, folder, modified; a copy sharing blocks
  shows "Shares storage" in `type-style-callout` `colour-text-2` after its
  folder, with `link` at 12. A `colour-hairline` between groups.

### The basket

`conventions.md`'s inspector, padding `space-l`.

| Part | Look |
|---|---|
| Head | "Basket" in `type-style-headline`; `space-xs`; the total in `type-style-title-2`, tabular; "{n} items" in `type-style-subheadline` `colour-text-2` |
| Items | `space-m` under; rows of `size-row` inset `space-s`: the kind tile, the name in `type-style-body` cut in the middle, the size in `type-style-mono` `colour-text-2` trailing; on hover and on the selected row the size gives way to `x-circle` Fill at 16 in `colour-text-2`, a 20 target, which takes it out |
| Foot | Fixed, a `colour-hairline` above, padding `space-l`: "Frees {size} on {disk} once the Trash is emptied", one line per disk, `type-style-callout` `colour-text-2`, wrapping; `space-m`; "Move to Trash", a regular button at `size-control-large`, the full width |
| Moving | The button gives way to `conventions.md`'s determinate bar (6) |
| Moved | Head and items give way to "Moved {n} items to the Trash" in `type-style-headline`, "Undo" a regular button `space-s` under; the foot to "Nothing is freed until the Trash is emptied" and "Empty Trash…" |

## States

| State | What changes |
|---|---|
| Scanning | The progress line; "Scanning… {n}%"; Stop for Rescan; views fill in place, the map re-laid at most once every `timing-scan-layout-ms` (1000) |
| Rest | The view; the basket if it holds anything |
| Empty view | `conventions.md`'s empty line in the content |
| No results | `conventions.md`'s line |
| Partial scan | Subtitle "Partial scan"; Overview's Not scanned segment and row; the progress line gone |
| Unreadable folders | Overview's disks group row |
| Moving to Trash, Moved | The basket's forms; the switcher card's bar |
| System clear refused | Nothing changes |
| Narrow | The basket folds into its toggle with a quiet badge of its count; then the sidebar; the map drops labels before symbols |
| Inactive | `conventions.md`'s inactive form; the progress line `colour-text-2` |
| Second screen, game in front, locked, Focus on | No change of their own |

## Motion

| Transition | How | Reduce Motion |
|---|---|---|
| Opening a folder in Space | **Zoom from the tile** (structure; brief P5: the folder opens from where it was). The tile's rectangle grows to the map's on `motion-open`. Its children are laid out at the map's size and each child's rectangle is mapped through the growth, so tiles never stretch text: labels are drawn at their own size and fade in with the second half. Siblings fade out over `motion-exit-fade-ms` from the start, in place. The new crumb appears at the start | Crossfade in place over `motion-reduced-fade-ms` |
| Up, Back | The reverse on `motion-exit`: the map shrinks into the folder's tile in the parent's layout, its labels gone by the first half; siblings fade in over `motion-fade-ms` | As above |
| Map and List | Crossfade over `motion-fade-ms`, the old out over `motion-exit-fade-ms` | Over `motion-reduced-fade-ms` |
| Scanning | Bar segments change width on `motion-move` at each update; the map is re-laid in one frame; the progress line's length on `motion-move` | Widths and line cut |
| Basket shows, hides | `conventions.md`'s inspector: width on `motion-move` | Crossfade |
| An item goes in or out | Its basket row's height on `motion-move`, content with the second half going in, gone by the first half going out; the total changes in place | Cut |
| Card "more" | Height on `motion-move`, rows with the second half | Cut |
| Moved | The basket's content crossfades to its moved form over `motion-fade-ms` | Over `motion-reduced-fade-ms` |
| Hover, selection, ticks | At once | Same |

**Game Mode:** every row is a cut.

## Accessibility

- **Surfaces.** Window roles only: text on `colour-window`, `colour-view`,
  `colour-window-raised` and `colour-sidebar`; `colour-text-3` only for the
  search placeholder.
- **Kind colours as marks** reach 3:1 on every window surface: at least 4.7
  in dark and 3.5 in light (`kind-images`, on `colour-sidebar`). A kind
  tile's glyph in `colour-view` passes 4.1 or more on every fill. On a map
  tile's mix, `colour-text` passes 9.9 (dark) and 11.8 (light),
  `colour-text-2` 5.1 and 5.6, the band 4.1 and 3.1.
- **Never colour alone.** Ten kinds cannot all be told apart by colour where
  any two touch (the map: pairs fall to ΔE 0.4 under deuteranopia and 11.9 in
  normal vision); symbols, names in the legend, labels and the tooltip carry
  identity, and gaps of the surface keep neighbours apart.
- **Increase Contrast.** Map tiles, bar segments and kind tiles ringed 1
  point `colour-hairline-strong` (structure: kind marks outlined); rows as
  `conventions.md`; glyphs bold.
- **Reduce Transparency.** No change: the sidebar and basket are opaque.
- **Focus.** Lists and tables show focus as the accent selection; the map's
  selected ring is its focus, arrows moving to the nearest tile that way;
  Review, Clear…, the basket's buttons and remove targets take the ring.
- **Largest text size.** Rows grow; the map's label and symbol thresholds
  grow by the text factor, so fewer tiles carry labels (structure); the
  legend goes to one column; the basket's foot lines wrap.
- **Targets.** Rows 28 and 40; checkboxes in their row's full height;
  `x-circle` 20; tiles their size, with any tile under 20 reached by arrows
  and the list.

## Board

Wave C's Cleaner board at 3440×1440 with the 1920×1080 screen beside it,
dark first and each in light. The window at its default place; crops are x,
y, width, height on the ultrawide.

1. **Kinds**: the ten tiles, bar segments and map tiles in both
   appearances, with each contrast. Crop: the strip, 1600 by 400.
2. **Overview** after a scan of All disks, basket shown. Crop 920, 260,
   1600, 900.
3. **Scanning**: Overview at 30% and 61%; the progress line. Crops as 2.
4. **Cleanup**: Safe to clear ticked, Worth a look with reasons, System;
   "{n} more" open. Crop as 2.
5. **Space, map**: a home folder with labels, symbols-only and smaller
   items; hover with its tooltip; a selection; items in the basket.
6. **Space drill**: opening `~/.local/share/Steam` in five frames, and up.
7. **Space, list**, with shares and disclosure.
8. **Duplicates** with "Shares storage"; **Large files**; **Untouched**.
9. **Basket**: 31 items; moving; moved with Undo and Empty Trash….
10. **States**: each empty view, no results, partial with unreadable
    folders, System's polkit frame (`prompts.md`'s crop).
11. **Narrow**: 720 wide, basket folded with its badge.
12. **Accessibility**: scenes 2 and 5 under Increase Contrast and at the
    largest text size.
13. **Motion**, on the harness's spring: drill in and up, the basket
    arriving, an item going in, Map to List; each under Reduce Motion.

## New tokens

| Group | Token | Value |
|---|---|---|
| `kind` | `games` … `other` per appearance | The table under Kinds |
| | `mix` | 0.22 (a map tile's share of its kind's colour) |
| `size` | `cleaner-width`, `cleaner-height` | 1200, 760 |
| | `cleaner-column` | 760 (Overview's and Cleanup's column) |
| | `kind-bar` | 20 |
| | `tile-symbol` | 32 |
| | `tile-label-width`, `tile-label-height` | 96, 44 |
| | `tile-min` | 8 |
| `stroke` | `kind` | 3 (a map tile's band) |
| `limit` | `overview-rows`, `cleanup-rows` | 5, 5 |
| `timing` | `scan-layout-ms` | 1000 |

The `kind` group is the one `schema.md` promised; `contrast.py` should add
its colours as marks (3:1) on the window surfaces. Phosphor glyphs to
vendor: `chart-pie-slice`, `broom`, `layout`, `copy`, `arrows-out-simple`,
`clock-counter-clockwise`, `hard-drive`, `film-strip`, `music-notes`,
`file-text`, `file-zip`, `code`, `package`, `stack`, `shapes`,
`check-circle`, `link`, `info`.
